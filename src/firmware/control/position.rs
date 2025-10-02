use fixed::types::I16F16;

/// Position controller configuration.
#[derive(Clone, Copy, Debug)]
pub struct PositionConfig {
    /// Proportional gain (rad/s per rad error)
    pub kp: I16F16,
    /// Maximum velocity output (rad/s)
    pub max_velocity: I16F16,
}

impl Default for PositionConfig {
    fn default() -> Self {
        Self {
            kp: I16F16::from_num(10.0), // 10 rad/s per rad error
            max_velocity: I16F16::from_num(50.0), // 50 rad/s max
        }
    }
}

/// Proportional position controller.
///
/// Outputs velocity setpoint from position error.
pub struct PositionController {
    config: PositionConfig,
    target: I16F16,
}

impl PositionController {
    /// Create a new position controller.
    pub fn new(config: PositionConfig) -> Self {
        Self {
            config,
            target: I16F16::ZERO,
        }
    }

    /// Set target position in radians.
    pub fn set_target(&mut self, position: I16F16) {
        self.target = position;
    }

    /// Get target position.
    pub fn target(&self) -> I16F16 {
        self.target
    }

    /// Update controller with current position.
    ///
    /// Returns velocity setpoint in rad/s.
    pub fn update(&mut self, position: I16F16) -> I16F16 {
        let error = self.target - position;
        let velocity = self.config.kp * error;
        
        // Clamp output
        if velocity > self.config.max_velocity {
            self.config.max_velocity
        } else if velocity < -self.config.max_velocity {
            -self.config.max_velocity
        } else {
            velocity
        }
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: PositionConfig) {
        self.config = config;
    }

    /// Get current configuration.
    pub fn config(&self) -> PositionConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_controller_zero_error() {
        let mut controller = PositionController::new(PositionConfig::default());
        controller.set_target(I16F16::from_num(1.0));
        
        let velocity = controller.update(I16F16::from_num(1.0));
        assert_eq!(velocity, I16F16::ZERO);
    }

    #[test]
    fn position_controller_positive_error() {
        let config = PositionConfig {
            kp: I16F16::from_num(10.0),
            max_velocity: I16F16::from_num(100.0),
        };
        let mut controller = PositionController::new(config);
        controller.set_target(I16F16::from_num(2.0));
        
        let velocity = controller.update(I16F16::from_num(1.0));
        // Error = 1.0, kp = 10.0 => velocity = 10.0
        assert_eq!(velocity, I16F16::from_num(10.0));
    }

    #[test]
    fn position_controller_saturation() {
        let config = PositionConfig {
            kp: I16F16::from_num(10.0),
            max_velocity: I16F16::from_num(20.0),
        };
        let mut controller = PositionController::new(config);
        controller.set_target(I16F16::from_num(10.0));
        
        let velocity = controller.update(I16F16::ZERO);
        // Error = 10.0, kp = 10.0 => 100.0, but clamped to 20.0
        assert_eq!(velocity, I16F16::from_num(20.0));
    }
}

