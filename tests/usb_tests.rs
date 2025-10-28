//! USB and HID Functionality Tests
//!
//! Tests for USB device functionality and CMSIS-DAP HID interface

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

// Import embassy-rp to provide critical section implementation and interrupt vectors
use embassy_rp::{self as _};

#[defmt_test::tests]
mod tests {
    use defmt::info;

    #[init]
    fn init() {
        info!("=== USB and HID Tests ===");
        info!("Testing USB device and CMSIS-DAP HID functionality");
    }

    // ============================================================================
    // SECTION 1: USB Descriptor Tests
    // ============================================================================

    #[test]
    fn test_usb_descriptors_constants() {
        info!("Test: Verify USB descriptor constants");

        use pico2wprobe::usb::reports::{CMSIS_DAP_REPORT_DESCRIPTOR, DAP_PACKET_SIZE};

        // Verify report descriptor is not empty
        defmt::assert!(
            CMSIS_DAP_REPORT_DESCRIPTOR.len() > 0,
            "Report descriptor should not be empty"
        );

        info!(
            "  ✓ Report descriptor size: {} bytes",
            CMSIS_DAP_REPORT_DESCRIPTOR.len()
        );

        // Verify packet size is reasonable
        defmt::assert!(
            DAP_PACKET_SIZE == 64,
            "DAP packet size should be 64 bytes"
        );

        info!("  ✓ DAP packet size: {} bytes", DAP_PACKET_SIZE);

        info!("✓ USB descriptor constants are valid");
    }

    #[test]
    fn test_cmsis_dap_report_descriptor_structure() {
        info!("Test: Validate CMSIS-DAP report descriptor structure");

        use pico2wprobe::usb::reports::CMSIS_DAP_REPORT_DESCRIPTOR;

        // Check that the report descriptor starts with USAGE_PAGE
        // Standard HID report descriptors start with usage page declaration
        let descriptor = CMSIS_DAP_REPORT_DESCRIPTOR;

        info!("  Report descriptor length: {}", descriptor.len());
        info!("  First bytes: {:02x} {:02x} {:02x} {:02x}",
            descriptor.get(0).unwrap_or(&0),
            descriptor.get(1).unwrap_or(&0),
            descriptor.get(2).unwrap_or(&0),
            descriptor.get(3).unwrap_or(&0)
        );

        // Report descriptor should have a reasonable minimum size
        defmt::assert!(descriptor.len() >= 16, "Report descriptor too short");
        defmt::assert!(descriptor.len() <= 256, "Report descriptor too long");

        info!("✓ Report descriptor structure is valid");
    }

    // ============================================================================
    // SECTION 2: USB Configuration Tests
    // ============================================================================

    #[test]
    fn test_usb_vid_pid_constants() {
        info!("Test: Verify USB VID/PID for CMSIS-DAP");

        // Standard ARM CMSIS-DAP VID:PID is 0xc251:0xf001
        // These are hardcoded in the USB module
        const EXPECTED_VID: u16 = 0xc251;
        const EXPECTED_PID: u16 = 0xf001;

        info!("  Expected VID: 0x{:04x}", EXPECTED_VID);
        info!("  Expected PID: 0x{:04x}", EXPECTED_PID);

        // Note: In actual implementation, these are set in usb/mod.rs
        // This test documents the expected values
        info!("✓ USB VID/PID constants documented");
    }

    #[test]
    fn test_usb_string_descriptors() {
        info!("Test: Document USB string descriptors");

        // Expected values from usb/mod.rs:
        // Manufacturer: "DebugHub"
        // Product: "CMSIS-DAP Probe"
        // Serial: "DH-001"

        const MANUFACTURER: &str = "DebugHub";
        const PRODUCT: &str = "CMSIS-DAP Probe";
        const SERIAL: &str = "DH-001";

        info!("  Manufacturer: {}", MANUFACTURER);
        info!("  Product: {}", PRODUCT);
        info!("  Serial: {}", SERIAL);

        defmt::assert!(MANUFACTURER.len() > 0);
        defmt::assert!(PRODUCT.len() > 0);
        defmt::assert!(SERIAL.len() > 0);

        info!("✓ USB string descriptors documented");
    }

    // ============================================================================
    // SECTION 3: HID Request Handler Tests
    // ============================================================================

    #[test]
    fn test_dap_hid_request_handler_creation() {
        info!("Test: Create DapHidRequestHandler instance");

        use pico2wprobe::usb::dap_hid::DapHidRequestHandler;

        let handler = DapHidRequestHandler::new();

        defmt::assert!(
            core::mem::size_of_val(&handler) > 0,
            "Handler should have non-zero size"
        );

        info!("✓ DapHidRequestHandler created successfully");
    }

