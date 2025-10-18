use defmt::*;
use embassy_rp::Peri;
use embassy_rp::pio::program::pio_file;
use embassy_rp::pio::{Config, Instance, Pio, PioPin};
use fixed::traits::ToFixed;
use fixed_macro::types::U56F8;

// PIO program function addresses are now dynamically retrieved from the program

pub struct Probe<'a, T: Instance> {
    sm: embassy_rp::pio::StateMachine<'a, T, 0>,
    origin: u8,
    write_cmd_addr: u32,
    get_next_cmd_addr: u32,
    read_cmd_addr: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProbePioCommand {
    Write = 0,
    Skip = 1,
    Turnaround = 2,
    Read = 3,
}

impl<'a, T: Instance> Probe<'a, T> {
    pub fn new<SWDIO: PioPin, SWCLK: PioPin>(
        mut pio: Pio<'a, T>,
        swdio_pin: Peri<'a, SWDIO>,
        swclk_pin: Peri<'a, SWCLK>,
    ) -> Self {
        // Configure pins
        let swdio_pio_pin = pio.common.make_pio_pin(swdio_pin);
        let swclk_pio_pin = pio.common.make_pio_pin(swclk_pin);

        // Load the SWD probe PIO program
        let prg = pio_file!("src/probe/probe.pio");
        let loaded_program = pio.common.load_program(&prg.program);

        // // Load program with sideset pins (SWCLK for sideset)
        let mut cfg = Config::default();
        cfg.use_program(&loaded_program, &[&swclk_pio_pin]);

        // // Configure SWDIO pin (data)
        cfg.set_out_pins(&[&swdio_pio_pin]);
        cfg.set_set_pins(&[&swdio_pio_pin]);
        cfg.set_in_pins(&[&swdio_pio_pin]);

        // // Configure shifts for SWD protocol (LSB first)
        cfg.shift_out.direction = embassy_rp::pio::ShiftDirection::Right;
        cfg.shift_out.auto_fill = false;
        cfg.shift_in.direction = embassy_rp::pio::ShiftDirection::Right;
        cfg.shift_in.auto_fill = false;

        // // Set clock divider for appropriate SWD timing
        cfg.clock_divider = (U56F8!(4.0)).to_fixed(); // 125MHz / 4 = ~31.25MHz

        // We'll use the first state machine
        pio.sm0.set_config(&cfg);
        pio.sm0.set_enable(true);

        Self {
            sm: pio.sm0,
            origin: loaded_program.origin,
            write_cmd_addr: prg.public_defines.write_cmd as u32,
            get_next_cmd_addr: prg.public_defines.get_next_cmd as u32,
            read_cmd_addr: prg.public_defines.read_cmd as u32,
        }
    }

    /// Read the SWD IDCODE register - simplest SWD operation to test connectivity
    pub async fn swd_read_idcode(&mut self) -> Result<u32, &'static str> {
        info!("Starting SWD IDCODE read");

        // SWD line reset sequence: 50+ high bits
        let write_cmd_32_bits = self.fmt_probe_command(31, false, ProbePioCommand::Write); // 32 bits output (31+1)
        self.sm.tx().push(write_cmd_32_bits);
        self.sm.tx().push(0xFFFFFFFF_u32); // 32 high bits

        self.sm.tx().push(write_cmd_32_bits);
        self.sm.tx().push(0xFFFFFFFF_u32); // Another 32 high bits

        // JTAG-to-SWD switching sequence (16 bits)
        let write_cmd_16_bits = self.fmt_probe_command(15, false, ProbePioCommand::Write); // 16 bits output (15+1)
        self.sm.tx().push(write_cmd_16_bits);
        self.sm.tx().push(0x79E7_u32); // JTAG-to-SWD sequence

        // More line reset
        self.sm.tx().push(write_cmd_32_bits);
        self.sm.tx().push(0xFFFFFFFF_u32);

