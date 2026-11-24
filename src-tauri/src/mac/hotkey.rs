//! Global hotkeys on macOS via a listen-only `CGEventTap` on a dedicated run-loop thread.
//!
//! Same API as the Windows hook (`src/hotkey.rs`). Differences, by design:
//! - Listen-only: the dictation key and Esc still reach the focused app. A lone Option, Control
//!   or Command press types nothing, and an active tap that loses Accessibility can freeze input.
//! - Needs Accessibility permission (which also covers listening). Until it is granted the
//!   thread polls and creates the tap as soon as it can.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::runloop::CFRunLoop;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult, EventField,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Down { at_ms: u64 },
    Up { at_ms: u64 },
    Escape,
    CycleMode,
    Repaste,
}

struct TapState {
    sink: Box<dyn Fn(HotkeyEvent) + Send + Sync>,
    dictation: AtomicU32,
    cycle: AtomicU32,
    repaste: AtomicU32,
    held: AtomicBool,
    recording: AtomicBool,
    paused: AtomicBool,
}

static STATE: OnceLock<TapState> = OnceLock::new();
static TAP_RUNNING: AtomicBool = AtomicBool::new(false);

/// Setting names (shared with Windows) → macOS virtual key codes (HIToolbox `kVK_*`).
pub const KEYS: &[(&str, u32)] = &[
    ("RightAlt", 61), // Right Option
    ("RightCtrl", 62),
    ("RightShift", 60),
    ("RightCmd", 54),
    ("F13", 105),
    ("F14", 107),
    ("F15", 113),
    ("F16", 106),
    ("F17", 64),
    ("F18", 79),
    ("F19", 80),
];
const ESC: u32 = 53;
/// Modifier keys arrive as FlagsChanged; everything else as KeyDown/KeyUp.
const MODIFIER_CODES: &[u32] = &[54, 55, 56, 58, 59, 60, 61, 62, 63];

pub fn vk_for(name: &str) -> u32 {
    KEYS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| *v).unwrap_or(61)
}

/// ANSI layout positions for the Control+Option shortcut letters.
fn letter_code(c: char) -> u32 {
    const MAP: &[(char, u32)] = &[
        ('a', 0),
        ('s', 1),
        ('d', 2),
        ('f', 3),
        ('h', 4),
        ('g', 5),
        ('z', 6),
        ('x', 7),
        ('c', 8),
        ('v', 9),
        ('b', 11),
        ('q', 12),
        ('w', 13),
        ('e', 14),
        ('r', 15),
        ('y', 16),
        ('t', 17),
        ('1', 18),
        ('2', 19),
        ('3', 20),
        ('4', 21),
        ('6', 22),
        ('5', 23),
        ('9', 25),
        ('7', 26),
        ('8', 28),
        ('0', 29),
        ('o', 31),
        ('u', 32),
        ('i', 34),
        ('p', 35),
        ('l', 37),
        ('j', 38),
        ('k', 40),
        ('n', 45),
        ('m', 46),
    ];
    let c = c.to_ascii_lowercase();
    MAP.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(u32::MAX)
}

// ── Accessibility (ApplicationServices / HIServices) ────────────────────────────
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> bool;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

pub fn accessibility_granted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Shows the system prompt that adds Voxora to Privacy & Security > Accessibility, and opens
/// that pane so the user can flip the switch.
pub fn request_permission() {
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let opts = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
        AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef());
    }
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// What is missing for the hotkey and pasting to work, in words for the UI.
pub fn permission_problem() -> Option<&'static str> {
    (!accessibility_granted() || !TAP_RUNNING.load(Ordering::Relaxed)).then_some(
        "Voxora needs Accessibility access to hear the dictation key and paste. Turn it on in System Settings > \
         Privacy & Security > Accessibility.",
    )
}

