# Research 093 — Batch 79 pool verdict: BAIR K-Search CUDA→MLX + Burn/cubek SOTA matmul (kernel_opt)

**Status:** DISTILLED — pool verified at upstream heads, awaiting batch dispatch (owner-directed stop: pool only, no corpus edits). Plan 037 holds the tasks.

Found via the idle-loop online-search pool (the dispatched unit after Plan 035/Batch 77).
Two sources cleared the bar after dedup + subagent source-verification; the rest of the
search pool is drops (recorded §5). **No rust_perf crossover qualified this round** — the
Rust-perf searches returned the known listicle class (Reintech/Khimananda/oneuptime, all
prior drops); honest negative, Batch 79 is kernel-only. (The sibling's concurrently-landed
Batch 78, Plan 036/Research 092, covers the rust_perf lane — Datadog RTDB.)

## 1. Source A — BAIR "From CUDA to MLX: How K-Search Brings Decades of Kernel Expertise to Apple Silicon"

bair.berkeley.edu/blog/2026/07/29/cuda-to-mlx-k-search/ (2026-07-29; C.K. Wolfe, Shiyi Cao,
Gal Bloch, et al. — IBM Research + Berkeley Sky Lab). K-Search = evolutionary kernel search
(Cao et al., arXiv:2602.19128) with a CUDA→MLX translation layer. Grade **A− as a source**
(A-grade narrative + measured table; repo anchoring PARTIAL — see the provenance correction).

