// Core 0 is the main core and is responsible for interfcing with bluetooth and passing commands down to core 1

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::tasks::socket::tcp_client_task;

// WiFi credentials from .env file (generated at build time)
include!(concat!(env!("OUT_DIR"), "/wifi_config.rs"));

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn core0_task(
    spawner: Spawner,
    mut control: cyw43::Control<'static>,
    net_device: cyw43::NetDriver<'static>,
) {
    info!("Core 0 starting - initializing WiFi...");

    info!("Connecting to WiFi network: {}", WIFI_SSID);
    let result = control
        .join(WIFI_SSID, cyw43::JoinOptions::new(WIFI_PASSWORD.as_bytes()))
        .await;
    result.unwrap();

    // Initialize embassy-net stack
    static STACK: StaticCell<Stack<'static>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

    let config = Config::dhcpv4(Default::default());
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<3>::new()),
        embassy_rp::clocks::RoscRng.next_u64(),
    );
    let stack = &*STACK.init(stack);

    // Spawn the network task
    spawner.spawn(net_task(runner)).unwrap();

    info!("Network stack initialized, waiting for DHCP...");

    // Wait for DHCP to assign an IP address
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("Network configured:");
        info!("  IP: {}", config.address.address());
        info!("  Gateway: {:?}", config.gateway);
        info!("  DNS: {:?}", config.dns_servers);
    }

    let delay = Duration::from_millis(250);
    for _ in 0..3 {
        // Blink LED to indicate successful network connection
        control.gpio_set(0, true).await;
        Timer::after(delay).await;
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }

    // Spawn TCP client task
    info!("Starting TCP client task...");
    spawner.spawn(tcp_client_task(spawner, stack)).unwrap();

    // Main loop - keep WiFi alive and monitor connection
    loop {
        Timer::after(Duration::from_secs(30)).await;

        // Keep WiFi connection alive - just log periodically
        info!("WiFi monitoring - connection active");
    }
}
