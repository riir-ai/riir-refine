# Bench 101 — Issue 134 T3 re-arm: the seven-domain demonstrability sweep (thin-domain fixture sets + the degenerate-axis verdict)

**Status:** RECORD — instrument re-armed, verdict REFUSED (degenerate gain axis); `demonstrability_gate` stays default-off (measurement-only).

**Date:** 2026-09-24
**Plan/Issue:** [Issue 134](../.issues/134_demonstrability_selection_gap_gate.md) T3 re-arm
**Predecessor:** [Bench 100](100_demonstrability_t3_depth_sweep.md) (the REFUSED-at-n=3 landing)
**Box:** 4090 workstation (Windows 11, i7-13700K) — build+eval wall only, no GPU compute; quiet-ish (one sibling session).

## 1. What was re-armed

Bench 100's refusal named the triggers: *authored labeled fixture sets for sec/docker/dist/shader_perf (each adds a qualifying domain)*. Landed this session (commit chain below):

- `src/draft/eval_fixtures_sec.rs` — **16 spans** (one per corpus rule)
- `src/draft/eval_fixtures_docker.rs` — **31 spans**
- `src/draft/eval_fixtures_dist.rs` — **6 spans**
- `src/draft/eval_fixtures_shader.rs` — **8 spans**

All hand-authored variants of the corpus anti-patterns (the `eval_fixtures_perf` convention: variant names, canonical shapes, one span per rule), wired into `labeled_eval_spans()` behind their domain features. Four label-verification tests added (`*_fixture_labels_verify`): every label is a REAL corpus rule id, and proposer-covered rules (FIXABLE_SEC / FIXABLE_DOCKER / FIXABLE_SHADER) fire their own proposer on their span.

