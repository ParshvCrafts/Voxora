# Deep Research and Implementation Blueprint: Local-First, Zero-Cost AI Voice Dictation System (`VoxFlow`)

---

## 1. Executive Summary and Final Recommendation

### What Should Be Built

Build **VoxFlow**, a native-speed, zero-cost, 100% local-first Windows desktop voice dictation and intelligent rewriting application engineered specifically for your **Lenovo Legion 5i (Intel Core i9-14900HX, NVIDIA GeForce RTX 4060 Laptop GPU with 8 GB VRAM, 32 GB DDR5 RAM, Windows 11)**.

Rather than writing an entire speech stack from scratch or settling for a slow Python script, the recommended architecture composes **proven open-source Rust and CUDA engines** into a lightweight, high-reliability desktop utility:

1. **Native Desktop Shell & OS Integration (Rust + Tauri v2 + Win32 APIs):** A ~20 MB background system-tray application with a non-activating floating pill overlay (`WS_EX_NOACTIVATE`), low-level global keyboard hooks (`WH_KEYBOARD_LL`) for true Push-to-Talk and Toggle modes, low-latency WASAPI microphone capture (`cpal`), real-time Silero VAD v5 voice activity detection, active-application context inspection, and atomic Win32 clipboard backup/paste/restore (`SendInput`) backed by a local SQLite write-ahead recovery log.
2. **CUDA-Accelerated Speech-to-Text Engine:** Dual-engine local ASR supporting **`whisper-large-v3-turbo`** (via `whisper.cpp` CUDA or `faster-whisper` FP16) as the primary high-precision technical/mathematical/multilingual model, alongside **`nvidia/parakeet-tdt-0.6b-v2`** (via ONNX Runtime CUDA / `transcribe-rs`) as an ultra-low-latency (< 150 ms) English alternative.
3. **Hybrid Deterministic + Local LLM Rewriting Pipeline:** A two-stage text refinement layer combining a **sub-5ms deterministic normalizer** (spoken punctuation, mathematical operators, LaTeX/Unicode symbols, `snake_case`/`camelCase` code identifiers, custom glossary replacements) with a **CUDA-accelerated local `llama.cpp` server (`Qwen2.5-3B-Instruct` Q4_K_M)** using grammar-constrained JSON output (`GBNF`) and post-generation invariant checks so meaning, numbers, and equations are never altered or hallucinated.

### Why It Is the Preferred Solution

* **Genuinely $0 Forever:** Runs 100% offline on your RTX 4060 and i9-14900HX using MIT/Apache-2.0 open-source software and open-weight models. No accounts, no API keys, no word quotas, and zero telemetry.
* **Faster Than Cloud Wispr Flow:** Your RTX 4060 (3,072 CUDA cores, 256 GB/s bandwidth, 8 GB VRAM) can hold both the ASR model (~1.6 GB VRAM) and a 3B–4B quantized LLM (~2.4 GB VRAM) warm in GPU memory simultaneously—leaving ~4 GB VRAM free for your IDE and browser. End-to-end latency from key release to text insertion targets **0.65–1.10 seconds**, outperforming cloud round-trips (~1.2–2.5 seconds).
* **Superior Technical, Code, and Math Precision:** Commercial dictation tools frequently mangled ML/data science jargon, variable names, and mathematical notation. Custom `initial_prompt` vocabulary biasing, per-application context profiles (Cursor/VS Code vs. ChatGPT/Claude vs. Overleaf vs. Slack), and dedicated math/code modes solve this directly.
* **Zero Silent Data Loss:** Every captured utterance is persisted to a local SQLite database (`WAL` mode) *before* rewriting or pasting is attempted. If focus shifts, an elevated administrator window blocks `SendInput`, or the LLM times out, your text is preserved on the clipboard and retrievable with one hotkey.

### Expected Capabilities and Realistic Limitations

* **Capabilities:** System-wide dictation into browsers, Electron apps (Cursor, VS Code, Slack, Discord, Notion), native Win32/UWP apps, and terminals; Push-to-Talk, Toggle, and Hands-Free VAD modes; 6 rewrite modes (`Verbatim`, `Clean`, `Concise`, `Professional`, `Technical`, `Math/LaTeX`); spoken self-correction handling; personal dictionary; and full latency/WER telemetry.
* **Verified OS Limitations:** Windows **User Interface Privilege Isolation (UIPI)** blocks standard-privilege processes from sending synthetic keystrokes (`SendInput`) into elevated Administrator windows (e.g., an Admin PowerShell or elevated IDE). VoxFlow handles this explicitly by detecting UIPI elevation mismatches, keeping the text safely on the clipboard, and alerting you via a non-intrusive toast rather than failing silently.

---

## 2. Wispr Flow Analysis and Competitive Landscape

### What Makes Wispr Flow Compelling (and Where It Falls Short)

Public product documentation, user reviews, and architectural teardowns reveal why Wispr Flow gained rapid adoption—and why power users outgrow it:

| Dimension | Wispr Flow Behavior | Strengths to Reproduce | Weaknesses to Improve Upon |
| --- | --- | --- | --- |
| **Trigger & UX** | Hold `Fn` (macOS) or custom hotkey (Windows) for Push-to-Talk; double-tap for hands-free; minimal floating pill bar at screen bottom. | Immediate (< 50 ms) visual confirmation; zero window focus stealing; effortless single-key ergonomics. | Aggressive free-tier word cap (2,000 words/week); $12–$15/mo subscription; requires always-on account login. |
| **Speech Recognition** | Cloud-hosted Whisper-derived ASR pipeline with custom acoustic tuning. | High accuracy on natural conversational English and mid-sentence pauses. | Audio leaves your machine to cloud servers; unusable offline; noticeable latency spikes under network load. |
| **Intelligent Edits** | LLM post-processing removes fillers ("um", "uh"), resolves self-corrections ("meet at 2—actually 3 PM"), and formats lists. | Transforms rambling spoken thoughts into structured written prose automatically. | Over-edits technical prose; occasionally alters subtle phrasing; weak support for formal math/LaTeX notation. |
| **Context Awareness** | Reads active application and surrounding screen/text-box context to adjust formality and casing. | Adapts tone between email, Slack, and code editors. | Privacy risk: relies on accessibility/screen context inspection transmitted to cloud inference endpoints. |
| **Text Insertion** | Simulates paste via clipboard manipulation and restores prior clipboard contents. | Works across nearly all desktop and web inputs without custom app plugins. | Occasional clipboard clobbering or dropped pastes in high-latency web editors without transparent local history recovery. |

### Open-Source Alternatives & Reusable Ecosystem Analysis

Rather than reinventing low-level audio or inference bindings, we evaluated the strongest maintained open-source projects in this space:

1. **Handy (`cjpais/Handy`)** — *MIT License | Rust + Tauri v2 + React/TypeScript*
* **Architecture:** Uses Tauri v2 for a lightweight tray + floating overlay UI, `cpal` for cross-platform microphone capture, Silero VAD for silence filtering, `transcribe-rs` (supporting `whisper.cpp` and NVIDIA Parakeet ONNX models) for local ASR, and `enigo` / OS APIs for paste injection.
* **Lessons & Reuse Value:** Demonstrates that **Rust + Tauri v2** is the optimal desktop architecture—delivering sub-30 MB memory overhead for the UI shell, native non-activating overlays, and zero Python packaging headaches. However, Handy lacks an integrated local LLM rewrite pipeline, per-app context switching, mathematical/LaTeX formatting, UIPI elevation detection, and rich recovery/telemetry.


2. **Whispering (`braden-w/whispering`)** — *MIT License | Tauri + Rust + TypeScript (Svelte/React)*
* **Architecture:** Modular provider architecture supporting local engines (`whisper.cpp` / Speaches) alongside optional cloud endpoints, configurable global shortcuts, recording history, and transformation pipelines.
* **Lessons & Reuse Value:** Excellent UX patterns for **transformation pipelines** (chaining raw transcription -> post-processing rule/LLM -> output) and searchable local recording history with one-click re-copy.


