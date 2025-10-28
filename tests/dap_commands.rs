//! CMSIS-DAP Command Tests
//!
//! Comprehensive tests for all CMSIS-DAP commands and protocols
//! Tests the C FFI bindings and command processing logic

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

// Import embassy-rp to provide critical section implementation and interrupt vectors
use embassy_rp::{self as _};

#[defmt_test::tests]
mod tests {
    use defmt::info;
    use pico2wprobe::probe::cbindings::DAP_ProcessCommand;

    #[init]
    fn init() {
        info!("=== CMSIS-DAP Command Tests ===");
        info!("Testing CMSIS-DAP v2 command processing");
    }

    // Helper function to process DAP commands
    fn process_dap_command(request: &[u8]) -> ([u8; 64], usize) {
        let mut response = [0u8; 64];
        let mut req_buf = [0u8; 64];
        req_buf[..request.len()].copy_from_slice(request);

        let len = unsafe {
            DAP_ProcessCommand(req_buf.as_ptr(), response.as_mut_ptr()) as usize
        };

        (response, len)
    }

    // ============================================================================
    // SECTION 1: DAP_Info Command Tests (0x00)
    // ============================================================================

    #[test]
    fn test_dap_info_vendor_name() {
        info!("Test: DAP_Info - Vendor Name (0x01)");

        let request = [0x00, 0x01]; // DAP_Info, VENDOR_NAME
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(len >= 2, "Should have response");
        defmt::assert!(response[0] == 0x00, "Should echo command");

        if len > 2 {
            let str_len = response[1] as usize;
            info!("  Vendor name length: {}", str_len);
            if str_len > 0 && str_len < 60 {
                info!("  ✓ Vendor name returned");
            }
        }

        info!("✓ DAP_Info Vendor Name command processed");
    }

    #[test]
    fn test_dap_info_product_name() {
        info!("Test: DAP_Info - Product Name (0x02)");

        let request = [0x00, 0x02]; // DAP_Info, PRODUCT_NAME
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x00, "Should echo command");

        if len > 2 {
            let str_len = response[1] as usize;
            info!("  Product name length: {}", str_len);
            info!("  ✓ Product name returned");
        }

