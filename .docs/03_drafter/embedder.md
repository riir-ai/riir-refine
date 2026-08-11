# Embedder — BLAKE3-deterministic modelless embedding

Produces a fixed-size `[f32; 8]` latent direction vector from arbitrary
`(lint_name, code)` input using only BLAKE3 hashing + deterministic
byte-folding. No weights, no training, no GPU.

## Properties

- **Deterministic**: same `(lint_name, code)` → identical `[f32; 8]` bits.
- **Bounded**: every component ∈ `[-1.0, 1.0]` (via `u32 → normalized f32`).
- **Diverse**: BLAKE3's avalanche property ensures distinct inputs
  produce near-orthogonal embeddings (good for dot-product retrieval).
- **Alloc-free**: no heap allocation; writes into a caller-provided array.

## Algorithm

```text
1. BLAKE3 hash of (lint_name \0 code) → 32 bytes
2. Split 32 bytes into 8 little-endian u32 values
3. Normalize each u32 [0, 2^32-1] → f32 [-1, 1]:
     ((u32 as f64) / (u32::MAX as f64)) * 2.0 - 1.0
   (f64 intermediate avoids precision loss: u32::MAX as f32 rounds up
   to 4294967296.0, which would push the ratio slightly > 1.0. The
   final f64→f32 cast is intentional.)
```

## API

```rust
pub const EMBED_DIM: usize = 8;
pub type Direction = [f32; EMBED_DIM];

// Alloc-free: writes into caller's array
pub fn embed_into(lint_name: &str, code: &str, out: &mut Direction);

// Convenience: returns a fresh [f32; 8] (stack alloc, no heap)
pub fn embed(lint_name: &str, code: &str) -> Direction;

// Embed a (lint, buggy, fixed) triple — for fix trajectory memory
pub fn embed_fix_into(lint_name: &str, buggy_code: &str,
                      fixed_code: &str, out: &mut Direction);
```

## Why 8 dimensions?

Matches `riir-agents`' `ModellessReasoner` direction-vector size (8-dim)
so the `dot + sigmoid` scoring pattern is directly applicable. Small
enough for zero-alloc stack arrays; large enough for diverse embeddings
(BLAKE3's avalanche ensures near-orthogonal directions).

## Pattern source

Borrows the `ModellessEmbedder` pattern from `riir-rag` (riir-ai):
BLAKE3 + deterministic projection + bounded output. Reimplemented
locally (~30 LOC) because riir-clippy cannot depend on riir-ai
(boundary rule).

## Test coverage

- `embed_is_deterministic` — same input → identical bits (G1)
- `embed_different_input_different_output` — distinct lints differ
- `embed_same_lint_different_code_different_output` — same lint, different
  code differs
- `embed_components_in_valid_range` — all components ∈ `[-1, 1]`, no NaN
- `embed_into_matches_embed` — the two APIs agree
- `embed_fix_distinguishes_fix_variants` — different fixes embed distinctly

## See also

- [`clippy_ternary_drafter.md`](clippy_ternary_drafter.md) — L1 uses
  BLAKE3 hashing (not this embedder directly, but the same pattern)
- [`../09_lessons/modelless_first.md`](../09_lessons/modelless_first.md) —
  why modelless embedding is preferred over learned embeddings
