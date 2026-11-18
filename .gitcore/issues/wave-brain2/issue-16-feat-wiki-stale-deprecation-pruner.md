# [Ola 33.16] feat-wiki-stale-deprecation-pruner — Stale Concept and Deprecation Pruner

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `wiki`, `prune`, `deprecation`

## Current State (measurable)
- Stale notes in Xavier either linger indefinitely or get deleted during arbitrary memory decay (`src/memory/decay.rs`).
- Deleting an entity completely destroys historical context, while keeping it unflagged confuses agents.

## Desired State (delta)
- Create `src/wiki/prune.rs`:
  - Implement `WikiPruner`:
    - Takes `LintReport` from Issue 15.
    - For entities identified with Code-Doc Drift (code deleted/moved):
      - Instead of deleting the `.md` file, prepend a Markdown deprecation notice:
        `> [!WARNING]`  
        `> **DEPRECATED / ARCHIVED:** The underlying code symbol was removed or renamed in commit <hash>.`
      - Update frontmatter: `deprecated: true`, `archived_at: "YYYY-MM-DD"`.
    - Move completely unreferenced orphan notes older than 60 days to `.xavier/wiki/archive/`.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::prune` — unit tests pass (appends deprecation badge and moves to archive)
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/prune.rs` | Does not exist | Implement `WikiPruner` with non-destructive deprecation tagging | LOW |

## DO NOT touch
- `src/wiki/linter.rs` — owned by Issue 15
- `src/memory/decay.rs` — raw vector decay remains independent

## Anti-Hallucination Guard
1. Always prefer soft deprecation (`deprecated: true`) over hard file deletion.
2. Maintain Git history cleanliness.

## Merge order
- Within wave: 6/9 · Effort: Small (<45m) · Parallel with: all in Ola 33
