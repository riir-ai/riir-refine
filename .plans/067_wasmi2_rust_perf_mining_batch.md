# Plan 067 — Batch 99: wasmi v2.0 blog distill → rust_perf (branch-site merge + GPR budget)

**Status:** QUEUED — verdict YES B (`.research/119_wasmi_v2_blog_distill.md`); batch not yet mined. The linked workspace action (wasmi 1.0 → 2.0 dependency upgrade) runs under riir-ai Plan 563 and is NOT this batch.

## Source

- wasmi v2.0 engineering blog — https://wasmi-labs.github.io/blog/posts/wasmi-v2.0/ (Robin Freyler, 2026-09-01), first-party + measured; the DestProp finding cross-verified on Stitch.
- The `Accidental Rust Deoptimization` section is the GOAT lead; the GPR calling-convention section is the secondary.

## Candidates (from the Research 119 ranked table)

1. `mir-destprop-branch-site-merge` (dispatch, A−) — merged conditional-dispatch call site → single branch site → predictor history mixing (rustc ≥1.92 DestProp default); fix = two distinct load+call sites. Suggest-only. Bench adjudicates: re-merge → Tie → annotation or kill.
2. `hot-dispatch-gpr-arg-budget` (dispatch, C+) — hot handler fns over the ~6-GPR budget spill per dispatch; pack rarely-used args into the float domain. Suggest-only. Kill freely if the bench is flat.

## Tasks

- [ ] T1 Dedup re-verification at mine time (grep `mir-destprop|branch.?site|destprop|gpr|calling.?convention` over all domain corpora — vocab translation incl. `predictor|dispatch|csel|register`), record one line of evidence per candidate.
- [ ] T2 Author candidate 1 corpus entry: original minimal buggy/fixed fixtures (merged call-site dispatch loop vs split sites), `context_notes` carrying (a) rustc ≥1.92 DestProp regime, (b) tail-call/indirect-dispatch-only win regime, (c) the re-merge caveat, source cited (wasmi v2.0 blog, Research 119).
- [ ] T3 Bench spec for candidate 1 (perf-bench Pareto validator + alloc axis): two arms, sizes axis on dispatch count; G1 output-identity gate; verdict-stability ×2. If Tie with re-merge evidence → land as honest-annotation (the varint-compose precedent) or kill with the measured reason.
- [ ] T4 Candidate 2 corpus entry + bench spec IF T3 lands ≥1 rule and time permits; otherwise record as queued-secondary with the kill-free note.
- [ ] T5 Standing gates: `batch99_rules_present` + `batch99_concepts_covered` + `batch99_pseudocode_parses` (cfg(not(kernel_verify)) per the B70 convention).
- [ ] T6 Count pins: mod.rs header, category headers + comments, tests.rs corpus-count/expected-array/sums, router.rs expert total, the `(10/88)`-style Not-benchable line if the count moves.
- [ ] T7 Retrieval floors (14/14) + score-bench run — must be FLAT or better; re-pin only if justified.
- [ ] T8 Clippy 0 both states `--all-targets`; doc-markdown backticks pre-empted (the B37 lesson).
- [ ] T9 Docs: AGENTS.md batch entry (rust_perf count bump + annotated counts), `.docs/11_domains/rust_perf.md` table rows, this plan's checkboxes, `.research/119` status → RECORD, queue snapshot line update (Batch 99 landed / counts).
- [ ] T10 Commit + push (`feat(batch99): ...`), same-unit queue update per the idle-loop discipline.

## Honest caveats carried into the batch

- Blog-only source: fixtures ORIGINAL (license hygiene + binary-only rule); the blog's aarch64 listings are evidence, never pasted.
- The DestProp deopt is toolchain-version-regime (rustc 1.91→1.92 boundary measured in the source); a toolchain bump re-gate applies (the debug-assert-length-hoist cross-arch precedent).
- Batch 99 numbering assumes no sibling batch lands first — re-scan the AGENTS.md ledger at WRITE time (the 18th-count-pin-site lesson).
