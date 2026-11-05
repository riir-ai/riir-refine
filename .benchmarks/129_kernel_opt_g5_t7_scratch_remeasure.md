# Bench 129 — Issue 191 T7: the G5-cost clearing lane re-measure (the scratch-reuse twin adoption)

**FAIL — the fence does not clear** (added scan latency p50 33.127 ms/file, p95 158.556 ms/file vs the pre-registered p50 ≤ 1.0 / p95 ≤ 5.0 ms ceilings — 33× / 32× over). The scratch-reuse lever is REAL but ~9-15%, not the ~38× the fence needs. Promotion verdict unchanged: **kernel_opt stays opt-in** (G5-cost failing; G2-GPU-host still pending, parked).

## The change under test

Issue 191 T7 (commit `77f7e46a9`): `SuggestScratch` carries a substrate `RetrievalScratch` (candidate `Vec` + `by_zone` map + the two-stage scan buffers, riir-neuron-db Issue 642's `retrieve_for_scoring_into` / `DiverseScanScratch` twins); `retrieve_scored_into` routes through the scratch twin, so the per-line suggest loop pays the two fixed allocations once per SCRATCH instead of once per call (flat: 1 + significant-lines calls per span; narrow: ×routed slices + tail per line — the lane with the most per-call instances). Byte-identical by construction (the substrate's `retrieve_for_scoring_into_matches_plain` contract test) and by measurement (narrow-posture blended top-3 ratio IDENTICAL with/without the change at 0.90697676 — see Issue 207 for that floor's own pre-existing decay).

## Result table

| posture | added p50 ms/file | added p95 ms/file | files over 1.0 ms | verdict |
|---|---|---|---|---|
| flat (kernel_opt) | **33.127** (tail support 298) | **158.556** (tail support 31) | 483/594 | **FAIL** — 33× over at the median |
| narrow (kernel_route_narrow, K + tail) | **37.306** (tail support 298) | **175.165** (tail support 31) | 488/594 | **FAIL** — but −19% vs its Bench-126 45.849 |

Utility legs **HOLD**: 15/15 distinct rank-1 experts over 4,277 kernel-cited spans (both postures — the Bench-125 floor holds); rule census 753/1053 cited flat, 731/1053 narrow (300/322 zero-match disclosed, never auto-demoted).

## Cross-bench attribution (not normalized — disclosed)

| bench | commit era | files | rules era | flat added p50 | narrow added p50 |
|---|---|---|---|---|---|
| 125 (T5 campaign) | `320ec193` | 565 | 1049 | 38.795 | — |
| 126 (route-narrow) | post-B323 | 566 | post-B323 | 36.346 | 45.849 |
| **129 (this)** | `77f7e46a9` (post-B333) | 594 | 1053 | **33.127** | **37.306** |

The corpus GREW (+4 kernel rules, +8 rust_perf since B323) and the walk grew (+28 files) while the added p50 still dropped −9% flat / −19% narrow — the scratch lever's direction is unambiguous; the exact magnitude against a fixed corpus is not isolated (all three legs ran in one session, minutes apart, same binary pair).

## Where the remaining cost lives (the named next lever)

The added latency is the kernel domain's per-span multi-scale query MASS, not allocation: per span, 1 + significant-lines queries, each paying `query_for` (BM25 tokenization + latent direction build) + a scan over all N=1053 corpus shards (two-stage oversample: O(N) cheap pass + M·k rerank) + BM25 search + rank merge. `src/bin/heal.rs` (1.17 MB, thousands of significant lines) is the p95 driver. Allocation reuse cannot touch this; the structural levers are (a) capping/batching the per-line query count per span, (b) a cheaper first-pass scan key, or (c) the route-narrow shape at reduced K with the scratch already in place. Any of these is a DESIGN change to the retrieval contract — owner-visible, never a silent tuning pass. Filed as the T7 follow-up note in Issue 191.

## Provenance

- Box: m3 (Apple M3 Max, 16 cores, 64 GiB), AC power, battery 100% charged, load 5.62 falling from 8.77 (this session's own builds); one concurrent sibling session (light doc/WIP edits, one remote ssh build — no local compute consumer at run time; no GPU consumer).
- Date: 2026-11-05, runs at 09:24 (off) / 09:32 (on) / 09:40 (narrow) local, minutes apart, three timed passes each after one untimed warmup pass, per-file medians, `top_k=5`, walk = `src/**/*.rs` sorted (594 files, the D2 mix condition: the fix touched zero walk files' SET — two files' contents changed).
- Worktree `riir-refine-i191t7` at `77f7e46a9` (clean develop + the T7 commit); isolated target dirs per posture (`/tmp/g5_t_{off,on,narrow}`); the same p50 definition as Benches 125/126 (nearest-rank on the per-file added distribution, tail support disclosed).
- The runner script's `.exe` paths are Windows-era; the three postures were invoked directly with the script's exact adjudication math (verbatim python) on the three JSONs (`/tmp/g5_work/`).
- ⛔ Known red riding alongside: the narrow posture's retrieval floor decayed PRE-EXISTING on clean develop (blended top-3 0.90697676 < 0.9084507, byte-identical both states) — Issue 207, not this bench's variable.
- Maintained release binary rebuilt at `77f7e46a9e6e` (complete release set latent_retrieval,points,net-mainnet,fix_verify,update_check), installed through the `~/.cargo/bin/cargo-refine` symlink target.
