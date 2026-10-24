# Research 310 — ml-explore/mlx #4712 few-row qmv_mma/qmv_nax + #4682 staged affine_qmm_t_nax split-K

**Status:** DISTILLED — pending owner decision (staged as nominal Batch 273; NOT claimed — the next claimant opens it per the queue, after the in-flight Batch 272 sonar lane)

## Source

- Repo: ml-explore/mlx (Apple ML framework, MIT), Metal backend.
- PR #4712 — `[Metal] Add qmv_mma/qmv_nax kernels for 6-16 rows with large N/K`, merge `c6abf0dbb4fc141fad739b32ae486c06728e49e5` (2026-10-22), fp_quantized{,_nax}.h/.metal + quantized.cpp (+558).
- PR #4682 — `[Metal] Add staged affine_qmm_t_nax kernels with split-k`, merge `bff10220c742907f0c68e55ff174aaea56f74e3d` (2026-10-20), quantized_nax.h + quantized.cpp (+660).
- Inspection 2026-10-24 (zed-m3/M3), clone `.raw/mlx` at origin/main `56c2bf92` (both merges in-tree); PR bodies fetched via GitHub API — both carry paired A/B numbers with 95% CIs. Quotes re-verifiable at the merge shas. Clone removed at landing (`.raw` hygiene).
- Provenance chain: Apple's own kernels; #4712 co-authored with Cheng (zcbenz); #4682 by Yanzhao Wang. No upstream port cited inside these PRs (unlike the corpus's qmv2 rule, which cites @sudoingX's CUDA small-batch PTQ1_0 kernels).

## What the commits do

#4712 — few-row (M 6..16) fp-quantized matvec (nvfp4/mxfp4/mxfp8) routed to matrix units: one `fp_qmv_mma` kernel, BK=128, WK=4/8 simdgroups along K, TM∈{1,2} (8/16 rows), per-thread `frag_t C[TN][TM]` accumulators over 8×8 mma fragments; each weight block is READ ONCE, unpacked in registers (e2m1→half embed via `((q & 0x00070007) << 9) | ((q & 0x00080008) << 12)` + one `16384.0h` bias multiply for gs16; mxfp8 byte-pair form), and applied to ALL rows; partial rows masked via `x_rows[TM][2]` (M=5/13 legal); WK−1 cross-simdgroup partial sets exchanged through threadgroup smem, simd 0 folds. NAX twin `qmv_nax` (bn=32, wk=8) reuses the NAX tile path. Dispatch `use_qmv_mma`: K%128==0, NOT affine mode, alignment gates (x.offset %(8·itemsize), w.offset%16, scales.offset%2 — misaligned views REFUSE, refusal is test-covered `test_qmm_few_rows`), then a THREE-TIER per-silicon threshold table — NAX: N,K≥2048, N%32==0 (+tf32/f32 gate); arch ending 'd' (M3-class): N,K≥4096, N%16==0; older: N,K≥512 AND N·K≥2^22. Route `transpose && non_batched && M>=6 && M<=16`. Measured (M5 Max, mlx_lm.benchmark, PR body): Qwen3.8-27B-nvfp4 batch 8 113.1→187.9 tok/s (1.66×), batch 16 149.5→298.4 (2.00×), mxfp4 2.10×, mxfp8 1.72×; batch 1 and prompt UNCHANGED; "~3× on M1/M2" (their claim).

#4682 — transposed AFFINE quantized qmm on NAX: (a) `QuantizedBlockLoader` split into `fetch()` (packed words + group scale/bias for step k+1 into registers) and `store()` (dequant into threadgroup memory) — double-buffered `Ws`, ONE barrier per K step, prefetch hides device reads behind MMAs; fp32 EXCLUDED (double buffering exceeds 32 KB threadgroup memory — the budget-exclusion arm, quoted in the PR body); (b) 16-row schedule: `WN==1` selects the staged impl, 4× 16×64 simdgroups per 64×64 tile instead of 2×2 of 32×32, flat `(128,1,1)` threadgroups ("A flat threadgroup runs the 16-row schedule faster than (32, 1, wm)"); (c) NAX split-K: fp32 partials + existing `steel_gemm_splitk_accum` (single rounding after the sum), ladder `qmm_nax_k_partitions`: double up to 4 while `tiles*split_k*2 <= 256` AND `K/(split_k*2) >= 256` AND `K % (split_k*2*max(64,group_size)) == 0`; route requires biases present (`!biases → false`), N,K%64==0. BIT-IDENTITY: "The MMA order is unchanged, so outputs are bit-identical at the same split count"; the schedule comparison ran 336 cases "bit-identical when the split count was unchanged". Measured (M5 Max, paired geo-means + CIs): d1-3B 85-token forward 30.1→14.5 ms (2.07 [2.05,2.09]); operator table 2.03–4.35× on rerouted shapes (qmv_wide/qmm_t_splitk/qmm_t_nax baselines); compute-bound M=512 control 1.048 (no regression); keep-route control 1.003.

