# Voxora: agent notes

Tier 2 project (public, not a hosted service). Local-first dictation app for Windows and macOS (beta).

- Read `docs/ARCHITECTURE.md` first; decisions + evidence in `docs/DECISIONS.md` (append, don't rewrite).
- `crates/voxora-core` must stay free of Win32/Tauri deps; all text logic + tests live there.
- Gate: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm build`.
- Real-hardware checks: `bench_eval` (see `docs/BENCHMARKS.md`) and `scripts/e2e.ps1` (needs VB-Audio Virtual Cable; the script aborts unless its own window has focus; never weaken that guard, the app pastes into whatever is focused).
- OS code lives in four modules (`hotkey`, `inject`, `target`, `winutil`); macOS copies are in `src-tauri/src/mac/` with the same API. macOS cannot be compiled on Windows: the `macos` CI job is the check (it also smoke-tests real binaries).
- UI copy and docs: no em dashes (owner rule, D11).
- Data dir: `%LOCALAPPDATA%\Voxora` (models, runtime, logs, voxora.db, settings.json).
- Pinned model/runtime URLs + SHA-256 live only in `crates/voxora-core/src/engines/download.rs`.
