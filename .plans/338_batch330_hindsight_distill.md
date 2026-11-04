# Plan 338 — Batch 330: vectorize-io/hindsight distill conversion (sec + rust_perf)

**Status:** STAGED — waiting for a free batch slot (behind plans 336 / 337)

**Claimed:** zed-m3-hindsight-b330, 1793807728, expires 2026-11-06T15:55:28+00:00

Source: vectorize-io/hindsight @ `841979dfda4e1bebf7cbf68d0144d5d5454d93df`
(MIT, tip 2026-11-04) — verdict Research 368 (YES B−). Nominal Batch 330,
staged behind plan 336 (Batch 328, oss-fuzz) and plan 337 (Batch 329, voxl).
Claim plan-first at the next free slot; push is the arbiter.

Scope (from Research 368's ranked table, re-graded at T0):

- `ascii-token-unicode-word-boundary-scan-miss` (sec, B−) — founding member of
  the sensitive-data-at-rest family.
- `luhn-validate-numeric-pii-candidates-before-redact` (sec, B−) — with the
  overlap-rescan rider note.
- `fanout-first-error-leaves-siblings-running` (rust_perf concurrency, B,
  NotBenchable-by-domain annotated) — std-thread fixture, law names the async
  JoinSet spelling; error-envelope note as context, not a second rule.
- `redaction-audit-hit-carries-raw-match` (sec, C+) — drop honestly if the T0
  adjacency read kills it.
- `query-term-truncation-by-selectivity-not-input-order` (rust_perf io, C+) —
  drop honestly if it collapses to the generic pre-filter law.

Honest drops allowed; expected landing 2–3 rules (sec 38→40±1,
rust_perf 294→295±1 — re-derive live counts at claim time from the pins
manifest, never from prose).

Tasks:

- [ ] Claim the batch slot (lease-check item 338 clean; append the claim row; expiry 48 h).
- [ ] T0 re-clone `.raw/hindsight` at the pinned sha (`841979df…`); verify HEAD == pin, moved HEAD = re-pin + re-verify every quote BEFORE authoring; read the Research-368 unread list (reranking.py, recall_boost tail, consolidator body, retain/, reflect/, transfer/).
- [ ] Re-grade all 5 candidates at the pin (T0): confirm or drop with reasons; adjudicate C3/C5 honestly.
- [ ] Dedup re-verification at 3+ vocabularies over sec + rust_perf corpora (live counts from the pins manifest), one evidence line per surviving candidate.
- [ ] Author sec entries (sensitive-data-at-rest family header + rules) — original minimal Rust fixtures, standalone rustc-compiled AND executed before landing; suggest-only, NotBenchable-by-domain annotations.
- [ ] Author the rust_perf concurrency entry — std-thread cancellation-flag fixture; the law text carries the async JoinSet twin + the anyhow-downcast caveat as context.
- [ ] Detectors (sec proposers) for any FIXABLE sec rules; suggest-only otherwise; probe-parity posture for the sec lanes per the B258/B291 discipline.
- [ ] Count pins via ONE shape-asserted python pass (tests.rs sites + mod.rs docs + AGENTS.md + README + .docs/11_domains/*.md batch blocks).
- [ ] Gates per Protocol A: batch330 needle + parse gates, default lib, rust_perf_bench lib (if rust_perf moved), sec-domain probe-parity posture, clippy −D × all postures, retrieval floors re-measure with pre/post attribution (detached worktree at the pre-batch sha), score-bench appended; freeze-table NOT owed unless clippy_lints moves (P30); release-set binary rebuild + install BEFORE push if rust_perf/sec surface is release-set (the §0 bin-age law).
- [ ] Research 368 addendum (T0 verdicts + drift disclosures) in the same landing commit.
- [ ] Update .distill/001_mining_queue_snapshot.md (landing row discharges the claim), AGENTS.md counts, docs/AGENTS queue rows.
- [ ] Commit + push (docs:/feat: per convention); `rm -rf .raw/hindsight` at close.
