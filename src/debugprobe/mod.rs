// Re-export the dap module at the crate root level

// Implement functions that are used by the C code

#[unsafe(no_mangle)]
pub fn time_us_32() -> u32 {
    embassy_time::Instant::now().as_micros() as u32
}
