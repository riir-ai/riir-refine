# Bench 125 — kernel_opt G5 utility/cost leg (Issue 191 bar 5): utility PASS, cost FAIL (the disclosed Bench-124 deferral, measured)

**Status:** MEASURED COMPLETE — utility leg **PASS** (15/15 experts fire at rank-1), cost leg
**FAIL** (added scan latency p50 38.795 ms/file, p95 177.612 ms/file vs the pre-registered
p50 ≤ 1.0 / p95 ≤ 5.0 ms ceilings — 39× / 35× over). Run by zed-4090-193 (epoch 1793554051)
on shikuwa (4090 box), 2026-11-01, commit `320ec193` (develop, the Issue-193-closed corpus).

This leg is the ONE disclosed deferral of the Bench-124 campaign ("G5 NOT RUN this campaign
— … the census runs at the re-campaign on the repaired corpus"), measured early as its own
bench because it is convention-INDEPENDENT: it exercises the production scan path over real
code, not the G2 microbench axis whose convention decision is owner-gated. Measuring it now
settles, before the owner gates anything, that **the re-campaign cannot pass at the shipped
scan cost**: even a favorable G2-convention outcome leaves the G5 cost leg failing at its
pre-registered bar (Issue 191 T1: "the numbers are the contract").

## The verdict table

| Leg | Bar (pre-registered Issue 191 T1, pinned 2026-11-01 BEFORE any measurement) | Measured | Verdict |
|---|---|---|---|
| **Utility floor** | the domain fires on ≥ 1 span through ≥ 3 distinct category experts | **15/15 experts** own at least one span's rank-1 kernel suggestion, over 4,055 kernel-cited spans of 15,7xx total spans (565 files); rank-1 distribution `{0: 960, 1: 148, 2: 111, 3: 959, 4: 567, 5: 176, 6: 53, 7: 190, 8: 26, 9: 13, 10: 609, 11: 44, 12: 46, 13: 125, 14: 28}` | **PASS** — every expert reachable, not one hot expert |
| **Cost (added ON−OFF)** | p50 ≤ 1.0 ms/file, p95 ≤ 5.0 ms/file | **p50 38.795 ms (tail support 284), p95 177.612 ms (tail support 29)**; 463/565 files exceed the 1 ms ceiling; max added 4,362 ms (one outlier file) | **FAIL** — 39× over at the median |
| Rule census (disclosure) | zero-match rules disclosed, never auto-demoted | **750/1051 rules cited** at least once (top-k=5, any rank); **301 zero-match** disclosed; full top-k citations per expert: `{0: 3079, 1: 1549, 2: 891, 3: 6400, 4: 1780, 5: 273, 6: 302, 7: 886, 8: 167, 9: 81, 10: 1778, 11: 324, 12: 600, 13: 1551, 14: 614}` | disclosure only |

## How it was measured (the instrument)

- Harness: `examples/kernel_goat_g5.rs` (new, standing — the goat_g1/g2 pattern): the
  production scan path `LatentMatcher::suggest_file` over the pinned corpus walk
  (`src/**/*.rs` sorted, 565 files), `top_k=5`, one untimed warmup pass then three timed
  passes, per-file medians computed by the runner; census read from the last pass.
- Runner: `scripts/g5_utility_cost.sh` — builds BOTH postures in isolated target dirs, runs
  the identical walk twice, diffs per-file medians. The only feature delta between the
  postures is `kernel_opt` (OFF = `latent_retrieval`; ON = `latent_retrieval,kernel_opt`),
  so the diff isolates exactly the kernel index's cost.
- "Fires" operationalization: rank-1 per span (the top-ranked kernel suggestion after the
  instantiation rerank). Deterministic, threshold-free — the corpus's own no-min-score law
  (`corpus_index.rs`: text similarity is NOT rule instantiation; there is NO threshold that
  separates them). Rank-1 is the strictest honest reading; the full top-k citation counts
  are disclosed beside it and only widen the pass.
- Absolute posture costs (the baseline context): OFF p50 27.77 ms / p95 127.36 ms; ON p50
  66.92 ms / p95 307.69 ms. The added-cost ceiling applies to the DIFF, per bar 5's wording
  ("the added ON-posture scan latency per file").

## Why the cost fails (the mechanism, not a mystery)

The kernel domain's L0 retrieval scores every span chunk against the FULL 1051-rule corpus —
the largest rule mass of any domain (clippy 112 + perf 289 + docker 31 + dist 7 ≈ 439 rules
combined). The category-expert router (`KernelExpertRouter`, 15 experts, 15/15 reachable
since Plan 004) was built exactly to narrow this ("consult category-relevant rules first
without scanning all") but was never wired into the matcher's kernel index — the demo
(`kernel_opt_demo`) routes signatures, the production scan fans out to every rule. The
39×-over-ceiling cost is that unwired narrowing, priced. Repair path filed as **Issue 196**
(router-narrowed kernel retrieval inside the matcher, behind a feature flag, same ceilings
re-measured at the re-campaign).

## Provenance

- Box: shikuwa (4090, Windows 11, i7-13700K 24 cpus, 32 GB), AC power, QUIET (CPU 8%,
  17.8/31.8 GiB free at run time — the start-context reading; the script's own mem probe
  printed n/a: wmic absent on this Win11 build, fallback chain exhausted). NOT a
  `bench_preflight` PROVENANCE line (that sibling script is macOS/riir-reflex-lane); the
  box-state law satisfied by disclosure here.
- Timing validity: the two postures ran as separate sequential processes (the
  sequential-AB confound class, +5–22% under load) — but each posture took the MEDIAN of 3
  passes per file, and the fail margin (39×) is orders outside the confound class. The
  verdict is load-robust; the exact ms values carry the usual sequential-process caveat.
- Toolchain: rustc 1.99.0 (b9404084d7 2026-09-28), the repo pin. Corpus: this repo's
  `src/**/*.rs` at commit `320ec193` (the Issue-193-closed corpus; 565 tracked-source files
  walked — the G5 spec's "this repo's src/**/*.rs at the campaign commit" leg; no `.raw`
  vendor clones are named in Bench 124, so none were pinned).
- Reproduce: `sh scripts/g5_utility_cost.sh --out-dir <dir>` (the two JSONs land in the dir;
  the script prints the full adjudication). **Reproduction run** (same commit, same box,
  fresh target dirs, post clippy-fixes): added p50 38.686 / p95 175.597 ms — 0.3% drift,
  identical verdicts, byte-identical census. The instrument is stable.

## What this means for the re-campaign (Bench 124 §Next)

1. ~~Issue 193 repairs → G1 green~~ DONE.
2. The G2 convention decision (owner-gated) — unchanged, still parked.
3. The re-campaign (G1+G2+G5 in one run) — now **fenced on TWO axes**: G2 (convention
   pending) AND G5-cost (39× over the pre-registered ceiling; repair path = Issue 196's
   router narrowing, re-measured at the same bar). A G5-cost pass requires either Issue 196
   landing a ~40× reduction (plausible: the router narrowing cuts the scored rule mass
   15×) or an owner re-registration of the ceiling (a bar change — Issue 191 T1 makes that
   an owner act, never a campaign-time adjustment).
