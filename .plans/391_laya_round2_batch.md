# Plan 391 — Batch 375: laya round-2 distill (the staged unread-sweep candidates)

**Status:** IN FLIGHT 2026-11-20 (zed-m3-b375@m3, epoch 1795176927 — executing the queue snapshot's staged pool from the Batch-374 landing: 7 tier-1 candidates + ~9 B-tier + the face ledger, ALL re-derived at T0 per the snapshot's re-derivation note — the prior session's subagent transcripts died with its thread)
**Claimed:** zed-m3-b375@m3, 1795176927, expires 2026-11-22T12:15:27Z

Source: `github.com/NandhaKishorM/laya` — landing pin `416dd9ed20da2d43fc40e23712e9adceefabf634` (T0 tip check at clone 2026-11-20T12:15Z: **STILL THE REMOTE TIP** — no re-pin). Apache-2.0. Clone ephemeral at `.raw/laya`, removed at batch close.

Staged pool (Batch 374's T0 unread sweep, ~11.1k lines over 16 files):

1. `gate-cut-fitted-on-serving-representation` A− (calibrate.py rounding/binning + serve.py repr-vs-json; measured 9.8%→25.6% divergence; three instances across two languages)
2. `reference-round-replicated-in-integer-arithmetic` B+/A− (calibration.rs round4 L154-212, the sticky-bit correctly-rounded conversion; **benchability graded at T0**)
3. `deepcopy-snapshot-before-untrusted-mutation` B+ (onnx_agent L546-553 + the single-window budget-check face L566-572; the shallow-copy-also-fails subtlety)
4. `per-item-bound-not-batch-max` B+/A− (common.py collate_items L1183-1191, their #311 error message cited)
5. `artifact-load-replaces-whole-state` B+ (vega L1027-1032 + calibrate.py L783-786)
6. `stride-pair-leniency-vs-refusal` B+/A− (common.py window_budget L440-458, the self-consistent-pair distinction)
7. `lower-bound-precheck-understates-never-fabricates` B+ (serve.py L959-1020)

Pre-registered bars + falsifiers BEFORE the first corpus edit (the ungoat law):

- Size discipline 4–6 rules + faces; the pool is graded at T0 — dedup/adjudication may drop rows, drops are recorded.
- If any candidate grades **BENCHABLE** at T0 (expected only #2), its G1–G4 bars are committed in a plan addendum BEFORE the first corpus edit; the work model is REAL work, never a cost model (a simulated cost is the falsifier that voids the cell); a Mixed verdict lands honestly; a failed bar demotes in the landing commit.
- Suggestion-only rows: decision-content first-class, `NotBenchable-by-domain` + `bench_spec: None`; the four-arm fixture discipline (buggy/fixed/all-safe/tamper) standalone-compiled AND executed BEFORE the first corpus edit.

- [ ] T0 — pin + tip check (DONE at clone: pin == remote tip); re-read every staged range in the clone, re-derive the quotes byte-exact (two reader subagents: the Python half + the serve/Rust half); grade the ~9 B-tier rows + the face ledger from the staged note; record drops + benchability verdicts per candidate.
- [ ] T1 — dedup greps at landing HEAD, 3+ vocabularies per candidate; the named adjudications carried IN-RULE: #2 vs the exact-arithmetic family (magic-anchor-mma-exact + two-float compensated accumulation + anchor algebra — kin cites, never re-owned); #1 vs `confidence-threshold-bucketed-by-answer-shape` (B374 R2) + the calibrator family; #3 vs `clone-before-mutating-loop-borrow-appeasement` (B356) + `capacity-gated-exact-undo-loud-degrade`; #4 vs `untrusted-keyed-state-unbounded` + `length-prefix-validate-before-alloc` + `request-amplification-guard-before-expensive-path` (B374 R8); #5 vs `frozen-artifact-plus-delta-tail-overlay` (B365) + the epoch-tag hot-swap family; #6 vs the tri-state-gate face family (B374 R2's in-notes face) + paired-closer gate law; #7 vs `degenerate-output-invalidates-throughput-claim` + `work-removed-build-is-not-a-valid-lower-bound`.
- [ ] T2 — authoring: rust_perf `entries_88` (re-derived by `ls` at landing) + sec corpus rows if routed; the dedup KILLs recorded in the entries' notes.
- [ ] T3 — ORIGINAL fixtures (`src/domains/rust_perf/corpus/fixtures/b375_fixtures.rs` + sec fixtures if routed), four-arm discipline, standalone-compiled AND executed BEFORE the first corpus edit.
- [ ] T4 — GOAT cell only for a T0-graded BENCHABLE survivor, at its committed bars, two-run stable, box-state PROVENANCE quoted; `.benchmarks/NNN_batch375_goat_cells.md` per numbering discipline.
- [ ] T5 — gates at isolated `CARGO_TARGET_DIR=/tmp/b375_target`: batch375 needles + parse, rust_perf lib, default lib, sec lib (if routed), clippy `-D warnings` × postures, retrieval floors HELD (annotate-only rows expected), fmt on touched files, pins_manifest `--write` + bare AGREE, count web one-landing (AGENTS + README + `.docs/11_domains/rust_perf.md` + health_views + the kernel-side witness + annotated pins) re-derived AT LANDING HEAD.
- [ ] T6 — score-bench NOT owed if zero clippy_lints surface (assert + disclose); freeze NOT owed (not clippy-domain); release-set binary rebuild stays the standing residue (corpus-only delta disclosure).
- [ ] T7 — Research 425 status → `DISTILLED — batch-375-landed` (or the honest NEGATIVE row per ungoat); snapshot LANDING row; `dual_allocation` + `numbering_gate` clean at push.
- [ ] T8 — ONE `feat: batch 375 …` commit + push develop; `rm -rf .raw/laya` + `/tmp/b375_target` teardown at close (the cleanup law, df in the summary).

Yield rules: a pushed claim row naming plan 391 / Batch 375 / the laya round-2 lane from another session supersedes on fetch; expiry 48 h from claim; push is the arbiter. NOTHING INVENTED — every quote at the landing pin.
