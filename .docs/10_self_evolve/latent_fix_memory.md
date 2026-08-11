# LatentFixMemory — 3-axis retrieval over fix trajectories

Stores past fix attempts (`FixTrajectory`) and retrieves the top-K most
relevant ones for a given query. Borrows the 3-axis retrieval shape from
`riir-rag` (riir-ai), reimplemented locally.

## The three retrieval axes

### 1. Latent (dot + sigmoid)

```text
score = sigmoid(dot(embed(query), trajectory.direction))
```

"Similar lint + code pattern" signal. Uses the BLAKE3-deterministic
embedder (`embed_into`) for the query direction, and each trajectory's
cached `direction` field (computed once at insert time).

### 2. BM25 (simplified)

```text
if query.lint_name_hash == trajectory.lint_name_hash:
    score += BM25_BOOST
```

"Same lint, different code" signal. Uses a pre-computed `lint_name_hash`
(BLAKE3-derived `u64`) for O(1) matching — avoids per-trajectory string
comparison in the retrieval hot path.

A full BM25 over code tokens would add complexity for marginal POC gain;
the latent axis already captures code similarity.

### 3. Graph (deferred)

Out of scope per Issue 004 — deferred until G5 passes with latent + BM25
only (it did). Will revisit if a concrete use case for graph-based
retrieval (e.g., "fixes that depend on other fixes") materializes.

## FixTrajectory

```rust
pub struct FixTrajectory {
    pub lint_name: String,         // e.g. "needless_return"
    pub lint_name_hash: u64,       // BLAKE3 hash — for O(1) BM25 matching
    pub buggy_code: String,        // the code that triggered the lint
    pub fixed_code: String,        // the fix that was applied
    pub direction: Direction,      // cached [f32; 8] embedding
    pub success_count: u32,        // times this fix led to success
    pub fail_count: u32,           // times this fix failed
}
```

The `direction` field is the latent embedding of `(lint_name, fixed_code)`
computed once at `new()` time and cached for O(1) retrieval scoring.

## Alloc-free read path

The retrieve path reuses a caller-provided scratch `Vec<usize>` for the
index sort, and writes results into a caller-provided slice. The only
allocation is the `Vec<FixTrajectory>` growth on insert (amortized).

## API

```rust
pub struct LatentFixMemory {
    trajectories: Vec<FixTrajectory>,
}

impl LatentFixMemory {
    pub fn insert(&mut self, trajectory: FixTrajectory);
    pub fn retrieve(&self, query: &LatentQuery, k: usize,
                    scratch: &mut Vec<usize>,
                    out: &mut [ScoredTrajectory]) -> usize;
    pub fn record_outcome(&mut self, lint_name: &str, fixed_code: &str, success: bool);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

## LatentQuery

```rust
pub struct LatentQuery {
    pub lint_name: String,
    pub lint_name_hash: u64,   // pre-computed for O(1) BM25
    pub direction: Direction,  // pre-computed embed(lint_name, buggy_code)
}
```

The query pre-computes its embedding + hash at construction time so the
retrieval hot path is pure arithmetic (no hashing in the loop).

## See also

- [`heal_pipeline.md`](heal_pipeline.md) — how the memory is composed
- [`../03_drafter/embedder.md`](../03_drafter/embedder.md) — the BLAKE3 embedder
