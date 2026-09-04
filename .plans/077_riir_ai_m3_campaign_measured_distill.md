# Plan 077 — riir-ai M3 campaign measured-GOAT distill (prefill GEMM + decode attention)

Status: IN PROGRESS (plan-first per the B60 discipline) — verification lanes DONE, batch not yet opened
Date: 2026-09-04 · Box: M3 (CPU-only) · Source: riir-ai @ pin `59a172623bb3748ed49061e2202b787b4e4e3f08`
(origin/develop at allocation time; committed state only — the working tree is dirty with sibling waves)

Provenance (numbering): drafted 2026-09-04 09:22 in katgpt-rs as an untracked
`.plans/075_riir_ai_m3_campaign_measured_distill.md` — that number collided with the
tracked `075_residual_attention_simd_audit.md` (katgpt-rs Issue 724 T1) and the
"Batch 105" slot it claimed was consumed by the uzu+lily combine (this repo's Plan 075,
landed 2026-09-05 as Batch 105; Batch 106 = MTPLX residue, Plan 076, also landed).
Re-homed here 2026-09-04 (the mining domain per BOUNDARY.md / Issue 724 T1's own
arbitration) as the next free number; its BATCH number is assigned when the mining
queue opens it, not by this file.

## Motivation

The last measured-GOAT distillation from riir-ai's own kernels was Batch 63 (2026-08-20,
the ternary_deltanet CubeCL twin). Everything since — the entire M3 prefill/decode
campaign (Issues 761/764/767/768/771/773/774/779/790/800/804/805/831/843, Benches
761-858) — landed UN-MINED while the program pivoted external (B64-B104). That campaign
is the richest measured source the corpus has: every rule candidate below carries a
measured verdict (kernel TF, e2e tok/s, refutation ratios) from OUR OWN silicon.

Two-lane verification (subagents, committed-state reads at the pin, byte-accurate quotes
+ dedup adjudication vs named corpus entries) completed before this plan was authored;
their verdicts are the task set below.

## Source map (pin 59a172623)

| # | File | Content |
|---|---|---|
| S1 | `crates/riir-gpu/src/gemm_ternary_simdgroup_cubecl.rs` | Issue 768 32×32 input-reuse GEMM |
| S2 | `crates/riir-gpu/src/gemm_ternary_simdgroup_smem_cubecl.rs` | Issue 771 T1 smem 128×64 multi-simdgroup GEMM + Issue 843 T3 staged epilogue |
| S3 | `crates/riir-gpu/src/cudarc_kernels/attention.rs` | Issue 779 `verify_kf4_for` envelope gate (8 launch sites) + V-read guard |
| S4 | `crates/riir-gpu/src/qwen_attention_cubecl.rs` | Issue 831 4-chain unroll + valid_n + f16-KV refutation |
| S5 | `crates/riir-gpu/src/ternary_deltanet_gpu_forward.rs` | dispatch ladders + PREFILL_USE_TALL_GEMM |
| S6 | `crates/riir-gpu/tests/probe_831_attn_pair_time.rs` | lane-split TIE record + geometry scan + measurement discipline |

## Landing set (lane-verified)

### New rules (4, corpus 466 → 470)

- [ ] R1 `smem-envelope-gate-kernel-variant` (G) — Lane B candidate 1, DISTINCT. Derived
  capacity predicate `32*(head_dim+4) <= head_dim*33` selects the k4 smem-layout variant
  at ALL launch sites; below threshold the staging silently overflows into p_smem and
  races row-0 score writes with no barrier (hd-64 fixtures corrupt nondeterministically,
  production hd-256 safe by slack); fallback arm is bit-identical (load width only);
  predicate deterministic in head_dim → CUDA-graph capture-safe; companion dead-lane
  V-read guard. Distinct-from: B23 hardcoded-fast-path (config-match constant-folding
  vs derived correctness capacity gate), B57 pair (composes: race-structure vocabulary),
  B55/B46/B100 (smem-extent family — sizing, not variant gating). Quotes: verify_kf4_for
  2899-2918, envelope comment 1556-1572, qg 2029-2032, V guard 1715-1724.
