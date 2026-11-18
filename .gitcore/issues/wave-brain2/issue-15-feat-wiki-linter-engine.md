# [Ola 33.15] feat-wiki-linter-engine — Living Wiki Consistency Linter Engine

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `wiki`, `linter`, `brain2`

## Current State (measurable)
- In standard RAG and current Xavier, there is no linting process to ensure documentation consistency.
- If a function is deleted or renamed in code, existing documentation continues referencing the old symbol without warning.
- Broken wikilinks (`[[NonExistentPage]]`) and orphaned concepts accumulate undetected.

## Desired State (delta)
- Create `src/wiki/linter.rs`:
  - Implement `WikiLinter` providing `lint_wiki(wiki_root: &Path, code_db: &CodeGraphDB) -> Result<LintReport>`.
  - Check 3 core integrity rules:
    1. **Broken Wikilinks:** Any `[[Target]]` where `Target.md` does not exist in `entities/` or `concepts/`.
    2. **Orphaned Pages:** Notes that have zero incoming backlinks from `index.md` or other notes.
    3. **Code-Doc Drift:** Entity pages whose `file_path` or `name` no longer exists in `code_graph.db`'s active symbols table.
  - Return structured report: `{ total_checked, broken_links: [...], orphans: [...], drift_warnings: [...] }`.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::linter` — unit tests pass (detects broken link, orphan, and drift on mock wiki directory)
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/linter.rs` | Does not exist | Implement `WikiLinter` for broken links, orphans, and code-doc drift | MED |

## DO NOT touch
- `src/wiki/graph.rs` — owned by Issue 12
- `src/wiki/prune.rs` — owned by Issue 16

## Anti-Hallucination Guard
1. READ `src/wiki/graph.rs` and `code-graph/src/db/mod.rs` for query APIs.
2. The linter must be read-only; it reports violations without mutating files.

## Merge order
- Within wave: 5/9 · Effort: Medium (<1.5h) · Parallel with: all in Ola 33
