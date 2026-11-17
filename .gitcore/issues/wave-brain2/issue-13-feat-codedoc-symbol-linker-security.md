# [Ola 33.13] feat-codedoc-symbol-linker-security — Privacy-Preserving Code-Doc Symbol Linking

> Ola 33 — Security. Labels: `ola33`, `wave-33`, `security`, `codegraph`, `memory`

## Current State (measurable)
- File: `src/memory/sqlite_vec_store/store_impl.rs:1856-1858`
- Line 1856 enforces:
  `if !super::at_rest::is_explicitly_public(metadata) { return Ok(Vec::new()); }`
- Because agent conversation sessions and project memories are marked `internal` or unclassified, symbol derivation is 100% aborted.
- Conversely, lines 1872–1880 previously fell back to tokenizing ANY 4-letter word from user text, causing massive bloat in `memory_symbol_links` and leaking plaintext user wording into unencrypted tables.

## Desired State (delta)
- In `src/memory/sqlite_vec_store/store_impl.rs`:
  - Eradicate the naive 4-letter word fallback entirely (delete lines 1872–1880).
  - Allow symbol linking for local-workspace records if and only if candidate words match verified symbols in `code_graph.db`'s FTS index:
    - Never write raw user words to `memory_symbol_links`.
    - Only write verified `symbol_id` foreign keys that exist in `code_graph.db`.
  - Maintain the security gate for classified cloud records, but allow local workspace matching when operating in single-user mode.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib memory::sqlite_vec_store::store_impl::tests::private_row_never_gets_raw_word_links` — passes
- [ ] `cargo test -p xavier --lib memory::sqlite_vec_store::store_impl::tests::private_row_never_links_through_the_codegraph` — passes
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/memory/sqlite_vec_store/store_impl.rs` | Lines 1845–1880 either abort or insert all 4-letter words | Strict symbol-existence validation + eliminate naive word fallback | HIGH (Security) |

## DO NOT touch
- `src/memory/sqlite_vec_store/at_rest.rs` — encryption invariants unchanged
- `code-graph/src/*` — owned by other issues

## Anti-Hallucination Guard
1. READ `src/memory/sqlite_vec_store/store_impl.rs` lines 1835–1890 and `at_rest.rs` lines 830–850 before writing.
2. Under no circumstance may plaintext user words be inserted into `memory_symbol_links`.

## Merge order
- Within wave: 3/9 · Effort: Medium (<1.5h) · Parallel with: all in Ola 33
