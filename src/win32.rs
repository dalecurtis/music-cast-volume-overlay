use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, PostMessageW,
    RegisterClassExW, TranslateMessage, WM_APP, WM_POWERBROADCAST, WNDCLASSEXW,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Win32Event {
    None,
    ResumeAutomatic,
}

const WM_APP_RESUMEAUTOMATIC: u32 = WM_APP + 1;

fn to_pcwstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        // PBT_APMRESUMEAUTOMATIC is 0x0012
        if msg == WM_POWERBROADCAST && wparam == 0x0012 {
            PostMessageW(hwnd, WM_APP_RESUMEAUTOMATIC, 0, 0);
        }
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
}

/// Runs the Windows message loop until `on_event` returns `false`.
///
/// Creates a custom hidden top-level window with a dedicated `WindowProc` to intercept non-queued
/// Win32 power broadcast messages (`WM_POWERBROADCAST`) and post them to the thread message queue,
/// keeping the main application logic 100% safe and idiomatic Rust.
pub fn run_message_loop(mut on_event: impl FnMut(Win32Event) -> bool) {
    let class_name = to_pcwstr("MusicCastPowerClass");
    let window_name = to_pcwstr("MusicCastPowerWindow");

    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: 0,
        };

        RegisterClassExW(&wnd_class);

        // Create an invisible top-level window whose sole purpose is to receive WM_POWERBROADCAST messages
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            0, // WS_OVERLAPPED (0), no WS_VISIBLE
            0,
            0,
            0,
            0,
            0, // HW_DESKTOP (top-level window)
            0,
            hinstance,
            std::ptr::null(),
        );
    }

    let mut msg: MSG = unsafe { std::mem::zeroed() };

    // GetMessageW blocks until a Windows message is available
    while unsafe { GetMessageW(&mut msg, 0 as _, 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let event = if msg.message == WM_APP_RESUMEAUTOMATIC {
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
