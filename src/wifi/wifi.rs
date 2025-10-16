use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO1},
    pio::Pio,
};
use fixed::types::U24F8;
use static_cell::StaticCell;

use crate::shared::Irqs1;

// Clock divider for RP2040 compatibility
const RM2_CLOCK_DIVIDER: U24F8 = U24F8::from_bits(32 << 8);

// WiFi credentials from .env file (generated at build time)
include!(concat!(env!("OUT_DIR"), "/wifi_config.rs"));

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO1, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

/// Initialize CYW43 WiFi chip and return the control interface and network device
pub async fn init_cyw43(
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

    let pwr = Output::new(pwr_pin, Level::Low);
    let cs = Output::new(cs_pin, Level::High);
    let mut pio_instance = Pio::new(pio, Irqs1);
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
