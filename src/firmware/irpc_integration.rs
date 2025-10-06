/// iRPC integration module
///
/// Bridges iRPC protocol with FOC control system.

use irpc::protocol::{DeviceId, LifecycleState, Message, Payload, SetTargetPayload, SetTargetPayloadV2, MotionProfile};
use irpc::joint::Joint;
use fixed::types::I16F16;

use crate::firmware::control::position::PositionController;
use crate::firmware::control::velocity::VelocityController;
use crate::firmware::control::motion_planner::{MotionPlanner, MotionConfig, Trajectory};

/// Bridge between iRPC Joint and FOC control system.
///
/// Translates high-level iRPC commands to low-level FOC setpoints.
pub struct JointFocBridge {
    /// iRPC protocol handler
    joint: Joint,
    /// Position controller
    position_ctrl: PositionController,
    /// Velocity controller
    velocity_ctrl: VelocityController,
    /// Motion planner for trajectory generation
    motion_planner: MotionPlanner,
    /// Current trajectory (if following a planned motion)
    current_trajectory: Option<Trajectory>,
    /// Trajectory start time (microseconds)
    trajectory_start_time: u64,
    /// Current position estimate (radians)
    current_position: I16F16,
    /// Current control mode
    mode: ControlMode,
}

/// Control mode for the joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlMode {
    /// Position control (angle + max velocity)
    Position,
    /// Velocity control (angular velocity)
    Velocity,
    /// Torque control (current setpoint)
    Torque,
}

impl JointFocBridge {
    /// Create a new iRPC-FOC bridge.
    pub fn new(device_id: DeviceId) -> Self {
        Self {
            joint: Joint::new(device_id),
            position_ctrl: PositionController::new(Default::default()),
            velocity_ctrl: VelocityController::new(Default::default()),
            motion_planner: MotionPlanner::new(MotionConfig::default()),
            current_trajectory: None,
            trajectory_start_time: 0,
            current_position: I16F16::ZERO,
            mode: ControlMode::Position,
        }
    }

    /// Get current lifecycle state.
    pub fn state(&self) -> LifecycleState {
        self.joint.state()
    }

    /// Process an incoming iRPC message and return response.
    ///
    /// This integrates iRPC lifecycle management with FOC control.
    pub fn handle_message(&mut self, msg: &Message) -> Option<Message> {
        // Let iRPC handle lifecycle transitions
        let response = self.joint.handle_message(msg)?;

        // Handle SetTarget commands when Active
        if self.state() == LifecycleState::Active {
            match &msg.payload {
                Payload::SetTarget(target) => {
                    self.apply_target_v1(target);
                }
                Payload::SetTargetV2(target) => {
                    self.apply_target_v2(target);
                }
                _ => {}
            }
        }

        Some(response)
    }

    /// Apply iRPC v1 target to FOC controllers (simple mode).
    fn apply_target_v1(&mut self, target: &SetTargetPayload) {
        // Clear any ongoing trajectory
        self.current_trajectory = None;
        
        match self.mode {
            ControlMode::Position => {
                // Convert degrees to radians
                let angle_rad = target.target_angle * core::f32::consts::PI / 180.0;
                let target_pos = I16F16::from_num(angle_rad);
                
                self.position_ctrl.set_target(target_pos);
                
                // Velocity limit is max velocity for position control
                let max_vel = I16F16::from_num(target.velocity_limit * core::f32::consts::PI / 180.0);
                let mut config = self.position_ctrl.config();
                config.max_velocity = max_vel;
                self.position_ctrl.set_config(config);
                
                defmt::info!("iRPC v1: Set position target {}° @ {}°/s", target.target_angle, target.velocity_limit);
            }
            ControlMode::Velocity => {
                // Direct velocity control
                let vel_rad_s = target.velocity_limit * core::f32::consts::PI / 180.0;
                let target_vel = I16F16::from_num(vel_rad_s);
                
                self.velocity_ctrl.set_target(target_vel);
                
                defmt::info!("iRPC v1: Set velocity target {}°/s", target.velocity_limit);
            }
            ControlMode::Torque => {
                defmt::info!("iRPC v1: Torque control not yet implemented");
            }
        }
    }