3. **Vibe (`thewh1teagle/vibe`)** — *MIT License | Tauri + Rust + `whisper.cpp*`
* **Architecture:** Cross-platform desktop transcription tool built around `whisper-rs` with hardware-accelerated GPU builds (CUDA/Vulkan/Metal) and clean model download management.
* **Lessons & Reuse Value:** Proven patterns for **runtime hardware detection, SHA-256 verified GGUF/GGML model downloads**, and bundling precompiled CUDA libraries cleanly on Windows.


4. **WhisperWriter (`savbell/whisper-writer`) & Nerd-Dictation (`ideasman42/nerd-dictation`)** — *GPL-3.0 / MIT | Python*
* **Lessons & Reuse Value:** WhisperWriter demonstrates effective VAD-triggered continuous dictation loops, while `nerd-dictation` highlights the importance of **post-recognition deterministic regex/dictionary replacement** before touching an LLM. However, pure Python desktop daemons suffer from GIL contention, higher idle RAM (~350–600 MB), and fragile Windows global hook/focus handling compared to Rust.



### Architectural Synthesis Decision

**Do not blindly fork a single repo and inherit its constraints, and do not write inference engines from scratch.** Instead, scaffold a clean, modular **Rust + Tauri v2** application (`VoxFlow`) that directly consumes battle-tested open-source crates (`transcribe-rs` / `whisper-rs`, `cpal`, `vad-rs`, `windows-rs`, `rusqlite`) and manages a local CUDA `llama-server.exe` sidecar, adopting Handy's non-activating overlay ergonomics and Whispering's transformation/history workflows while adding the missing engineering-grade reliability, math/code intelligence, and benchmarking layers.

---

## 3. Speech Recognition and LLM Technology Evaluation

### Speech-to-Text (ASR) Engine & Model Comparison (RTX 4060 8GB Laptop)

| Model / Backend | Runtime | Disk Size | VRAM Footprint | Latency (10s Audio on RTX 4060) | Technical & Math Accuracy | Hallucination Risk | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **`whisper-large-v3-turbo` (FP16 / Q8_0)** | `whisper.cpp` (CUDA) or `faster-whisper` (CTranslate2) | ~1.6 GB (FP16) / ~870 MB (Q8_0) | ~1.6 – 2.1 GB | **~180 – 280 ms** | **Very High** — best-in-class recall on ML, statistics, Python, and rare terms; supports `initial_prompt` vocabulary conditioning. | Low when gated by Silero VAD v5. | **Primary Default Engine** |
| **`nvidia/parakeet-tdt-0.6b-v2`** | ONNX Runtime (CUDA) via `transcribe-rs` / `sherpa-onnx` | ~1.2 GB (FP16/ONNX) | ~1.2 – 1.5 GB | **~80 – 140 ms** | **High (English)** — top-tier Open ASR Leaderboard WER (~6.05% avg); strong punctuation; no `initial_prompt` biasing. | **Near-Zero** — RNN-T/TDT architecture does not loop on silence. | **Primary Ultra-Fast English Engine** |
| **`whisper-large-v3` (FP16)** | `faster-whisper` / `whisper.cpp` CUDA | ~3.1 GB | ~3.2 – 3.8 GB | ~450 – 650 ms | Highest multilingual & heavy accent accuracy. | Moderate without strict VAD + compression ratio thresholds. | Optional "Max Accuracy" preset |
| **`UsefulSensors/moonshine-base`** | ONNX Runtime (CPU/CUDA) | ~240 MB | ~350 MB | ~60 – 100 ms | Moderate — variable-length RoPE encoder is fast, but weaker on dense domain jargon. | Low. | Battery-saver / CPU-only fallback |
| **Windows Native Speech (`SAPI` / `WinRT`)** | Windows OS built-in | 0 MB | 0 MB | ~150 ms | Poor on technical terms, code identifiers, and mathematical phrasing. | Low. | Emergency zero-GPU fallback only |

### Local LLM Rewriting Engine Comparison (RTX 4060 8GB Laptop)

| Model (Quantization) | Runtime | VRAM Footprint | Decode Speed (RTX 4060) | Latency (80 Output Tokens) | Instruction Following & Meaning Preservation | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| **`Qwen2.5-3B-Instruct` (`Q4_K_M` GGUF)** | `llama.cpp` (`llama-server` CUDA) | **~2.3 GB** (with 4k context) | **~115 – 145 tok/s** | **~550 – 700 ms** | **Excellent** — superior structured formatting, mathematical/LaTeX symbol preservation, and strict adherence to system constraints. | **Primary Default Rewrite Model** |
| **`Gemma-3-4B-IT` (`Q4_K_M` GGUF)** | `llama.cpp` (`llama-server` CUDA) | **~2.8 GB** | ~95 – 120 tok/s | ~650 – 850 ms | **Excellent** — very natural prose cleanup and tone adaptation; slightly larger VRAM footprint. | **Alternative High-Style Preset** |
| **`Llama-3.2-3B-Instruct` (`Q4_K_M` GGUF)** | `llama.cpp` (`llama-server` CUDA) | ~2.2 GB | ~120 – 150 tok/s | ~530 – 680 ms | Good prose cleanup, occasionally more verbose unless constrained by GBNF grammar. | Supported compatible model |
| **`Qwen2.5-7B-Instruct` (`Q4_K_M` GGUF)** | `llama.cpp` (`llama-server` CUDA) | ~4.9 GB | ~65 – 80 tok/s | ~1.0 – 1.25 s | Outstanding reasoning & complex multi-clause restructuring; tight fit alongside `large-v3-turbo` (~6.6 GB total VRAM). | Optional "Deep Rewrite" preset (AC power) |
| **Deterministic Hybrid Pipeline (Regex + SymSpell + Rules)** | Native Rust (`< 5 ms`) | 0 MB | N/A | **< 5 ms** | 100% deterministic; zero hallucination risk; handles punctuation, math symbols, casing, and custom dictionary. | **Always-On Pre/Post Layer + Fast-Path** |

### VRAM Co-Residency Budget on Your RTX 4060 (8,192 MB Total)

* **Windows 11 Desktop Window Manager + Browser/IDE baseline:** ~1.2 – 1.8 GB
* **ASR Engine (`whisper-large-v3-turbo` FP16 or `parakeet-tdt-0.6b-v2`):** ~1.4 – 1.8 GB
* **Rewrite LLM (`Qwen2.5-3B-Instruct` Q4_K_M + 4,096 KV cache):** ~2.3 GB
* **Silero VAD v5 + Audio Buffers:** < 15 MB
* **Total Peak Allocation:** **~4.9 – 5.9 GB / 8.0 GB** — leaving **> 2.0 GB safety margin** to prevent out-of-memory spills into slow shared system RAM.

### Local vs. Free-Tier Cloud Trade-Offs

* **Optional Free-Tier Cloud Plugins (Strictly Disabled by Default):** Providers such as **Groq** (free developer tier offering `whisper-large-v3-turbo` and Llama models with strict RPM/TPM rate limits) and **Google AI Studio** (`gemini-2.5-flash` free tier with per-minute rate limits and potential data-logging terms on free tiers) exist, **but should NOT be relied upon as core dependencies**:
1. Free cloud tiers impose strict requests-per-minute (RPM) and audio-seconds-per-hour caps that break heavy daily dictation.
2. Dictating proprietary code, research notes, passwords, or private messages to third-party free APIs violates local-first privacy.
3. Your RTX 4060 runs `parakeet-tdt-0.6b` + `Qwen2.5-3B` locally in **< 0.85s** with zero network jitter and zero quota limits. Cloud providers should exist only as an optional user-configured plugin interface.



---

## 4. Complete Feature Inventory and Prioritized PRD

### Product Vision & Primary Workflows

