use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};

const SERVER_HOST: &str = "echo.websocket.org";
const SERVER_PORT: u16 = 80;

#[embassy_executor::task]
pub async fn tcp_client_task(_spawner: Spawner, stack: &'static embassy_net::Stack<'static>) {
    // Wait for network to be ready
    stack.wait_config_up().await;
    info!("Network is ready, starting TCP client");

    loop {
        // Create TCP socket with buffers
        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];
        let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);

        info!("Resolving hostname: {}", SERVER_HOST);

        // Resolve hostname
        let remote_endpoint: embassy_net::IpEndpoint = match stack
            .dns_query(SERVER_HOST, embassy_net::dns::DnsQueryType::A)
            .await
        {
            Ok(addresses) => {
                if let Some(addr) = addresses.iter().find_map(|addr| {
                    if let embassy_net::IpAddress::Ipv4(ipv4) = addr {
                        Some(*ipv4)
                    } else {
                        None
                    }
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

                // Send a simple HTTP request
                let request =
                    "GET / HTTP/1.1\r\nHost: echo.websocket.org\r\nConnection: close\r\n\r\n";

                match socket.write_all(request.as_bytes()).await {
                    Ok(()) => {
                        info!("HTTP request sent: {}", request.trim());

                        // Read response
                        let mut response_buffer = [0u8; 2048];
                        match socket.read(&mut response_buffer).await {
                            Ok(len) => {
                                if len > 0 {
                                    let response = core::str::from_utf8(&response_buffer[..len])
                                        .unwrap_or("<invalid utf8>");
                                    info!("Received response ({} bytes):\n{}", len, response);
                                } else {
                                    info!("Connection closed by server");
                                }
                            }
                            Err(e) => {
                                error!("Failed to read response: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to send HTTP request: {:?}", e);
                    }
                }

                socket.close();
                info!("TCP connection closed");
            }
            Err(e) => {
                error!("TCP connection failed: {:?}", e);
            }
        }

        info!("Waiting 30 seconds before next connection attempt...");
        Timer::after(Duration::from_secs(30)).await;
    }
}
