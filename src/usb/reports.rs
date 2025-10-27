/// CMSIS-DAP v1 HID Report Descriptor
/// This defines a vendor-specific HID device with 64-byte input/output reports
/// for sending DAP commands and receiving responses.

pub const CMSIS_DAP_REPORT_DESCRIPTOR: &[u8] = &[
    0x06, 0x00, 0xFF,  // Usage Page (Vendor Defined 0xFF00)
    0x09, 0x01,        // Usage (0x01)
    0xA1, 0x01,        // Collection (Application)

    // Input Report (Device -> Host)
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0xFF, 0x00,  //   Logical Maximum (255)
    0x75, 0x08,        //   Report Size (8 bits)
    0x95, 0x40,        //   Report Count (64 bytes)
    0x09, 0x01,        //   Usage (Vendor Usage 1)
    0x81, 0x02,        //   Input (Data, Variable, Absolute)

    // Output Report (Host -> Device)
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0xFF, 0x00,  //   Logical Maximum (255)
    0x75, 0x08,        //   Report Size (8 bits)
    0x95, 0x40,        //   Report Count (64 bytes)
    0x09, 0x01,        //   Usage (Vendor Usage 1)
    0x91, 0x02,        //   Output (Data, Variable, Absolute)

    0xC0,              // End Collection
];

/// Report size for CMSIS-DAP HID (64 bytes as per CMSIS-DAP specification)
pub const DAP_PACKET_SIZE: usize = 64;
