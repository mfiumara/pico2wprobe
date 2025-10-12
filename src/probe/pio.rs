// RP2350 has 4 PIO state machines. Each state machine has:
//  Two 32-bit shift registers (either direction, any shift count)
//  Two 32-bit scratch registers
//  4 × 32-bit bus FIFO in each direction (TX/RX), reconfigurable as 8 × 32 in a single direction
//  Fractional clock divider (16 integer, 8 fractional bits)
//  Flexible GPIO mapping
//  DMA interface (sustained throughput up to 1 word per clock from system DMA)
//  IRQ flag set/clear/status

use embassy_rp::{
    peripherals::PIO0,
    pio::{Common, Config, PioPin, StateMachine},
    Peri,
};
use embassy_rp::pio::program::pio_file;
use fixed::traits::ToFixed;
use fixed_macro::types::U56F8;

pub fn setup_pio_task_sm1<'a>(
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
    cfg.clock_divider = (U56F8!(125_000_000)).to_fixed();
    cfg.shift_out.auto_fill = true;
    sm.set_config(&cfg);
}