    /// Apply iRPC v2 target with motion profiling.
    fn apply_target_v2(&mut self, target: &SetTargetPayloadV2) {
        if self.mode != ControlMode::Position {
            defmt::warn!("iRPC v2: Motion profiling only available in Position mode");
            return;
        }

        // Convert degrees to radians
        let start_rad = self.current_position;
        let end_rad = I16F16::from_num(target.target_angle * core::f32::consts::PI / 180.0);
        let max_vel = I16F16::from_num(target.max_velocity * core::f32::consts::PI / 180.0);
        let max_accel = I16F16::from_num(target.max_acceleration * core::f32::consts::PI / 180.0);

        // Generate trajectory based on profile type
        let trajectory = match target.profile {
            MotionProfile::Trapezoidal => {
                self.motion_planner.plan_trapezoidal(start_rad, end_rad, max_vel, max_accel)
            }
            MotionProfile::SCurve => {
                let max_jerk = if target.max_jerk > 0.0 {
                    I16F16::from_num(target.max_jerk * core::f32::consts::PI / 180.0)
                } else {
                    self.motion_planner.config().max_jerk
                };
                self.motion_planner.plan_scurve(start_rad, end_rad, max_vel, max_accel, max_jerk)
            }
            MotionProfile::Adaptive => {
                // TODO: Implement adaptive profiling
                defmt::warn!("iRPC v2: Adaptive profiling not yet implemented, using trapezoidal");
                self.motion_planner.plan_trapezoidal(start_rad, end_rad, max_vel, max_accel)
            }
        };

        match trajectory {
            Ok(traj) => {
                defmt::info!("iRPC v2: Generated {:?} trajectory: {}° → {}°, {}s, {} waypoints",
                    traj.profile_type, 
                    target.target_angle,
                    target.target_angle,
                    traj.total_time.to_num::<f32>(),
                    traj.waypoints.len()
                );
                
                self.current_trajectory = Some(traj);
                self.trajectory_start_time = 0; // Will be set on first update
            }
            Err(e) => {
                defmt::error!("iRPC v2: Motion planning failed: {:?}", e);
            }
        }
    }

    /// Get position controller reference.
    pub fn position_controller(&mut self) -> &mut PositionController {
        &mut self.position_ctrl
    }

    /// Get velocity controller reference.
    pub fn velocity_controller(&mut self) -> &mut VelocityController {
        &mut self.velocity_ctrl
    }

    /// Get current control mode.
    pub fn control_mode(&self) -> ControlMode {
        self.mode
    }

    /// Set control mode.
    pub fn set_control_mode(&mut self, mode: ControlMode) {
        self.mode = mode;
        defmt::info!("iRPC: Control mode changed to {:?}", mode);
    }

    /// Update trajectory following (call this in FOC loop).
    ///
    /// Returns the target position from the trajectory, or None if no trajectory is active.
    pub fn update_trajectory(&mut self, current_time_us: u64, current_position: I16F16) -> Option<I16F16> {
        self.current_position = current_position;

        let trajectory = self.current_trajectory.as_ref()?;

        // Initialize start time on first call
        if self.trajectory_start_time == 0 {
            self.trajectory_start_time = current_time_us;
        }

        // Calculate elapsed time in seconds
        let elapsed_us = current_time_us - self.trajectory_start_time;
        let elapsed_s = I16F16::from_num(elapsed_us as f32 / 1_000_000.0);

        // Check if trajectory is complete
        if elapsed_s >= trajectory.total_time {
            let final_pos = trajectory.end_position;
            self.current_trajectory = None;
            self.trajectory_start_time = 0;
            defmt::debug!("iRPC v2: Trajectory complete at {} rad", final_pos);
            return Some(final_pos);
        }

        // Interpolate trajectory
        let point = trajectory.interpolate(elapsed_s);
        
        // Update position controller target
        self.position_ctrl.set_target(point.position);

        Some(point.position)
    }

    /// Check if a trajectory is currently active.
    pub fn has_active_trajectory(&self) -> bool {
        self.current_trajectory.is_some()
    }

