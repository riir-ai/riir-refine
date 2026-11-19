# Bench 162 — Issue 250 T0: the fix-lane cost-model census

**Status:** RECORDED 2026-11-20 (katgpt-rs-93@amo). CPU only. The data is the persisted `blueprint_history.jsonl` VerifyCapture rows (one per `heal fix` run). They were collected fleet-wide: amo (147 rows), 4090 (16) and bmo (1), from 23 repos, deduplicated on (digest, ts). Script, inputs and output are in `162_i250_t0_fix_lane_cost_census_data/`. These are counts only, so no latency is claimed.

## What the runs cost

- **164 fix runs:** 71 ran verify; 29 ran under the `verify: false` posture; the rest had nothing to verify.
- **cargo checks per verify run:** min 2, **median 2**, mean 5.27, max **206**. The max is ONE outlier, a riir-refine run with 207 whole-file reverts; 16 of 71 runs exceeded 2. Total 374 checks, 168 without the outlier.
- **(b) Wasted verifies** (the whole batch reverted, nothing kept): **8 / 71 verify runs = 11.3%** (4.9% of all 164 fix runs). They cost 26 checks (7.0% of all checks; 15.5% without the outlier).
- **(c) Revert rounds:** 8 partial-revert runs + 8 wasted runs; 55 runs reverted nothing. Checks beyond baseline + one clean round: 232, of which 204 are the outlier.
- **Reverted edits:** 306 / 572 written (53.5%; 37% without the outlier).
- **(d) Pool sizes per finding: NOT RECORDED.** No persisted row carries the drafted-vs-verified pool. This matters only for the regeneration-ladder arm, so it is deferred to the shadow-instrumentation step that arm would need.

## The savings ceiling (a perfect pre-verify decline-guard)

A perfect guard declines exactly the edits that would revert. A run with surviving edits then costs the measured floor (2 checks), and a fully doomed run never verifies (0 checks).

| population | actual checks | perfect-guard checks | ceiling reduction |
|---|---:|---:|---:|
| all 71 verify runs | 374 | 126 | **66.3%** |
| without the 206-check outlier | 168 | 124 | **26.2%** |

## Revert concentration (the T1 hint)

| rule (written ≥ 5) | written | reverted | rate |
|---|---:|---:|---:|
| assert_is_empty | 30 | 32 | 1.07 (whole-file attribution, so the stratum over-counts) |
| unnecessary_get_then_check | 48 | 39 | 0.81 |
| explicit_auto_deref | 70 | 10 | 0.14 |
| nine others (doc_markdown, ptr_arg, uninlined_format_args, …) | 8–69 each | 0 | 0.00 |

Reverts are concentrated in two rules. A **per-rule historical revert prior** (the cheapest modelless floor, cheaper than T1's diff-similarity arm) already separates most of the doomed mass. T1 should run that prior as the floor's floor.

## Verdict (T0's pre-registered DOA rule)

**NOT DOA.** The DOA rule is "wasted < ~5% of runs AND revert rounds ≈ 1". Wasted verifies are 11.3% of verify runs, and the ceiling is 26% even without the outlier.

**Derived G3 bar:** a realized compile-invocation reduction of **≥ 10% on the outlier-excluded population** (≈ 38% of its 26% ceiling), with the outlier class (whole-file revert storms) reported separately. That is the "≥ ~10%" T0 asked the number to justify.

**Disclosed limits:**
- n is small: 71 verify runs, 16 with any revert.
- One run dominates the tail.
- (d) is unmeasured.
- The amo / 4090 / bmo populations are pooled.