**Verified at repo head** (`github.com/caoshiyi/K-Search`, branch `main`, HEAD
`53c8fab9a5e8fab2c86610d24fbec5067f90e115`, last push 2026-06-01 — two months BEFORE the
article; the article's appendix script names are stale):

- The Mamba MLX task IS committed: `k_search/tasks/mlx_mamba/naive_selective_scan_fwd.py`
  (~95 lines; the sequential O(N) baseline loop at ≈L68-82:
  `x = deltaA[:,:,i,:] * x + deltaB_u[:,:,i,:]`) + `spec.py`.
- **PROVENANCE CORRECTION (load-bearing):** the associative-scan formulation is
  **prompt-seeded in the task spec**, not discovered by the search — `spec.py`
  `get_definition_text_mlx()` ≈L90-115 hands the LLM the combine
  `(a₁,b₁)∘(a₀,b₀) = (a₁a₀, a₁b₀+b₁)` + the chunked threadgroup mapping
  (`CHUNK_SIZE = NUM_THREADS * ITEMS_PER_THREAD`, one threadgroup per (batch,dim), via
  `mx.fast.metal_kernel`), and a gotcha at ~L250 notes `mx.associative_scan` does NOT
  exist — implement it explicitly. The evolved Metal kernel is NOT committed (zero
  `.metal` files in the tree; generated solutions land in gitignored
  `.ksearch-output-mlx-mamba/`). The attention kernel (exp2 trick, simd_shuffle_xor
  reduction) is blog-only — not in the repo at all.
- **Baseline side-check:** mlx-lm at HEAD is still sequential —
  `mlx_lm/models/mamba.py` `_process_sequence` ≈L122-142 runs the per-token Python loop
  (`ssm_step` = exactly h_t = ā_t·h_{t-1} + b̄_t). The article's community-baseline
  characterization holds at head; the ~20× prefill numbers (mamba-370m f16, M1 Max:
  5,751 vs 329 tok/s @L=512 … 6,743 vs 339 @L=4096; decode 152 vs 116, flat) are
  blog-only — no repo benchmark to check against.

### Keep-candidate A1 — `first-order-affine-recurrence-associative-scan` (G; A-grade lead)

A first-order affine recurrence h_t = a_t·h_{t-1} + b_t IS an associative operator over
pair-space: (a₂,b₂)∘(a₁,b₁) = (a₂a₁, a₂b₁+b₂) (the 2×2 matrix [[a,b],[0,1]] monoid without
the padding row) — so the whole sequence evaluates by parallel prefix scan in O(log N)
dependent steps instead of O(N). **Fills a real corpus gap**: the recurrence family has
`chunkwise-parallel-recurrence` (chunk split, O(N/C) depth) + `cumsum-gate-to-decay-matrix`
(materialize for matmul) + `reverse-running-decay-weight` (dematerialized streaming) — grep
for scan/monoid/associative/prefix-scan across kernel_opt returns ZERO hits. The scan is a
fourth, distinct strategy, and the recognition rule ("affine recurrence ⇒ associative
operator exists") is portable decision content.

- **Regime note (beyond the source, encode in the rule):** the combine cost picks the
  strategy — scalar/small-state recurrences (SSM per-channel pairs) have cheap combines →
  scan; large state-matrix recurrences (DeltaNet/GDN d×d) combine via matmul → minimize
  combine COUNT via chunking. Cross-references our Issue 734 negative on chunked-GDN.
- **REL-ERROR GATE REQUIRED (pre-filter):** tree-order combination reorders floating-point
  association → outputs are NOT bit-identical to the sequential form; equivalence gate on
  rel-error over adversarial sequences (the ported-numeric MANDATE shape), stated in the
  rule text and the ref/opt pattern comment.
- Provenance: `k_search/tasks/mlx_mamba/spec.py` L90-115 (combine, prompt-seeded) +
  `naive_selective_scan_fwd.py` L68-82 (baseline) + the blog Table 1 (numbers, blog-only).
  Rule text must say "spec-seeded, search-realized" — never "independently discovered".

### Dedup kills from Source A (external-validation riders at most)

- **exp2 trick** → `exp2-log2-base2` exists verbatim (Batch 2 popcorn, RCP_LN2). KILL as
  rule; optional rider: external validation + Apple `fast::exp2` naming.
- **simd_shuffle_xor register softmax reduction** (Listing 1 world-model node) → positive
  instance of the B22/B28 family (`warp-shuffle-intrinsics-for-sub-warp-reduction`,
  `workgroup-equals-plane-native-reduction`, `row-owned-register-blocked-recurrence`'s
  barriers-drop-to-zero). KILL.
- CUDA→Metal constraint tables (32KB vs 48KB threadgroup, HBM vs unified-DRAM bandwidth
  ratios) → ambient regime content, C+. Drop.

## 2. Source B — Burn "State-of-the-Art Multiplatform Matrix Multiplication Kernels"

burn.dev/blog/sota-multiplatform-matmul/ (2025-07-18; Louis Fortier-Dubois + Nathaniel
Simard). Grade **A−** (deep engineering article, our own GPU substrate's upstream).
**Verified at repo head with a PROVENANCE CORRECTION: `tracel-i/cubecl-matmul` no longer
exists — the engine moved to repo `tracel-ai/cubek`, crate `cubek-matmul`, HEAD `c2323a2`
("fix promote (#559)", 2026-08-26, by the article's co-author), layout restructured
`src/components/…` → `src/multi_level/components/…`.** All five mechanisms verified at
current head:

### Keep-candidate B1 — `plane-specialization-load-compute-role-split` (E; B+/A−)

VERIFIED: `crates/cubek-matmul/src/multi_level/plane_flow.rs` (~240 lines) —
`PlaneFlowCounts{main_flow, load_only}`, `PlaneFlowPartitionRule::{MainFlowOnly,
LoadOnlyFirst{load_only}, LoadOnlyLast{main_flow}}` (comptime), runtime cube enum
`PlaneFlowPartition` with `is_load_plane()/is_compute_plane()` — a runtime branch on plane
id (`UNIT_POS_Y`) vs a comptime-baked threshold, roles kept warp-uniform via
`plane_broadcast`. Dispatch: `multi_level/components/global/multi_stage/
double_buffer_execution.rs` — `read_first`/`execute_current_and_read_next`/
`execute_last_and_write_results` all branch `if !rule.is_load_plane()` → compute planes run
`acc.mma_partition`, load-only planes run `execute_whole_job`. Algorithm wiring:
`multi_level/routines/batch/specialized.rs` + `specialization/{roles,specializer,config}.rs`.
Decision content: dedicate planes to load vs compute (register pressure is
accumulator-dominated on compute planes; load-only planes raise occupancy without adding
accumulator registers) + the caveat **never specialize WITHIN a plane (branching); only
across planes** + the regime note (the blog: Specialized underperforms on Metal — "less
low-level plane control"). Zero corpus hits for plane/warp role specialization (all
"specializ" hits are JIT/dtype/enum kernel-variant specialization).

### Keep-candidate B2 — `vectorized-smem-write-no-bank-padding` (A; B+)

Refutation-type rule (the corpus values these: powi-vs-powf, filter-count-to-any,
occupancy-grid-cap-refuted). The folk advice (pad smem to dodge bank conflicts — the
LU/QR/Cholesky lineage) is UNNECESSARY when writes are vectorized: a line-sized store
naturally distributes transactions across banks. VERIFIED: vectorized global→smem in
`multi_level/components/global/read/strategy/sync_full_tilewise.rs`
(`load_and_store_vector`: read `Vector<EG,NG>`, store
`stage.as_slice_mut::<NS>()[offset] = Vector::cast_from(v)`); ZERO pad/bank/skew fields in
`multi_level/stage/stage_memory/{config,layout,memory,swizzle}.rs`. **Evolved nuance to
encode:** an optional explicit swizzle NOW exists (`SwizzleMode::{None(default), B32, B64,
B128}` — "Matches the base TMA functionality"), i.e. the refutation holds for the default
and the swizzle is the modern complement, not padding.

### Keep-candidate B3 — `ordered-lhs-skip-smem-fragments-as-second-buffer` (A/D; B)

VERIFIED: `multi_level/components/global/multi_stage/ordered/matmul.rs` (~260 lines) with
the struct doc **verbatim**: "Uses double buffering with two shared memory buffers for Rhs,
but only one for Lhs: the second 'buffer' for Lhs is the fragments themselves. For this to
work, the Lhs reader planes must compute using only the data they have loaded themselves."
+ runtime guard "In Ordered lhs loading cannot be outside of main flow" (Lhs reader advances
1×k-step, Rhs 2×k-step) + `OrderedTilingOrder` in `stage_memory/layout.rs` (per-plane
contiguous tile groups: "tiles needed for a given k iteration stored contiguously").
**Dedup tension (resolve at authoring):** the skip-smem-for-single-consumer half is
adjacent to `smem-staging-gated-by-cross-thread-reuse` (B35); the NEW halves are (i)
fragments-as-second-buffer (register tiles double as the ping-pong slot) and (ii)
ordering-replaces-synchronization (per-plane ownership discipline eliminates inter-plane
sync). If the authoring collapses to B35's fix shape, drop honestly (the T5-precedent).

### Keep-candidate B4 — `interleave-loads-between-tile-matmuls` (D; B−)

The asynchrony-induction finding: double buffering only overlaps when the next load's
instructions sit BETWEEN tile-matmul compute in the instruction stream — abstracting loads
away from compute (clean separation) removes the overlap. VERIFIED:
`multi_level/components/global/multi_stage/event_listener.rs` (~300 lines) —
`DoubleBufferingEventListener` implements `StageEventListener::on_event` over
`StageEvent::{Begin, TileMatmulCompleted, Finish}`, injecting Lhs/Rhs load tasks between
tile matmuls ("allowing data for the next tile to be loaded while the current tile is being
computed"); `EventLoadingMode::{Relaxed, Ordered}`. Related family: CODA
register-phase epilogue (B29) — do work while data is in registers. May fold into B3 as
one Ordered-pipeline rule at authoring time.

### Secondary (optional, owner call at authoring)

- `sm-divisor-cube-allocation` (C+) — VERIFIED as `cubek-std/src/cube_count/hypercube/
  sm_allocation.rs`: `SmAllocation::{Exact, Full, Ratio}` via descending divisor
  enumeration ("120 cubes, 16 SMs → 4 SMs × 30 cubes"); the blog's 19/46 SM examples are
  article-only. Thin but real decision content.
- `tensor-core-opaque-store-smem-bounce` (B−) — blog-only (not repo-verified this pass):
  the TC cooperative store writes row-major; when global layout differs, bounce through
  smem for deterministic re-read. Adjacent to B44's fold-tile-transpose-into-dequant-scatter.
- Double-buffer allocation note (VERIFIED, correction): ONE contiguous region
  `Shared::new_aligned_slice(num_stages * stage_size, align)`, `with_buffer_index` selects
  SEQUENTIAL HALVES — the article's "interleaved halves" phrasing is wrong; record as a
  rider/note if it doesn't earn a rule.

### Dedup kills from Source B

- **Swizzle zigzag cube dispatch for L2** → positive instance of `l2-grouped-tile-schedule`
  (popcorn + B72 rider). VERIFIED renamed at head: `cubek-std/src/cube_count/hypercube/
  global_order.rs` `GlobalOrder::{RowMajor, ColMajor, SwizzleRow(u32), SwizzleCol(u32)}` +
  `#[cube] fn swizzle(index, num_steps, step_length)`. KILL (rider at most).
- k-divisibility bound-check avoidance → adjacent to clamp-tile-sentinel / ragged-chunk-pad
  / tail-guard family. Drop.

## 3. Ranked pool (expected corpus value)

| # | Rule (tentative id) | Cat | Grade | Provenance (verified) |
|---|---|---|---|---|
| 1 | first-order-affine-recurrence-associative-scan | G | A | K-Search spec.py L90-115 + naive L68-82 + blog Table 1 (numbers blog-only; spec-seeded) |
| 2 | plane-specialization-load-compute-role-split | E | B+/A− | cubek plane_flow.rs ~240L + double_buffer_execution.rs + specialized.rs @ c2323a2 |
| 3 | vectorized-smem-write-no-bank-padding | A | B+ | cubek sync_full_tilewise.rs + stage_memory/* (zero pad) + SwizzleMode nuance |
| 4 | ordered-lhs-skip-smem-fragments-as-second-buffer | A/D | B | cubek ordered/matmul.rs ~260L verbatim + OrderedTilingOrder |
| 5 | interleave-loads-between-tile-matmuls | D | B− | cubek event_listener.rs ~300L (may fold into #4) |
| opt | sm-divisor-cube-allocation / opaque-store-bounce | M/C | C+/B− | sm_allocation.rs / blog-only |

Yield estimate: 4–5 rules + 1–2 riders. Batch 78 (sibling) owns rust_perf; this batch is
kernel_opt-only — corpus-disjoint.

## 4. Pre-filter gates (the dispatch requirements)

- **Rel-error gate:** applies to #1 (association-order change ⇒ NOT bit-identical; gate on
  rel-error + adversarial sequences incl. 2^n crossings, the ported-numeric MANDATE).
  #2–#5 are structural (syn-parse + bench-compile gates; no numeric gate).
- **Retrieval-floor pre-check:** every keep-candidate's concept vocabulary greps ZERO-hit
  against the current corpus (scan/monoid/associative; plane specialization/load planes;
  bank padding; ordered Lhs; event listener) — no crowding risk; floors are ratio-based
  and have held through A→379 growth. Re-measure at GOAT per Protocol A.

## 5. Pool drops (with reasons)

ISO-Bench (arxiv 2602.19594 — agent-eval benchmark, not an optimization-technique source;
STRATEGIC: external validation surface for heal agents, see §6) · FireQ (arxiv 2505.20839
— mixed-width GEMM dedup-adjacent to our quant families; RoPE pre/post scaling is
quant-calibration domain, other repo) · Ollama MLX blog (snapshot-at-branch-points for
irreversible state: mechanism thin, qualitative evidence; NVFP4/fusion content dedup-
adjacent) · Metal Medium deep-dives + contracollective Metal-vs-MLX (tutorial/comparison
grade) · ruvector PDX gist (self-promotional, SoA dedup) · polyfill2-rs SIMD JSON (B74
dedup) · ik_llama.cpp (fork last synced 2024-08; the ggml-family vein was just mined by
B77 — re-probe after upstream movement) · ai-muninn 4-bit-2080Ti (tutorial-grade regime
content) · vLLM/SGLang comparison pieces + Cloudflare/Pingora infra stories (not
mechanism-level for our domains).

## 6. Strategic findings (not corpus — for the proposal owners)

- **K-Search's world model** (persistent decision prefix-tree; nodes carry action +
  difficulty + impacts{memory_bandwidth, register_pressure, compute_hw_fit} + rating +
  confidence; Insert/Update/Prune; stagnation-window backoff) is a direct design pattern
  for the KAT heal loop (riir-clippy Proposal 005) and Issue 048's GP acquisition layer
  (ordering candidates by input-space uncertainty) — the impacts triple is exactly a
  multi-axis prior over candidate edits. The CUDA→MLX "translation layer = concept
  mapping tables + hard constraints + reusable assertions" is ALSO the shape of a
  cross-domain heal-corpus transfer (clippy wisdom → rust_perf suggestions).
- **ISO-Bench** (54 curated optimization tasks with hard+soft metrics and the
  quadrant framework separating lucky wins from true success) is an off-the-shelf
  external eval for any heal/fix agent — worth a Proposal-005 reference.

## 7. Numbering collision note

This pool was dispatched as "Batch 78 / plan 036"; the sibling's Datadog RTDB verdict
(Research 092 / Plan 036 / Batch 78, rust_perf) landed on origin MID-SEARCH with live
rust_perf WIP in the shared tree. Push-wins tie-breaker applied: theirs keeps 036/092/78;
this pool renumbered to Research 093 / Plan 037 / Batch 79. Corpus surfaces disjoint
(theirs: rust_perf corpus.rs/bench.rs/examples; ours: kernel_opt corpus + tests.rs gates).
