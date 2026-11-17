# [Ola 32.02] feat-mcp-codegraph-deterministic-load — Deterministic CodeGraphDB Loading in MCP

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `mcp`, `codegraph`

## Current State (measurable)
- File: `src/cli/mcp.rs:43-63`
- If `./.xavier/code_graph.db` does not exist in the working directory when starting the MCP server, line 62 silently executes:
  `Arc::new(code_graph::db::CodeGraphDB::in_memory()?)`.
- There is no warning emitted, and subsequent MCP calls (`get_code_graph`, `codegraph_explore`, `trace_path`) return empty symbol arrays (`{"returned": 0, "symbols": []}`) without alerting the agent that the repository has not been indexed or that a persistent DB path was not found.

## Desired State (delta)
- In `src/cli/mcp.rs` (`build_mcp_state`):
  - Check canonical workspace locations in order:
    1. Local workspace `.xavier/code_graph.db`
    2. `XavierSettings::resolve_data_dir().join("code_graph.db")`
    3. Look up project root from active git repository
  - If no persistent DB is found, log an explicit warning (`tracing::warn!("No persistent CodeGraphDB found at {}; starting unpopulated in-memory graph. Run 'xavier code scan .' to populate", path.display())`).
  - Store a `db_degraded_reason` flag in `AppState` so MCP tools report an actionable hint (`"CodeGraph is empty. Please run 'xavier code scan .' to index this workspace"`) rather than silent zeroes.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test --test mcp_tool_parity` — all tests pass
- [ ] `cargo test --test mcp_health_honesty_test` — all tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] Running MCP with missing DB emits explicit warning in tracing log

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/cli/mcp.rs` | Lines 43–63 fall back silently to in-memory DB | Add deterministic lookup + explicit warning + degraded state flag | LOW |

## DO NOT touch
- `src/server/mcp/tools_core.rs` — owned by later issues
- `code-graph/src/*` — owned by other issues

## Anti-Hallucination Guard
1. READ `src/cli/mcp.rs` lines 20–75 before writing.
2. Do not remove the in-memory fallback completely (to avoid hard crashes when running in temporary dirs), but eliminate its silence and track the degraded state honestly.

## Merge order
- Within wave: 2/10 · Effort: Small (<45m) · Parallel with: all in Ola 32
