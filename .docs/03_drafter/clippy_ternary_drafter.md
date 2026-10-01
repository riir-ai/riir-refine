# ClippyTernaryDrafter — modelless ternary SIMD matvec drafter

The L1 layer of the clippy-healing pipeline. Proposes fixes using frozen
ternary direction vectors via a multiplication-free SIMD matvec from
`katgpt-core`. Zero-alloc hot path.

## Two modes

### Fallback mode (no weights)

```rust
let mut drafter = ClippyTernaryDrafter::fallback();
```

Returns the first corpus entry for the lint. Confidence = 0.5 (lower
than the ternary path). Source = `CorpusTemplate`.

### With weights (L1 hot path)

```rust
let weights = katgpt_core::TernaryWeights::quantize_from_f32(&[...], rows, cols);
let mut drafter = ClippyTernaryDrafter::with_weights(weights);
```

Runs the ternary SIMD matvec:

1. **BLAKE3 hash** `(lint_name \0 buggy_code)` → 32 bytes of
   high-entropy seed.
2. **Cycle** the 32 hash bytes through `input_scratch` (cols may be > 32),
   normalizing byte `[0,255]` → `f32 [-0.5, 0.5]` (zero-centered input).
3. **`simd_ternary_matvec`** → `output_scratch` (multiplication-free:
   ternary weights ∈ {-1, 0, +1} mean the matvec is add/sub/skip, not
   multiply).
4. **Confidence**: `sigmoid(mean(output))` → scalar in `(0, 1)`.
   Never softmax (per AGENTS.md rule).
5. **Candidate selection**: FNV-1a fold over output scores → index modulo
   `candidates.len()`. Same deterministic reduction as
   `quest_grammar::plasma_draft::generate_with_weights`.

## Zero-alloc hot path

`draft_fix` reuses two pre-allocated scratch buffers:

- `input_scratch` — sized to `weights.cols`, allocated at `with_weights`
  time.
- `output_scratch` — sized to `weights.rows`, allocated at `with_weights`
  time.

The G4 alloc-free gate verifies capacity stability across 1000 calls.
`load_weights()` resizes the buffers in place (clear + resize, never
shrink the allocation).

## API

```rust
pub struct ClippyTernaryDrafter {
    weights: Option<katgpt_core::TernaryWeights>,
    input_scratch: Vec<f32>,
    output_scratch: Vec<f32>,
}

impl ClippyTernaryDrafter {
    pub fn fallback() -> Self;
    pub fn with_weights(weights: katgpt_core::TernaryWeights) -> Self;
    pub fn load_weights(&mut self, weights: katgpt_core::TernaryWeights);
    pub fn is_fallback(&self) -> bool;
    pub fn input_scratch_capacity(&self) -> usize;
    pub fn output_scratch_capacity(&self) -> usize;
    pub fn draft_fix(&mut self, lint_name: &str, buggy_code: &str,
                     candidates: &[ClippyFixEntry]) -> Option<DraftedFix>;
}
```

## DraftedFix

```rust
pub struct DraftedFix {
    pub code: String,           // the proposed fixed code
    pub confidence: f32,        // sigmoid(mean(output)) ∈ (0, 1)
    pub source: DraftSource,    // CorpusTemplate or TernaryMatvec
}

pub enum DraftSource {
    CorpusTemplate,   // L0 — no weights
    TernaryMatvec,    // L1 — frozen direction vectors
}
```

## Pattern source

Mirrors the `TernaryDraftModel` pattern from
`riir-games-quest/quest_grammar/plasma_draft.rs` but specialized for
clippy lint fixes. Uses `katgpt_core::TernaryWeights` for the matvec.
Reimplemented locally (~200 LOC) because riir-refine cannot depend on
riir-ai (boundary rule).

## GOAT gate results

| Gate | Target | Measured | Status |
|---|---|---|---|
| G1 (determinism) | bit-identical | bit-identical | ✅ PASS |
| G2 (latency) | p50 < 10µs | **p50 = 2.250µs** | ✅ PASS |
| G3 (no-regression) | L0 fallback unchanged | CorpusTemplate, conf=0.5 | ✅ PASS |
| G4 (alloc-free) | capacity stable | 256→256, 64→64 | ✅ PASS |

See [`.benchmarks/001_goat_consolidated.md`](../../.benchmarks/001_goat_consolidated.md).

## See also

- [`embedder.md`](embedder.md) — the BLAKE3 embedding used by L1
- [`../04_pruner/rust_analyzer_pruner.md`](../04_pruner/rust_analyzer_pruner.md) — L2 validates the draft
