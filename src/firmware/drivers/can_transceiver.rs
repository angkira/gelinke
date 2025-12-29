/// CAN Transceiver Control for CLN17 V2.0
///
/// Controls the CAN transceiver operating mode via GPIO pins.
/// Hardware connections:
/// - PA9 (GPIO Output): CAN_SHDN - Shutdown control (active low)
/// - PA10 (GPIO Output): CAN_S - Standby/Normal mode select
///
/// Typical CAN transceivers (like TJA1051 or similar):
/// - SHDN pin: Low = shutdown (no power), High = operational
/// - S pin: Low = high-speed mode, High = standby/silent mode

use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::Peripherals;

/// CAN transceiver operating modes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanMode {
    /// Transceiver powered off (lowest power)
    Shutdown,
    /// High-speed mode (normal operation)
    Normal,
    /// Standby mode (listen-only, low power)
    Standby,
}

/// CAN transceiver controller.
pub struct CanTransceiver {
    /// SHDN pin - Shutdown control (active low)
    shutdown: Output<'static>,
    /// S pin - Standby mode select
    standby: Output<'static>,
}

impl CanTransceiver {
    /// Create a new CAN transceiver controller.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct
    ///
    /// # Initial State
    /// - Transceiver powered off (SHDN = Low)
    /// - Standby mode selected (S = High)
    pub fn new(p: Peripherals) -> Self {
        // SHDN: Start in shutdown (LOW)
        let shutdown = Output::new(p.PA9, Level::Low, Speed::Medium);

        // S: Start in standby mode (HIGH)
        let standby = Output::new(p.PA10, Level::High, Speed::Medium);

        Self { shutdown, standby }
    }

    /// Set CAN transceiver operating mode.
    pub fn set_mode(&mut self, mode: CanMode) {
        match mode {
            CanMode::Shutdown => {
                // Power off transceiver
                self.shutdown.set_low();
                defmt::info!("CAN transceiver: Shutdown");
            }
            CanMode::Normal => {
                // Enable transceiver in normal high-speed mode
                self.shutdown.set_high(); // Power on
                self.standby.set_low();   // Normal mode
                defmt::info!("CAN transceiver: Normal mode");
            }
            CanMode::Standby => {
                // Enable transceiver in standby/listen mode
                self.shutdown.set_high(); // Power on
                self.standby.set_high();  // Standby mode
                defmt::info!("CAN transceiver: Standby mode");
            }
        }
    }

    /// Enable CAN transceiver in normal mode.
    pub fn enable(&mut self) {
        self.set_mode(CanMode::Normal);
    }

    /// Disable CAN transceiver (shutdown).
    pub fn disable(&mut self) {
        self.set_mode(CanMode::Shutdown);
    }

    /// Put transceiver in standby/listen-only mode.
    pub fn standby(&mut self) {
        self.set_mode(CanMode::Standby);
    }

    /// Check if transceiver is powered on.
    pub fn is_enabled(&self) -> bool {
        self.shutdown.is_set_high()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_enum() {
        assert_eq!(CanMode::Shutdown, CanMode::Shutdown);
        assert_ne!(CanMode::Normal, CanMode::Standby);
    }
}
