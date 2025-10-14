// Core 0 is the main core and is responsible for interfcing with bluetooth and passing commands down to core 1

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0},
    pio::Pio,
};
use embassy_time::{Duration, Timer};
use fixed::types::U24F8;
use static_cell::StaticCell;

use crate::shared::Irqs;

// Clock divider for RP2040 compatibility
const RM2_CLOCK_DIVIDER: U24F8 = U24F8::from_bits(32 << 8);

// WiFi credentials from .env file (generated at build time)
include!(concat!(env!("OUT_DIR"), "/wifi_config.rs"));

/// Initialize CYW43 WiFi chip and return the control interface
async fn init_cyw43(spawner: Spawner) -> cyw43::Control<'static> {
    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");
    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000o as *const u8, 4752) };

    let p = embassy_rp::init(Default::default());

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        // SPI communication won't work if the speed is too high, so we use a divider larger than `DEFAULT_CLOCK_DIVIDER`.
        // See: https://github.com/embassy-rs/embassy/issues/3960.
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    // Spawn the CYW43 runner task
    unwrap!(spawner.spawn(cyw43_task(runner)));

    // Initialize the control interface
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    info!("CYW43 WiFi chip initialized successfully");
    control
}

#[embassy_executor::task]
pub async fn core0_task(spawner: Spawner) {
    info!("Core 0 starting - initializing WiFi...");

    // Initialize CYW43 WiFi chip
    let mut control = init_cyw43(spawner).await;

    info!("Core 0 ready - starting main loop");

    info!("Connecting to WiFi network: {}", WIFI_SSID);
    let result = control
        .join(WIFI_SSID, cyw43::JoinOptions::new(WIFI_PASSWORD))
        .await;
    result.unwrap();

    // Main loop - focus on WiFi operations
    let delay = Duration::from_millis(250);
    loop {
        // TODO: Replace this LED blinking with WiFi scanning logic
        info!("led on!");
        control.gpio_set(0, true).await;
        Timer::after(delay).await;

        info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;

        // TODO: Add WiFi scanning here:
        // - control.start_ap_scan(...)
        // - control.get_scan_results(...)
        // - Send results to core1 via channel
    }
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}
