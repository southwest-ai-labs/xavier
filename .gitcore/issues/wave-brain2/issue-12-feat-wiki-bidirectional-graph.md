# [Ola 33.12] feat-wiki-bidirectional-graph — Bi-directional Wikilink Graph Engine

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `wiki`, `graph`, `backlinks`

## Current State (measurable)
- When notes or memories reference other notes (via `[[Link]]`), links are stored as a flat array in `metadata["wikilinks"]`.
- There is no bi-directional index. Querying "which notes link to this entity?" requires a full table scan.
- Finding orphan pages (notes that are never linked) is not supported.

## Desired State (delta)
- Create `src/wiki/graph.rs`:
  - Implement `WikiGraph`:
    - Maintains SQLite table `wiki_links (source_id TEXT, target_id TEXT, link_type TEXT, created_at TEXT)`.
    - Automatically updates edges when a wiki page is created, updated, or deleted.
    - Exposes queries:
      - `get_forward_links(page_id: &str) -> Result<Vec<String>>`
      - `get_backlinks(page_id: &str) -> Result<Vec<String>>`
      - `find_orphan_pages() -> Result<Vec<String>>`
      - `get_neighborhood(page_id: &str, depth: usize) -> Result<WikiSubGraph>`

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::graph` — unit tests pass (forward links, backlinks, and orphan detection)
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/graph.rs` | Does not exist | Implement `WikiGraph` with bi-directional edge indexing in SQLite | MED |

## DO NOT touch
- `src/wiki/linter.rs` — owned by Issue 15
- `code-graph/*` — AST graph stays separate from wiki note graph

## Anti-Hallucination Guard
1. READ `src/server/v1_api.rs` lines 2513–2551 for existing `GraphViewNode` patterns.
2. Use indexed SQLite queries (`CREATE INDEX IF NOT EXISTS idx_wiki_links_target ON wiki_links(target_id)`).

## Merge order
- Within wave: 2/9 · Effort: Medium (<1.5h) · Parallel with: all in Ola 33
