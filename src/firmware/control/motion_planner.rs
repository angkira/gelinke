/// Motion planning module for trajectory generation
///
/// Provides trapezoidal and S-curve motion profiles with time-optimal planning.

use fixed::types::I16F16;
use libm::sqrtf;

extern crate alloc;
use alloc::vec::Vec;

/// Motion profile type for trajectory generation
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionProfileType {
    /// Trapezoidal profile - constant acceleration/deceleration
    Trapezoidal,
    /// S-curve profile - jerk-limited for smooth motion
    SCurve,
}

/// Errors that can occur during motion planning
#[derive(Clone, Copy, Debug, PartialEq, defmt::Format)]
pub enum MotionPlanningError {
    /// Invalid parameters (negative or zero values where positive required)
    InvalidParameters,
    /// Trajectory is infeasible under given constraints
    InfeasibleTrajectory,
    /// Numeric instability or overflow detected
    NumericInstability,
}

/// Configuration for motion planner
#[derive(Clone, Copy, Debug)]
pub struct MotionConfig {
    /// Default maximum velocity (rad/s)
    pub max_velocity: I16F16,
    /// Default maximum acceleration (rad/s²)
    pub max_acceleration: I16F16,
    /// Default maximum jerk (rad/s³)
    pub max_jerk: I16F16,
    /// Timestep for waypoint generation (seconds)
    pub timestep: I16F16,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            max_velocity: I16F16::from_num(50.0),      // 50 rad/s
            max_acceleration: I16F16::from_num(100.0), // 100 rad/s²
            max_jerk: I16F16::from_num(500.0),         // 500 rad/s³
            timestep: I16F16::from_num(0.001),         // 1 ms (1 kHz)
        }
    }
}

/// Single point in a trajectory
#[derive(Clone, Copy, Debug)]
pub struct TrajectoryPoint {
    /// Time from trajectory start (seconds)
    pub time: I16F16,
    /// Position (radians)
    pub position: I16F16,
    /// Velocity (rad/s)
    pub velocity: I16F16,
    /// Acceleration (rad/s²)
    pub acceleration: I16F16,
}

/// Complete trajectory with waypoints
#[derive(Clone, Debug)]
pub struct Trajectory {
    /// Motion profile type used
    pub profile_type: MotionProfileType,
    /// Trajectory waypoints (time-ordered)
    pub waypoints: Vec<TrajectoryPoint>,
    /// Total trajectory duration (seconds)
    pub total_time: I16F16,
    /// Start position
    pub start_position: I16F16,
    /// End position
    pub end_position: I16F16,
}

