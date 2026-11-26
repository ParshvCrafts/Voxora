//! Text insertion: snapshot the clipboard, put our text on it, send the paste chord, then
//! restore the user's clipboard once the target app has read it, unless they copied something
//! new in the meantime (checked via the clipboard sequence number).
//!
//! Dictated text is always tagged so Windows clipboard history (Win+V) and cloud clipboard skip it.

use std::time::Duration;

use voxora_core::context::PasteKeys;
use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, VIRTUAL_KEY, VK_CONTROL, VK_INSERT, VK_LSHIFT,
    VK_LWIN, VK_MENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_V,
};
use windows::core::w;

struct ClipGuard;

impl ClipGuard {
    /// Other apps hold the clipboard briefly all the time; retry like a polite client.
    fn open() -> Result<Self, String> {
        // Clipboard managers and Win+V hold it for tens of ms right after every change.
        for _ in 0..25 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(ClipGuard);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err("clipboard is locked by another app".into())
    }
}

impl Drop for ClipGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

/// Formats whose handles are not HGLOBAL memory (GDI objects, private/owner-drawn data).
/// CF_BITMAP etc. are re-synthesized by Windows from CF_DIB, which we do keep.
fn restorable(fmt: u32) -> bool {
    !matches!(fmt, 2 | 3 | 9 | 14 | 0x80 | 0x82 | 0x83 | 0x8E | 0x200..=0x3FF)
}

pub struct Snapshot(Vec<(u32, Vec<u8>)>);

pub fn snapshot() -> Result<Snapshot, String> {
    let _g = ClipGuard::open()?;
    let mut out = Vec::new();
    let mut fmt = 0u32;
    unsafe {
        loop {
            fmt = EnumClipboardFormats(fmt);
            if fmt == 0 {
                break;
            }
            if !restorable(fmt) {
                continue;
            }
            let Ok(h) = GetClipboardData(fmt) else { continue };
            let hg = HGLOBAL(h.0);
            let p = GlobalLock(hg);
            if p.is_null() {
                continue;
            }
            let size = GlobalSize(hg);
            out.push((fmt, std::slice::from_raw_parts(p as *const u8, size).to_vec()));
            let _ = GlobalUnlock(hg);
        }
    }
    Ok(Snapshot(out))
}

