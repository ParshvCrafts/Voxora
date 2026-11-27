# Contributing to Voxora

Thanks for helping. Voxora is a local-first dictation app: hold a key, speak, and clean text
lands in whatever app you are typing in. Everything runs on the user's own computer, so the
bar for changes is "does it still work offline, privately, and fast?"

## Ways to help that matter most right now

1. **Try it on a real Mac** and [report what happened](https://github.com/ParshvCrafts/Voxora/issues/new?template=mac_report.yml).
   The macOS build is compiled and smoke-tested in CI, but nobody has used it by hand yet.
2. **Report a dictation that came out wrong.** Use the bug template, paste the raw and final
   text from History, and say which app and mode. These become benchmark cases.
3. **Pick a [good first issue](https://github.com/ParshvCrafts/Voxora/labels/good%20first%20issue).**
4. See [docs/ROADMAP.md](docs/ROADMAP.md) for bigger pieces of work.

## Set up

You need Rust (stable), Node 22+ and pnpm 10.

```bash
pnpm install
pnpm tauri dev           # run the app; first launch downloads the models
cargo test --workspace   # unit tests, no GPU or models needed
```

Text logic (normalization, math, the guard, app rules, storage) lives in `crates/voxora-core`
and is fully testable without a GPU, a microphone or the app. Most contributions only touch that
crate. Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the big picture.

## Before you open a pull request

Run the same gate CI runs:

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm build
```

- **Add a test for the bug or feature**, in `voxora-core` where possible. A test should fail
  without your change.
- **Keep `voxora-core` free of OS and Tauri code.** Windows code lives in `src-tauri/src/`
  (`hotkey`, `inject`, `target`, `winutil`), the macOS copies in `src-tauri/src/mac/` with the
  same API. macOS cannot be compiled on Windows; the `macos` CI job checks it.
- **Never log what the user types or says.** Input handling may only look at the dictation key
  and Esc. Logs must not contain transcript text.
- **Style:** `cargo fmt`, and no em dashes in UI copy or docs (use a comma, colon or period).
- **Decisions:** if you change a default or pick between approaches, add a short entry to
  [docs/DECISIONS.md](docs/DECISIONS.md) (append, do not rewrite) with the evidence.
- Changing accuracy-sensitive code (normalization, prompts, the guard)? Run `bench_eval` as
  described in [docs/BENCHMARKS.md](docs/BENCHMARKS.md) and paste the before and after.

## Commits and pull requests

- Small, focused pull requests are reviewed fastest. One change per PR.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
  (`fix(hotkey): ...`, `feat(ui): ...`, `docs: ...`).
- By contributing you agree your work is released under the [MIT license](LICENSE).

## Reporting security issues

Please do not open a public issue; see [SECURITY.md](SECURITY.md).

Everyone taking part is expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
