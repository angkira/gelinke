//! Mock FOC task for Renode testing
//! 
//! This module provides a slower FOC control loop (1 Hz instead of 10 kHz)
//! to avoid overwhelming the Renode executor.

use embassy_time::{Duration, Timer};

/// Mock FOC control loop for Renode
/// 
/// Runs at 1 Hz instead of 10 kHz to avoid overwhelming the emulator.
/// Real hardware needs 10 kHz for proper motor control, but Renode
/// tests only need to verify the task starts correctly.
#[embassy_executor::task]
pub async fn control_loop_mock() {
    defmt::info!("[MOCK FOC] Control loop starting (1 Hz mode)");
    
    // Send log message
    crate::firmware::uart_log::log(crate::firmware::uart_log::LogMessage::FocStarted);
    
    let mut iteration = 0u32;
    
    loop {
        Timer::after(Duration::from_secs(1)).await;
        
        iteration = iteration.wrapping_add(1);
        defmt::debug!("[MOCK FOC] Iteration: {}", iteration);
    }
}
