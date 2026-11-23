//! Voxora desktop app: tray, overlay, settings window, and the Win32 integration around the
//! platform-neutral `voxora-core` pipeline.

mod audio;
mod commands;
mod engine;
mod hotkey;
mod inject;
mod target;
mod tray;
mod winutil;

use std::sync::Arc;

use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use crate::engine::{Engine, Paths};

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn autostart_sync(app: &AppHandle, enabled: bool) {
    let al = app.autolaunch();
    let current = al.is_enabled().unwrap_or(false);
    let res = match (enabled, current) {
        (true, false) => al.enable(),
        (false, true) => al.disable(),
        _ => Ok(()),
    };
    if let Err(e) = res {
        tracing::warn!("autostart update failed: {e}");
    }
}

fn init_logging(paths: &Paths) -> tracing_appender::non_blocking::WorkerGuard {
    let _ = std::fs::create_dir_all(&paths.logs);
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .max_log_files(7)
        .filename_prefix("voxora")
        .filename_suffix("log")
        .build(&paths.logs)
        .expect("log directory is writable");
    let (writer, guard) = tracing_appender::non_blocking(file);
    let filter = tracing_subscriber::EnvFilter::try_from_env("VOXORA_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(writer).with_ansi(false).init();
    guard
}

pub fn run() {
    let _log_guard = init_logging(&Paths::new());
    tracing::info!("Voxora {} starting", env!("CARGO_PKG_VERSION"));
    let minimized = std::env::args().any(|a| a == "--minimized");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .setup(move |app| {
            let handle = app.handle().clone();
            let (engine, rx) = Engine::new(handle.clone());
            app.manage(engine.clone());
            // Windows are declared with `create: false` and built only now: a fast WebView would
            // otherwise call commands before the engine state above is managed (seen in release).
            for cfg in app.config().app.windows.clone() {
                tauri::WebviewWindowBuilder::from_config(&handle, &cfg)?.build()?;
            }

            if let Some(ov) = app.get_webview_window("overlay")
                && let Ok(h) = ov.hwnd()
            {
                winutil::make_overlay(target::hwnd(h.0 as isize));
            }
            let s = engine.settings();
            hotkey::install(
                engine.hotkey_sink(),
                &s.hotkey.dictation_key,
                s.hotkey.cycle_mode_key,
                s.hotkey.repaste_key,
            );
            engine.spawn_loop(rx);
            tray::create(&handle)?;
            autostart_sync(&handle, s.autostart);
            engine.start_engines();
            if !minimized || !s.onboarded || !engine.missing_assets().is_empty() {
                show_main(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings window keeps Voxora running in the tray.
            if window.label() == "main"
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_status,
            commands::list_devices,
            commands::list_keys,
            commands::history,
            commands::history_delete,
            commands::history_clear,
            commands::stats,
            commands::copy_text,
            commands::reprocess,
            commands::catalog,
            commands::install,
            commands::restart_engines,
            commands::set_paused,
            commands::open_folder,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Voxora")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<Arc<Engine>>().stop_engines();
            }
        });
}
