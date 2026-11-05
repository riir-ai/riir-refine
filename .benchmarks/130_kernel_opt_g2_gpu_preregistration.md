# Bench 130 — PRE-REGISTRATION: the kernel_opt GPU-host G2 re-gate (Issue 191 T6, the 2026-11-04 D1 verdict's three conditions)

**Status:** PRE-REGISTRATION LANDED 2026-11-05 (zed-m3-i191t6, lease on issue 191) — **NO TIMING RUN HAS HAPPENED.** This record pins the full bench registration BEFORE any run, per the D1 verdict ("PRE-REGISTRATION IS REQUIRED BEFORE ANY RUN — T6's three conditions"). The sources lane (a companion `.benchmarks/131_sources*.md` commit at the time it lands — number allocated fresh at write time) materializes the pinned pair sources; the timing run may not start until that commit exists and its BLAKE3 digests are recorded in the same family. The run record is a SEPARATE later bench document; nothing in this file may be amended after the first timed chunk (the T1 law: amended pre-measurement, never after).

**Claimed:** zed-m3-i191t6, 1793878648, expires 2026-11-07T11:37:28+00:00 — this record + the sources lane + the run are one lease.

## What this is (and is not)

- **IS:** the Tier-B GPU-host G2 re-gate trigger, exercised. The 2026-11-02 verdict closed the CPU-portable G2 axis with the measured disclosure (tranche median 1.0000 — "NEVER a CPU tranche re-run"); the newer 2026-11-04 delegated verdict (D1, session fe55a9d0) re-opened G2 as **the GPU-host microbench on the 4090** with the ≥ 6% median bar UNCHANGED, under the owner's greenlight ("do 8" — the exclusive 4090 rig hold is approved; sequenced after the v0.4.x publish + the 202/190/198/192 records, both now at their owner-gated rest state).
- **IS NOT:** a promotion run. Per the 2026-11-04 D2 verdict, promotion still requires BOTH G2(GPU-host) AND G5-cost, and T7 (Bench 129) measured the G5-cost fence FAIL structurally (added p50 33.127/37.306 ms vs the ≤ 1.0 ms ceiling — "the ceiling is never re-registered"). This leg prices the GPU-shaped surface; its verdict stands as evidence whichever way it lands. A GPU-host failure refuses promotion the same as a CPU one would have (T6, verbatim).

## Condition (a) — the full bench registration

### The deterministic selector (the CPU rule extended to the GPU-shaped surface, unchanged in form)

Corpus files ranked by their **G1-green rules' embedded `#[test]` token count descending, tie-break file name ascending, cap 15** — the same law as the CPU tranche (T1 bar 2), now selecting the GPU-pair surface. Selector output, computed 2026-11-05 at develop `2498cc501` via `kernel_goat_g2 --tranche` (no timing, the disclosure command only):

| # | file | green #[test] tokens | green rules | registered pair(s) |
|---|---|---|---|---|
| 1 | entries_gpu_117 | 66 | 11 | P117-A dispatch-fusion, P117-B split-K, P117-C register-tile threadgroups |
| 2 | entries_gpu_125 | 48 | 7 | P125-A operand-role swap |
| 3 | entries_gpu_111 | 41 | 7 | P111-A weight-panel smem share, P111-B head-row smem stage (sm_89 adaptation, disclosed) |
| 4 | entries_gpu_136 | 35 | 8 | P136-A fp32-pipe role split, P136-B quad-lane softmax |
| 5 | entries_gpu_138 | 34 | 7 | P138-A statistic row-split, P138-B dual-bank scratch |
| 6 | entries_gpu_134 | 26 | 8 | P134-A stream-table consolidation, P134-B drain-time partial sum |
| 7 | entries_gpu_122 | 24 | 3 | — NOT REGISTERED (no numeric pair: dispatch-scoping, zero-copy views, adoption-policy rules; the pair would measure nothing the rule names) |
| 8 | entries_gpu_110 | 23 | 4 | P110-A per-block state exchange |
| 9 | entries_gpu_123 | 20 | 3 | — NOT REGISTERED (no numeric pair: host intern table, compiler funnel, policy rule) |
| 10 | entries_gpu_137 | 20 | 6 | P137-A coalesced tile walk, P137-B branch-free codebook rank |
| 11 | entries_gpu_112 | 19 | 3 | P112-A row transcendental hoist, P112-B fused chain |
| 12 | entries_gpu_133 | 19 | 4 | P133-A smem tile cap |
| 13 | entries_gpu_135 | 18 | 5 | P135-A specialized top-k |
| 14 | entries_gpu_121 | 17 | 3 | P121-A int8 screen + rescore |
| 15 | entries_gpu_75 | 17 | 8 | P75-A resident MMA accumulator, P75-B batched prefetch unroll, P75-C partial-unroll spill bound |

