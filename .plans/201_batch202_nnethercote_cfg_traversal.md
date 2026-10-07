# Plan 201 — Batch 202: the nnethercote min-rank worklist rule (rust_perf, the freed slot)

**Status:** ACTIVE — the next rust_perf slot per the 10-06/10-07 intake rows (B201 discharged the slot flag). One rule, `NotBenchable` decision-content (the B182/186 corpus-only precedent): the dataflow fixpoint's convergence is traversal-order-dependent — process dirty nodes in dataflow order (min-rank), never FIFO push-back.

Source: nnethercote perf blog 2026-09-30 (`/2026/09/30/how-to-speed-up-the-rust-compiler-in-september-2026.html`) + rust-lang/rust `#160193` "Improve CFG traversal" (merged 2026-08-01T04:45:44Z, milestone 1.99.0, r=cjgillot, merge commit `cb9d1b0640549f1b041aae430dc413ce93f8c204`, head `4aadc68a8a49`). Quotes re-verified THIS pass (live reads: blog HTML + merged `compiler/rustc_mir_dataflow/src/framework/mod.rs` @ `cb9d1b06`).

⛔ **DRIFT CORRECTED vs the 10-02 staging row:** the "17% reduction in instruction counts" figure belongs to **#160033** (EverInitializedPlaces projection-data pruning, match-stress benchmark) — a DIFFERENT PR. `#160193`'s own figures: **1.5 million → 90,000 `apply_effects_in_block` calls** (blog, verbatim) and **~30% check-build wall-time on cranelift-codegen** (blog; the 10-02 row's "−33% best-case try-run" is consistent). The 1,508,264/89,809 precise pair from the 10-02 read was NOT re-found in the PR issue/comments this pass — NOT carried; the blog's rounded figures + the merged implementation comment carry the rule.

## Tasks

- [x] Dedup grep: fixpoint/worklist/traversal/convergence over rust_perf + kernel_opt corpora (DONE pre-plan this session — zero collision; nearest = entries_06's unknown-amount parallelism, different family; record in the entry's NOVEL clause).
- [x] `src/domains/rust_perf/corpus/entries_16.rs` — 1 rule `dataflow-fixpoint-convergence-traversal-order` (complexity): buggy = FIFO push-back worklist; fixed = min-rank over RPO ranks (the `curr_rank` invariant + `first_set_at_or_after` + back-edge rewind); context_notes with verbatim quotes (blog + framework comment), the 17%→#160033 drift disclosure, regime/anti-regime, kinship, cite.
- [x] `corpus/mod.rs`: register `entries_16`; repair the stale doc-comment headline (167 → 180, honest parenthetical naming the B198 +1 and B200 +11 skips) + complexity 31 → 32.
- [x] `corpus/tests.rs`: count assert 179 → 180 (Batch 202 label) + `batch202_rules_present_and_parse` (presence + category + syn-parse + notes needles: NotBenchable / cb9d1b06 / 160193 / 1.5 million / 90,000 / 160033-drift).
- [x] Gates: `cargo test --lib batch202` ✓ + the count assert ✓; `cargo clippy --all-targets` — zero findings from the batch (one doc_markdown healed en route). Freeze + score-bench NOT owed (zero clippy-corpus delta, P30). En-route: the first context_notes write was missing Some('s closing paren — rustc "unclosed delimiter" bisected to it; fixed before any landing.
- [x] Retrieval floors: `cargo test --features latent_retrieval --lib retrieval` ✓ 20/20 (32 s; no decay; no re-pin).
- [x] Research note `.research/238_nnethercote_cfg_traversal_min_rank.md` (RECORD: the verified quotes, the drift corrections, the not-re-carried 1,508,264/89,809 pair).
- [x] AGENTS.md rust_perf count line 179 → 180 (Batch 202 label, ×2 sites); `.docs/11_domains/rust_perf.md` 179 → 180 with the entry summary.
- [ ] `.distill/001_mining_queue_snapshot.md`: R134 slot DONE + commit hash; queue state updated (this commit).
- [ ] Fetch origin immediately before the commit (the S-series lesson); commit `feat: batch 202: ...` + push; tick this plan (this commit).