* **Vision:** Make speaking a faster, cleaner, and more reliable input method than typing across every Windows workflow—without subscriptions, cloud lock-in, or silent failures.
* **Primary Workflows:**
1. **Rapid Prompting & Coding (Cursor, VS Code, Terminal, ChatGPT/Claude):** Hold hotkey -> speak technical instructions with variable names and architecture terms -> release -> clean, properly cased text appears at cursor in < 1 second.
2. **Mathematical & Data Science Dictation (Overleaf, Jupyter, Notion, Markdown):** Speak equations naturally ("sigma from i equals 1 to n of x sub i") -> automatically formatted as LaTeX (`\sum_{i=1}^{n} x_i`) or Unicode (`∑_{i=1}^n x_i`) based on active app/mode.
3. **Long-Form Writing & Email (Gmail, Docs, Word):** Toggle hands-free dictation -> speak multi-paragraph thoughts with pauses and verbal self-corrections -> structured, grammatical paragraphs inserted cleanly.
4. **Zero-Loss Recovery:** If pasting fails or focus was lost, press the recovery hotkey (`Ctrl+Shift+V` / tray click) to immediately insert or copy the last raw or cleaned transcription.



### Prioritized Requirements Matrix

| ID | Priority | Feature | Rationale | Acceptance Criteria |
| --- | --- | --- | --- | --- |
| **FR-01** | **P0** | **Global Low-Level Hotkeys (PTT & Toggle)** | Core trigger must work globally regardless of focused app without leaking stray keystrokes. | Hold-to-talk (e.g., `Right Alt` or `Ctrl+Win`) starts capture in < 40 ms; tap-to-toggle locks recording; `Esc` aborts cleanly without pasting. |
| **FR-02** | **P0** | **Immediate Non-Activating Recording Overlay** | Eliminates silent failure anxiety while keeping cursor focus intact in the target app. | Floating pill appears within 50 ms with live microphone RMS waveform, timer, and mode badge; `GetForegroundWindow()` does not change. |
| **FR-03** | **P0** | **Resilient WASAPI Audio Capture + Silero VAD** | Prevents empty recordings, dead-mic sessions, and Whisper silence hallucinations. | Detects missing/muted mic immediately; warns user if RMS stays below noise floor for > 2s; trims leading/trailing silence via Silero VAD v5. |
| **FR-04** | **P0** | **CUDA Local Speech Recognition** | Fast, accurate, $0 offline transcription on RTX 4060. | Transcribes 10s speech in < 300 ms (`whisper-large-v3-turbo` or `parakeet-tdt-0.6b-v2`) with automatic CPU fallback if GPU is busy/unavailable. |
| **FR-05** | **P0** | **Atomic Clipboard & `SendInput` Text Insertion** | Reliable insertion across browsers, Electron IDEs, and native Windows apps without clobbering clipboard. | Backs up existing clipboard formats, writes `CF_UNICODETEXT`, triggers `Ctrl+V` (or `Shift+Insert` in terminals), and restores original clipboard after 200 ms. |
| **FR-06** | **P0** | **Zero-Loss SQLite Persistence & Fallback Chain** | Dictated speech must never vanish if LLM or paste fails. | Every utterance is written to local SQLite before injection; if LLM fails, raw ASR text is inserted; if paste fails, text remains on clipboard + toast notification. |
| **FR-07** | **P0** | **Intelligent Hybrid Text Cleanup (`Verbatim`, `Clean`, `Concise`)** | Converts messy spoken drafts into polished text without altering meaning. | Removes fillers ("um", "uh"), fixes punctuation/casing, resolves self-corrections, and passes post-LLM number/entity preservation checks. |
| **FR-08** | **P1** | **Technical & Mathematical Dictation (`Technical`, `Math/LaTeX`)** | Essential for data science, ML, programming, and academic workflows. | Converts spoken math to valid LaTeX or Unicode (`x²`, `\sum`, `θᵀx + b`), preserves `snake_case`/`camelCase`, and injects custom glossary terms into ASR `initial_prompt`. |
| **FR-09** | **P1** | **Privacy-Safe Active-App Context Switching** | Automatically adapts formatting between code editors, browsers, and chat apps without screen scraping. | Reads only foreground executable name & window title (zero OCR/screen capture) to auto-select mode (e.g., `Cursor.exe` -> `Technical`, `Code.exe` -> `Technical`). |
| **FR-10** | **P1** | **Personal Dictionary & Spoken Snippets** | Fixes recurring proper nouns, project names (`Interlace`, `AtlasMind`, `PyTorch`), and expands boilerplate snippets. | User-editable JSON/SQLite dictionary applied both as ASR vocabulary bias and deterministic post-replacement. |
| **FR-11** | **P1** | **Searchable History & Re-Paste / Undo Last** | Enables instant recovery, re-formatting in another mode, or undoing an unwanted insertion. | Tray/hotkey opens lightweight history drawer showing timestamp, target app, raw vs. cleaned text, latency metrics, and 1-click Copy/Re-run. |
| **FR-12** | **P1** | **First-Run Hardware & Model Setup Wizard** | Ensures smooth setup without manual CLI dependency hunting. | Detects RTX 4060 CUDA capability, downloads verified GGUF/ONNX weights with SHA-256 checks, and verifies microphone permissions. |
| **FR-13** | **P2** | **Voice-Selected Text Transformation ("Edit Selection")** | Highlight existing text, hold secondary hotkey, speak an instruction ("make this more concise", "convert to docstring"). | Reads selected text via safe `Ctrl+C`, passes `(selected_text, spoken_command)` to local LLM, and replaces selection safely. |
| **FR-14** | **P2** | **Power-Profile Adaptive VRAM Management** | Saves battery when unplugged while maximizing speed on AC power. | Keeps models pinned in VRAM on AC power; optionally unloads LLM after 5 min idle on battery power. |
| **FR-15** | **P2** | **Optional Free-Tier Cloud Plugin Interface** | Allows optional external API use (e.g., Groq/Gemini free tier) if user explicitly opts in. | Disabled by default; strict opt-in with API key stored in Windows Credential Manager (`CredWriteW`) and automatic local fallback on rate limit. |

### Explicit Exclusions

* **No Paid Cloud Dependencies:** No proprietary API keys required for core functionality.
* **No Screen-Content OCR / Screenshot Scraping:** Context awareness uses process metadata and explicit user rules only—never background screen capture.
* **No Always-On Wake-Word Listening by Default:** Microphone hardware stream opens *only* when the user triggers a hotkey, ensuring zero background acoustic surveillance and zero idle CPU drain.

---

## 5. Architecture Alternatives and Decision Analysis

### Viable End-to-End Architectural Candidates

1. **Candidate A (Recommended): Native Rust + Tauri v2 Core with Local CUDA Sidecar/FFI (`whisper.cpp` / `Parakeet` + `llama-server.exe`)**
* **How it works:** A compiled Rust binary manages Windows global keyboard hooks, WASAPI audio capture, Silero VAD, Win32 foreground window inspection, clipboard/paste injection, and SQLite state, paired with a lightweight Tauri v2 WebView for the floating pill and settings UI. ASR runs via native Rust FFI (`transcribe-rs` / `whisper-rs` with CUDA) and LLM rewriting runs via a managed local `llama-server.exe` (CUDA) bound to `127.0.0.1` with GBNF grammar enforcement.
* **Pros:** Lowest possible latency (< 40 ms UI/hook overhead); tiny idle footprint (~25 MB shell + resident GPU models); zero Python runtime fragility; strict memory safety; clean separation between audio/OS real-time threads and GPU inference.
* **Cons:** Requires MSVC + Rust toolchain + CUDA SDK/runtime during build phase (handled autonomously by the coding agent).