pub fn install(sink: impl Fn(HotkeyEvent) + Send + Sync + 'static, dictation_key: &str, cycle: char, repaste: char) {
    let state = TapState {
        sink: Box::new(sink),
        dictation: AtomicU32::new(vk_for(dictation_key)),
        cycle: AtomicU32::new(letter_code(cycle)),
        repaste: AtomicU32::new(letter_code(repaste)),
        held: AtomicBool::new(false),
        recording: AtomicBool::new(false),
        paused: AtomicBool::new(false),
    };
    if STATE.set(state).is_err() {
        tracing::warn!("hotkey tap already installed");
        return;
    }
    std::thread::Builder::new().name("voxora-hotkeys".into()).spawn(run).expect("spawn hotkey thread");
}

fn run() {
    let mut asked = false;
    loop {
        if !accessibility_granted() {
            if !asked {
                tracing::warn!("Accessibility not granted; waiting before creating the event tap");
                asked = true;
            }
            std::thread::sleep(Duration::from_millis(1500));
            continue;
        }
        let result = CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::KeyDown, CGEventType::KeyUp, CGEventType::FlagsChanged],
            |_proxy, ty, event| {
                match ty {
                    // macOS disables slow or interrupted taps: leave the run loop and rebuild.
                    CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                        CFRunLoop::get_current().stop();
                    }
                    _ => {
                        if let Some(s) = STATE.get() {
                            handle(s, ty, event);
                        }
                    }
                }
                CallbackResult::Keep
            },
            || {
                TAP_RUNNING.store(true, Ordering::Relaxed);
                tracing::info!("keyboard event tap installed");
                CFRunLoop::run_current();
                TAP_RUNNING.store(false, Ordering::Relaxed);
            },
        );
        if result.is_err() {
            tracing::error!("CGEventTapCreate failed (permission missing or revoked)");
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn handle(s: &TapState, ty: CGEventType, e: &CGEvent) {
    if s.paused.load(Ordering::Relaxed) {
        return;
    }
    let code = e.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u32;
    let key = s.dictation.load(Ordering::Relaxed);
    let at_ms = now_ms();
    if code == key {
        let down = if MODIFIER_CODES.contains(&code) {
            // FlagsChanged carries no up/down bit: track our own state for this key code.
            matches!(ty, CGEventType::FlagsChanged) && !s.held.load(Ordering::Relaxed)
        } else {
            matches!(ty, CGEventType::KeyDown)
        };
        let up = if MODIFIER_CODES.contains(&code) {
            matches!(ty, CGEventType::FlagsChanged) && s.held.load(Ordering::Relaxed)
        } else {
            matches!(ty, CGEventType::KeyUp)
        };
        if down && !s.held.swap(true, Ordering::Relaxed) {
            (s.sink)(HotkeyEvent::Down { at_ms });
        } else if up {
            s.held.store(false, Ordering::Relaxed);
            (s.sink)(HotkeyEvent::Up { at_ms });
        }
        return;
    }
    if !matches!(ty, CGEventType::KeyDown) {
        return;
    }
    if code == ESC && s.recording.load(Ordering::Relaxed) {
        (s.sink)(HotkeyEvent::Escape);
        return;
    }
    let flags = e.get_flags();
    if flags.contains(CGEventFlags::CGEventFlagControl) && flags.contains(CGEventFlags::CGEventFlagAlternate) {
        if code == s.cycle.load(Ordering::Relaxed) {
            (s.sink)(HotkeyEvent::CycleMode);
        } else if code == s.repaste.load(Ordering::Relaxed) {
            (s.sink)(HotkeyEvent::Repaste);
        }
    }
}

pub fn configure(dictation_key: &str, cycle: char, repaste: char) {
    if let Some(s) = STATE.get() {
        s.dictation.store(vk_for(dictation_key), Ordering::Relaxed);
        s.cycle.store(letter_code(cycle), Ordering::Relaxed);
        s.repaste.store(letter_code(repaste), Ordering::Relaxed);
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_key_codes() {
        assert_eq!(vk_for("RightAlt"), 61, "Right Option");
        assert_eq!(vk_for("CapsLock"), 61, "keys macOS lacks fall back to Right Option");
        assert_eq!(letter_code('M'), 46);
        assert_eq!(letter_code('v'), 9);
    }
}
