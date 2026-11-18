# [Ola 32.06] feat-git-head-auto-sync — Automatic Git HEAD Delta Trigger for CodeGraph

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `codegraph`, `git`

## Current State (measurable)
- File: `src/cli/codegraph_sync.rs:89-189`
- `sync_codegraph_with_state` implements incremental git delta computation using `git diff --name-status` against `.xavier/codegraph-sync-commit`.
- However, this function is only executed when a user explicitly runs `xavier code sync --git` in terminal or hits `POST /code/sync`.
- When an agent or developer pulls or commits in git, CodeGraph stays stale until manual sync.

## Desired State (delta)
- Create `src/codebase/sync.rs`:
  - Implement `GitHeadWatcher` that checks `git rev-parse HEAD` on a lightweight periodic timer or on workspace activation.
  - If the commit hash differs from `.xavier/codegraph-sync-commit`, invoke `sync_codegraph_from_git` automatically in background.
- In `src/cli/codegraph_sync.rs`:
  - Expose helper `check_and_sync_if_head_changed(workspace: &Path) -> Result<Option<IndexStats>>`.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib cli::codegraph_sync::tests::parses_name_status_z_basic` — passes
- [ ] `cargo test -p xavier --lib cli::codegraph_sync::tests::skips_xavier_and_db_paths` — passes
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/codebase/sync.rs` | Does not exist | Create `GitHeadWatcher` and HEAD change checker | LOW |
| `src/cli/codegraph_sync.rs` | Lines 58–189 require explicit CLI call | Expose non-CLI programmatic sync entry point | LOW |

## DO NOT touch
- `code-graph/src/*` — owned by other issues
- `src/server/*` — owned by later issues

## Anti-Hallucination Guard
1. READ `src/cli/codegraph_sync.rs` lines 40–120 before writing.
2. Fail-open: if git command fails (e.g. not a git repo), log debug and return `Ok(None)`.

## Merge order
- Within wave: 6/10 · Effort: Small (<45m) · Parallel with: all in Ola 32
