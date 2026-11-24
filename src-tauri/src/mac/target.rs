//! The app that will receive the text, on macOS.
//!
//! Only the frontmost app's name is read (NSWorkspace, no permission needed). Window titles of
//! other apps would need Screen Recording permission, which Voxora never asks for, so title
//! rules simply do not match on macOS. `elevated` maps to Secure Keyboard Entry: while a password
//! field or a terminal with that setting is focused, macOS drops synthetic keystrokes, exactly
//! like UIPI on Windows, so the text is left on the clipboard instead.

use objc2_app_kit::NSWorkspace;
use voxora_core::context::AppContext;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}

#[derive(Debug, Clone)]
pub struct Target {
    pub ctx: AppContext,
}

pub fn foreground() -> Target {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication();
    let exe = app.and_then(|a| a.localizedName()).map(|n| n.to_string()).unwrap_or_default();
    let secure = unsafe { IsSecureEventInputEnabled() };
    Target { ctx: AppContext { exe, title: String::new(), elevated: secure } }
}

pub fn has_foreground() -> bool {
    NSWorkspace::sharedWorkspace().frontmostApplication().is_some()
}
