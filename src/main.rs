//! This example tests the RP Pico 2 W onboard LED.
//!
//! It does not work with the RP Pico 2 board. See `blinky.rs`.

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
// use embassy_futures::join::join;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::program::pio_file;
use embassy_rp::pio::{Common, Config, InterruptHandler, Pio, PioPin, StateMachine};
use embassy_rp::{Peri, bind_interrupts};
use embassy_time::{Duration, Timer};
use fixed_macro::types::U56F8;
use static_cell::StaticCell;

use defmt::info;
use fixed::traits::ToFixed;

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

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

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
    unwrap!(spawner.spawn(cyw43_task(runner)));

    setup_pio_task_sm1(&mut pio.common, &mut pio.sm1, p.PIN_0);
    unwrap!(spawner.spawn(pio_task_sm1(pio.sm1)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let delay = Duration::from_millis(250);
    loop {
        info!("led on!");
        control.gpio_set(0, true).await;
        Timer::after(delay).await;

        info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}

fn setup_pio_task_sm1<'a>(
    pio: &mut Common<'a, PIO0>,
    sm: &mut StateMachine<'a, PIO0, 1>,
    pin: Peri<'a, impl PioPin>,
) {
    // Send data serially to pin
    let prg = pio_file!("src/probe.pio");

    let mut cfg = Config::default();
    cfg.use_program(&pio.load_program(&prg.program), &[]);
    let out_pin = pio.make_pio_pin(pin);
    cfg.set_out_pins(&[&out_pin]);
    cfg.set_set_pins(&[&out_pin]);
    cfg.clock_divider = (U56F8!(125_000_000) / 20 / 200).to_fixed();
    cfg.shift_out.auto_fill = true;
    sm.set_config(&cfg);
}

#[embassy_executor::task]
async fn pio_task_sm1(mut sm: StateMachine<'static, PIO0, 1>) {
    sm.set_enable(true);

    loop {
        // let (rx, tx) = sm.rx_tx();
        // join(
        //     tx.dma_push(dma_out_ref.reborrow(), &dout, false),
        //     rx.dma_pull(dma_in_ref.reborrow(), &mut din, false),
        // )
        // .await;
        sm.set_enable(true);

        let mut v = 0x0f0caffa;
        sm.tx().wait_push(v).await;
        v ^= 0xffff;
        info!("Pushed {:032b} to FIFO", v);
    }
}
