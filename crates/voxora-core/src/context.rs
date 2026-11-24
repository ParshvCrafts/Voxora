//! Per-app profiles. Uses only the foreground process name and window title: no screen
//! capture, no reading other apps' text.

use serde::{Deserialize, Serialize};

use crate::mode::Mode;

/// What the OS told us about the window that will receive the text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppContext {
    /// Executable file name, e.g. `Cursor.exe`.
    pub exe: String,
    pub title: String,
    /// Target runs elevated while we don't: Windows UIPI will block synthetic input.
    pub elevated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PasteKeys {
    #[default]
    CtrlV,
    CtrlShiftV,
    ShiftInsert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRule {
    /// Case-insensitive executable name (`code.exe`); `None` matches any app.
    #[serde(default)]
    pub exe: Option<String>,
    /// Case-insensitive substring of the window title (`Overleaf`, `Gmail`).
    #[serde(default)]
    pub title_contains: Option<String>,
    pub mode: Mode,
    #[serde(default)]
    pub paste: Option<PasteKeys>,
    /// Terminals and code: don't append a period.
    #[serde(default)]
    pub terminal_period: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub mode: Mode,
    pub paste: PasteKeys,
    pub terminal_period: bool,
}

fn rule(exe: Option<&str>, title: Option<&str>, mode: Mode) -> AppRule {
    AppRule {
        exe: exe.map(str::to_string),
        title_contains: title.map(str::to_string),
        mode,
        paste: None,
        terminal_period: None,
    }
}

fn terminal(exe: &str, paste: PasteKeys) -> AppRule {
    AppRule { paste: Some(paste), terminal_period: Some(false), ..rule(Some(exe), None, Mode::Technical) }
}

/// Sensible defaults; users edit/extend these in Settings. Title rules come first because a
/// browser hosts many apps.
pub fn default_rules() -> Vec<AppRule> {
    let mut v = vec![
        rule(None, Some("Overleaf"), Mode::Math),
        rule(None, Some(".tex"), Mode::Math),
        rule(None, Some("Gmail"), Mode::Professional),
        rule(None, Some("Outlook"), Mode::Professional),
        rule(None, Some("ChatGPT"), Mode::Technical),
        rule(None, Some("Claude"), Mode::Technical),
        rule(None, Some("Jupyter"), Mode::Technical),
        rule(None, Some("Colab"), Mode::Technical),
        rule(None, Some("GitHub"), Mode::Technical),
    ];
    for exe in [
        "Cursor.exe",
        "Code.exe",
        "Windsurf.exe",
        "Zed.exe",
        "idea64.exe",
        "pycharm64.exe",
        "devenv.exe",
        "rider64.exe",
    ] {
        v.push(rule(Some(exe), None, Mode::Technical));
    }
    for exe in ["WindowsTerminal.exe", "powershell.exe", "pwsh.exe", "cmd.exe", "OpenConsole.exe"] {
        v.push(terminal(exe, PasteKeys::CtrlV));
    }
    v.push(terminal("mintty.exe", PasteKeys::ShiftInsert));
    for exe in ["alacritty.exe", "wezterm-gui.exe"] {
        v.push(terminal(exe, PasteKeys::CtrlShiftV));
    }
    v.push(rule(Some("OUTLOOK.EXE"), None, Mode::Professional));
    v.push(rule(Some("olk.exe"), None, Mode::Professional));
    v.push(rule(Some("WINWORD.EXE"), None, Mode::Professional));
    // macOS reports the app's display name (no window titles without Screen Recording).
    for app in ["Code", "Cursor", "Windsurf", "Zed", "Xcode", "IntelliJ IDEA", "PyCharm"] {
        v.push(rule(Some(app), None, Mode::Technical));
    }
    for app in ["Terminal", "iTerm2", "Warp", "Ghostty", "Alacritty", "WezTerm"] {
        v.push(terminal(app, PasteKeys::CtrlV));
    }
    for app in ["Mail", "Microsoft Outlook", "Microsoft Word", "Pages"] {
        v.push(rule(Some(app), None, Mode::Professional));
    }
    v
}

fn matches(r: &AppRule, ctx: &AppContext) -> bool {
    if r.exe.is_none() && r.title_contains.is_none() {
        return false;
    }
    let exe_ok = r.exe.as_ref().is_none_or(|e| e.eq_ignore_ascii_case(&ctx.exe));
    let title_ok = r.title_contains.as_ref().is_none_or(|t| ctx.title.to_lowercase().contains(&t.to_lowercase()));
    exe_ok && title_ok
}

/// First matching rule wins. `auto` off = always the user's chosen mode.
pub fn resolve(ctx: &AppContext, rules: &[AppRule], chosen: Mode, auto: bool) -> Profile {
    let hit = rules.iter().find(|r| matches(r, ctx));
    let mode = match (auto, hit) {
        // A manual Verbatim choice is a deliberate "don't touch my words": never override it.
        (true, Some(r)) if chosen != Mode::Verbatim => r.mode,
        _ => chosen,
    };
    Profile {
        mode,
        paste: hit.and_then(|r| r.paste).unwrap_or_default(),
        terminal_period: hit.and_then(|r| r.terminal_period).unwrap_or(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(exe: &str, title: &str) -> AppContext {
        AppContext { exe: exe.into(), title: title.into(), elevated: false }
    }

    #[test]
    fn editors_terminals_and_browser_tabs() {
        let r = default_rules();
        assert_eq!(resolve(&ctx("cursor.exe", "main.rs"), &r, Mode::Clean, true).mode, Mode::Technical);
        let wt = resolve(&ctx("WindowsTerminal.exe", "pwsh"), &r, Mode::Clean, true);
        assert_eq!((wt.mode, wt.terminal_period), (Mode::Technical, false));
        assert_eq!(resolve(&ctx("mintty.exe", ""), &r, Mode::Clean, true).paste, PasteKeys::ShiftInsert);
        let tab = ctx("chrome.exe", "Thesis - Overleaf, Online LaTeX Editor - Google Chrome");
        assert_eq!(resolve(&tab, &r, Mode::Clean, true).mode, Mode::Math);
        assert_eq!(resolve(&ctx("chrome.exe", "Inbox - Gmail"), &r, Mode::Clean, true).mode, Mode::Professional);
    }

    #[test]
    fn unknown_apps_and_manual_choices() {
        let r = default_rules();
        let p = resolve(&ctx("notepad.exe", "Untitled"), &r, Mode::Concise, true);
        assert_eq!((p.mode, p.paste, p.terminal_period), (Mode::Concise, PasteKeys::CtrlV, true));
        assert_eq!(resolve(&ctx("Code.exe", ""), &r, Mode::Clean, false).mode, Mode::Clean);
        assert_eq!(resolve(&ctx("Code.exe", ""), &r, Mode::Verbatim, true).mode, Mode::Verbatim);
    }

    #[test]
    fn empty_rule_matches_nothing() {
        let r = vec![rule(None, None, Mode::Math)];
        assert_eq!(resolve(&ctx("x.exe", "y"), &r, Mode::Clean, true).mode, Mode::Clean);
    }
}