    /// Get motion planner reference.
    pub fn motion_planner(&mut self) -> &mut MotionPlanner {
        &mut self.motion_planner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use irpc::protocol::Header;

    #[test]
    fn test_joint_bridge_initialization() {
        let bridge = JointFocBridge::new(0x0010);
        assert_eq!(bridge.state(), LifecycleState::Unconfigured);
        assert_eq!(bridge.control_mode(), ControlMode::Position);
    }

    #[test]
    fn test_lifecycle_transitions() {
        let mut bridge = JointFocBridge::new(0x0010);

        // Configure
        let msg = Message {
            header: Header {
                source_id: 0x0000,
                target_id: 0x0010,
                msg_id: 1,
            },
            payload: Payload::Configure,
        };
        let response = bridge.handle_message(&msg).unwrap();
        assert!(matches!(response.payload, Payload::Ack(_)));
        assert_eq!(bridge.state(), LifecycleState::Inactive);

        // Activate
        let msg = Message {
            header: Header {
                source_id: 0x0000,
                target_id: 0x0010,
                msg_id: 2,
            },
            payload: Payload::Activate,
        };
        let response = bridge.handle_message(&msg).unwrap();
        assert!(matches!(response.payload, Payload::Ack(_)));
        assert_eq!(bridge.state(), LifecycleState::Active);
    }

    #[test]
    fn test_set_target_when_active() {
        let mut bridge = JointFocBridge::new(0x0010);

        // Configure and activate
        let configure = Message {
            header: Header { source_id: 0x0000, target_id: 0x0010, msg_id: 1 },
            payload: Payload::Configure,
        };
        bridge.handle_message(&configure);

        let activate = Message {
            header: Header { source_id: 0x0000, target_id: 0x0010, msg_id: 2 },
            payload: Payload::Activate,
        };
        bridge.handle_message(&activate);

        // Now set target
        let target = Message {
            header: Header { source_id: 0x0000, target_id: 0x0010, msg_id: 3 },
            payload: Payload::SetTarget(SetTargetPayload {
                target_angle: 90.0,
                velocity_limit: 150.0,
            }),
        };
        let response = bridge.handle_message(&target).unwrap();
        assert!(matches!(response.payload, Payload::Ack(_)));

        // Verify target was set
        let expected_rad = 90.0 * core::f32::consts::PI / 180.0;
        assert!((bridge.position_controller().target().to_num::<f32>() - expected_rad).abs() < 0.01);
    }

    #[test]
    fn test_set_target_when_inactive_rejected() {
        let mut bridge = JointFocBridge::new(0x0010);

        // Try to set target without activation
        let target = Message {
            header: Header { source_id: 0x0000, target_id: 0x0010, msg_id: 1 },
            payload: Payload::SetTarget(SetTargetPayload {
                target_angle: 90.0,
                velocity_limit: 150.0,
            }),
        };
        let response = bridge.handle_message(&target).unwrap();
        assert!(matches!(response.payload, Payload::Nack { .. }));
    }

    #[test]
    fn test_irpc_serialization() {
        // Test NEW iRPC serialization API
        let msg = Message {
            header: Header {
                source_id: 0x0000,
                target_id: 0x0010,
                msg_id: 42,
            },
            payload: Payload::SetTarget(SetTargetPayload {
                target_angle: 90.0,
                velocity_limit: 150.0,
            }),
        };

        // Serialize
        let serialized = msg.serialize().expect("Serialization should succeed");
        
        // Check size is within bounds
        assert!(serialized.len() <= Message::max_size());
        assert!(serialized.len() > 0);

        // Deserialize
        let deserialized = Message::deserialize(&serialized)
            .expect("Deserialization should succeed");

        // Verify round-trip
        assert_eq!(deserialized.header.source_id, 0x0000);
        assert_eq!(deserialized.header.target_id, 0x0010);
        assert_eq!(deserialized.header.msg_id, 42);
        
        if let Payload::SetTarget(target) = deserialized.payload {
            assert!((target.target_angle - 90.0).abs() < 0.01);
            assert!((target.velocity_limit - 150.0).abs() < 0.01);
        } else {
            panic!("Payload should be SetTarget");
        }
    }

    #[test]
    fn test_irpc_max_size() {
        // Verify max_size constant is reasonable for CAN-FD
        let max = Message::max_size();
        assert_eq!(max, 128);
        assert!(max <= 64 * 2); // Within reasonable bounds for CAN-FD
    }
}

