// Include the generated bindings at compile time
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Implement all the C functions that need linking

// probe_config.h
//
// #include "probe.h"
// DAP_Data

// DAP_config.h

#[no_mangle]
pub extern "C" fn clock_get_hz() -> u32 {
    embassy_rp::clocks::clk_sys_freq()
}

#[no_mangle]
#[used]
pub extern "C" fn probe_init() {}
