use defmt::*;
use embassy_rp::Peri;
use embassy_rp::pio::program::pio_file;
use embassy_rp::pio::{Config, Instance, Pio, PioPin};
use fixed::traits::ToFixed;
use fixed::types::U24F8;
use fixed_macro::types::U56F8;

use crate::probe::dap;

pub struct Probe<'a, T: Instance> {
    sm: embassy_rp::pio::StateMachine<'a, T, 0>,
    origin: u8,
    write_cmd_addr: u32,
    get_next_cmd_addr: u32,
    turnaround_cmd_addr: u32,
    read_cmd_addr: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
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
        // Configure pins with proper pullups for SWD
        let mut swdio_pio_pin = pio.common.make_pio_pin(swdio_pin);
        let mut swclk_pio_pin = pio.common.make_pio_pin(swclk_pin);

        // Set pullups as required by SWD protocol - SWDIO should be pulled up
        swdio_pio_pin.set_pull(embassy_rp::gpio::Pull::Up);
        swclk_pio_pin.set_pull(embassy_rp::gpio::Pull::Down); // SWCLK typically pulled down

        // Load the SWD probe PIO program
        let prg = pio_file!("src/probe/probe.pio");
        let loaded_program = pio.common.load_program(&prg.program);

        // Configure PIO state machine
        let mut cfg = Config::default();
        cfg.use_program(&loaded_program, &[&swclk_pio_pin]);

        // Configure SWDIO pin (data) - both input and output
        cfg.set_out_pins(&[&swdio_pio_pin]);
        cfg.set_set_pins(&[&swdio_pio_pin]);
        cfg.set_in_pins(&[&swdio_pio_pin]);

        // Configure shifts for SWD protocol (LSB first)
        cfg.shift_out.direction = embassy_rp::pio::ShiftDirection::Right;
        cfg.shift_out.auto_fill = false;
        cfg.shift_in.direction = embassy_rp::pio::ShiftDirection::Right;
        cfg.shift_in.auto_fill = false;

        // Set clock divider for SWD timing - extremely slow for maximum reliability
        // Try ultra-conservative speed for hardware debugging
        cfg.clock_divider = (U56F8!(125.0)).to_fixed(); // 125MHz / 125 = 1MHz

        // Initialize pin directions - start with 2 consecutive pins as outputs (matching C code)
        // C code: pio_sm_set_consecutive_pindirs(pio0, PROBE_SM, PROBE_PIN_OFFSET, 2, true);
        // This sets SWDIO and SWCLK as outputs initially
        pio.sm0.set_pin_dirs(
            embassy_rp::pio::Direction::Out,
            &[&swdio_pio_pin, &swclk_pio_pin],
        );

        // Apply configuration
        pio.sm0.set_config(&cfg);

        // CRITICAL: Jump to get_next_cmd routine before enabling
        let jump_to_get_next_cmd = loaded_program.origin + prg.public_defines.get_next_cmd as u8;
        info!(
            "Jumping PIO SM to get_next_cmd at address: {}",
            jump_to_get_next_cmd
        );
        unsafe {
            pio.sm0.exec_jmp(jump_to_get_next_cmd);
        }
        // Now enable the state machine
        pio.sm0.set_enable(true);

        // Debug: Print PIO program addresses and pin assignments
        info!("PIO program loaded:");
        info!("  Origin: {}", loaded_program.origin);
        info!("  write_cmd: {}", prg.public_defines.write_cmd);
        info!("  get_next_cmd: {}", prg.public_defines.get_next_cmd);
        info!("  read_cmd: {}", prg.public_defines.read_cmd);
        info!("Pin assignments:");
        info!("  SWDIO (data): PIN_{}", swdio_pio_pin.pin());
        info!("  SWCLK (sideset): PIN_{}", swclk_pio_pin.pin());

        Self {
            sm: pio.sm0,
            origin: loaded_program.origin,
            write_cmd_addr: prg.public_defines.write_cmd as u32,
            get_next_cmd_addr: prg.public_defines.get_next_cmd as u32,
            turnaround_cmd_addr: prg.public_defines.turnaround_cmd as u32,
            read_cmd_addr: prg.public_defines.read_cmd as u32,
        }
    }

    pub fn set_swclk_freq(&mut self, freq_khz: u32) {
        let clk_sys_freq_hz = embassy_rp::clocks::clk_sys_freq();
        let target_freq_hz = freq_khz * 1000; // Convert kHz to Hz

        info!(
            "Set swclk freq {}KHz ({}Hz) sysclk {}Hz\n",
            freq_khz, target_freq_hz, clk_sys_freq_hz
        );

        // Calculate clock divider using embassy's approach
        // set_clock_divider expects FixedU32<U8> (24.8 fixed point)
        let divider_ratio = clk_sys_freq_hz as f32 / target_freq_hz as f32;

        // Create U24F8 fixed-point number from the ratio
        let clock_divider = U24F8::from_num(divider_ratio);

        debug!(
            "Calculated clock divider: ratio={}, fixed_point=0x{:08X}",
            divider_ratio,
            clock_divider.to_bits()
        );

        self.sm.set_clock_divider(clock_divider);
    }

    fn fmt_probe_command(&self, bit_count: u32, out_en: bool, cmd: ProbePioCommand) -> u32 {
        // All commands go through get_next_cmd which decodes the command type from the address
        let cmd_addr = match cmd {
            ProbePioCommand::Write => self.write_cmd_addr,
            ProbePioCommand::Skip => self.get_next_cmd_addr,
            ProbePioCommand::Turnaround => self.turnaround_cmd_addr,
            ProbePioCommand::Read => self.read_cmd_addr,
        };

        // Format: | 13:9 | 8 | 7:0 |
        //         | Cmd  |Dir|Count|
        // The PIO program expects: count (8 bits), direction (1 bit), command address (5 bits)
        let formatted_cmd = ((bit_count - 1) & 0xff) | ((out_en as u32) << 8) | ((cmd_addr) << 9);

        // return ((bit_count - 1) & 0xff) | ((uint)out_en << 8) | (cmd_addr << 9);

        debug!(
            "fmt_probe_command: bits={}, out_en={}, cmd={:?} -> addr={}, dir={}, formatted=0x{:08X}",
            bit_count, out_en, cmd, cmd_addr, out_en as u32, formatted_cmd
        );
        formatted_cmd
    }

    pub fn probe_wait_idle(&mut self) {
        // Wait until the state machine is not stalled on TX FIFO
        // This replaces the direct fdebug register access from the C code
        while self.sm.tx().stalled() {
            // Busy wait until TX FIFO is no longer stalled
        }
    }

    pub fn write_bits(&mut self, bit_count: u32, data: u32) {
        let command = self.fmt_probe_command(bit_count, true, ProbePioCommand::Write);
        self.sm.tx().push(command);
        self.sm.tx().push(data);

        // Debug output (equivalent to probe_dump)
        debug!("Write {} bits 0x{:x}", bit_count, data);
    }

    pub fn read_bits(&mut self, bit_count: u32) -> u32 {
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

        data_shifted
    }

    /// Generate SWJ Sequence
    ///
    /// Sends a raw SWD/JTAG sequence by writing data bits in chunks.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of bits to send from the data
    /// * `data` - Slice containing the sequence bit data
    ///
    /// # Examples
    ///
    /// ```
    /// let sequence = [0x9E, 0xE7]; // SWD line reset sequence
    /// probe.swj_sequence(16, &sequence);
    /// ```
    pub fn swj_sequence(&mut self, count: u32, data: &[u8]) {
        // TODO: Implement clock frequency adjustment based on DAP_Data.clock_delay
        // if (DAP_Data.clock_delay != cached_delay) {
        //     probe_set_swclk_freq(MAKE_KHZ(DAP_Data.clock_delay));
        //     cached_delay = DAP_Data.clock_delay;
        // }
        debug!(
            "SWJ sequence count = {} data[0] = 0x{:02x}",
            count,
            data.get(0).unwrap_or(&0)
        );

        let mut n = count;
        let mut data_iter = data.iter();

        while n > 0 {
            let bits_to_send = if n > 8 { 8 } else { n };

            if let Some(&byte_data) = data_iter.next() {
                self.write_bits(bits_to_send, byte_data as u32);
                n -= bits_to_send;
            } else {
                warn!(
                    "SWJ sequence: Still {} bits to send, but no more data available",
                    n
                );
                break;
            }
        }
    }

    /// Generate SWD Sequence
    ///
    /// Performs bidirectional SWD sequences - can read from or write to the target.
    ///
    /// # Arguments
    ///
    /// * `info` - Sequence info containing bit count and direction flag
    /// * `swdo` - Output data slice (for write operations)
    /// * `swdi` - Input data slice (for read operations, will be filled)
    ///
    /// # Examples
    ///
    /// ```
    /// let mut read_buffer = [0u8; 8];
    /// let write_data = [0xA5, 0x5A];
    ///
    /// // Read 16 bits
    /// probe.swd_sequence(0x90, &[], &mut read_buffer); // SWD_SEQUENCE_DIN | 16
    ///
    /// // Write 16 bits  
    /// probe.swd_sequence(0x10, &write_data, &mut []);
    /// ```
    pub fn swd_sequence(&mut self, info: u32, swdo: &[u8], swdi: &mut [u8]) {
        // TODO: Implement clock frequency adjustment based on DAP_Data.clock_delay
        // if (DAP_Data.clock_delay != cached_delay) {
        //     probe_set_swclk_freq(MAKE_KHZ(DAP_Data.clock_delay));
        //     cached_delay = DAP_Data.clock_delay;
        // }

        debug!("SWD sequence");

        // Extract bit count from info (lower 6 bits)
        let mut n = info & dap::swd::SEQUENCE_CLK;
        if n == 0 {
            n = 64; // 0 means 64 bits
        }

        if (info & dap::swd::SEQUENCE_DIN) != 0 {
            // Read sequence - read data from target into swdi
            let mut swdi_iter = swdi.iter_mut();
            let mut remaining = n;

            while remaining > 0 {
                let bits = if remaining > 8 { 8 } else { remaining };

                if let Some(byte_ref) = swdi_iter.next() {
                    *byte_ref = self.read_bits(bits) as u8;
                    remaining -= bits;
                } else {
                    warn!(
                        "SWD sequence read: Still {} bits to read, but no more buffer space",
                        remaining
                    );
                    break;
                }
            }
        } else {
            // Write sequence - write data from swdo to target
            let mut swdo_iter = swdo.iter();
            let mut remaining = n;

            while remaining > 0 {
                let bits = if remaining > 8 { 8 } else { remaining };

                if let Some(&byte_data) = swdo_iter.next() {
                    self.write_bits(bits, byte_data as u32);
                    remaining -= bits;
                } else {
                    warn!(
                        "SWD sequence write: Still {} bits to write, but no more data available",
                        remaining
                    );
                    break;
                }
            }
        }
    }

    /// SWD Transfer I/O
    ///
    /// Performs a complete SWD transfer including request generation, ACK handling,
    /// data transfer, and error recovery.
    ///
    /// # Arguments
    ///
    /// * `request` - Transfer request containing A[3:2], RnW, APnDP bits
    /// * `data` - For writes: data to send; for reads: receives read data
    ///
    /// # Returns
    ///
    /// ACK response: OK (0x1), WAIT (0x2), FAULT (0x4), or ERROR (0x7)
    ///
    /// # Examples
    ///
    /// ```
    /// // Read DP IDCODE register
    /// let mut idcode = 0u32;
    /// let ack = probe.swd_transfer(0x02, Some(&mut idcode)); // A2=0, A3=0, RnW=1, APnDP=0
    ///
    /// // Write to DP SELECT register  
    /// let ack = probe.swd_transfer(0x08, Some(&mut 0x00000000)); // A2=0, A3=1, RnW=0, APnDP=0
    /// ```
    pub fn swd_transfer(&mut self, request: u32, data: Option<&mut u32>) -> u8 {
        // TODO: Implement clock frequency adjustment based on DAP_Data.clock_delay
        // if (DAP_Data.clock_delay != cached_delay) {
        //     probe_set_swclk_freq(MAKE_KHZ(DAP_Data.clock_delay));
        //     cached_delay = DAP_Data.clock_delay;
        // }

        debug!("SWD_transfer");

        // Generate the request packet
        let mut prq = 0u8;
        let mut parity = 0u32;

        // Start Bit
        prq |= 1 << 0;

        // Add request bits and calculate parity
        for n in 1..5 {
            let bit = (request >> (n - 1)) & 0x1;
            prq |= (bit as u8) << n;
            parity += bit;
        }

        prq |= ((parity & 0x1) as u8) << 5; // Parity Bit
        prq |= 0 << 6; // Stop Bit (always 0)
        prq |= 1 << 7; // Park bit (always 1)

        self.write_bits(8, prq as u32);

        // Turnaround + ACK (ignore turnaround bits, extract ACK)
        // TODO: Get turnaround from DAP_Data.swd_conf.turnaround
        let turnaround = 1; // Default turnaround cycles
        let ack_raw = self.read_bits(turnaround + 3);
        let mut ack = (ack_raw >> turnaround) as u8;

        if ack == dap::transfer::OK {
            // Data transfer phase
            if (request & dap::transfer::RnW) != 0 {
                // Read operation
                let val = self.read_bits(32);
                let parity_bit = self.read_bits(1);
                let calculated_parity = val.count_ones();

                if (calculated_parity ^ parity_bit) & 1 != 0 {
                    // Parity error
                    ack = dap::transfer::ERROR;
                }

                if let Some(data_ref) = data {
                    *data_ref = val;
                }

                debug!(
                    "Read prq=0x{:02x} ack=0x{:02x} data=0x{:08x} parity=0x{:01x}",
                    prq, ack, val, parity_bit
                );

                // Turnaround for line idle
                // TODO: self.hiz_clocks(DAP_Data.swd_conf.turnaround);
                self.hiz_clocks(turnaround);
            } else {
                // Write operation
                // Turnaround for write
                self.hiz_clocks(turnaround);

                let val = data.map(|d| *d).unwrap_or(0);
                self.write_bits(32, val);

                let parity = val.count_ones() & 1;
                self.write_bits(1, parity);

                debug!(
                    "Write prq=0x{:02x} ack=0x{:02x} data=0x{:08x} parity=0x{:01x}",
                    prq, ack, val, parity
                );
            }

            // TODO: Capture Timestamp
            // if (request & DAP_TRANSFER_TIMESTAMP) != 0 {
            //     DAP_Data.timestamp = time_us_32();
            // }

            // TODO: Idle cycles - drive 0 for N clocks
            // if DAP_Data.transfer.idle_cycles > 0 {
            //     let mut remaining = DAP_Data.transfer.idle_cycles;
            //     while remaining > 0 {
            //         let cycles = if remaining > 256 { 256 } else { remaining };
            //         self.write_bits(cycles, 0);
            //         remaining -= cycles;
            //     }
            // }

            return ack;
        }

        if ack == dap::transfer::WAIT || ack == dap::transfer::FAULT {
            // TODO: Handle data_phase configuration
            let data_phase = false; // Default assumption

            if data_phase && (request & dap::transfer::RnW) != 0 {
                // Dummy Read RDATA[0:31] + Parity
                self.read_bits(33);
            }

            self.hiz_clocks(turnaround);

            if data_phase && (request & dap::transfer::RnW) == 0 {
                // Dummy Write WDATA[0:31] + Parity
                self.write_bits(32, 0);
                self.write_bits(1, 0);
            }

            return ack;
        }

        // Protocol error - back off data phase
        let backoff_bits = turnaround + 32 + 1;
        self.read_bits(backoff_bits);
        ack
    }

    /// Generate high-impedance clock cycles
    ///
    /// Drives the clock line while keeping data line in high-Z state.
    /// Used for turnaround periods in SWD protocol.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of clock cycles to generate
    fn hiz_clocks(&mut self, cycles: u32) {
        // Send turnaround command to PIO state machine
        // fmt_probe_command(bit_count, false, CMD_TURNAROUND)
        // - bit_count: number of cycles
        // - false: not a read operation (no data capture)
        // - CMD_TURNAROUND: turnaround command type
        let command = self.fmt_probe_command(cycles, false, ProbePioCommand::Turnaround);

        // Send the command and data (0 for turnaround)
        self.sm.tx().push(command);
        self.sm.tx().push(0);
    }
}
