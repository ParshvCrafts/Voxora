//! Global hotkeys via a `WH_KEYBOARD_LL` hook on a dedicated thread with its own message pump.
//!
//! The hook procedure does the minimum (classify + send on a channel) so Windows never evicts it
//! for being slow. The dictation key is swallowed entirely so a held Right Alt never reaches the
//! focused app (no menu bar flashes, no stray characters). Injected events (our own Ctrl+V) are
//! ignored.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_MENU};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, SetWindowsHookExW,
    TranslateMessage, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// Dictation key pressed; `at_ms` is the OS event time.
    Down {
        at_ms: u64,
    },
    Up {
        at_ms: u64,
    },
    Escape,
    CycleMode,
    Repaste,
}

struct HookState {
    sink: Box<dyn Fn(HotkeyEvent) + Send + Sync>,
    dictation_vk: AtomicU32,
    cycle_vk: AtomicU32,
    repaste_vk: AtomicU32,
    held: AtomicBool,
    recording: AtomicBool,
    paused: AtomicBool,
}

static STATE: OnceLock<HookState> = OnceLock::new();

/// `VOXORA_TEST_INJECTED=1` lets scripted (SendInput) keystrokes drive the hotkey so the
/// end-to-end test in `scripts/e2e.ps1` can run without a human. Off unless the variable is set.
fn accept_injected() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VOXORA_TEST_INJECTED").is_some())
}

/// Selectable dictation keys (name shown in Settings → virtual-key code).
pub const KEYS: &[(&str, u32)] = &[
    ("RightAlt", 0xA5),
    ("RightCtrl", 0xA3),
    ("RightShift", 0xA1),
    ("CapsLock", 0x14),
    ("ScrollLock", 0x91),
    ("Pause", 0x13),
    ("Insert", 0x2D),
    ("F13", 0x7C),
    ("F14", 0x7D),
    ("F15", 0x7E),
    ("F16", 0x7F),
    ("F17", 0x80),
    ("F18", 0x81),
    ("F19", 0x82),
    ("F20", 0x83),
];

pub fn vk_for(name: &str) -> u32 {
    KEYS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| *v).unwrap_or(0xA5)
}

fn letter_vk(c: char) -> u32 {
    c.to_ascii_uppercase() as u32 // 'A'..'Z' and '0'..'9' map 1:1 to virtual-key codes
}

/// Install the hook thread. Call once.
pub fn install(sink: impl Fn(HotkeyEvent) + Send + Sync + 'static, dictation_key: &str, cycle: char, repaste: char) {
    let state = HookState {
        sink: Box::new(sink),
        dictation_vk: AtomicU32::new(vk_for(dictation_key)),
        cycle_vk: AtomicU32::new(letter_vk(cycle)),
        repaste_vk: AtomicU32::new(letter_vk(repaste)),
        held: AtomicBool::new(false),
        recording: AtomicBool::new(false),
        paused: AtomicBool::new(false),
    };
    if STATE.set(state).is_err() {
        tracing::warn!("hotkey hook already installed");
        return;
    }
    std::thread::Builder::new()
        .name("voxora-hotkeys".into())
        .spawn(|| unsafe {
            let hmod = GetModuleHandleW(None).ok().map(|h| HINSTANCE(h.0));
            match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), hmod, 0) {
                Ok(_) => tracing::info!("keyboard hook installed"),
                Err(e) => {
                    tracing::error!("SetWindowsHookExW failed: {e}");
                    return;
                }
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("spawn hotkey thread");
}

pub fn configure(dictation_key: &str, cycle: char, repaste: char) {
    if let Some(s) = STATE.get() {
        s.dictation_vk.store(vk_for(dictation_key), Ordering::Relaxed);
        s.cycle_vk.store(letter_vk(cycle), Ordering::Relaxed);
        s.repaste_vk.store(letter_vk(repaste), Ordering::Relaxed);
    }
}

/// While recording, Escape is captured to cancel instead of reaching the app.
pub fn set_recording(on: bool) {
    if let Some(s) = STATE.get() {
        s.recording.store(on, Ordering::Relaxed);
    }
}

pub fn set_paused(on: bool) {
    if let Some(s) = STATE.get() {
        s.paused.store(on, Ordering::Relaxed);
    }
}

pub fn is_paused() -> bool {
    STATE.get().is_some_and(|s| s.paused.load(Ordering::Relaxed))
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && let Some(state) = STATE.get()
    {
        let k = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let injected = k.flags.0 & LLKHF_INJECTED.0 != 0;
        if (!injected || accept_injected()) && handle(state, wparam.0 as u32, k) {
            return LRESULT(1); // swallow
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn pressed(vk: u16) -> bool {
    let state = unsafe { GetAsyncKeyState(vk as i32) };
    state < 0
}

/// Returns true to swallow the key.
fn handle(s: &HookState, msg: u32, k: &KBDLLHOOKSTRUCT) -> bool {
    if s.paused.load(Ordering::Relaxed) {
        return false;
    }
    let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    let at_ms = u64::from(k.time);
    if k.vkCode == s.dictation_vk.load(Ordering::Relaxed) {
        if down && !s.held.swap(true, Ordering::Relaxed) {
            (s.sink)(HotkeyEvent::Down { at_ms });
        } else if up {
            s.held.store(false, Ordering::Relaxed);
            (s.sink)(HotkeyEvent::Up { at_ms });
        }
        return true;
    }
    if k.vkCode == VK_ESCAPE.0 as u32 && s.recording.load(Ordering::Relaxed) {
        if down {
            (s.sink)(HotkeyEvent::Escape);
        }
        return true;
    }
    if down && pressed(VK_CONTROL.0) && pressed(VK_MENU.0) {
        if k.vkCode == s.cycle_vk.load(Ordering::Relaxed) {
            (s.sink)(HotkeyEvent::CycleMode);
            return true;
        }
        if k.vkCode == s.repaste_vk.load(Ordering::Relaxed) {
            (s.sink)(HotkeyEvent::Repaste);
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_map_to_virtual_keys() {
        assert_eq!(vk_for("RightAlt"), 0xA5);
        assert_eq!(vk_for("capslock"), 0x14);
        assert_eq!(vk_for("nonsense"), 0xA5, "unknown names fall back to Right Alt");
        assert_eq!(letter_vk('m'), 0x4D);
    }
}
