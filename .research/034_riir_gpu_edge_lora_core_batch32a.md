# Research 034 — riir-gpu Batch 32a: the edge_lora core cluster (edge_module + types + training + adaptive_width)

**Date:** 2026-08-15
**Batch:** 32a (corpus 217 → **228**)
**Sources:** `riir-ai/crates/riir-gpu/src/edge_lora/edge_module.rs` (506 lines) + `types.rs` (627) + `training.rs` (953) + `adaptive_width.rs` (650) — the host-side training orchestrators, first half of the edge_lora module. Two parallel mining subagents (A: edge_module+types, B: training+adaptive_width) + synthesizer re-verification of every load-bearing quote (all 25+ quotes verified exact against the files).

**Split note:** the snapshot's suggested 32a/32b split — this is 32a (core). 32b remains: `topology_trainer.rs` (1468), `sleep_consolidation.rs` (615), `arena_eval.rs` (604), `{go,bomber,fft}_self_play.rs` (730+521+506), `cross_game_{edge,real}.rs` (504+224), `chain_commit.rs` (394), `sigmoid_gate.rs` (394), `mod.rs` (249).

## The headline: a per-episode construction storm, root-caused in its own comments

`AdaptiveWidthTrainer::train` (adaptive_width.rs L280-320) builds per EPISODE: (1) a full `.clone()` of every active edge's weight matrices, (2) a fresh `TopologyTrainer` whose zero-initialized edges are immediately discarded (`sub_trainer.edges = active_edges;`), (3) a fresh `ForwardCache`, (4) a fresh `EdgeGradients::zeroed`. Only **3 distinct schedules exist** (easy/medium/hard, fixed active-index sets) — all schedule-keyed state could pre-build once at construction. The root cause is documented in the source itself (L216-218): `forward_cached` checks `cache.edge_src_hidden.len() != self.edges.len()` EXACTLY, so sub-topology use forces per-episode caches — the comment even notes "sub uses fewer slots" is safe, i.e. the exact-length check is stricter than the safety argument requires. Filed as **riir-ai Issue 683** together with: silently ignored `TrainingConfig` fields (`weight_decay`/`grad_clip_norm`/`per_layer_lr_scale` — unlike the sibling trainer), misreported `mean_weighted_loss` (unweighted copied into the weighted field), the `zeroed()` doc causal error (B=0, not A=0), a release-mode panic path in `route_into`, and the additive-aggregation stale-out footgun.

## Rules landed (11)

| Rule | Cat | Source | Concept |
|---|---|---|---|
| `caller-owned-split-scratch` | A | edge_module.rs L173-199 + L301-313 | Hot-path fn takes ONE caller-owned scratch, `split_at_mut` carves intermediate+output (borrow checker proves disjoint), returns a borrowed view; trait-shaped allocating paths quarantined as documented cold adapters. |
| `inplace-gate-scale-at-funnel-bottleneck` | C | edge_module.rs L203-212 | The mandatory sigmoid gate runs IN PLACE on the rank-wide intermediate AND absorbs the broadcast scale — rank ops, not out_dim, no wide second pass. Placement-of-work at the funnel's narrowest point. |
| `sign-branch-stable-sigmoid` | B | edge_module.rs L182-186 + L328-337 | Branch on sign so exp's argument is always <= 0; the naive form overflows f32 exp at x < -88.7 (inf slow paths en route to the correct limit 0). Matches katgpt-rs `latent_to_raw_scalar` — the house gate-math style. |
| `incremental-multi-part-hash-no-concat` | A | types.rs L476-487 + L381 | BLAKE3 `hasher.update` per tensor replaces materializing `a \|\| b`: the streaming update IS the concatenation — zero per-step allocation on the commitment path (runs after every gradient step). |
| `bool-fields-over-bitset-hot-reads` | L | types.rs L35-45 + L69-84 | The REGIME BOUNDARY of packing: per-tick-READ flags stay as bools (decode ALU dominates, 4 direct 1-byte loads share a cache line); pack only when bytes STREAMED dominate. The inverse of the packed-weights rules. |
| `per-layer-grad-norm-lr-equalization` | B | training.rs L388-392 + L441-449 + L510-513 | Per-step per-layer grad-norm LR scaling `min(max_norm/layer_norm, cap)` combats a measured ~1000× vanishing gradient; zero-norm → 1.0 rails (never NaN); edge→layer map precomputed at construction. |
| `width-invariant-superset-training` | D | adaptive_width.rs L2-6 + L20-23 + L407-409 | ONE superset weight set trained across difficulty-sampled widths serves every inference budget (anytime-inference, zero extra storage); falsifiable gate: width-1 retains >= 95% of width-4. |
| `superset-sized-scratch-across-variants` | A | adaptive_width.rs L202-210 + L216-218 | Scratch sized ONCE for the statically-known worst-case variant; every smaller variant fits. Caveat from the source: consumers with EXACT-length checks force per-variant allocs — relax to prefix-view semantics. |
| `active-index-grad-scatter-subset-backward` | H | adaptive_width.rs L44-46 + L329-335 | Backward in dense sub-space + precomputed active-index scatter into the full buffer; inactive params zero BY CONSTRUCTION — no masks, no zeroing pass, no atomics. The host-side shape of MoE backward. |
| `inplace-grad-accumulate-deferred-postscale` | B | training.rs L297 + L325-336 | Accumulate RAW per sample into one buffer; ONE post-loop reciprocal sweep (`inv_batch`) — 1 division total, not batch×params; decay/clip reuse the same in-place sweep shape. |
| `fuse-loss-and-output-grad-one-pass` | C | training.rs L312-320 | Loss scalar + analytic output-gradient (`2·reward·diff`) from the SAME diff in one sweep — the standard fused-loss-grad shape; write into persistent scratch, not a per-sample Vec. |

