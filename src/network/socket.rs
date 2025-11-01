use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
// use embedded_io_async::{Read, Write};

use crate::probe::cbindings::DAP_ProcessCommand;
// For testing TLS with tcpbin.com
// const SERVER_HOST: &str = "tcpbin.com";
// const SERVER_PORT: u16 = 4242;

// For testing RPC locally
const SERVER_HOST: &str = "192.168.2.9";
const SERVER_PORT: u16 = 8080;

#[embassy_executor::task]
pub async fn tcp_client_task(_spawner: Spawner, stack: &'static embassy_net::Stack<'static>) {
    // Wait for network to be ready
    stack.wait_config_up().await;
    info!("Network is ready, starting TCP client");

    loop {
        // Create TCP socket with buffers
        let mut tcp_rx_buffer = [0; 4096];
        let mut tcp_tx_buffer = [0; 4096];
        let mut socket = TcpSocket::new(*stack, &mut tcp_rx_buffer, &mut tcp_tx_buffer);

        info!("Resolving hostname: {}", SERVER_HOST);

        // Resolve hostname
        let remote_endpoint: embassy_net::IpEndpoint = match stack
            .dns_query(SERVER_HOST, embassy_net::dns::DnsQueryType::A)
            .await
        {
            Ok(addresses) => {
                if let Some(addr) = addresses
                    .iter()
                    .map(|addr| {
                        let embassy_net::IpAddress::Ipv4(ipv4) = addr;
                        *ipv4
                    })
                    .next()
                {
                    info!("Resolved {} to {}", SERVER_HOST, addr);
                    (addr, SERVER_PORT).into()
                } else {
                    error!("No IPv4 address found for {}", SERVER_HOST);
                    Timer::after(Duration::from_secs(10)).await;
                    continue;
                }
            }
            Err(e) => {
                error!("DNS query failed: {:?}", e);
                Timer::after(Duration::from_secs(10)).await;
                continue;
            }
        };

        info!("Connecting to {}:{}", SERVER_HOST, SERVER_PORT);

        // Connect to the server
        match socket.connect(remote_endpoint).await {
            Ok(()) => {
                info!("TCP connection established!");
                loop {
                    let mut rx_buffer = [0; 64];
                    let mut tx_buffer = [0; 64];

                    // Add timeout to socket read to detect disconnections
                    let read_result = embassy_time::with_timeout(
                        Duration::from_secs(30),
                        socket.read(&mut rx_buffer)
                    ).await;

                    match read_result {
                        Ok(Ok(len)) => {
                            if len > 0 {
                                debug!("Received DAP command: {} bytes", len);

                                // Process DAP command with timeout to prevent hanging
                                let process_result = embassy_time::with_timeout(
                                    Duration::from_millis(5000),
                                    async {
                                        // Process the DAP command using the C library
                                        // Note: DAP_ProcessCommand returns a packed u32:
                                        //   - Lower 16 bits: response length
                                        //   - Upper 16 bits: request length
                                        unsafe {
                                            DAP_ProcessCommand(rx_buffer.as_ptr(), tx_buffer.as_mut_ptr())
                                        }
                                    }
                                ).await;

                                match process_result {
                                    Ok(result) => {
                                        let response_len = (result & 0xFFFF) as usize;

                                        // Validate response length doesn't exceed buffer size
                                        if response_len > tx_buffer.len() {
                                            error!(
                                                "DAP_ProcessCommand returned invalid response length: {} (max: {})",
                                                response_len,
                                                tx_buffer.len()
                                            );
                                        } else if response_len > 0 {
                                            debug!("DAP response ({} bytes): {:x}", response_len, &tx_buffer[..response_len]);
                                            match socket.write(&tx_buffer[..response_len]).await {
                                                Ok(len) => {
                                                    if len > 0 {
                                                        debug!("Sent {} bytes to server", len);
                                                    }
                                                }
                                                Err(e) => {
                                                    error!("Failed to send data: {:?}", e);
                                                    break; // Exit loop to reconnect
                                                }
                                            }
                                        } else {
                                            debug!("DAP_ProcessCommand returned empty response for command: {:x}", &rx_buffer[..len]);
                                        }
                                    }
                                    Err(_) => {
                                        error!("DAP_ProcessCommand timed out after 5s - command: {:x}", &rx_buffer[..len]);
                                        break; // Exit loop to reconnect
                                    }
                                }
                            } else {
                                // len == 0 means connection closed
                                info!("Server closed connection");
                                break;
                            }
                        }
                        Ok(Err(e)) => {
                            error!("Failed to read data: {:?}", e);
                            break; // Exit loop to reconnect
                        }
                        Err(_) => {
                            error!("Socket read timeout after 30s - connection may be dead");
                            break; // Exit loop to reconnect
                        }
                    }
                }
            }
            Err(e) => {
                error!("TCP connection failed: {:?}", e);
            }
        }
        info!("TCP socket closed, waiting for 10 seconds before re-opening...");
        Timer::after(Duration::from_secs(10)).await;
    }
}
