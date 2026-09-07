# Research 147 — Batch 124: arXiv:2609.04040 matched-control evaluation-design distill

**Status:** RECORD — Batch 124 landed 2026-09-07 (M3 box, CPU-only).

## Source

arXiv:2609.04040 "Confidence-Gated Admission for Hardware Prefetching: When the Gate Matters More Than the Predictor" (Majdane, Casartelli, Lopedoto; cs.AR/cs.PF, 2026-09-03) — the walk-#8 headline. No code repo mined; the rule is an EVALUATION-DESIGN distill (the ab-isolate lineage), so the source citations are the paper's own measured claims, quoted verbatim in-rule.

## The rule

`matched-control-disentangles-policy-from-model` (rust_perf io, NotBenchable): a learned predictor benchmarked against an always-issues baseline conflates the MODEL with the ADMISSION POLICY. Three decision contents:

1. **Matched controls** — the same admission gate wraps BOTH the learned model and the classical baseline; the paper's control made a 257-parameter online MLP indistinguishable from gated stride on random traffic and SLOWER on most regular streams — the neural advantage was the gate all along.
2. **Proxy-vs-endpoint** — the gate alone removed 35% of prefetches and lifted accuracy 11%→15% while DRAM reads changed only ±0.07%: better proxies do not imply better endpoints; the report carries both and the win condition is the endpoint.
3. **Null-arm identity** — "We prove gate-closed execution reproduces the no-prefetch baseline exactly": an arm you cannot prove inert when off can corrupt the experiment (the admission-gate twin of feature-flag bit-identity).

## Workspace hooks (why this rule earns its slot)

- External confirmation of the gate-first design law: Issue 880's kill-switch (no consumer → no prefetch), `evpi_gate`, convergence-cadence exits, BandEdgeTrigger's enabled-gate.
- The honesty clause ("better proxies do not imply better endpoints") is the same instinct as the lossy-surface promotion rule (aggregate metrics flat while family-conditional behavior flips) and B109's dead-result rule (the instrument lies).
- Distinct-from ledger: B121 ab-isolate (artifact isolation vs policy/model separation), Batch 123 full-vector-bit-identity (identity semantics vs comparison meaningfulness), B109, B122 diagnostics.

## Gates

- `batch124_rule_present_and_parses` (present + syn-parse + concepts + pinned phrases) 1/1 (one needle fix: "matched" appears only uppercase in the name — the test asserts case-sensitively on "control").
- Count pins walked: total 110 → **111**, annotated 53 → **54**, io 14 → **15**, header ledger (87/111, 24 NotBenchable, ledger line + the enumeration +1).
- Lib default **1166/0**; clippy 0 (default + rust_perf); retrieval floors byte-identical (kernel 100/100/100, blended 94.7/96.9/97.7); score-bench **FLAT 96.2/1.9**.

## Session incident recorded

The box's Data volume hit 100% (116 MiB free) mid-batch — root cause `/tmp/plan047_*` sibling build dirs (~9 GB, NOT mine — left untouched) + my 46 GiB target. `cargo clean` on MY target freed 66 GiB (52 GiB free). Lesson: a full-disk write can silently truncate a source file (the heredoc that appended the rule died mid-write with a real-newline-inside-string corruption + a dropped closing paren) — the parse gate caught it immediately; after any ENOSPC, re-read the whole touched region before trusting it.
