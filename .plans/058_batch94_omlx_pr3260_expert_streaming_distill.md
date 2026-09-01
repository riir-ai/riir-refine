# Plan 058 — Batch 94: oMLX PR #3260 SSD Expert Streaming + Soft-REAP distill (Protocol A)

**Status:** DONE — landed 2026-09-01 (8 kernel_opt D + 1 rust_perf io + 3 riders; corpus 414 → 422 / 96 → 97; GOAT all green; Research 112)

## Source

- [jundot/omlx PR #3260](https://github.com/jundot/omlx/pull/3260) — "feat(moe): SSD
  Expert Streaming and Soft-REAP for MoE models", ~3.3k LOC, module
  `omlx/expert_streaming/` (10 files) + `docs/soft_reap.md` + bench.
  Head `3343e44` at research time — **force-push-prone: re-clone
  `refs/pull/3260/head` at mine time and re-verify every anchor.**
- Research 110 §6 = the rule candidate table (R1–R9 + riders); §7 = bug yield;
  §5 = coverage map (what is already covered → riders/cross-refs only).

## Tasks

- [x] T1 Mine-time re-read: shallow-clone PR head under `target/ext/omlx3260`
      (NOT /tmp — the vanish hazard), record the resolved head SHA, re-verify
      all rule anchors byte-accurate vs the research table.
- [x] T2 Dedup grep BEFORE authoring: R1–R9 concepts vs existing
      `src/domains/kernel_opt/` + `src/domains/rust_perf/` corpora (Protocol A
      step 2; Research 110 §5 is the prior map — confirm at mine time).
- [x] T3 Two parallel subagent lanes (anchor re-verification, not fresh
      proposals — the rules were proposed at research time):
      lane A = kernel_opt D cluster R1–R8 anchors; lane B = rust_perf R9 +
      riders (B63 4th member, B71 producer complement, B84 cross-ref) + the 4
      corpus-grade bug-yield anchors.
- [x] T4 Corpus entries: kernel_opt D × R1–R8 (final routing per mine-time
      verification) + rust_perf io × R9; riders as doc/cross-ref edits on B63,
      B71, B84; category counts updated (D 85 → 93, io 4 → 5; corpus totals
      414 → 422 / 96 → 97 — the plan's draft estimate 423 was an arithmetic
      transposition, caught by the category-sum gate at GOAT time).
- [x] T5 Standing gates: `batch94_pseudocode_parses`
      (`#[cfg(not(feature = "kernel_verify"))]` per B70) +
      `batch94_rules_present` + count pins walked (tests.rs, router.rs,
      mod.rs comments + expected-array gates + the corpus header + the stale
      count comments Batch 95 flagged).
- [x] T6 GOAT: G1 syn-parse all new rules; G2 lib tests (kernel_heal + default
      + all-features); G3 clippy 0 both states + CI guard layers (or recorded
      honest skip); retrieval floors 14/14 (`kernel_heal,latent_retrieval`
      retrieval test — kernel self-variants 100/100/100, no re-pins);
      score-bench flat (`--fix` path untouched).
- [x] T7 Docs: Research note 112 (batch record; Research 110 status → LANDED),
      AGENTS.md Batch 94 entry + corpus count bump (414 → ~423 kernel_opt;
      rust_perf 96 → 97), `.docs/11_domains/` updates (kernel_opt +
      rust_perf), mining-queue snapshot update (same commit).
- [x] T8 Commit `feat: batch 94: oMLX PR #3260 expert streaming — N rules...`
      + push (ONE commit per Protocol A).

## Defers

(none yet)

## Notes

- Mine-time verification may REFINE routing (a candidate can be killed as dup
  or re-filed as rider — record every disposition in Research 112).
- Generator lessons: corpus category-count comments are plain `//` inside fn
  bodies (B85); string-heavy corpus entries via python heredocs with explicit
  escaping (B77); re-grep after multi-line comment edits.
