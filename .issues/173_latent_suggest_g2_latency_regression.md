# 173 — `latent_suggest_goat` G2: per-chunk latency 1.6× over the macOS bar at its own feature set
**Status:** OPEN — measured 2026-10-26 (from Issue 172 T3); root cause not yet profiled

## Measurement
Box: amo (M5 Ultra, AC power), quiet: no other cargo/rustc, load 3.9 on 30
cores. `--release`, `g2_latency_bar_per_chunk` alone, 480 chunks, two runs each.

| feature set | µs/chunk | bar (macOS) |
|---|---|---|
| target's own `latent_retrieval,clippy_lints,rust_perf` | **1592, 1586** | 1000 |
| `--all-features` (ci_feature_guard Layer 5c) | **4830, 4832** | 1000 |

Two separable effects:
1. **Own-set regression (~1.6×):** the `< 1 ms` bar is the M3 / Plan-005
   calibration; this box is faster than an M3, yet it misses by 59%. The
   suspect is retrieval cost scaling with mined-corpus growth (rust_perf
   198 → 200+ at batch 239, `4085aa24`, and the mining lane keeps adding rows).
   **Not verified:** the first step is a profile of `LatentMatcher::suggest_span`,
   not a guess.
2. **All-features posture (~3× on top):** `from_enabled_domains()` also
   indexes kernel_opt/sec/shader_perf, so the bar's population differs. The
   bar needs a posture-aware form (the Issue 143 dual-pin precedent), or a
   per-domain budget.

## Tasks
- [ ] T1 — profile `suggest_span` at the own feature set (samply or Instruments); name the hot term (route vs fan-out retrieve vs scoring), and check whether it is O(rules).
- [ ] T2 — fix the hot term (index or prefilter) and re-measure against the 1000 µs bar. **No bar re-pin to hide the regression.**
- [ ] T3 — posture-aware G2 for the all-features build (dual pin with measured headroom, or per-enabled-domain budget), measured alone and with the box state recorded.