Category deltas: A 32→35, B 34→37, C 22→24, D 32→33, H 9→10, L 7→8. Total **228** across 15 categories.

## Bugs found (filed as riir-ai Issue 683 — all unfixed)

1. **H1** per-episode construction storm (headline above) — P1 perf.
2. **H1b** per-episode `grad_out` alloc + dead zero-init in BOTH hot loops (training.rs L314 + adaptive_width.rs L310).
3. **H2** silently ignored `TrainingConfig` fields in `AdaptiveWidthTrainer::train` (L346-358 — plain `sgd_step(..., config.lr)`; the sibling trainer honors all three).
4. **H3** `mean_weighted_loss: mean_loss` (L367-370) — the reward-weighted sum is never accumulated.
5. **M1** `zeroed()` doc teaches the wrong init cause (A=0 → sigmoid(0)=0.5; the output is zero because B=0).
6. **M2** `route_into` sizes scratch by the caller's `out.len()` but `matvec_b` indexes by stored `out_dim` — release-mode panic on `out.len() > out_dim` (only a `debug_assert_eq!` guards it).
7. **M3** shape-mismatch leaves `out` untouched — in an additive-aggregation context a failing edge contributes the buffer's prior contents, not zero.
8. **M4** dead full-size buffers (`bwd_scratch_full`, `cache_sub`) held for the trainer's lifetime while the live path pays H1.

## Candidates dropped (dedup, with the covering rule)

- **8-lane manual unroll in `matvec_a`/`matvec_b`** (edge_module.rs L228-268) — `unroll-fixed-inner` (Batch 1 family); the chunked `while j + 8 <= in_dim` shape adds nothing.
- **`bytemuck::cast_slice` zero-copy reinterpretation standalone** — `alignment-gated-zero-copy-view`; only the streaming-hash aspect was kept (Rule 4).
- **Struct field-order padding comments** (edge_module.rs L75-78, types.rs L125-128+) — size-NEUTRAL in these actual structs (EdgeModule 16 B either order; EdgeLoraWeights 48 B either way): the doc claims an optimization that measurably does nothing. Not corpus-worthy; noted as trivia.
- **edge→layer metadata precompute** (training.rs L207-209) — folded into `per-layer-grad-norm-lr-equalization` (standalone it would duplicate `prebuilt-bind-groups-fixed-buffer-combos`).
- **Config-gated skip of decay/clip passes** — `optional-bias-branch`.
- **Reciprocal hoist standalone** — folded into `inplace-grad-accumulate-deferred-postscale` (float-side sibling of `strength-reduce-div-pow2`, too thin alone).
- **Two-branch sigmoid from subagent B** — same concept as subagent A's `sign-branch-stable-sigmoid` (MERGED, one rule).
- **`#[cold]` error paths** — real but below the bar.
- **`SimpleRng` seed-0 remap + upper-24-bit float extraction** (adaptive_width.rs L713) — correct hygiene, but randomness quality is out of kernel_opt scope.

## GOAT gate

- **G1 (parses):** `batch32_pseudocode_parses` — 11/11 rules' ref_shape + opt_pattern syn-parse. PASS.
- **G2 (tests):** kernel_opt lib suite **306 tests PASS** (0.15s) incl. the new batch32 gates (`batch32_rules_present`, `batch32_concepts_covered`, `batch32_pseudocode_parses`); default suite 201 PASS.
- **G3 (no-regression):** clippy clean in BOTH compiled feature states (default + kernel_opt; fixed 13 doc-markdown warnings in the new batch entry before commit). `--all-features` is broken by the SIBLING's uncommitted WIP (`rust_perf/bench.rs:335` `alloc_axis` — 0 occurrences at HEAD, 161 uncommitted insertions; verified via `git show HEAD:... | grep -c`), NOT by this batch. Pre-existing relative to my commits.
- **G4:** unchanged (no hot-path additions — corpus is static data).
- **G5 (corpus):** 228 flat → G5-drafter reduction maintained.

## State

`riir-gpu` mined: ~58 of ~200 files. Next candidates: **Batch 32b** — the edge_lora self-play/arena/consolidation half (`topology_trainer.rs` 1468L, `sleep_consolidation.rs` 615L, `arena_eval.rs` 604L, `{go,bomber,fft}_self_play.rs`, `cross_game_{edge,real}.rs`, `chain_commit.rs`, `sigmoid_gate.rs`, `mod.rs`). `kernels/` = mostly `.wgsl`, mine the Rust wrappers only. `ternary_deltanet_gpu_forward.rs` still BLOCKED on sibling WIP.
