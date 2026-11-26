//! The orchestrator. Owns settings, history, the audio thread and both inference servers, and
//! runs the dictation lifecycle:
//!
//! ```text
//! IDLE ─key─► RECORDING ─release/tap/silence─► TRANSCRIBING ─► [persist raw] ─► REWRITING
//!   ▲            │ Esc                                                │
//!   │            ▼                                                    ▼
//!   └───────── cancelled            [persist final] ◄─ guard/fallback ┘ ─► INJECTING ─► IDLE
//!                                                        UIPI / paste failure ─► clipboard + notice
//! ```
//! Ordering guarantee: the raw transcript is committed to SQLite before any rewrite or paste.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use voxora_core::asr_filter;
use voxora_core::context::{AppContext, Profile, resolve};
use voxora_core::engines::download::{self, Asset, Kind};
use voxora_core::engines::llm::OpenAiCompat;
use voxora_core::engines::sidecar::{Sidecar, Spec};
use voxora_core::engines::whisper::{WhisperClient, vocabulary_prompt};
use voxora_core::gesture::{Action, Gesture, Input};
use voxora_core::mode::Mode;
use voxora_core::normalize::DictMatcher;
use voxora_core::pipeline::{self, Path};
use voxora_core::settings::{self, Settings};
use voxora_core::store::{Status as RowStatus, Store};

use crate::audio::{self, Audio, Recording};
use crate::hotkey::{self, HotkeyEvent};
use crate::inject::{self, PasteOptions};
use crate::target::{self, Target};
use crate::winutil;

pub struct Paths {
    pub root: PathBuf,
    pub models: PathBuf,
    pub runtime: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub db: PathBuf,
    pub settings: PathBuf,
}

impl Paths {
    pub fn new() -> Self {
        // %LOCALAPPDATA%\Voxora on Windows, ~/Library/Application Support/Voxora on macOS.
        #[cfg(windows)]
        let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        #[cfg(target_os = "macos")]
        let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
        let root = base.unwrap_or_else(std::env::temp_dir).join("Voxora");
        Self {
            models: root.join("models"),
            runtime: root.join("runtime"),
            cache: root.join("cache"),
            logs: root.join("logs"),
            db: root.join("voxora.db"),
            settings: root.join("settings.json"),
            root,
        }
    }
    fn whisper_exe(&self) -> PathBuf {
        // macOS: whisper.cpp publishes no server binary, so CI builds one and bundles it inside
        // Voxora.app next to the main executable (tauri externalBin).
        #[cfg(target_os = "macos")]
        if let Some(p) = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.join("whisper-server")))
            && p.exists()
        {
            return p;
        }
        self.runtime.join("whisper").join(runtime_exe("whisper"))
    }
    fn llama_exe(&self) -> PathBuf {
        self.runtime.join("llama").join(runtime_exe("llama"))
    }
}

