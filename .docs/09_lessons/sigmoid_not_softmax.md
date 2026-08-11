# Sigmoid, not softmax

Per the global AGENTS.md rule: "Use sigmoid not softmax".

## Where sigmoid is used

### L1 drafter confidence

```rust
let mean = output.iter().sum::<f32>() / output.len() as f32;
let confidence = sigmoid(mean);  // ∈ (0, 1)
```

The confidence is a scalar in `(0, 1)`, derived from the mean of the
matvec output via logistic sigmoid. This is a per-fix confidence score,
not a probability distribution over fixes.

### Why not softmax?

Softmax produces a probability distribution (sums to 1) over a set of
candidates. This would be appropriate if we were selecting ONE fix from
N candidates probabilistically. But the clippy-healing pipeline:

1. **Selects deterministically** (FNV-1a hash → index modulo
   `candidates.len()`). The confidence is metadata, not the selection
   mechanism.
2. **Validates independently** (L2 checks each drafted fix separately).
   The confidence doesn't determine whether a fix is applied — the
   pruner does.
3. **Doesn't need normalization**. A confidence of 0.62 for fix A and
   0.58 for fix B are both valid independent scores. Softmax would
   artificially suppress one to boost the other.

## The sigmoid function

```rust
#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}
```

- Output ∈ `(0, 1)` — never exactly 0 or 1 (asymptotic).
- Monotonic: higher input → higher output.
- Bounded: no overflow risk for reasonable inputs.

## Pattern alignment

This matches the sibling-repo convention: `riir-agents` uses
`dot + sigmoid` for latent-space scoring; `riir-rag` uses sigmoid for
relevance scores; `riir-engine` HLA uses sigmoid for emotion scalars.
The entire 7-repo stack avoids softmax except where a true probability
distribution is needed (which is rare).

## See also

- [`../03_drafter/clippy_ternary_drafter.md`](../03_drafter/clippy_ternary_drafter.md) — where confidence is computed
