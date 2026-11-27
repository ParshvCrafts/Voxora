## What and why

<!-- One or two sentences. Link the issue: "Fixes #123". -->

## How I tested it

<!-- Unit test added? Ran the app? bench_eval before/after for accuracy-sensitive changes? -->

## Checklist

- [ ] `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm build` passes
- [ ] A test fails without this change (or explain why none fits)
- [ ] No transcript text or keystrokes are logged
- [ ] No em dashes in UI copy or docs; DECISIONS.md entry added if a default changed
