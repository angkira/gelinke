#![no_std]
#![no_main]

mod firmware;

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_time::Timer;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    firmware::startup::run(spawner).await
}
