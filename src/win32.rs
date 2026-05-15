use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage, WM_POWERBROADCAST,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Win32Event {
    None,
    ResumeAutomatic,
}

/// Runs the Windows message loop until `on_event` returns `false`.
///
/// This module encapsulates all unsafe FFI calls required to interact with the Win32 API,
/// keeping the main application logic 100% safe and idiomatic Rust.
pub fn run_message_loop(mut on_event: impl FnMut(Win32Event) -> bool) {
    let mut msg: MSG = unsafe { std::mem::zeroed() };

    // GetMessageW blocks until a Windows message is available
    while unsafe { GetMessageW(&mut msg, 0 as _, 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // PBT_APMRESUMEAUTOMATIC is 0x0012
        let event = if msg.message == WM_POWERBROADCAST && msg.wParam == 0x0012 {
            Win32Event::ResumeAutomatic
        } else {
            Win32Event::None
        };

        // Check for application events after dispatching the Windows message
        if !on_event(event) {
            break;
        }
    }
}
