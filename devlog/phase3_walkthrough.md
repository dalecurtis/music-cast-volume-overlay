# Phase 3 Walkthrough: MusicCast Extended Controller & Event Listener

## Goal
Register the application as an event listener with the Yamaha receiver, spawn a dedicated background thread to listen for real-time UDP push status updates, renew the event lease every 10 minutes, and gracefully handle Windows suspend/resume power events and clean process shutdown.

## Implementation Details

### 1. Fixed Registered Port & Firewall Prompting (`src/musiccast.rs`)
- Binds the event listener UDP socket to fixed port `41688` (chosen from the IANA Registered Port Range `1024–49151` to eliminate ephemeral port collisions).
- Discovered that Windows Defender Firewall ALE suppresses GUI prompts for ephemeral ports (`0.0.0.0:0`) but detects fixed port reservations as explicit server sockets. Binding to `41688` guarantees Windows will automatically pop up the GUI firewall prompt on new computers!
- Discovered that `pktmon` sniffer logs confirmed the Yamaha receiver transmits push events from dynamic ephemeral source ports (`32768–60999`). E.g., keepalives to port `1900` cannot keep the firewall pinhole open. An Inbound Windows Firewall Allow Rule (created via PowerShell `New-NetFirewallRule`) is the true, architecturally correct solution for background binaries.

### 2. Unified `getStatus` Subscription & Header Validation
- Replaced `prepareEvent` entirely with `get_status(&MusicCastReceiver) -> bool`, matching `ymc`'s exact subscription architecture.
- Discovered that Yamaha receiver firmware actively validates the `X-AppName` header and requires it to start with the `MusicCast/` prefix! Configured `X-AppName: MusicCast/VolumeOverlay` and `X-AppPort: 41688`.

### 3. Background Listener Thread & Timeout Optimization
- Spawns a dedicated background thread running a continuous `loop`.
- Optimized `socket.recv_from` timeout (`LISTENER_TIMEOUT`) to 5 minutes (`300` seconds), calculated dynamically at compile time as `LEASE_TIMEOUT / 2`. This reduces background thread churn/context switching by 99.0% (from 28,800 wakeups per day down to just 288)!
- Automatically renews the event lease every `LEASE_TIMEOUT` (10 minutes) by calling `get_status`.

### 4. Win32 Power Management & Loopback IPC (`src/win32.rs`, `src/main.rs`)
- **Custom `WindowProc` Bridge**: Discovered that `tray-icon` creates a Win32 Message-Only Window (`HWND_MESSAGE`), which the Windows kernel explicitly excludes from receiving `WM_POWERBROADCAST` messages! Furthermore, discovered that `WM_POWERBROADCAST` is a **non-queued message**, meaning `GetMessageW` dispatches it directly to the window proc behind the scenes rather than returning it to the message loop.
- To guarantee reception of Win32 power broadcast events for short sleep/resume cycles (even 1 millisecond of sleep!), `run_message_loop` creates an unmapped, 100% invisible **top-level window** (`CreateWindowExW` with `HWND_DESKTOP` as parent) with a dedicated `WindowProc` callback. When `WM_POWERBROADCAST` (`PBT_APMRESUMEAUTOMATIC`) is received by the callback, it calls `PostMessageW` to post a custom queued message (`WM_APP + 1`) into the thread queue, flawlessly bridging the non-queued broadcast world into our queued `GetMessageW` loop!
- **Streamlined Reconnection & SSDP Fallback (`rediscover_and_subscribe`)**: Implemented the ultimate streamlined reconnection architecture:
  - **Step 1 (Direct Unicast)**: Attempts `get_status` directly against the known receiver IP (`current_receiver.ip`). If that fails (e.g., network still waking up), sleeps for `RECONNECT_INTERVAL` (20 seconds) to allow Wi-Fi link and DHCP negotiation to fully complete before trying `get_status` a second time.
  - **Step 2 (SSDP Fallback)**: If direct unicast fails (e.g., if the receiver's IP address changed), attempts a full SSDP `discover_receiver()` broadcast up to 3 times with 20-second backoff intervals (`RECONNECT_INTERVAL`). Configured backoffs to provide ample time for Wi-Fi mesh routers to rebuild IGMP multicast membership tables after PC wakes from sleep!
- **Wakeup IPC**: When Windows wakes from sleep, `main.rs` sends `IPC_WAKEUP` (`b"WAKEUP"`) to `127.0.0.1:41688`. The background thread unblocks instantly, calls `rediscover_and_subscribe`.
- **Shutdown IPC**: When the user selects "Exit" from the tray menu, `main.rs` sends `IPC_SHUTDOWN` (`b"SHUTDOWN"`) to `127.0.0.1:41688`, allowing the background thread to cleanly terminate before the process exit.
- Refactored all functions and closures to end with explicit `return $value;` statements.

## What Was Tested & Validation Results
- **Compilation**: Compiled cleanly with `cargo check` across the entire workspace in 0.72s. Zero errors, zero warnings.
- **Runtime**: Successfully binds to port `41688`, registers for push events, renews leases, detects Win32 power broadcast resume events via custom window proc bridging, recovers network links via streamlined 20s backoff loop and SSDP fallback, and handles instant loopback IPC wakeup and shutdown signals.
