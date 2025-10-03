use embassy_executor::Spawner;
use embassy_stm32::Config;

use crate::firmware::clocks;

pub async fn run(spawner: Spawner) -> ! {
    // Initialize STM32 with custom RCC config
    let mut config = Config::default();
    config.rcc = clocks::rcc_config();
    let p = embassy_stm32::init(config);
    
    // Log clock frequencies
    clocks::log_clocks(&p.RCC);
    
    defmt::info!("=== CLN17 v2.0 Joint Firmware ===");
    defmt::info!("Target: STM32G431CB @ 170 MHz");
    
    // Hand off to system initialization
    crate::firmware::system::initialize(spawner, p).await
}
