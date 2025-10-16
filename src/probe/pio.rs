// RP2350 has 4 PIO state machines. Each state machine has:
//  Two 32-bit shift registers (either direction, any shift count)
//  Two 32-bit scratch registers
//  4 × 32-bit bus FIFO in each direction (TX/RX), reconfigurable as 8 × 32 in a single direction
//  Fractional clock divider (16 integer, 8 fractional bits)
//  Flexible GPIO mapping
//  DMA interface (sustained throughput up to 1 word per clock from system DMA)
//  IRQ flag set/clear/status

use defmt::*;
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

/// Read the SWD IDCODE register - simplest SWD operation to test connectivity
pub async fn swd_read_idcode(sm: &mut StateMachine<'_, PIO0, 1>) -> Result<u32, &'static str> {
    info!("Starting SWD IDCODE read");

    // SWD line reset sequence: 50+ high bits followed by 2 low bits
    let line_reset = 0xFFFFFFFF_u32; // 32 high bits
    sm.tx().push(line_reset);
    sm.tx().push(0xFFFF_u32 << 16); // 16 more high bits + 16 low for good measure

    // JTAG-to-SWD switching sequence
    let jtag_to_swd = 0x79E7_u32; // 16-bit switching sequence
    sm.tx().push(jtag_to_swd);

    // Another line reset
    sm.tx().push(0xFFFFFFFF_u32);
    sm.tx().push(0x00_u32); // 8 low bits to end reset

    // SWD packet for IDCODE read:
    // - Start bit: 1
    // - APnDP: 0 (DP access)
    // - RnW: 1 (Read)
    // - A[2:3]: 00 (IDCODE register address)
    // - Parity: 0 (even parity of APnDP + RnW + A[2:3] = 0+1+0+0 = 1, so parity = 0)
    // - Stop: 0
    // - Park: 1
    // Total: 10100001 = 0xA1
    let idcode_request = 0xA1_u32;
    sm.tx().push(idcode_request);

    info!("Sent SWD IDCODE request, waiting for response");

    // Wait for turnaround + ACK (3 bits) + data (32 bits) + parity (1 bit)
    // Total: 36 bits, but we'll read in chunks

    // Read ACK (should be 001 for OK)
    if let Some(ack_data) = sm.rx().try_pull() {
        let ack = ack_data & 0x7; // Bottom 3 bits
        if ack != 0x1 {
            // 001 = OK/VALID
            error!("SWD ACK error: {:03b}", ack);
            return Err("Invalid ACK response");
        }
        info!("SWD ACK OK");
    } else {
        error!("No ACK response received");
        return Err("No ACK response");
    }

    // Read IDCODE data (32 bits)
    if let Some(idcode) = sm.rx().try_pull() {
        info!("SWD IDCODE: 0x{:08X}", idcode);

        // Decode some basic info
        let designer = (idcode >> 1) & 0x7FF;
        let part_no = (idcode >> 12) & 0xFFFF;
        let version = (idcode >> 28) & 0xF;

        info!("  Designer: 0x{:03X}", designer);
        info!("  Part No: 0x{:04X}", part_no);
        info!("  Version: 0x{:X}", version);

        Ok(idcode)
    } else {
        error!("No IDCODE data received");
        Err("No IDCODE data")
    }
}
