//! IPC surface for the settings/history window. Every command validates input and returns a
//! user-readable error string.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use voxora_core::engines::download::CATALOG;
use voxora_core::mode::Mode;
use voxora_core::settings::Settings;
use voxora_core::store::{Dictation, LatencyStats};

use crate::engine::{Engine, EngineStatus};
use crate::{audio, hotkey, inject};

type E<'a> = State<'a, Arc<Engine>>;

#[tauri::command]
pub fn get_settings(engine: E<'_>) -> Settings {
    engine.settings()
}

#[tauri::command]
pub fn save_settings(engine: E<'_>, settings: Settings) -> Result<(), String> {
    engine.update_settings(settings)
}

#[tauri::command]
pub fn get_status(engine: E<'_>) -> EngineStatus {
    engine.status()
}

#[tauri::command]
pub fn list_devices() -> Vec<String> {
    audio::input_devices()
}

#[tauri::command]
pub fn list_keys() -> Vec<&'static str> {
    hotkey::KEYS.iter().map(|(n, _)| *n).collect()
}

#[tauri::command]
pub fn history(engine: E<'_>, query: String, limit: usize, offset: usize) -> Result<Vec<Dictation>, String> {
    engine.store(|s| s.search(query.trim(), limit.min(500), offset)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_delete(engine: E<'_>, id: i64) -> Result<(), String> {
    engine.store(|s| s.delete(id)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_clear(engine: E<'_>) -> Result<(), String> {
    engine.store(|s| s.clear()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stats(engine: E<'_>) -> Result<LatencyStats, String> {
    engine.store(|s| s.stats(500)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    inject::set_text(&text, false).map(|_| ())
}

#[tauri::command]
pub async fn reprocess(engine: E<'_>, id: i64, mode: Mode) -> Result<String, String> {
    engine.reprocess(id, mode).await
}

#[derive(Serialize)]
pub struct AssetInfo {
    id: &'static str,
    label: &'static str,
    bytes: u64,
}

#[tauri::command]
pub fn catalog() -> Vec<AssetInfo> {
    CATALOG.iter().map(|a| AssetInfo { id: a.id, label: a.label, bytes: a.bytes }).collect()
}

#[tauri::command]
pub async fn install(engine: E<'_>) -> Result<(), String> {
    let e = engine.inner().clone();
    e.install().await
}

#[tauri::command]
pub fn restart_engines(engine: E<'_>) {
    engine.stop_engines();
    engine.start_engines();
}

#[tauri::command]
pub fn set_paused(app: tauri::AppHandle, paused: bool) {
    hotkey::set_paused(paused);
    crate::tray::refresh(&app);
}

#[tauri::command]
pub fn request_permission() {
    hotkey::request_permission();
}

#[tauri::command]
pub fn open_folder(engine: E<'_>, which: String) -> Result<(), String> {
    let p = match which.as_str() {
        "logs" => &engine.paths.logs,
        "data" => &engine.paths.root,
        _ => return Err("unknown folder".into()),
    };
    let _ = std::fs::create_dir_all(p);
    let opener = if cfg!(windows) { "explorer.exe" } else { "open" };
    std::process::Command::new(opener).arg(p).spawn().map(|_| ()).map_err(|e| e.to_string())
}
