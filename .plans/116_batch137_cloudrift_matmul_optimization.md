# 116 — Batch 137: the CloudRift "Modern GPU Matmul Optimization" distill

**Status:** PLANNED — plan-first landed 2026-09-09 under the box-measurement gate (p335 parity arm PID 90715 saturating CPU; v8 quiet-gated behind it). T0+T1 done zero-build (source verified live + dedup pre-pass below); T2+ are cargo-gated and execute post-unblock, Protocol A order.

## Source

- **Article (a):** Dmitry Trifonov, "Modern GPU Matmul Optimization"
  (`https://www.cloudrift.ai/blog/gpu-matmul-optimization`, 2026-05-29).
  ⚠ URL SHAPE: the queue snapshot's guessed `/blog/modern-gpu-matmul-optimization`
  404s ("Article not found") — the real path is `/blog/gpu-matmul-optimization`
  (verified live 2026-09-09; the B130 404 lesson re-proven at plan time).
- **Code surface:** `cloudrift-ai/emmy` (deplodock renamed → Emmy, July 2026) —
  the article drives the COMPILER PASSES (`DEPLODOCK_KNOBS` + `--ir tile/cuda -vv`
  diffs; passes t:010 tile, t:020 staging, t:040 double-buffer, t:060 async, t:080
  pipeline-peel, t:085 warp-specialize, t:070 pad, t:025 grid-swizzle; k:050
  vectorize). B130 pin was `2aaa0328`; the tip WILL have diverged (B130 watched it
  restructure) → fresh clone at mine time, pin @ then-HEAD, license re-verify
  (B130 recorded Apache-2.0), `.raw` discipline, rm -rf at landing.
- **Provenance:** blog cite-shape + repo tip dual-source (the B130 shape exactly);
  all fixtures original; quotes re-verified against the live article text already
  fetched in full (2026-09-09, this session).
- **Sibling articles:** "Learning FlashAttention the Hard Way" Parts 1+2
  (2026-07-06/08) stay QUEUED as the natural Batch 138 — do NOT fold into 137
  (the pair reservation expired when 135/136 landed; one article per batch).

## What it is (decision content — every rung measured, RTX 5090, 2048³ fp32/fp16)

A matmul optimization LADDER with measured numbers per rung (naive → 96% of
cuBLAS fp32 SIMT SGEMM; fp16 tensor-core path 105% — beats cuBLAS HGEMM because
cuBLAS ships an Ampere-era cp.async kernel: no TMA, no warp specialization).
The article self-tags situational rungs "**Specialized**" — a built-in
benchable/decision-content split that maps 1:1 onto the corpus's annotation
discipline. Rungs:

1. **Register tiling** (the 5.2× lead): per-thread FM×FN register cell grid;
   load-bound → FMA-bound; the coarse tile WINS at 17% occupancy vs the
   cell-per-thread kernel's 67% — occupancy is not the goal.
2. **Vectorized loads — WASH (negative):** nvcc auto-coalesces scalar load runs
   into LDS.128/LDG.128 at every deployable -O level (0% diff even on a
   load-bound tile); only `-Xptxas -O0` changes the machine code; nvcc still
   fails on irregular kernels (unaligned, gapped strides) — the conditional.
3. **Load interleaving — WASH (negative):** ptxas schedules loads itself;
   hand-sinking next-to-use is a wash at deployable -O.
4. **Tensor cores:** `mma.sync.m16n8k16` + `ldmatrix` atom tier, fp16 in →
   fp32 accum; inline PTX proven faster than the WMMA API; 94µs vs 262µs.
5. **Staging ladder:** smem cooperative fill (bundle sync) → double-buffer
   (phase index) → `cp.async` (sm_80+) → TMA (sm_90+, descriptor + mbarrier).
6. **Software pipelining:** prologue/main/epilogue peel, depth = buffer count;
   pays on BOTH transports (PIPELINE_STAGES).
7. **Warp specialization (situational):** producer warp + consumer warps,
   mbarrier ring, `setmaxnreg` yield/claim; "most situational — only pays for
   deep pipelines with enough warps to spare a producer"; the fp16 endgame.
8. **Bank conflicts:** broadcast = free; TMA hardware XOR swizzle; +1 smem pad
   (situational): 3.7× on the cp.async transport's four-way float4 collision —
   the pad BREAKS 16-byte alignment and the conflict-free SCALAR still beats
   the serialized vector; does NOTHING under TMA (hardware swizzle).