## Verdict

**YES B− corpus (rider-batch staging) / LEAGUE INTEL MODERATE / no product-coverage family seed.**

The few-row band and the split-K ladder are the corpus's most heavily mined dispatch axes; what remains new here is rider-shaped, and it is GOOD rider-shaped — the numbers are the strongest this window (paired A/B, CIs, bit-identity contracts, honest anti-regimes).

## Domain routing

`kernel_opt` — Metal/NAX kernel + host dispatch. The tempting wrong route: none (this is squarely in-domain); the tempting wrong TREATMENT is a fresh own-rule batch, which the dedup gate below refuses.

## Dedup gate (both vocabularies, evidence per candidate)

1. **Few-row weight-read-once matvec** — the corpus owns the axis THREE deep:
   - `small-batch-matvec-gemm-bridge-rung` (B98, entries_gpu_13): the 1/GEMV → cap/bridge → GEMM ladder + the dequant-amortization middle mechanism via 2D lane reorg; B116 rider adds register-budget sizing, EXACT-FIT row counts (`ne11 == 3` own pipeline), and the per-TYPE mv/mm crossover override to 16 rows.
   - `fewrow-tensor-tile-decode-into-rhs-operand` (B66, entries_gpu_66): a tensor tile SIZED to the verify band (3..32 rows), decode fused into operand production, measured crossover (n=1..2 slower → min 3), probed cooperative-tensor layout, PQ2_0 carrier (THE league-model carrier — this rule already serves Bonsai PQ2_0 few-row).
   - `batch-width-routed-dual-tile-twins` (B77): batch-width-keyed tile-shape twins of one GEMM.
   ⇒ #4712's core ("read all weights once per block, multiply 6-16 rows on the matrix units") is the axis's next instance, NOT a new axis. What IS absent from every owner: (a) the MMA-FRAGMENT carrier that needs NO tensor API (works on M1–M4 plain simdgroup mma — B66's carrier is Metal-4-tensor-API-only, M5-class); (b) the PER-SILICON three-tier threshold table (B116's crossover is per-TYPE; this is per-SILICON: NAX/M3-class/older with three threshold sets); (c) partial-row masking inside the fragment grid (`x_rows[TM][2]`, M=13 in a TM=2 grid) with the alignment-REFUSAL gates test-covered; (d) the WK−1 cross-simdgroup smem partial-exchange epilogue. Host recommendation: fat rider on `small-batch-matvec-gemm-bridge-rung` (it owns the ladder + the middle-mechanism class; B116 is the rider precedent), `fewrow-tensor-tile…` named as kin. The batch author may instead judge the no-tensor-API fragment carrier a distinct rule — if so, author it WITH the ladder kinship stated both directions.
2. **Capacity-bounded split-K ladder** — `splitk-tier-ladder-per-tier-ceiling-min-partition-inputs` (B268, gpu_117, uzu 6198321) owns the mechanism: tier ladder, per-tier ceiling, min-partition floor, alignment precondition, reassociation disclosed. #4682's ladder (`≤4`, `tiles*2≤256`, `K/(split_k*2)≥256`, `K%(split_k*2*k_align)==0`) is the same shape with NAX constants ⇒ SECOND-INSTANCE rider on B268. The no-bias refusal (`!biases → false`) rides it.
3. **Staged fetch/store loader (register prefetch, double-buffered Ws, one barrier/K step, fp32 excluded by smem budget)** — kin `dequant-once-per-k-tile` (B16) + the cp.async swizzle-stage riders. The Metal-specific face (NO async-copy ⇒ explicit software pipeline; the fetch/store split of a block loader) is rider-class on the B16 family; the fp32 smem-budget exclusion is the honest anti-regime clause.
4. **16-row simdgroup schedule (WN==1, flat (128,1,1))** — kin `apple10-narrow-grid-register-tile-threadgroups-per-core` (B268 slice A, decode kernel-variant selection) and the mma-fragment-layout family. Small dispatch face; count-neutral rider at best.
5. DROPPED: the e2m1→half bit embed (`<<9`/`<<12` + `16384.0h`) — positive instance of the mantissa-embedding family (`lop3-int4-to-half2-dequant` + its B242 mlx-serve 2-bit ladder rider, which already names the fp-embed siblings); the paired-scale ushort fetch (gs16 reads both scale bytes in one `ushort`) — a two-line micro-face, fold into rider 1's notes, not a rule.

## GOAT candidate

