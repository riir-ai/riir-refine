# Bench 132 — RUN RECORD: the kernel pregate A/B (Bench 131's registration, executed) — cost leg FAIL at the registered ceilings in BOTH arms, the median leg FLIPPED GREEN, utility floor HOLDS 15/15

**Status:** RUN COMPLETE 2026-11-05 (zed-m3-i208pregate, the Bench-131 lease) — per the
pre-pinned arithmetic the G5-cost fence is **NOT cleared**: added p95 FAIL in both arms.
The pre-registered expectation (Bench 131 §"Expected shape") read exactly this outcome
before any run: "GATE passes p50 ... p95 likely stays on a signaled file ... GATE+SCOPE
exists to attack exactly that leg." The result is the registered contract working, not an
anomaly — no verdict call requested (the mechanism matches Bench 129's own attribution).

## The verdict table

| arm | added p50 ms/file (≤ 1.0) | added p95 ms/file (≤ 5.0) | files > 1 ms | cost verdict | utility floor |
|---|---|---|---|---|---|
| OFF (latent_retrieval) | baseline (abs p50 22.790) | baseline (abs p95 106.415) | — | reference | n/a |
| **GATE** (file-level) | **0.191** ✅ (tail support 300) | **122.481** ❌ (tail support 31) | 195/599 | **FAIL** (p95) | **15/15 HOLDS** (1,483 rank-1 spans; 597/1056 cited) |
| **GATE+SCOPE** (± 24-line) | **0.036** ✅ (tail support 300) | **92.444** ❌ (tail support 31) | 169/599 | **FAIL** (p95) | **15/15 HOLDS** (660 rank-1 spans; 458/1056 cited) |

- Ceilings unchanged (p50 ≤ 1.0 / p95 ≤ 5.0 — the D2 contract, never re-registered).
- **The fence verdict: FAIL — `kernel_opt` stays opt-in.** Promotion still requires BOTH
  fences (G5-cost ∧ G2-GPU-host per Bench 130's registration).
- **The median leg flipped green for the first time**: 33.127 ms (Bench 129 flat) → 0.191
  (GATE) / 0.036 (SCOPE) — a 173×/920× cut at the median. The 404 non-signaled files
  (599 − 195) now pay ≈ detector-only cost; the gate→scope step cuts the median again ~5×.

## The mechanism read (what the p95 tail IS)

The tail-31 files are the kernel-bearing population paying near-full matcher cost — the
kernel-corpus fixture files (`entries_gpu_*`, `src/domains/kernel_opt/**`), whose bodies
ARE kernel code, so the file gate cannot help them and the ±24-line scope trims only their
non-fixture margins (122.5 → 92.4 ms). This is Bench 129's own attribution standing:
per-span multi-scale query mass on real kernel files. The remaining registered gap is
therefore exactly **issue 208's deferred lever (a) — the per-span line-query cap** (or the
substrate score-only retrieval lever, riir-neuron-db's lane). The scope arm's residual
cuts the median but cannot reach the 5 ms tail; the cap bounds the per-span query count
itself.

## Utility (the kill criterion did NOT fire)

- Both arms hold **15/15 distinct rank-1 experts** — the Bench-125 floor holds with the
  gate ON; no needle-widening pre-registration is triggered.
- Census deltas, disclosed: GATE 1,483 rank-1 kernel-cited spans (597/1056 rules cited,
  459 zero-match); SCOPE 660 spans (458/1056 cited, 598 zero-match). Cross-bench context
  (NOT an arm delta — different commit + rule count): flat measured 4,277 spans at the
  1053-rule era (Bench 129). The gate hides shape-matches in non-signaled files — spans
  whose embedding matches a kernel rule without any kernel vocabulary (pattern-shaped
  code). The registered floor is expert reachability, which holds; the span-population
  delta is the honesty disclosure, and it is the pregate working as designed (recall over
  the FILES, precision preserved where it matters).
- **REFUSE disclosure:** the runner's paired rank-1 compare (gate→scope) refused on span
  population drift — mechanically correct: the scope arm queries a strictly smaller span
  set, so the paired-compare shape (built for same-walk same-span postures, flat vs
  narrow) does not apply BETWEEN arms. The per-arm census above is the disclosure.

## Provenance (T1 bar 2)

- Box: m3 (Apple M3 Max, 16 cores, 64 GiB), AC power, battery 100% finishing charge,
  **powermode 2** (High Power), load 3.11 falling from 4.94 at run start, **zero
  cargo/rustc processes** at start (the runs are CPU scan latency — m3 is Bench 129's own
  box; the GPU lane's "m3 never benches" is the 4090 rig contract, not this lane).
- Commit `99f809cca` (develop, the pregate implementation; tree clean at run time; rebased
  over origin 74bafecf4 — B335 landed mid-lane and GREW the corpus: **1,056 rules, 599
  walk files**, both disclosed here).
- Two release builds, isolated target dirs (`/tmp/i208_g5_off`, `/tmp/i208_g5_on`),
  2m07s + 2m09s; the SCOPE arm is the GATE binary + `RIIR_KERNEL_PREGATE_SCOPE=1` (the
  promote-only env, exact literal) — only two builds exist, three runs.
- Runs sequential minutes apart (~21:01–21:20 local): off → gate → scope, one untimed
  warmup pass then three timed passes each, per-file medians, `top_k=5`, the
  nearest-rank p50/p95 with tail support — the Bench-125/129 instrument verbatim
  (`kernel_goat_g5` + the runner script's adjudication math).
- Matcher build (once per process): OFF 437 ms / GATE 2,405 ms / SCOPE 1,977 ms — the
  kernel index build costs ~2 s at boot; boot-time, never per scan. Disclosed.
- Sequential-process caveat as recorded in Benches 125/129: each posture took the median
  of 3 passes; the p50 verdicts sit at 174×/28× inside the ceiling — orders outside the
  +5–22% confound class; the p95 FAIL margin (18×/25× over) likewise. No marginal pass
  exists, so the alone-rerun rule does not trigger.

## Reproduce

```sh
CARGO_TARGET_DIR=/tmp/i208_g5_off cargo build --release --example kernel_goat_g5 --features latent_retrieval
CARGO_TARGET_DIR=/tmp/i208_g5_on  cargo build --release --example kernel_goat_g5 --features latent_retrieval,kernel_opt,kernel_pregate
/tmp/i208_g5_off/release/examples/kernel_goat_g5 --root src --top-k 5 --out /tmp/g5_off.json
/tmp/i208_g5_on/release/examples/kernel_goat_g5  --root src --top-k 5 --out /tmp/g5_gate.json
RIIR_KERNEL_PREGATE_SCOPE=1 /tmp/i208_g5_on/release/examples/kernel_goat_g5 --root src --top-k 5 --out /tmp/g5_scope.json
# then the runner script's adjudication python (scripts/g5_utility_cost.sh) with the three JSONs
```

## What lands from this run (the honest claims)

1. **Claimable now (median-axis):** with `kernel_pregate` armed, the median file's kernel
   cost is 0.19 ms (gate) / 0.04 ms (scope) — the "everyone pays 38 ms" era is over. The
   gate ships as opt-in (`kernel_pregate`) with this bench as its evidence.
2. **NOT claimable:** the G5-cost fence — the registered p95 ceiling stays unmet; no
   ceiling re-registration is requested (forbidden by D2 and 18–25× the wrong direction
   at the tail would refuse anyway).
3. **The next pre-registration** (owner-visible design change, Issue 191 T-sequence):
   the per-span line-query CAP — issue 208's deferred task, now the active path; a fresh
   bench number, same ceilings, same floors.

## Refs

- Bench 131 (this run's pre-registration — nothing in it was amended), Issue 208
  (T4 ticked by this record; T5 verdict recorded), Benches 125/126/129 (the fence's
  measurement history), Bench 130 (the disjoint G2-GPU leg, lease zed-m3-i191t6),
  Issue 191 (the re-campaign sequence: T6 ∧ this lane → G1+G3+G4+G5), riir-neuron-db
  Issue 642 (the substrate scratch twins already adopted; score-only retrieval the
  substrate-side alternative).
