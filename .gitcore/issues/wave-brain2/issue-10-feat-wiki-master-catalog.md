# [Ola 32.10] feat-wiki-master-catalog — Master Catalog and MOC Indexer Generator

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `wiki`, `indexer`, `brain2`

## Current State (measurable)
- Repositories stored in Xavier lack a master catalog (`index.md`) that maps architectural modules, core entities, and entry points.
- Agents entering a codebase must discover everything from scratch by running broad searches or reading many files.

## Desired State (delta)
- Create `src/wiki/indexer.rs`:
  - Implement `WikiCatalogBuilder`:
    - Generates `.xavier/wiki/index.md`.
    - Synthesizes:
      1. Repository metadata (name, language breakdown from CodeGraph `IndexStats`).
      2. High-degree architectural hubs (from `code_query.hubs()`).
      3. Categorized entity list by domain/module with wikilinks (`[[entities/...]]`).
      4. Recent changelog summary (top 5 entries from `log.md`).
    - Provides single-entry map of content (MOC) for agents querying project structure.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::indexer` — unit tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] `index.md` contains valid relative wikilinks to all generated entity pages

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/indexer.rs` | Does not exist | Implement `WikiCatalogBuilder` to produce master `index.md` | MED |

## DO NOT touch
- `src/context/builder.rs` — owned by Issue 17
- `src/wiki/compiler.rs` — owned by Issue 08

## Anti-Hallucination Guard
1. READ `code-graph/src/types.rs` for `HubNode` and `IndexStats` definitions.
2. Ensure markdown tables render valid GitHub Flavored Markdown.

## Merge order
- Within wave: 10/10 · Effort: Medium (<1h) · Parallel with: all in Ola 32
