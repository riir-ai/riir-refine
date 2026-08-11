# Bench 001 — Consolidated GOAT Gate Results (L1 + L2 + Phase 3 + L3)

**Captured:** 2026-08-11
**Hardware:** Apple M3 Max (release mode, `cargo bench` / `cargo run --release`)
**Commit:** `5b46556` (`feat: wire L3 ruliology Pareto-front candidate search (Issue 005)`)
**Branch:** `develop`

This record persists the GOAT gate measurements for the four committed
layers of the clippy-healing pipeline. Each gate was measured by running
its dedicated bench/example target in release mode on a clean tree.

The Issue 004 (`self_evolve`) GOAT results will ship in a separate record
once that work commits.

---

## L1 — Ternary SIMD matvec drafter (Issue 001)

**Bench target:** `benches/bench_001_l1_matvec.rs` (`cargo bench --bench bench_001_l1_matvec`)
**Feature gate:** default-on (no feature flag)
**Hot path:** yes — called per clippy lint invocation

| Gate | What | Target | Measured | Status |
|---|---|---|---|---|
| G1 (determinism) | same input → identical output (code + confidence bits + source) | bit-identical | bit-identical | ✅ PASS |
| G2 (latency) | matvec p50 @ cols=256, rows=64, 10K iters | p50 < 10µs | **p50 = 2.250µs** (p99 = 2.917µs, mean = 2.302µs) | ✅ PASS |
| G3 (no-regression) | L0 fallback path unchanged (CorpusTemplate, conf=0.5) | source=CorpusTemplate, conf=0.5 | source=CorpusTemplate, conf=0.500, code len=19 | ✅ PASS |
| G4 (alloc-free) | scratch buffer capacity stable across 1000 calls | capacity unchanged | input_scratch 256→256, output_scratch 64→64 | ✅ PASS |

**Verdict:** L1 ships at **2.25µs p50** — 4.4× under the 10µs target. Zero
steady-state allocation. The ternary SIMD matvec path is production-ready.

---

## L2 — Syntax validator (Issue 002)

**Bench target:** `benches/bench_002_l2_pruner_syn.rs` (`cargo bench --bench bench_002_l2_pruner_syn`)
**Feature gate:** default-on (no feature flag — `syn` is always in the dep tree)
**Hot path:** yes — called per drafted fix

| Gate | What | Target | Measured | Status |
|---|---|---|---|---|
| G1 (correctness) | deterministic parse classification (8 valid + 8 invalid snippets) | valid→parses=true, invalid→parses=false, deterministic | 16/16 correct + deterministic | ✅ PASS |
| G2 (latency) | parse check p50 @ ~40-byte snippet, 10K iters | p50 < 100µs | **p50 = 5.459µs** (p99 = 7.167µs, mean = 5.592µs) | ✅ PASS |
| G3 (no-regression) | real parse check, not stub (stub always returned parses=true) | valid parses + invalid rejects + no stub regression | valid=true, invalid=false, stub regressed=no | ✅ PASS |
| G4 (alloc-bounded) | diagnostics empty on success, bounded on failure (1000 iters) | success empty, failure ≤2 entries | success all empty (max=0), failure bounded (max=1) | ✅ PASS |

**Verdict:** L2 ships at **5.46µs p50** — 18.3× under the 100µs target.
The `syn`-based parse check is the modelless validation backbone.

---

## Phase 3 — Lint resolution verification (Issue 003)

**Bench target:** `examples/clippy_verify_demo.rs` (`cargo run --example clippy_verify_demo --features clippy_verify --release`)
**Feature gate:** `clippy_verify` (opt-in — slow, requires cargo+clippy on PATH)
**Hot path:** no — invoked on-demand for verification, not per-lint

