# Decision Log

One line of context, the decision, the trade-off accepted. Newest last.

## D1: Name: Voxora (2026-11-23)
Blueprint (`docs/BLUEPRINT.md`) calls the app "VoxFlow"; the GitHub repo is `Voxora`. All code, paths and
data dirs use **Voxora** (`%LOCALAPPDATA%\Voxora`).

## D2: Inference runs in prebuilt CUDA sidecars, not FFI (2026-11-23)
**Options:** (a) `whisper-rs`/`transcribe-rs` compiled with CUDA via FFI; (b) official prebuilt
`whisper-server.exe` + `llama-server.exe` CUDA binaries supervised as localhost child processes.
**Decision:** (b). Verified 2026-11-23 via GitHub API: whisper.cpp `b5907` ships
`whisper-bin-win-cuda-12.4.0-x64.zip`; llama.cpp `b12649` ships `llama-*-bin-win-cuda-12.4-x64.zip`
(+ `cudart-*` runtime zip). No CUDA Toolkit / nvcc needed to build or contribute.
**Trade-off:** ~1-3 ms localhost HTTP overhead per call and ~1 GB one-time download; in exchange the Rust
build is plain MSVC, a crash in inference cannot take down the hotkey/clipboard process, and engines are
swappable (any OpenAI-compatible server, e.g. Ollama, works for rewriting).

## D3: Provider-neutral rewrite client (2026-11-23)
Rewrite stage talks to an OpenAI-compatible `/v1/chat/completions` endpoint with JSON-schema constrained
output. Default: managed `llama-server` on `127.0.0.1:18080`. Ollama (`127.0.0.1:11434/v1`) works by
config. Cloud providers stay out of core (blueprint FR-15 is P2, opt-in only).

## D4: Cargo workspace: pure `voxora-core` + Windows `src-tauri` app (2026-11-23)
All text logic (normalizer, math, invariant guard, hallucination filter, modes, prompts, context rules,
storage, state machine) lives in `crates/voxora-core` with no Win32/Tauri deps, so it unit-tests in
seconds and the `bench_eval` CLI reuses it. Win32 hooks, audio, clipboard, sidecars live in `src-tauri`.

## D5: Owner defaults (2026-11-23)
Hotkey **Right Alt** (hold = push-to-talk, tap < 250 ms = hands-free toggle), ASR default
**whisper-large-v3-turbo**, mic = **Windows default input**, **autostart on login, minimized to tray**.

## D6: Rewrite model: Qwen3.5-4B Q4_K_M default, 2B as "Fast" preset (2026-11-23)
Blueprint named Qwen2.5-3B (2024); newer Apache-2.0 Qwen3.5 (Feb 2026) GGUFs exist. Measured on the
RTX 4060 with `bench_eval` (30 cases, synthesized speech, 3 runs): **4B: 29/30, LLM P50 357 ms / P95
929 ms; 2B: 28/30, P50 195 ms / P95 466 ms** but 2B failed the blueprint's flagship self-correction case.
Correctness wins; 2B stays one click away. Thinking is disabled per request
(`chat_template_kwargs.enable_thinking=false`); verified no reasoning tokens are produced.
VRAM with whisper turbo Q8 + 4B loaded: 4.3 GB of 8 GB.

## D7: Share CUDA DLLs between runtimes (2026-11-23)
`cudart-llama-bin-win-cuda-12.4-x64.zip` contains exactly `cublas64_12.dll`, `cublasLt64_12.dll`,
`cudart64_12.dll`, which the whisper.cpp CUDA zip already ships. Setup hard-links them into the llama
runtime instead of downloading 391 MB again. Each server keeps its own folder because their
`ggml*.dll` builds differ.

## D8: Whisper vocabulary prompt on by default (2026-11-23)
Without a prompt Whisper heard "FAISS dense" as "phaistense"; with `Glossary: …, FAISS, BM25.` it was
exact. Warm latency unchanged (~200 ms for 8.8 s audio).

## D9: No speculative GPU keep-warm (2026-11-23)
whisper-server is ~2x slower on the first request after a few seconds idle (8.8 s clip: 410-560 ms
cold vs ~200 ms warm). Tried: a 1-token llama-server request at key-down, then every 0.9 s while
recording. Measured with curl: cold 562 ms → 499 ms after pings, 314 ms when overlapping; not a
reliable win, and it adds GPU work per dictation. Removed. End-to-end latency (key release → text
inserted, release build, 7 runs) is 0.45-0.77 s without the LLM and 0.85-1.24 s with it, inside the
blueprint's < 1.1 s P50 target. Next idea if needed: send partial audio to whisper while recording.

## D10: E2E harness must never paste outside its own window (2026-11-23)
Voxora pastes into whatever window has focus. Two early harness runs lost foreground to the
developer's editor and pasted test sentences there. `scripts/e2e.ps1` now acquires focus with
`AttachThreadInput`, verifies it before every key release, and on loss presses Esc (cancel) before
releasing the key. Run it only when nobody is using the machine.

