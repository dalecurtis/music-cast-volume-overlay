use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject,
    DrawTextW, EndPaint, FW_NORMAL, FillRect, GetDeviceCaps, InvalidateRect, LOGPIXELSY,
    PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::Console::FreeConsole;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, KillTimer,
    LWA_ALPHA, MSG, PostMessageW, RegisterClassExW, SPI_GETWORKAREA, SW_HIDE, SW_SHOWNA,
    SetLayeredWindowAttributes, SetTimer, ShowWindow, SystemParametersInfoW, TranslateMessage,
    WM_APP, WM_PAINT, WM_POWERBROADCAST, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Win32Event {
    None,
    ResumeAutomatic,
}

const WM_APP_RESUMEAUTOMATIC: u32 = WM_APP + 1;
const WM_APP_VOLUMECHANGE: u32 = WM_APP + 3;
static mut CURRENT_VOLUME_TEXT: Option<String> = None;

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
        if msg == WM_POWERBROADCAST && wparam == 0x0012 {
            // PBT_APMRESUMEAUTOMATIC is 0x0012
            PostMessageW(hwnd, WM_APP_RESUMEAUTOMATIC, 0, 0);
            return 1;
        } else if msg == WM_APP_VOLUMECHANGE {
            // Reconstruct f64 directly from LPARAM bits (zero allocation IPC)
            let val = f64::from_bits(lparam as u64);
            CURRENT_VOLUME_TEXT = Some(format!("{:.1}dB 🔊", val));
            InvalidateRect(hwnd, std::ptr::null(), 1); // TRUE is 1
            ShowWindow(hwnd, SW_SHOWNA); // Show without activating/stealing focus
            SetTimer(hwnd, 1, 2000, None); // Set/reset 2-second inactivity timer
            return 0;
        } else if msg == WM_TIMER && wparam == 1 {
            KillTimer(hwnd, 1);
            ShowWindow(hwnd, SW_HIDE);
            return 0;
        } else if msg == WM_PAINT {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Fill background with solid black
            let hbrush = CreateSolidBrush(0x00000000);
            FillRect(hdc, &ps.rcPaint, hbrush);
            DeleteObject(hbrush as _);

            if let Some(ref text) = CURRENT_VOLUME_TEXT {
                SetTextColor(hdc, 0x00FFFFFF); // Pure white text
                SetBkMode(hdc, TRANSPARENT as _);

                // Calculate 48pt font height in pixels
                let dpi_y = GetDeviceCaps(hdc, LOGPIXELSY as _);
                let font_height = -((48 * dpi_y) / 72);

                let font_name = to_pcwstr("Consolas");
                let hfont = CreateFontW(
                    font_height,
                    0,
                    0,
                    0,
                    FW_NORMAL as _,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    font_name.as_ptr(),
                );

                let old_font = SelectObject(hdc, hfont as _);

                let mut client_rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut client_rect);

                let text_w = to_pcwstr(text);
                DrawTextW(
                    hdc,
                    text_w.as_ptr(),
                    (text_w.len() - 1) as i32, // Exclude null terminator
                    &mut client_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                SelectObject(hdc, old_font);
                DeleteObject(hfont as _);
            }

            EndPaint(hwnd, &ps);
            return 0;
        }

        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
}

/// Detaches the process from its parent console window.
pub fn free_console() {
    unsafe {
        FreeConsole();
    }
}

/// Posts a volume change message to the overlay window from a background thread.
///
/// This helper passes the `f64` volume value directly inside `LPARAM` via `to_bits()`,
/// achieving 100% zero-allocation cross-thread IPC.
pub fn post_volume_change(hwnd: HWND, volume_val: f64) {
    let bits = volume_val.to_bits();
    unsafe {
        PostMessageW(hwnd, WM_APP_VOLUMECHANGE, 0, bits as LPARAM);
    }
}

/// Creates the unified Win32 layered popup window for power monitoring and volume overlay display.
pub fn create_overlay_window() -> HWND {
    let class_name = to_pcwstr("MusicCastOverlayClass");
    let window_name = to_pcwstr("MusicCastOverlayWindow");

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

        let mut work_area: RECT = std::mem::zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work_area as *mut _ as _, 0);

        let width = 400;
        let height = 120;
        let x = work_area.right - width - 30;
        let y = work_area.bottom - height - 30;

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_POPUP, // Starts out hidden (no WS_VISIBLE)
            x,
            y,
            width,
            height,
            0,
            0,
            hinstance,
            std::ptr::null(),
        );

        // Set alpha transparency to ~86% (220/255)
        SetLayeredWindowAttributes(hwnd, 0, 220, LWA_ALPHA);

        return hwnd;
    }
}

/// Runs the Windows message loop until `on_event` returns `false`.
pub fn run_message_loop(_hwnd: HWND, mut on_event: impl FnMut(Win32Event) -> bool) {
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
