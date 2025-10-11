use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    peripherals::PIO0,
    pio::{Common, Config, Pio, PioPin, StateMachine, program::pio_file},
};
use embassy_time::Timer;
use fixed::traits::ToFixed;
use fixed_macro::types::U56F8;

use crate::shared::Irqs;

#[embassy_executor::task]
pub async fn core1_task(spawner: Spawner) {
    info!("Hello from core 1");

    let p = embassy_rp::init(Default::default());
    let mut pio = Pio::new(p.PIO0, Irqs);

    setup_pio_task_sm1(&mut pio.common, &mut pio.sm1, p.PIN_0);
    unwrap!(spawner.spawn(pio_task_sm1(pio.sm1)));
    loop {
        // match CHANNEL.receive().await {
        //     LedState::On => led.set_high(),
        //     LedState::Off => led.set_low(),
        // }
        Timer::after_millis(400).await;
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
