# Decision Log

One line of context, the decision, the trade-off accepted. Newest last.

## D1 — Name: Voxora (2026-11-23)
Blueprint (`docs/BLUEPRINT.md`) calls the app "VoxFlow"; the GitHub repo is `Voxora`. All code, paths and
data dirs use **Voxora** (`%LOCALAPPDATA%\Voxora`).

## D2 — Inference runs in prebuilt CUDA sidecars, not FFI (2026-11-23)
**Options:** (a) `whisper-rs`/`transcribe-rs` compiled with CUDA via FFI; (b) official prebuilt
`whisper-server.exe` + `llama-server.exe` CUDA binaries supervised as localhost child processes.
**Decision:** (b). Verified 2026-11-23 via GitHub API: whisper.cpp `b5907` ships
`whisper-bin-win-cuda-12.4.0-x64.zip`; llama.cpp `b12649` ships `llama-*-bin-win-cuda-12.4-x64.zip`
(+ `cudart-*` runtime zip). No CUDA Toolkit / nvcc needed to build or contribute.
**Trade-off:** ~1-3 ms localhost HTTP overhead per call and ~1 GB one-time download; in exchange the Rust
build is plain MSVC, a crash in inference cannot take down the hotkey/clipboard process, and engines are
swappable (any OpenAI-compatible server, e.g. Ollama, works for rewriting).

## D3 — Provider-neutral rewrite client (2026-11-23)
Rewrite stage talks to an OpenAI-compatible `/v1/chat/completions` endpoint with JSON-schema constrained
output. Default: managed `llama-server` on `127.0.0.1:18080`. Ollama (`127.0.0.1:11434/v1`) works by
config. Cloud providers stay out of core (blueprint FR-15 is P2, opt-in only).

## D4 — Cargo workspace: pure `voxora-core` + Windows `src-tauri` app (2026-11-23)
All text logic (normalizer, math, invariant guard, hallucination filter, modes, prompts, context rules,
storage, state machine) lives in `crates/voxora-core` with no Win32/Tauri deps, so it unit-tests in
seconds and the `bench_eval` CLI reuses it. Win32 hooks, audio, clipboard, sidecars live in `src-tauri`.

## D5 — Owner defaults (2026-11-23)
Hotkey **Right Alt** (hold = push-to-talk, tap < 250 ms = hands-free toggle), ASR default
**whisper-large-v3-turbo**, mic = **Windows default input**, **autostart on login, minimized to tray**.
