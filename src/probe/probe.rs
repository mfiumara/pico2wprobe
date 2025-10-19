use defmt::*;
use embassy_rp::Peri;
use embassy_rp::pio::program::pio_file;
use embassy_rp::pio::{Config, Instance, Pio, PioPin};
use fixed::traits::ToFixed;
use fixed::types::U24F8;
use fixed_macro::types::U56F8;

// PIO program function addresses are now dynamically retrieved from the program

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

    pub fn fmt_probe_command(&self, bit_count: u32, out_en: bool, cmd: ProbePioCommand) -> u32 {
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
}
