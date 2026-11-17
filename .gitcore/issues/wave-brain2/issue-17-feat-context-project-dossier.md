# [Ola 33.17] feat-context-project-dossier — Compiled Project Dossier Engine

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `context`, `dossier`, `mcp`

## Current State (measurable)
- File: `src/context/builder.rs:281-335`
- `get_project_context` simply filters documents by `doc.metadata.namespace.project == project_id` or path prefix.
- In practice, it returns verbatim chunks of terminal logs (`[run_finished] agy.exe ...`, `[run_started] xavier.exe ...`).
- An agent invoking `get_project_context` receives 8,000 characters of terminal noise with `0` architectural insight.

## Desired State (delta)
- Create `src/context/dossier.rs`:
  - Implement `ProjectDossierBuilder`:
    - Constructs a synthesized project overview from:
      1. `.xavier/wiki/index.md` (Project summary & core modules)
      2. Top 5 architectural hubs from `code_query.hubs()`
      3. Active ADRs and guidelines from `.xavier/wiki/concepts/`
      4. Last 3 entries from `.xavier/wiki/log.md`
    - Filters out 100% of raw terminal execution traces and runner heartbeat ticks (`is_heartbeat_tick`).
- In `src/context/builder.rs`:
  - When `get_project_context` is invoked, prioritize the compiled Project Dossier as the primary payload.
  - Virtualize detailed execution logs as reference IDs (`[REF:id]`) rather than dumping full bodies.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib context::builder::tests` — passes
- [ ] `cargo test -p xavier --lib context::dossier::tests` — passes (dossier contains hubs, summary, and no runner noise)
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] `get_project_context` token size reduced by >= 60% compared to raw execution dump

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/context/dossier.rs` | Does not exist | Implement `ProjectDossierBuilder` synthesizing wiki index, hubs, and log | MED |
| `src/context/builder.rs` | Lines 281–335 return raw unfiltered documents | Delegate to `ProjectDossierBuilder` | MED |

## DO NOT touch
- `src/server/mcp/*` — owned by Issue 18
- `src/memory/sanitizer.rs` — use existing sanitizer helpers

## Anti-Hallucination Guard
1. READ `src/context/builder.rs` lines 270–345 before writing.
2. Follow progressive disclosure: summary in context, full raw records available on-demand by ID.

## Merge order
- Within wave: 7/9 · Effort: Medium (<1.5h) · Parallel with: all in Ola 33
