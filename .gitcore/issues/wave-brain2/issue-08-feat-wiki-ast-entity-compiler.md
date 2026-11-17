# [Ola 32.08] feat-wiki-ast-entity-compiler — AST-to-Entity Markdown Compiler

> Ola 32 — Core. Labels: `ola32`, `wave-32`, `wiki`, `compiler`, `brain2`

## Current State (measurable)
- CodeGraph extracts symbols (`code_graph::types::Symbol`) with Tree-sitter parsers, but these symbols stay confined inside SQLite (`symbols` table in `code_graph.db`).
- There is no compiler that converts these symbols into readable, linked Markdown dossier notes in `.xavier/wiki/entities/`.
- Agents cannot browse or grep code entities in Markdown format.

## Desired State (delta)
- Create `src/wiki/compiler.rs`:
  - Implement `EntityCompiler` that takes `&[Symbol]` and call edges (`CodeEdge`) from `CodeGraphDB`.
  - For each top-level struct, trait, class, or public function:
    - Generate or update `.xavier/wiki/entities/<lang>/<symbol_name>.md`.
    - Format with YAML frontmatter:
      ```yaml
      ---
      id: "entity-uuid"
      name: "SymbolName"
      kind: "Struct"
      lang: "Rust"
      file_path: "src/path/file.rs"
      calls: ["DependencyA", "DependencyB"]
      called_by: ["CallerX"]
      wikilinks: ["[[ConceptY]]"]
      ---
      ```
    - Body includes: Signature code block, doc comments extracted from AST, and cross-reference links (`[[CallerX]]`).

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::compiler` — unit tests pass (compiles mock symbols to `.md` files in tempdir)
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] Generated markdown files validate through `parse_markdown_content`

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/compiler.rs` | Does not exist | Implement `EntityCompiler` transforming AST symbols to markdown dossiers | MED |

## DO NOT touch
- `code-graph/src/*` — owned by other issues
- `src/wiki/schema.rs` — owned by Issue 07

## Anti-Hallucination Guard
1. READ `code-graph/src/types.rs` lines 50–120 to see the exact structure of `Symbol` and `SymbolKind`.
2. Sanitize symbol names for safe filenames (replace `::`, `/`, `\` with `_`).

## Merge order
- Within wave: 8/10 · Effort: Medium (<1.5h) · Parallel with: all in Ola 32
