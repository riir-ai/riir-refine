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
- [x] T2a — (zero-build read half, DONE 2026-09-09) Fresh clone `cloudrift-ai/emmy` →
      `.raw/emmy` @ `1c058c9d31b02fe1b229a295a855e02d7e941b35` (2026-09-08, Apache-2.0
      re-verified; tip HEAD = "fused attention kernel past FlashAttention-2" — the FA
      articles' kernel landed here too, relevant for Batch 138). Clone RETAINED for T3
      quote re-verification (rm -rf at landing per the .raw discipline). Two parallel
      subagent read-lanes ran; proposals + adjudications in the Appendix below.
- [ ] T2b — (cargo-gated, folds into T3) subagent proposal lanes COMPLETE — no further
      read work needed; authoring may cite the Appendix directly.
- [x] T3 — (write half DONE 2026-09-09, commit `288f984`) Author entries
      (`entries_gpu_37.rs`, 11 rules) + batch137 standing gates
      (`batch137_rules_present` / `batch137_concepts_covered` /
      `batch137_pseudocode_parses`) + count pins (entries.len ×3 → 591,
      category table A/E/L/N, KERNEL_CATEGORIES slices + slice comments,
      mod.rs history paragraph + 591-rule doc). Quotes re-verified at the
      pin BEFORE authoring (all load-bearing lines, incl. the corrected
      featurization quote — actual text adds "2026-07-07 ... warp tier
      repeated the same drop until 2026-07-28"). Validated rustfmt-parse
      (all 3 files exit 0) + python pin-phrase check (44/44 pins present);
      COMPILE validation pending — gate RUNS are T4, post-unblock.
      rule_id-anchored edits throughout (no fuzzy anchors).
      Blog-vs-repo divergences recorded in-rule (the washes: article
      measured, repo encodes as doctrine).
- [x] T4 — (DONE 2026-09-09, `4c2e94a` + `b638e71`) GOAT: G1 batch137 gates 3/3
      by name; G2 lib default 1187/0 + kernel_heal 1518/0/1i +
      kernel_heal+latent 1635/0/2i; G3 clippy 0 ×3 all-targets + retrieval
      floors HELD byte-identical (kernel 100/100/100, blended 94.7/96.9/97.7) +
      router annotation re-pinned 88→86/131 per the Issue 020 contract (measured
      91/131 = 69.5%, was 93/131 — the dense B137 concept vocabulary attracts 2
      more generic spans to the kernel centroids, the B131 sec-centroid
      mechanism; decay recorded, never silent). T4 caught ONE stale count pin
      (router expert-coverage 580→591) + 4 PRE-EXISTING test-target clippy rot
      (batch119 ×3 + batch130 too_many_lines — the states B136's clippy never
      covered), fixed as cited drive-bys. `--corpus-stats` 591/120/49/14/31.
      `--score-bench` REMAINS — timing-gated on the p335 parity arm (numerics
      only unaffected; run when the box quiets).
- [ ] T5 — (cargo-gated) rust_perf bench-feature Pareto/alloc validator for any
      rust_perf-routed candidate; NotBenchable annotation + context_notes for
      decision-content rules (expected: MOST of this batch — the article's
      numbers are 5090/CUDA-shaped, not M3/Metal-executable).
- [x] T6 — (DONE 2026-09-09, `efe724c`) Count pins walk: corpus.rs header
      totals ×2 + category table + mod.rs lists + entries.len were T3-landed and
      compile-verified; AGENTS ×2 (domain list 580→591 + the feature-gate list
      570→591 PRE-EXISTINGLY stale two batches — repaired), README ×2 (rule row
      + tree, Batch 1-137), `.docs/11_domains/kernel_opt.md` (status row + the
      Batch 137 ledger paragraph). Grep-verified zero stale 570/575/580 prose
      counts remain.
- [x] T7 — (DONE 2026-09-09, Research 156) `.research/156_batch137_cloudrift_matmul_distill.md`
      born-complete: source + URL-shape correction, the 11-rule table, both
      adjudications, the two washes as recorded negatives, lane-intel YES with
      the per-rule split, blog-vs-repo honesty, the full gate row + the
      write-blind-compile-free lesson, the parity-arm session context;
      `.highwater` 155 → 156.
- [ ] T8 — Commit `feat(kernel_opt): batch 137: ...` + push; update
      `.distill/001_mining_queue_snapshot.md` in the same commit (lead-pair
      entry → landed; Batch 138 = FA parts 1+2 stays queued); rm -rf the
      `.raw` clone.

## Appendix — T2a subagent proposals (2026-09-09, two read-lanes @ pin `1c058c9d`)

Eleven candidates, all rule_ids grep-verified novel, all quotes file:line-verified
at the pin by the lanes; T3 must RE-verify quotes before authoring (the B31
drift discipline). All expected NotBenchable-annotated (RTX 5090/CUDA numbers;
M3 box).

**Lane A (kernel_opt structure):**
1. `smem-fill-align-chunk-not-element` (N dtype/alignment, CORRECTNESS class):
   smem slab base alignment must derive from the FILL CHUNK (16 B cp.async / TMA
   box), not the element width — a 2 B-aligned f16 slab takes 16 B writes that
   fault at 8 mod 16 (live fault, `_stage.py:310-318`; TMA pins 128 B even at
   NONE swizzle, `_stage.py:288-289,854-856`). Rider: load-side static alignment
   proof (`050_vectorize_loads.py:174-185` — fp16 n=2 NOT a freebie).
2. `software-swizzle-shift-row-stride` (A): software-fill slab XOR swizzle shift
   must derive from the ROW STRIDE, not default log2(atom) — a 128-elem fp16 row
   spans two 128 B atoms and collapses rows onto a quarter of chunk positions
   (measured 7.8-way ldmatrix conflict at D=128, vanishes at D=64;
   `_stage.py:349-362`; gate = NONE | inner==atom | !pow2, else shift;
   `sm_89`-scoped — hardware/TMA slabs excluded). Rider: the derivation is
   consumed by 3 parties (descriptor encode, TMA box split, drain mode) and
   disagreement = TMA COPY DEADLOCK (`_stage.py:336-337,862-875`).
3. `companion-slab-ring-slot-sync` (A, CORRECTNESS, live-found bug): a scale/
   sidecar slab riding the SAME depth-D ring as its codes must carry the slot-row
   offset; bare coordinate reads slot 0 while codes read slot s → a minority of
   cells scaled wrong by 2-3×, invisible in source AND most-output parity
   (`_atom.py:411-420,386-389,1172-1175`). Per-channel fragment naming is part of
   the rule.
4. `split-k-atomic-arm-legality` (E, CORRECTNESS): the atomicAdd combine arm is
   SEMANTICALLY illegal regardless of shape unless: ONE state component ·
   ADDITIVE monoid · output rounds-into-able (low-precision → deferred f32
   finalize) · projection DISTRIBUTES over the add (`_split.py:88-114`); storage
   asked BEFORE projection; over-refusal is fail-safe (`_split.py:61-71` static-K
   divisibility admission).
5. `setmaxnreg-fit-gate-elected-arrive` (E, sm_90+ ONLY): (a) emit the
   `setmaxnreg.dec/inc` pair only when raised total fits the SM register file
   (`24·prod + 240·cons ≤ 65536`, `_stage.py:911-917,967`) — the article presents
   it as free, the repo gates it; (b) consumer band syncs via NAMED barrier
   (`Sync(barrier_id=1, count=threads)`) — CTA-wide `__syncthreads()` is UB on
   the divergent role branch (`:944-945`); (c) ONE elected arrive releases the
   empty-slot mbarrier (count 1, not block_threads) + carries
   `fence.proxy.async` (`:945-950`); (d) raw `tid==0` election for mbarrier init
   in split kernels (`:957-959`).
6. `ldmatrix-pair-x2-to-x4-adjacency` (L, bit-identical positive transform):
   fuse two atom-adjacent `ldmatrix.x2` (+8 elems on one axis) into one `.x4`
   when same slab/role/swizzle, no masks, not byte slabs, barrier-free
   straight-line between; second load moves UP — an intervening read of the
   MOVED fragment blocks (WAR), read of the KEPT fragment is fine
   (`096_pair_ldmatrix_loads.py:11-24,45-46,107-136`). ⚠ ADJUDICATION: lane B
   dropped this ("owned by xor-swizzle family + positive transform"); lane A's
   distinct-from is more precise (xor-swizzle owns the TABULATION axis, this is
   the FUSION axis; positive transforms have corpus precedent, e.g.
   `vectorized-pack-load`). T3 decision: admit as rule with the fusion-axis
   distinct-from, or fold as rider on `ldmatrix-mma-layout-loads`.

**Lane B (wash/decision rungs):**
7. `ptxas-recoalesces-scalar-runs-wash` (decision/refutation): hand-vectorizing
   regular stride-1 scalar-load runs is a WASH — ptxas recoalesces at every
   deployable -O; the pass is IR-legibility only and deliberately NOT a search
   dimension (`hints=(True,)`, `050_vectorize_loads.py:34-38`,
   `search/space.py:22-24,153-159`); escape arm = irregular kernels (unprovable
   alignment, gapped strides — `:176-179`); stores symmetric
   (`080_vectorize_stores.py`) folded in-fix-space. COMPOSES with
   `vector-width-pow2-sweep` via silicon scope split (Metal/AMD keep the sweep).
8. `ptxas-reschedules-load-order-wash` (decision/refutation): hand load/
   FMA interleaving is a wash for the same reason
   (`095_interleave_loads.py:23-26`, `space.py:161-167`); escape arm = ASYNC
   issue-site placement (stays `interleave-loads-between-tile-matmuls`'s
   territory); `-Xptxas -O0` = debug only.
9. `smem-pad-transport-gated` (decision): TWO stacked gates — (1) TRANSPORT:
   under TMA never pad (descriptor swizzle replaces it, `_SWIZZLE` table
   `_tma.py:58`); under cp.async pad is live; (2) GRANULARITY: pad one 16 B
   transfer unit per row (bank-quartet break, chunks stay aligned —
   `ir/address.py:17-28` "Derived, not tuned"; "cp.async only: a TMA box deposit
   is dense") OR deliberately +1 element to DE-VECTORIZE a conflicted float4 —
   the article's 3.7× arm ("conflict-free scalar beats serialized vector");
   fill/drain contract: pad lives on `Operand.pad_cols`, "the two cannot
   disagree" (`_atom.py:910-911`). TENSION RESOLVED: composes with
   `vectorized-smem-write-no-bank-padding` (that rule owns default-layout
   vectorized STORES; this owns CONFLICTED READS under a transport key).
10. `splitk-width-shape-need-gate` (decision, STRONGEST lane intel):
    `needed = max(2·SM/free_ctas, K/√(M·N), 1)` (`search/features.py:675-686`) —
    grid fullness ALONE under-splits: emmy's own mlp_down counterexample
    (M=512,N=4096,K=14336: free_ctas saturates SMs → naive gate ranked the
    winning g8k ~9000-deep; `:679-683`); excess = pure combine waste
    (`:666-667`); paired excess/deficit features (`:687-693`). Worked 4090
    example: prefill down-proj needs ≈10-way split where occupancy-only says 2.
11. `splitk-atomic-free-contention-threshold` (decision, EPILOGUE economics):
    atomicAdd vs partials+reduce fork keyed on SPLITK·M·N contention with a
    FITTED (not hand-set) threshold (`search/prior/linear_model.py:65,84-93`);
    round-trip cost side named (`features.py:694-697`); decision-content
    integrity: the fork key must be STAMPED on candidate rows or the interaction
    silently dead-weights to atomic default (`:512-515` lesson). Legality
    preconditions shared with #4. ⚠ ADJUDICATION: #4 and #11 overlap on the
    legality set — T3 decision: two rules (legality=semantic correctness,
    contention=perf economics) cross-referenced, or one rule with two arms.

**Lane intel (both lanes agree, condensed):** YES — this batch is league-relevant.
4090 = sm_89 Ada: cp.async live, TMA/setmaxnreg/mbarrier-ring ABSENT → #9's
TMA-dead arm never fires on 4090 (pad arm live), #5 is Hopper+ only (still valid
corpus decision content), #10 strongest (prefill GEMMs are exactly the K-heavy
family; qwen down-proj shapes), #1/#2/#6 Ada-portable. #7/#8 = bench-budget
intel (don't spend league cycles on hand-vectorize/interleave toggles).
Post-merge 4090 verification runs: candidate rows for the perf league queue.

**Dropped by the lanes (mapped to owning neighbors):** CTA-swizzle GROUP_M walk
(→ `l2-grouped-tile-schedule` rider), +1 pad on cp.async float4 as standalone
(→ folded into #9), reg_depth register double-buffer (→
`register-double-buffer-operand-pipeline`), prologue-peel pipelining (→
`cpasync-multistage-tile-pipeline`), WS role split itself (→
`plane-specialization-load-compute-role-split`; #5 keeps post-split deltas),
ring-clamp/barrier-under-mask (borderline — strongest rider candidate for
`cpasync-multistage-tile-pipeline`), symbolic-K overhang masks (→
`clamp-tile-indices-to-sentinel-discard-at-write` rider), TMA descriptor scratch
liveness (→ `hazard-tracked-aliasing-scratch-allocator` rider),
drop-redundant-syncs peephole (below the bar this batch; future positive-rung
candidate), scratch-slab liveness packing (host allocator, out of kernel_opt
scope), TMA single-issuer / m16n8k16 atom / ldmatrix layout (B130/B92 owned).
