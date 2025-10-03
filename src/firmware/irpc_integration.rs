/// iRPC integration module
///
/// Bridges iRPC protocol with FOC control system.

use irpc::protocol::{DeviceId, LifecycleState, Message, Payload, SetTargetPayload};
use irpc::joint::Joint;
use fixed::types::I16F16;

use crate::firmware::control::position::PositionController;
use crate::firmware::control::velocity::VelocityController;

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

        // If it's a SetTarget command and we're Active, configure FOC
        if let Payload::SetTarget(target) = &msg.payload {
            if self.state() == LifecycleState::Active {
                self.apply_target(target);
            }
        }

        Some(response)
    }

    /// Apply iRPC target to FOC controllers.
    fn apply_target(&mut self, target: &SetTargetPayload) {
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
                
                defmt::info!("iRPC: Set position target {}° @ {}°/s", target.target_angle, target.velocity_limit);
            }
            ControlMode::Velocity => {
                // Direct velocity control
                let vel_rad_s = target.velocity_limit * core::f32::consts::PI / 180.0;
                let target_vel = I16F16::from_num(vel_rad_s);
                
                self.velocity_ctrl.set_target(target_vel);
                
                defmt::info!("iRPC: Set velocity target {}°/s", target.velocity_limit);
            }
            ControlMode::Torque => {
                // Torque control would set current directly
                defmt::info!("iRPC: Torque control not yet implemented");
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
}

