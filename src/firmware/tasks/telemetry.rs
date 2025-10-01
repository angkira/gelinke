use embassy_time::Timer;

use crate::firmware::config;

pub async fn usb_telemetry() {
    loop {
        Timer::after_secs(config::HEARTBEAT_PERIOD_SECS).await;
    }
}
