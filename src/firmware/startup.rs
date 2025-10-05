use embassy_executor::Spawner;
use embassy_stm32::Config;

use crate::firmware::clocks;

pub async fn run(spawner: Spawner) -> ! {
    defmt::info!("========================================");
    defmt::info!("=== STARTUP BEGIN ===");
    defmt::info!("========================================");
    
    // Initialize STM32 with custom RCC config
    defmt::info!("[TRACE] Creating Embassy config...");
    let mut config = Config::default();
    config.rcc = clocks::rcc_config();
    
    defmt::info!("[TRACE] Calling embassy_stm32::init()...");
    let p = embassy_stm32::init(config);
    defmt::info!("[TRACE] ✓ Embassy initialized!");
    
    // Log clock frequencies
    defmt::info!("[TRACE] Checking clocks...");
    clocks::log_clocks(&p.RCC);
    
    defmt::info!("=== CLN17 v2.0 Joint Firmware ===");
    defmt::info!("Target: STM32G431CB @ 170 MHz");
    
    // Hand off to system initialization
    defmt::info!("[TRACE] Handing off to system::initialize()...");
    crate::firmware::system::initialize(spawner, p).await
}
