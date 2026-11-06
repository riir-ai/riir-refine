# Issue 212 — promote `sec` + `shader_perf` (+ `ruliology_search`) to default: the G5 fan-out cost leg first

**Status:** IN PROGRESS 2026-11-06 (katgpt-rs-18 lane) — T1 harness landed; run protocol pinned below BEFORE any measurement; T2/T3 dispatched to a quiet rig. Filed from the 11-06 feature audit under the owner law "any gain no perf hurt must enabled by default, dont make good goat sad".

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

## Run protocol (pinned 2026-11-06 BEFORE any measurement — refines HOW, never the ceilings)

- **Base (OFF) posture** = crate defaults + `latent_retrieval` (the shipped scan's matcher; the release set carries `latent_retrieval`, and `kernel_pregate` is already default). ON = base + the feature. The runner REFUSES unless the two build stamps (`compiled_features`) differ by exactly the feature under test.
- **Walk** = `src/**/*.rs` at the measured commit (603 files at `370854fe`; the "600-file" figure was Bench 137's count of the same walk). No pinned `.raw` vendor set exists in the repo → `src` alone, disclosed. `src` carries 0 `.wgsl` files, so shader_perf's `.wgsl` render lane (`run_shader_lane`) is outside this walk by construction — the walk prices its MATCHER fan-out on `.rs` spans (the every-scan cost); the `.wgsl` lane only runs on `.wgsl` inputs.
- **Interleaving** (katgpt-rs sequential-A/B law): 3 ROUNDS per feature, alternating binary order (odd rounds OFF first, even ON first); each round is a full instance of the bar (3 timed passes per binary, per-file medians, nearest-rank); **verdict PASS only if ALL 3 rounds pass** (stricter than one run, never looser). Marginal = any passing round under 2× headroom on p50 or p95 → re-run alone (bar text).
- Not in the bar, disclosed only: the `--fix` lane's sec proposer pass (`FIXABLE_SEC`) — a fix-time cost, not a scan fan-out.

## Tasks

- [x] T1 the cost harness for an arbitrary domain feature — `examples/kernel_goat_g5.rs` consumed (adds the `compiled_features` build stamp to both emitters), `scripts/g5_domain_cost.sh --feature <f>` the generic interleaved runner, `scripts/g5_cost_math.py` the ONE copy of the medians/nearest-rank math (g5_utility_cost.sh now imports it), `scripts/g3_scorebench_stability.sh [feature]` generalized for T3
- [ ] T2 run sec + shader_perf on bmo exclusive; record a benchmark with PROVENANCE
- [ ] T3 G3 byte-stability ON vs OFF per feature
- [ ] T4 promote the passers in ONE commit: `default` + RELEASE_FEATURES + release.yml (build AND licenses lines) + manual_release.sh in lockstep (build_stamp tests pin them); Cargo.toml comments + AGENTS.md rows; demote the failers' comments to name the failing axis
- [x] T5 side fixes from the audit: `Cargo.toml` elo_k_eff comment cites "Issue 190" (double-allocated number — Mozilla grant issue vs the elo item; re-point to the commit/plan that owns it); disclose in docs that the shipped binary's `self_evolve` EMA-learning half is inert without `clippy_verify` (memory half runs) — DONE: the Cargo.toml + AGENTS.md `elo_k_eff` rows re-pointed to the owning commit `280f15766`; the release-set inert-EMA disclosure added to the Cargo.toml `self_evolve` comment + the AGENTS.md row
