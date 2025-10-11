use embassy_rp::{bind_interrupts, peripherals::PIO0, pio::InterruptHandler};

bind_interrupts!(pub struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});
