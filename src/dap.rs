//! CMSIS-DAP Protocol Constants
//!
//! This module contains all the constants and types from the CMSIS-DAP specification.

/// DAP Command IDs
pub mod commands {
    pub const INFO: u8 = 0x00;
    pub const HOST_STATUS: u8 = 0x01;
    pub const CONNECT: u8 = 0x02;
    pub const DISCONNECT: u8 = 0x03;
    pub const TRANSFER_CONFIGURE: u8 = 0x04;
    pub const TRANSFER: u8 = 0x05;
    pub const TRANSFER_BLOCK: u8 = 0x06;
    pub const TRANSFER_ABORT: u8 = 0x07;
    pub const WRITE_ABORT: u8 = 0x08;
    pub const DELAY: u8 = 0x09;
    pub const RESET_TARGET: u8 = 0x0A;
    pub const SWJ_PINS: u8 = 0x10;
    pub const SWJ_CLOCK: u8 = 0x11;
    pub const SWJ_SEQUENCE: u8 = 0x12;
    pub const SWD_CONFIGURE: u8 = 0x13;
    pub const SWD_SEQUENCE: u8 = 0x1D;
    pub const JTAG_SEQUENCE: u8 = 0x14;
    pub const JTAG_CONFIGURE: u8 = 0x15;
    pub const JTAG_IDCODE: u8 = 0x16;
}

/// Debug Port Register Addresses
pub mod dp_registers {
    pub const IDCODE: u8 = 0x00; // IDCODE Register (SW Read only)
    pub const ABORT: u8 = 0x00; // Abort Register (SW Write only)
    pub const CTRL_STAT: u8 = 0x04; // Control & Status
    pub const WCR: u8 = 0x04; // Wire Control Register (SW Only)
    pub const SELECT: u8 = 0x08; // Select Register (JTAG R/W & SW W)
    pub const RESEND: u8 = 0x08; // Resend (SW Read Only)
    pub const RDBUFF: u8 = 0x0C; // Read Buffer (Read Only)
}

/// JTAG IR Codes
pub mod jtag {
    pub const ABORT: u8 = 0x08;
    pub const DPACC: u8 = 0x0A;
    pub const APACC: u8 = 0x0B;
    pub const IDCODE: u8 = 0x0E;
    pub const BYPASS: u8 = 0x0F;

    /// JTAG Sequence Info bits
    pub const SEQUENCE_TCK: u32 = 0x3F; // TCK count
    pub const SEQUENCE_TMS: u32 = 0x40; // TMS value
    pub const SEQUENCE_TDO: u32 = 0x80; // TDO capture
}

/// SWD Protocol Constants
pub mod swd {
    /// SWD Sequence Info bits
    pub const SEQUENCE_CLK: u32 = 0x3F; // SWCLK count (lower 6 bits)
    pub const SEQUENCE_DIN: u32 = 0x80; // SWDIO capture (bit 7)
}

pub mod transfer {
    // DAP Transfer Request
    pub const APnDP: u32 = 1 << 0;
    pub const RnW: u32 = 1 << 1;
    pub const A2: u32 = 1 << 2;
    pub const A3: u32 = 1 << 3;
    pub const MATCH_VALUE: u32 = 1 << 4;
    pub const MATCH_MASK: u32 = 1 << 5;
    pub const TIMESTAMP: u32 = 1 << 7;

    // DAP Transfer Response
    pub const OK: u8 = 1 << 0;
    pub const WAIT: u8 = 1 << 1;
    pub const FAULT: u8 = 1 << 2;
    pub const ERROR: u8 = 1 << 3;
    pub const MISMATCH: u8 = 1 << 4;
}

/// DAP Status and Error Codes
pub mod status {
    pub const OK: u8 = 0x00;
    pub const ERROR: u8 = 0xFF;
}

/// Port Types
pub mod ports {
    pub const DEFAULT: u8 = 0x00;
    pub const SWD: u8 = 0x01;
    pub const JTAG: u8 = 0x02;
}
