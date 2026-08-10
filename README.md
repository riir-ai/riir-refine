# riir-clippy

Clippy-healing drafter+pruner pipeline: modelless ternary drafter × rust-analyzer validator × adaptive LoRA fallback.

## Status

**POC — Phase 0 (rule distillation foundation)**

The clippy-healing pipeline that distills clippy rules into a ruliology-style
search space, drafts fixes via a modelless ternary SIMD matvec, and validates
with a rust-analyzer constraint pruner. Heavyweight LoRA is the last-resort
fallback, not the co-equal path.

## Architecture

```text
Clippy corpus (BLAKE3-committed freeze/thaw)
  (lint_name, buggy_code, fixed_code, lint_group) tuples
         │
         ▼
L1 — MODELLESS DRAFTER (PlasmaPath ternary SIMD matvec)
  Frozen clippy-fix direction vectors (.bits files)
  Template fallback when no weights loaded
  Zero-alloc hot path
         │
         ▼
L2 — VALIDATOR (always-on)
  Native rust-analyzer (default) — parse + type-check + clippy re-run
  WASM rust-analyzer (opt-in, deferred on rubrc maturity)
         │
         ▼
  [ validated clippy fix ]
```

## Modelless-first mandate

The primary path (L0-L3) is entirely modelless — no training, no backprop:
- **L0**: rule distillation (named rules + bounded fix-space)
- **L1**: ternary SIMD matvec drafter (frozen direction vectors)
- **L2**: rust-analyzer validation
- **L3** (opt-in): ruliology Pareto-front candidate search

The LoRA fallback (L4) is deferred to riir-train and is the conservative last
resort, not a co-equal path.

## Usage

```rust
use riir_clippy::{ClippyCorpus, ClippyTernaryDrafter, RustAnalyzerPruner};

// Build the corpus from (lint, buggy, fixed) entries
let corpus = ClippyCorpus::new(entries);

// Draft a fix (fallback mode = corpus template lookup)
let mut drafter = ClippyTernaryDrafter::fallback();
let candidates: Vec<_> = corpus.lookup("needless_return").collect();
let drafted = drafter.draft_fix("needless_return", buggy_code, &candidates)?;

// Validate with rust-analyzer
let pruner = RustAnalyzerPruner::new();
let result = pruner.validate(&drafted.code, "needless_return");
if result.is_valid() {
    // Apply the fix
}
```

## References

- [Proposal 034](../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) — the design argument
- [Plan 331 Phase A](../riir-train/.plans/331_lora_rust_coder_candidates_consolidated.md) — Gemma-2 r16 production (LoRA fallback base)
- [Plan 333](../riir-train/.plans/333_bitnet_ternary_moe_neuro_symbolic_poc.md) — Ternary-Bonsai-27B (LoRA fallback primary base)
- [RustAssistant](https://arxiv.org/abs/2412.16358) (Microsoft Research, 2024) — 75% prior art ceiling
- [Proposal 011](../katgpt-rs/.proposals/011_rust_swe_bench_latent_space_via_wasm_pruner.md) — WASM rustc pruner (rubrc)

## License

MIT OR Apache-2.0