2. **Candidate B: Pure Python Desktop Daemon (`faster-whisper` + `pynput` + `PySide6` + Ollama)**
* **How it works:** All-Python application using `sounddevice`, `faster-whisper`, `pynput` for hotkeys, `pyperclip`/`pyautogui` for pasting, and local Ollama HTTP API.
* **Pros:** Fastest initial prototyping; mature `faster-whisper` Python ecosystem.
* **Cons:** Noticeably higher idle RAM (~400–700 MB); sluggish startup; `pynput` hooks on Windows frequently drop events or conflict with modifiers under heavy CPU load; `pyautogui` clipboard handling is primitive and prone to race conditions.


3. **Candidate C: Direct Fork of `cjpais/Handy` with Minor Edits**
* **How it works:** Clone Handy and patch in an Ollama HTTP call after `transcribe-rs`.
* **Pros:** Working Tauri v2 + `transcribe-rs` scaffold out of the box.
* **Cons:** Handy's internal state machine and UI were not designed around multi-mode LLM rewriting, mathematical/LaTeX post-processing, per-app context rules, SQLite recovery guarantees, or automated WER/latency benchmarking. Refactoring around upstream assumptions creates unnecessary friction.


4. **Candidate D: Local ASR + Cloud Free-Tier LLM (Groq / Gemini Free API)**
* **How it works:** Transcribe locally, send text to Groq or Google AI Studio free tier for cleanup.
* **Pros:** Access to 70B-class cloud LLMs without local VRAM usage.
* **Cons:** Violates offline reliability, exposes sensitive dictated text to external servers, introduces 400–1,500 ms network latency variance, and hits free-tier RPM/daily quotas during heavy coding/writing sessions.



### Decision Matrix

| Criterion | Candidate A (Rust + Tauri v2 + Local CUDA) | Candidate B (Pure Python + Ollama) | Candidate C (Direct Handy Fork) | Candidate D (Local ASR + Cloud Free API) |
| --- | --- | --- | --- | --- |
| **End-to-End Latency** | **Best (~0.65–1.05s)** | Moderate (~1.1–1.8s) | Fast (~0.7–1.2s) | Variable (~1.2–2.5s) |
| **OS Hook & Paste Reliability** | **Best (Native Win32 APIs)** | Fair (`pynput` quirks) | Good | Good |
| **Intelligent Math/Code Rewriting** | **Best (Hybrid + GBNF LLM)** | Good | Basic (needs heavy rework) | High (but privacy/quota risk) |
| **Resource Efficiency (RAM/CPU)** | **Best (~25 MB host shell)** | Poor (~500 MB+ Python) | Good (~30 MB) | Good |
| **Offline & $0 Sustainability** | **100% Guaranteed** | 100% Guaranteed | 100% Guaranteed | Fails offline / quota risk |
| **Maintainability & Testability** | **High (Typed Rust + Bench CLI)** | Moderate | Moderate | Moderate |

**Final Architectural Decision:** Proceed with **Candidate A**—a clean, modular **Rust + Tauri v2** desktop application (`VoxFlow`) that leverages `transcribe-rs` / `whisper-rs` (CUDA) and a managed local `llama-server.exe` (CUDA) with deterministic pre/post-processing and SQLite persistence.

---

## 6. Recommended System Architecture

### Core Components and Responsibilities

1. **Input & Hotkey Manager (`hotkey_engine` — Rust `windows-rs`):**
* Installs a dedicated high-priority thread running a Win32 `WH_KEYBOARD_LL` low-level keyboard hook.
* Distinguishes **Hold (> 250 ms)** for Push-to-Talk, **Quick Tap (< 250 ms)** for Hands-Free Toggle, and **`Escape`** for immediate cancellation.
* Consumes only the trigger key events when active so stray characters never leak into the active document.


2. **Audio Capture & VAD Pipeline (`audio_engine` — Rust `cpal` + `vad-rs`):**
* Captures 16 kHz, 16-bit/f32 mono PCM audio via Windows WASAPI shared mode with lock-free ring buffer (`ringbuf`).
* Computes real-time RMS energy (throttled to 30 FPS) for the floating pill waveform and runs **Silero VAD v5** frame-by-frame (32 ms windows) to detect speech onset, trim leading/trailing silence, and trigger auto-stop in Hands-Free mode after configurable trailing silence (default `650 ms`).
* Monitors stream health: detects disconnected devices, zero-amplitude muted inputs, and buffer overruns immediately.


3. **Context Inspector (`context_engine` — Rust `windows-rs`):**
* On hotkey press, captures `HWND` via `GetForegroundWindow()`, resolves the executable path (`QueryFullProcessImageNameW`), window title (`GetWindowTextW`), and checks UIPI elevation (`OpenProcessToken` + `GetTokenInformation(TokenElevation)`).
* Maps the executable/title to a user-configurable **Context Profile** (e.g., `Cursor.exe` / `Code.exe` -> `Technical`, `WindowsTerminal.exe` -> `Technical + Shift+Insert`, `Overleaf` / `.tex` -> `Math/LaTeX`, `Gmail` / `Outlook` -> `Professional`).


4. **Speech Recognition Engine (`asr_engine` — Rust FFI via `transcribe-rs` / `whisper-rs`):**
* Keeps the active ASR model (`whisper-large-v3-turbo` FP16/Q8_0 or `parakeet-tdt-0.6b-v2` ONNX CUDA) warm in RTX 4060 VRAM.
* Passes the user's **Personal Dictionary** and domain terminology into Whisper's `initial_prompt` parameter to bias decoding toward exact spellings (`PyTorch`, `FastAPI`, `LangGraph`, `eigenvalue`, `heteroscedasticity`).
* Filters out known Whisper hallucination artifacts (empty/whitespace outputs, repetitive n-gram loops, "Thank you for watching" silence hallucinations when VAD speech ratio < 5%).


5. **Hybrid Text Refinement Engine (`rewrite_engine` — Rust + `llama-server.exe`):**
* **Stage 1 — Deterministic Pre-Normalizer (< 5 ms):** Applies custom dictionary replacements, expands spoken punctuation commands if enabled ("new line", "open paren"), and normalizes unambiguous mathematical/programming patterns.
* **Stage 2 — Fast-Path Bypass:** If mode is `Verbatim`, or utterance has <= 3 words with no filler words, skips the LLM completely for near-instant insertion.
* **Stage 3 — Constrained Local LLM (`Qwen2.5-3B-Instruct` Q4_K_M):** Sends system prompt tailored to the active mode (`Clean`, `Concise`, `Professional`, `Technical`, `Math/LaTeX`) to the local `llama-server` (`[http://127.0.0.1:18080/completion](http://127.0.0.1:18080/completion)`) using a **GBNF grammar** requiring JSON output `{"cleaned_text": "..."}` at `temperature: 0.1`.
* **Stage 4 — Post-LLM Invariant Guard (< 2 ms):** Verifies that all numeric literals, mathematical operators, and backticked/camelCase/snake_case code identifiers present in the raw transcript remain intact in `cleaned_text`. If an invariant is violated or the LLM exceeds its 2.5s timeout, automatically falls back to Stage 1 output.


6. **Atomic Clipboard & Text Injector (`injection_engine` — Rust `windows-rs`):**
* Acquires the Windows clipboard (`OpenClipboard`), snapshots existing clipboard formats (`CF_UNICODETEXT`, `CF_HDROP`, etc.) into memory, writes `cleaned_text` as `CF_UNICODETEXT`, and releases the clipboard.
* Verifies `GetForegroundWindow()` still matches the target window (or pastes to current focus if user intentionally clicked into a target box), then dispatches `SendInput` (`Ctrl+V`, or `Ctrl+Shift+V` / `Shift+Insert` for configured terminal emulators).
* Spawns a non-blocking 220 ms timer to restore the user's prior clipboard snapshot cleanly, checking `GetClipboardSequenceNumber()` so it never overwrites anything the user copied afterward.


7. **Persistence & Recovery Store (`storage_engine` — SQLite WAL via `rusqlite`):**
* Logs every dictation event (`id`, `created_at`, `app_exe`, `mode`, `raw_text`, `final_text`, `audio_duration_ms`, `asr_ms`, `llm_ms`, `inject_ms`, `status`, `error_code`) in `%LOCALAPPDATA%\VoxFlow\voxflow.db`.
* Raw audio buffers exist in volatile RAM only and are zeroed/dropped immediately after ASR inference (optional opt-in audio saving for debugging/benchmarking only).


