use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
// use embedded_io_async::{Read, Write};

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
                if let Some(addr) = addresses.iter().find_map(|addr| {
                    let embassy_net::IpAddress::Ipv4(ipv4) = addr;
                    Some(*ipv4)
                }) {
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
                    let mut rx_buffer = [0; 1024];
                    let mut tx_buffer = [0; 1024];

                    match socket.read(&mut rx_buffer).await {
                        Ok(len) => {
                            if len > 0 {
                                info!(
                                    "Received data: {}",
                                    core::str::from_utf8(&rx_buffer[..len])
                                        .unwrap_or("<invalid utf8>")
                                );
                            }
                        }
                        Err(e) => {
                            error!("Failed to read data: {:?}", e);
                        }
                    }
                    match socket.write(&mut tx_buffer).await {
                        Ok(len) => {
                            if len > 0 {
                                info!(
                                    "Sent data: {}",
                                    core::str::from_utf8(&tx_buffer[..len])
                                        .unwrap_or("<invalid utf8>")
                                );
                            }
                        }
                        Err(e) => {
                            error!("Failed to send data: {:?}", e);
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
