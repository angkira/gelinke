use embassy_executor::Spawner;
use embassy_stm32::{bind_interrupts, Peripherals};
use embassy_stm32::usart::Uart;
use embassy_stm32::peripherals;
use embassy_time::{Duration, Timer};

use crate::firmware::config::{MotorConfig, EncoderConfig};
use crate::firmware::control::observer::ObserverConfig;
use crate::firmware::control::position::PositionConfig;
use crate::firmware::control::velocity::VelocityConfig;
use crate::firmware::drivers::can::DEFAULT_NODE_ID;
use crate::firmware::uart_log::{self, LogMessage};

bind_interrupts!(struct UartIrqs {
    USART1 => embassy_stm32::usart::InterruptHandler<peripherals::USART1>;
});

/// System state shared between tasks.
pub struct SystemState {
    pub motor_config: MotorConfig,
    pub encoder_config: EncoderConfig,
    pub position_config: PositionConfig,
    pub velocity_config: VelocityConfig,
    pub observer_config: ObserverConfig,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            motor_config: MotorConfig::default(),
            encoder_config: EncoderConfig::tle5012b(),
            position_config: PositionConfig::default(),
            velocity_config: VelocityConfig::default(),
            observer_config: ObserverConfig::default(),
        }
    }
}

/// Initialize system and spawn tasks.
pub async fn initialize(spawner: Spawner, p: Peripherals) -> ! {
    defmt::info!("=== Joint Firmware Initialization START ===");
    
    // Initialize UART for test logging (USART1: PA9=TX, PA10=RX)
    defmt::info!("[TRACE] About to initialize UART...");
    defmt::info!("[TRACE] UART pins: PA9=TX, PA10=RX");
    defmt::info!("[TRACE] UART DMA: CH1=TX, CH2=RX");
    
    let mut uart = Uart::new(
        p.USART1,
        p.PA10, // RX
        p.PA9,  // TX
        UartIrqs,
        p.DMA1_CH1,  // TX DMA
        p.DMA1_CH2,  // RX DMA
        uart_log::uart_config(),
    ).expect("UART init failed");
    
    defmt::info!("[TRACE] ✓ UART initialized successfully!");
    
    // Send async banner to UART for tests
    defmt::info!("[TRACE] About to write banner to UART...");
    let _ = uart.write(b"===========================================\r\n").await.ok();
    defmt::info!("[TRACE] ✓ Banner line 1 written");
    let _ = uart.write(b"  CLN17 v2.0 Joint Firmware\r\n").await.ok();
    defmt::info!("[TRACE] ✓ Banner line 2 written");
    let _ = uart.write(b"  Target: STM32G431CB @ 170 MHz\r\n").await.ok();
    defmt::info!("[TRACE] ✓ Banner line 3 written");
    let _ = uart.write(b"  Framework: Embassy + iRPC\r\n").await.ok();
    defmt::info!("[TRACE] ✓ Banner line 4 written");
    let _ = uart.write(b"===========================================\r\n").await.ok();
    defmt::info!("[TRACE] ✓ All banner lines written!");
    
    // Spawn UART logger task
    defmt::info!("[TRACE] About to spawn UART logger task...");
    spawner.spawn(uart_log::uart_logger_task(uart)).ok();
    defmt::info!("[TRACE] ✓ UART logger task spawned!");
    
    defmt::info!("Target: STM32G431CB @ 170 MHz");
    defmt::info!("Framework: Embassy + iRPC");
    uart_log::log(LogMessage::Init);
    
    // TODO: Initialize other drivers (PWM, ADC, Encoder)
    // This requires proper peripheral splitting
    
    // Send log messages via channel (async)
    uart_log::log(LogMessage::Init);
    
    // Spawn CAN communication task
    // In Renode mock mode, use a no-op task that doesn't block on CAN messages
    #[cfg(feature = "renode-mock")]
    {
        defmt::info!("[TRACE] Spawning MOCK CAN task (Renode mode)...");
        spawner.spawn(crate::firmware::tasks::mock_can::can_communication_mock(
            DEFAULT_NODE_ID,
        )).ok();
        defmt::info!("[TRACE] ✓ MOCK CAN task spawned!");
    }
    
    // In production mode, use real iRPC CAN transport
    #[cfg(not(feature = "renode-mock"))]
    {
        defmt::info!("[TRACE] About to spawn CAN task...");
        spawner.spawn(crate::firmware::tasks::can_comm::can_communication(
            DEFAULT_NODE_ID,
            p.FDCAN1,  // FDCAN peripheral (iRPC takes ownership)
            p.PA12,    // TX pin
            p.PA11,    // RX pin
        )).ok();
        defmt::info!("[TRACE] ✓ CAN task spawned!");
        uart_log::log(LogMessage::CanStarted);
    }
    
    // Spawn FOC control loop
    // In Renode mock mode, use 1 Hz loop to avoid overwhelming executor
    #[cfg(feature = "renode-mock")]
    {
        defmt::info!("[TRACE] Spawning MOCK FOC task (1 Hz mode)...");
        spawner.spawn(crate::firmware::tasks::mock_foc::control_loop_mock()).ok();
        defmt::info!("[TRACE] ✓ MOCK FOC task spawned!");
    }
    
    // In production mode, use real 10 kHz FOC loop
    #[cfg(not(feature = "renode-mock"))]
    {
        defmt::info!("[TRACE] About to spawn FOC task...");
        spawner.spawn(crate::firmware::tasks::foc::control_loop()).ok();
        defmt::info!("[TRACE] ✓ FOC task spawned!");
        uart_log::log(LogMessage::FocStarted);
    }
    
    defmt::info!("=== System Ready ===");
    uart_log::log(LogMessage::Ready);
    
    // Main heartbeat
    let mut counter = 0u32;
    loop {
        Timer::after(Duration::from_secs(1)).await;
        counter = counter.wrapping_add(1);
        defmt::info!("System heartbeat: {} sec", counter);
        uart_log::log(LogMessage::Heartbeat(counter));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_state_default() {
        let state = SystemState::default();
        assert_eq!(state.motor_config.pole_pairs, 7);
        assert_eq!(state.encoder_config.resolution_bits, 15);
    }
}

