# Plan 080 — Batch 109 (post-mining P10): the LTO fold-elimination timing-honesty rule

**Status:** COMPLETE — landed 2026-09-05; the two draft fix-pattern refutations (XOR GF(2)
cancellation; wrapping_add balanced-batch collapse) are recorded in Research 132.
Branch: `develop` · Queue item: 14 (post-mining intake, the single open class — P10, 2026-09-04)
Evidence: katgpt-rs Issue 723 Class A2 fixes (`b4cef008`), `tests/bench_gdsd_modelless.rs` +
`tests/rv_gated_routing.rs` sink docs; the intake entry in `.distill/001_mining_queue_snapshot.md`.

## The class (from the intake)

rustc 1.98.1 + fat LTO eliminates inlined-callee work whose outer result is dead, EVEN THROUGH
`black_box` INSIDE the callee — measured by a direct-call probe in one binary: used result
16.6 µs vs `let _ = f()` ≈ 0. A timing gate whose timed loop discards its work result measures
nothing for its entire release life (the rv gate's 0 ns → NaN% class). The fix patterns proven
in-tree at the finding: (1) xorshift data-dependent work + caller-consumed sinks (`black_box` at
the CONSUMER); (2) a bare CONSTANT work fn still folds through a sink (XOR of a constant 2M times
= 0) — the sink must mix per-iteration data; (3) equal non-pruner work in both arms (one shared
timed fn) so the ratio isolates the delta; (4) interleaved (baseline, candidate) chunks with a
median-of-ratios for bar-adjacent gates.

## Tasks

- [x] Plan first (this file; highwater 79 → 80 verified against the folder at write time).
- [x] Dedup grep — no existing rule covers the timed-harness shape; the black_box hits are
      other rules' bench internals; `debug-stat-full-scan-out-of-hot-path` is the nearest
      neighbor (unconsumed PRODUCTION statistic) and is differentiated in-rule.
- [x] Corpus entry `dead-result-timing-loop-folded-under-lto` — landed with the CHAINED-state
      fix pattern (see Research 132: the XOR-sink and wrapping_add-sink drafts were BOTH
      refuted by the G1 fixture — GF(2) cancellation and balanced-bit additive collapse).
- [x] Loud-baseline assert guidance in-rule, both NaN comparison directions named.
- [x] Standing gates: batch109_rules_present + batch109_pseudocode_parses +
      batch109_concepts_covered + batch109_sink_consumes_per_iteration_data (the two
      anti-shapes pinned as assertions). Count pins walked: header (100/19/14), build-fn doc,
      entries pin, annotated 42→43 ×4 + forty_two→forty_three rename, unannotated message,
      thirteen→fourteen rename + 14-pin + membership assert.
- [x] Fixture: buggy/fixed both parse (syn gate); annotated-membership + NotBenchable
      membership asserts added.
- [x] GOAT: G1 ✓; G2 default 1082/0 + bench 1100/0 + latent 1189/0/1i ✓; G3 clippy 0 real
      findings both states (exFAT noise only) + retrieval_precision_regression_gate PASS,
      floors HELD zero re-pins ✓.
- [x] NOTBENCHABLE-class verdict: no Pareto bench, `bench_spec: None`, pinned by test
      (the concepts gate asserts bench_spec.is_none() under rust_perf_bench).
- [x] Research note `.research/132_p10_lto_fold_timing_honesty.md` (Status: RECORD).
- [x] Docs: AGENTS.md (100 + batch row), `.docs/11_domains/rust_perf.md` (100 + batch line),
      snapshot intake DONE; commit `feat: batch 109: ...` + push.
