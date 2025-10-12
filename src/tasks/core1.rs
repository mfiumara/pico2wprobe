use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    peripherals::PIO0,
    pio::{Pio, StateMachine},
};
use embassy_time::Timer;

use crate::probe::pio::setup_pio_task_sm1;
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
