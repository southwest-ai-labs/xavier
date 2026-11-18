# [Ola 33.20] feat-wiki-sqlite-container — Unified SQLite Living Wiki Container with ZSTD Compression

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `wiki`, `storage`, `sqlite`

## Current State (measurable)
- Living wiki entities and concepts are generated as loose `.md` files on disk.
- Managing thousands of individual markdown files creates filesystem overhead, lack of atomic transactions on multi-file refactors, and slow query-time joins.

## Desired State (delta)
- Create `src/wiki/storage.rs`:
  - Implement `WikiStorageEngine`:
    - Opens or creates `.xavier/wiki.sqlite`.
    - Creates tables:
      - `wiki_nodes (node_id INTEGER PRIMARY KEY, stable_id TEXT UNIQUE, title TEXT, kind INTEGER, project_id INTEGER, file_path TEXT, content_zstd BLOB, token_count INTEGER, updated_at INTEGER)`
      - `wiki_edges (source_id INTEGER, target_id INTEGER, edge_type INTEGER, weight REAL, PRIMARY KEY (source_id, target_id, edge_type))`
      - Index: `idx_wiki_edges_target ON wiki_edges(target_id)`
    - Compresses markdown content using `zstd::encode_all(..., 3)`.
    - Provides atomic transactional updates: inserting or modifying an entity and its edges occurs within a single SQLite transaction.
    - Exposes export method `export_to_markdown_dir(target_dir: &Path)` and import method `import_from_markdown_dir(source_dir: &Path)` for Obsidian and Git interop.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::storage` — passes with node/edge persistence and ZSTD roundtrip tests
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] Exporting and re-importing a wiki directory preserves 100% of nodes, frontmatter, and edges

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/storage.rs` | Does not exist | Implement `WikiStorageEngine` with ZSTD compressed nodes and edge indexing | MED |
| `src/wiki/mod.rs` | Missing `pub mod storage;` | Export `storage` module | LOW |

## DO NOT touch
- `src/wiki/schema.rs` — schema types remain stable
- `src/wiki/gpu_vram.rs` — owned by Issue 21

## Anti-Hallucination Guard
1. Use `rusqlite` with WAL journal mode (`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;`).
2. Compression level 3 provides optimal compression speed and ratio for markdown text.

## Merge order
- Within wave: 10/11 · Effort: Medium (<1.5h) · Parallel with: all in Ola 33
