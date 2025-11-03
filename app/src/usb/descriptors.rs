// use embassy_usb::control::bos::BosWriter;

// // Microsoft OS 2.0 Descriptor Set Header (MS_OS_20_DESCRIPTOR_INDEX)
// pub const MS_OS_20_DESC_LEN: u16 = 0xB2;

// // Microsoft OS 2.0 Descriptor Types
// const MS_OS_20_SET_HEADER_DESCRIPTOR: u16 = 0x00;
// const MS_OS_20_SUBSET_HEADER_CONFIGURATION: u16 = 0x01;
// const MS_OS_20_SUBSET_HEADER_FUNCTION: u16 = 0x02;
// const MS_OS_20_FEATURE_COMPATBLE_ID: u16 = 0x03;
// const MS_OS_20_FEATURE_REG_PROPERTY: u16 = 0x04;

// // Interface number for the probe
// const ITF_NUM_PROBE: u8 = 0;

// /// Microsoft OS 2.0 Descriptor Set
// pub const MS_OS_20_DESCRIPTOR: &[u8] = &[
//     // Set header: length, type, windows version, total length
//     0x0A,
//     0x00, // wLength
//     0x00,
//     0x00, // wDescriptorType (MS_OS_20_SET_HEADER_DESCRIPTOR)
//     0x00,
//     0x00,
//     0x03,
//     0x06, // dwWindowsVersion (0x06030000 for Windows 8.1+)
//     MS_OS_20_DESC_LEN as u8,
//     0x00, // wTotalLength
//     // Configuration subset header: length, type, configuration index, reserved, configuration total length
//     0x08,
//     0x00, // wLength
//     0x01,
//     0x00, // wDescriptorType (MS_OS_20_SUBSET_HEADER_CONFIGURATION)
//     0x00, // bConfigurationValue
//     0x00, // bReserved
//     0xA8,
//     0x00, // wTotalLength (MS_OS_20_DESC_LEN - 0x0A)
//     // Function Subset header: length, type, first interface, reserved, subset length
//     0x08,
//     0x00, // wLength
//     0x02,
//     0x00,          // wDescriptorType (MS_OS_20_SUBSET_HEADER_FUNCTION)
//     ITF_NUM_PROBE, // bFirstInterface
//     0x00,          // bReserved
//     0xA0,
//     0x00, // wSubsetLength (MS_OS_20_DESC_LEN - 0x0A - 0x08)
//     // MS OS 2.0 Compatible ID descriptor: length, type, compatible ID, sub compatible ID
//     0x14,
//     0x00, // wLength
//     0x03,
//     0x00, // wDescriptorType (MS_OS_20_FEATURE_COMPATBLE_ID)
//     b'W',
//     b'I',
//     b'N',
//     b'U',
//     b'S',
//     b'B',
//     0x00,
//     0x00, // compatibleID
//     0x00,
//     0x00,
//     0x00,
//     0x00,
//     0x00,
//     0x00,
//     0x00,
//     0x00, // subCompatibleID
//     // MS OS 2.0 Registry property descriptor: length, type
//     0x84,
//     0x00, // wLength (0x84 = 132 bytes)
//     0x04,
//     0x00, // wDescriptorType (MS_OS_20_FEATURE_REG_PROPERTY)
//     0x07,
//     0x00, // wPropertyDataType (REG_MULTI_SZ)
//     0x2A,
//     0x00, // wPropertyNameLength (42 bytes)
//     // PropertyName: "DeviceInterfaceGUIDs" in UTF-16LE
//     b'D',
//     0x00,
//     b'e',
//     0x00,
//     b'v',
//     0x00,
//     b'i',
//     0x00,
//     b'c',
//     0x00,
//     b'e',
//     0x00,
//     b'I',
//     0x00,
//     b'n',
//     0x00,
//     b't',
//     0x00,
//     b'e',
//     0x00,
//     b'r',
//     0x00,
//     b'f',
//     0x00,
//     b'a',
//     0x00,
//     b'c',
//     0x00,
//     b'e',
//     0x00,
//     b'G',
//     0x00,
//     b'U',
//     0x00,
//     b'I',
//     0x00,
//     b'D',
//     0x00,
//     b's',
//     0x00,
//     0x00,
//     0x00,
//     0x50,
//     0x00, // wPropertyDataLength (80 bytes)
//     // PropertyData: "{CDB3B5AD-293B-4663-AA36-1AAE46463776}" in UTF-16LE
//     b'{',
//     0x00,
//     b'C',
//     0x00,
//     b'D',
//     0x00,
//     b'B',
//     0x00,
//     b'3',
//     0x00,
//     b'B',
//     0x00,
//     b'5',
//     0x00,
//     b'A',
//     0x00,
//     b'D',
//     0x00,
//     b'-',
//     0x00,
//     b'2',
//     0x00,
//     b'9',
//     0x00,
//     b'3',
//     0x00,
//     b'B',
//     0x00,
//     b'-',
//     0x00,
//     b'4',
//     0x00,
//     b'6',
//     0x00,
//     b'6',
//     0x00,
//     b'3',
//     0x00,
//     b'-',
//     0x00,
//     b'A',
//     0x00,
//     b'A',
//     0x00,
//     b'3',
//     0x00,
//     b'6',
//     0x00,
//     b'-',
//     0x00,
//     b'1',
//     0x00,
//     b'A',
//     0x00,
//     b'A',
//     0x00,
//     b'E',
//     0x00,
//     b'4',
//     0x00,
//     b'6',
//     0x00,
//     b'4',
//     0x00,
//     b'6',
//     0x00,
//     b'3',
//     0x00,
//     b'7',
//     0x00,
//     b'7',
//     0x00,
//     b'6',
//     0x00,
//     b'}',
//     0x00,
//     0x00,
//     0x00,
//     0x00,
//     0x00,
// ];

