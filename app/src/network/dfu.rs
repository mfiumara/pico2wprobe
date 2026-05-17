use core::cell::RefCell;

use defmt::{info, warn};
use embassy_boot_rp::*;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_rp::flash::Flash;
use embassy_rp::watchdog::Watchdog;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_io_async::Read;

const FLASH_SIZE: usize = 4 * 1024 * 1024; // RP2350 Pico 2W has 4MB flash
const DFU_SERVER_PORT: u16 = 3240; // Port to listen for DFU connections
const CHUNK_SIZE: usize = 4096;

/// Network-based DFU (Device Firmware Update) task
///
/// This task listens on a TCP port for incoming firmware updates.
/// Protocol:
/// 1. Client connects to DFU_SERVER_PORT
/// 2. Client sends firmware binary data in chunks
/// 3. Server writes chunks to DFU partition
/// 4. On completion (client closes connection), marks update and resets
#[embassy_executor::task]
pub async fn dfu_task(
    stack: &'static Stack<cyw43::NetDriver<'static>>,
    flash: embassy_rp::peripherals::FLASH,
) {
    // Initialize flash access
    let flash = Flash::<_, _, FLASH_SIZE>::new_blocking(flash);
    let flash = Mutex::new(RefCell::new(flash));

    // Configure firmware updater from linker script partitions
    let config = FirmwareUpdaterConfig::from_linkerfile_blocking(&flash, &flash);
    let mut aligned = AlignedBuffer([0; 1]);
    let mut updater = BlockingFirmwareUpdater::new(config, &mut aligned.0);

    // Setup TCP socket for receiving firmware
    let mut rx_buffer = [0; 8192];
    let mut tx_buffer = [0; 1024];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(60)));

    info!("DFU task started, listening on port {}", DFU_SERVER_PORT);

    loop {
        // Wait for incoming DFU connection
        info!("DFU: Waiting for connection on port {}...", DFU_SERVER_PORT);

        if let Err(e) = socket.accept(DFU_SERVER_PORT).await {
            warn!("DFU: Accept error: {:?}", e);
            Timer::after_secs(1).await;
            continue;
        }

        info!("DFU: Client connected from {:?}", socket.remote_endpoint());

        // Start firmware update process
        if let Err(e) = handle_dfu_connection(&mut socket, &mut updater).await {
            warn!("DFU: Update failed: {:?}", e);
            socket.close();
            Timer::after_secs(1).await;
            continue;
        }

        info!("DFU: Update successful, rebooting in 2 seconds...");
        socket.close();
        Timer::after_secs(2).await;

        // Reboot to bootloader which will apply the update
        cortex_m::peripheral::SCB::sys_reset();
    }
}

async fn handle_dfu_connection<'a>(
    socket: &mut TcpSocket<'a>,
    updater: &mut BlockingFirmwareUpdater<'a>,
) -> Result<(), &'static str> {
    info!("DFU: Preparing update...");

    let writer = updater
        .prepare_update()
        .map_err(|_| "Failed to prepare update")?;

    let mut buf: AlignedBuffer<CHUNK_SIZE> = AlignedBuffer([0; CHUNK_SIZE]);
    let mut offset = 0u32;
    let mut total_bytes = 0usize;

    info!("DFU: Ready to receive firmware data");

    // Read firmware data from TCP socket and write to flash
    loop {
        match socket.read(&mut buf.0).await {
            Ok(0) => {
                // Connection closed by client - firmware transfer complete
                info!("DFU: Received {} bytes total", total_bytes);
                break;
            }
            Ok(n) => {
                // Write chunk to DFU partition
                writer
                    .write(offset, &buf.0[..n])
                    .map_err(|_| "Flash write failed")?;

                offset += n as u32;
                total_bytes += n;

                if total_bytes % (64 * 1024) == 0 {
                    info!("DFU: Written {} KB", total_bytes / 1024);
                }
            }
            Err(e) => {
                warn!("DFU: Read error: {:?}", e);
                return Err("Socket read failed");
            }
        }
    }

    // Mark the update as ready for bootloader
    info!("DFU: Marking update as ready...");
    updater.mark_updated().map_err(|_| "Failed to mark update")?;

    Ok(())
}
