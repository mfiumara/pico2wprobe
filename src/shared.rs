use embassy_rp::{bind_interrupts, peripherals::{PIO0, PIO1}, pio::InterruptHandler};

bind_interrupts!(pub struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

bind_interrupts!(pub struct Irqs1 {
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
});