| Gate | What | Target | Measured | Status |
|---|---|---|---|---|
| G1 (correctness) | `cargo clippy` JSON parsing identifies the target lint | lint_name extracted from JSON | correctly parses clippy JSON output | ✅ PASS |
| G2 (latency) | per-verify latency (temp dir + cargo clippy re-run) | documented, not hot-path | **~130ms/verify** | ✅ PASS (documented) |
| G3 (no-regression) | default features (no `clippy_verify`) still pass | default `cargo test --lib` green | green | ✅ PASS |
| G4 (alloc-free) | N/A — runs once per verification, not per-tick | N/A | N/A | ✅ PASS (documented N/A) |

**Verdict:** Phase 3 ships at **~130ms/verify** — not a hot-path gate.
The feature stays opt-in because it shells out to `cargo clippy` (seconds-scale
external process). The fast L2 `syn` check (5.46µs) is the always-on validation.

---

## L3 — Ruliology Pareto-front candidate search (Issue 005)

**Bench target:** `examples/ruliology_search_demo.rs` (`cargo run --example ruliology_search_demo --features ruliology_search --release`)
**Feature gate:** `ruliology_search` (opt-in — refinement layer)
**Hot path:** no — runs once per lint invocation when multiple valid fixes exist

| Gate | What | Target | Measured | Status |
|---|---|---|---|---|
| G1 (correctness) | Pareto-front filters dominated candidates; minimum-AST-delta fix picked | front=[2,3] (not 0,1); simpler < complex (2<4); best_fix picks min-complexity | all three sub-checks pass | ✅ PASS |
| G2 (latency) | search over 10 candidates | < 10ms (not hot-path) | **392.96µs** (returned 10 scored candidates) | ✅ PASS |
| G3 (no-regression) | seed corpus (10 lints) all searchable | all 10 return a best_fix | 10/10 searchable | ✅ PASS |
| G4 (alloc-free) | N/A — runs once per invocation, not per-tick | N/A | N/A | ✅ PASS (documented N/A) |

**Verdict:** L3 ships at **393µs/10-candidate search** — 25.5× under the
10ms target. The Pareto-front algorithm is reimplemented locally (~30 LOC)
from `katgpt-ruliology` (Bench 572) — no code-level dep on the substrate
because clippy fixes have ground-truth payoff (lint_satisfied: bool), not
tournament payoffs.

---

## Composite pipeline latency budget

For a single clippy lint invocation (one lint, one fix candidate):

| Layer | When | p50 latency |
|---|---|---|
| L1 drafter | per lint | 2.25µs |
| L2 pruner | per drafted fix | 5.46µs |
| L3 search (opt-in) | per lint, only if multiple valid fixes | 393µs |
| Phase 3 verify (opt-in) | per lint, on-demand verification | ~130ms |

**Hot path (L1+L2, default features):** **7.71µs** per lint — well under
any perceptible latency. The opt-in layers (L3 + Phase 3) are not in the
hot path.

---

## Measurement methodology

- All measurements taken on Apple M3 Max (Darwin) in release mode
  (`cargo bench` for L1/L2, `cargo run --release` for L3/Phase 3 examples)
- 10,000 iterations for latency gates (p50/p99/mean)
- 1,000 iterations for alloc-free gates (capacity stability)
- Determinism gates: two consecutive runs, bit-identical comparison
- No CPU pinning (laptop measurement, not a controlled bench environment)
- Results are representative, not authoritative — production hardware may differ

## Re-running

```sh
# L1 (hot path)
cargo bench --bench bench_001_l1_matvec

# L2 (hot path)
cargo bench --bench bench_002_l2_pruner_syn

# L3 (opt-in refinement)
cargo run --example ruliology_search_demo --features ruliology_search --release

# Phase 3 (opt-in verification — requires cargo+clippy on PATH)
cargo run --example clippy_verify_demo --features clippy_verify --release
```

## References

- [Issue 001](../.issues/) — L1 ternary matvec (resolved + removed per noise-reduction)
- [Issue 002](../.issues/) — L2 syn pruner (resolved + removed per noise-reduction)
- [Issue 003](../.issues/003_phase3_clippy_lint_resolution_verify.md) — Phase 3 clippy verify
- [Issue 005](../.issues/005_l3_ruliology_pareto_front_candidate_search.md) — L3 ruliology search
