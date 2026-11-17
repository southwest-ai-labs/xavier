# [Ola 33.11] feat-wiki-markdown-parser-elevation — Elevate Markdown Parser to Wiki Core

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `wiki`, `parser`, `markdown`

## Current State (measurable)
- File: `src/cli/handlers/export.rs:21-114`
- `ParsedMarkdownNote` and `parse_markdown_content` are buried inside a CLI export handler, used only for offline batch import/export of Obsidian `.zip` vaults.
- They lack support for section anchors (`[[Note#Heading]]`), block anchors (`[[Note#^block-id]]`), and transclusions (`![[Note]]`).

## Desired State (delta)
- Create `src/wiki/parser.rs`:
  - Promote `ParsedMarkdownNote` and `parse_markdown_content` into the `wiki` core module.
  - Extend parser with regex and AST parsing to extract:
    1. Base wikilinks: `[[TargetNote]]`
    2. Heading wikilinks: `[[TargetNote#Section]]`
    3. Aliased wikilinks: `[[TargetNote|Display Name]]`
    4. Transclusions / Embeds: `![[TargetNote]]`
    5. Tags and metadata frontmatter
- In `src/cli/handlers/export.rs`:
  - Deprecate local helper and delegate to `crate::wiki::parser::parse_markdown_content`.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::parser` — new tests pass with anchor, alias, and embed parsing
- [ ] `cargo test -p xavier --lib cli::handlers::export::tests` — passes without regression
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/parser.rs` | Does not exist | Create rich markdown note parser with full wikilink anchor syntax | LOW |
| `src/cli/handlers/export.rs` | Lines 21–114 contain isolated parser | Delegate to `wiki::parser` | LOW |

## DO NOT touch
- `src/wiki/graph.rs` — owned by Issue 12
- `src/wiki/schema.rs` — owned by Ola 32

## Anti-Hallucination Guard
1. READ `src/cli/handlers/export.rs` lines 20–115 before writing.
2. Ensure regexes handle edge cases (nested brackets, escaped characters).

## Merge order
- Within wave: 1/9 · Effort: Small (<45m) · Parallel with: all in Ola 33
