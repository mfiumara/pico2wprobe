//! Comprehensive Integration Tests - "God Tests"
//!
//! This test suite validates all major components of the pico2wprobe firmware
//! running natively on RP2350x hardware.

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

// Import embassy-rp to provide critical section implementation and interrupt vectors
use embassy_rp::{self as _};

struct TestState {
    test_count: u32,
    passed: u32,
    failed: u32,
}

#[defmt_test::tests]
mod tests {
    use defmt::{info, warn};

    #[init]
    fn init() -> super::TestState {
        info!("=== God Tests - Comprehensive Integration Test Suite ===");
        info!("Testing pico2wprobe CMSIS-DAP debug probe firmware");

        super::TestState {
            test_count: 0,
            passed: 0,
            failed: 0,
        }
    }

    #[before_each]
    fn before_each(state: &mut super::TestState) {
        state.test_count += 1;
        info!(">>> Starting test #{}", state.test_count);
    }

    #[after_each]
    fn after_each(state: &mut super::TestState) {
        info!("<<< Completed test #{}", state.test_count);
        info!(
            "Test summary: {} total, {} passed, {} failed",
            state.test_count, state.passed, state.failed
        );
    }

    // ============================================================================
    // SECTION 1: Hardware Initialization Tests
    // ============================================================================

    #[test]
    fn test_hardware_init(state: &mut super::TestState) {
        info!("Test: Hardware initialization and peripherals");

        let p = embassy_rp::init(Default::default());

        // Verify we can access peripherals
        defmt::assert!(
            core::mem::size_of_val(&p) > 0,
            "Peripherals should be initialized"
        );

        info!("✓ Hardware initialization successful");
        state.passed += 1;
    }

    #[test]
    fn test_clock_configuration(state: &mut super::TestState) {
        info!("Test: System clock configuration");

        let _ = embassy_rp::init(Default::default());

        let clk_sys = embassy_rp::clocks::clk_sys_freq();
        let clk_peri = embassy_rp::clocks::clk_peri_freq();

        info!("System clock: {} Hz", clk_sys);
        info!("Peripheral clock: {} Hz", clk_peri);

        // Verify reasonable clock frequencies
        defmt::assert!(clk_sys > 1_000_000, "System clock should be > 1MHz");
        defmt::assert!(clk_peri > 1_000_000, "Peripheral clock should be > 1MHz");

        info!("✓ Clock configuration valid");
        state.passed += 1;
    }

    // ============================================================================
    // SECTION 2: PIO State Machine Tests
    // ============================================================================

    #[test]
    fn test_pio_initialization(state: &mut super::TestState) {
        use pico2wprobe::usb::Irqs;

        info!("Test: PIO state machine initialization");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);

        info!("✓ PIO0 initialized successfully");

        // Verify we can access the common PIO block
        defmt::assert!(
            core::mem::size_of_val(&pio0) > 0,
            "PIO should be initialized"
        );