8. **Tauri v2 UI Shell (`ui_shell` — React + TypeScript + Tailwind):**
* **Floating Pill Overlay:** Frameless, transparent, click-through (`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`), bottom-center status pill showing real-time audio bars, active mode pill, processing spinner, and recoverable error/warning badges.
* **Settings & History Window:** Opened from system tray or hotkey; provides model manager, hotkey customizer, custom dictionary editor, per-app rules, latency dashboard, and searchable dictation history.



### End-to-End Lifecycle & State Machine

* **State Transitions:**
1. `IDLE` -> *(Hotkey Down)* -> `RECORDING` (Overlay shown, WASAPI stream active, context captured)
2. `RECORDING` -> *(Esc Pressed)* -> `IDLE` (Audio buffer discarded, overlay hidden)
3. `RECORDING` -> *(Hotkey Up in PTT / VAD Silence in Toggle)* -> `TRANSCRIBING` (Overlay switches to compact spinner; VAD-trimmed PCM sent to GPU ASR)
4. `TRANSCRIBING` -> *(No speech / RMS below threshold)* -> `IDLE` (Subtle "No speech detected" pill flash for 900 ms)
5. `TRANSCRIBING` -> *(ASR Text Ready)* -> Persist raw text to SQLite -> `REWRITING` (Deterministic normalizer -> optional local LLM -> invariant check)
6. `REWRITING` -> *(Final Text Ready)* -> Persist final text to SQLite -> `INJECTING` (Backup clipboard -> set `CF_UNICODETEXT` -> `SendInput` paste -> schedule clipboard restore)
7. `INJECTING` -> *(Success)* -> `IDLE` (Overlay fades out after 300 ms)
8. `INJECTING` -> *(UIPI Blocked / Focus Lost)* -> `RECOVERABLE_HOLD` (Text stays on clipboard; overlay shows `"Copied — Press Ctrl+V (Admin window)"` for 3 seconds -> `IDLE`)



---

## 7. User Experience and Interaction Design

### Ergonomic Hotkey Scheme (Configurable)

* **Primary Dictation Key (`Right Alt` or `Ctrl + Space`):**
* **Hold (> 250 ms):** Push-to-Talk — records while held, transcribes and pastes immediately on release.
* **Quick Tap (< 250 ms):** Hands-Free Toggle — locks recording on until you tap again or pause past the configurable VAD silence threshold (`1.2s` default in hands-free mode).


* **Abort Key (`Escape` while recording/processing):** Immediately cancels the current utterance and restores `IDLE` without modifying the clipboard.
* **Quick Mode Cycle (`Ctrl + Alt + M`):** Cycles through active modes (`Clean` -> `Technical` -> `Math/LaTeX` -> `Concise` -> `Verbatim`) with a 1-second pill badge update.
* **Instant Recovery / Re-Paste (`Ctrl + Alt + V`):** Pastes the most recent successful transcription directly from SQLite history (useful if an app missed the initial paste or you want to paste the same block twice).

### Floating Pill Overlay Design Principles

* **Zero Focus Theft:** Configured at the Win32 window-style level with `WS_EX_NOACTIVATE` and `WS_EX_TOOLWINDOW` so clicking or showing the pill *never* blurs the cursor in Cursor, VS Code, Chrome, or Word.
* **Compact Visual Footprint:** 180 px × 36 px rounded dark/light glassmorphic pill positioned 48 px above the Windows taskbar (user-draggable, remembers position per monitor).
* **Clear Visual States:**
* **Listening:** Pulsing emerald/cyan mic dot + 12-bar live RMS audio equalizer + compact mode tag (`CLEAN`, `TECH`, `MATH`).
* **Muted / Dead Mic Warning:** Amber dot + `"No mic input detected"` within 1.5 seconds if audio amplitude is strictly zero.
* **Processing:** Smooth indeterminate accent ring (`~0.5–0.9s` duration).
* **Clipboard Fallback Notice:** Subtle blue badge `"Copied to clipboard (Admin app)"` when Windows UIPI prevents auto-paste.



---

## 8. Performance and Reliability Strategy

### Measurable Latency & Resource Targets (Lenovo Legion 5i: i9-14900HX + RTX 4060 8GB)

| Pipeline Stage | Target P50 Latency | Target P95 Latency | Optimization Mechanism |
| --- | --- | --- | --- |
| **Hotkey Press -> Mic Active & Overlay Visible** | `< 25 ms` | `< 45 ms` | Persistent WASAPI audio device handle in standby + pre-created hidden Tauri overlay window. |
| **ASR Inference (5s utterance)** | `~110 ms` (Parakeet) / `~180 ms` (Whisper Turbo) | `< 320 ms` | Models pinned in RTX 4060 VRAM (`FP16` / `Q8_0`), FlashAttention enabled, CUDA graphs warm-started at launch. |
| **Deterministic Normalizer + Invariant Guard** | `< 3 ms` | `< 8 ms` | Compiled Rust regex sets (`regex::RegexSet`) + Aho-Corasick dictionary matcher. |
| **Local LLM Rewrite (`Qwen2.5-3B` Q4_K_M, ~60 tok)** | `~450 ms` | `< 750 ms` | `llama-server.exe` with `-ngl 99` (all layers on RTX 4060), GBNF JSON grammar, KV cache reuse for static system prompt prefix. |
| **Clipboard Backup + `SendInput` Injection** | `< 35 ms` | `< 60 ms` | Direct Win32 `OpenClipboard` + `SendInput` batch array (`[Ctrl Down, V Down, V Up, Ctrl Up]`). |
| **Total End-to-End (Key Release -> Text in App)** | **`~620 – 720 ms`** | **`< 1,150 ms`** | Pipelined execution; fast-path bypass skips LLM for short/verbatim utterances (`~220 ms` total). |

### Mathematical & Technical Dictation Architecture

Mathematical speech is inherently ambiguous without context (e.g., *"x squared plus y squared"* could be plain Unicode `x² + y²` in Slack or LaTeX `$x^2 + y^2$` in Overleaf/Markdown). VoxFlow resolves this cleanly without guessing blindly:

1. **Explicit Output Format Selection:**
* **`Technical` Mode (Default for IDEs, ChatGPT/Claude, Docs):** Uses clean Unicode math (`x² + y² = z²`, `θᵀx + b`, `α`, `β`, `∇`, `≤`, `≠`, `→`) and preserves programming identifiers (`snake_case`, `camelCase`, `PascalCase`, CLI flags like `--quantize`).
* **`Math / LaTeX` Mode (Auto-selected in Overleaf/`.tex` editors or via `Ctrl+Alt+M`):** Converts spoken expressions into valid LaTeX (`\frac{df}{dx}`, `\sum_{i=1}^{n} x_i`, `P(Y = 1 \mid X)`), wrapping inline expressions in `$ ... $` when embedded inside prose sentences.


2. **Two-Layer Math Translation:**
* **Layer 1 (Deterministic Canonicalizer):** Converts unambiguous spoken tokens (`"alpha"`, `"theta transpose"`, `"squared"`, `"cubed"`, `"sub i"`, `"greater than or equal to"`) into canonical symbolic tokens before LLM evaluation.
* **Layer 2 (Constrained LLM Formatter):** Assembles complex nested expressions (integrals, summations, conditional probabilities, matrix dimensions) under a strict system instruction: *Never alter variable names, exponents, subscripts, or numeric constants; if an expression is ambiguous, preserve the literal mathematical phrasing rather than inventing symbols.*



### 5-Tier Failure Recovery Hierarchy (Zero Silent Loss)

