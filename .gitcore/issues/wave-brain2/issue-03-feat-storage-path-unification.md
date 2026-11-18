# [Ola 32.03] feat-storage-path-unification — Unify Storage in LocalAppData on Windows

> Ola 32 — Infra. Labels: `ola32`, `wave-32`, `storage`, `settings`, `windows`

## Current State (measurable)
- Files: `src/settings/defaults.rs:86` and `src/settings/serialization.rs:161-170`
- `resolve_data_dir()` calls `dirs::data_dir()` which on Windows resolves to `%APPDATA%` (`C:\Users\<user>\AppData\Roaming\xavier`).
- In contrast, runtime state (`resolve_runtime_state_path`) uses `dirs::state_dir()` which resolves to `%LOCALAPPDATA%` (`C:\Users\<user>\AppData\Local\xavier`).
- High-churn SQLite WAL databases (`vec-store.sqlite3`, `xavier_memory.db`) are duplicated between Roaming and Local, causing storage sprawl and sync conflicts.
- Host environments with legacy `XAVIER_DATA_DIR` set in user registry (e.g. `E:\scripts-python\xavier\data`) pollute all local test runs.

## Desired State (delta)
- In `src/settings/serialization.rs`:
  - On Windows (`#[cfg(windows)]`), resolve data root using `dirs::data_local_dir().map(|dir| dir.join("xavier"))` instead of `dirs::data_dir()`.
  - Add path normalization to ensure database paths do not create nested `data/data` directories.
  - Guard `test_support::guard_user_path` to verify that test executions never write to production user data directories.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib settings::tests::test_resolve_data_dir` — passes
- [ ] `cargo test -p xavier --lib settings::serialization::tests` — passes
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/settings/serialization.rs` | Uses `dirs::data_dir()` (Roaming) on Windows | Switch to `dirs::data_local_dir()` on Windows + anti-nesting guard | MED |
| `src/settings/defaults.rs` | References default data dir | Ensure default paths align with unified local directory | LOW |

## DO NOT touch
- `src/storage/migrations.rs` — migrations contract remains unchanged
- `src/memory/*` — owned by other issues

## Anti-Hallucination Guard
1. READ `src/settings/serialization.rs` lines 150–225 before writing.
2. Follow cross-platform conditional compilation (`#[cfg(windows)]` vs `#[cfg(not(windows))]`).

## Merge order
- Within wave: 3/10 · Effort: Small (<45m) · Parallel with: all in Ola 32