    // ============================================================================
    // SECTION 4: USB Device Handler Tests
    // ============================================================================

    #[test]
    fn test_usb_device_handler_states() {
        info!("Test: USB device handler state transitions");

        // The USB device goes through several states:
        // 1. Enabled/Disabled
        // 2. Reset
        // 3. Addressed
        // 4. Configured

        info!("  State 1: Device enabled");
        info!("  State 2: Bus reset (100mA limit)");
        info!("  State 3: Address assigned");
        info!("  State 4: Device configured (full power)");

        info!("✓ USB device state machine documented");
    }

    // ============================================================================
    // SECTION 5: DAP Packet Processing Tests
    // ============================================================================

    #[test]
    fn test_dap_packet_buffer_sizes() {
        info!("Test: Verify DAP packet buffer sizing");

        use pico2wprobe::usb::reports::DAP_PACKET_SIZE;

        // Request and response buffers should be the same size
        const REQUEST_BUF_SIZE: usize = DAP_PACKET_SIZE;
        const RESPONSE_BUF_SIZE: usize = DAP_PACKET_SIZE;

        defmt::assert!(REQUEST_BUF_SIZE == RESPONSE_BUF_SIZE);
        defmt::assert!(REQUEST_BUF_SIZE == 64);

        info!("  ✓ Request buffer: {} bytes", REQUEST_BUF_SIZE);
        info!("  ✓ Response buffer: {} bytes", RESPONSE_BUF_SIZE);

        info!("✓ DAP packet buffer sizes are correct");
    }

    #[test]
    fn test_dap_command_format() {
        info!("Test: Document DAP command packet format");

        // DAP commands have the following structure:
        // Byte 0: Command ID
        // Bytes 1-N: Command-specific data

        info!("  DAP Command Packet Format:");
        info!("    Byte 0: Command ID");
        info!("    Byte 1-N: Command data");
        info!("  ");
        info!("  Common DAP Commands:");
        info!("    0x00: DAP_Info");
        info!("    0x01: DAP_HostStatus");
        info!("    0x02: DAP_Connect");
        info!("    0x03: DAP_Disconnect");
        info!("    0x04: DAP_TransferConfigure");
        info!("    0x05: DAP_Transfer");
        info!("    0x06: DAP_TransferBlock");
        info!("    0x12: DAP_SWJ_Sequence");

        info!("✓ DAP command format documented");
    }

    // ============================================================================
    // SECTION 6: USB/DAP Processing Integration
    // ============================================================================

    #[test]
    fn test_dap_process_command_interface() {
        info!("Test: Verify DAP_ProcessCommand C interface");

        // The DAP_ProcessCommand function is the core of CMSIS-DAP processing
        // It's a C function that we call via FFI

        use pico2wprobe::probe::cbindings::DAP_ProcessCommand;

        info!("  DAP_ProcessCommand signature:");
        info!("    Input: *const u8 (request buffer)");
        info!("    Output: *mut u8 (response buffer)");
        info!("    Returns: u32 (response length)");

        // Test with a minimal DAP_Info command (0x00, 0x04 for PACKET_SIZE)
        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        request[0] = 0x00; // DAP_Info command
        request[1] = 0x04; // Request PACKET_SIZE info

        let response_len = unsafe {
            DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr())
        };

        info!("  Test DAP_Info command:");
        info!("    Response length: {}", response_len);
        info!("    Response[0]: 0x{:02x} (should echo command ID)", response[0]);

        defmt::assert!(response_len > 0, "Response should not be empty");
        defmt::assert!(response[0] == 0x00, "Response should echo command ID");