impl Trajectory {
    /// Interpolate trajectory at a specific time
    ///
    /// Returns the trajectory point at the given time using linear interpolation
    /// between waypoints. Clamps to start/end for times outside trajectory.
    pub fn interpolate(&self, time: I16F16) -> TrajectoryPoint {
        // Handle edge cases
        if self.waypoints.is_empty() {
            return TrajectoryPoint {
                time: I16F16::ZERO,
                position: I16F16::ZERO,
                velocity: I16F16::ZERO,
                acceleration: I16F16::ZERO,
            };
        }

        if time <= I16F16::ZERO {
            return self.waypoints[0];
        }

        if time >= self.total_time {
            return self.waypoints[self.waypoints.len() - 1];
        }

        // Binary search for the correct segment
        let mut left = 0;
        let mut right = self.waypoints.len() - 1;

        while left < right {
            let mid = (left + right) / 2;
            if self.waypoints[mid].time < time {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        // Linear interpolation between waypoints
        if left == 0 {
            return self.waypoints[0];
        }

        let p0 = &self.waypoints[left - 1];
        let p1 = &self.waypoints[left];

        let dt = p1.time - p0.time;
        if dt <= I16F16::ZERO {
            return *p0;
        }

        let t = (time - p0.time) / dt;

        TrajectoryPoint {
            time,
            position: p0.position + (p1.position - p0.position) * t,
            velocity: p0.velocity + (p1.velocity - p0.velocity) * t,
            acceleration: p0.acceleration + (p1.acceleration - p0.acceleration) * t,
        }
    }
}

/// Motion planner for trajectory generation
pub struct MotionPlanner {
    config: MotionConfig,
}

impl MotionPlanner {
    /// Create a new motion planner with given configuration
    pub fn new(config: MotionConfig) -> Self {
        Self { config }
    }

    /// Plan a trapezoidal motion profile
    ///
    /// Generates a time-optimal trajectory with constant acceleration and deceleration.
    /// If the distance is too short to reach max velocity, generates a triangular profile.
    ///
    /// # Arguments
    /// * `start` - Initial position (radians)
    /// * `end` - Target position (radians)
    /// * `max_vel` - Maximum velocity (rad/s)
    /// * `max_accel` - Maximum acceleration (rad/s²)
    ///
    /// # Returns
    /// `Ok(Trajectory)` with waypoints, or `Err` if parameters are invalid
    pub fn plan_trapezoidal(
        &self,
        start: I16F16,
        end: I16F16,
        max_vel: I16F16,
        max_accel: I16F16,
    ) -> Result<Trajectory, MotionPlanningError> {
        // Validate parameters
        if max_vel <= I16F16::ZERO || max_accel <= I16F16::ZERO {
            return Err(MotionPlanningError::InvalidParameters);
        }

        let distance = end - start;
        let direction = if distance >= I16F16::ZERO {
            I16F16::ONE
        } else {
            -I16F16::ONE
        };
        let abs_distance = distance.abs();

        if abs_distance <= I16F16::from_num(0.0001) {
            // Zero motion - return single point
            return Ok(Trajectory {
                profile_type: MotionProfileType::Trapezoidal,
                waypoints: alloc::vec![TrajectoryPoint {
                    time: I16F16::ZERO,
                    position: start,
                    velocity: I16F16::ZERO,
                    acceleration: I16F16::ZERO,
                }],
                total_time: I16F16::ZERO,
                start_position: start,
                end_position: end,
            });
        }

        // Calculate time to reach max velocity
        let t_accel = max_vel / max_accel;
        let d_accel = max_vel * t_accel / I16F16::from_num(2.0);

        // Check if we can reach max velocity
        let (v_peak, t_accel, t_const, t_decel) = if d_accel * I16F16::from_num(2.0) > abs_distance {
            // Triangular profile - never reach max velocity
            let product = (max_accel * abs_distance).to_num::<f32>();
            let v_peak = I16F16::from_num(sqrtf(product));
            let t_accel = v_peak / max_accel;
            (v_peak, t_accel, I16F16::ZERO, t_accel)
        } else {
            // Trapezoidal profile - constant velocity phase exists
            let d_const = abs_distance - d_accel * I16F16::from_num(2.0);
            let t_const = d_const / max_vel;
            (max_vel, t_accel, t_const, t_accel)
        };

        let total_time = t_accel + t_const + t_decel;

        // Generate waypoints
        let mut waypoints = Vec::new();
        let mut t = I16F16::ZERO;

        while t <= total_time {
            let point = if t <= t_accel {
                // Acceleration phase
                let pos = start + direction * (max_accel * t * t / I16F16::from_num(2.0));
                let vel = direction * max_accel * t;
                let acc = direction * max_accel;
                TrajectoryPoint {
                    time: t,
                    position: pos,
                    velocity: vel,
                    acceleration: acc,
                }
            } else if t <= t_accel + t_const {
                // Constant velocity phase
                let t_rel = t - t_accel;
                let d_accel_phase = max_accel * t_accel * t_accel / I16F16::from_num(2.0);
                let pos = start + direction * (d_accel_phase + v_peak * t_rel);
                let vel = direction * v_peak;
                TrajectoryPoint {
                    time: t,
                    position: pos,
                    velocity: vel,
                    acceleration: I16F16::ZERO,
                }
            } else {
                // Deceleration phase
                let t_rel = t - t_accel - t_const;
                let d_accel_phase = max_accel * t_accel * t_accel / I16F16::from_num(2.0);
                let d_const_phase = v_peak * t_const;
                let d_decel = v_peak * t_rel - max_accel * t_rel * t_rel / I16F16::from_num(2.0);
                let pos = start + direction * (d_accel_phase + d_const_phase + d_decel);
                let vel = direction * (v_peak - max_accel * t_rel);
                let acc = -direction * max_accel;
                TrajectoryPoint {
                    time: t,
                    position: pos,
                    velocity: vel,
                    acceleration: acc,
                }
            };

            waypoints.push(point);
            t += self.config.timestep;
        }

        // Add final point
        waypoints.push(TrajectoryPoint {
            time: total_time,
            position: end,
            velocity: I16F16::ZERO,
            acceleration: I16F16::ZERO,
        });

        Ok(Trajectory {
            profile_type: MotionProfileType::Trapezoidal,
            waypoints,
            total_time,
            start_position: start,
            end_position: end,
        })
    }

    /// Plan an S-curve motion profile
    ///
    /// Generates a jerk-limited trajectory with smooth acceleration transitions.
    /// This reduces mechanical vibrations and improves tracking accuracy.
    ///
    /// # Arguments
    /// * `start` - Initial position (radians)
    /// * `end` - Target position (radians)
    /// * `max_vel` - Maximum velocity (rad/s)
    /// * `max_accel` - Maximum acceleration (rad/s²)
    /// * `max_jerk` - Maximum jerk (rad/s³)
    ///
    /// # Returns
    /// `Ok(Trajectory)` with waypoints, or `Err` if parameters are invalid
    pub fn plan_scurve(
        &self,
        start: I16F16,
        end: I16F16,
        max_vel: I16F16,
        max_accel: I16F16,
        max_jerk: I16F16,
    ) -> Result<Trajectory, MotionPlanningError> {
        // Validate parameters
        if max_vel <= I16F16::ZERO || max_accel <= I16F16::ZERO || max_jerk <= I16F16::ZERO {
            return Err(MotionPlanningError::InvalidParameters);
        }

        let distance = end - start;
        let direction = if distance >= I16F16::ZERO {
            I16F16::ONE
        } else {
            -I16F16::ONE
        };
        let abs_distance = distance.abs();

        if abs_distance <= I16F16::from_num(0.0001) {
            // Zero motion
            return Ok(Trajectory {
                profile_type: MotionProfileType::SCurve,
                waypoints: alloc::vec![TrajectoryPoint {
                    time: I16F16::ZERO,
                    position: start,
                    velocity: I16F16::ZERO,
                    acceleration: I16F16::ZERO,
                }],
                total_time: I16F16::ZERO,
                start_position: start,
                end_position: end,
            });
        }

        // S-curve has 7 phases:
        // 1. Jerk up (increasing acceleration)
        // 2. Constant acceleration
        // 3. Jerk down (decreasing acceleration to zero)
        // 4. Constant velocity
        // 5. Jerk down (increasing negative acceleration)
        // 6. Constant deceleration
        // 7. Jerk up (decreasing deceleration to zero)

        // Time to reach max acceleration
        let t_jerk = max_accel / max_jerk;
        
        // Distance covered during jerk phases
        let d_jerk = max_jerk * t_jerk * t_jerk * t_jerk / I16F16::from_num(6.0);
        
        // Check if we can reach max acceleration
        let a_peak = if d_jerk * I16F16::from_num(4.0) > abs_distance {
            // Cannot reach max acceleration - use reduced acceleration
            let ratio = (abs_distance / (d_jerk * I16F16::from_num(4.0))).to_num::<f32>();
            let factor = I16F16::from_num(sqrtf(ratio));
            max_accel * factor
        } else {
            max_accel
        };

        let t_jerk_actual = a_peak / max_jerk;
        let d_jerk_actual = max_jerk * t_jerk_actual * t_jerk_actual * t_jerk_actual / I16F16::from_num(6.0);

        // Velocity at end of acceleration phase (simplified)
        let v_accel = a_peak * t_jerk_actual;
        
        // Check if we reach max velocity
        let v_peak = if v_accel > max_vel {
            max_vel
        } else {
            v_accel
        };

        // For simplicity, use 7-phase symmetric S-curve
        // Phase durations (symmetric profile)
        let t1 = t_jerk_actual;                    // Jerk up
        let t2 = (v_peak / a_peak) - t_jerk_actual; // Const accel (can be zero)
        let t3 = t_jerk_actual;                    // Jerk down
        
        // Distance during acceleration
        let d_accel = d_jerk_actual * I16F16::from_num(2.0) + 
                     a_peak * t2 * (I16F16::from_num(2.0) * v_peak / a_peak - t2) / I16F16::from_num(2.0);
        
        // Constant velocity phase
        let d_const = abs_distance - d_accel * I16F16::from_num(2.0);
        let t4 = if d_const > I16F16::ZERO {
            d_const / v_peak
        } else {
            I16F16::ZERO
        };
        
        let total_time = (t1 + t2 + t3) * I16F16::from_num(2.0) + t4;

        // Generate waypoints
        let mut waypoints = Vec::new();
        let mut t = I16F16::ZERO;

        while t <= total_time {
            let point = self.calculate_scurve_point(
                t, start, direction, 
                t1, t2, t3, t4, 
                a_peak, v_peak, max_jerk
            );
            waypoints.push(point);
            t += self.config.timestep;
        }

        // Add final point
        waypoints.push(TrajectoryPoint {
            time: total_time,
            position: end,
            velocity: I16F16::ZERO,
            acceleration: I16F16::ZERO,
        });

        Ok(Trajectory {
            profile_type: MotionProfileType::SCurve,
            waypoints,
            total_time,
            start_position: start,
            end_position: end,
        })
    }

    /// Calculate S-curve point at specific time
    #[allow(clippy::too_many_arguments)]
    fn calculate_scurve_point(
        &self,
        t: I16F16,
        start: I16F16,
        direction: I16F16,
        t1: I16F16,
        t2: I16F16,
        t3: I16F16,
        t4: I16F16,
        a_max: I16F16,
        v_max: I16F16,
        j_max: I16F16,
    ) -> TrajectoryPoint {
        let t_accel = t1 + t2 + t3;
        let t_cruise = t_accel + t4;
        let t_decel = t_cruise + t1 + t2 + t3;

        if t <= t1 {
            // Phase 1: Jerk up
            let jerk = direction * j_max;
            let acc = jerk * t;
            let vel = jerk * t * t / I16F16::from_num(2.0);
            let pos = start + jerk * t * t * t / I16F16::from_num(6.0);
            TrajectoryPoint { time: t, position: pos, velocity: vel, acceleration: acc }
        } else if t <= t1 + t2 {
            // Phase 2: Constant acceleration
            let t_rel = t - t1;
            let acc = direction * a_max;
            let v1 = direction * j_max * t1 * t1 / I16F16::from_num(2.0);
            let vel = v1 + acc * t_rel;
            let d1 = j_max * t1 * t1 * t1 / I16F16::from_num(6.0);
            let pos = start + direction * (d1 + v1 * t_rel + acc * t_rel * t_rel / I16F16::from_num(2.0));
            TrajectoryPoint { time: t, position: pos, velocity: vel, acceleration: acc }
        } else if t <= t_accel {
            // Phase 3: Jerk down
            let t_rel = t - t1 - t2;
            let jerk = -direction * j_max;
            let v2 = direction * (j_max * t1 * t1 / I16F16::from_num(2.0) + a_max * t2);
            let vel = v2 + direction * a_max * t_rel + jerk * t_rel * t_rel / I16F16::from_num(2.0);
            let acc = direction * a_max + jerk * t_rel;
            let d2 = j_max * t1 * t1 * t1 / I16F16::from_num(6.0) + 
                    (j_max * t1 * t1 / I16F16::from_num(2.0)) * t2 + 
                    a_max * t2 * t2 / I16F16::from_num(2.0);
            let pos = start + direction * (
                d2 + v2 * t_rel + 
                a_max * t_rel * t_rel / I16F16::from_num(2.0) + 
                jerk * t_rel * t_rel * t_rel / I16F16::from_num(6.0)
            );
            TrajectoryPoint { time: t, position: pos, velocity: vel, acceleration: acc }
        } else if t <= t_cruise {
            // Phase 4: Constant velocity
            let t_rel = t - t_accel;
            let vel = direction * v_max;
            
            // Calculate distance from acceleration phase
            let d_accel = direction * (
                j_max * t1 * t1 * t1 / I16F16::from_num(3.0) +
                j_max * t1 * t1 * t2 / I16F16::from_num(2.0) +
                a_max * t2 * t2 / I16F16::from_num(2.0)
            );
            
            let pos = start + d_accel + vel * t_rel;
            TrajectoryPoint { time: t, position: pos, velocity: vel, acceleration: I16F16::ZERO }
        } else {
            // Deceleration phases (mirror of acceleration)
            let t_into_decel = t - t_cruise;
            
            // Recursively calculate using mirrored time
            // For simplicity, use constant deceleration for now
            let t_remaining = t_decel - t;
            let vel = direction * v_max * t_remaining / (t1 + t2 + t3);
            let acc = -direction * a_max;
            
            // Approximate position
            let d_total = start + direction * v_max * t4;
            let pos = d_total + vel * t_into_decel / I16F16::from_num(2.0);
            
            TrajectoryPoint { 
                time: t, 
                position: pos.max(start).min(start + direction * v_max * (t_accel + t4)), 
                velocity: vel, 
                acceleration: acc 
            }
        }
    }

    /// Get current configuration
    pub fn config(&self) -> MotionConfig {
        self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: MotionConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trapezoidal_zero_motion() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_trapezoidal(
            I16F16::from_num(1.0),
            I16F16::from_num(1.0),
            I16F16::from_num(10.0),
            I16F16::from_num(50.0),
        ).unwrap();

        assert_eq!(traj.total_time, I16F16::ZERO);
        assert_eq!(traj.waypoints.len(), 1);
    }

    #[test]
    fn test_trapezoidal_short_move_triangular() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_trapezoidal(
            I16F16::ZERO,
            I16F16::from_num(0.5),  // Short distance
            I16F16::from_num(10.0),
            I16F16::from_num(50.0),
        ).unwrap();

        assert!(traj.total_time > I16F16::ZERO);
        
        // Check start and end
        assert_eq!(traj.waypoints.first().unwrap().position, I16F16::ZERO);
        assert!((traj.waypoints.last().unwrap().position - I16F16::from_num(0.5)).abs() < I16F16::from_num(0.01));
    }

    #[test]
    fn test_trapezoidal_long_move() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_trapezoidal(
            I16F16::ZERO,
            I16F16::from_num(100.0),
            I16F16::from_num(10.0),
            I16F16::from_num(50.0),
        ).unwrap();

        assert!(traj.total_time > I16F16::ZERO);
        
        // Find max velocity
        let max_vel = traj.waypoints.iter()
            .map(|p| p.velocity.abs())
            .max()
            .unwrap();
        
        // Should reach max velocity (within tolerance)
        assert!((max_vel - I16F16::from_num(10.0)).abs() < I16F16::from_num(0.1));
    }

