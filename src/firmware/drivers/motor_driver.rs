/// DRV8844 Motor Driver Control
///
/// This module handles the control and safety pins for the DRV8844 dual H-bridge
/// stepper motor driver on CLN17 V2.0.
///
/// Hardware connections:
/// - PA4 (GPIO Output): nSLEEP - Motor driver enable (active high)
/// - PB1 (GPIO Input): nFAULT - Fault detection (active low)
/// - PB2 (GPIO Output): nRESET - Driver reset (active low)

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};

/// DRV8844 motor driver control interface.
///
/// Provides safe enable/disable, fault detection, and reset functionality.
pub struct MotorDriver {
    /// nSLEEP pin - Enable/disable motor driver (high = enabled)
    enable: Output<'static>,
    /// nFAULT pin - Fault status input (low = fault)
    fault: Input<'static>,
    /// nRESET pin - Driver reset (low = reset)
    reset: Output<'static>,
}

impl MotorDriver {
    /// Create a new motor driver control instance.
    ///
    /// # Arguments
    /// * `pa4` - PA4 pin for nSLEEP (enable/disable)
    /// * `pb1` - PB1 pin for nFAULT (fault detection)
    /// * `pb2` - PB2 pin for nRESET (driver reset)
    ///
    /// # Initial State
    /// - Motor driver disabled (nSLEEP = LOW)
    /// - Reset released (nRESET = HIGH)
    pub fn new(
        pa4: embassy_stm32::Peri<'static, impl embassy_stm32::gpio::Pin>,
        pb1: embassy_stm32::Peri<'static, impl embassy_stm32::gpio::Pin>,
        pb2: embassy_stm32::Peri<'static, impl embassy_stm32::gpio::Pin>,
    ) -> Self {
        // nSLEEP: Start disabled (LOW)
        let enable = Output::new(pa4, Level::Low, Speed::Medium);

        // nFAULT: Input with pull-up (fault is active low)
        let fault = Input::new(pb1, Pull::Up);

        // nRESET: Start with reset released (HIGH)
        let reset = Output::new(pb2, Level::High, Speed::Medium);

        Self {
            enable,
            fault,
            reset,
        }
    }

    /// Enable the motor driver.
    ///
    /// Sets nSLEEP high to enable the H-bridge outputs.
    /// Motor can draw current after this command.
    pub fn enable(&mut self) {
        self.enable.set_high();
        defmt::info!("Motor driver enabled");
    }

    /// Disable the motor driver.
    ///
    /// Sets nSLEEP low to put the driver in sleep mode.
    /// All outputs are high-impedance (motor coasts).
    pub fn disable(&mut self) {
        self.enable.set_low();
        defmt::info!("Motor driver disabled");
    }

    /// Check if motor driver is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enable.is_set_high()
    }

    /// Check if a fault condition is present.
    ///
    /// Returns `true` if the DRV8844 nFAULT pin is low (fault detected).
    ///
    /// Fault conditions include:
    /// - Overcurrent (ILIMIT exceeded)
    /// - Overtemperature shutdown (TSD)
    /// - Undervoltage lockout (UVLO)
    pub fn is_fault(&self) -> bool {
        self.fault.is_low()
    }

    /// Reset the motor driver.
    ///
    /// Pulses the nRESET pin low to clear latched faults and reinitialize
    /// the DRV8844 internal logic.
    ///
    /// The driver will be disabled after reset and must be re-enabled.
    pub fn reset(&mut self) {
        defmt::warn!("Resetting motor driver");

        // Disable driver first
        self.disable();

        // Pulse reset low
        self.reset.set_low();

        // Hold reset for sufficient time (datasheet: min 10 ns, use 1 us for safety)
        cortex_m::asm::delay(170);  // ~1 µs at 170 MHz

        // Release reset
        self.reset.set_high();

        defmt::info!("Motor driver reset complete");
    }

    /// Emergency stop - disable driver immediately.
    ///
    /// This is the same as disable() but with explicit emergency semantics.
    pub fn emergency_stop(&mut self) {
        defmt::error!("EMERGENCY STOP - Motor driver disabled");
        self.disable();
    }

    /// Check driver status and handle faults.
    ///
    /// Returns `Ok(())` if driver is healthy, `Err(())` if fault detected.
    /// Automatically disables driver on fault.
    pub fn check_status(&mut self) -> Result<(), ()> {
        if self.is_fault() {
            defmt::error!("DRV8844 fault detected - disabling driver");
            self.disable();
            Err(())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Cannot test GPIO operations without hardware or mocks
    // These tests verify logic only

    #[test]
    fn driver_states() {
        // Test that driver state enum is properly defined
        // This is a placeholder for future logic tests
        assert!(true);
    }
}
