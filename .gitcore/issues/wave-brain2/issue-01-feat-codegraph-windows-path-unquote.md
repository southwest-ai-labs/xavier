# [Ola 32.01] feat-codegraph-windows-path-unquote — Sanitize and Unquote PATH Candidates

> Ola 32 — Infra. Labels: `ola32`, `wave-32`, `windows`, `codegraph`

## Current State (measurable)
- File: `src/codebase/codegraph_sidecar.rs:126-131`
- `sidecar_candidates()` iterates over `std::env::split_paths(&path)` and directly calls `dir.join(&name)`.
- On Windows, `PATH` entries with surrounding quotation marks (e.g. `"C:\Program Files\..."`) are not stripped by `split_paths`. Probing them via `std::fs::metadata` triggers Win32 `ERROR_INVALID_NAME` (code 123).
- `xavier doctor` currently reports: `[⚠] WARN CodeGraph Sidecar Candidates: ... (os error 123)`.

## Desired State (delta)
- In `src/codebase/codegraph_sidecar.rs`, sanitize each candidate directory produced by `split_paths`:
  - Trim surrounding single and double quotation marks and whitespace.
  - Drop empty segments before calling `dir.join(&name)`.
- Ensure Windows candidate names always prefer `.exe` suffix when present.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib codebase::codegraph_sidecar::tests::test_is_absent_candidate_error` — passes
- [ ] `cargo test -p xavier --lib codebase::codegraph_sidecar_tests::inspect_candidates_skips_absent_and_nonexistent_paths_without_warning` — passes
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] Run `xavier doctor` — no `os error 123` reported in diagnostic output

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/codebase/codegraph_sidecar.rs` | Lines 126–131 produce malformed quoted paths | Add `trim_matches(['\"', '\''])` sanitation in candidate collection | LOW |

## DO NOT touch
- `code-graph/src/*` — owned by other issues
- `.gitcore/features.json` — reconciled at wave close

## Anti-Hallucination Guard
1. READ `src/codebase/codegraph_sidecar.rs` lines 110–190 before writing.
2. Use `std::env::split_paths` followed by clean string unquoting, never raw regex string splitting.

## Merge order
- Within wave: 1/10 · Effort: Small (<30m) · Parallel with: all in Ola 32
