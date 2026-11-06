# Bench 140 — RUN RECORD: the Issue-212 default-promotion G5 cost leg — `sec` FAILS (added p50 ~2.0 ms / p95 ~9–10 ms per file, every round), `shader_perf` FAILS NARROWLY (p50 1.10–1.28 / p95 5.30–5.92); both stay opt-in, `ruliology_search` promoted on its compile-only gate

**Status:** RUN COMPLETE 2026-11-06 (katgpt-rs-18-i212). The bar is Issue 212's pre-registered
bar (Issue 191 bar 5 verbatim) + the run protocol pinned in the issue BEFORE measuring
(`5f985649`); nothing was amended after measuring.

## Verdict tables (walk `src/**/*.rs`, 605 files, top_k 5, 3 timed passes per binary, per-file medians, nearest-rank; 3 interleaved rounds)

Added ON−OFF matcher latency per file (ms). Ceilings p50 ≤ 1.0, p95 ≤ 5.0; PASS needs all 3 rounds.

| feature | round | added p50 (support) | added p95 (support) | files > 1 ms | walk total OFF → ON | round |
|---|---|---|---|---|---|---|
| `sec` | 1 (OFF first) | 2.155 (304) | 10.306 (31) | 444/605 | 22235 → 24531 ms | FAIL |
| `sec` | 2 (ON first) | 1.950 (304) | 9.837 (31) | 430/605 | 22493 → 24617 ms | FAIL |
| `sec` | 3 (OFF first) | 2.131 (304) | 9.180 (31) | 433/605 | 22506 → 24648 ms | FAIL |
| `shader_perf` | 1 | 1.098 (304) | 5.297 (31) | 321/605 | 22621 → 23811 ms | FAIL |
| `shader_perf` | 2 | 1.278 (304) | 5.644 (31) | 351/605 | 22508 → 23802 ms | FAIL |
| `shader_perf` | 3 | 1.255 (304) | 5.923 (31) | 345/605 | 22494 → 23784 ms | FAIL |

- **`sec`: FAIL** — both ceilings, every round, at ~0.5× headroom (≈ 2× over). Walk total +9.5%.
- **`shader_perf`: FAIL (narrow)** — both ceilings, every round, at 0.8–0.9× headroom. Walk total +5.7%.
- **A/A noise floor (same-posture binaries across rounds, same run):** added p50 −0.06…0.18 ms,
  p95 0.79…1.82 ms. `sec`'s effect is far outside it on both statistics; `shader_perf`'s p50
  (1.10–1.28) is outside it, its p95 overshoot (0.3–0.9 ms) is INSIDE the p95 noise band — the
  p50 axis alone carries its FAIL. Per the bar: "Fail → stays opt-in with the axis named; never
  re-register the ceiling."
- **The axis, both features:** the domain's corpus is indexed into the production
  `LatentMatcher` and joins EVERY `.rs` span's fan-out. For `shader_perf` this is structurally
  wasted work — its corpus is WGSL render-stage code and `src` carries 0 `.wgsl` files; scoping
  the shader index to `.wgsl` inputs (the `kernel_pregate` shape) is the named lever. For `sec`
  the rules are Rust, so the lever is a file/span pregate on sec signals (FFI/`unsafe`/Drop
  needles) — same shape, needs its own recall check.
- **Not in the bar (disclosed, unmeasured):** the `--fix` lane's `FIXABLE_SEC` proposer pass; the
  shader `.wgsl` lane (`run_shader_lane`, never reached by this walk).

## G3 — existing domains' score-bench verdicts ON vs OFF (`scripts/g3_scorebench_stability.sh <feature>`)

Run on amo (non-timed; isolated target dir `~/i212/g3t`, the OFF/ON postures `--features
score_bench` vs `+<feature>`), at `5f985649`, 19:55–20:2x:

| feature | clippy lane (v1) | perf lane (gen-5) | G3 |
|---|---|---|---|
| `sec` | BYTE-STABLE (41 verdict lines) | BYTE-STABLE (68 verdict lines) | PASS |
| `shader_perf` | BYTE-STABLE (41 verdict lines) | BYTE-STABLE (68 verdict lines) | PASS |