        info!("✓ DAP_Info Product Name command processed");
    }

    #[test]
    fn test_dap_info_serial_number() {
        info!("Test: DAP_Info - Serial Number (0x03)");

        let request = [0x00, 0x03]; // DAP_Info, SERIAL_NUMBER
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x00);

        if len > 2 {
            let str_len = response[1] as usize;
            info!("  Serial number length: {}", str_len);
            info!("  ✓ Serial number returned");
        }

        info!("✓ DAP_Info Serial Number command processed");
    }

    #[test]
    fn test_dap_info_packet_count() {
        info!("Test: DAP_Info - Packet Count (0x04)");

        let request = [0x00, 0x04]; // DAP_Info, PACKET_COUNT
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x00);

        if len >= 3 {
            let count = response[2];
            info!("  Packet count: {}", count);
            defmt::assert!(count > 0, "Should support at least 1 packet");
            info!("  ✓ Packet count returned");
        }

        info!("✓ DAP_Info Packet Count command processed");
    }

    #[test]
    fn test_dap_info_packet_size() {
        info!("Test: DAP_Info - Packet Size (0x05)");

        let request = [0x00, 0x05]; // DAP_Info, PACKET_SIZE
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x00);

        if len >= 4 {
            let size = u16::from_le_bytes([response[2], response[3]]);
            info!("  Packet size: {} bytes", size);
            defmt::assert!(size == 64, "Should be 64 bytes for HID");
            info!("  ✓ Packet size is correct");
        }

        info!("✓ DAP_Info Packet Size command processed");
    }

    #[test]
    fn test_dap_info_capabilities() {
        info!("Test: DAP_Info - Capabilities (0x06)");

        let request = [0x00, 0x06]; // DAP_Info, CAPABILITIES
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x00);

        if len >= 3 {
            let caps = response[2];
            info!("  Capabilities: 0x{:02x}", caps);
            info!("    SWD: {}", if caps & 0x01 != 0 { "YES" } else { "NO" });
            info!("    JTAG: {}", if caps & 0x02 != 0 { "YES" } else { "NO" });
            info!("    SWO_UART: {}", if caps & 0x04 != 0 { "YES" } else { "NO" });
            info!("    SWO_Manchester: {}", if caps & 0x08 != 0 { "YES" } else { "NO" });
            info!("    Atomic cmds: {}", if caps & 0x10 != 0 { "YES" } else { "NO" });

            defmt::assert!(caps & 0x01 != 0, "Should support SWD");
            info!("  ✓ Capabilities indicate SWD support");
        }

        info!("✓ DAP_Info Capabilities command processed");
    }

    // ============================================================================
    // SECTION 2: DAP_HostStatus Command Tests (0x01)
    // ============================================================================

    #[test]
    fn test_dap_host_status_connected() {
        info!("Test: DAP_HostStatus - Connected LED (0x00)");

        let request = [0x01, 0x00, 0x01]; // DAP_HostStatus, CONNECTED, ON
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(len >= 2, "Should have response");
        defmt::assert!(response[0] == 0x01, "Should echo command");
        defmt::assert!(response[1] == 0x00, "Should return OK");

        info!("✓ DAP_HostStatus Connected command processed");
    }

    #[test]
    fn test_dap_host_status_running() {
        info!("Test: DAP_HostStatus - Running LED (0x01)");

        let request = [0x01, 0x01, 0x01]; // DAP_HostStatus, RUNNING, ON
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x01);
        defmt::assert!(response[1] == 0x00, "Should return OK");

        info!("✓ DAP_HostStatus Running command processed");
    }

    // ============================================================================
    // SECTION 3: DAP_Connect Command Tests (0x02)
    // ============================================================================

    #[test]
    fn test_dap_connect_default() {
        info!("Test: DAP_Connect - Default mode (0x00)");

        let request = [0x02, 0x00]; // DAP_Connect, DEFAULT
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x02);

        if len >= 2 {
            let mode = response[1];
            info!("  Connected mode: {}", mode);
            // 0 = failed, 1 = SWD, 2 = JTAG
            if mode == 1 {
                info!("  ✓ Connected in SWD mode");
            } else if mode == 2 {
                info!("  ✓ Connected in JTAG mode");
            } else {
                info!("  ⚠ Connection failed (no target?)");
            }
        }

        info!("✓ DAP_Connect command processed");
    }

    #[test]
    fn test_dap_connect_swd() {
        info!("Test: DAP_Connect - SWD mode (0x01)");

        let request = [0x02, 0x01]; // DAP_Connect, SWD
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x02);

        if len >= 2 {
            let mode = response[1];
            info!("  Connected mode: 0x{:02x}", mode);
            // Should return 1 for SWD (or 0 if failed)
        }

        info!("✓ DAP_Connect SWD command processed");
    }

    #[test]
    fn test_dap_connect_jtag() {
        info!("Test: DAP_Connect - JTAG mode (0x02)");

        let request = [0x02, 0x02]; // DAP_Connect, JTAG
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x02);

        if len >= 2 {
            let mode = response[1];
            info!("  Connected mode: 0x{:02x}", mode);
        }

        info!("✓ DAP_Connect JTAG command processed");
    }

    // ============================================================================
    // SECTION 4: DAP_Disconnect Command Tests (0x03)
    // ============================================================================

    #[test]
    fn test_dap_disconnect() {
        info!("Test: DAP_Disconnect");

        let request = [0x03]; // DAP_Disconnect
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(len >= 2, "Should have response");
        defmt::assert!(response[0] == 0x03, "Should echo command");
        defmt::assert!(response[1] == 0x00, "Should return OK");

        info!("✓ DAP_Disconnect command processed");
    }

    // ============================================================================
    // SECTION 5: DAP_TransferConfigure Command Tests (0x04)
    // ============================================================================

    #[test]
    fn test_dap_transfer_configure() {
        info!("Test: DAP_TransferConfigure");

        // Configure: idle_cycles=8, wait_retry=100, match_retry=0
        let request = [0x04, 0x08, 0x64, 0x00, 0x00, 0x00];
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x04);
        defmt::assert!(response[1] == 0x00, "Should return OK");

        info!("  ✓ Transfer configured: idle=8, wait_retry=100");

        info!("✓ DAP_TransferConfigure command processed");
    }

    // ============================================================================
    // SECTION 6: DAP_Transfer Command Tests (0x05)
    // ============================================================================

    #[test]
    fn test_dap_transfer_read_idcode() {
        info!("Test: DAP_Transfer - Read IDCODE");

        // Transfer: DAP_index=0, transfer_count=1, request=READ DP IDCODE
        let request = [
            0x05, // DAP_Transfer
            0x00, // DAP index
            0x01, // Transfer count
            0x02, // Request: Read DP register 0 (IDCODE)
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x05);

        if len >= 3 {
            let count = response[1];
            let ack = response[2];

            info!("  Transfers completed: {}", count);
            info!("  Last ACK: 0x{:02x}", ack);

            if ack == 0x01 {
                // ACK OK
                if len >= 7 {
                    let idcode = u32::from_le_bytes([
                        response[3],
                        response[4],
                        response[5],
                        response[6],
                    ]);
                    info!("  ✓ IDCODE: 0x{:08x}", idcode);

                    if idcode != 0x00000000 && idcode != 0xFFFFFFFF {
                        info!("  ✓ Valid IDCODE received");
                    }
                }
            } else {
                info!("  ⚠ Transfer ACK not OK (no target?)");
            }
        }

        info!("✓ DAP_Transfer command processed");
    }

    #[test]
    fn test_dap_transfer_write_abort() {
        info!("Test: DAP_Transfer - Write ABORT register");

        // Transfer: DAP_index=0, count=1, request=WRITE DP ABORT, data=0x1E
        let request = [
            0x05, // DAP_Transfer
            0x00, // DAP index
            0x01, // Transfer count
            0x08, // Request: Write DP register 0 (ABORT)
            0x1E, 0x00, 0x00, 0x00, // Data: Clear all error flags
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x05);

        if len >= 3 {
            let count = response[1];
            let ack = response[2];

            info!("  Transfers completed: {}", count);
            info!("  ACK: 0x{:02x}", ack);
        }

        info!("✓ DAP_Transfer write command processed");
    }

    // ============================================================================
    // SECTION 7: DAP_TransferBlock Command Tests (0x06)
    // ============================================================================

    #[test]
    fn test_dap_transfer_block_structure() {
        info!("Test: DAP_TransferBlock - Command structure");

        // TransferBlock: DAP_index=0, count=4, request=READ AP, but we'll just
        // send the header to test structure

        let request = [
            0x06, // DAP_TransferBlock
            0x00, // DAP index
            0x04, 0x00, // Transfer count (4 transfers, little-endian)
            0x07, // Request: Read AP register
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x06, "Should echo command");

        if len >= 4 {
            let count = u16::from_le_bytes([response[1], response[2]]);
            let ack = response[3];

            info!("  Transfers completed: {}", count);
            info!("  ACK: 0x{:02x}", ack);
        }

        info!("✓ DAP_TransferBlock command structure processed");
    }

    // ============================================================================
    // SECTION 8: DAP_SWJ_Pins Command Tests (0x10)
    // ============================================================================

    #[test]
    fn test_dap_swj_pins_read() {
        info!("Test: DAP_SWJ_Pins - Read pin state");

        let request = [
            0x10, // DAP_SWJ_Pins
            0x00, // Pin output (no change)
            0x00, // Pin select (read only)
            0x00, 0x00, 0x00, 0x00, // Wait time = 0
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x10);

        if len >= 2 {
            let pins = response[1];
            info!("  Pin state: 0x{:02x}", pins);
            info!("    SWCLK/TCK: {}", if pins & 0x01 != 0 { "HIGH" } else { "LOW" });
            info!("    SWDIO/TMS: {}", if pins & 0x02 != 0 { "HIGH" } else { "LOW" });
            info!("    TDI: {}", if pins & 0x04 != 0 { "HIGH" } else { "LOW" });
            info!("    TDO: {}", if pins & 0x08 != 0 { "HIGH" } else { "LOW" });
            info!("    nTRST: {}", if pins & 0x20 != 0 { "HIGH" } else { "LOW" });
            info!("    nRESET: {}", if pins & 0x80 != 0 { "HIGH" } else { "LOW" });
        }

        info!("✓ DAP_SWJ_Pins read command processed");
    }

    // ============================================================================
    // SECTION 9: DAP_SWJ_Clock Command Tests (0x11)
    // ============================================================================

    #[test]
    fn test_dap_swj_clock_1mhz() {
        info!("Test: DAP_SWJ_Clock - Set to 1 MHz");

        let request = [
            0x11, // DAP_SWJ_Clock
            0x40, 0x42, 0x0F, 0x00, // 1,000,000 Hz in little-endian
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x11);
        defmt::assert!(response[1] == 0x00, "Should return OK");

        info!("  ✓ Clock set to 1 MHz");

        info!("✓ DAP_SWJ_Clock command processed");
    }

    #[test]
    fn test_dap_swj_clock_100khz() {
        info!("Test: DAP_SWJ_Clock - Set to 100 kHz");

        let request = [
            0x11, // DAP_SWJ_Clock
            0xA0, 0x86, 0x01, 0x00, // 100,000 Hz in little-endian
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x11);
        defmt::assert!(response[1] == 0x00);

        info!("  ✓ Clock set to 100 kHz");

        info!("✓ DAP_SWJ_Clock command processed");
    }

    // ============================================================================
    // SECTION 10: DAP_SWJ_Sequence Command Tests (0x12)
    // ============================================================================

    #[test]
    fn test_dap_swj_sequence_line_reset() {
        info!("Test: DAP_SWJ_Sequence - Line reset");

        let request = [
            0x12, // DAP_SWJ_Sequence
            0x38, // Bit count: 56 bits
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03, // Line reset pattern
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x12);
        defmt::assert!(response[1] == 0x00, "Should return OK");

        info!("  ✓ Line reset sequence sent");

        info!("✓ DAP_SWJ_Sequence command processed");
    }

    #[test]
    fn test_dap_swj_sequence_jtag_to_swd() {
        info!("Test: DAP_SWJ_Sequence - JTAG to SWD switch");

        let request = [
            0x12, // DAP_SWJ_Sequence
            0x10, // Bit count: 16 bits
            0xE7, 0x9E, // JTAG-to-SWD sequence (0x79E7)
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x12);
        defmt::assert!(response[1] == 0x00);

        info!("  ✓ JTAG-to-SWD sequence sent");

        info!("✓ DAP_SWJ_Sequence JTAG-to-SWD processed");
    }

    // ============================================================================
    // SECTION 11: DAP_SWD_Configure Command Tests (0x13)
    // ============================================================================

    #[test]
    fn test_dap_swd_configure() {
        info!("Test: DAP_SWD_Configure");

        let request = [
            0x13, // DAP_SWD_Configure
            0x00, // Config: turnaround=1, no data phase on WAIT/FAULT
        ];

        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        defmt::assert!(response[0] == 0x13);
        defmt::assert!(response[1] == 0x00, "Should return OK");

        info!("  ✓ SWD configured: turnaround=1 cycle");

        info!("✓ DAP_SWD_Configure command processed");
    }

    // ============================================================================
    // SECTION 12: Command Sequence Tests
    // ============================================================================

    #[test]
    fn test_complete_connection_sequence() {
        info!("Test: Complete DAP connection sequence");

        // Step 1: Configure SWD
        info!("  Step 1: Configure SWD");
        let request1 = [0x13, 0x00];
        let (resp1, _) = process_dap_command(&request1);
        defmt::assert!(resp1[1] == 0x00);

        // Step 2: Set clock to 1 MHz
        info!("  Step 2: Set clock");
        let request2 = [0x11, 0x40, 0x42, 0x0F, 0x00];
        let (resp2, _) = process_dap_command(&request2);
        defmt::assert!(resp2[1] == 0x00);

        // Step 3: Connect in SWD mode
        info!("  Step 3: Connect SWD");
        let request3 = [0x02, 0x01];
        let (resp3, _) = process_dap_command(&request3);
        info!("    Mode: 0x{:02x}", resp3[1]);

        // Step 4: Send line reset
        info!("  Step 4: Line reset");
        let request4 = [0x12, 0x38, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x03];
        let (resp4, _) = process_dap_command(&request4);
        defmt::assert!(resp4[1] == 0x00);

        // Step 5: JTAG to SWD
        info!("  Step 5: JTAG-to-SWD");
        let request5 = [0x12, 0x10, 0xE7, 0x9E];
        let (resp5, _) = process_dap_command(&request5);
        defmt::assert!(resp5[1] == 0x00);

        info!("✓ Complete connection sequence processed successfully");
    }

    // ============================================================================
    // SECTION 13: Error Handling Tests
    // ============================================================================

    #[test]
    fn test_invalid_command_id() {
        info!("Test: Invalid command ID handling");

        let request = [0xFE, 0x00, 0x00]; // Invalid command
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);
        info!("  Response[0]: 0x{:02x}", response[0]);

        // Should either echo the command or return an error indicator
        info!("  ✓ Invalid command handled without crash");

        info!("✓ Invalid command handling verified");
    }

    #[test]
    fn test_malformed_packet() {
        info!("Test: Malformed packet handling");

        // Send a command with insufficient data
        let request = [0x05]; // DAP_Transfer with no parameters
        let (response, len) = process_dap_command(&request);

        info!("  Response length: {}", len);

        // Should handle gracefully without crashing
        info!("  ✓ Malformed packet handled safely");

        info!("✓ Malformed packet handling verified");
    }

    // ============================================================================
    // SECTION 14: Performance Tests
    // ============================================================================

    #[test]
    fn test_command_processing_throughput() {
        info!("Test: Command processing throughput");

        let request = [0x00, 0x04]; // Simple DAP_Info command

        let start = embassy_time::Instant::now();
        const ITERATIONS: usize = 1000;

        for _ in 0..ITERATIONS {
            let _ = process_dap_command(&request);
        }

        let end = embassy_time::Instant::now();
        let duration = end - start;

        let avg_us = duration.as_micros() / ITERATIONS as u64;
        let throughput = 1_000_000 / avg_us; // Commands per second

        info!("  Iterations: {}", ITERATIONS);
        info!("  Total time: {} us", duration.as_micros());
        info!("  Average: {} us/command", avg_us);
        info!("  Throughput: {} commands/sec", throughput);

        info!("✓ Command processing throughput measured");
    }
}