1. **Tier 1 (Normal Path):** ASR -> Hybrid Rewrite -> Auto-Paste -> Delayed Clipboard Restore.
2. **Tier 2 (LLM Timeout / Invariant Violation):** If `llama-server` takes > 2.5s, crashes, or alters a number/identifier, immediately insert the **Stage 1 deterministic-cleaned ASR text** and log a warning.
3. **Tier 3 (GPU / CUDA OOM Fallback):** If RTX 4060 VRAM is exhausted (e.g., while running heavy local PyTorch training), automatically reload ASR on i9-14900HX CPU (`int8` quantized, ~24 threads) and run deterministic-only cleanup without crashing.
4. **Tier 4 (UIPI Elevation / No Editable Focus):** If foreground window is elevated (`Admin`) or rejects paste, keep text on the system clipboard without restoring the old clipboard and display an actionable pill toast (`"Copied to clipboard — paste manually"`).
5. **Tier 5 (Crash / Power Loss Recovery):** Because SQLite WAL commits occur immediately after ASR and after Rewrite, restarting VoxFlow or pressing `Ctrl+Alt+V` recovers the exact text even after an unexpected crash.

---

## 9. Detailed Autonomous Implementation Plan

The coding agent should execute the following phases sequentially, verifying each milestone before proceeding.

### Phase 0: Environment & Hardware Discovery (Mandatory First Step)

* **Goal:** Inspect the actual Windows machine, CUDA drivers, build toolchains, and audio hardware before writing configuration or build scripts.
* **Agent Tasks:**
1. Run system inspection commands in PowerShell:
* OS & Build: `Get-ComputerInfo | Select-Object WindowsProductName, WindowsVersion, OsBuildNumber`
* GPU & CUDA: `nvidia-smi` and `nvcc --version` (verify RTX 4060 driver version and CUDA 12.x availability)
* Build Toolchains: `rustc --version`, `cargo --version`, `node --version`, `pnpm --version` (or `npm`), `cmake --version`, and check Visual Studio C++ Build Tools (`where.exe cl.exe`).
* Existing Local AI Runtimes: Check if `ollama` or `llama-server.exe` is already installed (`Get-Command ollama, llama-server -ErrorAction SilentlyContinue`).
* Audio Devices: Query available Windows recording endpoints via PowerShell/Rust `cpal` enumeration.


2. Install any missing prerequisites via `winget` (e.g., Rustup, LLVM/Clang for bindgen, CMake, Node.js LTS) only if absent.


* **Completion Criteria:** Verified JSON/Markdown environment report confirming CUDA capability, C++ linker availability, and active default microphone device name.

### Phase 1: Project Scaffolding & Core Desktop Shell

* **Goal:** Create a clean Tauri v2 + Rust + React/TypeScript workspace with system tray, settings window, and a non-activating floating pill overlay.
* **Dependencies:** `tauri` (v2), `tauri-plugin-autostart`, `serde`, `serde_json`, `tokio`, `tracing`, `tracing-appender`, `rusqlite` (with `bundled` feature).
* **Architectural Decisions:**
* Configure two Tauri windows in `tauri.conf.json`: `overlay` (frameless, transparent, always-on-top, hidden on startup, `focusable: false`) and `main` (settings/history dashboard, hidden until opened from tray).
* Apply Win32 extended styles `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` on the `overlay` HWND via `windows-rs` so showing/updating the pill never steals focus.


* **Verification:** Launch app -> verify tray icon works -> trigger test overlay show/hide while typing in Notepad -> confirm Notepad never loses cursor focus.

### Phase 2: WASAPI Audio Capture, Silero VAD v5 & Dead-Mic Detection

* **Goal:** Implement low-latency, resilient microphone capture with real-time RMS metering and voice activity detection.
* **Dependencies:** `cpal`, `ringbuf`, `rubato` (if resampling from 48 kHz Windows default to 16 kHz mono is needed), `ort` (ONNX Runtime) or `vad-rs` with Silero VAD v5 ONNX weights.
* **Key Tasks:**
1. Build a dedicated audio capture manager that opens the selected WASAPI input device, resamples to 16,000 Hz mono `f32`, and writes into a bounded lock-free ring buffer (max 10 minutes cap to prevent runaway memory growth).
2. Compute 30 Hz RMS amplitude events emitted to the overlay UI for the live waveform.
3. Run Silero VAD v5 on 512-sample (32 ms) frames to strip leading/trailing silence and drive hands-free endpointing.
4. Implement immediate health checks: missing device error, zero-amplitude warning after 1.5s, and automatic stream recovery on device change.


* **Verification:** Unit tests with synthetic sine/silence WAV buffers + live microphone test verifying waveform animation, silence trimming, and muted-mic warning.

### Phase 3: Local CUDA Speech Recognition Integration