G3 holds for both — the cost leg alone keeps them opt-in.

## ruliology_search

No scan fan-out (lib-only re-export, no bin consumer) → compile-only gate per the issue: default
`cargo clippy --all-targets -- -D warnings` exit 0, release-set
(`latent_retrieval,points,net-mainnet,fix_verify,update_check,kernel_pregate`) clippy
`--all-targets -D warnings` exit 0, default `cargo test --lib` 3475 passed / 0 failed / 1 ignored (m3, compile+test only, not timed); build_stamp tests 14/0.
**PROMOTED to `default`** (the promotion commit). RELEASE_FEATURES / release.yml /
manual_release.sh unchanged: crate defaults ride every release build, and the feature has zero
deps (the kernel_pregate release-set listing existed for cargo-about's license graph; nothing to
add here).

## PROVENANCE

- **Rig deviation (disclosed):** the bar names `bmo --exclusive`. bmo was OFFLINE the whole
  session (`ssh: connect … Operation timed out`, `agent-mail rig status` OFFLINE ×4 between
  18:55 and 19:34); 4090 held a 955-min exclusive job; amo carried the ~36 h sr1098 campaign
  (shared job, head-of-line behind a queued exclusive), so no exclusive slot existed. Per the
  dispatch instruction the run went to **amo, directly over ssh (not the queue), disclosed**.
- Box: amo — Apple M5 Ultra, 96 GB, macOS 27.2, AC. Preflight (riir-reflex
  `bench_preflight.sh`) **REFUSED** at every checkpoint:
  `PROVENANCE: power=AC Power load=9.01 swap=56157.19M canary=skipped powermode=0(auto)` (start
  19:34), `load=12.27 swap=55840.81M` (between features 19:44), `load=11.21 swap=57198.25M`
  (end 19:53). Contenders at start: sr1098 `seal-viewer-treat` 162% + `seal-viewer-ctrl`
  157% + 92% CPU, Zed 35%, `probe_realctx_i1179` 19%. 30 cores, so the single-threaded harness
  had idle cores, but the load numbers above are what they are; the interleaved rounds + the A/A
  floor are the mitigation, not a substitute for a quiet rig.
- Binaries: built on m3 (compile only) from the riir-refine tree at `5f985649` (+ the
  comment-only T5 Cargo.toml diff later committed as `76b6af81`), `--release --example
  kernel_goat_g5`, features `latent_retrieval` (OFF), `+sec`, `+shader_perf` (ON); siblings at
  katgpt-rs `634424c11`, riir-neuron-db `04fc3c8`, riir-kat `52d7571`, riir-auth `f4ba282`,
  riir-reflexer `8cad872`, riir-ledger `f4f5bfa` (all = origin/develop, clean). Each JSON's
  `compiled_features` build stamp confirms OFF/ON differ by exactly the feature under test.
- Walk: amo worktree of riir-refine at `5f985649`, `src` = 605 `.rs` files (603 at `370854fe`;
  the sibling batch-338 commit added files before the measured commit).
- Instrument note: the first adjudication of `shader_perf` REFUSED (the build stamp spells
  features `shader-perf`); the guard's comparison was normalised (`_`↔`-`) in the promotion
  commit and the SAME JSONs re-adjudicated — no re-measurement.
- Runs 19:34–19:53, rounds alternate binary order (odd OFF first, even ON first).

## Reproduce

```sh
sh scripts/g5_domain_cost.sh --feature sec --out-dir /tmp/g5_sec          # base latent_retrieval
sh scripts/g5_domain_cost.sh --feature shader_perf --out-dir /tmp/g5_shader
sh scripts/g3_scorebench_stability.sh sec   # and shader_perf
```

## Refs

Issue 212 (the registration + protocol), Issue 191 bar 5 (the cost definition), Bench 137 (the
kernel_pregate precedent the levers above follow), Bench 001 (ruliology_search G1–G4).
