# [Ola 32.05] feat-codegraph-reactive-watcher — Implement Debounced AST Filesystem Watcher

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `codegraph`, `watcher`

## Current State (measurable)
- File: `code-graph/src/indexer/mod.rs:9`
- `// pub mod watcher;` is commented out.
- `code-graph/src/indexer/watcher.rs` was removed during refactoring.
- The dependency `notify = "6.1.1"` is already declared in `code-graph/Cargo.toml:59`.
- Changes to source files on disk do NOT update `code_graph.db` automatically; developers/agents must manually invoke `xavier code scan .` or `xavier code sync --git`.

## Desired State (delta)
- Create `code-graph/src/indexer/watcher.rs`:
  - Implement `CodeGraphWatcher` using `notify::RecommendedWatcher`.
  - Add debouncing (500ms window) to batch rapid file save events.
  - Implement ignore rules: ignore `.git/`, `target/`, `node_modules/`, `.xavier/`, `dist/`, `build/`.
  - Map `notify::Event` into `crate::indexer::PathChange` (`added`, `modified`, `deleted`, `renamed`).
  - Forward batched changes to `Indexer::apply_paths(&root, &changes).await`.
- In `code-graph/src/indexer/mod.rs`:
  - Uncomment `pub mod watcher;`.
  - Expose `pub use watcher::CodeGraphWatcher;`.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p code-graph --lib indexer::watcher` — new unit tests pass (tempdir file creation triggers index update)
- [ ] `cargo test -p code-graph --lib indexer::tests::apply_paths_adds_updates_and_deletes` — passes
- [ ] `cargo clippy -p code-graph --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `code-graph/src/indexer/watcher.rs` | Does not exist | Create `CodeGraphWatcher` with `notify` + debouncing + ignore rules | MED |
| `code-graph/src/indexer/mod.rs` | Line 9 commented out | Uncomment `pub mod watcher;` and export | LOW |

## DO NOT touch
- `code-graph/src/db/*` — owned by Issue 04
- `src/cli/codegraph_sync.rs` — owned by Issue 06

## Anti-Hallucination Guard
1. Use `tokio::sync::mpsc` for cross-thread event delivery from the `notify` callback into Tokio async runtime.
2. Never call blocking filesystem calls directly on the Tokio thread; use `spawn_blocking` where appropriate.

## Merge order
- Within wave: 5/10 · Effort: Medium (<1.5h) · Parallel with: all in Ola 32
