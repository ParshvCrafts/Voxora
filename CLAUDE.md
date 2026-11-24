# Voxora: agent notes

Tier 2 project (public, not a hosted service). Local-first Windows dictation app.

- Read `docs/ARCHITECTURE.md` first; decisions + evidence in `docs/DECISIONS.md` (append, don't rewrite).
- `crates/voxora-core` must stay free of Win32/Tauri deps; all text logic + tests live there.
- Gate: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm build`.
- Real-hardware checks: `bench_eval` (see `docs/BENCHMARKS.md`) and `scripts/e2e.ps1` (needs VB-Audio Virtual Cable; the script aborts unless its own window has focus; never weaken that guard, the app pastes into whatever is focused).
- Data dir: `%LOCALAPPDATA%\Voxora` (models, runtime, logs, voxora.db, settings.json).
- Pinned model/runtime URLs + SHA-256 live only in `crates/voxora-core/src/engines/download.rs`.
