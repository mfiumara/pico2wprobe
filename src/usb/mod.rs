use core::sync::atomic::{AtomicBool, Ordering};

use crate::probe::cbindings::DAP_ProcessCommand;
use crate::probe::probe::Probe;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::peripherals::{USB, PIO0, PIN_2, PIN_3};
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler};
use embassy_rp::{Peri, bind_interrupts};
use embassy_usb::class::hid::{HidReaderWriter, State as HidState};
use embassy_usb::{Builder, Config, Handler};

use {defmt_rtt as _, panic_probe as _};

pub mod dap_hid;
pub mod descriptors;
pub mod reports;

use dap_hid::DapHidRequestHandler;
use reports::{CMSIS_DAP_REPORT_DESCRIPTOR, DAP_PACKET_SIZE};

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

bind_interrupts!(struct PioIrqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

pub struct UsbConfig {
    pub usb: Peri<'static, USB>,
}

#[embassy_executor::task]
pub async fn run_and_init_usb(
    _spawner: Spawner,
    usb: Peri<'static, USB>,
    pio0: Peri<'static, PIO0>,
    swclk_pin: Peri<'static, PIN_2>,
    swdio_pin: Peri<'static, PIN_3>,
) {
    // Initialize the probe before starting USB
    let pio = embassy_rp::pio::Pio::new(pio0, PioIrqs);
    let probe = Probe::new(pio, swdio_pin, swclk_pin);

    // Store probe in global state for C code to access
    crate::probe::init_probe(probe);
    info!("Probe initialized successfully");
    // Create the driver, from the HAL.
    let driver = UsbDriver::new(usb, UsbIrqs);

    // Create embassy-usb Config
    // Using standard ARM CMSIS-DAP VID/PID
    let mut config = Config::new(0xc251, 0xf001); // ARM CMSIS-DAP VID:PID
    config.manufacturer = Some("DebugHub");
    config.product = Some("CMSIS-DAP Probe");
    config.serial_number = Some("DH-001");
    config.max_power = 100; // 100mA
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut request_handler = DapHidRequestHandler::new();
    let mut device_handler = MyDeviceHandler::new();

    let mut state = HidState::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );

    builder.handler(&mut device_handler);

    // Create CMSIS-DAP HID interface
    let hid_config = embassy_usb::class::hid::Config {
        report_descriptor: CMSIS_DAP_REPORT_DESCRIPTOR,
        request_handler: Some(&mut request_handler),
        poll_ms: 1, // Poll every 1ms for responsive debugging
        max_packet_size: DAP_PACKET_SIZE as u16,
    };
    let hid = HidReaderWriter::<_, DAP_PACKET_SIZE, DAP_PACKET_SIZE>::new(
        &mut builder,
        &mut state,
        hid_config,
    );

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let (mut reader, mut writer) = hid.split();

    // DAP command processing task
    let dap_fut = async {
        let mut request_buf = [0u8; DAP_PACKET_SIZE];
        let mut response_buf = [0u8; DAP_PACKET_SIZE];

        loop {
            // Read DAP command from host
            match reader.read(&mut request_buf).await {
                Ok(n) => {
                    if n > 0 {
                        debug!("Received DAP command: {} bytes", n);

                        // Process the DAP command using the C library
                        let response_len = unsafe {
                            DAP_ProcessCommand(
                                request_buf.as_ptr(),
                                response_buf.as_mut_ptr(),
                            ) as usize
                        };

                        if response_len > 0 {
                            // Send response back to host
                            match writer.write(&response_buf[..response_len]).await {
                                Ok(_) => {
                                    debug!("Sent DAP response: {} bytes", response_len);
                                }
                                Err(e) => {
                                    error!("Failed to send DAP response: {:?}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read DAP command: {:?}", e);
                }
            }
        }
    };

    // Run everything concurrently
    info!("Starting CMSIS-DAP USB interface");
    join(usb_fut, dap_fut).await;
}

// struct MyRequestHandler {}

// impl RequestHandler for MyRequestHandler {
//     fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
//         info!("Get report for {:?}", id);
//         None
//     }

//     fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
//         info!("Set report for {:?}: {=[u8]}", id, data);
//         OutResponse::Accepted
//     }

//     fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
//         info!("Set idle rate for {:?} to {:?}", id, dur);
//     }

//     fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
//         info!("Get idle rate for {:?}", id);
//         None
//     }
// }

struct MyDeviceHandler {
    configured: AtomicBool,
}

impl MyDeviceHandler {
    fn new() -> Self {
        MyDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}
