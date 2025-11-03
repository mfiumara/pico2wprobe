#![no_std]
#![no_main]

use core::cell::RefCell;

use cortex_m_rt::{entry, exception};
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use embassy_boot_rp::*;
use embassy_sync::blocking_mutex::Mutex;

// RP2350 Pico 2W has 4MB flash
const FLASH_SIZE: usize = 4 * 1024 * 1024;

#[entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    // Uncomment this if you are debugging the bootloader with debugger/RTT attached,
    // as it prevents a hard fault when accessing flash 'too early' after boot.
    for _i in 0..10000000 {
        cortex_m::asm::nop();
    }

    #[cfg(feature = "defmt")]
    defmt::info!("🚀 Pico2W Bootloader v0.1.0 (embassy-boot)");

    #[cfg(not(debug_assertions))]
    let flash = {
        // Use WatchdogFlash for release builds to prevent bootloader hangs
        let watchdog = embassy_rp::watchdog::Watchdog::new(p.WATCHDOG);
        let flash = embassy_rp::flash::Flash::<_, embassy_rp::flash::Blocking, FLASH_SIZE>::new_blocking(p.FLASH);
        WatchdogFlash::start(flash, watchdog, embassy_time::Duration::from_secs(8))
    };

    #[cfg(debug_assertions)]
    let flash = {
        // Use regular Flash for debug builds to simplify development and debugging
        embassy_rp::flash::Flash::<_, embassy_rp::flash::Blocking, FLASH_SIZE>::new_blocking(p.FLASH)
    };

    let flash = Mutex::new(RefCell::new(flash));

    let config = BootLoaderConfig::from_linkerfile_blocking(&flash, &flash, &flash);
    let active_offset = config.active.offset();

    #[cfg(feature = "defmt")]
    {
        defmt::info!("Bootloader state: 0x{:08x}", config.state.offset());
        defmt::info!("Active partition:  0x{:08x}", config.active.offset());
        defmt::info!("DFU partition:     0x{:08x}", config.dfu.offset());
        defmt::info!(
            "Booting from:      0x{:08x}",
            embassy_rp::flash::FLASH_BASE as u32 + active_offset
        );
    }

    let bl: BootLoader = BootLoader::prepare(config);

    unsafe { bl.load(embassy_rp::flash::FLASH_BASE as u32 + active_offset) }
}

#[unsafe(no_mangle)]
#[cfg_attr(target_os = "none", unsafe(link_section = ".HardFault.user"))]
unsafe extern "C" fn HardFault() {
    #[cfg(feature = "defmt")]
    defmt::error!("HardFault! Resetting system...");
    cortex_m::peripheral::SCB::sys_reset();
}

#[exception]
unsafe fn DefaultHandler(_: i16) -> ! {
    const SCB_ICSR: *const u32 = 0xE000_ED04 as *const u32;
    let irqn = unsafe { core::ptr::read_volatile(SCB_ICSR) } as u8 as i16 - 16;

    #[cfg(feature = "defmt")]
    defmt::panic!("DefaultHandler #{:?}", irqn);

    #[cfg(not(feature = "defmt"))]
    panic!("DefaultHandler #{:?}", irqn);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    #[cfg(feature = "defmt")]
    defmt::error!("Bootloader panic: {}", defmt::Debug2Format(_info));

    cortex_m::asm::udf();
}
