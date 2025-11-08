use defmt::*;
use embassy_rp::Peri;
use embassy_rp::pio::program::pio_file;
use embassy_rp::pio::{Config, Instance, Pio, PioPin};
use fixed::traits::ToFixed;
use fixed::types::U24F8;
use fixed_macro::types::U56F8;

use crate::probe::cbindings::{self, DAP_Data};

fn make_khz(x: u32) -> u32 {
    cbindings::CPU_CLOCK / (2000 * (x + 1))
}
fn time_us_32() -> u32 {
    embassy_time::Instant::now().as_micros() as u32
}

pub struct Probe<'a, T: Instance> {
    // sm: embassy_rp::pio::StateMachine<'a, T, 0>,
    pio: Pio<'a, T>,
    write_cmd_addr: u32,
    get_next_cmd_addr: u32,
    turnaround_cmd_addr: u32,
    read_cmd_addr: u32,
    cached_delay: u32,
    protocol: Protocol,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum ProbePioCommand {
    Write = 0,
    Skip = 1,
    Turnaround = 2,
    Read = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum Protocol {
    SWD = 0,
    JTAG = 1,
}

impl<'a, T: Instance> Probe<'a, T> {
    pub fn new<SWDIO: PioPin, SWCLK: PioPin>(
        mut pio: Pio<'a, T>,
        swdio_pin: Peri<'a, SWDIO>,
        swclk_pin: Peri<'a, SWCLK>,
    ) -> Self {
        // GPIO initialization - equivalent to probe_gpio_init()
        // Funcsel pins (hand over to PIO)
        let mut swclk_pio_pin = pio.common.make_pio_pin(swclk_pin);
        let mut swdio_pio_pin = pio.common.make_pio_pin(swdio_pin);

        // Make sure SWDIO and SWCLK have a pullup on it. Idle state is high
        swdio_pio_pin.set_pull(embassy_rp::gpio::Pull::Up);

        // Load the SWD probe PIO program
        let prg = pio_file!("src/probe/probe.pio");
        let loaded_program = pio.common.load_program(&prg.program);

        // State machine configuration - equivalent to probe_sm_init()
        let mut cfg = Config::default();

        // use_program sets up the program and sideset pins
        // SWCLK is the sideset pin (matches sm_config_set_sideset_pins(sm_config, PROBE_PIN_SWCLK))
        cfg.use_program(&loaded_program, &[&swclk_pio_pin]);

        // Set SWDIO offset (for OUT, SET, and IN operations)
        // This matches the C code's sm_config_set_out_pins, sm_config_set_set_pins, sm_config_set_in_pins
        cfg.set_out_pins(&[&swdio_pio_pin]);
        cfg.set_set_pins(&[&swdio_pio_pin]);
        // For 2-pin bidirectional mode (not PROBE_IO_SWDI), IN uses SWDIO
        cfg.set_in_pins(&[&swdio_pio_pin]);

        // Configure shifts for SWD protocol (LSB first)
        cfg.shift_out.direction = embassy_rp::pio::ShiftDirection::Right;
        cfg.shift_out.auto_fill = false;
        cfg.shift_in.direction = embassy_rp::pio::ShiftDirection::Right;
        cfg.shift_in.auto_fill = false;

        // Set clock divider for SWD timing based on DAP_Data.clock_delay
        // Read initial clock delay from DAP_Data (set by DAP_Setup)
        let initial_clock_delay = unsafe { cbindings::DAP_Data.clock_delay };
        let initial_freq_khz = make_khz(initial_clock_delay);

        let clk_sys_freq_hz = embassy_rp::clocks::clk_sys_freq();
        let target_freq_hz = initial_freq_khz * 1000;
        let pio_freq_hz = target_freq_hz * 4; // PIO runs 4x faster than SWD clock
        let divider_ratio = clk_sys_freq_hz as f32 / pio_freq_hz as f32;
        cfg.clock_divider = U24F8::from_num(divider_ratio);

        info!(
            "Initial SWD clock: {}KHz, PIO clock: {}Hz, divider: {}",
            initial_freq_khz, pio_freq_hz, divider_ratio
        );

        // Apply configuration (equivalent to pio_sm_init)
        pio.sm0.set_config(&cfg);

        // Set SWCLK and SWDIO pins as output to start
        // C code: pio_sm_set_consecutive_pindirs(pio0, PROBE_SM, PROBE_PIN_OFFSET, 2, true);
        // NOTE: SWDIO direction will be dynamically controlled by PIO via 'out pindirs' instruction
        pio.sm0.set_pin_dirs(
            embassy_rp::pio::Direction::Out,
            &[&swclk_pio_pin, &swdio_pio_pin],
        );

        // Jump to get_next_cmd routine before enabling
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

        // CRITICAL: Command addresses must include the program origin
        // The PIO 'out pc, 5' instruction expects absolute addresses in PIO memory (0-31),
        // not relative offsets within the program
        let origin = loaded_program.origin as u32;

        info!(
            "PIO program loaded at origin={}, write={}, read={}, get_next={}, turnaround={}",
            origin,
            prg.public_defines.write_cmd,
            prg.public_defines.read_cmd,
            prg.public_defines.get_next_cmd,
            prg.public_defines.turnaround_cmd
        );

        // Read the current clock delay to initialize cached_delay
        let current_clock_delay = unsafe { cbindings::DAP_Data.clock_delay };

        Self {
            pio: pio,
            write_cmd_addr: origin + prg.public_defines.write_cmd as u32,
            get_next_cmd_addr: origin + prg.public_defines.get_next_cmd as u32,
            turnaround_cmd_addr: origin + prg.public_defines.turnaround_cmd as u32,
            read_cmd_addr: origin + prg.public_defines.read_cmd as u32,
            cached_delay: current_clock_delay,
            protocol: Protocol::SWD,
        }
    }

    /// Get the cached delay value
    pub fn get_cached_delay(&self) -> u32 {
        self.cached_delay
    }

    pub fn set_swclk_freq(&mut self, freq_khz: u32) {
        let clk_sys_freq_hz = embassy_rp::clocks::clk_sys_freq();
        let target_freq_hz = freq_khz * 1000; // Convert kHz to Hz

        info!(
            "Set swclk freq {}KHz ({}Hz) sysclk {}Hz\n",
            freq_khz, target_freq_hz, clk_sys_freq_hz
        );

        // Calculate clock divider using embassy's approach
        // IMPORTANT: Each SWD clock cycle takes 4 PIO cycles (see probe.pio timing)
        // So PIO needs to run at 4x the target SWD frequency
        // set_clock_divider expects FixedU32<U8> (24.8 fixed point)
        let pio_freq_hz = target_freq_hz * 4; // PIO runs 4x faster than SWD clock
        let divider_ratio = clk_sys_freq_hz as f32 / pio_freq_hz as f32;

        // Create U24F8 fixed-point number from the ratio
        let clock_divider = U24F8::from_num(divider_ratio);

        debug!(
            "Calculated clock divider: ratio={}, pio_freq={}Hz, fixed_point=0x{:08X}",
            divider_ratio,
            pio_freq_hz,
            clock_divider.to_bits()
        );

        self.pio.sm0.set_clock_divider(clock_divider);
    }
    // void probe_assert_reset(bool state)
    // int probe_reset_level(void)

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
        // Note: bit_count can be 0 for mode switching commands, so we use wrapping_sub
        let formatted_cmd =
            ((bit_count.wrapping_sub(1)) & 0xff) | ((out_en as u32) << 8) | ((cmd_addr) << 9);

        debug!(
            "fmt_probe_command: bits={}, out_en={}, cmd={:?} -> addr={}, dir={}, formatted=0x{:08X}",
            bit_count, out_en, cmd, cmd_addr, out_en as u32, formatted_cmd
        );
        formatted_cmd
    }

    pub fn write_bits(&mut self, bit_count: u32, data: u32) {
        let command = self.fmt_probe_command(bit_count, true, ProbePioCommand::Write);
        self.pio.sm0.tx().push(command);
        self.pio.sm0.tx().push(data);
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
        self.pio.sm0.tx().push(command);
        self.pio.sm0.tx().push(0);
    }

    pub fn read_bits(&mut self, bit_count: u32) -> u32 {
        let command = self.fmt_probe_command(bit_count, false, ProbePioCommand::Read);
        debug!(
            "read_bits: Pushing command 0x{:08x} (expects SWDIO as INPUT)",
            command
        );
        self.pio.sm0.tx().push(command);

        debug!("read_bits: Waiting for data from RX FIFO...");
        let data = self.pio.sm0.rx().pull();
        let data_shifted = if bit_count < 32 {
            data >> (32 - bit_count)
        } else {
            data
        };

        // Debug output (equivalent to probe_dump)
        debug!(
            "Read {} bits raw=0x{:08x} shifted=0x{:08x}",
            bit_count, data, data_shifted
        );

        data_shifted
    }

    pub fn probe_wait_idle(&mut self) {
        // Wait until the state machine is not stalled on TX FIFO
        // This replaces the direct fdebug register access from the C code
        while self.pio.sm0.tx().stalled() {
            // Busy wait until TX FIFO is no longer stalled
        }
    }

    pub fn probe_read_mode(&mut self) {
        let cmd = self.fmt_probe_command(0, false, ProbePioCommand::Skip);
        self.pio.sm0.tx().push(cmd);
        self.probe_wait_idle();
    }

    pub fn probe_write_mode(&mut self) {
        let cmd = self.fmt_probe_command(0, true, ProbePioCommand::Skip);
        self.pio.sm0.tx().push(cmd);
        self.probe_wait_idle();
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
        // TODO: Check if data is big enough and corresponds to the bit count
        let clock_delay = unsafe { DAP_Data.clock_delay };
        if clock_delay != self.cached_delay {
            self.set_swclk_freq(make_khz(clock_delay));
            self.cached_delay = clock_delay;
        }
        // debug!("SWJ sequence count = {} data = {:02x}", count, data);

        let mut bits_left = count;
        let mut it = data.iter();

        // Calculate number of iterations (each processes up to 32 bits)
        // Each iteration needs 2 words: 1 command + 1 data
        let num_iterations = (count + 31) / 32;
        let words_needed = (num_iterations * 2) as usize;

        // Static allocation with maximum reasonable size
        let mut words_buf: [u32; 64] = [0; 64];
        let mut word_idx = 0;

        while bits_left > 0 {
            // Try to write 32 bits in one go
            let bits = if bits_left > 32 { 32 } else { bits_left };
            let write_cmd = self.fmt_probe_command(bits, true, ProbePioCommand::Write);

            // Now we should compress the input data (u8) into a u32
            let mut word: u32 = 0x00000000;

            let bytes_to_send = bits / 8 + if bits % 8 != 0 { 1 } else { 0 };
            for n in 0..bytes_to_send {
                if let Some(&byte) = it.next() {
                    word |= (byte as u32) << (n * 8);
                } else {
                    break;
                }
            }

            debug!("PIO bits = {} data = {:02x}", bits, word);

            // Append write_cmd and word to buffer
            words_buf[word_idx] = write_cmd;
            word_idx += 1;
            words_buf[word_idx] = word;
            word_idx += 1;

            bits_left -= bits;
        }

        // Write everything to PIO in one go
        for i in 0..word_idx {
            self.pio.sm0.tx().push(words_buf[i]);
        }
    }

    /// Perform SWD line reset sequence
    ///
    /// The SWD interface does not include a dedicated reset signal. A line reset is
    /// achieved by holding the data signal HIGH for at least 50 clock cycles, followed
    /// by at least two idle cycles.
    ///
    /// A debugger must use a line reset sequence to ensure that hot-plugging the serial
    /// connection does not result in unintentional transfers. The line reset sequence
    /// ensures that the SW-DP is synchronized correctly to the header that signals a
    /// connection.
    fn line_reset(&mut self) {
        // Hold data HIGH for at least 50 clock cycles (> 50 minimum)
        // and 2 idle cycles (LOW).
        // So we'll transmit 53 bits in total
        let line_reset_data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xEF];
        self.swj_sequence(53, &line_reset_data);
    }

    /// Perform complete SWD reset sequence
    pub fn reset_sequence(&mut self) {
        self.line_reset();

        // Step 2: JTAG-to-SWD selection sequence (16 bits: 0xE79E as 0x9E, 0xE7)
        let jtag_to_swd = [0x9E, 0xE7];
        self.swj_sequence(16, &jtag_to_swd);

        // Step 3: Another extended line reset (51 bits)
        self.line_reset();
    }

    fn jtag_to_swd_sequence(&mut self, count: u32, data: &[u8]) {
        let clock_delay = unsafe { DAP_Data.clock_delay };
        if clock_delay != self.cached_delay {
            self.set_swclk_freq(make_khz(clock_delay));
            self.cached_delay = clock_delay;
        }
        debug!(
            "SWJ sequence count = {} data[0] = 0x{:02x}",
            count,
            data.get(0).unwrap_or(&0)
        );

        let mut n = count;
        let mut data_iter = data.iter();

        while n > 0 {
            let bits = if n > 8 { 8 } else { n };

            if let Some(&byte) = data_iter.next() {
                self.write_bits(bits, byte as u32);
                n -= bits;
            } else {
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
        let clock_delay = unsafe { DAP_Data.clock_delay };
        if clock_delay != self.cached_delay {
            self.set_swclk_freq(make_khz(clock_delay));
            self.cached_delay = clock_delay;
        }

        debug!("SWD sequence");

        // Extract bit count from info (lower 6 bits)
        let mut n = info & cbindings::SEQUENCE_CLK;
        if n == 0 {
            n = 64; // 0 means 64 bits
        }

        if (info & cbindings::SEQUENCE_DIN) != 0 {
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
        let clock_delay = unsafe { DAP_Data.clock_delay };
        if clock_delay != self.cached_delay {
            self.set_swclk_freq(make_khz(clock_delay));
            self.cached_delay = clock_delay;
        }

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

        debug!(
            "SWD request: 0x{:02x} (APnDP={}, RnW={}, A[3:2]={:02b}, parity={})",
            prq,
            (request & 0x1),
            (request >> 1) & 0x1,
            (request >> 2) & 0x3,
            (parity & 0x1)
        );
        self.write_bits(8, prq as u32);

        // Turnaround + ACK (ignore turnaround bits, extract ACK)
        let turnaround = unsafe { DAP_Data.swd_conf.turnaround } as u32;
        debug!("Turnaround cycles: {}", turnaround);
        let ack_raw = self.read_bits(turnaround + 3);
        debug!(
            "ACK raw (with turnaround): 0x{:08x} ({} bits)",
            ack_raw,
            turnaround + 3
        );
        let mut ack = (ack_raw >> turnaround) as u8;
        debug!(
            "ACK extracted: 0x{:02x} (expected: 0x01=OK, 0x02=WAIT, 0x04=FAULT)",
            ack & 0x07
        );

        if ack == cbindings::TRANSFER_OK as u8 {
            // Data transfer phase
            if (request & cbindings::TRANSFER_RnW) != 0 {
                // Read operation
                let val = self.read_bits(32);
                let parity_bit = self.read_bits(1);
                let calculated_parity = val.count_ones();

                if (calculated_parity ^ parity_bit) & 1 != 0 {
                    // Parity error
                    ack = cbindings::TRANSFER_ERROR as u8;
                }

                if let Some(data_ref) = data {
                    *data_ref = val;
                }

                debug!(
                    "Read prq=0x{:02x} ack=0x{:02x} data=0x{:08x} parity=0x{:01x}",
                    prq, ack, val, parity_bit
                );

                // Turnaround for line idle
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

            if (request & cbindings::DAP_TRANSFER_TIMESTAMP) != 0 {
                unsafe {
                    DAP_Data.timestamp = time_us_32();
                }
            }

            // TODO: Idle cycles - drive 0 for N clocks
            let idle_cycles = unsafe { DAP_Data.transfer.idle_cycles };
            if idle_cycles > 0 {
                let mut remaining = idle_cycles as u32;
                while remaining > 0 {
                    let cycles = if remaining > 256 { 256 } else { remaining };
                    self.write_bits(cycles, 0);
                    remaining -= cycles;
                }
            }

            return ack;
        }

        if ack == cbindings::TRANSFER_WAIT as u8 || ack == cbindings::TRANSFER_FAULT as u8 {
            let data_phase = unsafe { DAP_Data.swd_conf.data_phase } != 0;
            if data_phase && (request & cbindings::TRANSFER_RnW) != 0 {
                // Dummy Read RDATA[0:31] + Parity
                self.read_bits(33);
            }

            self.hiz_clocks(turnaround);

            if data_phase && (request & cbindings::TRANSFER_RnW) == 0 {
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
}
