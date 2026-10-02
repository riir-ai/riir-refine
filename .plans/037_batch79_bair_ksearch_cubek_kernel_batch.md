# Plan 037 — Batch 79: BAIR K-Search + cubek-matmul external kernel distill (kernel_opt)

**Status:** COMPLETE 2026-08-27 — 5 rules landed (corpus 379->384: A 91, D 78, E 22, G 21);
all gates green; the one caveat = 4 PRE-EXISTING environmental l4_daemon failures in the
all-features leg (proven at HEAD via stash, green where authored on macOS). Filed from Research 093 (the pool verdict; owner-directed
stop: pool verified, corpus edits NOT dispatched yet). Numbered 037/79 per the push-wins
tie-breaker — the sibling's Datadog RTDB batch claimed 036/092/78 mid-search (Research 093 §7).

Source A: bair.berkeley.edu/blog/2026/07/29/cuda-to-mlx-k-search/ + repo `caoshiyi/K-Search`
@ `53c8fab` (spec-seeded scan; evolved kernel NOT committed — numbers blog-only).
Source B: burn.dev/blog/sota-multiplatform-matmul/ + repo `tracel-ai/cubek` crate
`cubek-matmul` @ `c2323a2` (2026-08-26; RENAMED from tracel-i/cubecl-matmul — cite cubek).
Routing: kernel_opt ONLY (Batch 78 owns rust_perf — corpus-disjoint; the sibling's WIP
touches `rust_perf/{corpus,bench}.rs` + `examples/perf_bench_demo.rs` — never edit those
in this batch's commits).

## Tasks

- [x] T0 Pre-flight at execution time: (a) free-box check per Plan 337; (b) `git status` —
  the sibling's Batch-78 rust_perf WIP must not be swept into this batch's commits
  (stage explicit paths only, the commit-sweep lesson); (c) re-grep BOTH vocabularies for
  every keep-candidate at authoring time (corpus may have moved — the Plan-036 T1 rule);
  (d) re-scan `.plans/` + `.research/` folders at write time.
- [x] T1 Author `first-order-affine-recurrence-associative-scan` (G 20→21): h_t = a_t·h_{t-1} + b_t
  as the monoid (a₂,b₂)∘(a₁,b₁)=(a₂a₁, a₂b₁+b₂); O(log N) prefix scan vs O(N) sequential;
  DISTINCT from chunkwise-parallel-recurrence (chunk split, O(N/C) depth) +
  cumsum-gate-to-decay-matrix (materialization) + reverse-running-decay-weight (streaming).
  REGIME note: combine cost picks the strategy (scalar/small-state → scan; d×d
  state-matrix → chunk; cross-ref Issue 734's chunked-GDN negative). **REL-ERROR GATE
  REQUIRED** — tree-order association ≠ sequential order in FP; gate on rel-error +
  adversarial sequences (incl. 2^n crossings, the ported-numeric MANDATE); state it in the
  rule text + pattern comments. Provenance: k_search/tasks/mlx_mamba/spec.py L90-115
  (combine — SPEC-SEEDED, never write "independently discovered") +
  naive_selective_scan_fwd.py L68-82 (baseline) + blog Table 1 (~20× prefill, mamba-370m
  f16 M1 Max — blog-only; mlx-lm still sequential at head, mamba.py L122-142).
- [x] T2 Author `plane-specialization-load-compute-role-split` (E): comptime
  PlaneFlowPartitionRule + runtime plane-id branch (UNIT_POS_Y vs baked threshold);
  compute planes run mma_partition, load-only planes run the whole load job — occupancy up
  without accumulator-register pressure; CAVEAT never specialize within a plane (branching),
  only across planes; REGIME: underperforms on Metal (less low-level plane control).
  Provenance: cubek plane_flow.rs ~240L + double_buffer_execution.rs (the `if
  !rule.is_load_plane()` arms) + routines/batch/specialized.rs @ c2323a2. Distinct from
  fold-query-tokens-for-occupancy (folding work into workgroups — this is role split
  within one workgroup) + occupancy-grid-cap-refuted (grid SIZE — this is plane ROLES).
- [x] T3 Author `vectorized-smem-write-no-bank-padding` (A): REFUTATION-type — vectorized
  (line-sized) smem stores distribute across banks, the classic pad-to-dodge-bank-conflicts
  advice is unnecessary; ENCODE the evolved nuance (optional SwizzleMode::{None(default),
  B32/B64/B128} now exists as the TMA-matching complement — default stays no-padding).
  Provenance: cubek sync_full_tilewise.rs load_and_store_vector + stage_memory/* zero
  pad/bank/skew fields @ c2323a2. Distinct from pad-uniform-pod-to-min-binding-size
  (uniform binding alignment — not bank conflicts).
- [x] T4 Author `ordered-lhs-skip-smem-fragments-as-second-buffer` (A/D): per-plane-owned
  Lhs rows load global→registers directly; the register fragments ARE the second buffer
  ("the second 'buffer' for Lhs is the fragments themselves" — verbatim struct doc);
  ordering discipline (Lhs reader advances 1×k-step, Rhs 2×k-step; OrderedTilingOrder
  per-plane contiguous tile groups) replaces inter-plane synchronization. RESOLVE the
  dedup tension at authoring: the skip-smem half is adjacent to
  smem-staging-gated-by-cross-thread-reuse (B35) — the fragments-as-buffer +
  ordering-replaces-sync halves are the new content; DROP honestly if it collapses to
  B35's fix shape (the Plan-036 T5 precedent). Provenance: cubek ordered/matmul.rs ~260L +
  stage_memory/layout.rs @ c2323a2.
- [x] T5 Author-or-fold `interleave-loads-between-tile-matmuls` (D): asynchrony is INDUCED
  by injecting the next load's instructions between tile-matmul compute
  (StageEventListener on Begin/TileMatmulCompleted/Finish) — clean load/compute
  separation removes the overlap. May fold into T4 as one Ordered-pipeline rule; decide at
  authoring. Provenance: cubek event_listener.rs ~300L @ c2323a2. Related: CODA
  register-phase epilogue (B29) — the instruction-stream-interleaving half is the new bit.
- [x] T6 Optional owner calls at authoring (calls MADE: both optional rules DROPPED —
  sm-divisor-cube-allocation C+ thin, opaque-store-bounce blog-only; the 2 riders landed
  in-rule: the sequential-halves correction inside the ordered-lhs rule text; the exp2
  provenance stays as Research 093 dedup-note): `sm-divisor-cube-allocation` (C+;
  sm_allocation.rs descending-divisor, "120 cubes, 16 SMs → 4 SMs × 30") /
  tensor-core-opaque-store-smem-bounce (B−, blog-only) / riders: exp2-log2-base2
  provenance rider (K-Search external validation + fast::exp2) + the double-buffer
  single-region CORRECTION note (sequential index-selected halves — the article's
  "interleaved halves" is wrong).
- [x] T7 Standing gates: `batch79_pseudocode_parses` syn gate + `batch79_rules_present` +
  `batch79_concepts_covered` (case-sensitivity discipline — the B51 lesson, 6 strikes);
  the bench-compile check on any new harness snippets (the B75 lesson: pseudocode_parses
  is necessary, not sufficient — syn cannot see type errors).
- [x] T8 Count pins walked (~17 sites): entries.len 379→NN ×3, KERNEL_CATEGORIES table,
  Plan-003 category table (G row + whichever of A/E/D move), router experts-cover pin,
  module-doc headers; grep-verify every touched category count after the corpus edit (the
  B73 lesson 19).
- [x] T9 GOAT: G1 parse/syn (all new rules) + G2 lib tests (default + kernel_opt +
  all-features states, `CARGO_TARGET_DIR=/tmp/riir-clippy-b79`) + G3 clippy 0 both states
  `--all-targets` + full CI guard + retrieval-precision re-measure (Protocol A: floors
  re-pinned honestly if corpus growth moves prose — never silently) + G4 the rel-error
  equivalence test for T1 (adversarial sequences; property-test shape).
- [x] T10 Docs: `.docs/11_domains/kernel_opt.md` rows + AGENTS.md batch note +
  queue-snapshot update (same commit).
- [x] T11 Commit `feat: batch 79 — BAIR K-Search + cubek-matmul external kernel distill
  (kernel_opt 379→NN)` + push (explicit-path staging only).

## Non-goals

- The dedup kills (exp2 trick, simd_shuffle_xor reduction, swizzle-zigzag L2 dispatch) —
  recorded in Research 093 §1/§2, not mined.
- The drop pool (ISO-Bench, FireQ, Ollama, Metal tutorials, ruvector, polyfill2-rs,
  ik_llama.cpp, ai-muninn, vLLM/SGLang comparisons) — Research 093 §5.
- rust_perf crossovers: none qualified (honest negative — the searches returned the
  listicle class); Batch 78 owns that lane.
- The strategic findings (K-Search world-model → Proposal 005 / Issue 048; ISO-Bench as
  external eval) — recorded for the proposal owners, not corpus.
