#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    Peri,
    multicore::{Stack, spawn_core1},
    peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO1, USB},
};
use panic_probe as _;
use static_cell::StaticCell;

/// Struct to group WiFi-related peripherals and reduce function argument count
struct WiFiPeripherals {
    pwr_pin: Peri<'static, PIN_23>,
    cs_pin: Peri<'static, PIN_25>,
    pio: Peri<'static, PIO1>,
    clk_pin: Peri<'static, PIN_24>,
    dio_pin: Peri<'static, PIN_29>,
    dma: Peri<'static, DMA_CH0>,
}

mod probe;
mod tasks;
mod usb;
mod wifi;
use tasks::{core0_task, core1_task};
use wifi::init_cyw43;

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

// Program metadata for `picotool info`.
// This isn't needed, but it's recommended to have these minimal entries.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Blinky Example"),
    embassy_rp::binary_info::rp_program_description!(
        c"This example tests the RP Pico 2 W's onboard LED, connected to GPIO 0 of the cyw43 \
        (WiFi chip) via PIO 0 over the SPI bus."
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

#[embassy_executor::task]
async fn init_and_run_core0(
    spawner: Spawner,
    wifi_peripherals: WiFiPeripherals,
    usb: Peri<'static, USB>,
) {
    let (control, net_device) = init_cyw43(
        spawner,
        wifi_peripherals.pwr_pin,
        wifi_peripherals.cs_pin,
        wifi_peripherals.pio,
        wifi_peripherals.clk_pin,
        wifi_peripherals.dio_pin,
        wifi_peripherals.dma,
    )
    .await;
    spawner
        .spawn(core0_task(spawner, control, net_device, usb))
        .unwrap();
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                spawner
                    .spawn(core1_task(spawner, p.PIO0, p.PIN_3, p.PIN_2))
                    .unwrap();
            });
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        // Initialize WiFi and pass control to core0_task
        let wifi_peripherals = WiFiPeripherals {
            pwr_pin: p.PIN_23,
            cs_pin: p.PIN_25,
            pio: p.PIO1,
            clk_pin: p.PIN_24,
            dio_pin: p.PIN_29,
            dma: p.DMA_CH0,
        };
        spawner
            .spawn(init_and_run_core0(spawner, wifi_peripherals, p.USB))
            .unwrap();
    });
}