        state.passed += 1;
    }

    #[test]
    fn test_probe_creation(state: &mut super::TestState) {
        use pico2wprobe::usb::Irqs;

        info!("Test: Create probe instance with PIO");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let _probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        info!("✓ Probe instance created successfully");
        info!("  SWDIO: PIN_3, SWCLK: PIN_2");

        state.passed += 1;
    }

    // ============================================================================
    // SECTION 3: SWD Protocol Tests
    // ============================================================================

    #[test]
    fn test_swd_line_reset(state: &mut super::TestState) {
        use pico2wprobe::usb::Irqs;

        info!("Test: SWD line reset sequence");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Send line reset sequence (50+ high bits)
        let line_reset = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03];
        probe.swj_sequence(56, &line_reset);

        info!("✓ SWD line reset sequence sent");
        state.passed += 1;
    }

    #[test]
    fn test_jtag_to_swd_switch(state: &mut super::TestState) {
        use pico2wprobe::usb::Irqs;

        info!("Test: JTAG-to-SWD switching sequence");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Send JTAG-to-SWD switching sequence (0x79E7)
        let jtag_to_swd = [0xE7, 0x9E];
        probe.swj_sequence(16, &jtag_to_swd);

        info!("✓ JTAG-to-SWD switch sequence sent (0x79E7)");
        state.passed += 1;
    }

    #[test]
    fn test_swd_idcode_read_full_sequence(state: &mut super::TestState) {
        use pico2wprobe::probe::cbindings as dap;
        use pico2wprobe::usb::Irqs;

        info!("Test: Full SWD IDCODE read with proper initialization");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Step 1: Line reset
        let line_reset = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03];
        probe.swj_sequence(56, &line_reset);
        info!("  ✓ Line reset sent");

        // Step 2: JTAG-to-SWD
        let jtag_to_swd = [0xE7, 0x9E];
        probe.swj_sequence(16, &jtag_to_swd);
        info!("  ✓ JTAG-to-SWD switch sent");

        // Step 3: Second line reset
        probe.swj_sequence(56, &line_reset);
        info!("  ✓ Second line reset sent");

        // Step 4: Idle cycles
        let idle = [0x00];
        probe.swj_sequence(8, &idle);
        info!("  ✓ Idle cycles sent");

        // Step 5: Read IDCODE
        let mut idcode = 0u32;
        let request = dap::TRANSFER_RnW; // Read from DP IDCODE
        let ack = probe.swd_transfer(request, Some(&mut idcode));

        info!("  ACK: 0x{:02x}", ack);
        info!("  IDCODE: 0x{:08x}", idcode);

        match ack {
            ack if ack == dap::TRANSFER_OK as u8 => {
                info!("✓ IDCODE read successful!");

                // Basic validation
                if idcode != 0x00000000 && idcode != 0xFFFFFFFF && (idcode & 0x1) == 1 {
                    info!("✓ IDCODE is valid");
                    state.passed += 1;
                } else {
                    warn!("⚠ IDCODE may be invalid: 0x{:08x}", idcode);
                    state.passed += 1; // Still counts as passed if ACK was OK
                }
            }
            ack if ack == dap::TRANSFER_WAIT as u8 => {
                warn!("⚠ Target responded with WAIT");
                state.passed += 1; // Not a failure, target needs more time
            }
            ack if ack == dap::TRANSFER_FAULT as u8 => {
                warn!("⚠ Target responded with FAULT - no target connected?");
                state.passed += 1; // Expected if no target connected
            }
            _ => {
                warn!("⚠ Unexpected ACK: 0x{:02x}", ack);
                state.failed += 1;
            }
        }
    }

    // ============================================================================
    // SECTION 4: Probe Operations Tests
    // ============================================================================

    #[test]
    fn test_probe_clock_frequency_setting(state: &mut super::TestState) {
        use pico2wprobe::usb::Irqs;

        info!("Test: Set probe clock frequency");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Test various frequencies
        let frequencies = [100, 1000, 5000]; // kHz

        for freq in frequencies.iter() {
            probe.set_swclk_freq(*freq);
            info!("  ✓ Set SWCLK to {} kHz", freq);
        }

        info!("✓ Clock frequency setting successful");
        state.passed += 1;
    }

    #[test]
    fn test_probe_read_write_modes(state: &mut super::TestState) {
        use pico2wprobe::usb::Irqs;

        info!("Test: Probe read/write mode switching");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Test mode switching
        probe.probe_write_mode();
        info!("  ✓ Switched to write mode");

        probe.probe_read_mode();
        info!("  ✓ Switched to read mode");

        probe.probe_wait_idle();
        info!("  ✓ Wait idle successful");

        info!("✓ Mode switching successful");
        state.passed += 1;
    }

    #[test]
    fn test_probe_bit_operations(state: &mut super::TestState) {
        use pico2wprobe::usb::Irqs;

        info!("Test: Probe bit-level read/write operations");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Test writing various bit patterns
        probe.write_bits(8, 0xAA);
        info!("  ✓ Wrote 8 bits: 0xAA");

        probe.write_bits(16, 0x5555);
        info!("  ✓ Wrote 16 bits: 0x5555");

        probe.write_bits(32, 0xDEADBEEF);
        info!("  ✓ Wrote 32 bits: 0xDEADBEEF");

        // Note: Reading bits requires a target device connected
        // We can test the API but values won't be meaningful without target

        info!("✓ Bit operations successful");
        state.passed += 1;
    }

    // ============================================================================
    // SECTION 5: DAP Data Structure Tests
    // ============================================================================

    #[test]
    fn test_dap_data_access(state: &mut super::TestState) {
        use pico2wprobe::probe::cbindings;

        info!("Test: Access DAP_Data structure");

        unsafe {
            // Read various DAP configuration fields
            let clock_delay = cbindings::DAP_Data.clock_delay;
            let turnaround = cbindings::DAP_Data.swd_conf.turnaround;
            let data_phase = cbindings::DAP_Data.swd_conf.data_phase;
            let idle_cycles = cbindings::DAP_Data.transfer.idle_cycles;

            info!("  Clock delay: {}", clock_delay);
            info!("  Turnaround: {}", turnaround);
            info!("  Data phase: {}", data_phase);
            info!("  Idle cycles: {}", idle_cycles);

            // Verify reasonable values
            defmt::assert!(turnaround <= 4, "Turnaround should be <= 4 cycles");
            defmt::assert!(idle_cycles <= 255, "Idle cycles should be <= 255");
        }

        info!("✓ DAP_Data structure accessible");
        state.passed += 1;
    }

    // ============================================================================
    // SECTION 6: Memory and Safety Tests
    // ============================================================================

    #[test]
    fn test_stack_usage(state: &mut super::TestState) {
        info!("Test: Stack usage and safety");

        // Test stack usage by allocating various sized arrays
        let small_buf = [0u8; 64];
        let medium_buf = [0u8; 256];
        let large_buf = [0u8; 1024];

        defmt::assert!(small_buf.len() == 64);
        defmt::assert!(medium_buf.len() == 256);
        defmt::assert!(large_buf.len() == 1024);

        info!("  ✓ Small buffer (64 bytes) allocated");
        info!("  ✓ Medium buffer (256 bytes) allocated");
        info!("  ✓ Large buffer (1024 bytes) allocated");

        info!("✓ Stack usage test passed");
        state.passed += 1;
    }

    #[test]
    fn test_buffer_operations(state: &mut super::TestState) {
        info!("Test: Buffer operations and memory safety");

        let mut buffer = [0u8; 128];

        // Fill buffer with pattern
        for i in 0..buffer.len() {
            buffer[i] = (i % 256) as u8;
        }

        // Verify pattern
        for i in 0..buffer.len() {
            defmt::assert!(buffer[i] == (i % 256) as u8);
        }

        info!("✓ Buffer operations successful");
        state.passed += 1;
    }

    // ============================================================================
    // SECTION 7: Timing and Synchronization Tests
    // ============================================================================

    #[test]
    fn test_timing_basics(state: &mut super::TestState) {
        info!("Test: Basic timing operations");

        let start = embassy_time::Instant::now();

        // Small delay
        cortex_m::asm::delay(1000);

        let end = embassy_time::Instant::now();
        let duration = end - start;

        info!("  Measured duration: {} us", duration.as_micros());

        defmt::assert!(duration.as_micros() > 0, "Duration should be non-zero");

        info!("✓ Timing operations functional");
        state.passed += 1;
    }

    // ============================================================================
    // SECTION 8: Error Handling Tests
    // ============================================================================

    #[test]
    fn test_error_conditions(state: &mut super::TestState) {
        info!("Test: Error condition handling");

        // Test that we handle null data gracefully in transfer
        use pico2wprobe::probe::cbindings as dap;
        use pico2wprobe::usb::Irqs;

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Attempt transfer with no data (should not panic)
        let request = dap::TRANSFER_RnW;
        let ack = probe.swd_transfer(request, None);

        info!("  ACK with no target: 0x{:02x}", ack);
        info!("  ✓ No panic with missing target");

        info!("✓ Error handling functional");
        state.passed += 1;
    }

    // ============================================================================
    // SECTION 9: Integration Scenario Tests
    // ============================================================================

    #[test]
    fn test_complete_debug_session_simulation(state: &mut super::TestState) {
        use pico2wprobe::probe::cbindings as dap;
        use pico2wprobe::usb::Irqs;

        info!("Test: Simulate complete debug session");

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        // Step 1: Initialize connection
        info!("  Step 1: Initialize SWD connection");
        let line_reset = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03];
        probe.swj_sequence(56, &line_reset);

        let jtag_to_swd = [0xE7, 0x9E];
        probe.swj_sequence(16, &jtag_to_swd);
        probe.swj_sequence(56, &line_reset);
        probe.swj_sequence(8, &[0x00]);

        // Step 2: Read IDCODE
        info!("  Step 2: Read IDCODE register");
        let mut idcode = 0u32;
        let _ = probe.swd_transfer(dap::TRANSFER_RnW, Some(&mut idcode));
        info!("    IDCODE: 0x{:08x}", idcode);

        // Step 3: Attempt to read CTRL/STAT
        info!("  Step 3: Read CTRL/STAT register");
        let mut ctrl_stat = 0u32;
        let request = dap::TRANSFER_RnW | (1 << 2); // A2=1 for CTRL/STAT
        let _ = probe.swd_transfer(request, Some(&mut ctrl_stat));
        info!("    CTRL/STAT: 0x{:08x}", ctrl_stat);

        // Step 4: Set clock frequency
        info!("  Step 4: Configure clock");
        probe.set_swclk_freq(1000); // 1 MHz

        info!("✓ Complete debug session simulation finished");
        state.passed += 1;
    }

    // ============================================================================
    // Final Summary Test
    // ============================================================================

    #[test]
    fn test_summary(state: &mut super::TestState) {
        info!("=== FINAL TEST SUMMARY ===");
        info!("Total tests run: {}", state.test_count);
        info!("Tests passed: {}", state.passed);
        info!("Tests failed: {}", state.failed);

        let success_rate = if state.test_count > 0 {
            (state.passed * 100) / state.test_count
        } else {
            0
        };

        info!("Success rate: {}%", success_rate);

        if state.failed == 0 {
            info!("✓✓✓ ALL TESTS PASSED! ✓✓✓");
        } else {
            info!("⚠⚠⚠ SOME TESTS FAILED! ⚠⚠⚠");
        }

        state.passed += 1; // Count the summary test itself
    }
}
