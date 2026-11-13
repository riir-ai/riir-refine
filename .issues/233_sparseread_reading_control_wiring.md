# Issue 233 — SparseRead reading-control wiring for the rulewright read lane

**Status:** open — unclaimed

- **Date:** 2026-11-13 (m3)
- **Research:** [`.research/404_SparseRead_Reading_Control_Layer.md`](../.research/404_SparseRead_Reading_Control_Layer.md) — arXiv:2608.22237 (SparseRead, MIT, `Liuzedooong/SparseReading @ d30f75f8`)
- **Class:** GOAT-tier Gain. Training-free serving/infrastructure — modelless end to end; no riir-train deferral.

## Premise

cargo refine's admission gates are single-axis and stateless: kernel_pregate admits a domain per file (needles + span scope), span_shape_dispatch narrows rules per span (hand-registered shape rows, fail-open), QUERY_TEXT_CAP bounds one query (lossy, disclosed). What none of them carries — and what SparseRead measured as the biggest lever (protocol repair alone: 2.1–6.3× token reduction; repeated reads −68.8%) — is **state across a campaign**: what was already read, under which intent, at what cumulative exposure cost, with a faithful fallback when evidence doesn't fit intact. Honest scope note: the read lane is second-order cost (the auto-oracle is ~97% cargo clippy wall time); the expected wins are scan-lane wall time and repeated-query elimination, not the compile loop. Do not touch the compile-gate loop.

## Tasks

- [ ] **T1 — read-episode coverage record (in-memory per run).** **T1.0 GATES THIS TASK — count repeated span queries on the oracle-fed battery BEFORE building anything**; if the count comes back near zero the premise is unmeasured and T1 is DEFERRED, with the count recorded here (verdict round 1 note). Then, if the count justifies it: a per-campaign map keyed `(artifact path, blake3(content))` → spans/lints already queried or ranked; span-query sites consult it before re-querying a known span; content change (blake3 drift) invalidates the record (the SparseRead fingerprint law, blake3 not SHA-256). No persistence — a `--fix` run is one episode; the index cache (issue 226) already owns cross-run keys.
  - Fixture: the oracle-anchor lane (`--clippy-json` feed) over labeled spans; the record's hit/miss counts printed on the run tail.
  - Falsification: env-off (`RIIR_REFINE_READ_EPISODES=0`, exact-literal demote) byte-identical to today.
  - Metric: repeated span queries per fix campaign, before/after, on the oracle-fed battery.
- [ ] **T2 — cumulative per-file-session query budget with faithful fallback.** **Scope law (verdict round 1): the ≤2L bound here is a BYTES bound** — in refine there is no model context; exposure is query text sent to a retrieval index, and retrieval cost need not grow with query length, so the bound makes NO wall-time promise. The p50/p95 bars own the time axis separately. Budget `L` = the serialized length of the uncapped query for that file (computable without a network); cumulative `E` = Σ capped query bytes this file-session; when a next query would push `E > L`, **withhold the capped form and issue the uncapped query once** (byte-identical to the pre-pregate posture — the faithful full view), then close the episode. Non-lossy by construction: unlike QUERY_TEXT_CAP (accepted rank-1 loss, Bench 137), nothing is truncated mid-anchor; the fallback replays the exact incumbent query. Ride the pregate A/B methodology (Benches 131–137 pattern).
  - Pre-registered bars: added p50 ≤ 1.0 ms / p95 ≤ 5.0 ms per file (quiet-box, bench_preflight PROVENANCE quoted), utility floor 15/15 held in every arm, fallback posture byte-identical to uncapped.
  - Exposure metric (new): cumulative query bytes per file-session, reported beside the `⏱` phase split.
  - Demote: `RIIR_REFINE_QUERY_BUDGET=0` (exact literal), DEMOTE-ONLY if promoted.
- [ ] **T3 (stretch, opt-in) — calibrated acquisition-config selection.** Replace span_shape_dispatch's hand-registered reliability with a measured floor: per (domain × span-shape) group, Wilson-LCB (95%, lower end) of oracle-labeled evidence success over the existing `retrieval_eval.rs` + `clippy_oracle.rs` fixtures; floor τ = 0.6 first (the paper's fixed pre-evaluation value); a config under the floor falls back to the FULL fan-out (never silently relaxes — the paper's law). Groups with <20 labeled spans use the default (no dispatch), per the paper's small-group rule.
  - GOAT gate: recall@k of the narrowed posture ≥ full fan-out at the floor, plus a measured cost cut on the G2 leg; widening a group = the Issue 216 registration discipline (re-verify the claim + re-run the G2 leg).
  - Feature: `calibrated_dispatch` (opt-in; implies `span_shape_dispatch`); never in the release set until the gate passes.

## Non-goals

- NOT adopting the paper's MCP/harness integration surface — that's their product lane; fleet-side adoption of SparseRead itself is an owner ops call, not this issue.
- NOT touching the compile-gate loop (`fix_verify`), the escalation chain, or ESC enforcement (rethink-private; the per-episode escalation-budget fusion option is recorded in Research 404 only).
- NOT re-opening QUERY_TEXT_CAP's lossy posture — T2 is a NEW cumulative axis beside it, complementary, with the faithful fallback the cap never had.

**Lease closed:** zed-4430bf70@4090, 1794579705, was expires 2026-11-13T16:21:45Z (expired unworked — the routing hand-off) — housekeeping routing (2026-11-13): routed to the amo lane (the active refine-batch rig; 4090 cannot host refine builds right now — 2/32 GB RAM free, the sibling t679 job holds the box, and T2 own bars demand a quiet-box preflight). Order of work per the issue own discipline: T1.0 FIRST (the repeated-span-query count on the oracle-fed battery — a near-zero count DEFERS T1 with the count recorded), then T1 (in-memory read-episode record, env-off byte-identity), T2 (the bytes-bound budget with the faithful uncapped fallback), T3 stretch behind its feature gate. Claim expected from the taking agent on pickup.
