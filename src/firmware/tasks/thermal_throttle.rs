/// Thermal Throttling Coordination
///
/// Provides lockless, real-time access to thermal throttle state
/// for high-frequency control loops (FOC @ 10 kHz).
///
/// Uses atomic operations to avoid blocking in control loops.

use core::sync::atomic::{AtomicU16, Ordering};

/// Thermal throttle factor (0.0 to 1.0) stored as u16 (0-10000).
///
/// This static allows FOC and Step-Dir tasks to read throttle state
/// without mutex contention in 10 kHz loops.
///
/// - 10000 = 1.0 (100%, no throttling)
/// - 5000 = 0.5 (50%, moderate throttling)
/// - 0 = 0.0 (0%, full throttle/shutdown)
static THROTTLE_FACTOR_U16: AtomicU16 = AtomicU16::new(10000);

/// Set thermal throttle factor (called by power_monitor task).
///
/// # Arguments
/// * `factor` - Throttle factor (0.0 to 1.0)
///   - 1.0 = No throttling (full power)
///   - 0.7 = 70% power (moderate throttling)
///   - 0.0 = Emergency shutdown
pub fn set_throttle_factor(factor: f32) {
    let factor_clamped = factor.clamp(0.0, 1.0);
    let factor_u16 = (factor_clamped * 10000.0) as u16;
    THROTTLE_FACTOR_U16.store(factor_u16, Ordering::Relaxed);
}

/// Get thermal throttle factor (called by FOC/Step-Dir tasks).
///
/// This is lockless and safe to call from 10 kHz loops.
///
/// # Returns
/// Throttle factor (0.0 to 1.0)
/// - 1.0 = No throttling
/// - <1.0 = Throttling active (reduce current proportionally)
/// - 0.0 = Emergency shutdown (disable immediately)
#[inline]
pub fn get_throttle_factor() -> f32 {
    let factor_u16 = THROTTLE_FACTOR_U16.load(Ordering::Relaxed);
    (factor_u16 as f32) / 10000.0
}

/// Check if thermal throttling is active.
#[inline]
pub fn is_throttling_active() -> bool {
    THROTTLE_FACTOR_U16.load(Ordering::Relaxed) < 10000
}

/// Check if emergency shutdown is triggered.
#[inline]
pub fn is_emergency_shutdown() -> bool {
    THROTTLE_FACTOR_U16.load(Ordering::Relaxed) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_factor_roundtrip() {
        set_throttle_factor(1.0);
        assert_eq!(get_throttle_factor(), 1.0);

        set_throttle_factor(0.5);
        assert!((get_throttle_factor() - 0.5).abs() < 0.001);

        set_throttle_factor(0.0);
        assert_eq!(get_throttle_factor(), 0.0);
    }

    #[test]
    fn throttle_clamping() {
        set_throttle_factor(1.5); // Over max
        assert_eq!(get_throttle_factor(), 1.0);

        set_throttle_factor(-0.5); // Under min
        assert_eq!(get_throttle_factor(), 0.0);
    }

    #[test]
    fn throttle_status_checks() {
        set_throttle_factor(1.0);
        assert!(!is_throttling_active());
        assert!(!is_emergency_shutdown());

        set_throttle_factor(0.7);
        assert!(is_throttling_active());
        assert!(!is_emergency_shutdown());

        set_throttle_factor(0.0);
        assert!(is_throttling_active());
        assert!(is_emergency_shutdown());
    }
}
