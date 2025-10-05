//! UART logging for Renode testing
//! Duplicates key log messages to UART for test harness

use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::mode::Async;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use core::fmt::Write;

/// Log message types
#[derive(Clone, Copy)]
pub enum LogMessage {
    Banner,
    Init,
    Ready,
    Heartbeat(u32),
    CanStarted,
    FocStarted,
    PwmInit,
}

/// Global log channel (8 message buffer)
pub static LOG_CHANNEL: Channel<CriticalSectionRawMutex, LogMessage, 8> = Channel::new();

/// UART logging task
#[embassy_executor::task]
pub async fn uart_logger_task(mut uart: Uart<'static, Async>) {
    // Send banner on startup
    log_str(&mut uart, "===========================================").await;
    log_str(&mut uart, "  CLN17 v2.0 Joint Firmware").await;
    log_str(&mut uart, "  Target: STM32G431CB @ 170 MHz").await;
    log_str(&mut uart, "  Framework: Embassy + iRPC").await;
    log_str(&mut uart, "===========================================").await;
    
    loop {
        let msg = LOG_CHANNEL.receive().await;
        
        match msg {
            LogMessage::Banner => {
                log_str(&mut uart, "=== System Banner ===").await;
            }
            LogMessage::Init => {
                log_str(&mut uart, "Joint Firmware Initialization").await;
            }
            LogMessage::Ready => {
                log_str(&mut uart, "System Ready").await;
            }
            LogMessage::Heartbeat(count) => {
                let mut buf = heapless::String::<64>::new();
                let _ = write!(&mut buf, "System heartbeat: {} sec", count);
                log_str(&mut uart, &buf).await;
            }
            LogMessage::CanStarted => {
                log_str(&mut uart, "CAN task started").await;
            }
            LogMessage::FocStarted => {
                log_str(&mut uart, "FOC task started").await;
            }
            LogMessage::PwmInit => {
                log_str(&mut uart, "PWM initialized").await;
            }
        }
    }
}

async fn log_str(uart: &mut Uart<'_, Async>, msg: &str) {
    let _ = uart.write(msg.as_bytes()).await;
    let _ = uart.write(b"\r\n").await;
}

/// Send log message (non-blocking)
pub fn log(msg: LogMessage) {
    let _ = LOG_CHANNEL.try_send(msg);
}

/// UART configuration for logging
pub fn uart_config() -> Config {
    let mut config = Config::default();
    config.baudrate = 115200;
    config
}
