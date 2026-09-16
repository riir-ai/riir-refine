# Plan 135 — Batch 149: the sm_89 spill-law / nodecode-probe STANDALONE pack (the Batch-148 dual-allocation yield's re-issue)

**Status:** COMPLETE 2026-09-16 — LANDED at `cdcbea49` (batch 149; the yield's ref `a970a3e8` preserved in Research 175's record; global binary rebuilt same-session, kernel_opt 619 verified through the symlink)

## Context — the dual-allocation

The session authored a full Batch 148 (5 rules + 7 riders, Plan 133 / Research
174 / entries_gpu_43.rs, local commit `a970a3e8`, all gates green) from the
same staged intake lead (the Bakhshaliyev 4090 FP8 series) that a sibling
session landed as Batch 148 on origin/develop (`8f8b7dd6`) DURING the session.
Both allocated Plan 133 / Research 174 / batch 148 / entries_gpu_43.rs.
Per the Issue-665 discipline the local batch was YIELDED (branch
`b148-yielded`): the sibling landing is canonical. The yield preserved the
ref: `b148-yielded` = `a970a3e8`.

## The delta analysis (what their landing does NOT cover)

Their 2 rules embed the B933/B895 evidence as ROUTE-SPLIT regime content of
`occupancy-before-async-consumer-ada`; their 5 riders cover xor-swizzle /
ldmatrix / stream-k / cpasync / mma-fragment (incl. FP8 642.51/329.33 +
QMMA a1/a2 + TMA-absent probe facts). Non-duplicate delta:

- [x] `spill-fragility-law-accumulator-regcap-cliff` (E) — the ×4 law as a
      standalone rule (their rule CITES the law; this ships it — serving
      B895's original context: pricing a FUSION's register need, occupancy
      irrelevant)
- [x] `nodecode-arm-bounds-overhead-elimination` (D) — the probe METHODOLOGY
      (their rule CONSUMES the 64.6%/7.3% numbers; this is the instrument)
- [x] `tile-shrink-occupancy-pays-feed-economics` (E) — the feed-scaling law
      (1/tile-area) + the both-directions local optimum + reopen triggers
      (their rule carries the v12 cell as tile-route evidence; this prices
      tile decisions that are not occupancy buys)
- [x] 1 rider: setmaxnreg-fit-gate-elected-arrive (the sm_89 no-setmaxnreg
      counterpart — the boundary belongs ON the sm_90 rule)
- DROPPED as duplicates: occupancy rule, mbarrier rule, fast-accumulate /
  tma / cpasync / xor-swizzle / ldmatrix / stream-k riders (all landed by
  the sibling batch)

## Deliverables

- [x] `entries_gpu_44.rs` — 3 rules (corpus 616 → 619; E 55→57, D 134→135)
- [x] mod.rs wiring (decl + extend + KERNEL_CATEGORIES + count comments,
      fixing the sibling's stale E comment 54→57)
- [x] Count pins 616 → 619 (tests.rs ×5 incl. per-category table, router.rs,
      the stale mod.rs 614-rule doc → 619)
- [x] tests.rs: `batch149_{rules_present,concepts_covered,pseudocode_parses}`
- [x] The setmaxnreg rider (binary-safe append, LF preserved)
- [x] Research note `.research/175_*.md` (Status: RECORD) — the yield record
      + the complementary verification corrections
- [x] AGENTS.md kernel_opt count 616 → 619
- [x] `.docs/11_domains/kernel_opt.md` L13 + Batch 149 entry
- [x] `.distill/001` Batch 149 LANDED block + the yield note

## GOAT

- [x] G1: batch149_pseudocode_parses 3/3
- [x] G2: lib tests green — default + kernel_heal (+ kernel_heal,latent_retrieval retrieval floors)
- [x] G3: clippy 0 on write-set; score-bench FLAT
- [x] Count pins walked; commit + push (delete `b148-yielded` after)
