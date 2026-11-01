# Bench 124 — the kernel_opt GOAT-gate campaign (Issue 191 T5): G1–G4 measured, G2+G1 FAIL, promotion REFUSED (the domain stays opt-in)

**Status:** CAMPAIGN COMPLETE — the pre-registered verdict is determined by the gates; the
promotion decision rides the bars, not a judgment call. Claimed + executed by
zed-shikuwa-i191gate (epoch 1793523566) on the 4090 box (shikuwa), 2026-11-01.

## The verdict table

| Gate | Bar (pre-registered T1) | Measured | Verdict |
|---|---|---|---|
| **G1** correctness | every fixture rule compiles + its own asserts pass under the cascade; census reconciliation; floors ≥ 248 rules / ≥ 72 files | 248 fixture-bearing rules / 72 files (floors hold); **183 green / 65 FAIL** (compile or assert); 803 parse-only disclosed; 209.9 s; census reconciled (gpu_63/64 pinned at 1 each) | **FAIL** — 65 parse-valid fixtures are compile-invalid or assert-failing (Issue 193; 3 re-verified standalone → fixture bugs, not harness artifacts) |
| **G2** perf | CPU-portable ref-vs-opt microbench, interleaved 15-pair median ≥ 1.06 at release, deterministic tranche (15 files), rig exclusivity mandatory | tranche = 15 files / 74 green rules; **21 measured / 1 passing / 7 regressing / 53 not-measured** (no mirrored fn pair); tranche median **1.0000 < 1.06**; best: fp32-pipe-mma-role-split 2.05× | **FAIL** — the GPU-shape wins do not transfer to the CPU-portable surface (bar 2's own caveat: "a GPU-shaped rule's win is not CPU-measurable") |
| **G3** no-regression | clippy_lints + rust_perf score-bench verdicts byte-stable with kernel_opt ON (existing domains, disclosure boundary) | `scripts/g3_scorebench_stability.sh`: clippy lane 40 verdict lines byte-identical, perf lane 67 byte-identical | **PASS** |
| **G4** alloc-free | default build (kernel_opt OFF) byte-identical — compile-time-only cost | structural: the whole domain compiles only behind `kernel_opt`; the default clippy gate is clean and untouched | **PASS** (structural) |
| **G5** utility/cost | match census on a pinned corpus + p50 ≤ 1 ms / p95 ≤ 5 ms per file + ≥ 3-expert utility floor | **NOT RUN this campaign** — the verdict was already determined by the G1+G2 failures, and the Issue-193 repairs change the census population anyway; G5 runs at the re-campaign | n/a (disclosed) |

**Promotion verdict: REFUSED — `kernel_opt` stays opt-in**, failing axes named: G1 (65
fixture defects) and G2 (no CPU-demonstrable median win). Per Issue-191 bar 6: "If any gate
fails, the domain stays opt-in with the failing axis named in the record."

## What the campaign measured (the honest reads)

1. **The gates WORK — both failures are the gates doing their job.** G1's first standing run
   found the corpus's fixture-compile debt (the per-entry gate was parse-only, never
   compile-enforced). G2's first standing run measured the CPU-portable surface and found it
   flat: 20 of 21 measured rules sit at 1.00±0.01 (the opt form is the same scalar code or a
   GPU-only shape), one at 2.05×, seven slightly below 1.0 (noise or CPU-hostile reordering).
2. **The not-measured majority is the real G2 blocker.** 53/74 tranche rules expose NO
   mirrored fn pair — their fixtures are test-shaped (asserts over inline code), not
   bench-shaped (a callable ref/opt pair). A future G2 pass needs either a bench-entry
   convention in the corpus (owner-gated amendment) or GPU-host benches (the named Tier-B
   re-gate trigger), not more harness work.
3. **The one PASS (fp32-pipe-mma-role-split, 2.05×)** is exactly the kind of rule G2 exists
   to find: a CPU-meaningful arithmetic reordering (biased-bf16 operand split) with a real
   scalar twin. The tranche discipline surfaced it deterministically.

## Provenance

- Box: shikuwa (4090, Windows 11, i7-13700K 24 cpus, 32 GB) — verified QUIET via
  `agent-mail rig status` (no heavy compute consumers) before the G2 full run; the G1 full
  run and the G3 arm ran under the same posture.
- Toolchain: rustc 1.99.0 (b940084d7 2026-09-28), the repo pin.
- Harness commits: T2 `a9703c00a` (G1), T3 `82b77f88b` (verdict store + annotation), T4
  `2ff3c0a61` (G3 arm), T5 `96f3733ee` (G2 harness) — rebased onto origin/develop
  05a65e1ba, pushed.
- Reproduce:
  - G1: `cargo run --release --example kernel_goat_g1 --features kernel_opt -- --full --out <json>`
  - G2: `cargo run --release --example kernel_goat_g2 --features kernel_opt -- --full --out <json>`
    (`--tranche` prints the selector disclosure)
  - G3: `sh scripts/g3_scorebench_stability.sh`

## Next (the re-campaign path)

1. Issue 193 repairs (R1–R5) → G1 green (or recorded demotions + floor re-derivation).
2. The G2 convention decision (owner-gated): a corpus bench-entry convention vs GPU-host
   benches vs closing the CPU-G2 axis with the disclosure (the flat median IS the measured
   answer for scalar ports).
3. Re-campaign: G1 + G2 (tranche re-selected from the repaired corpus) + G5 census on the
   pinned corpus in the same run.
