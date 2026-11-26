//! macOS window and process helpers: the non-activating HUD, and cleanup of child servers.

use std::path::PathBuf;
use std::process::Child;

use objc2_app_kit::{NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior};
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

/// AppKit objects belong to the main thread; engine code runs on worker threads.
fn on_ns_window(w: &WebviewWindow, f: impl FnOnce(&NSWindow) + Send + 'static) {
    let win = w.clone();
    let _ = w.run_on_main_thread(move || {
        if let Ok(ptr) = win.ns_window() {
            // SAFETY: Tauri hands out the live NSWindow* of this webview window, and we are on
            // the main thread as AppKit requires.
            let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
            f(ns);
        }
    });
}

/// Float above everything, on every Space and over full-screen apps, and let clicks through.
pub fn make_overlay(w: &WebviewWindow) {
    on_ns_window(w, |ns| {
        ns.setLevel(NSStatusWindowLevel);
        ns.setIgnoresMouseEvents(true);
        ns.setHasShadow(false);
        ns.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary,
        );
    });
}

/// Bottom-center of the screen the pointer is on, sized `w_pt` x `h_pt`. `orderFrontRegardless`
/// shows the window without activating Voxora, so the app being dictated into keeps keyboard focus.
pub fn show_overlay(w: &WebviewWindow, w_pt: f64, h_pt: f64, bottom_margin_pt: i32) {
    let monitor = w
        .cursor_position()
        .ok()
        .and_then(|p| w.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let (width, height) = ((w_pt * scale).round() as i32, (h_pt * scale).round() as i32);
        let _ = w.set_size(PhysicalSize::new(width as u32, height as u32));
        let area = m.work_area();
        let x = area.position.x + (area.size.width as i32 - width) / 2;
        let y = area.position.y + area.size.height as i32 - height - (bottom_margin_pt as f64 * scale).round() as i32;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    on_ns_window(w, |ns| ns.orderFrontRegardless());
}

pub fn hide_overlay(w: &WebviewWindow) {
    on_ns_window(w, |ns| ns.orderOut(None));
}

fn pid_file() -> PathBuf {
    std::env::temp_dir().join("voxora-children.pids")
}

/// macOS has no kill-on-close job objects. Record each server's PID so the next launch can
/// stop servers a crashed Voxora left behind (they would otherwise hold gigabytes of memory).
pub fn adopt_child(child: &Child) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(pid_file()) {
        let _ = writeln!(f, "{}", child.id());
    }
}

/// Stop servers from a previous run. Only kills processes that are still our server binaries.
pub fn reap_orphans() {
    let Ok(list) = std::fs::read_to_string(pid_file()) else { return };
    for pid in list.lines().filter_map(|l| l.trim().parse::<u32>().ok()) {
        let name = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        if name.ends_with("whisper-server") || name.ends_with("llama-server") {
            tracing::warn!("stopping orphaned {name} (pid {pid})");
            let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
        }
    }
    let _ = std::fs::remove_file(pid_file());
}
