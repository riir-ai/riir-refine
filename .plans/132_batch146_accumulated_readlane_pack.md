# Plan 132 — Batch 146: the accumulated read-lane pack (nnethercote July-2026 + bevy 0.19)

**Status:** COMPLETE — Batch 146 landed 2026-09-16 (all GOAT gates green; see [Research 173](../.research/173_batch146_accumulated_readlane_pack.md) for the measured record + admission-time corrections). Opened on the accumulation rule (two staged
charter inputs landed the same day: the nnethercote read-lane's "stage until a
second input arrives" condition was MET by the bevy 0.19 read-lane, the
Batch-145 precedent). Executes ahead of the Mon 09-21 projection because the
box is quiet, no sibling is in this repo's corpus, and the snapshot's own
accumulation logic fires — windows are quiet-box events, not calendar slots
(Batch 145 landed the same way).

Charter inputs (verbatim blocks in `.distill/001_mining_queue_snapshot.md`):
- STAGED CHARTER INPUT 2026-09-16 (nnethercote "How to speed up the Rust
  compiler in July 2026", 2026-07-31) — 2 rust_perf + 1 dist suggest-only.
- STAGED CHARTER INPUT 2026-09-16 #2 (bevy 0.19 release notes, 2026-06-19) —
  2 rust_perf suggest-only.

Sources RE-VERIFIED by the coordinator against the live posts (not the charter
text): nnethercote.github.io/2026/07/31/how-to-speed-up-the-rust-compiler-in-july-2026.html
(#17124/#17132/#157762 visitor-dispatch quotes + 10-30%/20-80%/97% numbers;
#158720 72→64B / >10% / −29%; #159091 PGO +2.85%/6%) and
bevy.org/news/bevy-0-19/ (contiguous_iter table 5.58/4.88/1.87 + 4.40/1.58 µs;
density gates; encase→direct-copy light-clustering swap; Bistro bindless
+15%/−57 MB). All load-bearing claims verified verbatim.

## Tasks

- [x] T0 — Dedup re-verification (coordinator grep, vocabulary translation,
  both rust_perf + kernel_opt corpora): all 5 candidates NOVEL. Distinct-from
  neighbors identified: `dyn-to-enum` + entries_01 two-passes-fused (rule 1);
  `#[repr(align)]` entries_02 coherence rule + `hot-byval-move-over-memcpy-threshold`
  (rule 2); `AoS→SoA` entries_03 (rule 3); the weights-repack family entries_05
  (rule 4); zero PGO hits anywhere (dist rule).
- [x] T1 — Author 4 rust_perf entries (entries_06.rs, suggestion-only,
  NotBenchable, context_notes with verbatim source quotes + distinct-from rows
  + regime/anti-regime gates):
  1. `dispatch-pass-fusion-skips-empty-handlers` (complexity)
  2. `hot-alloc-struct-fits-cache-line-class` (layout)
  3. `dense-table-bulk-update-via-contiguous-iter` (bevy — carries the
     riir-ai Bench 930/Issue 960 negative as the anti-regime witness)
  4. `gpu-upload-repack-skipped-when-layout-matches` (layout)
- [x] T2 — Author the dist rule 6: `pgo-training-set-covers-real-workload-hint`
  (BuildTime family; ADMISSION-TIME DESIGN: fires ONLY on ships_bin ∧ all
  three effective knobs set — the fully-tuned audience; strictly additive,
  zero churn in existing gate fixtures; -hint suffix per the large-api
  precedent; #159091 citation conditional wording).
- [x] T3 — Standing gates: batch146 presence/annotation/pseudocode-parse tests
  (rust_perf tests.rs); count pins 127→131, without_spec 36→40, unannotated
  stays 57; dist pins entries.len() 5→6 + the new rule's fire/silent fixtures.
- [x] T4 — GOAT: G1 parse/syn (batch146 gates); G2 lib tests both feature
  states (default + latent_retrieval set incl. retrieval floors HELD); G3
  clippy 0 on the write-set (3 states); score-bench run (expect FLAT heal
  rate; row + frontier snapshot append).
- [x] T5 — Research note 173 + AGENTS.md corpus counts (rust_perf 127→131,
  dist 5→6) + `.docs/11_domains/{rust_perf,dist}.md` rows + queue-snapshot
  update (batch landed, charter inputs consumed, agave pool next).
- [x] T6 — Commit `feat: batch 146: ...` + push.

## GOAT gate

- G1: all new entries parse (syn gate) + present-and-annotated.
- G2: `cargo test --lib` green at default features AND at
  `--features latent_retrieval` (retrieval floors unchanged — suggestion-only
  additions must not decay retrieval).
- G3: clippy 0 warnings on the write-set in all 3 feature states; count pins
  exact; score-bench heal-rate FLAT (new rules add no mechanical fixtures).
- G4: bounded cost — zero new deps, zero new stores, corpus data only.

## Cautions

1. All 5 rules are SUGGESTION-ONLY — no bench spec, no bounded fix, no
   proposer wiring. They document restructures a parse-only proposer cannot
   see (visitor emptiness, allocation-site heat, engine layout, WGSL layout,
   PGO training sets).
2. The bevy rule carries BOTH sides (bevy's positive table + our Issue 960
   negative) — never cite the positive without the anti-regime witness.
3. The dist hint is PROXY-triggered by design (fully-tuned profile ≠ PGO
   exists); the -hint suffix + conditional advice wording + measure-first
   caveat are load-bearing, per the large-api-dependency precedent.
4. Score-bench: gen-8 admission is shape non-vacuity — this batch adds NO
   fixtures, so no generation bump; expect row append + snapshot unchanged
   (same frontier).
