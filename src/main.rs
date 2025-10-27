#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::Executor;
use embassy_rp::multicore::{spawn_core1, Stack};
use panic_probe as _;
use pico2wprobe::network::wifi::{WiFiConfig, init_and_run_wifi};
use pico2wprobe::usb::run_and_init_usb;
use static_cell::StaticCell;

#[allow(dead_code)]
static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
#[allow(dead_code)]
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

// Program metadata for `picotool info`.
// This isn't needed, but it's recommended to have these minimal entries.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"CMSIS-DAP Probe"),
    embassy_rp::binary_info::rp_program_description!(
        c"This program implements the cmsis-dap probe from rpi's debugprobe repo."
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    // This will spawn the USB task on core1, which will listen to DAP commands
    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                spawner
                    .spawn(run_and_init_usb(spawner, p.USB))
                    .unwrap();
            });
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        // Initialize WiFi and pass control to core0_task
        let wifi_config = WiFiConfig {
            pwr_pin: p.PIN_23,
            cs_pin: p.PIN_25,
            pio: p.PIO1,
            clk_pin: p.PIN_24,
            dio_pin: p.PIN_29,
            dma: p.DMA_CH0,
        };
        spawner
            .spawn(init_and_run_wifi(spawner, wifi_config))
            .unwrap();
    });
}
