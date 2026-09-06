# Plan 097 — Batch 118: antirez/ds4 PR #991 (Qwen3.8-Flash-Next) kernel_opt + rust_perf distill

**Status:** QUEUED — distill verdict YES B+ corpus / LEAGUE INTEL HIGH (Research 142); queued behind the arxiv-walk-#8 time gate (Mon 2026-09-07 ~07:00). No corpus edit until T0.

## Source + provenance

- **PR:** https://github.com/antirez/ds4/pull/991 ("Qwen3.8 flash next", ivanfioravanti → antirez, OPEN — 6 commits)
- **Pin:** ivanfioravanti/ds4-metal, branch `qwen3.8-flash-next` @ `236cb2a549d05ea1941a19bd4f154e0abc858661` (MIT). Upstream `antirez/ds4` main unchanged at `9ab7053` (= the B117 pin), so the PR branch is the entire delta vs Batch 117.
- **⚠ PR is OPEN and was force-pushed once (`ecf1da3` → `236cb2a`)** — every quote must be re-verified at mine time against a FRESH clone at the pin; if the head has moved, re-adjudicate the candidates against the new head before proceeding (the pin discipline, B96/B116 precedent).
- Verdict note: `.research/142_ds4_qwen38_flash_next_pr991_verdict.md` (Status flips DISTILLED → RECORD at batch close).
- Key surfaces: `metal/qwen4.metal` (2,648 ln), `ds4_metal.m` (+1,233), `QWEN38_PERF_HANDOFF.md` (2,103 ln, 21 measured sessions), `gqa_mma_proto.{m,metal}`, `qsa_prefill_bench.c`, `hc_chain_proto_*`.

## Scope

5 corpus rules (4 kernel_opt, 1 rust_perf/io) + 8–12 riders; lead with the role-scoped-store correctness rule + its falsifiable numeric gate. Expected yield after ~50% coordinator falsification: 3–4 rules. Model family: Qwen3.8-Flash-Next (48 layers, hidden 2560, vocab 248320, 36 GDN + 12 GQA w/ QSA indexer, 4-stream hyper-connections, 512-expert MoE, built-in MTP, external PLE n-gram sidecar).

## Tasks

### T0 — setup + re-verification (mandatory before any corpus edit)

- [ ] Re-clone fresh: `git clone --depth 1 -b qwen3.8-flash-next https://github.com/ivanfioravanti/ds4-metal.git .raw/ds4-metal` (or full clone if diff archaeology needed); verify `git rev-parse HEAD` == `236cb2a5…` and LICENSE == MIT. If head moved, STOP and re-adjudicate candidates against the new head first.
- [ ] Quote re-verification against the fresh clone, handoff-item by handoff-item: (a) 26c role-scoped fragment store + the exactly-uniform-attention symptom + "7 sessions" provenance; (b) 22 tile amplification arithmetic (512-row tiles = 16× weight re-read + 3.2× padding; 125→42 ms/layer) + the 22e staging-pass negative; (c) 25/26f precision split (scorer MMA +0.000008 vs attention MMA MAE 0.0581; Q-split P-tile attribution); (d) 8+11 scheduler break-even flip (+3.8% plain decode) + the 110% cumulative-ratio fix; (e) 23c cold PLE gather 6.9 s → 338 ms @ 32 threads, byte-identical; (f) 24c running-threshold prune + in-place-compaction aliasing; (g) 21b pipeline phase-role split (2 score + 4 PV simdgroups, 1.5× stage).
- [ ] Re-verify upstream main is still `9ab7053` (else restate the delta scope vs B117).

### T1 — dedup re-verification (both vocabularies, per candidate)

- [ ] #1 `role-scoped-store-must-match-compute-guard`: grep store/gate vocabulary (`fragment store`, `role-scoped`, `rank-scoped`, `store predicate`, B57/B108 barrier vocab) — expect zero kernel_opt hits (the rule is about the STORE's role contract, not barrier placement).
- [ ] #2 `tile-amplification-precedes-staging-pass`: grep tile/staging vocab (`tile amplification`, `pad ratio`, `B98 share-activation`, B77 vendor-batched-gemm contiguity, B92 cpasync multistage) — expect the tile-vs-tile-family boundary to hold (amplification-first is a SIZING predicate, the others are reuse/pipeline shapes).
- [ ] #3 `operand-precision-gated-by-consumer-invariance`: grep precision/argmax vocab (`argmax` per-rule precision, B110 router-math-stays-fp32, B92 cpa eligibility, B107 ordinal-bitmap bf16 key) — expect the consumer-invariance axis to be new; if the read lands as a B110 instance, demote to rider (adjudicate at mine time).
- [ ] #4 `speculation-scheduler-break-even-flip`: grep scheduler/break-even vocab (B94 fill-gate, B103 wavefront-pair-ceiling, B105 pipelined ping-pong) — expect the break-even-margin rule new (a DECISION rule, not a dispatch shape).
- [ ] #5 `parallel-disjoint-pread-hides-cold-page-latency` (rust_perf/io): grep pread/read vocab (B114 decayed-frequency read lanes, B106 mincore/reuse-key, rust_perf coalesce-nearby-reads B94 io) — expect the disjoint-parallel-pread axis new; if it collapses into coalesce-nearby-reads' complement, keep as io rule with explicit composition note.

