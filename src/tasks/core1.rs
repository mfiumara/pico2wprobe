use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    Peri, bind_interrupts,
    peripherals::{PIN_2, PIN_3, PIO0},
    pio::{InterruptHandler, Pio, StateMachine},
};
use embassy_time::Timer;

use crate::probe::pio::{pio_simple_test, setup_pio_task_sm1, swd_read_idcode};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
pub async fn core1_task(
    spawner: Spawner,
    pio_peripheral: Peri<'static, PIO0>,
    swdio_pin: Peri<'static, PIN_3>,
    swclk_pin: Peri<'static, PIN_2>,
) {
    info!("Hello from core 1");

    let mut pio = Pio::new(pio_peripheral, Irqs);

    setup_pio_task_sm1(&mut pio.common, &mut pio.sm1, swdio_pin, swclk_pin);
    unwrap!(spawner.spawn(pio_task_sm1(pio.sm1)));
    loop {
        Timer::after_millis(400).await;
    }
}

#[embassy_executor::task]
async fn pio_task_sm1(mut sm: StateMachine<'static, PIO0, 1>) {
    sm.set_enable(true);
    info!("PIO State Machine 1 enabled for SWD");

    // Wait a bit for things to settle
    Timer::after_millis(100).await;

    // First try the simple PIO test
    match pio_simple_test(&mut sm).await {
        Ok(()) => {
            info!("PIO simple test passed!");

            // Now try IDCODE read
            match swd_read_idcode(&mut sm).await {
                Ok(idcode) => {
                    info!("Successfully read target IDCODE: 0x{:08X}", idcode);
                }
                Err(e) => {
                    error!("Failed to read IDCODE: {}", e);
                }
            }
        }
        Err(e) => {
            error!("PIO simple test failed: {}", e);
            error!("Check PIO configuration and pin connections");
        }
    }

    // Keep the task alive and periodically try to read IDCODE
    loop {
        Timer::after_millis(5000).await; // Every 5 seconds

        match swd_read_idcode(&mut sm).await {
            Ok(idcode) => {
                info!("IDCODE check: 0x{:08X}", idcode);
            }
            Err(e) => {
                warn!("IDCODE read failed: {}", e);
            }
        }
    }
}
