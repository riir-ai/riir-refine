# Research 156 — Batch 137: the CloudRift "Modern GPU Matmul Optimization" distill

**Status:** RECORD (2026-09-09, M3 box, CPU-only under the p335 parity-arm gate; landed with Plan 116)

## Source

- Article: Dmitry Trifonov, "Modern GPU Matmul Optimization"
  (`cloudrift.ai/blog/gpu-matmul-optimization`, 2026-05-29). ⚠ URL shape: the
  queue snapshot's guessed `/blog/modern-gpu-matmul-optimization` 404s — the
  real path is `/blog/gpu-matmul-optimization` (the B130 404 lesson re-proven at
  plan time). Fetched full-text 2026-09-09; every rung measured on RTX 5090 at
  2048³ (naive → 96% of cuBLAS fp32 SIMT SGEMM; the fp16 tensor-core endgame
  105% — beats cuBLAS HGEMM because cuBLAS ships an Ampere-era cp.async kernel:
  no TMA, no warp specialization).
- Code: `cloudrift-ai/emmy` @ `1c058c9d` (Apache-2.0) — the SAME repo as Batch
  130 (pin `2aaa0328`), this batch takes the matmul-article surface the TMA
  batch left unmined: the article drives the COMPILER PASSES (`DEPLODOCK_KNOBS`,
  passes t:010 tile … t:085 warp-specialize, k:050 vectorize). Fresh clone, HEAD
  == pin, LICENSE verified, `.raw/emmy` removed at landing. `deplodock` renamed →
  Emmy, July 2026.

## What landed (11 rules, corpus 580 → 591; A +5, E +4, L +1, N +1; entries_gpu_37.rs)

