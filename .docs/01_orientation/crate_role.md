# Crate role

**riir-refine** is a modelless-first developer tool that drafts fixes for
`cargo clippy` warnings. It is NOT a game concern, NOT a training repo,
NOT a chain repo.

## The domain test

> Does this code serve a **developer tooling** concern (clippy lint
> detection, automated fix proposal, syntax validation)?
>
> - **YES** → it belongs in riir-refine.
> - **NO** → it belongs in a different repo.

## What belongs here

- **L0 — Corpus**: BLAKE3-committed freeze/thaw of clippy lint fix examples
- **L1 — Drafter**: modelless ternary SIMD matvec proposing fixes from
  frozen direction vectors
- **L2 — Validator**: `syn`-based syntax parse check (always-on)
- **Phase 3 — Lint resolution**: real `cargo clippy` re-run (opt-in
  `clippy_verify` feature)
- **L3 — Ruliology search**: Pareto-front candidate enumeration when
  multiple valid fixes exist (opt-in `ruliology_search` feature)
- **L4 — LoRA fallback** (deferred): adaptive learning on residual lints

## What does NOT belong here

| Concern | Correct repo | Why not riir-refine |
|---|---|---|
| **Game runtime** (NPC cognition, perception, combat) | `riir-ai` | Developer tooling ≠ game runtime |
| **Training research** (LoRA SFT, GRPO, distillation) | `riir-train` | Inference tool ≠ training experiment |
| **Chain consensus** (block production, slashing) | `riir-chain` | Chain ≠ developer tool |
| **Storage** (shard persistence, KV store) | `riir-neuron-db` | Storage ≠ lint fixing |
| **Game SDK facade** | `riir-game-sdk` | Game vocabulary ≠ clippy vocabulary |

## The modelless-first mandate

The primary path (L0–L3) is **entirely modelless** — no training, no
backprop, no gradient descent. The only weight mutations allowed are:

1. **Freeze/thaw** — swapping a frozen snapshot (atomic, versioned,
   BLAKE3-checked).
2. **Frozen direction vectors** — ternary weights loaded at init, never
   mutated at runtime.
3. **Latent-space operations** — BLAKE3-deterministic embeddings,
   sigmoid gates, dot-product scoring. These update latent state, NOT
   base weights.

The LoRA fallback (L4) is deferred to riir-train and is the conservative
last resort, not a co-equal path.

## See also

- [`boundary_rationale.md`](boundary_rationale.md) — why this is a
  separate repo
- [`../09_lessons/modelless_first.md`](../09_lessons/modelless_first.md) —
  the modelless-first mandate in depth
