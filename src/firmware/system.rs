use embassy_executor::Spawner;
use embassy_stm32::Peripherals;
use embassy_time::{Duration, Timer};

use crate::firmware::config::{MotorConfig, EncoderConfig};
use crate::firmware::control::observer::{LuenbergerObserver, ObserverConfig};
use crate::firmware::control::position::{PositionController, PositionConfig};
use crate::firmware::control::velocity::{VelocityController, VelocityConfig};
use crate::firmware::drivers::can::{CanDriver, DEFAULT_NODE_ID};

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
    defmt::info!("=== Joint Firmware Initialization ===");
    
    // Initialize CAN driver
    let _can = CanDriver::new(p, DEFAULT_NODE_ID);
    defmt::info!("CAN-FD: initialized");
    
    // TODO: Initialize other drivers (PWM, ADC, Encoder)
    // This requires proper peripheral splitting
    
    // Spawn CAN communication task
    spawner.spawn(crate::firmware::tasks::can_comm::can_communication(DEFAULT_NODE_ID)).ok();
    
    // Spawn FOC control loop
    spawner.spawn(crate::firmware::tasks::foc::control_loop()).ok();
    
    defmt::info!("=== System Ready ===");
    
    // Main heartbeat
    let mut counter = 0u32;
    loop {
        Timer::after(Duration::from_secs(1)).await;
        counter = counter.wrapping_add(1);
        defmt::info!("System heartbeat: {} sec", counter);
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