/// What the floating pill shows.
#[derive(Debug, Clone, Serialize)]
pub struct Overlay {
    pub kind: &'static str, // listening | processing | done | notice | warning | error | hidden
    pub message: Option<String>,
    pub hands_free: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupPlan {
    pub speech_bytes: u64,
    pub rewrite_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EngineStatus {
    pub asr: String,
    pub llm: String,
    pub missing: Vec<&'static str>,
    pub paused: bool,
    pub gpu: Option<String>,
    /// macOS: what permission is missing, in words for the UI. Always None on Windows.
    pub permission: Option<&'static str>,
    /// Setup is downloading or unpacking; the UI must not start a second one.
    pub installing: bool,
}

pub enum Msg {
    Hotkey(HotkeyEvent),
    Audio(audio::Event),
    MaxDuration(u64),
}

struct Session {
    id: u64,
    target: Target,
    profile: Profile,
}

pub struct Engine {
    app: AppHandle,
    pub paths: Paths,
    settings: RwLock<Settings>,
    dict: RwLock<Arc<DictMatcher>>,
    store: Mutex<Store>,
    audio: Audio,
    gesture: Mutex<Gesture>,
    session: Mutex<Option<Session>>,
    session_seq: AtomicU64,
    asr: RwLock<Arc<Sidecar>>,
    llm_server: RwLock<Option<Arc<Sidecar>>>,
    whisper: RwLock<WhisperClient>,
    llm: RwLock<OpenAiCompat>,
    status: Mutex<EngineStatus>,
    inject_lock: tokio::sync::Mutex<()>,
    /// Dictations are transcribed and pasted in the order they were spoken (tokio's Mutex is FIFO).
    process_lock: tokio::sync::Mutex<()>,
    restarting_llm: AtomicBool,
    /// Last recording whose transcription failed, kept in RAM for "Retry".
    failed_audio: Mutex<Option<(Recording, Target, Profile)>>,
    tx: UnboundedSender<Msg>,
}

impl Engine {
    pub fn new(app: AppHandle) -> (Arc<Self>, UnboundedReceiver<Msg>) {
        let paths = Paths::new();
        let _ = std::fs::create_dir_all(&paths.root);
        let settings = match settings::load(&paths.settings) {
            settings::LoadOutcome::Recovered { settings, backup, error } => {
                tracing::error!("settings unreadable ({error}); moved to {}", backup.display());
                settings
            }
            other => other.settings(),
        };
        let store = Store::open(&paths.db).unwrap_or_else(|e| {
            tracing::error!("history database unavailable ({e}); using in-memory store");
            Store::open_in_memory().expect("in-memory sqlite")
        });
        if settings.history.enabled
            && let Ok(n) = store.prune(settings.history.retention_days)
            && n > 0
        {
            tracing::info!("pruned {n} old dictations");
        }
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Msg>();
        let audio_tx = tx.clone();
        let audio = Audio::spawn(move |e| {
            let _ = audio_tx.send(Msg::Audio(e));
        });
        let engine = Arc::new(Self {
            dict: RwLock::new(Arc::new(DictMatcher::new(&settings.dictionary))),
            gesture: Mutex::new(Gesture::new(settings.hotkey.hold_threshold_ms)),
            asr: RwLock::new(Arc::new(asr_sidecar(&paths, &settings, true))),
            llm_server: RwLock::new(None),
            whisper: RwLock::new(WhisperClient::new(settings.asr.port)),
            llm: RwLock::new(OpenAiCompat::new(&llm_endpoint(&settings), &settings.llm.model)),
            status: Mutex::new(EngineStatus { asr: "stopped".into(), llm: "stopped".into(), ..Default::default() }),
            store: Mutex::new(store),
            settings: RwLock::new(settings),
            session: Mutex::new(None),
            session_seq: AtomicU64::new(0),
            inject_lock: tokio::sync::Mutex::new(()),
            process_lock: tokio::sync::Mutex::new(()),
            restarting_llm: AtomicBool::new(false),
            failed_audio: Mutex::new(None),
            audio,
            paths,
            app,
            tx,
        });
        (engine, rx)
    }

    /// Where the keyboard hook delivers its events.
    pub fn hotkey_sink(&self) -> impl Fn(HotkeyEvent) + Send + Sync + 'static {
        let tx = self.tx.clone();
        move |e| {
            let _ = tx.send(Msg::Hotkey(e));
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn store<T>(&self, f: impl FnOnce(&Store) -> T) -> T {
        f(&self.store.lock().unwrap())
    }

    pub fn status(&self) -> EngineStatus {
        let mut s = self.status.lock().unwrap().clone();
        s.paused = hotkey::is_paused();
        s.missing = self.missing_assets();
        s.permission = hotkey::permission_problem();
        s
    }

    fn set_status(&self, f: impl FnOnce(&mut EngineStatus)) {
        f(&mut self.status.lock().unwrap());
        // The full status: `missing`, `paused` and `permission` are computed, not stored.
        let _ = self.app.emit("status", self.status());
        crate::tray::refresh(&self.app);
    }

    /// The pill shows no text except a message, so the window is only as wide as what it shows
    /// (D16): compact for the dot and bars, wide when a message must fit.
    fn overlay(&self, kind: &'static str, message: Option<String>, hands_free: bool) {
        let Some(w) = self.app.get_webview_window("overlay") else { return };
        let width = if message.is_some() { 340.0 } else { 184.0 };
        let _ = self.app.emit_to("overlay", "overlay", Overlay { kind, message, hands_free });
        if kind == "hidden" {
            winutil::hide_overlay(&w);
        } else {
            winutil::show_overlay(&w, width, 64.0, 56);
        }
    }

    /// Hide the pill after `ms`, unless something newer took it over.
    fn overlay_hide_after(self: &Arc<Self>, ms: u64) {
        let me = self.clone();
        let seq = self.session_seq.load(Ordering::SeqCst);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            if me.session_seq.load(Ordering::SeqCst) == seq && me.session.lock().unwrap().is_none() {
                me.overlay("hidden", None, false);
            }
        });
    }

    // ───────────────────────── event loop ─────────────────────────

    pub fn spawn_loop(self: &Arc<Self>, mut rx: UnboundedReceiver<Msg>) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(msg) = rx.recv().await {
                me.handle(msg).await;
            }
        });
    }

    async fn handle(self: &Arc<Self>, msg: Msg) {
        let input = match msg {
            Msg::Hotkey(HotkeyEvent::Down { at_ms }) => Input::Down { at_ms },
            Msg::Hotkey(HotkeyEvent::Up { at_ms }) => Input::Up { at_ms },
            Msg::Hotkey(HotkeyEvent::Escape) => Input::Cancel,
            Msg::Hotkey(HotkeyEvent::CycleMode) => return self.cycle_mode(),
            Msg::Hotkey(HotkeyEvent::Repaste) => return self.repaste_last().await,
            Msg::Audio(audio::Event::Silence) => Input::Silence,
            Msg::Audio(audio::Event::Level(l)) => {
                let _ = self.app.emit_to("overlay", "level", l);
                return;
            }
            Msg::Audio(audio::Event::DeadMic) => {
                self.overlay("warning", Some("No sound from the mic. Is it muted?".into()), false);
                return;
            }
            Msg::Audio(audio::Event::StreamError(e)) => {
                // Keep what was said before the mic dropped: stop and transcribe, don't cancel.
                tracing::error!("microphone stream error: {e}");
                Input::MaxDuration
            }
            Msg::MaxDuration(id) => {
                if self.session.lock().unwrap().as_ref().is_none_or(|s| s.id != id) {
                    return;
                }
                Input::MaxDuration
            }
        };
        let action = self.gesture.lock().unwrap().feed(input);
        tracing::debug!(?input, ?action, "gesture");
        match action {
            Action::None => {}
            Action::Start => self.start_recording().await,
            Action::LockHandsFree => self.overlay("listening", None, true),
            Action::Stop => self.stop_recording(),
            Action::Cancel => self.cancel_recording(),
        }
    }

    async fn start_recording(self: &Arc<Self>) {
        let s = self.settings();
        let target = target::foreground();
        let profile = resolve(&target.ctx, &s.app_rules, s.mode, s.auto_context);
        let id = self.session_seq.fetch_add(1, Ordering::SeqCst) + 1;
        *self.session.lock().unwrap() = Some(Session { id, target, profile });
        hotkey::set_recording(true);
        tracing::info!(app = %self.session.lock().unwrap().as_ref().map(|s| s.target.ctx.exe.clone()).unwrap_or_default(), mode = ?profile.mode, "recording started");
        self.overlay("listening", None, false);

        let audio = self.audio.clone();
        let (device, silence) = (s.audio.device.clone(), s.audio.hands_free_silence_ms);
        let started = tauri::async_runtime::spawn_blocking(move || audio.start(device, silence)).await;
        if let Err(e) = started.map_err(|e| e.to_string()).and_then(|r| r) {
            tracing::error!("cannot start recording: {e}");
            self.reset_session();
            self.overlay("error", Some(e), false);
            self.overlay_hide_after(2500);
            return;
        }
        let tx = self.tx.clone();
        let max = Duration::from_secs(u64::from(s.audio.max_recording_secs));
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(max).await;
            let _ = tx.send(Msg::MaxDuration(id));
        });
    }

    fn reset_session(&self) {
        hotkey::set_recording(false);
        *self.gesture.lock().unwrap() = Gesture::new(self.settings().hotkey.hold_threshold_ms);
        self.session.lock().unwrap().take();
        self.audio.cancel();
    }

    fn cancel_recording(self: &Arc<Self>) {
        self.reset_session();
        self.overlay("hidden", None, false);
    }

    fn stop_recording(self: &Arc<Self>) {
        hotkey::set_recording(false);
        let Some(session) = self.session.lock().unwrap().take() else { return };
        self.overlay("processing", None, false);
        let me = self.clone();
        let released = Instant::now();
        tauri::async_runtime::spawn(async move {
            let audio = me.audio.clone();
            let rec = tauri::async_runtime::spawn_blocking(move || audio.stop()).await.ok().flatten();
            let Some(rec) = rec else {
                me.overlay("error", Some("Recording failed".into()), false);
                return me.overlay_hide_after(2000);
            };
            me.process(rec, session.target, session.profile, released).await;
        });
    }

    // ───────────────────────── processing ─────────────────────────

    /// `released` = when the user let go of the key: `total_ms` is the latency they feel.
    async fn process(self: &Arc<Self>, rec: Recording, target: Target, profile: Profile, released: Instant) {
        let _in_order = self.process_lock.lock().await;
        let t0 = released;
        let mode = profile.mode;
        if rec.speech_ms < 200 {
            self.overlay("notice", Some("No speech detected".into()), false);
            return self.overlay_hide_after(900);
        }
        let s = self.settings();
        let asr = self.asr.read().unwrap().clone();
        if !asr.healthy().await {
            self.overlay("processing", Some("Loading speech model…".into()), false);
            if let Err(e) = self.start_asr().await {
                tracing::error!("speech engine unavailable: {e}");
                *self.failed_audio.lock().unwrap() = Some((rec, target, profile));
                self.overlay("error", Some("Speech engine stopped. Restart it in Settings.".into()), false);
                return self.overlay_hide_after(3500);
            }
        }
        let prompt = vocabulary_prompt(&s.dictionary.terms);
        let whisper = self.whisper.read().unwrap().clone();
        let t_asr = Instant::now();
        let text = match whisper.transcribe(&rec.samples, &s.asr.language, &prompt).await {
            Ok(t) => t.text,
            Err(e) => {
                tracing::warn!("transcription failed ({e}); restarting speech engine once");
                let retry = async {
                    asr.restart(Duration::from_secs(90)).await?;
                    whisper.transcribe(&rec.samples, &s.asr.language, &prompt).await
                };
                match retry.await {
                    Ok(t) => t.text,
                    Err(e) => {
                        tracing::error!("transcription failed after restart: {e}");
                        *self.failed_audio.lock().unwrap() = Some((rec, target, profile));
                        self.overlay("error", Some("Couldn't transcribe. Retry from the tray.".into()), false);
                        return self.overlay_hide_after(3500);
                    }
                }
            }
        };
        let asr_ms = t_asr.elapsed().as_millis() as i64;
        let Some(raw) = asr_filter::clean(&text, rec.speech_ms) else {
            self.overlay("notice", Some("No speech detected".into()), false);
            return self.overlay_hide_after(900);
        };

        // Zero-loss point: the words are on disk before anything else can fail.
        let id = if s.history.enabled {
            self.store(|st| st.insert_transcribed(&target.ctx.exe, mode, &raw, i64::from(rec.duration_ms), asr_ms)).ok()
        } else {
            None
        };

        let dict = self.dict.read().unwrap().clone();
        let llm = self.llm.read().unwrap().clone();
        let use_llm = s.llm.enabled && mode.uses_llm();
        let outcome = pipeline::process(
            pipeline::Request {
                raw: &raw,
                profile,
                app_exe: &target.ctx.exe,
                dict: &dict,
                terms: &s.dictionary.terms,
                spoken_commands: s.spoken_commands,
                fast_path_words: s.llm.fast_path_words,
                timeout: Duration::from_millis(s.llm.timeout_ms),
            },
            use_llm.then_some(&llm),
        )
        .await;
        if let Path::Fallback(why) = &outcome.path {
            // Guard messages quote the user's words; logs keep only the kind (history has the text).
            let kind = if why.starts_with("guard") { "guard rejected the rewrite" } else { why.as_str() };
            tracing::warn!("rewrite fallback: {kind}");
            if why.starts_with("llm error") || why.starts_with("timeout") {
                self.heal_llm();
            }
        }
        let note = match &outcome.path {
            Path::Fallback(w) => Some(w.as_str()),
            _ => None,
        };
        if let Some(id) = id {
            let _ = self.store(|st| {
                st.set_final(id, &outcome.text, outcome.path == Path::Llm, outcome.llm_ms.map(|m| m as i64), note)
            });
        }

        let (status, inject_ms, err) = self.insert(&outcome.text, &target.ctx, profile, &s).await;
        if let Some(id) = id {
            let total = t0.elapsed().as_millis() as i64;
            let _ = self.store(|st| st.set_status(id, status, Some(inject_ms), Some(total), err.as_deref()));
        }
        let _ = self.app.emit("history", ());
        if self.session.lock().unwrap().is_some() && status != RowStatus::Failed {
            return; // the user is already dictating again: keep the pill on "listening"
        }
        match status {
            RowStatus::Inserted => {
                self.overlay("done", None, false);
                self.overlay_hide_after(350);
            }
            RowStatus::Failed => {
                let msg = if s.history.enabled {
                    "Couldn't paste or copy. The text is in History."
                } else {
                    "Couldn't paste or copy: the clipboard is busy."
                };
                self.overlay("error", Some(msg.into()), false);
                self.overlay_hide_after(3500);
            }
            _ => {
                let msg = if target.ctx.elevated {
                    BLOCKED_HINT.to_string()
                } else {
                    format!("Copied. Paste it with {PASTE_KEYS}.")
                };
                self.overlay("notice", Some(msg), false);
                self.overlay_hide_after(3000);
            }
        }
    }

    /// Paste into the focused window, or leave the text on the clipboard when Windows would
    /// block synthetic input (elevated target) or the paste fails.
    async fn insert(
        &self,
        text: &str,
        ctx: &AppContext,
        profile: Profile,
        s: &Settings,
    ) -> (RowStatus, i64, Option<String>) {
        let _guard = self.inject_lock.lock().await;
        let t = Instant::now();
        // The user may have switched windows while we transcribed; text goes where focus is now.
        let now = target::has_foreground().then(target::foreground);
        let elevated = now.as_ref().map(|t| t.ctx.elevated).unwrap_or(ctx.elevated);
        let ms = || t.elapsed().as_millis() as i64;
        if elevated || now.is_none() {
            return match inject::set_text(text, true) {
                Ok(_) => (RowStatus::ClipboardOnly, ms(), Some(BLOCKED_REASON.into())),
                Err(e) => (RowStatus::Failed, ms(), Some(e)),
            };
        }
        let keys = now.map(|n| resolve(&n.ctx, &s.app_rules, profile.mode, true).paste).unwrap_or(profile.paste);
        let opts = PasteOptions {
            keys,
            restore_clipboard: s.paste.restore_clipboard,
            restore_delay_ms: s.paste.restore_delay_ms,
        };
        let text = text.to_string();
        let res = tauri::async_runtime::spawn_blocking(move || {
            inject::paste(&text, &opts).map_err(|e| {
                let copied = inject::set_text(&text, true).is_ok();
                (e, copied)
            })
        })
        .await
        .unwrap_or_else(|e| Err((e.to_string(), false)));
        match res {
            Ok(()) => (RowStatus::Inserted, ms(), None),
            Err((e, copied)) => {
                tracing::warn!("paste failed: {e} (text on clipboard: {copied})");
                (if copied { RowStatus::ClipboardOnly } else { RowStatus::Failed }, ms(), Some(e))
            }
        }
    }

    fn cycle_mode(self: &Arc<Self>) {
        let mode = {
            let mut s = self.settings.write().unwrap();
            s.mode = s.mode.next();
            let _ = settings::save(&self.paths.settings, &s);
            s.mode
        };
        self.overlay("notice", Some(format!("Mode: {}", mode_label(mode))), false);
        self.overlay_hide_after(1000);
        let _ = self.app.emit("settings", ());
        crate::tray::refresh(&self.app);
    }

    pub async fn repaste_last(self: &Arc<Self>) {
        let Some(last) = self.store(|st| st.last()).ok().flatten() else {
            self.overlay("notice", Some("Nothing to paste yet".into()), false);
            return self.overlay_hide_after(1200);
        };
        let s = self.settings();
        let ctx = target::foreground().ctx;
        let profile = resolve(&ctx, &s.app_rules, s.mode, s.auto_context);
        let (status, _, _) = self.insert(last.best_text(), &ctx, profile, &s).await;
        if status != RowStatus::Inserted {
            self.overlay("notice", Some(format!("Copied. Paste it with {PASTE_KEYS}.")), false);
            self.overlay_hide_after(2500);
        }
    }

    pub async fn retry_failed(self: &Arc<Self>) {
        let failed = self.failed_audio.lock().unwrap().take();
        if let Some((rec, target, profile)) = failed {
            self.overlay("processing", None, false);
            self.process(rec, target, profile, Instant::now()).await;
        }
    }

    pub fn has_failed_audio(&self) -> bool {
        self.failed_audio.lock().unwrap().is_some()
    }

    /// Re-run the text pipeline on a stored dictation in another mode (no paste).
    pub async fn reprocess(&self, id: i64, mode: Mode) -> Result<String, String> {
        let d = self.store(|st| st.get(id)).map_err(|e| e.to_string())?.ok_or("not found")?;
        let s = self.settings();
        let dict = self.dict.read().unwrap().clone();
        let llm = self.llm.read().unwrap().clone();
        let profile = Profile { mode, paste: Default::default(), terminal_period: true };
        let o = pipeline::process(
            pipeline::Request {
                raw: &d.raw_text,
                profile,
                app_exe: &d.app_exe,
                dict: &dict,
                terms: &s.dictionary.terms,
                spoken_commands: s.spoken_commands,
                fast_path_words: s.llm.fast_path_words,
                timeout: Duration::from_millis(s.llm.timeout_ms.max(4000)),
            },
            (s.llm.enabled && mode.uses_llm()).then_some(&llm),
        )
        .await;
        Ok(o.text)
    }

    // ───────────────────────── settings ─────────────────────────

    pub fn update_settings(self: &Arc<Self>, new: Settings) -> Result<(), String> {
        validate(&new)?;
        let old = self.settings();
        settings::save(&self.paths.settings, &new).map_err(|e| format!("could not save settings: {e}"))?;
        *self.dict.write().unwrap() = Arc::new(DictMatcher::new(&new.dictionary));
        hotkey::configure(&new.hotkey.dictation_key, new.hotkey.cycle_mode_key, new.hotkey.repaste_key);
        // Only the threshold: resetting the gesture mid-dictation would orphan the recording
        // (mic left open, key release ignored).
        self.gesture.lock().unwrap().hold_threshold_ms = new.hotkey.hold_threshold_ms;
        *self.llm.write().unwrap() = OpenAiCompat::new(&llm_endpoint(&new), &new.llm.model);
        *self.settings.write().unwrap() = new.clone();
        crate::autostart_sync(&self.app, new.autostart);
        let asr_changed = old.asr != new.asr;
        let llm_changed = old.llm != new.llm;
        if asr_changed || llm_changed {
            let me = self.clone();
            tauri::async_runtime::spawn(async move {
                if asr_changed {
                    *me.whisper.write().unwrap() = WhisperClient::new(new.asr.port);
                    let old =
                        std::mem::replace(&mut *me.asr.write().unwrap(), Arc::new(asr_sidecar(&me.paths, &new, true)));
                    old.stop();
                    let _ = me.start_asr().await;
                }
                if llm_changed {
                    if let Some(old) = me.llm_server.write().unwrap().take() {
                        old.stop();
                    }
                    let _ = me.start_llm().await;
                }
            });
        }
        crate::tray::refresh(&self.app);
        Ok(())
    }

    // ───────────────────────── inference servers ─────────────────────────

    pub fn missing_assets(&self) -> Vec<&'static str> {
        let s = self.settings.read().unwrap();
        let mut v = Vec::new();
        if !self.paths.whisper_exe().exists() {
            v.push("whisper-runtime");
        }
        if !self.paths.models.join(&s.asr.model).exists() {
            v.push("asr-model");
        }
        if !self.paths.models.join("ggml-silero-v6.2.0.bin").exists() {
            v.push("silero-vad");
        }
        if s.llm.enabled && s.llm.managed {
            if !self.paths.llama_exe().exists() {
                v.push("llama-runtime");
            }
            if !self.paths.models.join(&s.llm.model).exists() {
                v.push("llm-model");
            }
        }
        v
    }

    /// Start both servers in the background and warm them up so the first dictation is fast.
    pub fn start_engines(self: &Arc<Self>) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let gpu = tauri::async_runtime::spawn_blocking(gpu_info).await.ok().flatten();
            me.set_status(|st| st.gpu = gpu);
            if !me.missing_assets().is_empty() {
                me.set_status(|st| {
                    st.asr = "setup needed".into();
                    st.llm = "setup needed".into();
                });
                return;
            }
            let (a, l) = tokio::join!(me.start_asr(), me.start_llm());
            if let Err(e) = a {
                tracing::error!("speech engine: {e}");
            }
            if let Err(e) = l {
                tracing::error!("rewrite engine: {e}");
            }
            // Models are in VRAM now; show real usage.
            let gpu = tauri::async_runtime::spawn_blocking(gpu_info).await.ok().flatten();
            me.set_status(|st| st.gpu = gpu);
        });
    }

    async fn start_asr(self: &Arc<Self>) -> Result<(), String> {
        self.set_status(|st| st.asr = "starting".into());
        let side = self.asr.read().unwrap().clone();
        let mut res = side.ensure_running(Duration::from_secs(90)).await;
        if res.is_err() && self.settings().asr.use_gpu {
            // Tier 3 fallback: GPU busy / out of memory → CPU.
            tracing::warn!("GPU speech engine failed, retrying on CPU: {:?}", res);
            let cpu = Arc::new(asr_sidecar(&self.paths, &self.settings(), false));
            side.stop();
            *self.asr.write().unwrap() = cpu.clone();
            res = cpu.ensure_running(Duration::from_secs(90)).await.map(|_| ());
            if res.is_ok() {
                self.set_status(|st| st.asr = "ready (CPU)".into());
                return Ok(());
            }
        }
        match &res {
            Ok(()) => {
                // First CUDA inference compiles kernels; do it now, not on the user's first words.
                let w = self.whisper.read().unwrap().clone();
                let _ = w.transcribe(&vec![0.0; audio::RATE as usize], "en", "").await;
                self.set_status(|st| st.asr = "ready".into());
            }
            Err(e) => self.set_status(|st| st.asr = format!("error: {}", first_line(e))),
        }
        res
    }

    async fn start_llm(self: &Arc<Self>) -> Result<(), String> {
        let s = self.settings();
        if !s.llm.enabled {
            self.set_status(|st| st.llm = "off".into());
            return Ok(());
        }
        let llm = self.llm.read().unwrap().clone();
        if s.llm.managed {
            self.set_status(|st| st.llm = "starting".into());
            let side = Arc::new(llm_sidecar(&self.paths, &s, true));
            *self.llm_server.write().unwrap() = Some(side.clone());
            if let Err(e) = side.ensure_running(Duration::from_secs(120)).await {
                tracing::warn!("GPU rewrite engine failed ({e}); retrying on CPU");
                side.stop();
                let cpu = Arc::new(llm_sidecar(&self.paths, &s, false));
                *self.llm_server.write().unwrap() = Some(cpu.clone());
                if let Err(e) = cpu.ensure_running(Duration::from_secs(120)).await {
                    self.set_status(|st| st.llm = format!("error: {}", first_line(&e)));
                    return Err(e);
                }
                self.set_status(|st| st.llm = "ready (CPU)".into());
            }
        } else {
            self.set_status(|st| st.llm = "connecting".into());
        }
        match llm.warm_up().await {
            Ok(()) => {
                if !self.status.lock().unwrap().llm.contains("CPU") {
                    self.set_status(|st| st.llm = "ready".into());
                }
                Ok(())
            }
            Err(e) => {
                self.set_status(|st| st.llm = format!("error: {}", first_line(&e)));
                Err(e)
            }
        }
    }

    /// The LLM failed mid-dictation: restart it in the background (the user already got the
    /// deterministic text).
    fn heal_llm(self: &Arc<Self>) {
        if self.restarting_llm.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let side = me.llm_server.read().unwrap().clone();
            if let Some(side) = side
                && !side.healthy().await
            {
                tracing::warn!("rewrite engine unhealthy; restarting");
                me.set_status(|st| st.llm = "restarting".into());
                match side.restart(Duration::from_secs(120)).await {
                    Ok(()) => me.set_status(|st| st.llm = "ready".into()),
                    Err(e) => me.set_status(|st| st.llm = format!("error: {}", first_line(&e))),
                }
            }
            me.restarting_llm.store(false, Ordering::SeqCst);
        });
    }

    pub fn stop_engines(&self) {
        self.asr.read().unwrap().stop();
        if let Some(l) = self.llm_server.read().unwrap().as_ref() {
            l.stop();
        }
        self.set_status(|st| {
            st.asr = "stopped".into();
            st.llm = "stopped".into();
        });
    }

    // ───────────────────────── setup ─────────────────────────

    /// Download + verify + unpack everything the current settings need. One at a time: a second
    /// run would write the same `.part` files.
    pub async fn install(self: &Arc<Self>) -> Result<(), String> {
        let mut already = false;
        self.set_status(|st| already = std::mem::replace(&mut st.installing, true));
        if already {
            return Err("Setup is already running.".into());
        }
        // Cleared on every exit, including a panic or a dropped future.
        struct Done<'a>(&'a Engine);
        impl Drop for Done<'_> {
            fn drop(&mut self) {
                self.0.set_status(|st| st.installing = false);
            }
        }
        let _done = Done(self);
        self.install_inner().await
    }

    /// What setup still has to fetch on this machine: speech parts, and the AI rewrite parts
    /// (listed even when the rewrite is off, so the UI can show what turning it on costs).
    fn setup_todo(&self) -> (Vec<&'static Asset>, Vec<&'static Asset>) {
        let gpu = self.status.lock().unwrap().gpu.is_some();
        // A 4B model on CPU would miss the rewrite timeout most of the time; install switches to 2B.
        let small = !gpu || self.settings().llm.model.contains("2B");
        #[cfg(windows)]
        let (whisper_rt, llama_rt) =
            (Some(if gpu { "whisper-cuda" } else { "whisper-cpu" }), if gpu { "llama-cuda" } else { "llama-cpu" });
        #[cfg(target_os = "macos")]
        let (whisper_rt, llama_rt) = (None::<&str>, "llama-macos"); // whisper-server ships in the app bundle
        let mut speech: Vec<&str> = whisper_rt.into_iter().collect();
        speech.extend(["silero-vad", "whisper-large-v3-turbo-q8"]);
        let rewrite = [llama_rt, if small { "qwen3.5-2b" } else { "qwen3.5-4b" }];
        let missing = |ids: &[&str]| -> Vec<&'static Asset> {
            ids.iter()
                .filter_map(|id| download::asset(id))
                .filter(|a| match a.kind {
                    Kind::Model => !self.paths.models.join(a.file).exists(),
                    Kind::Runtime => !self.paths.runtime.join(runtime_dir(a.id)).join(runtime_exe(a.id)).exists(),
                })
                .collect()
        };
        (missing(&speech), missing(&rewrite))
    }

    pub fn setup_plan(&self) -> SetupPlan {
        let (speech, rewrite) = self.setup_todo();
        let sum = |v: &[&Asset]| v.iter().map(|a| a.bytes).sum();
        SetupPlan { speech_bytes: sum(&speech), rewrite_bytes: sum(&rewrite) }
    }

    async fn install_inner(self: &Arc<Self>) -> Result<(), String> {
        let gpu = self.status.lock().unwrap().gpu.is_some();
        if !gpu && self.settings().llm.model.contains("4B") {
            let mut s = self.settings();
            s.llm.model = "Qwen3.5-2B-Q4_K_M.gguf".into();
            self.update_settings(s)?;
        }
        let s = self.settings();
        let (mut todo, rewrite) = self.setup_todo();
        if s.llm.enabled && s.llm.managed {
            todo.extend(rewrite);
        }
        // Fail up front with a clear message instead of an OS error gigabytes in. A runtime
        // archive and its unpacked copy exist side by side for a moment, hence 3x.
        let need: u64 = todo.iter().map(|a| if a.kind == Kind::Runtime { a.bytes * 3 } else { a.bytes }).sum();
        let _ = std::fs::create_dir_all(&self.paths.root);
        if let Ok(free) = fs4::available_space(&self.paths.root)
            && free < need + 200_000_000
        {
            let gb = |b: u64| b as f64 / 1e9;
            return Err(format!(
                "Not enough disk space: setup needs about {:.1} GB free on the drive holding {}, and {:.1} GB is free.",
                gb(need + 200_000_000),
                self.paths.root.display(),
                gb(free)
            ));
        }
        for a in todo {
            let id = a.id;
            let dest = match a.kind {
                Kind::Model => self.paths.models.join(a.file),
                Kind::Runtime => self.paths.cache.join(a.file),
            };
            let app = self.app.clone();
            let last_emit = Mutex::new(Instant::now());
            download::download(a.url, &dest, a.sha256, move |done, total| {
                let mut last = last_emit.lock().unwrap();
                if last.elapsed() > Duration::from_millis(150) || done == total {
                    *last = Instant::now();
                    let _ = app.emit(
                        "setup-progress",
                        serde_json::json!({ "id": id, "label": a.label, "done": done, "total": total.max(a.bytes) }),
                    );
                }
            })
            .await?;
            if a.kind == Kind::Runtime {
                let _ = self
                    .app
                    .emit("setup-progress", serde_json::json!({ "id": id, "label": a.label, "phase": "unpacking" }));
                let dir = self.paths.runtime.join(runtime_dir(id));
                let (archive, d) = (dest.clone(), dir.clone());
                tauri::async_runtime::spawn_blocking(move || download::extract(&archive, &d))
                    .await
                    .map_err(|e| e.to_string())??;
                let _ = std::fs::remove_file(&dest);
            }
        }
        link_cuda_runtime(&self.paths);
        let _ = self.app.emit("setup-progress", serde_json::json!({ "phase": "done" }));
        self.start_engines();
        Ok(())
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").chars().take(120).collect()
}

