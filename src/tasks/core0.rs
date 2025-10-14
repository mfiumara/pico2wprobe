// Core 0 is the main core and is responsible for interfcing with bluetooth and passing commands down to core 1

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

// WiFi credentials from .env file (generated at build time)
include!(concat!(env!("OUT_DIR"), "/wifi_config.rs"));

#[embassy_executor::task]
pub async fn core0_task(_spawner: Spawner, mut control: cyw43::Control<'static>) {
    info!("Core 0 starting - initializing WiFi...");

    info!("Connecting to WiFi network: {}", WIFI_SSID);
    let result = control
        .join(WIFI_SSID, cyw43::JoinOptions::new(WIFI_PASSWORD.as_bytes()))
        .await;
    result.unwrap();

    let delay = Duration::from_millis(250);
    for _ in 0..3 {
        // TODO: Replace this LED blinking with WiFi scanning logic
        info!("led on!");
        control.gpio_set(0, true).await;
        Timer::after(delay).await;

        info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }

    // Main loop - focus on WiFi operations
    loop {
        Timer::after(delay).await;
    }
}
