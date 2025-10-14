// RP2350 has 4 PIO state machines. Each state machine has:
//  Two 32-bit shift registers (either direction, any shift count)
//  Two 32-bit scratch registers
//  4 × 32-bit bus FIFO in each direction (TX/RX), reconfigurable as 8 × 32 in a single direction
//  Fractional clock divider (16 integer, 8 fractional bits)
//  Flexible GPIO mapping
//  DMA interface (sustained throughput up to 1 word per clock from system DMA)
//  IRQ flag set/clear/status

use embassy_rp::pio::program::pio_file;
use embassy_rp::{
    Peri,
    peripherals::PIO0,
    pio::{Common, Config, PioPin, StateMachine},
};
use fixed::traits::ToFixed;
use fixed_macro::types::U56F8;

pub fn setup_pio_task_sm1<'a>(
    pio: &mut Common<'a, PIO0>,
    sm: &mut StateMachine<'a, PIO0, 1>,
    swdio_pin: Peri<'a, impl PioPin>,
    swclk_pin: Peri<'a, impl PioPin>,
) {
    // Load the SWD probe PIO program
    let prg = pio_file!("src/probe.pio");

    let mut cfg = Config::default();

    // Configure pins
    let swdio_pio_pin = pio.make_pio_pin(swdio_pin);
    let swclk_pio_pin = pio.make_pio_pin(swclk_pin);

    // Load program with sideset pins (SWCLK for sideset)
    cfg.use_program(&pio.load_program(&prg.program), &[&swclk_pio_pin]);

    // Configure SWDIO pin (data)
    cfg.set_out_pins(&[&swdio_pio_pin]);
    cfg.set_set_pins(&[&swdio_pio_pin]);
    cfg.set_in_pins(&[&swdio_pio_pin]);

    // Configure shifts for SWD protocol (LSB first)
    cfg.shift_out.direction = embassy_rp::pio::ShiftDirection::Right;
    cfg.shift_out.auto_fill = false;
    cfg.shift_in.direction = embassy_rp::pio::ShiftDirection::Right;
    cfg.shift_in.auto_fill = false;

    // Set clock divider for appropriate SWD timing
    cfg.clock_divider = (U56F8!(4.0)).to_fixed(); // 125MHz / 4 = ~31.25MHz

    sm.set_config(&cfg);
}