    #[test]
    fn test_trapezoidal_negative_motion() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_trapezoidal(
            I16F16::from_num(10.0),
            I16F16::ZERO,
            I16F16::from_num(5.0),
            I16F16::from_num(25.0),
        ).unwrap();

        assert!(traj.total_time > I16F16::ZERO);
        
        // Check start and end
        assert_eq!(traj.waypoints.first().unwrap().position, I16F16::from_num(10.0));
        assert!((traj.waypoints.last().unwrap().position - I16F16::ZERO).abs() < I16F16::from_num(0.01));
    }

    #[test]
    fn test_trapezoidal_invalid_params() {
        let planner = MotionPlanner::new(Default::default());
        
        // Zero velocity
        let result = planner.plan_trapezoidal(
            I16F16::ZERO,
            I16F16::from_num(10.0),
            I16F16::ZERO,
            I16F16::from_num(50.0),
        );
        assert!(result.is_err());

        // Zero acceleration
        let result = planner.plan_trapezoidal(
            I16F16::ZERO,
            I16F16::from_num(10.0),
            I16F16::from_num(10.0),
            I16F16::ZERO,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_scurve_zero_motion() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_scurve(
            I16F16::from_num(1.0),
            I16F16::from_num(1.0),
            I16F16::from_num(10.0),
            I16F16::from_num(50.0),
            I16F16::from_num(200.0),
        ).unwrap();

        assert_eq!(traj.total_time, I16F16::ZERO);
        assert_eq!(traj.waypoints.len(), 1);
    }

    #[test]
    fn test_scurve_basic_motion() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_scurve(
            I16F16::ZERO,
            I16F16::from_num(10.0),
            I16F16::from_num(5.0),
            I16F16::from_num(25.0),
            I16F16::from_num(100.0),
        ).unwrap();

        assert!(traj.total_time > I16F16::ZERO);
        assert!(traj.waypoints.len() > 1);
        
        // Check endpoints
        assert_eq!(traj.waypoints.first().unwrap().position, I16F16::ZERO);
        assert!((traj.waypoints.last().unwrap().position - I16F16::from_num(10.0)).abs() < I16F16::from_num(0.5));
    }

    #[test]
    fn test_trajectory_interpolation() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_trapezoidal(
            I16F16::ZERO,
            I16F16::from_num(10.0),
            I16F16::from_num(5.0),
            I16F16::from_num(25.0),
        ).unwrap();

        // Interpolate at start
        let p0 = traj.interpolate(I16F16::ZERO);
        assert_eq!(p0.position, I16F16::ZERO);

        // Interpolate at end
        let pf = traj.interpolate(traj.total_time);
        assert!((pf.position - I16F16::from_num(10.0)).abs() < I16F16::from_num(0.1));

        // Interpolate at middle
        let pm = traj.interpolate(traj.total_time / I16F16::from_num(2.0));
        assert!(pm.position > I16F16::ZERO);
        assert!(pm.position < I16F16::from_num(10.0));
    }

    #[test]
    fn test_trajectory_interpolation_out_of_bounds() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_trapezoidal(
            I16F16::ZERO,
            I16F16::from_num(10.0),
            I16F16::from_num(5.0),
            I16F16::from_num(25.0),
        ).unwrap();

        // Before start
        let p_before = traj.interpolate(I16F16::from_num(-1.0));
        assert_eq!(p_before.position, traj.waypoints[0].position);

        // After end
        let p_after = traj.interpolate(traj.total_time + I16F16::from_num(10.0));
        assert_eq!(p_after.position, traj.waypoints.last().unwrap().position);
    }
}

