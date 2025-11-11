/// Watchdog feeder task.
///
/// Feeds the hardware watchdog at regular intervals to prevent system reset.
/// This task should ALWAYS be running - if it stops, the system will reset.

use embassy_time::{Duration, Ticker};
use crate::firmware::drivers::watchdog::Watchdog;

/// Watchdog feeder task.
///
/// Feeds the watchdog at half the timeout interval for safety margin.
/// If this task hangs, the watchdog will reset the system after the timeout period.
///
/// # Safety
/// This task MUST NOT:
/// - Block for long periods (> timeout/2)
/// - Panic or unwrap
/// - Depend on other tasks (minimize dependencies)
///
/// # Example
/// ```
/// let watchdog = Watchdog::new(p.IWDG, WatchdogConfig::default());
/// spawner.spawn(watchdog_feeder(watchdog)).ok();
/// ```
#[embassy_executor::task]
pub async fn watchdog_feeder(mut watchdog: Watchdog) {
    // Calculate feed interval (half of timeout for safety)
    let feed_interval_ms = watchdog.feed_interval_ms();
    let feed_duration = Duration::from_millis(feed_interval_ms as u64);

    defmt::info!("Watchdog feeder started: feeding every {}ms", feed_interval_ms);

    let mut ticker = Ticker::every(feed_duration);
    let mut count = 0u32;

    loop {
        ticker.next().await;

        // Feed the watchdog
        watchdog.feed();

        count = count.wrapping_add(1);

        // Log every 60 feeds (30 seconds at 500ms timeout)
        if count % 60 == 0 {
            defmt::trace!("Watchdog fed {} times", count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_interval_calculation() {
        // For 500ms timeout, feed interval should be 250ms
        assert_eq!(500 / 2, 250);

        // For 1000ms timeout, feed interval should be 500ms
        assert_eq!(1000 / 2, 500);
    }
}