fn mode_label(m: Mode) -> &'static str {
    match m {
        Mode::Verbatim => "Verbatim",
        Mode::Clean => "Clean",
        Mode::Concise => "Concise",
        Mode::Professional => "Professional",
        Mode::Technical => "Technical",
        Mode::Math => "Math / LaTeX",
    }
}

fn llm_endpoint(s: &Settings) -> String {
    if s.llm.managed { format!("http://127.0.0.1:{}/v1", s.llm.port) } else { s.llm.endpoint.clone() }
}

fn runtime_dir(id: &str) -> &'static str {
    if id.starts_with("whisper") { "whisper" } else { "llama" }
}

fn runtime_exe(id: &str) -> &'static str {
    match (id.starts_with("whisper"), cfg!(windows)) {
        (true, true) => "whisper-server.exe",
        (true, false) => "whisper-server",
        (false, true) => "llama-server.exe",
        (false, false) => "llama-server",
    }
}

/// How the user pastes by hand, and why we could not paste for them.
#[cfg(windows)]
const PASTE_KEYS: &str = "Ctrl+V";
#[cfg(windows)]
const BLOCKED_HINT: &str = "Admin window: press Ctrl+V to paste";
#[cfg(windows)]
const BLOCKED_REASON: &str = "target window is elevated (UIPI)";
#[cfg(target_os = "macos")]
const PASTE_KEYS: &str = "Command+V";
#[cfg(target_os = "macos")]
const BLOCKED_HINT: &str = "Secure input is on: press Command+V";
#[cfg(target_os = "macos")]
const BLOCKED_REASON: &str = "secure keyboard entry is on (password field or terminal setting)";

