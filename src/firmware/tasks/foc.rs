use embassy_time::Timer;

pub async fn control_loop() {
    loop {
        Timer::after_secs(1).await;
    }
}