9. **CTA swizzle (situational):** wash on compute-bound 2048³; 5% at L2-bound
   8192³ with a low-intensity tile; only recovers what row-major wasted.
10. **Split-K (situational):** a LOSS on the square (grid already full); 7.1×
    at 128×128×16384 skinny; SPLITK=16/32 → 11× swing; atomic vs atomic-free
    two-kernel combine fork keyed on SPLITK·M·N contention.
11. Final-kernel shape notes: guarded float4 epilogue stores; setmaxnreg
    reg-file partitioning between producer/consumer warps.

## Dedup pre-pass (T1, DONE 2026-09-09 — zero-build grep over kernel_opt + rust_perf)

⚠ grep discipline: ERE (`grep -E`) required — the first pass ran BRE and every
`?`/`|` pattern silently false-emptied (the katgpt-rs `\s` false-empty class;
caught because `tma` MUST hit B130's entries_gpu_36.rs and didn't). Re-run with
`-E`; hit counts below are the corrected pass.

| concept | corpus state | nearest rule_ids (READ at mine time before ruling) |
|---|---|---|
| register tiling | NO direct rule (sole hit = incidental "register/tile schedule" prose in B129's expert-remainder rule) | `row-tile-amortize-input-load`, `shared-mem-aware-tile`, `batch-width-routed-dual-tile-twins` — tile STRUCTURE, not the per-thread register-tier cell grid |
| occupancy-vs-tile-size | theme exists, not this decision | `fold-query-tokens-for-occupancy`, `grid-collapsing-mega-kernel-occupancy-trap`, `occupancy-grid-cap-refuted-uncapped-default`, `independent-accumulator-chains-hide-load-latency-at-capped-occupancy` |
| vectorize/interleave WASH | corpus has the "how", NOT the "compiler already did it" | `vector-width-pow2-sweep`, `vectorized-pack-load`, `stride-align-vectorization-key`, `interleave-loads-between-tile-matmuls`, `interleave-independent-accumulators-only`, `coalesced-row-read`, `matvec-coalesce`, `layout-permute-for-coalescing` |
| mma/ldmatrix atom | COVERED | `mma-fragment-layout-m16n8k16`, `ldmatrix-mma-layout-loads`, `cmma-hardware-unit-over-scalar-alu`, `cooperative-tensor-mma-capability-route` (inline-PTX-over-WMMA facet may ride) |
| staging/TMA/double-buffer ladder | HEAVILY covered | `tma-single-issuer-bulk-tile` (B130), `cpasync-multistage-tile-pipeline` (B130 rider — 82.6% crossover + multistage), `register-double-buffer-operand-pipeline`, `no-blocking-sync-in-async-pipeline`, `barrier-after-async-mma-before-staged-tile-reuse`, `multi-simdgroup-shared-staged-tile`, `dequant-once-per-k-tile-into-shared-memory` |
| warp specialization | one adjacent | `plane-specialization-load-compute-role-split`, `compute-role-split-by-accumulator-pressure` — the delta candidate is the SITUATIONAL-PAYOFF gate (deep pipeline + spare warps), not the role split |
| smem pad / bank conflicts | EXISTS + a tension to adjudicate | `vectorized-smem-write-no-bank-padding` (vector writes need NO pad — the article's pad BREAKS alignment and de-vectorizes: likely COMPOSABLE decision pair, not a duplicate), `xor-swizzle-smem-static-index-tables`, `ane-bank-split-ladder`, B130 `lds-cluster-cross-warp-phase-not-static-distance` |
| CTA swizzle | HIGH dedup risk | B130 `l2-grouped-tile-schedule` (SWIZ=8 + DRAM-parity boundary) — likely rider/facet (the shape-gate: wash compute-bound vs pays L2-bound) at best |
| split-K | EXISTS | `split-k-two-pass-attention-decode-long-context`, `target-tile-split-k-deterministic-two-dispatch`, `streamk-starved-tile-grid-dispatch`, `row-parallelism-ladder-split-vs-pack` — delta candidate = the grid-fullness SHAPE GATE (loss on square / 7.1× skinny) + the atomic-vs-atomic-free fork keyed on SPLITK·M·N |
| pipeline peel | likely covered | `cpasync-multistage-tile-pipeline`, `pipelined-token-slot-ping-pong-submit-ahead`, `mid-pipeline-device-alloc-is-implicit-sync` |
| cuBLAS lagging-generation | adjacent | B130 `vendor-dispatch-verify-before-outsource` (the "suspiciously-good win = dispatcher bug" protocol) — the article's "cuBLAS ships an Ampere-era kernel" is the complementary vendor-lag facet; compose, don't duplicate |

Preliminary batch shape (rule-level dedup at mine time decides): 3-5 new rules
(register-tiling register tier; vectorize/interleave wash pair or single;
smem-pad-under-TMA gate; split-K shape gate; WS situational payoff) + 3-6
riders/facets (CTA swizzle shape gate → l2-grouped-tile-schedule; inline-PTX →
mma-fragment; vendor-lag → vendor-dispatch-verify; broadcast → xor-swizzle;
prologue-peel → cpasync-multistage). Route: **kernel_opt primary**
(entries_gpu_37.rs next), rust_perf io secondary (compiler-pass-shaped
candidates only).

**Lane-intel axis: TO ASSESS at mine time — NOT auto-does-not-fire.** This is
GEMM/attention kernel-structure content, the same family as the 4090 prefill
lane (Bench 896/897 ladder: `att_pf_mq8*`, GDN rec multi-row). Warp-spec / TMA
depth / split-K gates are candidate intel for the league's 4090 prefill arm.
B130 (same source family) recorded does-NOT-fire for host-side dispatch
verification; this batch is device-side kernel structure — record the verdict
explicitly either way.

## Tasks

- [x] T0 — Plan-first (this file); source article fetched FULL-TEXT live
      (2026-09-09), URL shape corrected, author/date verified; repo pin DEFERRED
      to mine time (tip discipline — no clone under the box gate; clone is
      zero-build and may precede T2 if convenient).
- [x] T1 — Dedup pre-pass over the CURRENT corpus (580 kernel_opt + 120
      rust_perf): table above; rule-level re-verification (READ each named
      neighbor) required at mine time BEFORE authoring — the pre-pass is
      file/grep-level, the B30 filter is load-bearing at rule granularity.
- [ ] T2 — (cargo-gated) Fresh clone `cloudrift-ai/emmy` into `.raw/`, pin @
      HEAD + license, walk the pass files the article cites
      (lower.py t-passes, backend `_tma.py`, decomposition 070_matmul.py,
      095_interleave_loads / 096_pair_ldmatrix_loads / 110_drop_redundant_syncs
      — note: article cites pass NUMBERS that may have moved with the Emmy
      rename; re-locate by name); spawn 2 parallel subagent proposal lanes
      (kernel_opt + compiler-pass/rust_perf), re-verify every quoted line.
- [ ] T3 — (cargo-gated) Author entries + the batch137 standing syn-parse gate
      (`batch137_pseudocode_parses` + presence/needles incl. NotBenchable
      membership where annotated); rule_id-anchored edits ONLY (the B136
      fuzzy-anchor lesson); blog-vs-repo divergences recorded in the research
      note (the B130/Research-152 discipline).
- [ ] T4 — (cargo-gated) GOAT: G1 parse/syn all new rules; G2 lib tests default
      + kernel_heal(+latent); G3 clippy 0 both states + retrieval floors
      (kernel 100/100/100, blended 94.7/96.9/97.7) + router annotation re-pin
      per the Issue 020 contract (floor re-derived from measured, never
      silently weakened) + `--score-bench` append + `--corpus-stats`.
- [ ] T5 — (cargo-gated) rust_perf bench-feature Pareto/alloc validator for any
      rust_perf-routed candidate; NotBenchable annotation + context_notes for
      decision-content rules (expected: MOST of this batch — the article's
      numbers are 5090/CUDA-shaped, not M3/Metal-executable).
- [ ] T6 — Count pins walk: corpus.rs header totals ×2 + category table +
      mod.rs lists + entries.len + without_spec/annotated membership + AGENTS
      ×2 + README ×2 + `.docs/11_domains/` (count + Batch-137 paragraph +
      index row). Repair any stale pin encountered (the B135 NotBenchable-drift
      precedent).
- [ ] T7 — Research note `.research/NNN_batch137_*.md` (next free number per
      `.research/.highwater`); lane-intel verdict recorded; negative results
      (the washes) recorded even when they land as riders.
- [ ] T8 — Commit `feat(kernel_opt): batch 137: ...` + push; update
      `.distill/001_mining_queue_snapshot.md` in the same commit (lead-pair
      entry → landed; Batch 138 = FA parts 1+2 stays queued); rm -rf the
      `.raw` clone.
