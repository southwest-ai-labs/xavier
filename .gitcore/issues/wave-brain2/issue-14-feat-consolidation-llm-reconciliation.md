# [Ola 33.14] feat-consolidation-llm-reconciliation — LLM Contradiction Reconciliation

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `consolidation`, `llm`, `contradiction`

## Current State (measurable)
- File: `src/consolidation/merger.rs:72-146`
- `merge_documents` clusters memories by cosine similarity (>0.88) and merges them by splitting on punctuation and deduplicating exact normalized strings (`combined_sentences.push(sentence)`).
- If Document A says *"Port is 8000"* and newer Document B says *"Port is 8006"*, both are concatenated together with `.join(" ")`.
- Contradictory statements are never reconciled or flagged.

## Desired State (delta)
- Create `src/consolidation/contradiction.rs`:
  - Implement `reconcile_documents_with_llm`:
    - Sends conflicting memory documents with timestamps and quality scores to LLM provider.
    - System prompt asks the model to detect contradictions, resolve them in favor of more recent/authoritative statements, and output structured JSON:
      `{ "canonical_content": "...", "contradictions_resolved": [{"old": "...", "new": "...", "reason": "..."}], "redundant_ids": [...] }`
    - If LLM provider is unavailable or fails, fall back gracefully to deterministic string dedup with a metadata flag `metadata["unresolved_conflict"] = true`.
- In `src/consolidation/merger.rs`:
  - Wire `reconcile_documents_with_llm` as the primary consolidation path when an LLM provider is active.

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib consolidation::merger::tests` — passes
- [ ] `cargo test -p xavier --lib consolidation::contradiction::tests` — passes with mock LLM provider
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/consolidation/contradiction.rs` | Does not exist | Implement LLM reconciliation engine + structured contradiction schema | MED |
| `src/consolidation/merger.rs` | Heuristic string concatenation only | Add async LLM reconciliation bridge with deterministic fallback | MED |

## DO NOT touch
- `src/consolidation/reflection.rs` — reflection module unchanged
- `src/tgd/*` — TGD optimization loop unchanged

## Anti-Hallucination Guard
1. READ `src/consolidation/merger.rs` lines 70–146 before writing.
2. The fallback path must remain 100% offline and deterministic.

## Merge order
- Within wave: 4/9 · Effort: Medium (<1.5h) · Parallel with: all in Ola 33
