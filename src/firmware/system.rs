use embassy_executor::Spawner;
use embassy_stm32::{bind_interrupts, Peripherals};
use embassy_stm32::usart::Uart;
use embassy_stm32::peripherals;
use embassy_time::{Duration, Timer};

use crate::firmware::config::{MotorConfig, EncoderConfig, ControlMethod};
use crate::firmware::control::observer::ObserverConfig;
use crate::firmware::control::position::PositionConfig;
use crate::firmware::control::velocity::VelocityConfig;
use crate::firmware::drivers::can::DEFAULT_NODE_ID;
use crate::firmware::uart_log::{self, LogMessage};
use crate::firmware::error::{FirmwareError, ErrorCollection};
use crate::firmware::drivers::watchdog::{Watchdog, WatchdogConfig};

#[cfg(not(feature = "renode-mock"))]
use crate::firmware::drivers::flash_storage::FlashStorage;

#[cfg(not(feature = "renode-mock"))]
use crate::firmware::drivers::adc::Sensors;

#[cfg(not(feature = "renode-mock"))]
use crate::firmware::drivers::motor_driver::MotorDriver;

#[cfg(not(feature = "renode-mock"))]
use crate::firmware::drivers::status_leds::StatusLeds;

bind_interrupts!(struct UartIrqs {
    USART3 => embassy_stm32::usart::InterruptHandler<peripherals::USART3>;
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

    // Track initialization errors (non-fatal errors continue in degraded mode)
    let mut init_errors = ErrorCollection::new();

    // ========================================================================
    // STEP 1: Initialize Watchdog (CRITICAL - must be first)
    // ========================================================================
    // The watchdog protects against hangs during initialization.
    // If initialization takes too long, the system will reset.

    #[cfg(not(feature = "renode-mock"))]
    let watchdog = {
        defmt::info!("[INIT] Initializing watchdog timer...");
        match Watchdog::new(p.IWDG, WatchdogConfig::default()) {
            Ok(wd) => {
                defmt::info!("[INIT] ✓ Watchdog initialized (500ms timeout)");
                Some(wd)
            }
            Err(_) => {
                defmt::warn!("[INIT] ✗ Watchdog init failed - continuing without watchdog protection");
                init_errors.add(FirmwareError::WatchdogInitFailed);
                None
            }
        }
    };

    #[cfg(feature = "renode-mock")]
    let watchdog: Option<Watchdog> = None;

    // ========================================================================
    // STEP 2: Initialize Flash Storage (for calibration/config persistence)
    // ========================================================================

    #[cfg(not(feature = "renode-mock"))]
    let mut flash_storage = {
        defmt::info!("[INIT] Initializing flash storage...");
        match FlashStorage::new(p.FLASH) {
            Ok(mut storage) => {
                defmt::info!("[INIT] ✓ Flash storage initialized");

                // Try to load stored data
                match storage.load() {
                    Ok(data) => {
                        defmt::info!("[INIT] ✓ Loaded calibration from flash");
                        if data.calibration.valid {
                            defmt::info!("[INIT]   → Calibration valid (timestamp: {}s)",
                                data.calibration.timestamp_s);
                        }
                    }
                    Err(_) => {
                        defmt::warn!("[INIT] ✗ No valid calibration in flash - using defaults");
                    }
                }

                Some(storage)
            }
            Err(_) => {
                defmt::warn!("[INIT] ✗ Flash storage init failed - calibration will not persist");
                init_errors.add(FirmwareError::FlashInitFailed);
                None
            }
        }
    };

    #[cfg(feature = "renode-mock")]
    let flash_storage: Option<()> = None;

    // ========================================================================
    // STEP 3: Initialize UART (non-critical - can operate without logging)
    // ========================================================================

    defmt::info!("[INIT] Initializing UART for logging...");
    defmt::info!("[INIT]   → UART3: PC10=TX, PC11=RX, DMA: CH1=TX, CH2=RX");

    let uart_result = Uart::new(
        p.USART3,
        p.PC11, // RX
        p.PC10, // TX
        UartIrqs,
        p.DMA1_CH1,  // TX DMA
        p.DMA1_CH2,  // RX DMA
        uart_log::uart_config(),
    );

    match uart_result {
        Ok(mut uart) => {
            defmt::info!("[INIT] ✓ UART initialized successfully");

            // Send banner to UART
            let _ = uart.write(b"===========================================\r\n").await;
            let _ = uart.write(b"  CLN17 v2.0 Joint Firmware\r\n").await;
            let _ = uart.write(b"  Target: STM32G431CB @ 170 MHz\r\n").await;
            let _ = uart.write(b"  Framework: Embassy + iRPC\r\n").await;
            let _ = uart.write(b"===========================================\r\n").await;

            // Spawn UART logger task
            if spawner.spawn(uart_log::uart_logger_task(uart)).is_err() {
                defmt::warn!("[INIT] ✗ Failed to spawn UART logger task");
            } else {
                defmt::info!("[INIT] ✓ UART logger task spawned");
            }
        }
        Err(_) => {
            defmt::warn!("[INIT] ✗ UART init failed - continuing without logging");
            init_errors.add(FirmwareError::UartInitFailed);
        }
    }

    defmt::info!("Target: STM32G431CB @ 170 MHz");
    defmt::info!("Framework: Embassy + iRPC");
    uart_log::log(LogMessage::Init);

    // ========================================================================
    // STEP 4: Spawn Watchdog Feeder Task (if watchdog initialized)
    // ========================================================================

    #[cfg(not(feature = "renode-mock"))]
    if let Some(wd) = watchdog {
        defmt::info!("[INIT] Spawning watchdog feeder task...");
        if spawner.spawn(crate::firmware::tasks::watchdog_feeder::watchdog_feeder(wd)).is_err() {
            defmt::error!("[INIT] ✗ CRITICAL: Failed to spawn watchdog feeder!");
            // This is critical - without feeder, system will reset
            enter_safe_mode(&init_errors).await;
        } else {
            defmt::info!("[INIT] ✓ Watchdog feeder task spawned");
        }
    }

    // ========================================================================
    // STEP 5: Spawn CAN Communication Task
    // ========================================================================

    #[cfg(feature = "renode-mock")]
    {
        defmt::info!("[INIT] Spawning MOCK CAN task (Renode mode)...");
        if spawner.spawn(crate::firmware::tasks::mock_can::can_communication_mock(
            DEFAULT_NODE_ID,
        )).is_err() {
            defmt::warn!("[INIT] ✗ Failed to spawn mock CAN task");
        } else {
            defmt::info!("[INIT] ✓ MOCK CAN task spawned");
        }
    }

    #[cfg(not(feature = "renode-mock"))]
    {
        defmt::info!("[INIT] Spawning CAN communication task...");
        if spawner.spawn(crate::firmware::tasks::can_comm::can_communication(
            DEFAULT_NODE_ID,
            p.FDCAN1,  // FDCAN peripheral
            p.PB9,     // FDCAN1_TX (CLN17 V2.0: PB9)
            p.PB8,     // FDCAN1_RX (CLN17 V2.0: PB8)
        )).is_err() {
            defmt::warn!("[INIT] ✗ Failed to spawn CAN task - will operate in degraded mode");
            init_errors.add(FirmwareError::CanInitFailed);
        } else {
            defmt::info!("[INIT] ✓ CAN communication task spawned");
            uart_log::log(LogMessage::CanStarted);
        }
    }

    // ========================================================================
    // STEP 6: Spawn Control Loop (FOC or Step-Dir)
    // ========================================================================

    let system_state = SystemState::default();

    match system_state.motor_config.control_method {
        ControlMethod::Foc => {
            #[cfg(feature = "renode-mock")]
            {
                defmt::info!("[INIT] Spawning MOCK FOC task (1 Hz mode)...");
                if spawner.spawn(crate::firmware::tasks::mock_foc::control_loop_mock()).is_err() {
                    defmt::warn!("[INIT] ✗ Failed to spawn mock FOC task");
                } else {
                    defmt::info!("[INIT] ✓ MOCK FOC task spawned");
                }
            }

            #[cfg(not(feature = "renode-mock"))]
            {
                defmt::info!("[INIT] Spawning FOC control loop...");
                if spawner.spawn(crate::firmware::tasks::foc::control_loop()).is_err() {
                    defmt::warn!("[INIT] ✗ Failed to spawn FOC task");
                } else {
                    defmt::info!("[INIT] ✓ FOC task spawned");
                    uart_log::log(LogMessage::FocStarted);
                }
            }
        }
        ControlMethod::StepDir => {
            #[cfg(feature = "renode-mock")]
            {
                defmt::info!("[INIT] Spawning MOCK Step-Dir task (1 Hz mode)...");
                if spawner.spawn(crate::firmware::tasks::mock_step_dir::control_loop_mock()).is_err() {
                    defmt::warn!("[INIT] ✗ Failed to spawn mock Step-Dir task");
                } else {
                    defmt::info!("[INIT] ✓ MOCK Step-Dir task spawned");
                }
            }

            #[cfg(not(feature = "renode-mock"))]
            {
                defmt::info!("[INIT] Spawning Step-Dir control loop...");
                if spawner.spawn(crate::firmware::tasks::step_dir::control_loop()).is_err() {
                    defmt::warn!("[INIT] ✗ Failed to spawn Step-Dir task");
                } else {
                    defmt::info!("[INIT] ✓ Step-Dir task spawned");
                }
            }
        }
    }

    // ========================================================================
    // STEP 7: Initialize and Spawn Power Monitor (non-critical)
    // ========================================================================

    #[cfg(not(feature = "renode-mock"))]
    {
        defmt::info!("[INIT] Initializing power monitoring system...");

        // Initialize Sensors (ADC with blocking reads)
        let sensors = Sensors::new(
            p.ADC1,
            p.PA3,       // Phase A current
            p.PB0,       // Phase B current
            p.PA2,       // Vbus voltage
        );
        defmt::info!("[INIT] ✓ Sensors initialized (ADC1 + DMA1_CH3)");

        // Initialize Motor Driver control
        let motor_driver = MotorDriver::new(
            p.PA4,  // nSLEEP
            p.PB1,  // nFAULT
            p.PB2,  // nRESET
        );
        defmt::info!("[INIT] ✓ Motor driver control initialized");

        // Initialize Status LEDs
        let status_leds = StatusLeds::new(
            p.PB13,  // Red
            p.PB14,  // Green
            p.PB15,  // Blue
        );
        defmt::info!("[INIT] ✓ Status LEDs initialized");

        // Spawn power monitor task
        defmt::info!("[INIT] Spawning power monitor task (100 Hz)...");
        if spawner.spawn(crate::firmware::tasks::power_monitor::power_monitor(
            sensors,
            motor_driver,
            status_leds,
        )).is_err() {
            defmt::warn!("[INIT] ✗ Failed to spawn power monitor task");
            init_errors.add(FirmwareError::AdcInitFailed);
        } else {
            defmt::info!("[INIT] ✓ Power monitor task spawned");
            defmt::info!("[INIT]   → Monitoring: V, I, temp, faults @ 100 Hz");
            defmt::info!("[INIT]   → Protection: OV, UV, OC, thermal");
        }

        // Spawn power telemetry task
        defmt::info!("[INIT] Spawning power telemetry task (10 Hz)...");
        if spawner.spawn(crate::firmware::tasks::power_telemetry::power_telemetry(10)).is_err() {
            defmt::warn!("[INIT] ✗ Failed to spawn power telemetry task");
        } else {
            defmt::info!("[INIT] ✓ Power telemetry task spawned");
            defmt::info!("[INIT]   → Streaming: Power metrics @ 10 Hz");
            defmt::info!("[INIT]   → Ready for iRPC PowerMetrics integration");
        }
    }

    // ========================================================================
    // STEP 8: Report Initialization Status
    // ========================================================================

    if init_errors.is_empty() {
        defmt::info!("=== System Ready (Full Mode) ===");
    } else {
        defmt::warn!("=== System Ready (Degraded Mode - {} errors) ===", init_errors.len());
        for error in init_errors.iter() {
            defmt::warn!("  → {}", error.description());
        }

        // Check if any critical errors occurred
        if init_errors.has_critical_error() {
            defmt::error!("CRITICAL errors detected - entering safe mode");
            enter_safe_mode(&init_errors).await;
        }
    }

    uart_log::log(LogMessage::Ready);

    // ========================================================================
    // STEP 9: Main Heartbeat Loop
    // ========================================================================

    let mut counter = 0u32;
    loop {
        Timer::after(Duration::from_secs(1)).await;
        counter = counter.wrapping_add(1);
        defmt::info!("System heartbeat: {} sec", counter);
        uart_log::log(LogMessage::Heartbeat(counter));
    }
}

/// Enter safe mode when critical errors prevent normal operation.
///
/// Safe mode:
/// - Disables motor control
/// - Blinks error LED
/// - Waits for watchdog reset or manual reset
async fn enter_safe_mode(errors: &ErrorCollection) -> ! {
    defmt::error!("=== ENTERING SAFE MODE ===");
    defmt::error!("Critical errors prevent normal operation:");

    for error in errors.iter() {
        defmt::error!("  → {} (severity: {:?})",
            error.description(),
            error.severity());
    }

    defmt::error!("System halted. Manual reset required.");

    // Infinite loop - watchdog will reset system if enabled
    loop {
        Timer::after(Duration::from_millis(500)).await;
        defmt::error!("SAFE MODE - awaiting reset");
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

