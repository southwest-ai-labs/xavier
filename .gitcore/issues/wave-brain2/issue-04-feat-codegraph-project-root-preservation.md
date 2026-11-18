# [Ola 32.04] feat-codegraph-project-root-preservation — Preserve project_root in CodeGraph SELECT Queries

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `codegraph`, `database`

## Current State (measurable)
- File: `code-graph/src/db/mod.rs` (lines 819, 854, 933, 974, 1026, 1279, 1316, 1459, 2126)
- In all 9 SELECT queries that construct `Symbol`, the SQL query selects only columns 0 through 12, omitting `s.project_root`.
- When constructing `Symbol`, line 2126 (and its sister locations) hardcodes:
  `project_root: None`.
- As a result, even though `project_root` is persisted on INSERT, it is discarded upon reading, breaking multi-project isolation and causing `stats_for_project` to filter incorrectly.

## Desired State (delta)
- In `code-graph/src/db/mod.rs`:
  - Update all `SELECT` statements for symbols to include `s.project_root` as column index 13:
    `SELECT id, stable_id, name, kind, lang, file_path, start_line, end_line, start_col, end_col, signature, parent, complexity, project_root`
  - Update row deserialization:
    `project_root: row.get(13).ok()`
  - Ensure `find_symbols_for_memory` (lines 2095–2134) selects `s.project_root` and preserves it in returned `Symbol` structs.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p code-graph --lib db::tests::test_insert_symbols_project_root` — passes
- [ ] `cargo test -p code-graph --lib db::tests` — all tests pass
- [ ] `cargo clippy -p code-graph --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] Querying symbols inserted with `project_root = Some("my-repo")` returns `symbol.project_root == Some("my-repo")`

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `code-graph/src/db/mod.rs` | 9 queries drop `project_root` and assign `project_root: None` | Include column 13 (`project_root`) in SELECT and row mapper | MED |

## DO NOT touch
- `code-graph/src/indexer/*` — owned by Issue 05
- `src/codebase/*` — owned by other issues

## Anti-Hallucination Guard
1. READ `code-graph/src/db/mod.rs` lines 800–1050 and 2090–2135 before writing.
2. Verify all SELECT statements use consistent column numbering (0 to 13).

## Merge order
- Within wave: 4/10 · Effort: Medium (<1h) · Parallel with: all in Ola 32
