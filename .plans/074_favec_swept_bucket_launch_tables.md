# Plan 074 — Batch 104: the fa-vec swept shape-bucket launch-table distill

**Status:** IN PROGRESS
**Source:** ggml-org/llama.cpp — infra `efeda76b9` (08-24, fa-vec machinery), table pin `8963a9bdc`
(08-28, #27863: M3 Max + M5 + M5 Pro rows), harness read at `d230ddd76` (b10793; the tools/tuning
tree is byte-identical from the pin to HEAD — verified by `git diff 8963a9bdc HEAD -- tools/tuning/`
= empty; the only post-pin delta in scope is more table ROWS, `9cc33944f` M3 / `8887a48f0` M2 Pro /
`be789c344` A18 Pro / `d08c7872d` M2 Max / `458681e1d` M1 Ultra). MIT license. Clone `.raw/llama-upstream`
(`--filter=blob:none`), `rm -rf` at landing.
**Queue provenance:** snapshot line "Distill lead (kernel_opt, mechanism class): `fa-vec` tuning-table
dispatch" (2026-09-02 late, league opponent watch #5 — upstream `8963a9bdc` adds the M3 Max rows).
**Verdict:** YES B+ (`.research/126`) — route kernel_opt, category **K (AutotuneDispatch)**.

## Tasks

- [x] T0 dedup re-verification (both vocabularies): zero corpus hits for swept-table fallback
      ladder / worst-case-regret pick / no-harm bucket representative / padding-witness sampling /
      exception-rows-over-collapsed-defaults; `anchor-shape-bucketing` (L) = cache-KEY bucketing,
      a lead not coverage — compose-cite, don't duplicate; K-family runtime-autotune rules
      (panic-safe / multi-priority / vector-width / shape-kind) = RUNTIME mechanics, not offline
      swept tables; B77 twins = compile-time tile instances; B103 shape-class = capability verdicts.
- [x] T1 verdict note `.research/126` (source URL, pins, provenance chain, ranked table, dedup
      evidence, caveats, queue slot; `**Status:** DISTILLED — pending owner decision` per the
      research-note shape; flipped to RECORD at batch landing).
- [x] T2 rule 1 `swept-shape-bucket-launch-tables` (K): the offline-swept table contract — bucket
      key (device × dtype × head dims × axis buckets), fallback ladder (exact bucket → collapsed
      domain default → family representative → template baseline), short-axis skip, consumer-side
      device-limit validation, shared legality constraint, exception-row sparsity.
- [x] T3 rule 2 `no-harm-bucket-representative` (K): worst-case-regret pick, pointwise-regret
      exception trigger, the padding-witness sampling rule, untrusted-cell drop, anchor+cooldown
      drift gating, empty-bucket→baseline.
- [x] T4 fixtures ORIGINAL (no verbatim source; license hygiene): rule 1 ref = one-axis hand-tuned
      launch gate used for every device/dtype + a consumer trusting a tuned constant without a
      device-limit check; opt = 8-byte packed key memcmp table + ladder + limit fallback;
      rule 2 ref = best-average pick over aligned points shipped as bucket representative;
      opt = no-harm filter + worst-padding-residue witness + worst-case-regret pick.
- [x] T5 gates: `batch104_rules_present` + `batch104_concepts_covered` + `batch104_pseudocode_parses`
      (`cfg(not(feature = "kernel_verify"))` per the B70 convention), following the batch103 shapes.
- [x] T6 count pins walked: mod.rs header 463→465; corpus-count test comment + expected;
      `kernel_categories_counts_match_plan_003_table` K 5→7 + total 463→465 + the "Sum must equal"
      assert; KERNEL_CATEGORIES K member list + the "(5 rules)" header comment; tests.rs
      `kernel_categories_cover_all_corpus_rules_exhaustively` needs no edit (derived); AGENTS.md
      kernel_opt line + rust_perf/clippy/sec/docker lines unchanged; snapshot corpus counts line.
- [x] T7 GOAT standing: kernel_heal lib + default lib + retrieval floors + clippy both states
      (the B95 honest-skip shape for the full CI guard if the box is loaded — each affected state
      validated individually + recorded).
- [x] T8 research note status → RECORD; AGENTS.md Batch 104 entry; snapshot queue update
      (Last landed + the fa-vec lead consumed); `.raw/llama-upstream` REMOVED.
- [x] T9 commit (one commit: corpus + gates + note + plan status + AGENTS + snapshot) + push develop.

## Non-goals

- No throughput claim against our tiled flash (the rule is mechanism-grade; our m16/m32 gate is
  the in-stack contrast, not a measured regression).
- No league action (the opponent pin stays `e311ed38f`; upstream rows beyond the pin are recorded
  as lineage only).
- rust_perf untouched; no new domain.
