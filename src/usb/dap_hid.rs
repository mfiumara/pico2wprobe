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
        Self {
            idle_rate_ms: 0,
        }
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

/// Process a CMSIS-DAP command received from the host
///
/// # Arguments
/// * `request` - The DAP command packet (up to 64 bytes)
/// * `response` - Buffer to write the response into (up to 64 bytes)
///
/// # Returns
/// The number of bytes written to the response buffer
pub fn process_dap_command(request: &[u8], response: &mut [u8]) -> usize {
    if request.is_empty() {
        warn!("Empty DAP request received");
        return 0;
    }

    let command_id = request[0];
    debug!("Processing DAP command: 0x{:02X}", command_id);

    // TODO: This is where we'll integrate with your probe logic
    // For now, return a basic response indicating command not implemented
    match command_id {
        0x00 => {
            // DAP_Info
            info!("DAP_Info command received");
            // Return minimal response for now
            response[0] = 0x00; // DAP_Info
            response[1] = 0x00; // Length = 0 (no info yet)
            2
        }
        0xFF => {
            // DAP_Invalid - should not be sent by host
            warn!("Invalid DAP command 0xFF received");
            response[0] = 0xFF;
            1
        }
        _ => {
            // Unknown command - return command ID with error
            info!("Unimplemented DAP command: 0x{:02X}", command_id);
            response[0] = command_id;
            response[1] = 0xFF; // Error indicator
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usb::reports::DAP_PACKET_SIZE;

    #[test]
    fn test_dap_info_command() {
        let request = [0x00]; // DAP_Info command
        let mut response = [0u8; DAP_PACKET_SIZE];

        let len = process_dap_command(&request, &mut response);

        assert_eq!(len, 2);
        assert_eq!(response[0], 0x00); // Command echo
    }

    #[test]
    fn test_empty_request() {
        let request = [];
        let mut response = [0u8; DAP_PACKET_SIZE];

        let len = process_dap_command(&request, &mut response);

        assert_eq!(len, 0);
    }
}