        // 8 low bits to end reset
        let write_cmd_8_bits = self.fmt_probe_command(7, false, ProbePioCommand::Write); // 8 bits output (7+1)
        self.sm.tx().push(write_cmd_8_bits);
        self.sm.tx().push(0x00_u32);

        // SWD packet for IDCODE read (8 bits)
        // - Start bit: 1
        // - APnDP: 0 (DP access)
        // - RnW: 1 (Read)
        // - A[2:3]: 00 (IDCODE register address)
        // - Parity: 0 (even parity of APnDP + RnW + A[2:3] = 0+1+0+0 = 1, so parity = 0)
        // - Stop: 0
        // - Park: 1
        // Total: 10100001 = 0xA1
        self.sm.tx().push(write_cmd_8_bits);
        self.sm.tx().push(0xA1_u32);

        info!("Sent SWD IDCODE request, waiting for response");

        // Turnaround (1 bit) - switch to input
        let read_cmd_1_bit = self.fmt_probe_command(1, true, ProbePioCommand::Turnaround); // 1 bit input (0+1)
        self.sm.tx().push(read_cmd_1_bit);

        // Read ACK (3 bits)
        let read_cmd_3_bits = self.fmt_probe_command(2, true, ProbePioCommand::Read); // 3 bits input (2+1)
        self.sm.tx().push(read_cmd_3_bits);

        // Wait for ACK response
        if let Some(ack_data) = self.sm.rx().try_pull() {
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
        let read_cmd_32_bits = self.fmt_probe_command(31, true, ProbePioCommand::Read); // 32 bits input (31+1)
        self.sm.tx().push(read_cmd_32_bits);

        if let Some(idcode) = self.sm.rx().try_pull() {
            info!("SWD IDCODE: 0x{:08X}", idcode);

            // Decode some basic info
            let designer = (idcode >> 1) & 0x7FF;
            let part_no = (idcode >> 12) & 0xFFFF;
            let version = (idcode >> 28) & 0xF;

            info!("  Designer: 0x{:03X}", designer);
            info!("  Part No: 0x{:04X}", part_no);
            info!("  Version: 0x{:X}", version);

            // Read parity bit (1 bit)
            let read_cmd_1_bit = self.fmt_probe_command(1, true, ProbePioCommand::Read); // 1 bit input (0+1)
            self.sm.tx().push(read_cmd_1_bit);
            if let Some(parity_data) = self.sm.rx().try_pull() {
                let parity = parity_data & 0x1;
                info!("  Parity: {}", parity);
            }

            Ok(idcode)
        } else {
            error!("No IDCODE data received");
            Err("No IDCODE data")
        }
    }

    pub fn read_bits(&mut self, bit_count: u32) -> u32 {
        // Set debug pins (equivalent to DEBUG_PINS_SET)
        // Note: Debug pin functionality would need to be implemented separately

        let command = self.fmt_probe_command(bit_count, false, ProbePioCommand::Read);
        self.sm.tx().push(command);

        let data = self.sm.rx().pull();
        let data_shifted = if bit_count < 32 {
            data >> (32 - bit_count)
        } else {
            data
        };

        // Debug output (equivalent to probe_dump)
        debug!(
            "Read {} bits 0x{:x} (shifted 0x{:x})",
            bit_count, data, data_shifted
        );

        // Clear debug pins (equivalent to DEBUG_PINS_CLR)
        // Note: Debug pin functionality would need to be implemented separately

        data_shifted
    }
    pub fn fmt_probe_command(&self, bit_count: u32, out_en: bool, cmd: ProbePioCommand) -> u32 {
        let cmd_addr = match cmd {
            ProbePioCommand::Write => self.origin as u32 + self.write_cmd_addr,
            ProbePioCommand::Skip => self.origin as u32 + self.get_next_cmd_addr,
            ProbePioCommand::Turnaround => self.origin as u32 + self.write_cmd_addr,
            ProbePioCommand::Read => self.origin as u32 + self.read_cmd_addr,
        };

        ((bit_count - 1) & 0xff) | ((out_en as u32) << 8) | (cmd_addr << 9)
    }
}
