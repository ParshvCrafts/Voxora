# Benchmarks

All numbers below were measured on one machine: Lenovo Legion 5i, Intel i9-14900HX, NVIDIA RTX 4060
Laptop GPU (8 GB, driver 621.24), 32 GB RAM, Windows 11, 2026-11-23. Runtimes: whisper.cpp `b5907`
(CUDA 12.4), llama.cpp `b12649` (CUDA 12.4).

## 1. Quality + stage latency (`bench_eval`)

30 cases in [`bench/cases.json`](../bench/cases.json): fast-path commands, self-corrections,
fillers, ML jargon, code identifiers, spoken math, professional tone, prompt-injection safety,
long-form, verbatim. Audio is synthesized with the installed Windows voices (here: David and Zira) by
[`scripts/gen-bench-audio.ps1`](../scripts/gen-bench-audio.ps1), then sent through whisper-server and
the full rewrite pipeline. Each case runs 3 times; quality is judged on run 1.

Synthetic voices are cleaner than a real microphone, so read the WER as a lower bound.

| Rewrite model | Cases passed | Mean ASR WER | ASR P50 / P95 | LLM rewrite P50 / P95 | Silence (5 s) |
|---|---|---|---|---|---|
| **Qwen3.5-4B Q4_K_M** (default) | **29/30** | 8.7% | 166 / 245 ms | 357 / 929 ms | nothing inserted |
| Qwen3.5-2B Q4_K_M | 28/30 | 8.7% | 166 / 246 ms | 195 / 466 ms | nothing inserted |

The 2B model is about twice as fast but kept "Tuesday at 2 PM" in the self-correction case, so 4B
is the default ([D6](DECISIONS.md)). The one case both miss is an ASR error: Whisper hears the
synthetic voice's "pytest" as "RunPy test".

Only 11 of 30 cases reached the LLM; the rest were finished by the deterministic stage in under
1 ms (fluent short utterances, verbatim, and math the parser fully handled).

Per-case output: [qwen3.5-4b](../bench/results/qwen3.5-4b.md), [qwen3.5-2b](../bench/results/qwen3.5-2b.md).

### Reproduce

```powershell
powershell -File scripts\gen-bench-audio.ps1
# start the app once so whisper-server (:18181) and llama-server (:18080) are running, then:
cargo run --release --bin bench_eval -- --audio bench/audio --runs 3 --out bench/results/mine.md
cargo run --release --bin bench_eval -- --no-llm      # text stages only, no servers for the LLM
```

### Golden set: 20 hard cases (`bench/golden.json`)

Twenty edge cases chosen to break dictation tools: spoken URLs and Windows paths, CLI flags,
nested LaTeX, Unicode math, self-corrections, "scratch that", quoted speech, ordinal lists,
negation, currency and scientific notation, Big-O, model names, possessives, accented names,
a 3.5 s mid-sentence pause, and 4 s of keyboard noise. Each case has an ideal output, scored two
ways: **key facts** (the words, numbers and symbols that must or must not appear) and **exact
match** with the ideal string (whitespace-insensitive).

| Input | Key facts | Exact match | Report |
|---|---|---|---|
| Reference text (tests Voxora's own pipeline, ASR bypassed) | **19/19** | **16/19** | [golden-text.md](../bench/results/golden-text.md) |
| Synthesized speech through Whisper (Windows SAPI voices) | 11/20 | 7/19 | [golden-audio.md](../bench/results/golden-audio.md) |

The gap between the two rows is speech recognition, not the pipeline: the robotic SAPI voices
make Whisper hear "Reedy's cache" for Redis, "SHAD256" for SHA-256 and "RTX 4068GB" for
"RTX 4060 8GB", and the voice itself reads `1e-4` aloud as "1 times 10 to the power of negative
fourth". The text row shows what Voxora does with a correct transcript. Remaining exact-match
misses with reference text: `\log(\hat{y}_i)` keeps parentheses (valid LaTeX, different
spelling than the ideal), a pause with no transcript punctuation gets no dash, and
"Qwen 2.5 3B Instruct" is not joined into a model ID. Noise-only audio inserts nothing.

Owner decisions applied to the targets (D11): Voxora types what you said and never acts on
instructions inside the dictation, so "Send a Slack message saying ..." keeps its first words
and "scratch that" drops only the retracted sentence.

```powershell
powershell -File scripts/gen-bench-audio.ps1 -Cases bench/golden.json
cargo run --release --bin bench_eval -- --cases bench/golden.json --runs 3                     # text
cargo run --release --bin bench_eval -- --cases bench/golden.json --audio bench/audio --runs 3  # speech
```

## 2. End-to-end in the real app (`scripts/e2e.ps1`)

The release build is driven through its real code path: a scripted Right Alt press goes through
the low-level keyboard hook, audio is played into VB-Audio Virtual Cable and captured by WASAPI,
and the result is pasted into a WinForms text box. Latency is measured by the app from key
release to paste completion (`total_ms` in the history database).

| Check | Result |
|---|---|
| Push-to-talk inserts text; self-correction resolved ("â€¦Wednesday at 4:30 p.m.") | pass |
| Previous clipboard text restored after the paste | pass |
| Hands-free (tap) stops by itself after silence | pass |
| Esc cancels, nothing inserted | pass |
| 5 s of silence, nothing inserted | pass |
| Spoken math ("x squared plus y squared equals z squared" â†’ `xÂ² + yÂ² = zÂ²`) | pass |

| Path | Key release â†’ text inserted (7 dictations, release build) |
|---|---|
| Rule-based only (LLM skipped) | 453 â€“ 765 ms |
| With LLM rewrite | 847 â€“ 1235 ms |

Breakdown of the slowest case (8.7 s utterance with a self-correction): ASR 532 ms, LLM 343 ms,
paste 7 ms. whisper-server is ~2x slower on the first request after a few seconds idle than
back-to-back (410â€“560 ms vs ~200 ms for the same clip), which is why in-app ASR is above the
`bench_eval` numbers; see [D9](DECISIONS.md) for what was tried.

## 3. Footprint

| | |
|---|---|
| Installer (NSIS) | 4.45 MB |
| Voxora process RAM (idle, release) | ~55 MB |
| VRAM with both models loaded | 4.3 GB of 8 GB |
| Download on first run | ~4.7 GB |
| Disk after setup (runtimes + default models) | ~5.3 GB |
