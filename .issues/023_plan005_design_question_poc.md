# Issue 023 — POC: the two Plan 005 design questions, measured

**Status:** RESOLVED (both questions answered; Plan 005 refined accordingly)
**Plan:** [005](../.plans/005_real_file_span_healing.md) Phases 9 + 11
**POC:** `tests/poc_023_plan005_design_questions.rs` (2 tests, release, 3 repeat runs)
**Date:** 2026-08-16

A review of Plan 005 raised two objections against the *unstarted* Phases 9
and 11. Both were arithmetic doubts, not opinions — so both were decidable by
measurement instead of debate. This POC is the measurement. It runs on default
features (`conformal_predictive_intervals` + `poincare_navigator` are both
DEFAULT-ON in katgpt-core, and riir-clippy already deps katgpt-core with
default features — **no new dependency was needed for either phase**, which is
itself a correction to T9.2's stated dep story).

---

## Q1 (Phase 9) — how many residuals before a per-rule 95% interval is real?

**The doubt.** T9.2 proposes `ConformalIntervalCalibrator<FixForecaster>` with a
**per-rule** residual ring of capacity 64; T9.5 gates on "empirical coverage of
the 95% interval ≥ 90%". Split conformal reads the `⌈(n+1)(1−α)⌉`-th order
statistic, which is only defined when `⌈0.95(n+1)⌉ ≤ n`, i.e. **n ≥ 19**. So the
gate looked unmeasurable until every rule key had ~19 verified fixes.

**Measured** (5 seeds averaged, 4000 held-out evals per point, deterministic LCG):

| n residuals | coverage | mean width | T9.5 gate |
|---:|---:|---:|:---|
| 0 | 0.0% | 0.0000 | FAIL |
| 1 | 0.0% | 0.0000 | FAIL |
| 2 | 27.6% | 0.3008 | FAIL |
| 4 | 59.0% | 0.6010 | FAIL |
| 8 | 80.7% | 1.1480 | FAIL |
| 16 | 88.9% | 1.2391 | FAIL |
| **19** | **92.2%** | 1.2626 | **PASS** |
| 24 | 95.8% | 1.2918 | PASS |
| 32 | 97.5% | 1.3291 | PASS |
| 48 | 95.1% | 1.2708 | PASS |
| 64 | 96.3% | 1.2952 | PASS |

**Verdict: CONFIRMED, and the empirical threshold is exactly the analytic one.**
The measured crossing is **n = 19** — identical to the `⌈(n+1)(1−α)⌉ ≤ n` bound,
across all 3 repeat runs. The generative model was deliberately made *friendly*
(a well-calibrated drafter, zero recency decay); a miscalibrated drafter needs
MORE residuals, so 19 is a lower bound.

**The dangerous part is n < 2, not n < 19.** The shipped
`weighted_quantile_pair` returns `(0.0, 0.0)` on an empty pool, so a cold
calibrator does not emit a wide, honestly-uncertain interval — it emits a
**zero-width interval centred on the point forecast**, i.e. a silent claim of
perfect confidence, measured at **0.0% coverage**. That is a false-confidence
signature, exactly the thing the "Report the Floor" rule exists to catch. Any
Phase 9 implementation MUST refuse to emit an interval below the warm-up count
rather than emit the degenerate one.

**Consequence for the plan.** Per-rule keying across the 38 `--fix`-eligible
rules (33 `FIXABLE_LINTS` + 5 perf proposers) needs **~722 verified fixes**
before T9.5 is measurable at all. Plan 005 T9.2 is amended to key the
calibrator **per-domain** (3 keys → ~57 fixes) with per-rule as an opt-in
upgrade once the trajectory corpus is deep enough, and T9.5 gains an explicit
warm-up guard + the conformal-naive floor.

---

## Q2 (Phase 11) — is the L2 validator actually the bottleneck?

**The doubt.** T11.5 asserts "the imagination gate is µs-scale; the L2 pruner is
ms-scale; net throughput rises if imagination rejects ≥20% of bad fixes", and
T11.6 gates at ≥30% rejection **and ≥1.5× latency**. But riir-clippy's default
L2 is `RustAnalyzerPruner`, whose fast path is an **in-process `syn` parse**,
not a `cargo` subprocess. If L2 is µs-scale, it cannot be the bottleneck the
gate assumes.

**Measured** (median of 200 reps over 5 real spans, 3 repeat runs):

| stage | cost per fix | source |
|---|---:|---|
| L2 (`syn` parse + owned-String handoff) | **18.9 – 23.0 µs** | this POC |
| imagination gate (`poincare_navigate_into` + dot + sigmoid) | **0.09 – 0.11 µs** | this POC |
| retrieval (chunk + embed + route + retrieve) | 349 µs | Bench 018 G2 |

**Verdict: CONFIRMED — T11.5's premise is false by ~43×, and T11.6's latency
gate is arithmetically unreachable.**

L2 is **~19-23 µs, not ms**. It is ~5-6% of the ~370 µs per-fix pipeline;
retrieval is ~94% of it. Imagination only avoids an L2 call on the fixes it *rejects*, so:

```
speedup = T_base / (T_base − base_rate · recall · T_L2 + T_imagination)
```

| bad-fix rate | recall | speedup |
|---:|---:|---:|
| 0.05 | 0.30 | 1.0007× |
| 0.30 | 0.30 | 1.0053× |
| 0.50 | 1.00 | 1.0316× |
| **1.00** | **1.00** | **1.05 – 1.07×** |

**Even if every proposed fix were bad and imagination caught 100% of them, the
ceiling is ~1.06× — against a 1.5× gate.** The gate cannot pass. This is not a
tuning problem; it is the wrong denominator.

**Consequence for the plan.** Plan 005 Phase 11's **latency** justification is
DEAD and is removed. The phase is rescoped to a **quality**-only claim (does
imagination reject bad fixes the visitor admits?) and demoted below Phase 12,
with an explicit note that any future latency claim must target **retrieval**
(349 µs, 94% of the budget), not L2. If the quality claim also fails, the phase
closes as a negative result.

Two further defects found while writing the POC:

1. **T11.5 cites an API that does not exist.** The plan writes
   `poincare_navigate_into(z_src, src_pos, dst_pos, z_out)`. The real signature
   is `poincare_navigate_into(z_src, delta_target, adapter, z_out, phi_scratch,
   hidden_scratch)` — a *delta* target plus a fitted adapter and two scratch
   buffers, not a src/dst position pair.
2. **T11.6 mislabels its own metric.** "accepting ≥90% of would-pass-L2 fixes"
   is specificity / true-negative rate, not precision.

---

## Reproduce

```bash
cargo test --release --test poc_023_plan005_design_questions -- --nocapture --test-threads=1
```

Both tests assert their load-bearing conclusions (zero-width cold interval;
sub-ms L2; sub-1.5× Amdahl ceiling), so they stand as regression witnesses: if
someone later makes L2 genuinely expensive (e.g. promotes `clippy_verify` /
`fix_verify` to the default `--fix` path), `poc023_q2` goes red and Phase 11's
latency claim becomes re-arguable on evidence.
