# Phase 1 Walkthrough: Project Initialization & Native Tray Icon

## Goal
Initialize a lightweight, robust Windows Rust project that runs in the background with a system tray icon, allowing the user to right-click the tray icon to cleanly exit the application.

## Implementation Details

### 1. Dependency Minimization (`Cargo.toml`)
- Configured `tray-icon = "0.24"` for native Windows taskbar tray integration.
- Replaced the heavy `image` crate with the lightweight `png = "0.18"` crate (which is already pulled in by `tray-icon`), keeping the dependency tree minimal and compile times fast.
- Configured `[profile.release]` with `opt-level = "z"`, `lto = true`, and `codegen-units = 1` to produce an ultra-tiny release binary size (~320 KB).
- Configured `.cargo/config.toml` to use the custom LLVM linker (`lld-link.exe`) to prevent PATH conflicts with GNU Coreutils.

### 2. Native Win32 Message Loop (`src/win32.rs`)
- Replaced heavy windowing frameworks (`winit` / `tao`) with a direct, lightweight Win32 message loop using `windows-sys`.
- Encapsulated unsafe Win32 FFI calls (`GetMessageW`, `TranslateMessage`, `DispatchMessageW`) inside a clean, 100% safe Rust helper function `run_message_loop`.

### 3. Application Entry Point (`src/main.rs`)
- Implemented `load_icon` using `png::Decoder` to load `vol-icon-256x256.png` by value at runtime.
- Set up the tray icon menu with an "Exit" item.
- Wired the non-blocking crossbeam event receivers (`MenuEvent::receiver()` and `TrayIconEvent::receiver()`) into the Win32 message loop closure.

## Verification Results
- **Compilation**: Compiled cleanly with `cargo check` and `cargo build`.
- **Runtime**: Successfully creates the tray icon in the Windows taskbar. Right-clicking the icon and selecting "Exit" cleanly terminates the Win32 message loop and exits the process.
