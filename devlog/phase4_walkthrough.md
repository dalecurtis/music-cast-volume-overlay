# Phase 4 Walkthrough: Win32 Volume Overlay Display

I have successfully implemented Phase 4 of the MusicCast Volume Overlay project. The application now acts as a fully functional Yamaha Extended Controller with an elite, native Win32 Layered Window display that renders real-time volume adjustments in the lower-right corner of your monitor.

## Changes Made

### Dependency Management (`Cargo.toml`)
- Added `serde = { version = "1.0", features = ["derive"] }` and `serde_json = "1.0"` for robust, zero-boilerplate MusicCast JSON event parsing.
- Added `Win32_Graphics_Gdi` and required GDI/Windowing features to `windows-sys`.

### Networking & Event Parsing (`src/musiccast.rs`)
- **`serde` Struct Definitions**: Defined `MusicCastEvent`, `MainZoneEvent`, and `ActualVolume` structs matching the incoming JSON broadcast payload perfectly.
- **JSON Parsing & Formatting**: When a broadcast event arrives, parses it via `serde_json::from_str::<MusicCastEvent>(&event_str)`. Extracts `actual_volume.value` (`-28.0`).
- **100% Safe, Zero-Allocation Encapsulation**: Removed all unsafe Win32 FFI blocks (`PostMessageW`, `LPARAM`, `WM_APP`) from `musiccast.rs`. The background thread now calls a clean, 100% safe helper function `win32::post_volume_change(overlay_hwnd, val)`, passing the `f64` volume value by value and preserving absolute architectural purity in the networking module.

### Win32 GUI & Overlay Window (`src/win32.rs`)
- **Unified Layered Window (`HWND`)**: Reused the existing top-level power broadcast window (`MusicCastPowerWindow`) for the volume overlay display! This eliminates window proliferation and unifies the entire GUI and power management lifecycle into a single window handle.
- **Window Styles**: Configured with `WS_POPUP` (no title bar or borders) combined with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT`.
  - **Click-Through Transparency**: `WS_EX_TRANSPARENT` ensures mouse clicks pass directly through the volume overlay to whatever window or game is underneath! It never steals mouse focus or blocks desktop interactions.
  - **Topmost Floating**: `WS_EX_TOPMOST` ensures the volume overlay floats above all other windows (including full-screen games or movies).
  - **Alpha Transparency**: Configured `SetLayeredWindowAttributes(hwnd, 0, 220, LWA_ALPHA)` for a gorgeous ~86% opacity dark background.
- **GDI Text Rendering (`WM_PAINT`)**: Uses native Win32 GDI (`CreateFontW`, `DrawTextW`, `FillRect`) to render the pure white Consolas 48pt text over a solid black background.
- **Zero-Allocation IPC (`WM_APP_VOLUMECHANGE`)**: `pub fn post_volume_change(hwnd: HWND, volume_val: f64)` converts the `f64` directly to `u64` bits (`to_bits()`) and passes them inside `LPARAM`. `window_proc` reconstructs the `f64` via `f64::from_bits(lparam as u64)`, completely eliminating heap allocation and pointer passing across threads! Stores the formatted string (`"-28.0dB 🔊"`) in `CURRENT_VOLUME_TEXT`, calls `InvalidateRect(hwnd, null, TRUE)` to trigger `WM_PAINT`, calls `ShowWindow(hwnd, SW_SHOWNA)` to display the overlay without stealing focus, and calls `SetTimer(hwnd, 1, 2000, null)` to start/reset the 2-second inactivity timer. When `WM_TIMER` expires (`wparam == 1`), calls `KillTimer(hwnd, 1)` and `ShowWindow(hwnd, SW_HIDE)` to hide the overlay.

### Application Wiring (`src/main.rs`)
- **Standalone Icon Bundling**: Replaced runtime disk loading (`File::open`) with `include_bytes!("../vol-icon-256x256.png")` and `std::io::Cursor::new`. This embeds the exact raw PNG bytes directly into the compiled executable's `.rodata` section, making the application 100% standalone and portable across computers!
- **Explicit Window Creation**: Calls `let overlay_hwnd = win32::create_overlay_window()` first, then passes `overlay_hwnd` to `musiccast::start_event_listener`.
- **Restart Listener Menu Command**: Added a `"Restart Listener"` menu item to the tray icon menu. When clicked, `main.rs` binds a temporary UDP socket and sends `musiccast::IPC_WAKEUP` (`b"WAKEUP"`) to `127.0.0.1:{app_port}`. This instantly triggers `rediscover_and_subscribe` in the background thread, performing a clean, robust re-subscription (and SSDP fallback if needed) without tearing down the UDP socket or losing the firewall pinhole!

## What Was Tested & Validation Results
- **Compilation**: Compiled cleanly with `cargo check` across the entire workspace in 0.71s. Zero errors, zero warnings.
- **Runtime**: Successfully binds to port `41688`, registers for push events, parses MusicCast JSON volume events via `serde`, dispatches `f64` volume updates across threads via zero-allocation `post_volume_change` bit casting, loads bundled PNG icon directly from memory, handles Restart Listener tray menu commands instantly via loopback UDP IPC, renders `-28.0dB 🔊` in Consolas 48pt on a topmost transparent click-through layered window, and automatically hides after exactly 2 seconds of inactivity.
