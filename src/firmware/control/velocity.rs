use fixed::types::I16F16;

/// Velocity controller configuration.
#[derive(Clone, Copy, Debug)]
pub struct VelocityConfig {
    /// Proportional gain (A per rad/s error)
    pub kp: I16F16,
    /// Integral gain (A per rad error)
    pub ki: I16F16,
    /// Maximum current output (A)
    pub max_current: I16F16,
}

impl Default for VelocityConfig {
    fn default() -> Self {
        Self {
            kp: I16F16::from_num(0.5), // 0.5 A per rad/s error
            ki: I16F16::from_num(0.1), // 0.1 A per rad error  
            max_current: I16F16::from_num(5.0), // 5 A max
        }
    }
}

/// PI velocity controller.
///
/// Outputs current setpoint (Q-axis) from velocity error.
pub struct VelocityController {
    config: VelocityConfig,
    target: I16F16,
    integral: I16F16,
}

impl VelocityController {
    /// Create a new velocity controller.
    pub fn new(config: VelocityConfig) -> Self {
        Self {
            config,
            target: I16F16::ZERO,
            integral: I16F16::ZERO,
        }
    }

    /// Set target velocity in rad/s.
    pub fn set_target(&mut self, velocity: I16F16) {
        self.target = velocity;
    }

    /// Get target velocity.
    pub fn target(&self) -> I16F16 {
        self.target
    }

    /// Update controller with current velocity.
    ///
    /// Returns current setpoint in Amperes (Q-axis).
    pub fn update(&mut self, velocity: I16F16, dt: f32) -> I16F16 {
        let error = self.target - velocity;
        
        // Proportional term
        let p_term = self.config.kp * error;
        
        // Integral term with anti-windup
        self.integral += self.config.ki * error * I16F16::from_num(dt);
        
        // Clamp integral
        if self.integral > self.config.max_current {
            self.integral = self.config.max_current;
        } else if self.integral < -self.config.max_current {
            self.integral = -self.config.max_current;
        }
        
        // Total output
        let output = p_term + self.integral;
        
        // Clamp output
        if output > self.config.max_current {
            self.config.max_current
        } else if output < -self.config.max_current {
            -self.config.max_current
        } else {
            output
        }
    }

    /// Reset integral term.
    pub fn reset(&mut self) {
        self.integral = I16F16::ZERO;
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: VelocityConfig) {
        self.config = config;
    }

    /// Get current configuration.
    pub fn config(&self) -> VelocityConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_controller_zero_error() {
        let mut controller = VelocityController::new(VelocityConfig::default());
        controller.set_target(I16F16::from_num(10.0));
        
        let current = controller.update(I16F16::from_num(10.0), 0.0001);
        assert_eq!(current, I16F16::ZERO);
    }

    #[test]
    fn velocity_controller_proportional() {
        let config = VelocityConfig {
            kp: I16F16::from_num(1.0),
            ki: I16F16::ZERO,
            max_current: I16F16::from_num(10.0),
        };
        let mut controller = VelocityController::new(config);
        controller.set_target(I16F16::from_num(5.0));
        
        let current = controller.update(I16F16::from_num(2.0), 0.0001);
        // Error = 3.0, kp = 1.0 => current = 3.0
        assert_eq!(current, I16F16::from_num(3.0));
    }

    #[test]
    fn velocity_controller_integral() {
        let config = VelocityConfig {
            kp: I16F16::ZERO,
            ki: I16F16::from_num(10.0),
            max_current: I16F16::from_num(10.0),
        };
        let mut controller = VelocityController::new(config);
        controller.set_target(I16F16::from_num(1.0));
        
        // First update: error = 1.0, dt = 0.1 => integral = 1.0
        controller.update(I16F16::ZERO, 0.1);
        
        // Second update: should accumulate
        let current = controller.update(I16F16::ZERO, 0.1);
        
        // integral = 10 * 1.0 * 0.1 + 10 * 1.0 * 0.1 = 2.0
        assert!((current.to_num::<f32>() - 2.0).abs() < 0.1);
    }

    #[test]
    fn velocity_controller_saturation() {
        let config = VelocityConfig {
            kp: I16F16::from_num(2.0),
            ki: I16F16::ZERO,
            max_current: I16F16::from_num(5.0),
        };
        let mut controller = VelocityController::new(config);
        controller.set_target(I16F16::from_num(10.0));
        
        let current = controller.update(I16F16::ZERO, 0.0001);
        // Error = 10.0, kp = 2.0 => 20.0, but clamped to 5.0
        assert_eq!(current, I16F16::from_num(5.0));
    }

    #[test]
    fn velocity_controller_reset() {
        let mut controller = VelocityController::new(VelocityConfig::default());
        controller.set_target(I16F16::from_num(10.0));
        
        // Build up integral
        controller.update(I16F16::ZERO, 0.1);
        
        // Reset
        controller.reset();
        
        // Check integral is zero
        assert_eq!(controller.integral, I16F16::ZERO);
    }
}