/// D7: llama.cpp needs the same CUDA 12.4 DLLs whisper.cpp ships; hard-link instead of a
/// second 391 MB download.
fn link_cuda_runtime(p: &Paths) {
    let (w, l) = (p.runtime.join("whisper"), p.runtime.join("llama"));
    if !l.join("ggml-cuda.dll").exists() {
        return;
    }
    for dll in ["cudart64_12.dll", "cublas64_12.dll", "cublasLt64_12.dll"] {
        let (src, dst) = (w.join(dll), l.join(dll));
        if src.exists() && !dst.exists() && std::fs::hard_link(&src, &dst).is_err() {
            let _ = std::fs::copy(&src, &dst);
        }
    }
}

fn asr_sidecar(p: &Paths, s: &Settings, gpu: bool) -> Sidecar {
    let mut args = vec![
        "-m".into(),
        p.models.join(&s.asr.model).display().to_string(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        s.asr.port.to_string(),
        "-nt".into(),
        "--vad".into(),
        "-vm".into(),
        p.models.join("ggml-silero-v6.2.0.bin").display().to_string(),
    ];
    if !gpu || !s.asr.use_gpu {
        args.push("-ng".into());
    }
    Sidecar::new(
        Spec {
            name: "whisper-server",
            exe: p.whisper_exe(),
            args,
            port: s.asr.port,
            health_path: "/health",
            log_path: p.logs.join("whisper-server.log"),
        },
        Some(Box::new(winutil::adopt_child)),
    )
}

fn llm_sidecar(p: &Paths, s: &Settings, gpu: bool) -> Sidecar {
    let args = vec![
        "-m".into(),
        p.models.join(&s.llm.model).display().to_string(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        s.llm.port.to_string(),
        "-ngl".into(),
        if gpu { "99".into() } else { "0".into() },
        "-c".into(),
        "4096".into(),
        "-np".into(),
        "1".into(),
        "--jinja".into(),
        "-fa".into(),
        "on".into(),
        "--no-webui".into(),
    ];
    Sidecar::new(
        Spec {
            name: "llama-server",
            exe: p.llama_exe(),
            args,
            port: s.llm.port,
            health_path: "/health",
            log_path: p.logs.join("llama-server.log"),
        },
        Some(Box::new(winutil::adopt_child)),
    )
}

fn validate(s: &Settings) -> Result<(), String> {
    if !(100..=1000).contains(&s.hotkey.hold_threshold_ms) {
        return Err("Hold threshold must be between 100 and 1000 ms".into());
    }
    if !(300..=15_000).contains(&s.llm.timeout_ms) {
        return Err("LLM timeout must be between 300 and 15000 ms".into());
    }
    if !(100..=2000).contains(&s.paste.restore_delay_ms) {
        return Err("Clipboard restore delay must be between 100 and 2000 ms".into());
    }
    if !(300..=10_000).contains(&s.audio.hands_free_silence_ms) {
        return Err("Hands-free silence must be between 300 and 10000 ms".into());
    }
    if s.asr.port == s.llm.port {
        return Err("Speech and rewrite servers need different ports".into());
    }
    if !s.llm.managed && !(s.llm.endpoint.starts_with("http://") || s.llm.endpoint.starts_with("https://")) {
        return Err("LLM endpoint must be an http(s) URL".into());
    }
    if !s.hotkey.cycle_mode_key.is_ascii_alphanumeric() || !s.hotkey.repaste_key.is_ascii_alphanumeric() {
        return Err("Shortcut keys must be a letter or digit".into());
    }
    if s.hotkey.cycle_mode_key.eq_ignore_ascii_case(&s.hotkey.repaste_key) {
        return Err("The mode and re-paste shortcuts need different letters".into());
    }
    if !(10..=3600).contains(&s.audio.max_recording_secs) {
        return Err("Maximum recording length must be between 10 and 3600 seconds".into());
    }
    Ok(())
}

/// "Apple M2 · 16 GB unified memory" on Apple silicon (Metal). Intel Macs get CPU builds.
#[cfg(target_os = "macos")]
pub fn gpu_info() -> Option<String> {
    if !cfg!(target_arch = "aarch64") {
        return None;
    }
    let sysctl = |k: &str| {
        let o = std::process::Command::new("sysctl").args(["-n", k]).output().ok()?;
        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let chip = sysctl("machdep.cpu.brand_string").filter(|c| !c.is_empty()).unwrap_or_else(|| "Apple silicon".into());
    let mem = sysctl("hw.memsize").and_then(|m| m.parse::<f64>().ok()).map(|b| b / 1_073_741_824.0);
    Some(match mem {
        Some(gb) => format!("{chip} · {gb:.0} GB unified memory"),
        None => chip,
    })
}

/// "NVIDIA GeForce RTX 4060 Laptop GPU · 4.3 / 8.0 GB" or None without an NVIDIA GPU.
#[cfg(windows)]
pub fn gpu_info() -> Option<String> {
    let mut cmd = std::process::Command::new("nvidia-smi");
    cmd.args(["--query-gpu=name,memory.used,memory.total", "--format=csv,noheader,nounits"]);
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok().filter(|o| o.status.success())?;
    let line = String::from_utf8_lossy(&out.stdout).lines().next()?.to_string();
    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
    let (name, used, total) = (parts.first()?, parts.get(1)?.parse::<f32>().ok()?, parts.get(2)?.parse::<f32>().ok()?);
    Some(format!("{name} · {:.1} / {:.1} GB", used / 1024.0, total / 1024.0))
}
