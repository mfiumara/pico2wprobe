// Basic probe tests without PIO conflicts

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

// Import embassy-rp to provide critical section implementation and interrupt vectors
use embassy_rp::{self as _};

#[defmt_test::tests]
mod tests {
    use pico2wprobe::usb::Irqs;

    #[init]
    fn init() {}

    #[test]
    fn test_swd_idcode_read() {
        use pico2wprobe::probe::dap;

        let p = embassy_rp::init(Default::default());
        let pio0 = embassy_rp::pio::Pio::new(p.PIO0, Irqs);
        let mut probe = pico2wprobe::probe::probe::Probe::new(pio0, p.PIN_3, p.PIN_2);

        defmt::info!("Starting SWD IDCODE read test");

        // Step 1: Send SWD line reset sequence (50+ high bits followed by 0x1A)
        let line_reset = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03]; // 50 high bits + some extra
        probe.swj_sequence(56, &line_reset);
        defmt::info!("Sent SWD line reset sequence");

        // Step 2: Send JTAG-to-SWD switching sequence (0x79E7)
        let jtag_to_swd = [0xE7, 0x9E]; // 0x79E7 in little-endian byte order
        probe.swj_sequence(16, &jtag_to_swd);
        defmt::info!("Sent JTAG-to-SWD switching sequence");

        // Step 3: Send another line reset to ensure clean state
        probe.swj_sequence(56, &line_reset);
        defmt::info!("Sent second line reset");

        // Step 4: Send idle cycles (8 low bits)
        let idle = [0x00];
        probe.swj_sequence(8, &idle);
        defmt::info!("Sent idle cycles");

        // Step 5: Read IDCODE register
        // IDCODE read: A[3:2]=00, RnW=1, APnDP=0 -> request = 0x02
        let mut idcode = 0u32;
        let request = dap::transfer::RnW; // Read from DP IDCODE (address 0x0)

        defmt::info!("Attempting to read IDCODE register...");
        let ack = probe.swd_transfer(request, Some(&mut idcode));

        defmt::info!("SWD Transfer completed:");
        defmt::info!("  ACK: 0x{:02x}", ack);
        defmt::info!("  IDCODE: 0x{:08x}", idcode);

        // Check if we got a valid response
        match ack {
            ack if ack == dap::transfer::OK => {
                defmt::info!("✅ IDCODE read successful!");
                defmt::info!("IDCODE: 0x{:08x}", idcode);

                // Basic validation - IDCODE should not be 0x00000000 or 0xFFFFFFFF
                defmt::assert!(idcode != 0x00000000, "IDCODE should not be 0x00000000");
                defmt::assert!(idcode != 0xFFFFFFFF, "IDCODE should not be 0xFFFFFFFF");

                // Check if it looks like a valid ARM IDCODE (bit 0 should be 1)
                defmt::assert!(
                    (idcode & 0x1) == 1,
                    "IDCODE bit 0 should be 1 for valid ARM cores"
                );
            }
            ack if ack == dap::transfer::WAIT => {
                defmt::warn!("⚠️  Target responded with WAIT - may need retry logic");
            }
            ack if ack == dap::transfer::FAULT => {
                defmt::error!("❌ Target responded with FAULT - check connections");
                defmt::panic!("SWD FAULT response");
            }
            _ => {
                defmt::error!("❌ Unexpected ACK response: 0x{:02x}", ack);
                defmt::panic!("Unexpected SWD response");
            }
        }
    }
}