| Rule | Cat | One-line decision content |
|---|---|---|
| `smem-fill-align-chunk-not-element` | A | smem cooperative fill aligns to the transport CHUNK (16 B transfer unit), not the element — the fill loop's granularity is the transport's contract |
| `software-swizzle-shift-row-stride` | A | software (non-TMA) swizzle = a row-stride shift in the index math, the hardware-XOR-swizzle's software twin for pre-sm_90 transports |
| `companion-slab-ring-slot-sync` | A | **the correctness lead**: a ring-riding scale slab must carry the slot ROW OFFSET in its drain index — a bare within-tile coordinate reads slot 0 while the codes read slot s and a subset of cells is scaled by the wrong block; invisible in emitted source AND in aggregate parity (found live, _atom.py:411-425); keys on HOW the companion is produced (copied/ring-riding vs compute-filled single-buffer) |
| `smem-pad-transport-gated` | A | smem padding is TRANSPORT-gATED two ways: under TMA never pad (hardware descriptor swizzle replaces it — `_SWIZZLE` table); under cp.async pad one 16 B unit per row (bank-quartet break, chunks stay aligned) OR +1 element to deliberately de-vectorize a conflicted float4 (the article's 3.7× arm: conflict-free scalar beats serialized vector); composes with `vectorized-smem-write-no-bank-padding` (that owns default-layout vectorized STORES, this owns CONFLICTED READS under a transport key) |
| `ptxas-recoalesces-scalar-runs-wash` | N | **recorded WASH**: nvcc auto-coalesces regular stride-1 scalar-load runs into LDS.128/LDG.128 at every deployable -O (0% diff even load-bound); hand-vectorizing is IR-legibility only and deliberately NOT a search dimension (`hints=(True,)`); escape arm = irregular kernels (unaligned, gapped strides); composes with `vector-width-pow2-sweep` via silicon scope split (Metal/AMD keep the sweep) |
| `ptxas-reschedules-load-order-wash` | N | **recorded WASH**: ptxas schedules loads itself — hand load/FMA interleaving is a wash at deployable -O; escape arm = ASYNC issue-site placement (stays `interleave-loads-between-tile-matmuls`'s territory); `-Xptxas -O0` is debug-only |
| `split-k-atomic-arm-legality` | E | **the legality lead**: a split-K DIRECT atomicAdd arm is SEMANTICALLY illegal for whole carrier families under a four-condition conjunction — ONE state component / ADDITIVE monoid / output the partials round into once per CTA (F16/BF16 refuse; f32 workspace + deferred finalize instead) / a projection that DISTRIBUTES over the add; storage asked BEFORE the projection; the only divergence is OVER-refusal (a withheld atomic row leaves the deferred finalize, never a crash); every refusal names the alternative |
| `splitk-width-shape-need-gate` | E | **the strongest lane intel**: the split WIDTH is a two-floor predicate `needed = max(2·SM/free_ctas, K/√(M·N), 1)` — grid fullness ALONE systematically under-splits (emmy's own mlp_down M=512,N=4096,K=14336: occupancy-only ranked the winning golden ~9000-deep); worked league-relevant example: our down-projection shape wants ~10-way where occupancy-only says none |
| `splitk-atomic-free-contention-threshold` | E | the PERF fork: atomic vs atomic-free two-kernel combine keyed on SPLITK·M·N contention — legality decides WHETHER the atomic arm may run, contention decides WHEN it should (the pair cross-referenced both directions) |
| `setmaxnreg-fit-gate-elected-arrive` | E | `setmaxnreg` reg-file partitioning between producer/consumer warps with the fit gate + elected-arrive discipline; `sm_90`+ ONLY (Hopper) — recorded as lane intel, not landed for our sm_89 |
| `ldmatrix-pair-x2-to-x4-adjacency` | L | two atom-adjacent `ldmatrix.x2` (+8 elements on one axis) fuse into one `.x4`/`.x4.trans` bit-identically: same slab/ldm/b_trans/swizzle (the per-lane XOR swizzle COMMUTES with the paired lane map), no masks, not byte slabs, barrier-free straight-line between; the second load moves UP — an intervening read of the MOVED fragment blocks (WAR), read of the KEPT fragment is fine; idempotent |

## The two authoring adjudications (T3)

1. **split-K legality + contention economics = TWO cross-referenced rules**, not
   one: `split-k-atomic-arm-legality` (semantic legality — whether the atomic
   arm may run at all) and `splitk-atomic-free-contention-threshold`
   (performance fork — when the atomic-free combine should win). The article
   covers split-K's performance fork only (7.1× skinny, loss on square); the
   legality fork is the repo's contribution and is unit-testable anywhere
   (monoid/projection classifiers are pure functions).
2. **`ldmatrix-pair-x2-to-x4-adjacency` admitted as a FULL rule** over lane B's
   kill ("owned by xor-swizzle + positive transforms have precedent"): xor-swizzle
   owns the TABULATION axis of the address transform, this rule owns the FUSION
   axis (two loads → one instruction); the paired lane map must COMMUTE with the
   swizzle — a legality condition, not a duplicate. Positive-transform corpus
   precedent exists (`vectorized-pack-load`).

## Negative results (the washes) — recorded even though they landed as rules

Two of the eleven rules are REFUTATIONS by design (`ptxas-recoalesces-scalar-runs-wash`,
`ptxas-reschedules-load-order-wash`): the article's negative rungs are the
decision content — the corpus's "how to vectorize/interleave" rules stay correct
for Metal/AMD, but on CUDA the compiler already does it, and encoding "the
compiler already did it" is the missing half. A third rung (CTA swizzle) was a
wash on compute-bound 2048³ but pays ~5% at L2-bound 8192³ — folded as the
shape-gate facet of B130's `l2-grouped-tile-schedule` family rather than a new
rule (dedup pre-pass verdict held).

## Blog-vs-repo notes

- The article's measured numbers (94µs vs 262µs tensor-core endgame, 7.1×
  skinny split-K, 11× SPLITK swing, 3.7× smem-pad arm, 5% CTA-swizzle at
  8192³) are 5090/CUDA-shaped — NotBenchable on the M3; rules carry the honest
  split (structural gates unit-testable anywhere, outcome numbers
  source-reported).
- T5 verdict: rust_perf N/A — all eleven rules routed kernel_opt (no
  compiler-pass-shaped host candidates). The article's "Specialized" self-tags
  map onto the corpus's benchable/NotBenchable annotation discipline 1:1.

## Lane-intel axis: YES (explicit verdict)

Device-side kernel-structure content — the 4090 prefill lane's family
(Bench 896/897 ladder). Verdict: **YES, with a per-rule split**: the
`splitk-width-shape-need-gate` two-floor predicate is directly applicable to
the league's GEMM-shaped prefill arms (our down-projection shape is the worked
example); `smem-pad-transport-gated`'s cp.async arm is live on sm_89;
`split-k-atomic-arm-legality` constrains any future split-K decode arm; the
`setmaxnreg` arm is Hopper+ only — recorded, NOT landed (4090 is sm_89 Ada).
B130 (same source) recorded does-NOT-fire for its host-side dispatch content;
the two batches compose rather than contradict.

## Gates (T4, run post-unblock 2026-09-09)

batch137 gates 3/3 by name (presence / concepts-covered / syn-parse) · lib
default 1187/0 · kernel_heal 1518/0/1i · kernel_heal+latent 1635/0/2i ·
retrieval floors HELD byte-identical (kernel 100/100/100, blended 94.7/96.9/97.7
— sixth consecutive batch) · router annotation re-pinned 88→86/131 per the
Issue 020 contract (measured 91/131 = 69.5%, was 93/131 = 71.0% at B130/B136 —
the dense split-K/ldmatrix/swizzle concept vocabulary attracts 2 more generic
Rust spans to the kernel centroids; the B131 sec-centroid mechanism, kernel
flavor; decay recorded, never silent) · clippy 0 ×3 all-targets ·
`--corpus-stats` 591/120/49/14/31.

The zero-build authoring discipline (T3 write half landed blind under the box
gate: rustfmt-parse + 44/44 test-pin phrase check BEFORE any cargo run) met the
compiler exactly once, in T4, and the compile caught ONE stale count pin
(router expert-coverage 580→591) + zero concept errors — the write-blind-
compile-free discipline working as designed. Clippy also surfaced 4 PRE-EXISTING
test-target warnings (batch119/B130 rot the B136 clippy states never covered) —
fixed as a cited drive-by (`b638e71`).

## Session context

The batch ran under the box-measurement gate (p335 parity arm PID 90715, the
other lane's training run): T0–T3 executed ZERO-BUILD; T4's functional gates ran
under the parity arm deliberately — the arm is `cpu_only_forward`, single-core,
and numerics-only (loss/ppl/grad are wall-clock-independent), so compilation and
count/assert gates alongside it corrupt nothing; only the timing-sensitive
`--score-bench` awaited the quiet box. Sibling session landed `ab4d4be` (Issue
079 soft-gate mine-and-sync offer) mid-batch on the same branch — disjoint
files, no conflict.

Record: Plan 116 / queue snapshot (B137 entry). Batch 138 = FlashAttention
Parts 1+2 stays queued (do NOT fold).
