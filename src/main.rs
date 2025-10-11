#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Executor;
use embassy_rp::{
    bind_interrupts,
    multicore::{Stack, spawn_core1},
    peripherals::PIO0,
    pio::InterruptHandler,
};
use embassy_time::Timer;
use panic_probe as _;
use static_cell::StaticCell;

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

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                spawner.spawn(core1_task()).unwrap();
            });
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner.spawn(core0_task()).unwrap();
    });
}

#[embassy_executor::task]
async fn core0_task() {
    info!("Hello from core 0");
    loop {
        // CHANNEL.send(LedState::On).await;
        // Timer::after_millis(100).await;
        // CHANNEL.send(LedState::Off).await;
        Timer::after_millis(400).await;
    }
}

#[embassy_executor::task]
async fn core1_task() {
    info!("Hello from core 1");
    loop {
        // match CHANNEL.receive().await {
        //     LedState::On => led.set_high(),
        //     LedState::Off => led.set_low(),
        // }
        Timer::after_millis(400).await;
    }
}
