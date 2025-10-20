//! Mock Step-Dir task for Renode testing
//!
//! This module provides a slower Step-Dir control loop (1 Hz instead of 1 kHz)
//! to avoid overwhelming the Renode executor.

use embassy_time::{Duration, Timer};

/// Mock Step-Dir control loop for Renode
///
/// Runs at 1 Hz instead of 1 kHz to avoid overwhelming the emulator.
/// Real hardware needs 1 kHz for proper step control, but Renode
/// tests only need to verify the task starts correctly.
#[embassy_executor::task]
pub async fn control_loop_mock() {
    defmt::info!("[MOCK STEP-DIR] Control loop starting (1 Hz mode)");

    // Log that step-dir task is active
    // Note: We could add LogMessage::StepDirStarted in the future

    let mut iteration = 0u32;
    let mut step_count = 0i32;

    loop {
        Timer::after(Duration::from_secs(1)).await;

        iteration = iteration.wrapping_add(1);

        // Simulate stepping
        step_count = step_count.wrapping_add(10);

        defmt::debug!(
            "[MOCK STEP-DIR] Iteration: {}, Steps: {}",
            iteration,
            step_count
        );
    }
}
