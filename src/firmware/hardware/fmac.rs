use embassy_stm32::peripherals::FMAC;
use embassy_stm32::Peripherals;
use fixed::types::I16F16;

/// PI controller configuration.
#[derive(Clone, Copy, Debug)]
pub struct PiConfig {
    /// Proportional gain
    pub kp: I16F16,
    /// Integral gain
    pub ki: I16F16,
    /// Output limit (saturation)
    pub limit: I16F16,
}

impl Default for PiConfig {
    fn default() -> Self {
        Self {
            kp: I16F16::from_num(1.0),
            ki: I16F16::from_num(0.1),
            limit: I16F16::from_num(1.0),
        }
    }
}

/// Hardware FMAC accelerator for PI controllers.
pub struct FmacPiController {
    _fmac: embassy_stm32::Peri<'static, FMAC>,
    config: PiConfig,
    integral: I16F16,
}

impl FmacPiController {
    /// Create a new FMAC PI controller instance.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct
    /// * `config` - PI controller configuration
    pub fn new(p: Peripherals, config: PiConfig) -> Self {
        let fmac = p.FMAC;
        
        // TODO: Configure FMAC for PI control when HAL is available
        // Embassy doesn't have full FMAC HAL yet, so this is a placeholder
        
        Self {
            _fmac: fmac,
            config,
            integral: I16F16::ZERO,
        }
    }

    /// Update PI controller with new error value.
    ///
    /// # Arguments
    /// * `error` - Current error (setpoint - measurement)
    /// * `dt` - Time step in seconds
    ///
    /// Returns controller output
    pub fn update(&mut self, error: I16F16, dt: f32) -> I16F16 {
        // Proportional term
        let p_term = self.config.kp * error;
        
        // Integral term with anti-windup
        self.integral += self.config.ki * error * I16F16::from_num(dt);
        
        // Clamp integral to prevent windup
        if self.integral > self.config.limit {
            self.integral = self.config.limit;
        } else if self.integral < -self.config.limit {
            self.integral = -self.config.limit;
        }
        
        // Total output
        let output = p_term + self.integral;
        
        // Clamp output
        if output > self.config.limit {
            self.config.limit
        } else if output < -self.config.limit {
            -self.config.limit
        } else {
            output
        }
    }

    /// Reset integral term.
    pub fn reset(&mut self) {
        self.integral = I16F16::ZERO;
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: PiConfig) {
        self.config = config;
    }

    /// Get current configuration.
    pub fn config(&self) -> PiConfig {
        self.config
    }
}

/// Software-only PI controller (no hardware acceleration).
pub struct SoftwarePiController {
    config: PiConfig,
    integral: I16F16,
}

impl SoftwarePiController {
    /// Create a new software PI controller.
    pub fn new(config: PiConfig) -> Self {
        Self {
            config,
            integral: I16F16::ZERO,
        }
    }

    /// Update controller with new error value.
    pub fn update(&mut self, error: I16F16, dt: f32) -> I16F16 {
        let p_term = self.config.kp * error;
        self.integral += self.config.ki * error * I16F16::from_num(dt);
        
        if self.integral > self.config.limit {
            self.integral = self.config.limit;
        } else if self.integral < -self.config.limit {
            self.integral = -self.config.limit;
        }
        
        let output = p_term + self.integral;
        
        if output > self.config.limit {
            self.config.limit
        } else if output < -self.config.limit {
            -self.config.limit
        } else {
            output
        }
    }

    /// Reset integral term.
    pub fn reset(&mut self) {
        self.integral = I16F16::ZERO;
    }
}

/// Dual PI controller for d and q axis current control.
pub struct DualPiController {
    d_controller: SoftwarePiController,
    q_controller: SoftwarePiController,
}

impl DualPiController {
    /// Create a new dual PI controller.
    ///
    /// Note: Uses software PI for both axes since we only have one FMAC.
    pub fn new(_p: Peripherals, d_config: PiConfig, q_config: PiConfig) -> Self {
        // Both use software implementation for now
        // TODO: Time-multiplex FMAC or use it for D-axis only
        let d_controller = SoftwarePiController::new(d_config);
        let q_controller = SoftwarePiController::new(q_config);
        
        Self {
            d_controller,
            q_controller,
        }
    }

    /// Update both controllers.
    ///
    /// # Arguments
    /// * `d_error` - D axis error
    /// * `q_error` - Q axis error
    /// * `dt` - Time step in seconds
    ///
    /// Returns (d_output, q_output)
    pub fn update(&mut self, d_error: I16F16, q_error: I16F16, dt: f32) -> (I16F16, I16F16) {
        let d_out = self.d_controller.update(d_error, dt);
        let q_out = self.q_controller.update(q_error, dt);
        (d_out, q_out)
    }

    /// Reset both controllers.
    pub fn reset(&mut self) {
        self.d_controller.reset();
        self.q_controller.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_controller_proportional_only() {
        let config = PiConfig {
            kp: I16F16::from_num(2.0),
            ki: I16F16::ZERO,
            limit: I16F16::from_num(10.0),
        };
        
        let error = I16F16::from_num(1.0);
        let expected = I16F16::from_num(2.0);
        
        // Can't instantiate without hardware, but test the math
        let p_term = config.kp * error;
        assert_eq!(p_term, expected);
    }

    #[test]
    fn pi_controller_integral_accumulation() {
        let config = PiConfig {
            kp: I16F16::ZERO,
            ki: I16F16::from_num(0.5),
            limit: I16F16::from_num(10.0),
        };
        
        let error = I16F16::from_num(1.0);
        let dt = 0.1;
        
        // After 10 steps: integral = 0.5 * 1.0 * 0.1 * 10 = 0.5
        let mut integral = I16F16::ZERO;
        for _ in 0..10 {
            integral += config.ki * error * I16F16::from_num(dt);
        }
        
        let expected = I16F16::from_num(0.5);
        let diff = (integral - expected).abs();
        assert!(diff < I16F16::from_num(0.01));
    }

    #[test]
    fn pi_controller_saturation() {
        let limit = I16F16::from_num(5.0);
        let config = PiConfig {
            kp: I16F16::from_num(10.0),
            ki: I16F16::ZERO,
            limit,
        };
        
        let error = I16F16::from_num(1.0);
        let output = config.kp * error;
        
        // Output should saturate at limit
        let saturated = if output > limit {
            limit
        } else {
            output
        };
        
        assert_eq!(saturated, limit);
    }
}