## D11: v0.2 scope from owner feedback (2026-11-24)
- Redesign to a calm, Apple-like look: monochrome logo (white tile, black bars), medium/large type,
  generous spacing. No em dashes anywhere in UI copy or docs (dictated output may contain them).
- macOS support, Apple Silicon first. No Mac available: validated on GitHub macOS runners only and
  labelled beta until a community tester confirms hotkey, paste and permissions on hardware.
  Default key on macOS: Right Option.
- Voxora stays literal: it types what you said minus fillers, retracted text and spoken
  punctuation. It never acts on instructions inside the dictation ("send a message saying X"
  stays as said).
- Golden set of 20 hard cases (bench/golden.json) added next to the 30 existing bench cases.

## D12: Golden edge cases handled by rules first, LLM decoding made greedy (2026-11-24)
The 20-case golden set started at 11/19 key facts with reference text. Most misses were spoken
syntax the LLM could not be trusted to rebuild (URLs, paths, flags, "quote ... end quote",
Big-O, ordinal lists), so they became deterministic rules in `normalize.rs`, each with a test.
Result: 19/19 key facts, 16/19 exact. The LLM sampled at temperature 0.1 gave different words
between runs ("ship v2" vs "shipping v2"); temperature 0 made two consecutive runs identical
with no quality loss on either benchmark (30/30 and 19/19). Spoken symbol joining is
Technical-mode only and needs a strong joiner (slash, backslash, underscore, or a dot before a
known extension/TLD), so prose like "the red dot" is never glued together.

## D13: macOS port shape (2026-11-24)
- Same engine, four swapped OS modules (`#[cfg_attr(path)]`), not a trait layer: each OS has
  exactly one implementation, so a trait would add indirection with no second caller.
- Listen-only `CGEventTap`, not an active tap: it cannot swallow the key, but an active tap
  that loses Accessibility can freeze keyboard input system-wide (Apple forum reports). One
  permission (Accessibility) covers listening and posting Command+V.
- HUD via `orderFrontRegardless` + ignore-mouse on Tauri's NSWindow instead of the
  `tauri-nspanel` plugin: no extra dependency; it never makes the window key. Revisit if a real
  Mac shows focus being stolen.
- whisper.cpp publishes no macOS server binary. CI builds `whisper-server` from tag b5907
  (Metal, static, shaders embedded, links only system frameworks) and bundles it with
  `externalBin`, so there is no third-party binary host to trust. llama.cpp's official
  b12649 macOS arm64 build is pinned by SHA-256 (GitHub digest and an independent hash agreed).
- Verified only on GitHub's macos-15 runners (owner has no Mac): clippy, tests, bundling, a
  spoken sentence transcribed correctly by the bundled server, llama runtime start, and app
  launch with the event tap installed. Labelled beta until someone confirms by hand.
- Bundle identifier changed from `dev.voxora.app` to `dev.voxora.desktop` (an `.app` suffix
  clashes with macOS bundles); safe because no installer had been published.

## D14: Audio glitches must not cancel a dictation (2026-11-24, v0.2.1)
Bug reported on the owner's laptop: holding Right Alt did nothing. Instrumented each stage:
the hook delivered the key, recording started, and 50 ms later the built-in Realtek mic
reported one WASAPI buffer overrun (`cpal::ErrorKind::Xrun`). The engine treated every stream
error as "microphone disconnected" and cancelled, so the HUD vanished before it was visible.
Earlier end-to-end tests used the VB-Audio virtual cable, which never overruns, so they could
not catch it. Fix: only errors that end the stream cancel; `Xrun`, `DeviceChanged` and
`RealtimeDenied` are logged at debug level and recording continues (regression test
`buffer_glitches_do_not_cancel_a_dictation`). Also added stage logs ("recording started",
gesture input/action at debug) so the next report can be diagnosed from the log alone.
Lesson: run the real-hardware check on the default microphone, not only the virtual cable.

## D15: Raw device input off; the hotkey must work inside Voxora too (2026-11-24, v0.2.2)
Owner report after v0.2.1: dictation worked in VS Code and Chrome but pressing Right Alt in
Voxora's own window did nothing. Reproduced with injected keys and a hook-level log: while any
Voxora window had focus, Windows never called our `WH_KEYBOARD_LL` hook for key-downs (Right
Alt down, Esc down); key-ups still arrived. Ruled out by experiment: hook-chain order
(re-installing the hook on focus changed nothing) and WebView2 focus. Cause: tao registers the
process for raw keyboard input while one of its windows is focused (`DeviceEventFilter::
Unfocused`, the default). Voxora uses no device events, so `device_event_filter(Always)`
removes the registration; the hook then sees every key, verified 6/6 and in `scripts/e2e.ps1`.
Second bug found on the way: the hook swallowed the lone key-up whose down it never saw, so
Windows kept Alt marked as held system-wide (later keys in any app arrived as Alt+key). Now a
key-up is swallowed only if we swallowed its key-down (tests in `hotkey.rs`). Rejected: a
JS key-forwarding workaround from the webview (worked, but lost the first key after window
activation and duplicated the root fix). The e2e suite now dictates into the Home "Try it" box.