**Registered: 13 families, 19 pairs.** Two families are NOT-REGISTERED with the reason in-line above (the ratified disclosure discipline — a family with no numeric mapping contributes nothing measurable and is said, never silently dropped). The per-family representative rule is named in the pair table below; the OTHER green rules of a registered family stay UNMEASURED and are disclosed as such in the run record (the tranche discipline: representative pairs, per-rule NOT-MEASURED rows).

### The pair table (the ref-vs-opt kernel pairs — pinned now, geometry included)

Every pair is **two CubeCL kernels** (`ref_naive`, `opt`) implementing the SAME mathematical result under the named rule's pattern, plus a correctness gate (see §Pair law). Geometry constants are part of the registration — the "optimization" is the named transform at the pinned geometry, never a runtime tuning pass. Sizes are fp32 unless stated.

| pair id | representative rule | ref (naive) | opt (the rule's pattern) | pinned geometry |
|---|---|---|---|---|
| P117-A | `same-input-consecutive-projections-one-dispatch-concatenated-tiles` | two launches: M×K·K×N1 then M×K·K×N2 | ONE launch, output tiles concatenated [N1+N2], input read once | M=1024 K=1024 N1=N2=512; ref tile 16×16, opt tile 16×32 |
| P117-B | `splitk-tier-ladder-per-tier-ceiling-min-partition-inputs` | one CTA-column K-loop matmul (K serial in-block) | split-K: K partitioned across 16 CTAs, per-CTA partials, one drain kernel sums once | M=N=1024 K=8192; tile 32×32, k-chunk 512 |
| P117-C | `apple10-narrow-grid-register-tile-threadgroups-per-core` | flat 1D grid, one thread per output element, row-walk | 2D register-tiled threadgroups (tile accumulator in registers, coalesced tile loads) | 2048×2048 GEMM; ref tile 1×1, opt tile 32×8 |
| P125-A | `swap-ab-decode-operand-roles-tile-legality` | C = A·B with B read column-strided (illegal-ish tile mapping) | swapped operand roles: the legal tile mapping, both streams coalesced | 1024×1024×1024 GEMM; tile 32×32 both arms |
| P111-A | `verify-row-tiles-share-loaded-weight-panel` | every row-tile block re-loads the K×N weight panel from global per tile | panel loaded ONCE per block into shared memory, all row tiles reuse it | panel 1024×128, 32 row tiles over M=4096; tile 64×128 |
| P111-B | `cluster-dsmem-shared-head-one-row-decode-attention` | per-block local reduction + global atomic combine of the head row | block-wide shared-memory staged head row, one combine (⚠ sm_89 adaptation: the rule names Apple cluster DSMEM — clusters are sm_90+; the pinned sm_89-legal mapping is block smem. Disclosed here, never hidden) | seq=2048 head_dim=128, 64 blocks |
| P136-A | `fp32-pipe-mma-role-split-biased-bf16-operands` | plain fp32 FMA dot over bf16-upcast operands | biased operand split (the pair split so the fp32 pipe does the fewer-value merges) — the CPU campaign's one 2.05× rule, now priced on its home surface | rows=512 K=4096; unroll 8 both arms |
| P136-B | `quad-lane-softmax-two-shuffle-16b-store` | scalar softmax, block max/sum via shared-memory round trips | quad-lane (sub-warp) reduction, two shuffle steps, packed 16-bit store | rows=4096 cols=512 |
| P138-A | `statistic-producer-row-split-single-threadgroup` | whole-row statistic serial-walked by one threadgroup | row-split across warps + one block combine | 4096 rows × 512 cols |
| P138-B | `dual-bank-scratch-retention-per-round-class` | single scratch buffer, per-round clear + rewrite | dual-bank ping-pong scratch (alternate rounds write the other bank; no clear pass) | rounds=256 × N=16384 |
| P134-A | `stream-table-consolidates-short-kernel-storms` | 64 tiny elementwise kernels launched back-to-back | ONE consolidated kernel over the stream table (the launch-storm law is the measured axis) | 64 ops × 4096 elems each |
| P134-B | `contraction-partials-sum-once-at-drain` | atomicAdd per partial into the global accumulator | per-CTA register partials + one drain kernel sums once | M=N=512 K=4096, 32 CTAs |
| P110-A | `attention-softmax-state-exchange-once-per-block` | running max/sum exchanged through global memory per chunk | block-local exchange in shared memory ONCE per block | seq=2048 head_dim=128 |
| P137-A | `strided-source-row-tile-element-outer-walk` | element-outer walk of a strided (transposed) source — column reads of row-major data | row-tile walk, coalesced reads + register transpose | 4096×4096 transpose-read |
| P137-B | `codebook-nearest-midpoint-rank-branch-free` | branchy nearest-centroid (data-dependent if per candidate) | branch-free midpoint rank (divergence-free argmin) | 16384 codes × 512 queries, dim=32 |
| P112-A | `per-row-transcendental-hoist-not-per-element` | expf per element inside the element loop | hoisted once per row, elements read the cached value | 512 rows × 2048 cols |
| P112-B | `precompute-chain-fused-intermediates-never-materialized` | three elementwise stages, each materialized to global | one fused kernel, intermediates in registers | N = 2^20 |
| P133-A | `head-dim-smem-tile-cap` | attention tile geometry with head_dim past the smem capacity (serializes/spills) | capped tile geometry that fits smem, two-pass over the cap | head_dim=128; ref uncapped, opt cap=64 two-pass; seq=1024 |
| P135-A | `topk-sampler-specialization-carries-general-tie-rule` | general top-k via full sort | specialized single-pass top-k carrying the SAME first-index tie rule (tie law pinned: lowest index wins, both arms) | vocab=131072 k=8 batch=64 |
| P121-A | `int8-screen-error-bound-certifies-exact-rescore` | fp32 rescore of ALL candidates | int8 screen pass + fp32 rescore of the error-bound-certified top slice (the certified slice is a pinned count, not a data-dependent one — the screen's correctness gate still asserts the same top-set) | 32768 candidates × 256 dim; top slice 256 |
| P75-A | `mma-accumulate-into-resident-accumulator-c-operand` | C read-modify-write from global per accumulation step | accumulator resident in registers, ONE writeback | M=N=1024 K=512, tile 32×32 |
| P75-B | `strided-loop-batched-prefetch-unroll-tail` | plain strided loop | batched prefetch (double-buffer) + unrolled tail | N = 2^22, stride 128 |
| P75-C | `partial-unroll-bounds-register-spill-cliff` | fully unrolled body (register spill cliff) | partially unrolled at the pinned bound (4) | window=64 conv scan over 2^20 |

*(23 named pairs across 13 registered families — several families carry 2–3; the leg median reads all measured pairs, the family median is the secondary read. PRE-RUN AMENDMENT 2026-11-05 (zed-m3-i191t6): this prose originally said "21 named pairs" — an under-count of the table's own 23 rows; corrected per the amendment record in [Bench 131](131_kernel_opt_g2_gpu_sources.md) §Pre-run amendments, which also pins the sources + digests.)*

### The harness law (unchanged from the CPU G2 where the law is box-agnostic)

- **Interleaved A/B:** per pair, `(ref chunk, opt chunk)` interleaved **PAIRS = 15** times; per-pair speedup = `t_ref / t_opt`; the pair verdict is the **median** across the 15. A ratio of two sequential arms measures the box — the interleave is the defense (T1 bar 2 verbatim; the sequential-AB audit's own lesson).
- **WARMUP = 3** per arm before the timed chunks; chunk budget calibrated to ≥ 2 ms wall each (the CPU harness's `TARGET_CHUNK_NS` law) so `Instant`-class timers resolve; a device sync bounds every timed chunk (no async bleed).
- **Deterministic inputs:** fixed seeded generation per pair (the seed is a constant in the sources commit), identical buffers handed to both arms; **loud zero:** inputs AND results go through `black_box`; a zero-time or dead-result chunk FAILS the pair, never passes it.
- **Correctness gate BEFORE timing:** opt's output must equal ref's within the pair's pinned tolerance (fp32 pairs 1e-3 relative L-inf unless the table narrows it; int8-screen P121-A additionally asserts the certified top-SET equality) — **a wrong kernel cannot win; a correctness-gate failure is a pair FAIL recorded per-pair, never a pass.** The modelless-gain law applies verbatim: a perf gain on a wrong answer is not a gain.
- **Posture:** `--release`, CubeCL **cuda** runtime, target **sm_89**, autotune DISABLED (the geometry is pinned in the table; a tuning pass would make the "win" a tuner artifact), fixed per-pair launch geometry from the table.
- **Stack:** standalone `cubecl` (crates.io) runner materialized from the sources commit — **zero dependency changes to riir-refine itself** (no root-manifest touch, no dev-dep bloat: the runner's sources live in the bench record family and are compiled in a temp dir on the 4090, BLAKE3-verified against the record before any compile). The exact crate versions resolve at the sources lane and are digest-pinned there.

### n and floors (condition (b) — pinned before running)

- **PAIRS = 15** interleaved chunks per measured pair (the CPU G2's n, unchanged).
- **MIN_MEASURED_PAIRS = 12** — at least 12 of the 23 registered pairs must produce a valid correctness-gated timing read. Below the floor the leg reads **UNSEEN**, never a pass (the CPU cautionary shape: 53-of-74 unmeasurable made the CPU read structurally blind; this floor exists so a GPU run cannot green on a sliver).
- A pair that fails to COMPILE on the 4090 (toolchain/runtime mismatch) is a pair FAIL counted toward the disclosure but NOT toward MIN_MEASURED_PAIRS' numerator — it is a registration defect to fix and re-run, not an absence (the green-zero law: an extraction/compile failure is a FAIL, never absent-from-population).

## Condition (c) — the CPU-regressing rules: restricted AND disclosed

The CPU campaign's 7 regressing rules are **unnamed by policy**: the per-rule CPU medians live only in an uncommitted run artifact, and re-deriving them requires a CPU tranche re-run, which the 2026-11-02 verdict forbids verbatim ("NEVER a CPU tranche re-run"). The pinned policy, satisfying BOTH branches of condition (c) honestly:

1. **Restricted to GPU contexts:** every registered pair IS a GPU-context implementation by construction — a rule enters the GPU leg only through its representative GPU pattern, never through its CPU fixture shape.
2. **Disclosed per-rule:** the run record discloses EVERY registered pair's verdict (median, per-pair spread, correctness verdict) — including each pair's CPU-era status where the CPU record names it (`fp32-pipe-mma-role-split` = the 2.05× CPU passer, named above; the 7 CPU-regressors, being unnamed, are covered by the blanket disclosure that ALL 21 CPU-measured rules are treated uniformly under this registration). No rule is pre-excluded by its CPU verdict, and no CPU number is re-measured here.

## The rig contract + PROVENANCE (T1 bar 2: a latency number without its BOX STATE is not a measurement)

- **Exclusivity is MANDATORY:** the run goes through `agent-mail rig run 4090 --exclusive --needs cuda --timeout 2h -- <cmd>`. The m3 never benches; an amo/bmo box never substitutes (the registration is sm_89 + 4090-class CUDA).
- **The PROVENANCE line** (pinned fields, captured at run start and end, both quoted in the run record): `nvidia-smi --query-gpu=name,driver_version,clocks.gr,clocks.mem,temperature.gpu,power.draw --format=csv` · the rig's QUIET receipt (agent-mail rig status at hold acquisition) · rustc/toolchain version · the commit hash of the sources commit · the BLAKE3 digests of every materialized source file (verified pre-compile) · the CubeCL/cuda runtime versions · start/end wall clock. Windows is the host OS: the runner compiles with the box's CUDA toolchain; a compile failure names the missing leg LOUD (a LOUD SKIP, never a green).
- **Thermal gate:** if `temperature.gpu` at run start exceeds the box's recorded ambient band by > 10 °C, or `power.draw` idles above the box's recorded idle band, the run records the anomaly and treats its own numbers PROVISIONAL (the GPU-exclusivity precedent: Bench 649's 12.3× swing) — a provisional leg cannot pass the bar, it can only fail it or read UNSEEN.

## The verdict arithmetic (pre-pinned)

- Per-pair verdict: `median_speedup = median(t_ref/t_opt over 15 interleaved pairs)`.
- **Leg verdict:** median over ALL measured pairs' median_speedups.
  - `≥ 1.06` → the GPU-host G2 leg **PASSES** (the ≥ 6% bar UNCHANGED — the T1/D1 contract).
  - `< 1.06` → **FAIL** (sub-bar medians are fails, not "close"); per-pair medians `< 1.0` are measured regressions, each recorded per-pair with the reason (the CPU bar's per-rule exit discipline).
  - `measured < MIN_MEASURED_PAIRS` → **UNSEEN** (never a pass, never folded into either verdict).
- The run record carries: the pair table reproduced with measured columns, every per-pair row, the aggregate, the PROVENANCE lines, and the honest next-step read (promotion still fenced on G5-cost per Bench 129 — a G2 pass alone promotes nothing).

## Kill criteria (pre-registered)

- Sources cannot compile against the 4090's CUDA/ CubeCL stack after one honest toolchain-alignment attempt → the leg records TOOLCHAIN-BLOCKED (a LOUD SKIP class), the lease releases, and the fence stays as measured. No third attempt, no silent substitution of another GPU stack.
- Fewer than 12 pairs produce valid reads → UNSEEN with the per-pair failure ledger; the registration may be re-issued as a NEW pre-registration document (fresh number), never amended in place.
- Any pair's correctness gate fails → that pair FAILS and is recorded; if ≥ 2 pairs fail correctness, the registration's pair-authoring is the finding (filed to the rulewright improvement queue), not a bench retry.

## Refs

- Issue 191 (T6, the three conditions + the owner greenlight "do 8"), Bench 124 (the CPU campaign + the G2 harness law), Bench 125/126/129 (the G5 fence state), katgpt-rs `scripts/sequential_ab_timing_audit.py` (the interleaved-pair law), `src/domains/kernel_opt/corpus/goat_g2.rs` (the harness the law was transcribed from).
