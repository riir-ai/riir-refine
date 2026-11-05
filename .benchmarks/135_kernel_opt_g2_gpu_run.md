# Bench 135 — RUN RECORD: the GPU-host G2 re-gate (Bench 130's registration + Bench 133's sources pin, executed) — **leg median 1.3774 ≥ 1.06, the G2(GPU) leg PASSES**; 23/23 measured, 23/23 correctness on CUDA

**Status:** RUN COMPLETE 2026-11-05 (zed-m3-i191t6, the issue-191 T6 lease) — the timing run Bench 130 pre-registered and Bench 133 digest-pinned. Every number below is quoted from the unmodified run log (`run4090.log`, fetched verbatim from the box). Verdict arithmetic per Bench 130 §The verdict arithmetic, pre-pinned.

## The verdict table (Bench 130's pre-pinned arithmetic)

| check | bar (pre-registered) | measured | verdict |
|---|---|---|---|
| correctness gate | all pairs pass BEFORE timing | **23 passed / 0 failed / 23 selected** (CUDA) | PASS — the modelless-gain law holds |
| measurable pairs | ≥ MIN_MEASURED_PAIRS = 12 | **23 of 23** measured (100%; the CPU campaign's cautionary shape was 21-of-74) | not UNSEEN — the leg is fully seated |
| **leg median** | **≥ 1.06** | **1.3774** over 23 per-pair medians | **PASS — the GPU-host G2 leg clears the ≥ 6% bar** |
| thermal gate | > 10 °C over ambient / anomalous idle power ⇒ PROVISIONAL | START 49 °C @ 31.40 W idle → END 51 °C @ 122.71 W load; clocks 210→2790 MHz (gr), 405→10251 MHz (mem) | no anomaly — numbers UNPROVISIONAL |
| per-rule regressions | median < 1.0 exits recorded per-pair | 8 pairs < 1.0 (table below) | recorded per-rule, never pooled away |

**The axis that was closed since Bench 124 ("the GPU-shape wins are structurally not CPU-demonstrable", tranche median 1.0000) now reads 1.3774 on its home surface — the CPU-portable closure stands (never re-run, per the 2026-11-02 verdict); the GPU-host re-gate the 2026-11-04 D1 verdict re-opened is measured and PASSES.**

**The honest next-step read (pre-pinned in Bench 130): promotion still requires BOTH G2(GPU-host) AND G5-cost. G2(GPU) now PASSES; the G5-cost fence is still UP — Bench 129 measured it FAIL structurally (added p50 33.127/37.306 ms vs the ≤ 1.0 ceiling, "the ceiling is never re-registered"), and the pregate lane's Bench 132 (issue 208, landed in parallel today) flipped the p50 leg GREEN (0.191 ms flat) with the p95 leg still FAIL (122.5 ms vs ≤ 5.0). `kernel_opt` stays opt-in: the failing axis named is G5-cost (p95), exactly per bar 6.**

## Provenance (the T1 bar-2 law: box state or it did not happen)

- **Job:** `agent-mail rig run 4090 --exclusive --needs cuda --timeout 2h`, job id `01a1ac26-e8e1-75a6-bf92-874b67fbe2e7` (the Exclusive lane receipt: the rig was QUIET at hold acquisition; an earlier attempt `01a1ac13-…a419844b` FAILED at BUILD — zero timed chunks, no pre-registration violation — see §The toolchain alignment below).
- **PROVENANCE START** (2026-11-05T21:39:53+07:00): `NVIDIA GeForce RTX 4090, 610.88, 210 MHz, 405 MHz, 49, 31.40 W` / `6 %, 7779 MiB` / `rustc 1.99.0 (b940084d7 2026-09-28)`.
- **PROVENANCE END** (2026-11-05T21:41:42+07:00): `NVIDIA GeForce RTX 4090, 610.88, 2790 MHz, 10251 MHz, 51, 122.71 W` / `100 %, 7781 MiB` / same rustc.
- **The full run wall time ≈ 110 s** (the log's START→END stamps) — the cold compile happened BEFORE the START snap (the script's Snap("START") precedes the build; the compile tail is in the log).
- **Sources custody chain:** Bench 133's 10 files BLAKE3-verified on the m3 at materialization → tar/scp → **sha256 verified on the box** (all 10 rows logged in `run4090.log`, byte-matching the m3 side; Cargo.lock `2787cec6…`, Cargo.toml `c6b12bd1…`, the six `src/*.rs` pairs and `probe.rs` all match) → compiled from the vendored patches resolving at `E:\git\riir-ai\vendor\cubecl-runtime-0.11.0-pre.2` + `E:\git\riir-infer\vendor\{cubecl-common,cubecl-wgpu}-0.11.0-pre.2` (the compile log names the vendored paths — the #1359 drop_queue fix rode along).
- **Env:** `BENCH_SOURCES_COMMIT=82b5aaf2` (the pin commit, renumbered to 133 by `3d5aff179` without byte changes), `BENCH_BOX_LABEL=4090`, `CARGO_TARGET_DIR=E:\t\kbg2` (isolated; no sibling target touched).
- **Autotune: OFF** (`launch_unchecked` only), per-pair launch geometry from Bench 130's pinned table.

### The toolchain alignment (disclosed; one honest attempt, successful)

The first job failed at BUILD: the box's default toolchain is `stable` = rustc 1.93.0, and `cubecl-zspace@0.11.0-pre.2` declares `rust-version ≥ 1.95` — cargo refused pre-compile (exit 101, **zero timed chunks**). Bench 130 pinned PROVENANCE *fields* but no toolchain version — a registration gap, now closed: the m3 verification ran at 1.99.0, and the box already had **1.99.0 installed** (the house pin; just not default). Fix: `rust-toolchain.toml` (`channel = "1.99.0"`, blake3 `4ea8e17cbdad375a5345393b3a0565a0acafd056f9c5aeeaf4409d5397b09bee`) added to the crate dir on BOTH boxes — an environment selector, not one of the 10 pinned sources (their bytes are untouched; the box-side sha256 rows in the log re-assert them). The second job compiled clean at the SAME rustc build the m3 verified (`b940084d7 2026-09-28`).

## The per-pair table (Bench 130's 23 named pairs, measured columns; t_* = calibrated chunk ms, verdict = median over 15 interleaved pairs)

| pair | t_ref_ms | t_opt_ms | median_speedup | iters | axis (Bench 130) | verdict |
|---|---|---|---|---|---|---|
| P117-A | 1.9278 | 1.8493 | 1.0428 | 4 | dispatch fusion | flat-positive |
| P117-B | 3.4632 | 3.5433 | 0.9797 | 1 | split-K @1024³ K8192 | measured regression (flat-ish) |
| P117-C | 3.3176 | 27.0965 | **0.1219** | 1 | register tile 2048² | measured regression — the 32-output tile is badly shaped at this geometry |
| P125-A | 3.3838 | 13.9625 | **0.2441** | 8 | operand-role swap | measured regression — the honest ≈1.0-anticipated pair landed worse; the layout mapping costs more than the strided read it removes at this shape |
| P111-A | 4.3437 | 1.6850 | **2.5948** | 1 | weight-panel smem share | PASS-class win |
| P75-A | 2.3294 | 1.7187 | **1.3530** | 8 | resident MMA accumulator | PASS-class win |
| P111-B | 2.9325 | 3.0175 | 0.9896 | 512 | head-row combine (i32 fixed-point atomics) | flat |
| P136-B | 3.4314 | 0.7951 | **4.3609** | 32 | quad-lane softmax | PASS-class win |
| P138-A | 2.2904 | 0.0610 | **37.5148** | 1 | row-split statistic | PASS-class win |
| P110-A | 3.2389 | 2.9735 | 1.0897 | 128 | block-local state exchange | PASS-class (thin) |
| P133-A | 2.9000 | 1.5105 | **1.9028** | 256 | smem tile cap | PASS-class win |
| P134-A | 2.6832 | 0.0655 | **44.0776** | 8 | launch-storm consolidation | PASS-class win (the launch-overhead axis is real on the 4090) |
| P134-B | 3.4970 | 3.5721 | 0.9787 | 8 | drain-time partials | flat (atomics ≈ drain at this shape) |
| P112-A | 2.0875 | 0.0532 | **39.2556** | 4 | transcendental hoist | PASS-class win |
| P112-B | 3.0312 | 1.0400 | **2.9371** | 128 | fused chain | PASS-class win |
| P138-B | 3.3749 | 1.6226 | **1.9840** | 256 | dual-bank scratch | PASS-class win |
| P75-B | 2.2255 | 1.0538 | **2.1220** | 128 | prefetch/ILP walk | PASS-class win |
| P75-C | 1.7528 | 1.2728 | **1.3774** | 64 | partial-unroll spill bound | PASS-class win |
| P136-A | 3.0805 | 6.2259 | **0.4949** | 4 | biased operand split | measured regression — the host-side 4-buffer split doubles traffic at this shape (the CPU-era 2.05× scalar twin did NOT transfer to this GPU mapping) |
| P137-A | 3.4536 | 1.3455 | **2.5808** | 8 | coalesced tile walk | PASS-class win |
| P137-B | 51.5441 | 51.6146 | 1.0014 | 1 | branch-free codebook rank | flat — the branchy ref does not diverge-cost at this dim |
| P135-A | 91.2286 | 0.1937 | **459.1864** | 1 | specialized top-k | PASS-class win — the selection-sort ref is O(k·N); the specialization is the rule's whole point |
| P121-A | 3.6284 | 286.3112 | **0.0127** | 128 | int8 screen + rescore | measured regression — the disclosed single-block selection shape (256 serial passes on one SM) dominates; the SCREEN itself is not priced by this pair shape |

Aggregate: **13 pairs ≥ 1.06 · 2 pairs 1.0–1.06 · 8 pairs < 1.0 · leg median 1.3774.** Every < 1.0 pair's exit reason is recorded above per-rule (the T1 bar-2 exit discipline); none is pooled away.

## Reads (honest, per Bench 130's pinned read rules)

1. **The gate did its job in BOTH directions.** The wins are exactly the rule classes the corpus names (launch consolidation, fusion, hoisting, smem sharing, specialization-by-shape) — and the 8 regressions are pairs whose pinned geometry mapping is genuinely bad on the 4090 (P117-C, P125-A, P136-A, P121-A) or flat (the atomic/partials family). A CPU-era flat surface (20 of 21 at 1.00±0.01) was hiding real GPU-shape structure; the tranche discipline surfaced it.
2. **The two biggest wins (P135-A 459×, P134-A 44×) are real but shape-dominated** — they price the ref's algorithmic pathology (O(k·N) selection; 64 launches), which IS what those rules name, but they are not "6% kernel improvements"; they are algorithm-admission rules. The record does not average the story away: the median (1.3774) sits on 13 genuine PASS-class wins spanning 1.08–39.3×.
3. **The P121-A loss is a pair-shape loss, not a rule loss** (disclosed at Bench 131/133 as "honest but slow"): the int8 screen + rescore rule remains viable; its bench pair needs a multi-block selection shape. Filed as the run record's own follow-up note — the pre-registration forbids re-shaping a pair inside a lease, so the loss stands recorded.
4. **Issue 193's fixture-repair discipline is vindicated on the GPU surface too** — 23/23 correctness on CUDA with zero re-runs.

## What this changes (and what it does not)

- **G2(GPU-host): PASS** — the Tier-B re-gate trigger is exercised and cleared at the pre-pinned bar.
- **Promotion: still REFUSED** — G5-cost remains the failing axis (Bench 129; Bench 132's pregate p50 leg is green at 0.191 ms but its p95 leg fails at 122.5 ms vs ≤ 5.0; the per-span line-query cap lever is pre-registered as Bench 134). Per bar 6 the domain stays opt-in with the failing axis named.
- **The re-campaign sequence (T6 ∧ T7 → re-run G1+G3+G4+G5)** — T6 is now DONE (this record); T7 landed (Bench 129, fence FAIL recorded); the G1+G3+G4+G5 re-run waits on the G5-cost fence (the pregate lane's active lever, Bench 134). Issue 191 stays open on that single fence; its T6 task row is ticked by this record.

## Reproduce

- Sources: Bench 133 (the digest-pinned 10 files) + the toolchain selector (blake3 `4ea8e17c…`, §The toolchain alignment).
- Rig: `agent-mail rig run 4090 --exclusive --needs cuda --timeout 2h -- powershell -NoProfile -ExecutionPolicy Bypass -File E:/git/kernel_bench_g2_gpu/run_4090.ps1` (the script: PROVENANCE snap → sha256 custody rows → `--correctness` (23/0 required) → `--bench --provenance` → PROVENANCE snap; log at `E:\git\kernel_bench_g2_gpu\run4090.log`).
- Cleanup: the scratch crate was removed from both boxes after this record (the pin in Bench 133 is the durable copy).

## Refs

- Bench 130 (the pre-registration; the three conditions), Bench 133 (the sources pin; renumbered from 131 by the pregate lane's `3d5aff179` first-allocation adjudication), Bench 124/125/126/129 (the campaign + the G5 fence state), Bench 131/132/134 (the pregate lane's parallel arc, issue 208), issue 191 (T6, the lease), issue 208 (the G5-cost clearing lane).
