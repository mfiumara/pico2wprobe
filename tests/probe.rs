// Probe API tests - Testing the Rust Probe interface directly

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

// Import embassy-rp to provide critical section implementation and interrupt vectors
use embassy_rp::{self as _};

#[defmt_test::tests]
mod tests {
    use pico2wprobe::probe::cbindings::{TRANSFER_FAULT, TRANSFER_OK, TRANSFER_WAIT, with_probe};

    #[init]
    fn init() {
        // Initialize embassy-rp (this sets up clocks, peripherals, etc.)
        let _ = embassy_rp::init(Default::default());

        // Note: with_probe will auto-initialize the probe on first use
    }

    /// Test basic SWJ sequence generation
    #[test]
    fn test_swj_sequence() {
        with_probe(|probe| {
            // Send a simple line reset sequence (50 bits of 1s)
            let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
            probe.swj_sequence(50, &data);
        });
    }

    /// Test SWD sequence (bidirectional)
    #[test]
    fn test_swd_sequence_write() {
        with_probe(|probe| {
            // Write 16 bits to target
            let write_data = [0xA5, 0x5A];
            probe.swd_sequence(16, &write_data, &mut []);
        });
    }

    /// Test SWD sequence read
    #[test]
    fn test_swd_sequence_read() {
        with_probe(|probe| {
            // Read 16 bits from target
            let mut read_buffer = [0u8; 2];
            // SWD_SEQUENCE_DIN = 0x80
            probe.swd_sequence(0x80 | 16, &[], &mut read_buffer);
        });
    }

    /// Test clock frequency setting
    #[test]
    fn test_set_clock_frequency() {
        with_probe(|probe| {
            // Set clock to 1 MHz
            probe.set_swclk_freq(1000);

            // Set clock to 250 KHz
            probe.set_swclk_freq(250);
        });
    }

    /// Test SWD Transfer - Read IDCODE from target
    /// NOTE: This requires an actual target device connected!
    #[test]
    fn test_swd_transfer_idcode() {
        with_probe(|probe| {
            // IMPORTANT: Send SWD initialization sequence first!
            // Step 1: Extended line reset (51 bits of 0xFF to wake dormant targets)
            let line_reset_data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
            probe.swj_sequence(51, &line_reset_data);

            // Step 2: JTAG-to-SWD selection sequence (16 bits: 0xE79E as 0x9E, 0xE7)
            let jtag_to_swd = [0x9E, 0xE7];
            probe.swj_sequence(16, &jtag_to_swd);

            // Step 3: Another extended line reset (64 bits)
            probe.swj_sequence(64, &line_reset_data);

            // Step 4: Idle cycles (16 bits of 0x00)
            let idle_data = [0x00, 0x00];
            probe.swj_sequence(16, &idle_data);

            // Set clock to 250 KHz (recommended for RP2350 in bootloader mode)
            probe.set_swclk_freq(250);

            // Read IDCODE: DP register, Address[3:2]=0, RnW=1 (Read), APnDP=0 (DP)
            // Request format: bit 0 = APnDP, bit 1 = RnW, bits[3:2] = A[3:2]
            // For IDCODE: APnDP=0, RnW=1, A[3:2]=00 -> request = 0x02
            let mut idcode = 0u32;
            let ack = probe.swd_transfer(0x02, Some(&mut idcode));

            if ack == TRANSFER_OK as u8 {
                // Validate IDCODE
                if idcode == 0x00000000 || idcode == 0xFFFFFFFF {
                    defmt::warn!(
                        "IDCODE is 0x{:08x} - may indicate no target or bad connection",
                        idcode
                    );
                }
            } else if ack == TRANSFER_WAIT as u8 {
                defmt::warn!("Target responded with WAIT (0x2) - target may be busy");
            } else if ack == TRANSFER_FAULT as u8 {
                defmt::warn!("Target responded with FAULT (0x4) - check target connections");
            } else {
                defmt::warn!("No ACK from target (got 0x{:02x})", ack);
            }
        });
    }

    /// Test complete SWD sequence - initialization, clock setting, and transfer
    #[test]
    fn test_complete_swd_sequence() {
        with_probe(|probe| {
            // 1. Send SWD initialization
            let line_reset = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
            probe.swj_sequence(50, &line_reset);

            let jtag_to_swd = [0x9E, 0xE7];
            probe.swj_sequence(16, &jtag_to_swd);

            probe.swj_sequence(50, &line_reset);

            let idle = [0x00, 0x00];
            probe.swj_sequence(16, &idle);

            // 2. Set clock frequency
            probe.set_swclk_freq(1000);

            // 3. Perform a transfer (read IDCODE)
            let mut idcode = 0u32;
            let ack = probe.swd_transfer(0x02, Some(&mut idcode));

            if ack != TRANSFER_OK as u8 {
                defmt::warn!("Transfer returned ACK: 0x{:02x}", ack);
            }
        });
    }

    /// Test probe mode switching
    #[test]
    fn test_probe_mode_switching() {
        with_probe(|probe| {
            // Switch to write mode
            probe.probe_write_mode();

            // Switch to read mode
            probe.probe_read_mode();
        });
    }
}
