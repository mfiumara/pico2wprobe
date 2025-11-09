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

    /// Test SWD Transfer - Read IDCODE from target
    /// NOTE: This requires an actual target device connected!
    #[test]
    fn test_swd_transfer_idcode() {
        with_probe(|probe| {
            probe.reset_sequence();

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
                } else {
                    defmt::info!("Read IDCode: 0x{}", idcode);
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
}
