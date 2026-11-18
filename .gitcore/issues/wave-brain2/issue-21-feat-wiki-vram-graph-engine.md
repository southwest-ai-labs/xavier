# [Ola 33.21] feat-wiki-vram-graph-engine — GPU VRAM Resident Graph & Tensor Retrieval Engine

> Ola 33 — Core. Labels: `ola33`, `wave-33`, `wiki`, `vram`, `gpu`, `tensor`, `brain2`

## Current State (measurable)
- Standard graph traversals and vector lookups query disk-based SQLite tables row-by-row.
- Multi-hop graph navigation (e.g. blast radius or 3-hop caller/callee context) incurs disk I/O and deserialization latency (15–50 ms).
- No GPU acceleration exists for real-time Graph-RAG navigation or fast dot-product ranking across thousands of wiki nodes.

## Desired State (delta)
- Create `src/wiki/gpu_vram.rs`:
  - Implement `WikiVramEngine`:
    - Packs node embeddings into contiguous `f16` / `f32` buffers in memory (CPU or GPU pinned).
    - Compresses wiki edge adjacency into a Compact Sparse Row (CSR) index (`row_offsets: Vec<u32>`, `column_indices: Vec<u32>`, `edge_weights: Vec<f32>`).
    - Provides bitmask filter vectors (`u64` words) for domain, language, and kind filtering.
    - Implements GPU/SIMD dot-product batch scoring:
      - Given a query vector (e.g. 768-dim), compute cosine similarities across all resident entity embeddings.
      - Given top-$k$ seed entities, perform $N$-hop CSR graph traversal in $<0.15$ ms without disk access.
    - Fallback gracefully: if CUDA / GPU runtime is unavailable, execute via optimized SIMD CPU matrix multiplication.
    - Exposes `load_from_storage(storage: &WikiStorageEngine) -> Result<Self>`.
    - Memory footprint for 50,000 entities is strictly bounded ($\approx 76.8$ MB for embeddings + 2 MB for CSR graph $< 100$ MB total).

## Acceptance Criteria (command-verifiable)
- [ ] `cargo test -p xavier --lib wiki::gpu_vram` — passes with CSR graph traversal and batch vector similarity tests
- [ ] `cargo clippy --all-targets -- -D warnings` — 0 warnings
- [ ] `cargo fmt --check` — clean
- [ ] 2-hop graph traversal across 1,000 nodes executes in $<1$ ms

## Files to Modify
| File | Current state | Change | Risk |
|---|---|---|---|
| `src/wiki/gpu_vram.rs` | Does not exist | Implement `WikiVramEngine` with CSR graph representation and batch tensor similarity | MED |
| `src/wiki/mod.rs` | Missing `pub mod gpu_vram;` | Export `gpu_vram` module | LOW |

## DO NOT touch
- `src/wiki/storage.rs` — owned by Issue 20
- `src/wiki/schema.rs` — schema types remain stable

## Anti-Hallucination Guard
1. Use Compact Sparse Row (CSR) layout for zero-allocation cache-coherent graph adjacency.
2. In the absence of an active CUDA device, use parallel CPU rayon matrix ops to guarantee 100% portability.

## Merge order
- Within wave: 11/11 · Effort: Medium (<2h) · Parallel with: all in Ola 33
