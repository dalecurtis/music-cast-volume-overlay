mod musiccast;
mod win32;

use std::net::UdpSocket;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};
use win32::Win32Event;

fn load_icon(path: &std::path::Path) -> Icon {
    let file = std::fs::File::open(path).expect("Failed to open icon file");
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().expect("Failed to read PNG info");
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader
        .next_frame(&mut buf)
        .expect("Failed to read PNG frame");

    return Icon::from_rgba(buf, info.width, info.height)
        .expect("Failed to create tray icon from RGBA");
}

fn main() {
    let menu = Menu::new();
    let exit_item = MenuItem::new("Exit", /*enabled=*/ true, None);
    let _ = menu.append(&exit_item);

    let icon = load_icon(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vol-icon-256x256.png"
    )));

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("MusicCast Volume Overlay")
        .with_icon(icon)
        .build()
        .unwrap();

    println!("Tray icon created successfully. Right-click the tray icon to exit.");

    // Run Phase 2 discovery and start Phase 3 event listener. Exit early if discovery fails.
    let app_port = if let Some(receiver) = musiccast::discover_receiver() {
        let port = musiccast::start_event_listener(receiver);
        if port == 0 {
            println!("Exiting application due to port binding failure.");
            return;
        }
        port
    } else {
        println!("Could not identify MusicCast receiver on the network. Exiting application.");
        return;
    };

    win32::run_message_loop(|win32_event| {
        if win32_event == Win32Event::ResumeAutomatic {
            println!("Win32 power resume detected. Sending synthetic WAKEUP packet...");
            if let Ok(sock) = UdpSocket::bind("127.0.0.1:0") {
                let _ = sock.send_to(musiccast::IPC_WAKEUP, format!("127.0.0.1:{}", app_port));
            }
        }

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            println!("Menu event received: {:?}", event);
            if event.id == exit_item.id() {
                println!("Exiting application...");
                if let Ok(sock) = UdpSocket::bind("127.0.0.1:0") {
                    let _ =
                        sock.send_to(musiccast::IPC_SHUTDOWN, format!("127.0.0.1:{}", app_port));
                }
                return false;
            }
        }

        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            println!("Tray event received: {:?}", event);
        }

        return true;
    });
}
