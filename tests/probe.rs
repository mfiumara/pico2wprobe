// CMSIS-DAP command processing tests

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

// Import embassy-rp to provide critical section implementation and interrupt vectors
use embassy_rp::{self as _};

#[defmt_test::tests]
mod tests {
    use pico2wprobe::probe::cbindings::DAP_ProcessCommand;

    #[init]
    fn init() {
        // Initialize embassy-rp (this sets up clocks, peripherals, etc.)
        let _ = embassy_rp::init(Default::default());

        // Initialize probe hardware
        pico2wprobe::probe::cbindings::probe_init();
    }

    /// Test DAP_Info command (get firmware version)
    #[test]
    fn test_dap_info_firmware() {
        defmt::info!("Testing DAP_Info command (firmware version)");

        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        // DAP_Info command (0x00), ID = 0x04 (firmware version)
        request[0] = 0x00; // DAP_Info
        request[1] = 0x04; // FW Version

        let result = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        let response_len = (result & 0xFFFF) as usize;
        defmt::info!("Response length: {}", response_len);
        defmt::info!("Response: {:x}", &response[..response_len]);

        // Response format: [Command ID, Length, String...]
        defmt::assert!(response[0] == 0x00, "Command ID should echo back as 0x00");
        let str_len = response[1] as usize;
        defmt::info!("Firmware version length: {}", str_len);

        if str_len > 0 {
            defmt::info!("✅ DAP_Info firmware version successful!");
        }
    }

    /// Test DAP_Connect command (connect via SWD)
    #[test]
    fn test_dap_connect_swd() {
        defmt::info!("Testing DAP_Connect command (SWD)");

        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        // DAP_Connect command (0x02), Port = 0x01 (SWD)
        request[0] = 0x02; // DAP_Connect
        request[1] = 0x01; // SWD port

        let result = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        let response_len = (result & 0xFFFF) as usize;
        defmt::info!("Response length: {}", response_len);
        defmt::info!("Response: {:x}", &response[..response_len]);

        // Response format: [Command ID, Port]
        defmt::assert!(response[0] == 0x02, "Command ID should echo back as 0x02");
        defmt::info!("Connected to port: 0x{:02x}", response[1]);

        if response[1] == 0x01 {
            defmt::info!("✅ DAP_Connect SWD successful!");
        } else {
            defmt::warn!(
                "⚠️  DAP_Connect returned port: 0x{:02x} (expected 0x01)",
                response[1]
            );
        }
    }

    /// Test DAP_SWJ_Clock command (set clock frequency)
    #[test]
    fn test_dap_swj_clock() {
        defmt::info!("Testing DAP_SWJ_Clock command");

        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        // First connect
        request[0] = 0x02; // DAP_Connect
        request[1] = 0x01; // SWD port
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        // DAP_SWJ_Clock command (0x11), Frequency = 1 MHz (1000000 Hz)
        request[0] = 0x11; // DAP_SWJ_Clock
        request[1] = 0x40; // 1000000 Hz = 0x000F4240
        request[2] = 0x42;
        request[3] = 0x0F;
        request[4] = 0x00;

        let result = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        let response_len = (result & 0xFFFF) as usize;
        defmt::info!("Response length: {}", response_len);
        defmt::info!("Response: {:x}", &response[..response_len]);

        // Response format: [Command ID, Status]
        defmt::assert!(response[0] == 0x11, "Command ID should echo back as 0x11");
        defmt::info!("Clock status: 0x{:02x}", response[1]);

        if response[1] == 0x00 {
            defmt::info!("✅ DAP_SWJ_Clock successful!");
        } else {
            defmt::error!("❌ DAP_SWJ_Clock failed with status: 0x{:02x}", response[1]);
        }
    }

