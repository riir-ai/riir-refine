# Modelless-first mandate

The primary path (L0–L3) is **entirely modelless** — no training, no
backprop, no gradient descent.

## What "modelless" means here

The only weight mutations allowed at runtime are:

1. **Freeze/thaw** — swapping a frozen snapshot (atomic, versioned,
   BLAKE3-checked). The corpus uses this.
2. **Frozen direction vectors** — ternary weights loaded at init, never
   mutated at runtime. The L1 drafter uses these.
3. **Latent-space operations** — BLAKE3-deterministic embeddings,
   sigmoid gates, dot-product scoring. These update latent state, NOT
   base weights. The embedder + drafter confidence use these.

## What "modelless" does NOT mean

- **Not "no computation"**: the L1 matvec is real computation (SIMD
  multiply-accumulate over ternary weights). It's just not *learning*.
- **Not "no weights"**: the L1 drafter loads frozen ternary weights
  (`.bits` files). The weights exist; they just don't change at runtime.
- **Not "no ML ever"**: the L4 LoRA fallback is explicitly deferred to
  riir-train. It's the conservative last resort, not banned.

## Why modelless-first?

1. **Determinism**: modelless primitives are bit-identical across runs
   (same input → same output). This is the G1 gate.
2. **Zero hot-path allocation**: no gradient buffers, no optimizer state,
   no training framework overhead. This is the G4 gate.
3. **Deployment simplicity**: no GPU required for inference. The entire
   L0–L3 pipeline runs on CPU in microseconds.
4. **Auditability**: modelless primitives are inspectable (you can read
   the ternary weights, the corpus entries, the parse results). Learned
   models are opaque.

## The LoRA fallback (L4)

L4 is deferred to riir-train. When it ships, it will be:

- **Opt-in** (`clippy_lora` feature flag)
- **Last resort**: only invoked when L0–L3 all fail to produce a valid fix
- **Not co-equal**: the modelless path is primary; LoRA is the fallback

This matches the user's core direction: "old-school LoRA is really
later."

## See also

- [`../01_orientation/crate_role.md`](../01_orientation/crate_role.md) — crate role + modelless mandate
- [`borrow_without_dep.md`](borrow_without_dep.md) — the pattern-borrowing convention
