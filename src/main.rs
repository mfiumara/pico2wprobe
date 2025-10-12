#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::Executor;
use embassy_rp::multicore::{Stack, spawn_core1};
use panic_probe as _;
use static_cell::StaticCell;

mod probe;
mod shared;
mod tasks;
use tasks::{core0_task, core1_task};

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

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                spawner.spawn(core1_task(spawner)).unwrap();
            });
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner.spawn(core0_task(spawner)).unwrap();
    });
}