// /// Build the BOS (Binary Object Store) descriptor
// pub fn build_bos_descriptor<const N: usize>(writer: &BosWriter<N>) -> heapless::Vec<u8, N> {
//     let mut buf = heapless::Vec::new();
//     buf.extend_from_slice(&[
//         // BOS header
//         0x05, // bLength
//         0x0F, // bDescriptorType (BOS)
//         0x1C,
//         0x00, // wTotalLength (28 bytes)
//         0x01, // bNumDeviceCaps
//         // Platform Capability Descriptor (Microsoft OS 2.0)
//         0x1C, // bLength
//         0x10, // bDescriptorType (Device Capability)
//         0x05, // bDevCapabilityType (Platform)
//         0x00, // bReserved
//         // Platform Capability UUID (Microsoft OS 2.0)
//         0xDF,
//         0x60,
//         0xDD,
//         0xD8,
//         0x89,
//         0x45,
//         0xC7,
//         0x4C,
//         0x9C,
//         0xD2,
//         0x65,
//         0x9D,
//         0x9E,
//         0x64,
//         0x8A,
//         0x9F,
//         // dwWindowsVersion (Windows 8.1+)
//         0x00,
//         0x00,
//         0x03,
//         0x06,
//         // wMSOSDescriptorSetTotalLength
//         MS_OS_20_DESC_LEN as u8,
//         0x00,
//         // bMS_VendorCode (0x02 for control request)
//         0x02,
//         // bAltEnumCode (0x00 for no alternate enumeration)
//         0x00,
//     ])
//     .unwrap();

//     buf
// }

// /// Get the Microsoft OS 2.0 descriptor set
// pub fn get_ms_os_20_descriptor() -> &'static [u8] {
//     MS_OS_20_DESCRIPTOR
// }

// /// Get the BOS descriptor
// pub fn get_bos_descriptor() -> heapless::Vec<u8, 28> {
//     build_bos_descriptor(&BosWriter::new())
// }