### T2 — entry authoring

- [ ] Author kernel_opt entries #1–#4 (+ #6/#7 if adjudicated as rules) into `entries_gpu_29.rs` (next slice file after `entries_gpu_28.rs`), via the Python heredoc generator discipline (ensure_ascii=False, needle self-verify, `//` comments in fn bodies — the B37/B85/B111 lessons).
- [ ] Author rust_perf io entry #5 into the rust_perf corpus (io category, currently 4 rules per B85) — fixtures ORIGINAL (never paste source verbatim; license hygiene).
- [ ] Decision-content check per entry: bounded fix-space + regime conditions (when it applies AND when it does not) stated in-rule; drop-table rows for any candidate killed at T1.
- [ ] Adjudicate at mine time: #3 rule-vs-B110 rider; #6 running-threshold-prune-tile-merge rule-vs-B94/B103 rider (+ the in-place-compaction aliasing hazard as gate content); #7 pipeline-phase-role-split rule-vs-B79/B102 rider (third role-split axis — confirm it is a new AXIS, not an instance).

### T3 — the falsifiable numeric gate for rule #1 (the load-bearing gate)

- [ ] `batch118_unguarded_store_yields_uniform_softmax` (or exact-name TBD at authoring): fixture computes attention over a fragment store where an UNGUARDED writer contributes rows from the WRONG role scope → softmax collapses to exactly-uniform distribution (detector: max-min logit spread < epsilon over N rows); the GUARDED twin stays spread-widened. Production-magnitude fixture (handoff 26c: the bug passed every small-magnitude test — the fixture MUST use real-magnitude fragment values, not toy ±1.0), + negative control (guarded store does NOT trip), + determinism ×2.
- [ ] Standing gates for #2–#5 per house pattern: batch118_rules_present / batch118_concepts_covered / batch118_pseudocode_parses (`cfg(not(kernel_verify))` per B70) + any entry-specific contract gate.

### T4 — count pins + category tables

- [ ] Walk the full count-pin inventory (kernel_opt: tests.rs corpus total + expected-array + sums ×2 + grouped + cover-all comment + router.rs expert total + mod.rs doc line + mod/extend wiring + category comments + member-list tails; rust_perf: corpus.rs header + entries pin + annotated/unannotated + serde + drafter-propagates). Expect kernel_opt 544 → 548-549, rust_perf 101 → 102 (adjust to actual landing).
- [ ] Regenerate FULL-STRING pins for any ridden rule whose concept text grows (the B23/B22 lesson).

### T5 — riders (adjudicate each at mine time; drop the ones that don't survive quote re-verification)

- [ ] B24 warp-shuffle sub-simdgroup twin (handoff 21b lane organization).
- [ ] B103 shape-class-keyed-capability fixture-engagement pin (the scheduler's armed-vs-served widths).
- [ ] B110 both-sides consumer-invariance (scorer-vs-attention MMA precision split as EXTERNAL instance; may fold into rule #3's adjudication).
- [ ] B95 whole-sequence-zero-barrier GDN R4 instance.
- [ ] B57/B108 barrier confirmation (hc_chain_proto stream handoff).
- [ ] B98 NSG ladder (score+PV simdgroup partition).
- [ ] B77 ApplyWeight/derived-sum member.
- [ ] B106 stale-binary provenance + profiler-decode-perturbation note.
- [ ] B92 thread_elements instability + B29 launch-fusion-neutral + B110 persistent-fusion negative (record-only riders where no corpus edit needed).

### T6 — GOAT + validation

- [ ] All new gates green (batch118_* 3–4 gates + rule-#1 numeric gate), deterministic ×2.
- [ ] kernel_heal lib + default lib + kernel_heal+latent combined: floors HELD zero re-pins; retrieval_precision_regression_gate PASS.
- [ ] clippy 0 in default / kernel_heal / kernel_heal+latent states + `--all-targets` clean in touched files.
- [ ] Count pins green everywhere.

### T7 — records + closeout

- [ ] `.research/142_…` Status → RECORD (batch landed, one-line pointer).
- [ ] AGENTS.md Batch-118 entry (the canonical batch ledger row).
- [ ] `.distill/001_mining_queue_snapshot.md`: mark the queued line LANDED with the landing summary; keep under ~400 lines (fold to history if needed).
- [ ] `.raw/ds4-metal` rm -rf (cleanup is part of the batch).
- [ ] Commit + push on `develop` (`feat:` prefix for the corpus batch commit; explicit paths only — sibling WIP in tree).

## Non-goals

- No league/lane doc edits from this repo (the league Flash-Next watch-row update + riir-ai Issue 769 addendum are NAMED as sibling acts in the queue line + verdict; sibling-repo files are not edited here).
- No ANE-offload, persistent-fusion, PLE-paging, HC/QSA/vision-kernel, Q2-imatrix mining (drop ledger in Research 142).
- No claim on the PR's absolute tok/s numbers (M3-Ultra-only, different silicon/model — league cells own same-box comparisons only).
