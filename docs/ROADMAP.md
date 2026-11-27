# Roadmap

What is planned, roughly in order. Status is honest: nothing here is promised by a date. Want
to help with one? Comment on its issue, or open one if it has none yet.

## Now (v0.3)

- **Verify the macOS beta on real Macs.** Hold-to-talk, the HUD, paste and clipboard restore
  have only run in CI. Reports via the "macOS test report" issue template.
- **Real-voice benchmarks.** Today's numbers use synthesized voices. A small, consented set of
  real recordings (accents, laptop mics, background noise) would make the accuracy numbers mean
  more.
- **Signed Windows builds** so SmartScreen stops warning (needs a free signing program such as
  SignPath for open source, or a paid certificate).

## Next

- **Linux (X11 first, then Wayland).** The engine is portable; the four OS modules (`hotkey`,
  `inject`, `target`, `winutil`) need Linux versions.
- **More languages.** Whisper large-v3-turbo is multilingual; the rules and the rewrite prompts
  are English-only today.
- **Opt-in update check.** One request to GitHub's releases API, off by default, so Voxora stays
  offline unless asked.
- **Microphone level meter in Settings**, so a wrong or muted mic is obvious before dictating.

## Good first issues

Small, well-scoped, mostly in `crates/voxora-core` (pure Rust, testable without a GPU):

- **AltGr layouts:** detect keyboard layouts where Right Alt is AltGr and suggest another
  dictation key on first run (Windows `src-tauri/src/hotkey.rs`, UI in `src/pages/Settings.tsx`).
- **Settings file validation on load:** a hand-edited `settings.json` with an out-of-range value
  should fall back for that field only, not fail later (`crates/voxora-core/src/settings.rs`).
- **Keep server logs across restarts:** `sidecar.rs` truncates the server log on every start,
  which erases the reason for a crash. Rotate it instead.
- **Quieter HUD between back-to-back dictations:** a notice from an earlier dictation ("No
  speech detected") can briefly replace the pill of the one being recorded (`engine.rs`,
  `process`).
- **More spoken symbols and units** in `normalize.rs`, each with a test.

## Not planned

- Cloud transcription or cloud AI by default. Voxora stays local-first; an OpenAI-compatible
  endpoint you run yourself is already supported.
- Accounts, telemetry or analytics.
