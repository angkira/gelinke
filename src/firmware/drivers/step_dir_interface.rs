/// Step-Dir Interface Hardware
///
/// GPIO interface for classic Step/Direction stepper motor control.
/// Compatible with legacy step/dir drivers and motion controllers.
///
/// CLN17 V2.0 Hardware connections:
/// - PB5 (GPIO Input + EXTI): STEP pulse input (rising edge triggered)
/// - PB4 (GPIO Input): DIR direction input (high = forward, low = reverse)
/// - PA8 (GPIO Input): ENABLE input (high = enabled, low = disabled)
/// - PB3 (GPIO Output): ERROR signal output (high = fault, low = ok)

use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::Peripherals;

/// Step-Dir interface controller.
///
/// Provides hardware interface for step pulse counting and direction sensing.
pub struct StepDirInterface {
    /// STEP input with EXTI interrupt (rising edge)
    step: ExtiInput<'static>,
    /// DIR input (direction)
    dir: Input<'static>,
    /// ENABLE input (motor enable)
    enable: Input<'static>,
    /// ERROR output (fault indication)
    error: Output<'static>,
}

impl StepDirInterface {
    /// Create a new Step-Dir interface.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct
    ///
    /// # Initial State
    /// - ERROR pin low (no error)
    /// - All inputs configured with pull-ups
    pub fn new(p: Peripherals) -> Self {
        // STEP: Input with pull-up, EXTI on rising edge
        let step = ExtiInput::new(p.PB5, p.EXTI5, Pull::Up);

        // DIR: Input with pull-up (high = forward)
        let dir = Input::new(p.PB4, Pull::Up);

        // ENABLE: Input with pull-up (high = enabled)
        let enable = Input::new(p.PA8, Pull::Up);

        // ERROR: Output, start low (no error)
        let error = Output::new(p.PB3, Level::Low, Speed::Medium);

        Self {
            step,
            dir,
            enable,
            error,
        }
    }

    /// Wait for next step pulse (rising edge on STEP pin).
    ///
    /// This is an async function that will await until a step pulse arrives.
    /// Use this in the main step-dir control loop.
    pub async fn wait_for_step(&mut self) {
        self.step.wait_for_rising_edge().await;
    }

    /// Read current direction input.
    ///
    /// Returns:
    /// - `true` = Forward direction (DIR pin high)
    /// - `false` = Reverse direction (DIR pin low)
    pub fn read_direction(&self) -> bool {
        self.dir.is_high()
    }

    /// Check if motor is enabled via ENABLE input.
    ///
    /// Returns:
    /// - `true` = Motor enabled (ENABLE pin high)
    /// - `false` = Motor disabled (ENABLE pin low)
    pub fn is_enabled(&self) -> bool {
        self.enable.is_high()
    }

    /// Set ERROR output high (indicate fault).
    pub fn set_error(&mut self) {
        self.error.set_high();
        defmt::warn!("Step-Dir ERROR signal asserted");
    }

    /// Clear ERROR output (no fault).
    pub fn clear_error(&mut self) {
        self.error.set_low();
    }

    /// Toggle ERROR output (for blinking).
    pub fn toggle_error(&mut self) {
        self.error.toggle();
    }

    /// Read all inputs at once.
    ///
    /// Returns (direction, enabled).
    pub fn read_inputs(&self) -> (bool, bool) {
        (self.read_direction(), self.is_enabled())
    }
}

/// Step-Dir signal polarity configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepDirConfig {
    /// Invert STEP signal (true = trigger on falling edge)
    pub invert_step: bool,
    /// Invert DIR signal (true = high = reverse)
    pub invert_dir: bool,
    /// Invert ENABLE signal (true = low = enabled)
    pub invert_enable: bool,
}

impl Default for StepDirConfig {
    fn default() -> Self {
        Self {
            invert_step: false,   // Rising edge
            invert_dir: false,    // High = forward
            invert_enable: false, // High = enabled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default() {
        let config = StepDirConfig::default();
        assert!(!config.invert_step);
        assert!(!config.invert_dir);
        assert!(!config.invert_enable);
    }
}