* **Goal:** Integrate warm, GPU-accelerated local ASR with custom vocabulary conditioning and hallucination filtering.
* **Dependencies:** `transcribe-rs` (with `whisper` + `parakeet` features enabled) or `whisper-rs` (compiled with CUDA feature flag), SHA-256 model downloader (`reqwest` + `sha2`).
* **Key Tasks:**
1. Implement an automated model manager storing verified models in `%LOCALAPPDATA%\VoxFlow\models\`:
* Default ASR: `ggml-large-v3-turbo-q8_0.bin` (or `FP16`) + optional `parakeet-tdt-0.6b-v2` ONNX bundle.
* VAD: `silero_vad_v5.onnx`.


2. Keep the selected ASR context warm in RTX 4060 VRAM at startup (with configurable idle unload timer when on battery).
3. Inject the user's Personal Dictionary words into Whisper's `initial_prompt`.
4. Add post-ASR guards to reject empty/punctuation-only outputs or Whisper repetition loops.


* **Verification:** Benchmark transcription latency and accuracy on 5 sample technical/math/conversational WAV files; confirm GPU utilization via `nvidia-smi` and sub-300ms inference on 10s clips.

### Phase 4: Global Low-Level Keyboard Hooks & Recording State Machine

* **Goal:** Implement deterministic global hotkeys supporting Push-to-Talk, Toggle, Cancel, and Re-Paste.
* **Dependencies:** `windows-rs` (`Win32::UI::WindowsAndMessaging::SetWindowsHookExW`, `WH_KEYBOARD_LL`).
* **Key Tasks:**
1. Run `WH_KEYBOARD_LL` on a dedicated OS thread with a Windows message pump (`GetMessageW`) to guarantee < 5 ms hook latency and prevent Windows hook timeout eviction.
2. Implement state machine (`IDLE`, `RECORDING`, `TRANSCRIBING`, `REWRITING`, `INJECTING`, `RECOVERABLE_HOLD`) with strict mutex/atomic guards preventing overlapping recordings or duplicate triggers.
3. Support hold-duration thresholding (`250 ms`) to seamlessly differentiate Push-to-Talk from Hands-Free Toggle on the same hotkey, plus `Esc` cancellation.


* **Verification:** Test rapid key taps, long holds, `Esc` aborts mid-recording, and hotkey triggering while focused inside Cursor, Chrome, and Windows Terminal.

### Phase 5: Active-Window Context Detection & Atomic Clipboard/Paste Injection

* **Goal:** Deliver rock-solid text insertion across Windows applications with clipboard preservation and UIPI safety.
* **Dependencies:** `windows-rs` (`DataExchange`, `KeyboardAndMouse`, `Threading`, `Security`).
* **Key Tasks:**
1. Capture target `HWND` at recording start, extract executable name (`Cursor.exe`, `Code.exe`, `chrome.exe`, `WindowsTerminal.exe`, etc.), and check process integrity/elevation level.
2. Implement `ClipboardGuard`: snapshot existing clipboard formats, set `CF_UNICODETEXT` with retry loop (handling transient clipboard locks from other apps up to 5 retries × 10 ms), send `Ctrl+V` (or `Ctrl+Shift+V` / `Shift+Insert` for terminal apps) via `SendInput`, and restore the original clipboard after 220 ms if `GetClipboardSequenceNumber()` hasn't changed externally.
3. Detect elevated target windows (UIPI boundary) -> skip `SendInput`, retain text on clipboard, and trigger the `"Copied to clipboard (Admin app)"` overlay state.


* **Verification:** Automated test verifying clipboard image/text restoration after injection + manual insertion tests across Chrome (ChatGPT), Cursor, Notepad, and elevated Admin PowerShell.

### Phase 6: Hybrid Deterministic + Local LLM Rewriting Engine

* **Goal:** Implement fast, meaning-preserving cleanup, technical formatting, and mathematical/LaTeX conversion.
* **Dependencies:** Prebuilt Windows CUDA binary of `llama-server.exe` (from official `ggerganov/llama.cpp` releases) + `Qwen2.5-3B-Instruct-Q4_K_M.gguf` + `reqwest`.
* **Key Tasks:**
1. Manage `llama-server.exe` as a supervised child process bound strictly to `127.0.0.1` (`--ctx-size 4096 -ngl 99 --port 18080`), with health checks and automatic restart on crash.
2. Build the Rust **Deterministic Normalizer** (spoken punctuation, Unicode/LaTeX math symbols, custom dictionary replacements, `snake_case`/`camelCase` helpers).
3. Implement the 6 mode prompts (`Verbatim`, `Clean`, `Concise`, `Professional`, `Technical`, `Math/LaTeX`) constrained by a GBNF JSON grammar so the LLM outputs *only* `{"text": "..."}` with zero conversational preamble.
4. Build the **Post-LLM Invariant Guard** comparing extracted numbers, code tokens, and key entities between raw and rewritten text—automatically falling back to deterministic output if an invariant breaks.


* **Verification:** Run unit test suite of 30+ messy/technical/mathematical utterances verifying filler removal, self-correction resolution, LaTeX formatting, and zero numeric/identifier alteration.

### Phase 7: Settings Dashboard, Personal Dictionary, Per-App Rules & History UI

* **Goal:** Build a clean, responsive Tauri/React interface for configuration, recovery, and observability.
* **Key Tasks:**
1. **General & Hotkeys Tab:** Microphone selector with live input level meter, Push-to-Talk / Toggle hotkey recorder, launch-on-startup toggle, theme (Dark/Light/System).
2. **Models & Performance Tab:** One-click download/switch between ASR models (`Whisper Large-v3-Turbo`, `Parakeet TDT 0.6B`) and LLM models (`Qwen2.5-3B`, `Gemma-3-4B`, `None/Deterministic-Only`), live VRAM usage readout, and battery behavior settings.
3. **Modes, Dictionary & App Rules Tab:** Edit custom vocabulary/replacements, customize per-mode prompts, and map `.exe` names (e.g., `Cursor.exe` -> `Technical`) to automatic modes.
4. **History & Diagnostics Tab:** Searchable SQLite dictation log with 1-click Copy, Re-paste, Re-process in another mode, and per-stage latency telemetry (`ASR ms`, `LLM ms`, `Total ms`).


* **Verification:** End-to-end UI walkthrough confirming every setting persists across app restarts and immediately updates runtime behavior.

### Phase 8: Benchmarking, Failure Injection & Release Packaging

* **Goal:** Validate production reliability, measure real hardware performance on the Lenovo Legion 5i, and package an installable Windows app.
* **Key Tasks:**
1. Execute the automated benchmark & failure-injection suite (Section 10).
2. Build release binary (`cargo tauri build`) producing an NSIS/MSI Windows installer and portable `.exe` in `src-tauri/target/release/`.
3. Configure optional Windows startup shortcut and write concise user documentation (`README.md` + `TROUBLESHOOTING.md`).


* **Verification:** Clean install test from built release executable without terminal involvement.

---

## 10. Testing, Benchmarking, and Release Readiness

### Reproducible Benchmarking & Evaluation Pipeline

The coding agent must include a CLI evaluation runner (`cargo run --bin bench_eval` or Python evaluation harness) that measures:

1. **Word Error Rate (WER) & Character Error Rate (CER):** Evaluated on a curated local test set of 25+ representative utterances covering conversational English, Python/ML technical explanations, and spoken mathematics.
2. **Technical & Mathematical Preservation Rate (%):** Automated assertion check verifying exact retention of numbers, equations, LaTeX commands, and `snake_case`/`camelCase` identifiers after LLM rewriting.
3. **Latency Breakdown (P50 / P95 / P99):** Instrumenting `t_mic_stop`, `t_vad_done`, `t_asr_done`, `t_llm_done`, and `t_paste_done` across 50 consecutive runs.
4. **Resource Footprint:** Idle RAM, peak RAM, peak RTX 4060 VRAM (`nvidia-smi` polling), and average inference power draw.

### Mandatory Test Matrix

| Test Category | Scenario / Input | Expected System Behavior |
| --- | --- | --- |
| **Short Utterance Fast-Path** | *"Run pytest on the auth module."* (1.8s) | Bypasses LLM; deterministic capitalization/punctuation; inserted in `< 300 ms`. |
| **Messy Self-Correction** | *"Let's schedule the sync for Tuesday at 2 PM—actually no, make it Wednesday at 4:30 PM."* | Cleaned output: *"Let's schedule the sync for Wednesday at 4:30 PM."* |
| **Data Science / ML Jargon** | *"Use Reciprocal Rank Fusion over FAISS dense embeddings and BM25 sparse scores."* | Exact casing and spelling of `Reciprocal Rank Fusion`, `FAISS`, and `BM25` preserved. |
| **Mathematical Dictation** | *"theta transpose x plus b equals zero"* & *"sigma from i equals 1 to n of x sub i"* | `θᵀx + b = 0` in `Technical` mode; `\sum_{i=1}^{n} x_i` in `Math/LaTeX` mode. |
| **Long-Form Dictation (3 min)** | Continuous multi-paragraph technical explanation with 1.5s natural pauses | No premature cutoff in PTT mode; complete transcription; clean paragraph breaks. |
| **Silence & Noise Hallucination** | 5 seconds of silence or keyboard typing noise with no speech | Silero VAD rejects audio; no `"Thank you for watching"` hallucination inserted; clean pill notice. |
| **Clipboard Preservation** | User has an image or important URL on clipboard before dictating | Dictated text is pasted into target app; original clipboard content is restored after 220 ms. |
| **Admin / Elevated App (UIPI)** | User triggers dictation while focused inside Administrator PowerShell | Text is not lost; remains on system clipboard; overlay notifies `"Copied to clipboard (Admin app)"`. |
| **LLM Timeout / Failure Injection** | Kill `llama-server.exe` mid-transcription | System seamlessly falls back to Stage 1 deterministic ASR text, pastes it, and restarts `llama-server` in background. |
| **Crash Recovery** | Force-terminate app immediately after transcription | Reopening app or pressing `Ctrl+Alt+V` recovers the exact dictation from SQLite WAL storage. |

### Definition of Done (DoD)

The project is complete **only** when:

1. A compiled Windows release executable runs standalone from the system tray without requiring an open terminal.
2. Global Push-to-Talk and Toggle hotkeys work reliably across Chrome (ChatGPT/Claude), Cursor/VS Code, Windows Terminal, and Notepad.
3. Both `whisper-large-v3-turbo` (or `parakeet-tdt-0.6b-v2`) and `Qwen2.5-3B-Instruct` run with CUDA acceleration on your RTX 4060, achieving `< 1.1s` P50 end-to-end latency on standard utterances.
4. All 10 scenarios in the Test Matrix pass, including clipboard preservation, UIPI fallback, and zero-loss SQLite recovery.
5. Benchmark results, known limitations, and a clear daily-use guide are documented in the repository.

---

## 11. Risks, Open Questions, and Hardware-Dependent Decisions

### Technical Risks & Mitigations

1. **Windows Defender / SmartScreen False Positives:** Unsigned Windows binaries that install global `WH_KEYBOARD_LL` hooks and call `SendInput` can occasionally trigger heuristic warnings from Windows Defender.
* *Mitigation:* Build cleanly from source in the user's local workspace and add a local folder exclusion in Windows Security if prompted during build/testing.


2. **Electron / Chromium Clipboard Race Conditions:** Apps like Cursor, VS Code, Slack, and Chrome read the clipboard asynchronously after receiving `Ctrl+V`. Restoring the previous clipboard too quickly (< 100 ms) causes the target app to paste the *old* clipboard content.
* *Mitigation:* Enforce a configurable clipboard-restore delay defaulting to **220 ms** (up to 350 ms for heavy web apps) and verify sequence numbers before restoring.


3. **VRAM Contention During Local ML Training:** If you are actively training a PyTorch model or running ComfyUI on the RTX 4060's 8 GB VRAM while dictating, `llama-server` or `whisper.cpp` could encounter CUDA allocation errors.
* *Mitigation:* Configure `llama-server` with moderate KV cache (`4096` tokens) and implement automatic CPU fallback + one-click `"Unload GPU Models"` toggle in the tray menu.



### Specific Questions for the Coding Agent to Confirm with You

Before finalizing user-facing defaults, the coding agent should inspect your system first and ask you only for:

1. **Preferred Primary Hotkey:** Whether you prefer `Right Alt` (hold for Push-to-Talk, tap for Toggle), `Ctrl + Space`, or another dedicated key combination that doesn't collide with your Cursor/VS Code keybindings.
2. **Default ASR Engine Preference:** Whether to default to **`whisper-large-v3-turbo`** (best technical/multilingual accuracy + custom vocabulary prompting, ~220 ms) or **`nvidia/parakeet-tdt-0.6b-v2`** (fastest English latency, ~110 ms), with the other available as a 1-click switch in Settings.
3. **Startup Behavior:** Whether you want VoxFlow registered to launch automatically on Windows login.

---

## 12. Final Coding-Agent Handoff

> **Execution Brief for Autonomous Coding Agent:**
> You are tasked with implementing **VoxFlow**, the local-first, $0 AI voice dictation and intelligent rewriting desktop application specified in this blueprint, on the user's **Lenovo Legion 5i (Intel Core i9-14900HX, NVIDIA RTX 4060 Laptop GPU 8 GB VRAM, 32 GB DDR5 RAM, Windows 11)**.
> **Operational Directives:**
> 1. **Inspect First (Phase 0):** Before scaffolding or downloading dependencies, inspect the actual Windows environment (`nvidia-smi`, `nvcc`, `rustc`, `cargo`, `node`, `cmake`, `cl.exe`, existing `ollama`/`llama-server` binaries, and active audio recording devices). Adapt build flags to the exact CUDA and MSVC versions present.
> 2. **Zero Recurring Cost:** Use strictly open-source MIT/Apache-2.0/BSD libraries and open-weight local models (`whisper-large-v3-turbo` / `parakeet-tdt-0.6b-v2`, `Silero VAD v5`, and `Qwen2.5-3B-Instruct-Q4_K_M` via `llama-server.exe` CUDA). Never introduce paid APIs or cloud dependencies.
> 3. **Ask Only What Requires User Input:** Do not ask routine technical questions. Ask the user only for: (a) their preferred global hotkey (`Right Alt` vs `Ctrl+Space` vs custom), (b) confirmation on microphone selection if multiple inputs exist, and (c) whether to enable launch-on-Windows-startup.
> 4. **Build Incrementally & Verify Real Integration:** Implement Phase 1 through Phase 8 in order. After each phase, compile and test on the real Windows desktop (verifying non-activating overlay focus behavior, real microphone RMS/VAD capture, CUDA GPU acceleration, Win32 clipboard backup/restore, and UIPI elevation handling)—do not rely solely on mocked tests.
> 5. **Guarantee Zero Silent Failures:** Ensure SQLite WAL persistence happens *before* LLM rewriting and text insertion, and verify every fallback path (LLM timeout -> deterministic ASR; UIPI admin window -> clipboard retention + toast notification).
> 6. **Deliver a Finished, Packaged Application:** Conclude by building the release binary (`cargo tauri build`), running the latency/accuracy benchmark suite, verifying standalone launch without a terminal, and providing a concise user guide covering daily shortcuts, modes, and troubleshooting.
> 
> 

---

## 13. Sources and Further Technical References

### Open-Source Desktop Dictation & Architecture References

* **Handy (Rust + Tauri v2 + `transcribe-rs` local dictation reference):** [https://github.com/cjpais/Handy](https://github.com/cjpais/Handy) — Reference for Tauri v2 system tray, non-activating overlay ergonomics, and Rust ASR integration.
* **Whispering (Tauri local/modular transcription & transformation pipeline reference):** [https://github.com/braden-w/whispering](https://github.com/braden-w/whispering) — Reference for post-transcription transformation workflows and searchable history UX.
* **Vibe (Tauri + Rust + `whisper.cpp` GPU desktop reference):** [https://github.com/thewh1teagle/vibe](https://github.com/thewh1teagle/vibe) — Reference for Windows CUDA runtime bundling and model download verification.
* **Tauri v2 Official Documentation (Multi-window, System Tray, Win32 handle access):** [https://v2.tauri.app/](https://v2.tauri.app/)

### Speech Recognition & Voice Activity Detection Engines

* **OpenAI Whisper (`whisper-large-v3-turbo` architecture & weights):** [https://github.com/openai/whisper](https://github.com/openai/whisper) and [https://huggingface.co/openai/whisper-large-v3-turbo](https://huggingface.co/openai/whisper-large-v3-turbo)
* **whisper.cpp (C/C++ CUDA inference & GGML/GGUF quantized models):** [https://github.com/ggerganov/whisper.cpp](https://github.com/ggerganov/whisper.cpp)
* **faster-whisper (CTranslate2 high-throughput CUDA Whisper runtime):** [https://github.com/SYSTRAN/faster-whisper](https://github.com/SYSTRAN/faster-whisper)
* **NVIDIA Parakeet TDT 0.6B v2 (Ultra-low-latency FastConformer-TDT ASR model):** [https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2)
* **transcribe-rs (Unified Rust library for Whisper, Parakeet, and Moonshine):** [https://github.com/cjpais/transcribe-rs](https://github.com/cjpais/transcribe-rs)
* **Silero VAD v5 (Enterprise-grade MIT-licensed voice activity detector):** [https://github.com/snakers4/silero-vad](https://github.com/snakers4/silero-vad)

### Local LLM Inference & Constrained Rewriting

* **llama.cpp (`llama-server` CUDA runtime & GBNF JSON grammar constraints):** [https://github.com/ggerganov/llama.cpp](https://github.com/ggerganov/llama.cpp)
* **Qwen2.5-3B-Instruct (Compact instruction-tuned model & GGUF weights):** [https://huggingface.co/Qwen/Qwen2.5-3B-Instruct](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct) and [https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF)
* **Gemma 3 4B IT (Alternative local style/prose cleanup model):** [https://huggingface.co/google/gemma-3-4b-it](https://huggingface.co/google/gemma-3-4b-it)

### Windows System Integration & Rust Crates

* **Microsoft Win32 `SetWindowsHookExW` / `WH_KEYBOARD_LL` Documentation:** [https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)
* **Microsoft Win32 `SendInput` & Clipboard API Documentation:** [https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput) and [https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard)
* **Microsoft Windows User Interface Privilege Isolation (UIPI) Overview:** [https://learn.microsoft.com/en-us/windows/win32/secauthz/user-interface-privilege-isolation](https://www.google.com/search?q=https://learn.microsoft.com/en-us/windows/win32/secauthz/user-interface-privilege-isolation)
* **`windows-rs` (Official Microsoft Rust bindings for Win32 APIs):** [https://github.com/microsoft/windows-rs](https://github.com/microsoft/windows-rs)
* **`cpal` (Cross-Platform Audio Library in Rust with WASAPI backend):** [https://github.com/RustAudio/cpal](https://github.com/RustAudio/cpal)