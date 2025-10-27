// Include the generated bindings at compile time
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use core::cell::RefCell;
use critical_section::Mutex;

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

// Global probe instance that will be initialized once
// We use Option to allow deferred initialization
type ProbeType = crate::probe::probe::Probe<'static, embassy_rp::peripherals::PIO0>;
static PROBE: Mutex<RefCell<Option<ProbeType>>> = Mutex::new(RefCell::new(None));

/// Initialize the probe with the given PIO and pins
/// This should be called before any other probe functions
pub fn init_probe(probe: ProbeType) {
    critical_section::with(|cs| {
        PROBE.borrow_ref_mut(cs).replace(probe);
    });
}

/// Execute a closure with access to the global probe instance
fn with_probe<F, R>(f: F) -> R
where
    F: FnOnce(&mut ProbeType) -> R,
    R: Default,
{
    critical_section::with(|cs| {
        if let Some(probe) = PROBE.borrow_ref_mut(cs).as_mut() {
            f(probe)
        } else {
            // Probe not initialized - return default value
            R::default()
        }
    })
}

// C-callable FFI functions

#[no_mangle]
pub extern "C" fn probe_init() {
    // Called by C code - probe should already be initialized via init_probe()
    // This is a no-op since we handle initialization differently in Rust
}

#[no_mangle]
pub extern "C" fn probe_deinit() {
    // Called by C code to deinitialize the probe
    critical_section::with(|cs| {
        *PROBE.borrow_ref_mut(cs) = None;
    });
}

#[no_mangle]
pub extern "C" fn probe_write_mode() {
    with_probe(|probe| probe.probe_write_mode());
}

#[no_mangle]
pub extern "C" fn probe_read_mode() {
    with_probe(|probe| probe.probe_read_mode());
}

#[no_mangle]
pub extern "C" fn SWJ_Sequence(count: u32, data: *const u8) {
    if data.is_null() {
        return;
    }

    // Convert C pointer to Rust slice
    let data_slice = unsafe {
        let len = ((count + 7) / 8) as usize;
        core::slice::from_raw_parts(data, len)
    };

    with_probe(|probe| probe.swj_sequence(count, data_slice));
}

#[no_mangle]
pub extern "C" fn SWD_Sequence(info: u32, swdo: *const u8, swdi: *mut u8) {
    // Calculate buffer size based on info
    let bit_count = if (info & SEQUENCE_CLK) == 0 {
        64
    } else {
        info & SEQUENCE_CLK
    };
    let byte_count = ((bit_count + 7) / 8) as usize;

    // Convert C pointers to Rust slices
    let swdo_slice = if swdo.is_null() {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(swdo, byte_count) }
    };

    let swdi_slice = if swdi.is_null() {
        &mut [][..]
    } else {
        unsafe { core::slice::from_raw_parts_mut(swdi, byte_count) }
    };

    with_probe(|probe| probe.swd_sequence(info, swdo_slice, swdi_slice));
}

#[no_mangle]
pub extern "C" fn SWD_Transfer(request: u32, data: *mut u32) -> u8 {
    let data_ref = if data.is_null() {
        None
    } else {
        Some(unsafe { &mut *data })
    };

    with_probe(|probe| probe.swd_transfer(request, data_ref))
}

#[no_mangle]
pub extern "C" fn cached_delay() -> u32 {
    with_probe(|probe| probe.cached_delay)
}

#[no_mangle]
pub extern "C" fn time_us_32() -> u32 {
    embassy_time::Instant::now().as_micros() as u32
}
