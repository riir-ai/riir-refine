# Issue 212 — promote `sec` + `shader_perf` (+ `ruliology_search`) to default: the G5 fan-out cost leg first

**Status:** OPEN 2026-11-06 (katgpt-rs-18) — filed from the 11-06 feature audit under the owner law "any gain no perf hurt must enabled by default, dont make good goat sad"; measurement lane dispatched same day.

## Why

The feature audit (68 features classified, report-only) found three opt-in features with a GOAT PASS on record that are neither crate-default nor in the shipped release set:

| feature | GOAT evidence | held by |
|---|---|---|
| `sec` | Bench 046:99-105 Phase 0 + Phase 1 PASS; zero deps; 44 rules | the retired "consumer pull" rule |
| `shader_perf` | Bench 093 GO (G5-real PASS, byte-identical render pre/post); Bench 095 tier shipped | the retired "consumer pull" rule (shader-toy editor) |
| `ruliology_search` | Bench 001:79-86 G1–G4 PASS; zero deps | lib-only re-export (`src/lib.rs`), no bin consumer — low impact |

The "consumer pull" hold is superseded by the 11-06 owner law. The one gap the law itself names is **perf hurt**: a new domain joins EVERY scan's fan-out, so the added scan cost is unmeasured — the kernel_pregate precedent (Bench 137) is the shape.

## Pre-registered bar (pinned BEFORE any measurement)

Per feature (sec, shader_perf), the Issue-191 bar-5 cost definition reused verbatim: added ON−OFF scan latency per file over the same walk (`src/**` + the pinned `.raw` vendor set if present; 600-file walk, 3 timed passes, per-file medians, nearest-rank), **p50 ≤ 1.0 ms/file AND p95 ≤ 5.0 ms/file**, `--release`, quiet rig (`agent-mail rig run bmo --exclusive`), PROVENANCE line beside every number. Plus G3: the existing domains' score-bench verdicts byte-stable ON vs OFF (`scripts/g3_scorebench_stability.sh` shape). Marginal (< 2× headroom) → re-run alone. Fail → stays opt-in with the axis named; never re-register the ceiling.

`ruliology_search`: no scan fan-out (no bin path) — promotion gate is compile-only (default lib tests + clippy −D clean).

## Tasks

- [ ] T1 the cost harness for an arbitrary domain feature (generalize `examples/kernel_goat_g5.rs`'s cost leg, or a sibling example) — consume, never fork
- [ ] T2 run sec + shader_perf on bmo exclusive; record a benchmark with PROVENANCE
- [ ] T3 G3 byte-stability ON vs OFF per feature
- [ ] T4 promote the passers in ONE commit: `default` + RELEASE_FEATURES + release.yml (build AND licenses lines) + manual_release.sh in lockstep (build_stamp tests pin them); Cargo.toml comments + AGENTS.md rows; demote the failers' comments to name the failing axis
- [ ] T5 side fixes from the audit: `Cargo.toml` elo_k_eff comment cites "Issue 190" (double-allocated number — Mozilla grant issue vs the elo item; re-point to the commit/plan that owns it); disclose in docs that the shipped binary's `self_evolve` EMA-learning half is inert without `clippy_verify` (memory half runs)