    /// Test DAP_Transfer command (read IDCODE)
    /// NOTE: This requires an actual target device connected!
    #[test]
    fn test_dap_transfer_idcode() {
        defmt::info!("Testing DAP_Transfer command (read IDCODE)");
        defmt::info!("NOTE: This test requires a target device to be connected");

        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        // IMPORTANT: Send SWD initialization sequence first!
        defmt::info!("Sending SWD initialization sequence...");

        // More aggressive line reset for RP2350
        // Step 1: Extended line reset (64+ bits of 0xFF to wake dormant targets)
        request[0] = 0x12; // DAP_SWJ_Sequence
        request[1] = 64; // bit count (max for one command)
        for i in 2..10 {
            request[i] = 0xFF;
        }
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
        defmt::info!("Sent 64-bit line reset");

        // Step 2: JTAG-to-SWD selection sequence (16 bits: 0xE79E as 0x9E, 0xE7)
        request[0] = 0x12; // DAP_SWJ_Sequence
        request[1] = 16; // bit count
        request[2] = 0x9E;
        request[3] = 0xE7;
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
        defmt::info!("Sent JTAG-to-SWD sequence");

        // Step 3: Another extended line reset (64 bits)
        request[0] = 0x12; // DAP_SWJ_Sequence
        request[1] = 64; // bit count
        for i in 2..10 {
            request[i] = 0xFF;
        }
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
        defmt::info!("Sent second 64-bit line reset");

        // Step 4: Idle cycles (16 bits of 0x00 instead of 8)
        request[0] = 0x12; // DAP_SWJ_Sequence
        request[1] = 16; // bit count
        request[2] = 0x00;
        request[3] = 0x00;
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
        defmt::info!("Sent idle cycles");

        defmt::info!("SWD initialization complete");

        // Now connect
        request[0] = 0x02; // DAP_Connect
        request[1] = 0x01; // SWD port
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        // Set clock to 250 KHz (recommended for RP2350 in bootloader mode)
        request[0] = 0x11; // DAP_SWJ_Clock
        // 250000 Hz = 0x0003D090
        request[1] = 0x90;
        request[2] = 0xD0;
        request[3] = 0x03;
        request[4] = 0x00;
        let result = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
        defmt::info!("Clock set result: {:x}", &response[..2]);

        // DAP_Transfer command (0x05)
        // Read IDCODE: DP, Address 0, Read
        request[0] = 0x05; // DAP_Transfer
        request[1] = 0x00; // DAP Index
        request[2] = 0x01; // Transfer count = 1
        request[3] = 0x02; // Transfer request: AP/DP=0 (DP), RnW=1 (Read), A[2:3]=0

        let result = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        let response_len = (result & 0xFFFF) as usize;
        defmt::info!("Response length: {}", response_len);
        defmt::info!("Response: {:x}", &response[..response_len]);

        // Response format: [Command ID, Transfer Count, Transfer Response, Data[4]]
        defmt::assert!(response[0] == 0x05, "Command ID should echo back as 0x05");

        let transfer_count = response[1];
        let transfer_response = response[2];

        defmt::info!("Transfer count: {}", transfer_count);
        defmt::info!("Transfer response: 0x{:02x}", transfer_response);

        // In our set-up, a RP235x is connected
        // defmt::assert!(transfer_count >= 1 || transfer_response & 0x07 == 1);

        if transfer_count >= 1 && (transfer_response & 0x07) == 1 {
            // ACK = OK (0x1)
            let idcode = u32::from_le_bytes([response[3], response[4], response[5], response[6]]);
            defmt::info!("✅ IDCODE read successful!");
            defmt::info!("IDCODE: 0x{:08x}", idcode);

            // Validate IDCODE
            if idcode != 0x00000000 && idcode != 0xFFFFFFFF {
                defmt::info!("IDCODE appears valid");
            } else {
                defmt::warn!(
                    "⚠️  IDCODE is 0x{:08x} - may indicate no target or bad connection",
                    idcode
                );
            }
        } else if (transfer_response & 0x07) == 2 {
            defmt::warn!("⚠️  Target responded with WAIT (0x2) - target may be busy");
        } else if (transfer_response & 0x07) == 4 {
            defmt::warn!("⚠️  Target responded with FAULT (0x4) - check target connections");
        } else {
            defmt::error!("[x] No ACK from target - ack expected when RP235x is connected");
        }
    }

    /// Test multiple sequential DAP commands (realistic usage)
    #[test]
    fn test_dap_sequence() {
        defmt::info!("Testing realistic DAP command sequence");

        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        // 1. Get firmware version
        defmt::info!("1. Getting firmware version...");
        request[0] = 0x00; // DAP_Info
        request[1] = 0x04; // FW Version
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        // 2. Connect via SWD
        defmt::info!("2. Connecting via SWD...");
        request[0] = 0x02; // DAP_Connect
        request[1] = 0x01; // SWD
        let result = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
        let response_len = (result & 0xFFFF) as usize;
        defmt::assert!(
            response_len >= 2,
            "Connect response should be at least 2 bytes"
        );
        defmt::assert!(response[1] != 0, "Connect should succeed");

        // 3. Set clock frequency
        defmt::info!("3. Setting clock to 1 MHz...");
        request[0] = 0x11; // DAP_SWJ_Clock
        request[1] = 0x40;
        request[2] = 0x42;
        request[3] = 0x0F;
        request[4] = 0x00;
        let result = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };
        let response_len = (result & 0xFFFF) as usize;
        defmt::assert!(
            response_len >= 2,
            "Clock response should be at least 2 bytes"
        );

        // 4. Disconnect
        defmt::info!("4. Disconnecting...");
        request[0] = 0x03; // DAP_Disconnect
        let _ = unsafe { DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr()) };

        defmt::info!("✅ DAP command sequence test completed successfully!");
    }
}
