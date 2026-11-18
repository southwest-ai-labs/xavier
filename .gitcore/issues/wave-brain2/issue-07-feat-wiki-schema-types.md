# [Ola 32.07] feat-wiki-schema-types — Define Living Wiki Core Schema & Data Types

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `wiki`, `schema`, `brain2`

## Current State (measurable)
- Xavier lacks a unified data model for living compiled knowledge.
- Notes and memory entries are represented solely by `MemoryDocument` / `MemoryRecord` (flat text + untyped JSON metadata).
- There are no typed representations for `WikiEntity`, `WikiConcept`, `WikiIndex`, or `WikiLogEntry`.

## Desired State (delta)
- Create `src/wiki/mod.rs`:
  - Declare `pub mod schema;`, `pub mod compiler;`, `pub mod log;`, `pub mod indexer;`.
- Create `src/wiki/schema.rs`:
  - Define `WikiEntity`:
    - `id: String`, `name: String`, `kind: String` ("struct" | "function" | "trait" | "module")
    - `lang: String`, `file_path: String`, `signature: Option<String>`
    - `summary: String`, `calls: Vec<String>`, `called_by: Vec<String>`
    - `wikilinks: Vec<String>`, `updated_at: DateTime<Utc>`
  - Define `WikiConcept`:
    - `id: String`, `title: String`, `category: String` ("architecture" | "adr" | "workflow")
    - `content: String`, `related_entities: Vec<String>`, `tags: Vec<String>`
  - Define `WikiLogEntry`:
    - `timestamp: DateTime<Utc>`, `author_or_agent: String`, `event_type: String`
    - `summary: String`, `touched_entities: Vec<String>`
  - Implement serialization to/from Markdown with YAML frontmatter.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo check --workspace --features ci-safe` — 0 errors
- [ ] `cargo test -p xavier --lib wiki::schema` — passes with serialization and YAML frontmatter roundtrip tests
- [ ] `cargo clippy --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/mod.rs` | Does not exist | Create module declaration and public re-exports | LOW |
| `src/wiki/schema.rs` | Does not exist | Implement `WikiEntity`, `WikiConcept`, `WikiLogEntry` + frontmatter serde | LOW |
| `src/lib.rs` | Missing `pub mod wiki;` | Add `pub mod wiki;` to top-level crate exports | LOW |

## DO NOT touch
- `src/memory/*` — owned by other issues
- `code-graph/*` — owned by other issues

## Anti-Hallucination Guard
1. Use `serde_yaml` for YAML frontmatter parsing and `serde_json` for in-memory serialization.
2. Follow `chrono::Utc` for all timestamps.

## Merge order
- Within wave: 7/10 · Effort: Small (<45m) · Parallel with: all in Ola 32
