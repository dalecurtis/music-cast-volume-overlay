# Phase 2 Walkthrough: SSDP Discovery & HTTP API Verification

## Goal
Identify the Yamaha MusicCast receiver on the local network via SSDP (UDP port 1900), print its discovered IP address, and fetch its initial status via the Yamaha Extended Control (YXC) HTTP API.

## Implementation Details

### 1. Blocking HTTP Client (`Cargo.toml`)
- Added `ureq = { version = "2.10", default-features = false }` for lightweight, blocking HTTP GET requests, avoiding heavy async runtime dependencies like `tokio` or `reqwest`.

### 2. SSDP M-SEARCH Discovery (`src/musiccast.rs`)
- Implemented `discover_receiver() -> Option<MusicCastReceiver>` using `std::net::UdpSocket`.
- Binds cleanly to `0.0.0.0:0` and broadcasts a standard UPnP `M-SEARCH` datagram targeting `ST: urn:schemas-upnp-org:device:MediaRenderer:1` to the multicast group `239.255.255.250:1900`.
- Configured a 3-second network timeout (`NETWORK_TIMEOUT`) for socket reads.

### 3. Zero-Allocation Substring Filtering
- Discovered that Yamaha receivers include `X-ModelName: RX-A` (or similar model headers) in their UDP SSDP response packets.
- Implemented a 100% zero-allocation, zero-copy raw byte substring search (`buf[..amt].windows(...).any(...)`) directly on the stack-allocated UDP packet buffer. This instantly filters out non-Yamaha devices (like Google Chromecasts or Sonos speakers) in nanoseconds without allocating strings or making unnecessary HTTP calls.

### 4. YXC API Status Fetching
- Implemented `get_status(&receiver)` to query `http://{IP}/YamahaExtendedControl/v1/main/getStatus`.
- Dumps the beautifully formatted JSON response body to the console to verify full network connectivity.

## Verification Results
- **Compilation**: Compiled cleanly with `cargo check`.
- **Runtime**: Successfully broadcasts SSDP, identifies the Yamaha receiver at `192.168.86.44`, and prints the full MusicCast status JSON to the console during startup.