fn put(fmt: u32, bytes: &[u8]) -> Result<(), String> {
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).map_err(|e| e.to_string())?;
        let p = GlobalLock(h);
        if p.is_null() {
            return Err("GlobalLock failed".into());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
        let _ = GlobalUnlock(h);
        // On success the system owns the memory.
        SetClipboardData(fmt, Some(HANDLE(h.0))).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn restore(s: &Snapshot) -> Result<(), String> {
    let _g = ClipGuard::open()?;
    unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
    for (fmt, data) in &s.0 {
        if let Err(e) = put(*fmt, data) {
            tracing::debug!("clipboard format {fmt} not restored: {e}");
        }
    }
    Ok(())
}

/// Put `text` on the clipboard. `transient` hides it from clipboard history / cloud sync.
/// Returns the clipboard sequence number after the write.
pub fn set_text(text: &str, transient: bool) -> Result<u32, String> {
    write_text(text, transient)?;
    // Read after CloseClipboard: the sequence number only settles once the clipboard is closed.
    Ok(unsafe { GetClipboardSequenceNumber() })
}

fn write_text(text: &str, transient: bool) -> Result<(), String> {
    let _g = ClipGuard::open()?;
    unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
    put(CF_UNICODETEXT.0 as u32, bytes)?;
    if transient {
        let zero = 0u32.to_le_bytes();
        unsafe {
            for name in [
                w!("ExcludeClipboardContentFromMonitorProcessing"),
                w!("CanIncludeInClipboardHistory"),
                w!("CanUploadToCloudClipboard"),
            ] {
                let f = RegisterClipboardFormatW(name);
                if f != 0 {
                    let _ = put(f, &zero);
                }
            }
        }
    }
    Ok(())
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let mut flags = if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
    if vk == VK_INSERT {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: unsafe { MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC) } as u16,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_paste(keys: PasteKeys) -> Result<(), String> {
    // The re-paste shortcut (Ctrl+Alt+V) fires while the user still holds Ctrl and Alt; a paste
    // sent now would arrive as Ctrl+Alt+V. Wait for them to let go (they do within ~300 ms).
    // Injecting an Alt key-up instead would open the menu bar in many apps.
    for _ in 0..50 {
        let held = |vk: VIRTUAL_KEY| unsafe { GetAsyncKeyState(vk.0 as i32) } < 0;
        if !held(VK_MENU) && !held(VK_CONTROL) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut inputs = Vec::new();
    // A physically held Shift/Win would turn Ctrl+V into something else.
    for vk in [VK_LSHIFT, VK_RSHIFT, VK_SHIFT, VK_LWIN, VK_RWIN] {
        if unsafe { GetAsyncKeyState(vk.0 as i32) } < 0 {
            inputs.push(key(vk, true));
        }
    }
    let chord: &[VIRTUAL_KEY] = match keys {
        PasteKeys::CtrlV => &[VK_CONTROL, VK_V],
        PasteKeys::CtrlShiftV => &[VK_CONTROL, VK_SHIFT, VK_V],
        PasteKeys::ShiftInsert => &[VK_SHIFT, VK_INSERT],
    };
    inputs.extend(chord.iter().map(|k| key(*k, false)));
    inputs.extend(chord.iter().rev().map(|k| key(*k, true)));
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(format!("SendInput delivered {sent}/{} events", inputs.len()));
    }
    Ok(())
}

pub struct PasteOptions {
    pub keys: PasteKeys,
    pub restore_clipboard: bool,
    pub restore_delay_ms: u64,
}

/// Paste `text` into the focused window. On error the text is still on the clipboard.
pub fn paste(text: &str, opts: &PasteOptions) -> Result<(), String> {
    let saved = if opts.restore_clipboard {
        snapshot().inspect_err(|e| tracing::warn!("clipboard not saved, it will not be restored: {e}")).ok()
    } else {
        None
    };
    // Always tagged: hiding dictated text from clipboard history does not stop Ctrl+V.
    let seq = set_text(text, true)?;
    send_paste(opts.keys)?;
    if let Some(saved) = saved {
        let delay = opts.restore_delay_ms;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(delay));
            // The user (or an app) copied something new: leave it alone.
            if unsafe { GetClipboardSequenceNumber() } == seq
                && let Err(e) = restore(&saved)
            {
                tracing::warn!("clipboard restore failed: {e}");
            }
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_text() -> Option<String> {
        let _g = ClipGuard::open().ok()?;
        unsafe {
            let h = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
            let p = GlobalLock(HGLOBAL(h.0)) as *const u16;
            let mut n = 0;
            while *p.add(n) != 0 {
                n += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
            let _ = GlobalUnlock(HGLOBAL(h.0));
            Some(s)
        }
    }

    /// Real clipboard round-trip (blueprint: "clipboard preservation"). Touches the live
    /// clipboard, so it is ignored by default: `cargo test -p voxora -- --ignored`.
    #[test]
    #[ignore]
    fn snapshot_set_restore_roundtrip() {
        set_text("user's important URL https://example.com/π", false).unwrap();
        let snap = snapshot().unwrap();
        let seq = set_text("dictated text", true).unwrap();
        assert_eq!(read_text().as_deref(), Some("dictated text"));
        assert_eq!(unsafe { GetClipboardSequenceNumber() }, seq);
        restore(&snap).unwrap();
        assert_eq!(read_text().as_deref(), Some("user's important URL https://example.com/π"));
    }

    #[test]
    fn gdi_formats_are_skipped() {
        assert!(!restorable(2)); // CF_BITMAP
        assert!(!restorable(0x300));
        assert!(restorable(13)); // CF_UNICODETEXT
        assert!(restorable(8)); // CF_DIB
        assert!(restorable(0xC0FF)); // registered formats (HTML Format, PNG)
    }
}
