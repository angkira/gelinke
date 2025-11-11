/// Independent Watchdog Timer (IWDG) driver for STM32G431.
///
/// Provides hardware-level protection against system hangs.
/// If not refreshed within timeout period, forces MCU reset.
///
/// # Safety
/// Once started, the watchdog CANNOT be stopped! The system must
/// feed the watchdog regularly or the MCU will reset.

use embassy_stm32::wdg::IndependentWatchdog;

/// Watchdog configuration.
#[derive(Clone, Copy)]
pub struct WatchdogConfig {
    /// Timeout period in milliseconds.
    /// Must be long enough for longest task execution.
    /// Recommended: 500-1000ms for motor controllers.
    pub timeout_ms: u32,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 500,  // 500ms timeout (conservative)
        }
    }
}

/// Hardware watchdog timer.
///
/// Protects against:
/// - Infinite loops in control code
/// - Task deadlocks
/// - Peripheral hangs (CAN, SPI, ADC)
/// - Memory corruption
///
/// # Usage
/// ```no_run
/// let mut watchdog = Watchdog::new(WatchdogConfig::default());
///
/// loop {
///     // Do work...
///     watchdog.feed();  // Must call before timeout
/// }
/// ```
pub struct Watchdog {
    iwdg: IndependentWatchdog<'static>,
    timeout_ms: u32,
}

impl Watchdog {
    /// Initialize and start the watchdog.
    ///
    /// **WARNING:** Once started, watchdog cannot be stopped!
    /// Must be fed regularly or MCU will reset.
    ///
    /// # Arguments
    /// * `iwdg` - IWDG peripheral singleton
    /// * `config` - Watchdog configuration
    pub fn new(iwdg: embassy_stm32::peripherals::IWDG, config: WatchdogConfig) -> Self {
        let iwdg = IndependentWatchdog::new(iwdg.into(), config.timeout_ms);

        defmt::info!("Watchdog initialized: {}ms timeout", config.timeout_ms);
        defmt::warn!("Watchdog active - must feed every {}ms", config.timeout_ms / 2);

        Self {
            iwdg,
            timeout_ms: config.timeout_ms,
        }
    }

    /// Feed the watchdog (reset timeout counter).
    ///
    /// Must be called at least once per timeout period.
    /// Recommended: Call at half the timeout period for safety margin.
    #[inline]
    pub fn feed(&mut self) {
        self.iwdg.pet();
    }

    /// Get configured timeout in milliseconds.
    pub fn timeout_ms(&self) -> u32 {
        self.timeout_ms
    }

    /// Get recommended feed interval (half of timeout).
    pub fn feed_interval_ms(&self) -> u32 {
        self.timeout_ms / 2
    }

    /// Check if last reset was caused by watchdog.
    ///
    /// Note: This should be called early in main(), before RCC reset flags are cleared.
    pub fn was_watchdog_reset() -> bool {
        // Check RCC reset flags
        // STM32G4: RCC.CSR register bit IWDGRSTF
        // Embassy may not expose this directly yet
        // For now, return false (needs HAL extension)

        // TODO: When Embassy supports it:
        // unsafe {
        //     let rcc = &*embassy_stm32::pac::RCC::ptr();
        //     (rcc.csr.read().bits() & (1 << 29)) != 0  // IWDGRSTF bit
        // }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_config_default() {
        let config = WatchdogConfig::default();
        assert_eq!(config.timeout_ms, 500);
    }

    #[test]
    fn feed_interval_calculation() {
        let config = WatchdogConfig { timeout_ms: 1000 };
        // Feed interval should be half of timeout
        assert_eq!(config.timeout_ms / 2, 500);
    }
}
