# [Ola 32.09] feat-wiki-chronicle-log — Append-Only Chronicle Log Engine for Wiki

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `wiki`, `chronicle`, `brain2`

## Current State (measurable)
- Chronological logs of changes in Xavier are scattered across ephemeral terminal sessions and unindexed JSON records in SQLite.
- Karpathy's LLM Wiki architecture mandates an append-only `log.md` that chronologically journals all ingested sources, git commits, and agent tasks.
- No unified file `.xavier/wiki/log.md` exists.

## Desired State (delta)
- Create `src/wiki/log.rs`:
  - Implement `WikiLogManager`:
    - Maintains `.xavier/wiki/log.md`.
    - Exposes `append_entry(entry: WikiLogEntry) -> Result<()>`.
    - Formats entries in markdown with chronological headers (`## YYYY-MM-DD HH:MM:SS — [Event] Title`), author/agent tag, summary, and touched wikilinks (`[[entities/SymbolName]]`).
    - Ensures concurrent write safety via file locking or Tokio mutex.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::log` — unit tests pass (append, read, and format entries in tempdir)
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] `log.md` renders cleanly in standard Markdown readers

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/log.rs` | Does not exist | Implement `WikiLogManager` for append-only `log.md` journaling | LOW |

## DO NOT touch
- `src/wiki/schema.rs` — owned by Issue 07
- `src/chronicle/*` — legacy chronicle module stays untouched

## Anti-Hallucination Guard
1. READ `src/wiki/schema.rs` for `WikiLogEntry` fields.
2. File appending must use `std::fs::OpenOptions::new().append(true).create(true)`.

## Merge order
- Within wave: 9/10 · Effort: Small (<45m) · Parallel with: all in Ola 32
