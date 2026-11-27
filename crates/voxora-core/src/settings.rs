//! User settings: one JSON file, every field defaulted so old files keep loading after upgrades.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::context::{AppRule, default_rules};
use crate::mode::Mode;
use crate::normalize::Dictionary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub hotkey: HotkeySettings,
    /// Mode used when no app rule matches (or auto-context is off).
    pub mode: Mode,
    pub auto_context: bool,
    pub app_rules: Vec<AppRule>,
    pub dictionary: Dictionary,
    pub spoken_commands: bool,
    pub asr: AsrSettings,
    pub llm: LlmSettings,
    pub audio: AudioSettings,
    pub paste: PasteSettings,
    pub history: HistorySettings,
    pub autostart: bool,
    pub theme: Theme,
    /// Set once the first-run wizard finished.
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: HotkeySettings::default(),
            mode: Mode::Clean,
            auto_context: true,
            app_rules: default_rules(),
            dictionary: Dictionary {
                terms: [
                    "PyTorch",
                    "NumPy",
                    "pandas",
                    "FastAPI",
                    "LangGraph",
                    "TypeScript",
                    "GitHub",
                    "pytest",
                    "YAML",
                    "JSON",
                    "API",
                    "SQL",
                    "CUDA",
                    "FAISS",
                    "BM25",
                    "WebSocket",
                    "Redis",
                    "PostgreSQL",
                    "Kubernetes",
                    "JavaScript",
                ]
                .map(String::from)
                .to_vec(),
                replacements: vec![],
            },
            spoken_commands: true,
            asr: AsrSettings::default(),
            llm: LlmSettings::default(),
            audio: AudioSettings::default(),
            paste: PasteSettings::default(),
            history: HistorySettings::default(),
            autostart: true,
            theme: Theme::System,
            onboarded: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HotkeySettings {
    /// Virtual-key name of the dictation key: `RightAlt`, `RightCtrl`, `CapsLock`, `F13`...
    pub dictation_key: String,
    /// Press shorter than this = hands-free toggle; longer = push-to-talk.
    pub hold_threshold_ms: u64,
    /// Ctrl+Alt+<key> cycles the mode.
    pub cycle_mode_key: char,
    /// Ctrl+Alt+<key> re-pastes the last dictation.
    pub repaste_key: char,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self { dictation_key: "RightAlt".into(), hold_threshold_ms: 250, cycle_mode_key: 'M', repaste_key: 'V' }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AsrSettings {
    /// Model file name inside the models directory.
    pub model: String,
    /// ISO code (`en`) or `auto`.
    pub language: String,
    pub port: u16,
    pub use_gpu: bool,
    /// Show the words in the HUD while recording (D19). Display only; the pasted text always
    /// comes from the full recording.
    pub live_preview: bool,
}

impl Default for AsrSettings {
    fn default() -> Self {
        Self {
            model: "ggml-large-v3-turbo-q8_0.bin".into(),
            language: "en".into(),
            port: 18181,
            use_gpu: true,
            live_preview: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LlmSettings {
    pub enabled: bool,
    /// Voxora starts and supervises llama-server itself. Off = use `endpoint` as-is (e.g. Ollama).
    pub managed: bool,
    /// OpenAI-compatible base URL.
    pub endpoint: String,
    /// GGUF file (managed) or model name sent to the endpoint (external).
    pub model: String,
    pub port: u16,
    pub timeout_ms: u64,
    /// Utterances with at most this many words and no fillers skip the LLM.
    pub fast_path_words: usize,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            managed: true,
            endpoint: "http://127.0.0.1:18080/v1".into(),
            model: "Qwen3.5-4B-Q4_K_M.gguf".into(),
            port: 18080,
            timeout_ms: 2500,
            fast_path_words: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioSettings {
    /// Input device name; `None` = Windows default.
    pub device: Option<String>,
    /// Hands-free mode stops after this much trailing silence.
    pub hands_free_silence_ms: u32,
    pub max_recording_secs: u32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { device: None, hands_free_silence_ms: 1200, max_recording_secs: 600 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PasteSettings {
    pub restore_clipboard: bool,
    /// Electron/Chromium apps read the clipboard asynchronously after Ctrl+V.
    pub restore_delay_ms: u64,
}

impl Default for PasteSettings {
    fn default() -> Self {
        Self { restore_clipboard: true, restore_delay_ms: 220 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HistorySettings {
    pub enabled: bool,
    pub retention_days: u32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self { enabled: true, retention_days: 30 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug)]
pub enum LoadOutcome {
    Loaded(Settings),
    /// No file yet: first run.
    Fresh(Settings),
    /// File was unreadable; it was moved aside to `backup` and defaults are in use.
    Recovered {
        settings: Settings,
        backup: std::path::PathBuf,
        error: String,
    },
}

impl LoadOutcome {
    pub fn settings(self) -> Settings {
        match self {
            LoadOutcome::Loaded(s) | LoadOutcome::Fresh(s) => s,
            LoadOutcome::Recovered { settings, .. } => settings,
        }
    }
}

pub fn load(path: &Path) -> LoadOutcome {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LoadOutcome::Fresh(Settings::default()),
        Err(e) => return recover(path, e.to_string()),
    };
    // Editors like older Notepad / PowerShell 5 write a UTF-8 BOM, which serde_json rejects.
    match serde_json::from_str(text.trim_start_matches('\u{feff}')) {
        Ok(s) => LoadOutcome::Loaded(s),
        Err(e) => recover(path, e.to_string()),
    }
}

fn recover(path: &Path, error: String) -> LoadOutcome {
    let backup = path.with_extension("invalid.json");
    let _ = std::fs::rename(path, &backup);
    LoadOutcome::Recovered { settings: Settings::default(), backup, error }
}

/// Write to a temp file then rename, so a crash mid-save never leaves a truncated file.
pub fn save(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_partial_files_fill_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        assert!(matches!(load(&p), LoadOutcome::Fresh(_)));

        let s = Settings { mode: Mode::Math, ..Settings::default() };
        save(&p, &s).unwrap();
        assert_eq!(load(&p).settings(), s);

        std::fs::write(&p, r#"{"mode":"concise","llm":{"timeout_ms":900}}"#).unwrap();
        let s = load(&p).settings();
        assert_eq!((s.mode, s.llm.timeout_ms, s.llm.port), (Mode::Concise, 900, 18080));
    }

    #[test]
    fn settings_from_before_live_preview_turn_it_on() {
        // A v0.2.3 file has an `asr` block without `live_preview`; it must not come back false.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, r#"{"asr":{"model":"m.bin","language":"en","port":18181,"use_gpu":false}}"#).unwrap();
        assert!(matches!(load(&p), LoadOutcome::Loaded(s) if s.asr.live_preview && !s.asr.use_gpu));
    }

    #[test]
    fn utf8_bom_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, "\u{feff}{\"mode\":\"math\"}").unwrap();
        assert!(matches!(load(&p), LoadOutcome::Loaded(s) if s.mode == Mode::Math));
    }

    #[test]
    fn corrupt_file_is_moved_aside_not_lost() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        std::fs::write(&p, "{ not json").unwrap();
        match load(&p) {
            LoadOutcome::Recovered { backup, .. } => {
                assert_eq!(std::fs::read_to_string(backup).unwrap(), "{ not json");
                assert!(!p.exists());
            }
            other => panic!("expected recovery, got {other:?}"),
        }
    }
}
