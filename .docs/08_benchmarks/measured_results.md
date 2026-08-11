# Measured GOAT gate results

**Captured:** 2026-08-11
**Hardware:** Apple M3 Max (release mode)
**Commit:** `5b46556` (`feat: wire L3 ruliology Pareto-front candidate search (Issue 005)`)

## Summary table

| Layer | Gate | Target | Measured | Status |
|---|---|---|---|---|
| L1 matvec | G1 determinism | bit-identical | bit-identical | ✅ |
| L1 matvec | G2 latency | p50 < 10µs | **2.250µs** | ✅ |
| L1 matvec | G3 no-regression | L0 fallback unchanged | CorpusTemplate, conf=0.5 | ✅ |
| L1 matvec | G4 alloc-free | capacity stable | 256→256, 64→64 | ✅ |
| L2 syn parse | G1 correctness | 16/16 deterministic | 16/16 | ✅ |
| L2 syn parse | G2 latency | p50 < 100µs | **5.459µs** | ✅ |
| L2 syn parse | G3 no-regression | real parse, not stub | valid=true, invalid=false | ✅ |
| L2 syn parse | G4 alloc-bounded | diagnostics bounded | success empty, failure ≤1 | ✅ |
| Phase 3 verify | G1 correctness | JSON identifies lint | correctly parses | ✅ |
| Phase 3 verify | G2 latency | documented | **~130ms** | ✅ |
| Phase 3 verify | G3 no-regression | default passes | green | ✅ |
| Phase 3 verify | G4 alloc-free | N/A | N/A | ✅ |
| L3 search | G1 correctness | Pareto + min-complexity | all 3 sub-checks | ✅ |
| L3 search | G2 latency | < 10ms | **392.96µs** | ✅ |
| L3 search | G3 no-regression | 10/10 searchable | 10/10 | ✅ |
| L3 search | G4 alloc-free | N/A | N/A | ✅ |

## Composite pipeline latency

For a single clippy lint invocation (one lint, one fix candidate):

| Layer | When | p50 latency |
|---|---|---|
| L1 drafter | per lint | 2.25µs |
| L2 pruner | per drafted fix | 5.46µs |
| L3 search (opt-in) | per lint, if multiple valid fixes | 393µs |
| Phase 3 verify (opt-in) | per lint, on-demand | ~130ms |

**Hot path (L1+L2, default features):** **7.71µs** per lint.

## Measurement methodology

- Apple M3 Max (Darwin), release mode
- 10,000 iterations for latency gates (p50/p99/mean)
- 1,000 iterations for alloc-free gates (capacity stability)
- Determinism gates: two consecutive runs, bit-identical comparison
- No CPU pinning (laptop measurement, not controlled bench environment)
- Results are representative, not authoritative

## Issue 004 (self_evolve)

The Issue 004 (`self_evolve`) GOAT results (G1-G5) are captured in the
`10_self_evolve/` folder. The real-world validation (G6/G7, Issue 008)
is in `.benchmarks/008_real_world_validation.md`.

## See also

- [`goat_methodology.md`](goat_methodology.md) — the 4-gate pattern
- [`../../.benchmarks/001_goat_consolidated.md`](../../.benchmarks/001_goat_consolidated.md) — the persisted record
- [`../../.benchmarks/008_real_world_validation.md`](../../.benchmarks/008_real_world_validation.md) — self_evolve G6/G7
