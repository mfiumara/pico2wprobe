// Include the generated bindings at compile time
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Include the generated bindings - this will make all the constants available
// under the debugprobe module
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Re-export the necessary constants
// pub const TRANSFER_ERROR: u32 = DAP_TRANSFER_ERROR as u32;
// pub const TRANSFER_FAULT: u32 = DAP_TRANSFER_FAULT as u32;
// pub const TRANSFER_OK: u32 = DAP_TRANSFER_OK as u32;
// pub const TRANSFER_RnW: u32 = DAP_TRANSFER_RnW as u32;
// pub const TRANSFER_WAIT: u32 = DAP_TRANSFER_WAIT as u32;
// pub const SEQUENCE_CLK: u32 = SWD_SEQUENCE_CLK as u32;
// pub const SEQUENCE_DIN: u32 = SWD_SEQUENCE_DIN as u32;

// Re-export only what you need in a clean, Rust-idiomatic way
pub use self::{
    DAP_Data as DAP_DATA, DAP_TRANSFER_ERROR as TRANSFER_ERROR,
    DAP_TRANSFER_FAULT as TRANSFER_FAULT, DAP_TRANSFER_OK as TRANSFER_OK,
    DAP_TRANSFER_RnW as TRANSFER_RnW, DAP_TRANSFER_WAIT as TRANSFER_WAIT,
    SWD_SEQUENCE_CLK as SEQUENCE_CLK, SWD_SEQUENCE_DIN as SEQUENCE_DIN,
};
