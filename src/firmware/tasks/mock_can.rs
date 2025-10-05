//! Mock CAN task for Renode testing
//! 
//! This module provides a no-op CAN communication task that doesn't block
//! on async-await for CAN messages, allowing Renode tests to complete.

use embassy_time::{Duration, Timer};

/// Mock CAN communication task for Renode
/// 
/// This task simulates CAN initialization and periodic "message processing"
/// without actually waiting for real CAN messages, which would block forever
/// in Renode without a real CAN bus.
#[embassy_executor::task]
pub async fn can_communication_mock(_node_id: u16) {
    defmt::info!("[MOCK CAN] Task started (Renode mode)");
    
    // Simulate initialization delay
    Timer::after(Duration::from_millis(10)).await;
    
    defmt::info!("[MOCK CAN] Initialization complete");
    
    // Send log message
    crate::firmware::uart_log::log(crate::firmware::uart_log::LogMessage::CanStarted);
    
    // Periodic "message processing" loop (no real CAN)
    loop {
        Timer::after(Duration::from_secs(5)).await;
        defmt::debug!("[MOCK CAN] Heartbeat (no real messages)");
    }
}