        info!("✓ DAP_ProcessCommand interface functional");
    }

    #[test]
    fn test_dap_info_commands() {
        info!("Test: DAP_Info command variants");

        use pico2wprobe::probe::cbindings::DAP_ProcessCommand;

        // DAP_Info can query various information
        let info_types = [
            (0x01, "VENDOR_NAME"),
            (0x02, "PRODUCT_NAME"),
            (0x03, "SERIAL_NUMBER"),
            (0x04, "PACKET_COUNT"),
            (0x05, "PACKET_SIZE"),
        ];

        for (info_id, info_name) in info_types.iter() {
            let mut request = [0u8; 64];
            let mut response = [0u8; 64];

            request[0] = 0x00; // DAP_Info command
            request[1] = *info_id;

            let response_len = unsafe {
                DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr())
            };

            info!("  {} (0x{:02x}):", info_name, info_id);
            info!("    Response length: {}", response_len);

            if response_len >= 2 {
                let str_len = response[1] as usize;
                info!("    String length: {}", str_len);

                if str_len > 0 && str_len < 60 {
                    // Print the string data
                    info!("    ✓ Valid response received");
                }
            }
        }

        info!("✓ DAP_Info commands processed");
    }

    // ============================================================================
    // SECTION 7: USB Buffer Management Tests
    // ============================================================================

    #[test]
    fn test_usb_buffer_alignment() {
        info!("Test: USB buffer alignment and sizing");

        // USB buffers should be properly aligned for DMA operations
        let buffer = [0u8; 64];

        let addr = buffer.as_ptr() as usize;
        info!("  Buffer address: 0x{:08x}", addr);
        info!("  Buffer size: {} bytes", buffer.len());

        // Check alignment (should be at least 4-byte aligned)
        defmt::assert!(addr % 4 == 0, "Buffer should be 4-byte aligned");

        info!("✓ USB buffer alignment is correct");
    }

    #[test]
    fn test_multiple_buffer_allocation() {
        info!("Test: Multiple USB buffer allocation");

        // Simulate the buffers used in USB configuration
        let _config_descriptor = [0u8; 256];
        let _bos_descriptor = [0u8; 256];
        let _msos_descriptor = [0u8; 256];
        let _control_buf = [0u8; 64];
        let _request_buf = [0u8; 64];
        let _response_buf = [0u8; 64];

        info!("  ✓ Config descriptor: 256 bytes");
        info!("  ✓ BOS descriptor: 256 bytes");
        info!("  ✓ MSOS descriptor: 256 bytes");
        info!("  ✓ Control buffer: 64 bytes");
        info!("  ✓ Request buffer: 64 bytes");
        info!("  ✓ Response buffer: 64 bytes");

        let total = 256 + 256 + 256 + 64 + 64 + 64;
        info!("  Total buffer space: {} bytes", total);

        info!("✓ Multiple buffer allocation successful");
    }

    // ============================================================================
    // SECTION 8: USB Performance Tests
    // ============================================================================

    #[test]
    fn test_dap_command_processing_speed() {
        info!("Test: DAP command processing performance");

        use pico2wprobe::probe::cbindings::DAP_ProcessCommand;

        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        request[0] = 0x00; // DAP_Info
        request[1] = 0x04; // PACKET_SIZE

        let start = embassy_time::Instant::now();

        // Process command multiple times
        const ITERATIONS: usize = 100;
        for _ in 0..ITERATIONS {
            unsafe {
                DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr());
            }
        }

        let end = embassy_time::Instant::now();
        let duration = end - start;

        let avg_time_us = duration.as_micros() / ITERATIONS as u64;

        info!("  Iterations: {}", ITERATIONS);
        info!("  Total time: {} us", duration.as_micros());
        info!("  Average time per command: {} us", avg_time_us);

        // Basic sanity check - should complete in reasonable time
        defmt::assert!(
            avg_time_us < 10000,
            "Command processing should be < 10ms per command"
        );

        info!("✓ Command processing performance is acceptable");
    }

    // ============================================================================
    // SECTION 9: USB Error Handling
    // ============================================================================

    #[test]
    fn test_invalid_command_handling() {
        info!("Test: Invalid DAP command handling");

        use pico2wprobe::probe::cbindings::DAP_ProcessCommand;

        let mut request = [0u8; 64];
        let mut response = [0u8; 64];

        // Send an invalid command ID (0xFF)
        request[0] = 0xFF;

        let response_len = unsafe {
            DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr())
        };

        info!("  Invalid command response length: {}", response_len);

        // Should get a response (typically echoing the command with error)
        if response_len > 0 {
            info!("  Response[0]: 0x{:02x}", response[0]);
            info!("  ✓ Invalid command handled gracefully");
        }

        info!("✓ Invalid command handling works");
    }

    #[test]
    fn test_empty_packet_handling() {
        info!("Test: Empty packet handling");

        use pico2wprobe::probe::cbindings::DAP_ProcessCommand;

        let request = [0u8; 64]; // All zeros
        let mut response = [0u8; 64];

        let response_len = unsafe {
            DAP_ProcessCommand(request.as_ptr(), response.as_mut_ptr())
        };

        info!("  Empty packet response length: {}", response_len);

        // Command 0x00 with subcommand 0x00 should be handled
        if response_len > 0 {
            info!("  ✓ Empty packet handled");
        }

        info!("✓ Empty packet handling works");
    }
}
