use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::pio::InterruptHandler;
use embassy_rp::{
    Peri, bind_interrupts,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO1},
    pio::Pio,
};
use embassy_time::{Duration, Timer};
use fixed::types::U24F8;
use static_cell::StaticCell;

use crate::network::socket::tcp_client_task;

bind_interrupts!(struct Irqs {
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
});

// WiFi credentials from .env file (generated at build time)
include!(concat!(env!("OUT_DIR"), "/wifi_config.rs"));

/// A struct to group all WiFi-related peripherals for easier initialization.
/// This reduces the number of arguments needed when initializing the WiFi module.
pub struct WiFiConfig {
    pub pwr_pin: Peri<'static, PIN_23>,
    pub cs_pin: Peri<'static, PIN_25>,
    pub pio: Peri<'static, PIO1>,
    pub clk_pin: Peri<'static, PIN_24>,
    pub dio_pin: Peri<'static, PIN_29>,
    pub dma: Peri<'static, DMA_CH0>,
}

#[embassy_executor::task]
pub async fn init_and_run_wifi(spawner: Spawner, wifi_config: WiFiConfig) {
    let (control, net_device) = init_cyw43(
        spawner,
        wifi_config.pwr_pin,
        wifi_config.cs_pin,
        wifi_config.pio,
        wifi_config.clk_pin,
        wifi_config.dio_pin,
        wifi_config.dma,
    )
    .await;
    spawner
        .spawn(wifi_task(spawner, control, net_device))
        .unwrap();
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

/// Initialize CYW43 WiFi chip and return the control interface and network device
async fn init_cyw43(
    spawner: Spawner,
    pwr_pin: Peri<'static, PIN_23>,
    cs_pin: Peri<'static, PIN_25>,
    pio: Peri<'static, PIO1>,
    clk_pin: Peri<'static, PIN_24>,
    dio_pin: Peri<'static, PIN_29>,
    dma: Peri<'static, DMA_CH0>,
) -> (cyw43::Control<'static>, cyw43::NetDriver<'static>) {
    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");
    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    // Clock divider for RP2040 compatibility
    const RM2_CLOCK_DIVIDER: U24F8 = U24F8::from_bits(32 << 8);

    let pwr = Output::new(pwr_pin, Level::Low);
    let cs = Output::new(cs_pin, Level::High);
    let mut pio_instance = Pio::new(pio, Irqs);
    let spi = PioSpi::new(
        &mut pio_instance.common,
        pio_instance.sm0,
        // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
        // See: https://github.com/embassy-rs/embassy/issues/3960.
        RM2_CLOCK_DIVIDER,
        pio_instance.irq0,
        cs,
        clk_pin,
        dio_pin,
        dma,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    // Spawn the CYW43 runner task
    unwrap!(spawner.spawn(cyw43_task(runner)));

    // Initialize the control interface
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    info!("CYW43 WiFi chip initialized successfully");
    (control, net_device)
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO1, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn wifi_task(
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
    static STACK: StaticCell<Stack> = StaticCell::new();
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