- [ ] R2 `multi-simdgroup-shared-staged-tile` (A) — Lane A candidate 2, DISTINCT. ONE
  128×64 output tile subdivided across 8 simdgroups of a 256-thread workgroup; W+X
  staged once cooperatively, consumed via per-sg slice offsets; 2 barriers per 32-K
  window; per-sg PRIVATE result regions ("one shared tile would race across the 8 sgs'
  stores"); 26 KB of 32 KB budget; regime m≥128 && p≥64. Distinct-from (mandatory):
  B97 sibling-projections (inter-STREAM vs intra-workgroup subdivision), B35 smem-staging
  whether-gate (composes), B79 ordered-lhs (buffer-count vs subdivision), B15/B63/B98
  (same-input family), B20 cooperative staging (mechanics consumed), B28
  workgroup-equals-plane (geometry contrast), B97-tail-cull (barrier preservation).
  Signed-zero tail-skip exactness quote (lines 50-53) folded in. K_T=16 refutation cites
  the batch record NOT the file (absent at pin — lane-verified).
- [ ] R3 `independent-accumulator-chains-hide-load-latency-at-capped-occupancy` (E) —
  Lane B candidate 2a, DISTINCT. 4 chains = 4 loads in flight per thread — the one lever
  that moved multi-wave lengths (~1.8× @4096; flat @2048 single-wave); `valid_n` loop
  bound replaces the per-position guard; tree-finished ((d0+d1)+(d2+d3)) — NOT
  bit-identical to serial, band-gated. Lane-split one-plane-per-position TIED at
  production occupancy (B44's regime boundary — its rider below). Cite B20/B24 (ILP-chain
  family), B44 (+rider), B58/B80 (occupancy family), B79/B92 (pipeline-latency cousins).
- [ ] R4 `byte-cut-loses-when-plateau-is-issue-bound` (E, refutation-type) — Lane B
  candidate 2b, separate rule per the filter-count-to-any/B58 tradition. Packed f16-KV
  built, G0-gated exhaustively exact, measured 1.44-1.60× SLOWER at every length:
  the plateau is occupancy/issue-bound (114 GB/s ≪ ~400 peak) — halving bytes buys
  nothing, in-kernel decode ALU adds issue pressure the capped occupancy cannot hide;
  cross-substrate corroboration in-source (Bench-756 4090 −3.4%, CVT-competing regime).
  Regime test: measure GB/s vs peak + occupancy before ANY byte-cut lever. Re-arm on a
  mechanism change (bandwidth-bound KV path or hardware-half loads). Toggle verdict
  comment 1348-1360 is the primary quote.

### Riders (3)

- [ ] RD1 `row-tile-amortize-input-load` (entries_gpu_01.rs) += EXTERNAL IN-STACK INSTANCE
  (Issue 768): the cmma staged-GEMM form of the same mechanism — 4 M-subtiles per input
  tile (traffic ÷4, 42-63% measured input-load share, 91.2 GB/GEMM), padding regime the
  entry lacks (m≥64; measured 0.773× at m=48), the cmma bit-identity argument (execute
  order within a k-tile free across distinct accumulators, S1:938-940), the untried-axis
  note (S1:563-565).
- [ ] RD2 `smem-slot-role-reversal-needs-barrier` (entries_gpu_07.rs) += EXPLOITATION HALF
  (Issue 843 T3 staged epilogue): B57 owns the obligation (barrier at every reversal);
  the rider adds the exploitation — reuse the DEAD W staging tile as the epilogue region
  (each sg owns tile_w[sg*512..+512)), 16 stores in two 8-tile halves → 32 barriers → 4,
  overwrite-only-after-half-1's-scatter-drained contract; measured +2.5-2.8% @16K / +4.5%
  @8K, bit-identical, default ON. Quotes S2:32-42, 595-605, 708-712.
- [ ] RD3 `row-dot-split-inner-axis-block-reduce` (entries_gpu_06.rs) += CAPPED-OCCUPANCY
  REGIME COMPLEMENT (Issue 831): the corpus shape B44 prescribes (lane-split the reduction
  axis when outputs are few) was BUILT and measured a TIE with thread-per-position at every
  length at production occupancy (median 1.01-1.06) — at a workgroup-supply cap, axis
  splitting buys nothing; the lever is per-thread loads-in-flight (→R3). Quote S6:13-17.

### Recorded-not-landed (batch record only)

- Co-resident-wave dispatch-order axis swap REFUTED (−6…−7.5% everywhere; W-slab-sharing
  order correct) — S2:138-142 + doc 40-42. Adjacent to B9 swap-large-axis; one-silicon
  verdict, rider if a second instance appears.
- 4-arm dispatch ladder (8×8 / 8×32 / 32×32 / smem, hand-measured per-arm) — in-stack
  instance of B104 swept-shape-bucket-launch-tables ethos; cited in the batch record.
- Empty-partial reduction identity (S4:975-988) — B35 instance at partial-buffer grain.

## Implementation

- [ ] T1 Plan filed (this file) + `.plans/.highwater` 74 → 75
- [ ] T2 `entries_gpu_18.rs` — R1-R4, original fixtures (buggy/fixed pseudocode authored
  fresh, never copied from the source repo; quotes only in concept/gpu fields, attributed)
- [ ] T3 Riders RD1-RD3 (append-only entry-text edits; no rule_id/category changes)
- [ ] T4 mod.rs wiring (category counts, batch list, file include) + tests.rs pins
  (corpus total 466→470, expected-array A+1/E+2/G+1, sums ×2, grouped, cover-all comment,
  router.rs expert total) + mod.rs header + category headers
- [ ] T5 Gates: batch105_rules_present + batch105_concepts_covered +
  batch105_pseudocode_parses (cfg(not(kernel_verify)) per the B70 convention)
- [ ] T6 GOAT: kernel_heal lib count-identical +3 gates; default lib count-identical;
  kernel_heal+latent retrieval floors HELD (kernel self-variants, no re-pins); clippy 0
  in the three states (doc_markdown pre-empted: backtick all identifiers in //! prose)
- [ ] T7 AGENTS.md batch-105 paragraph + snapshot cycle-12 + corpus docs if they name counts
- [ ] T8 Commit + push (staged-set audit; re-scan highwaters at write)

## Non-goals

- No re-measurement (quotes are in-file comment records of the campaign's own measured
  runs — the lanes flagged this honestly; the numbers belong to Benches 774/790/804/831).
- No sibling-repo edits (riir-ai read-only at the pin).
- T0a/Plan 073 untouched (still gated: no t504_confirm/status.txt at cycle start).
