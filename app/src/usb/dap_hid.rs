use defmt::*;
use embassy_usb::class::hid::{ReportId, RequestHandler};
use embassy_usb::control::OutResponse;

/// CMSIS-DAP HID Request Handler
/// Handles HID control requests like Set_Report, Get_Report, Set_Idle, Get_Idle
pub struct DapHidRequestHandler {
    /// Store the last idle rate set by the host (in 4ms units)
    idle_rate_ms: u32,
}

impl DapHidRequestHandler {
    pub fn new() -> Self {
        Self { idle_rate_ms: 0 }
    }
}

impl RequestHandler for DapHidRequestHandler {
    /// Handle Get_Report request
    /// For CMSIS-DAP, we typically don't use this for data transfer
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("DAP HID Get_Report for {:?}", id);
        None
    }

    /// Handle Set_Report request
    /// Some hosts may use this for control, but most DAP commands come via interrupt OUT
    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("DAP HID Set_Report for {:?}: {} bytes", id, data.len());
        OutResponse::Accepted
    }

    /// Handle Set_Idle request
    /// Sets the idle rate for the HID device (how often to send reports when no data changes)
    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("DAP HID Set_Idle for {:?} to {}ms", id, dur);
        self.idle_rate_ms = dur;
    }

    /// Handle Get_Idle request
    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("DAP HID Get_Idle for {:?}", id);
        Some(self.idle_rate_ms)
    }
}
