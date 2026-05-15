use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

const MULTICAST_ADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(239, 255, 255, 250), 1900);
const NETWORK_TIMEOUT: Duration = Duration::from_secs(3);

// MusicCast API requires us to renew our lease every 10 minutes to get updates.
const LEASE_TIMEOUT: Duration = Duration::from_mins(10);
const LISTENER_TIMEOUT: Duration = Duration::from_secs(LEASE_TIMEOUT.as_secs() / 2);

// We use a fixed port instead of an ephemeral port (0.0.0.0:0) to ensure Windows Defender Firewall exception
// prompt is generated. Otherwise packets coming back from the receiver will be dropped.
const APP_UDP_PORT: u16 = 41688;

pub const IPC_WAKEUP: &[u8] = b"WAKEUP";
pub const IPC_SHUTDOWN: &[u8] = b"SHUTDOWN";

const SSDP_MSEARCH: &[u8] = concat!(
    "M-SEARCH * HTTP/1.1\r\n",
    "HOST: 239.255.255.250:1900\r\n",
    "MAN: \"ssdp:discover\"\r\n",
    "MX: 3\r\n",
    "ST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n",
    "\r\n",
)
.as_bytes();

const MODEL_HEADER: &[u8] = b"X-ModelName: RX-A";

#[derive(Clone, Debug)]
pub struct MusicCastReceiver {
    pub ip: String,
}

/// Discovers a Yamaha MusicCast receiver on the local network using SSDP.
pub fn discover_receiver() -> Option<MusicCastReceiver> {
    println!("Starting SSDP discovery for MusicCast receiver...");

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind SSDP UDP socket");
    socket
        .set_read_timeout(Some(NETWORK_TIMEOUT))
        .expect("Failed to set UDP read timeout");
    socket
        .set_broadcast(true)
        .expect("Failed to set broadcast flag");

    if let Err(e) = socket.send_to(SSDP_MSEARCH, MULTICAST_ADDR) {
        println!("Failed to send SSDP M-SEARCH: {}", e);
        return None;
    }

    let mut buf = [0u8; 2048];
    let start_time = Instant::now();

    while start_time.elapsed() < NETWORK_TIMEOUT {
        if let Ok((amt, src_addr)) = socket.recv_from(&mut buf) {
            // Zero-allocation, zero-copy raw byte substring search
            if buf[..amt]
                .windows(MODEL_HEADER.len())
                .any(|w| w == MODEL_HEADER)
            {
                let ip = src_addr.ip().to_string();
                println!("Discovered MusicCast receiver at IP: {}", ip);
                return Some(MusicCastReceiver { ip });
            }
        }
    }

    println!("SSDP discovery timed out. No MusicCast receiver found.");
    return None;
}

/// Fetches and prints the status of the MusicCast receiver, attaching event subscription headers.
/// Returns true if the status was successfully fetched.
fn get_status(receiver: &MusicCastReceiver) -> bool {
    let url = format!(
        "http://{}/YamahaExtendedControl/v1/main/getStatus",
        receiver.ip
    );
    println!("Fetching receiver status from: {}", url);

    // X-AppName must begin with "MusicCast/" or the receiver will drop the request.
    match ureq::get(&url)
        .set("X-AppName", "MusicCast/VolumeOverlay")
        .set("X-AppPort", &APP_UDP_PORT.to_string())
        .timeout(NETWORK_TIMEOUT)
        .call()
    {
        Ok(response) => {
            let success = response.status() == 200;
            match response.into_string() {
                Ok(body) => {
                    println!("--- MusicCast Receiver Status ---");
                    println!("{}", body);
                    println!("---------------------------------");
                }
                Err(e) => println!("Failed to read status response body: {}", e),
            }
            return success;
        }
        Err(e) => {
            println!("Failed to fetch receiver status: {}", e);
            return false;
        }
    }
}

/// Spawns a dedicated background thread to listen for real-time MusicCast UDP broadcast events.
/// Returns the bound local UDP port number for synthetic control messages.
pub fn start_event_listener(initial_receiver: MusicCastReceiver) -> u16 {
    let socket = match UdpSocket::bind(("0.0.0.0", APP_UDP_PORT)) {
        Ok(s) => s,
        Err(e) => {
            println!(
                "Failed to bind to fixed UDP port {}. Is another instance already running? Error: {}",
                APP_UDP_PORT, e
            );
            return 0;
        }
    };
    socket
        .set_read_timeout(Some(LISTENER_TIMEOUT))
        .expect("Failed to set UDP read timeout");

    println!("Event listener bound to fixed UDP port: {}", APP_UDP_PORT);

    std::thread::spawn(move || {
        let mut current_receiver = initial_receiver;
        get_status(&current_receiver);
        let mut last_registration = Instant::now();

        let mut buf = [0u8; 4096];

        loop {
            match socket.recv_from(&mut buf) {
                Ok((amt, src_addr)) => {
                    // Check if it's a synthetic IPC message from our main thread
                    if src_addr.ip().is_loopback() {
                        if &buf[..amt] == IPC_WAKEUP {
                            println!(
                                "Synthetic WAKEUP received. Waiting 3 seconds for network link/DHCP..."
                            );
                            std::thread::sleep(Duration::from_secs(3));

                            if let Some(new_receiver) = discover_receiver() {
                                current_receiver = new_receiver;
                                get_status(&current_receiver);
                                last_registration = Instant::now();
                            } else {
                                println!("Re-discovery failed after wakeup.");
                            }
                            continue;
                        } else if &buf[..amt] == IPC_SHUTDOWN {
                            println!("Shutdown signal received. Stopping event listener thread...");
                            break;
                        }
                    }

                    // Otherwise, it's a genuine MusicCast broadcast event
                    let event_str = String::from_utf8_lossy(&buf[..amt]);
                    println!(
                        "--- MusicCast Broadcast Event Received from {} ---",
                        src_addr
                    );
                    println!("{}", event_str);
                    println!("--------------------------------------------------");
                }
                Err(_) => {
                    // Read timeout elapsed (every LISTENER_TIMEOUT)
                }
            }

            let now = Instant::now();

            // Every 10 minutes (LEASE_TIMEOUT), re-register the endpoint to prevent lease timeout
            if now.duration_since(last_registration) >= LEASE_TIMEOUT {
                println!("10-minute lease renewing...");
                if get_status(&current_receiver) {
                    last_registration = now;
                }
            }
        }
    });

    return APP_UDP_PORT;
}
