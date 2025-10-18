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
use embassy_time::Timer;
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
    // cfg.clock_divider = (U56F8!(4.0)).to_fixed(); // 125MHz / 4 = ~31.25MHz
    cfg.clock_divider = (U56F8!(16.0)).to_fixed(); // 125MHz / 4 = ~31.25MHz

    sm.set_config(&cfg);
}

/// Simple PIO test - just send some bits and see if we get anything back
pub async fn pio_simple_test(sm: &mut StateMachine<'_, PIO0, 1>) -> Result<(), &'static str> {
    info!("Starting simple PIO test");

    // Get the program addresses
    let prg = pio_file!("src/probe.pio");
    let write_cmd_addr = prg.public_defines.write_cmd as u32;
    let read_cmd_addr = prg.public_defines.read_cmd as u32;

    let make_cmd = |cmd_addr: u32, dir: u32, count: u32| -> u32 {
        (cmd_addr << 9) | (dir << 8) | (count & 0xFF)
    };

    // Send 8 high bits
    let write_8_bits = make_cmd(write_cmd_addr, 0, 7);
    info!("Sending write command: 0x{:08X}", write_8_bits);
    sm.tx().push(write_8_bits);
    sm.tx().push(0xFF_u32);

    // Wait a bit
    Timer::after_millis(10).await;

    // Try to read 8 bits
    let read_8_bits = make_cmd(read_cmd_addr, 1, 7);
    info!("Sending read command: 0x{:08X}", read_8_bits);
    sm.tx().push(read_8_bits);

    // Wait for response
    Timer::after_millis(10).await;

    if let Some(data) = sm.rx().try_pull() {
        info!("Got response: 0x{:08X} ({:08b})", data, data);
        Ok(())
    } else {
        error!("No response from PIO");
        Err("No PIO response")
    }
}

/// Read the SWD IDCODE register - simplest SWD operation to test connectivity
pub async fn swd_read_idcode(sm: &mut StateMachine<'_, PIO0, 1>) -> Result<u32, &'static str> {
    info!("Starting SWD IDCODE read");

    // Get the program addresses from the loaded PIO program
    let prg = pio_file!("src/probe.pio");
    let write_cmd_addr = prg.public_defines.write_cmd as u32;
    let read_cmd_addr = prg.public_defines.read_cmd as u32;

    // Helper function to create command word
    let make_cmd = |cmd_addr: u32, dir: u32, count: u32| -> u32 {
        (cmd_addr << 9) | (dir << 8) | (count & 0xFF)
    };

    // SWD line reset sequence: 50+ high bits
    let write_cmd_32_bits = make_cmd(write_cmd_addr, 0, 31); // 32 bits output (31+1)
    sm.tx().push(write_cmd_32_bits);
    sm.tx().push(0xFFFFFFFF_u32); // 32 high bits

    sm.tx().push(write_cmd_32_bits);
    sm.tx().push(0xFFFFFFFF_u32); // Another 32 high bits

    // JTAG-to-SWD switching sequence (16 bits)
    let write_cmd_16_bits = make_cmd(write_cmd_addr, 0, 15); // 16 bits output (15+1)
    sm.tx().push(write_cmd_16_bits);
    sm.tx().push(0x79E7_u32); // JTAG-to-SWD sequence

    // More line reset
    sm.tx().push(write_cmd_32_bits);
    sm.tx().push(0xFFFFFFFF_u32);

    // 8 low bits to end reset
    let write_cmd_8_bits = make_cmd(write_cmd_addr, 0, 7); // 8 bits output (7+1)
    sm.tx().push(write_cmd_8_bits);
    sm.tx().push(0x00_u32);

    // SWD packet for IDCODE read (8 bits)
    // - Start bit: 1
    // - APnDP: 0 (DP access)
    // - RnW: 1 (Read)
    // - A[2:3]: 00 (IDCODE register address)
    // - Parity: 0 (even parity of APnDP + RnW + A[2:3] = 0+1+0+0 = 1, so parity = 0)
    // - Stop: 0
    // - Park: 1
    // Total: 10100001 = 0xA1
    sm.tx().push(write_cmd_8_bits);
    sm.tx().push(0xA1_u32);

    info!("Sent SWD IDCODE request, waiting for response");

    // Turnaround (1 bit) - switch to input
    let read_cmd_1_bit = make_cmd(read_cmd_addr, 1, 0); // 1 bit input (0+1)
    sm.tx().push(read_cmd_1_bit);

    // Read ACK (3 bits)
    let read_cmd_3_bits = make_cmd(read_cmd_addr, 1, 2); // 3 bits input (2+1)
    sm.tx().push(read_cmd_3_bits);

    // Wait for ACK response
    if let Some(ack_data) = sm.rx().try_pull() {
        let ack = ack_data & 0x7; // Bottom 3 bits
        if ack != 0x1 {
            // 001 = OK/VALID
            error!("SWD ACK error: {:03b}", ack);
            return Err("Invalid ACK response");
        }
        info!("SWD ACK OK: {:03b}", ack);
    } else {
        error!("No ACK response received");
        return Err("No ACK response");
    }

    // Read IDCODE data (32 bits)
    let read_cmd_32_bits = make_cmd(read_cmd_addr, 1, 31); // 32 bits input (31+1)
    sm.tx().push(read_cmd_32_bits);

    if let Some(idcode) = sm.rx().try_pull() {
        info!("SWD IDCODE: 0x{:08X}", idcode);

        // Decode some basic info
        let designer = (idcode >> 1) & 0x7FF;
        let part_no = (idcode >> 12) & 0xFFFF;
        let version = (idcode >> 28) & 0xF;

        info!("  Designer: 0x{:03X}", designer);
        info!("  Part No: 0x{:04X}", part_no);
        info!("  Version: 0x{:X}", version);

        // Read parity bit (1 bit)
        sm.tx().push(read_cmd_1_bit);
        if let Some(parity_data) = sm.rx().try_pull() {
            let parity = parity_data & 0x1;
            info!("  Parity: {}", parity);
        }

        Ok(idcode)
    } else {
        error!("No IDCODE data received");
        Err("No IDCODE data")
    }
}
