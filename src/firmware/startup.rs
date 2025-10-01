use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_time::{Duration, Timer};

pub async fn run(_spawner: Spawner) -> ! {
    let _p = embassy_stm32::init(Config::default());

    defmt::info!("joint_firmware boot");

    loop {
        Timer::after(Duration::from_secs(1)).await;
        defmt::info!("heartbeat");
    }
}