None passes all four GOAT criteria this window: NotBenchable-here (Metal/NAX kernels; the corpus has no Metal validator; NAX does not fire on the M3 — entries_gpu_65's standing disclosure), exact semantics are achievable (bit-identity is in-source at unchanged split count) but the win metric lives on M5/M1-M4 Metal silicon. Per the two-class split this is the DOMAIN reason — annotation (`NotBenchable`, `bench_spec: None`), never a demotion. Lead content for the batch: the B98-hosted rider (fragment carrier + silicon-tier thresholds + partial-row masking), quoting the PR-body numbers as source-reported provenance.

## Lane-intel axis (the league + open lanes)

- **League (perf-rematch):** the MLX opponent's BATCHED decode on the league model family moved — Qwen3.8-27B fp4 batch 8/16 = 1.66–2.10× on M5 Max. The league matrix cells (pp2048/4096, tg128) sit OUTSIDE the 6..16-row regime (tg128 = 128 rows; prefill = wide M), so NO matrix cell moves and the escalation ladder does NOT fire. The m5ultra measuring box (M5 = NAX silicon) is IN the affected family for any future batch-8/16 cell. Record beside the next matrix run; no league doc edit owed today (matches the 10-24 watch pass's "no scored movement" — different axis, now priced).
- **riir-infer-laya (M3 Metal lane):** batch-1 decode UNCHANGED (PR: "No observed prompt or batch 1 change") — no direct threat. The M3-class tier of `use_qmv_mma` (simdgroup-mma fragments, N,K≥4096) is a candidate mechanism IF the serving lane ever batches 6–16 decisions — noted here, no issue filed (no consumer today).
- **riir-reflex d1 lane:** #4682's headline model IS LiquidAI d1-3B (the `--d1` comparison lane's competitor) — its short-input serving doubled (2.07×) on M5 Max. Reflex's bench-129 cells are frozen M3-box reads and stand; the NEXT d1 re-read should expect a faster competitor and re-pin with box-state provenance.
- **4090:** unaffected (not Metal).

## Product-coverage axis

MLX is Apple's Python/Swift/C++ framework, not a Rust famous-lib — no family seed fires. The patterns transfer to Rust Metal users (candle's Metal backend, our own riir-infer-laya MSL kernels); the rules land in `kernel_opt` where those consumers already read.

## Ranked candidates

| # | Candidate | Source | Grade | Disposition |
|---|---|---|---|---|
| 1 | Few-row matvec via mma fragments (no tensor API) + per-silicon three-tier thresholds + partial-row masking + alignment refusals | #4712 `fp_qmv_mma` + `use_qmv_mma` + `test_qmm_few_rows` | B (content) | Rider on `small-batch-matvec-gemm-bridge-rung` (or own rule WITH ladder kinship — batch author's call) |
| 2 | NAX split-K capacity-bounded ladder + no-bias refusal + fp32-partials single-rounding | #4682 `qmm_nax_k_partitions` + `use_qmm_nax_splitk` | B− | Second-instance rider on `splitk-tier-ladder-per-tier-ceiling-min-partition-inputs` (B268) |
| 3 | Staged fetch/store block loader (register prefetch, double-buffered Ws, one barrier/K step, fp32 smem-budget exclusion) | #4682 `QuantizedBlockLoaderStaged` | B− | Rider face on the `dequant-once-per-k-tile` (B16) family |
| 4 | 16-row schedule / flat-threadgroup dispatch face | #4682 `use_qmm_t_nax_staged` + WN==1 impl | C+ | Count-neutral rider, optional |
| 5 | e2m1→half embed + paired-scale ushort fetch | #4712 fragment dequant | DROPPED | Positive instances of the embedding family (B9 rider lineage) |

## Yield estimate

0–1 own rules + 2–3 riders after coordinator falsification (corpus 937 → +0..2). If the batch author judges candidate 1 an own rule, +1 (938).

## Honest caveats

- Staged, not claimed: Batch 272 (sonar lane) is in flight on this box — this note stages the NEXT batch only.
- The PR bodies' numbers are Apple's own benches on M5 Max / M1-M2 — provenance, never adoption bars; NAX paths do not fire on this box (M3), the M3-class qmv_mma tier is unverified here.
- Both kernels are FP-quant/affine-specific: PQ2_0 (the league quant) is NOT served by #4712 (affine/fp modes only; gather kernels unchanged) — B66's fewrow rule remains the league model's few-row carrier.
- TF32 context: MLX's `env::enable_tf32()` gates float32 into some routes; the PR test notes 1268 pre-existing fp32 subtest failures on main (shared with this PR) — not ours to fix, noted for provenance honesty.
- `.raw/mlx` removed at landing; pins recorded above.

## Queue slot

Staged: Plan 277 / Research 310, nominal Batch 273; NOT claimed — the next claimant opens it per the queue (after Batch 272 lands). NOTHING INVENTED — every quote from the two merge commits or the fetched PR bodies this session.
