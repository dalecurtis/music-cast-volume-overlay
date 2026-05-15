use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

const MULTICAST_ADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(239, 255, 255, 250), 1900);
const NETWORK_TIMEOUT: Duration = Duration::from_secs(3);

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

    // Each response is one UDP datagram, so we don't need to worry about the
    // headers being split across multiple socket receive events.
    while start_time.elapsed() < NETWORK_TIMEOUT {
        if let Ok((amt, src_addr)) = socket.recv_from(&mut buf) {
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

/// Fetches and prints the status of the MusicCast receiver.
pub fn get_status(receiver: &MusicCastReceiver) {
    let url = format!(
        "http://{}/YamahaExtendedControl/v1/main/getStatus",
        receiver.ip
    );
    println!("Fetching receiver status from: {}", url);

    match ureq::get(&url).timeout(NETWORK_TIMEOUT).call() {
        Ok(response) => match response.into_string() {
            Ok(body) => {
                println!("--- MusicCast Receiver Status ---");
                println!("{}", body);
                println!("---------------------------------");
            }
            Err(e) => println!("Failed to read status response body: {}", e),
        },
        Err(e) => println!("Failed to fetch receiver status: {}", e),
    }
}