**Fix-before-repin (the Issue 022 order, honored before any floor moved):** four under-authored docker spans were re-authored to the corpus shape — the first cut carried non-canonical base-image tokens (`FROM debian:bookworm-slim` / `FROM alpine`) where the corpus entries pin `FROM scratch` / `FROM rust:…-slim`; those four tokens are retrieval identity, not the anti-pattern. Docker measured **74.2% → 83.9%** top-1 after the re-authoring. A fifth span (`cache-mount-missing-on-cargo-build`) was narrowed to its OWN trigger (dropped `COPY . .` — manifest-first-layer-split's trigger; the two corpus buggy texts are byte-identical, an honest near-duplicate the fixture set now documents).

## 2. The T1 standing column — seven domains, real spread

Full state (`latent_retrieval,kernel_heal,sec,shader_perf`), 192 labeled spans:

| domain | labeled | top1@28 | top1@56 | top1@112 | constant |
|---|---|---|---|---|---|
| clippy | 48 | 89.6% | 89.6% | 89.6% | **0.935** |
| rust_perf | 53 | 96.2% | 96.2% | 96.2% | **0.962** |
| kernel_opt | 30 | 100.0% | 100.0% | 100.0% | **1.000** |
| sec | 16 | 100.0% | 100.0% | 100.0% | **1.000** |
| docker | 31 | 83.9% | 83.9% | 83.9% | **0.867** |
| dist | 6 | 66.7% | 66.7% | 66.7% | **0.800** |
| shader_perf | 8 | 100.0% | 100.0% | 100.0% | **1.000** |

The clippy/rust_perf constants reproduce Bench 100's parity pins exactly (0.935/0.962). The new spread is real and mechanistic:
- **sec 1.000** — 16 FFI/crypto-shaped anti-patterns with distinctive vocabulary; zero confusion.
- **docker 0.867** — the near-duplicate corpus texts (cache-mount vs manifest-first byte-identical; the apt family overlapping) plus 4 outranked/not-in-top8 misses.
- **dist 0.800** — the three ship-size rules share the bare-`[[bin]]` corpus shape; selection between them is a near-coin-flip (the fixture file documents the cluster).
- **shader_perf 1.000** — 8 WGSL shapes, each syntactically distinctive.

## 3. The verdict — and the artifact the instrument caught in itself

The n=7 Spearman printed **ρ = 0.536 ≥ 0.5 → "PASS"** on the first run. That number is an **artifact**: every gain-from-depth value is EXACTLY 0.0 (all seven curves flat at all three depths), and Spearman against a constant vector is mathematically undefined — the tie-averaged formula renders a pseudo-value determined by the OTHER axis's rank structure alone (arithmetic: 1 − 6·26/336 = 0.536). Promoting on it would be the "satisfied by absent work" class this repo walls against (`timed_region_guard_gate`).

**The repair (landed with this bench):** the sweep gained a **degenerate-axis guard** — `variance_nonzero` on BOTH axes, refusing (neither pass nor fail) when either carries zero spread, with the refusal naming which axis died and why that is itself a finding:

```
VERDICT: REFUSED — degenerate axis: gain-from-depth variance. The Spearman is
undefined against a constant vector … If the GAIN axis is flat everywhere
that is itself the finding: pool depth is a dead allocation axis at this
corpus scale — top-1 is decided within the head-28 in every domain, and
domains whose corpus is smaller than the baseline pool (dist: 6 rules)
cannot gain from depth by construction.
```

**Final verdict: REFUSED (degenerate gain axis), n=7.** The promotion question is unanswerable BY THIS INSTRUMENT at this corpus scale: the treatment (pool depth 28→56→112) has no effect anywhere, so demonstrability's predictive value for depth-allocation cannot be established. `demonstrability_gate` stays default-off; T1 survives as standing instrumentation (now with real spread); T2's own "rounds inert" finding is confirmed one level up — the axis it would reallocate is dead at this scale.

Re-open triggers (recorded in the issue): corpus growth past the pool (a domain whose rule count makes depth 56+ reachable with gold at raw ranks 28–112), or a rerank-head change that makes head-8 composition pool-sensitive.

## 4. Population-growth re-pins (the honest cost, all deltas recorded in-file)

- **BLENDED floors** re-pinned at the weakest state (default, 138 spans): 123/129/131 of 138 (measured 124/130/132). Per-domain clippy/perf/kernel floors UNCHANGED.
- **Router annotation floors** re-measured at all five reachable states: default 110/138 → floor 105/138; +shader 114/146 → 109/146; +sec 87/154 → 82/154; +kernel 105/168 → 100/168; ALL 86/192 → 81/192 (the seven-domain state's Rust-shaped thin domains compound the B131 sec-centroid mechanism — annotation-only, gating stays killed at the 85% bar).
- **rule_embed gate** exact pins re-pinned (population 101 → 138): baseline 90/34/31, trained 95/34/31, G2 win 129 > 124; a loud measured-numbers eprintln added beside the pins.
- **Latent bug found + fixed:** `HeadroomReport.per_domain` was `[_; 3]` through the docker/dist/shader landings — the exact out-of-bounds class `EvalReport`'s sizing law documents ("never fired only because no sec-labeled fixture existed"). The new fixtures fired it (`domain_index(Docker) == 4`, index OOB). Fixed to `[HeadroomMetrics; DomainId::ALL.len()]` + `print_headroom_report` generalized to the full domain table.

## 5. Validation

- `cargo test --features latent_retrieval --lib`: **2457/0**
- `cargo test --features latent_retrieval,kernel_heal,sec,shader_perf --lib`: **2963/0** (813 s)
- Regression gate green at ALL FIVE feature states (default / +kernel / +sec / +shader / ALL)
- `cargo test --features rule_embed_poc --test rule_embed_gate`: 1/0 (re-pinned)
- `cargo test --features latent_retrieval,kernel_heal --test modelability_sweep`: 2/0
- `cargo test --features demonstrability_gate --test demonstrability_gate_wiring`: 3/0
- `cargo test --lib` (default): **2285/0**
- `cargo clippy --all-targets -- -D warnings` / `--all-features --all-targets` / `--no-default-features --all-targets`: **0 errors** each (3 pre-existing Batch-174-era doc_markdown sites healed in passing)
- Default-state T3 sweep: n=4, power-floor refusal (recorded in §2's table shape; the full state is the verdict run)
