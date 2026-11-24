//! System tray: status at a glance, mode switch, recovery actions.

use std::sync::Arc;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use voxora_core::mode::Mode;

use crate::engine::Engine;
use crate::hotkey;

const MODES: [(Mode, &str); 6] = [
    (Mode::Clean, "Clean"),
    (Mode::Technical, "Technical"),
    (Mode::Math, "Math / LaTeX"),
    (Mode::Concise, "Concise"),
    (Mode::Professional, "Professional"),
    (Mode::Verbatim, "Verbatim"),
];

fn mode_id(m: Mode) -> String {
    format!("mode:{}", serde_json::to_value(m).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let engine = app.state::<Arc<Engine>>();
    let s = engine.settings();
    let st = engine.status();
    let key = s.hotkey.dictation_key.replace("Right", "Right ");
    let headline = if !st.missing.is_empty() {
        "Setup needed. Open Voxora.".to_string()
    } else if st.paused {
        "Paused".to_string()
    } else {
        format!("Hold {key} to dictate · speech {} · rewrite {}", short(&st.asr), short(&st.llm))
    };
    let status = MenuItem::with_id(app, "status", headline, false, None::<&str>)?;
    let modes = Submenu::with_id(app, "modes", "Mode", true)?;
    for (m, label) in MODES {
        modes.append(&CheckMenuItem::with_id(app, mode_id(m), label, true, s.mode == m, None::<&str>)?)?;
    }
    let sep = || PredefinedMenuItem::separator(app);
    Menu::with_items(
        app,
        &[
            &status,
            &sep()?,
            &MenuItem::with_id(app, "open", "Open Voxora", true, None::<&str>)?,
            &modes,
            &MenuItem::with_id(app, "repaste", "Paste last dictation", true, None::<&str>)?,
            &MenuItem::with_id(app, "retry", "Retry failed recording", engine.has_failed_audio(), None::<&str>)?,
            &sep()?,
            &CheckMenuItem::with_id(app, "pause", "Pause dictation", true, st.paused, None::<&str>)?,
            &MenuItem::with_id(app, "restart", "Restart engines", true, None::<&str>)?,
            &MenuItem::with_id(app, "unload", "Unload GPU models", true, None::<&str>)?,
            &sep()?,
            &MenuItem::with_id(app, "quit", "Quit Voxora", true, None::<&str>)?,
        ],
    )
}

fn short(s: &str) -> &str {
    if s.starts_with("error") { "error" } else { s }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    // macOS menu bar: a monochrome template glyph that follows light/dark menu bars.
    #[cfg(target_os = "macos")]
    let builder = TrayIconBuilder::with_id("main")
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?)
        .icon_as_template(true);
    #[cfg(not(target_os = "macos"))]
    let builder = TrayIconBuilder::with_id("main").icon(app.default_window_icon().cloned().expect("bundle icon"));
    builder
        .tooltip("Voxora")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Rebuild the menu so checkmarks and status reflect current state.
pub fn refresh(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    let engine = app.state::<Arc<Engine>>().inner().clone();
    match id {
        "open" => crate::show_main(app),
        "quit" => {
            engine.stop_engines();
            app.exit(0);
        }
        "pause" => {
            hotkey::set_paused(!hotkey::is_paused());
            refresh(app);
        }
        "repaste" => {
            tauri::async_runtime::spawn(async move { engine.repaste_last().await });
        }
        "retry" => {
            tauri::async_runtime::spawn(async move { engine.retry_failed().await });
        }
        "restart" => {
            engine.stop_engines();
            engine.start_engines();
        }
        "unload" => engine.stop_engines(),
        m if m.starts_with("mode:") => {
            let mode = serde_json::from_value::<Mode>(serde_json::Value::String(m[5..].to_string()));
            if let Ok(mode) = mode {
                let mut s = engine.settings();
                s.mode = mode;
                if let Err(e) = engine.update_settings(s) {
                    tracing::error!("mode change failed: {e}");
                }
                let _ = tauri::Emitter::emit(app, "settings", ());
            }
        }
        _ => {}
    }
}
