// Basic probe tests without PIO conflicts

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

// Import embassy-rp to provide critical section implementation and interrupt vectors
use embassy_rp::{self as _};

#[defmt_test::tests]
mod tests {
    use pico2wprobe::tasks::core1::Irqs;

    #[init]
    fn init() {}

    // Basic test to verify the test framework works
    #[test]
    fn basic_test() {
        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);
    }
}
