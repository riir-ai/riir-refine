# Issue 016 — Expand structural clippy lints (10 → top-N machine-applicable)

**Filed:** 2026-08-15, out of the clippy-vs-riir-clippy comparison session.
**Status:** OPEN.
**Type:** coverage expansion (distill axis of the flywheel).

## Background

The `clippy_lints` domain ships 10 seed lints; 6 are `--fix` eligible via the
T6.4 span-preserving emitter (`FIXABLE_LINTS`: needless_return,
redundant_field_names, manual_map, unnecessary_unwrap, needless_range_loop,
needless_lifetimes). cargo clippy ships ~700 lints. The emitter + proposer
pattern is proven end-to-end: the Bench 019 G5 run 2 landed 12 files / 20
edits clean on real repos.

The selection rule is established by the T6.2 lesson: **parse-only-provable**
lints go in `FIXABLE_LINTS`; **type-blind** lints (needless_borrow,
useless_conversion, unnecessary_mut_passed; redundant_closure demoted after
the E0631 catch) stay out of the default path — they belong to
`VERIFY_LINTS` under `--verify` (Issue 015, in flight).

This is the highest-leverage coverage gap: each lint is a small, independent
distill (corpus entry + syn AST proposer row + fixtures), and the pipeline
around them is already built.

## Proposal

Distill the frequent machine-applicable structural subset of clippy's lint
set into proposers:

1. **Selection**: rank by real-world trigger frequency (lint stats from
   large Rust repos) ∩ clippy's `MachineApplicable` applicability ∩
   parse-only provability (no type/borrow inference needed to prove the
   rewrite safe).
2. **Candidate pool** (final set filtered by criterion 1 — several of these
   may be type-blind and belong in VERIFY_LINTS instead): collapsible_if,
   collapsible_else_if, single_match, redundant_pattern_matching,
   manual_strip, question_mark, range_plus_one, iter_next_loop,
   while_let_on_iterator (verify-leaning), clone_on_copy (type-blind —
   Copy detection needs rustc).
3. Per lint: L0 corpus entry (bounded fix-space) + proposer match row +
   fixture tests (buggy → fixed, per the T6.4 test pattern) + an
   adversarial clean-snippet test asserting the visitor does NOT fire.

## Acceptance

- [ ] Frequency-ranked lint list committed (source data cited).
- [ ] ≥ 20 new structural lints land in `FIXABLE_LINTS` (or explicitly
      triaged into `VERIFY_LINTS` with the type-blindness reason recorded).
- [ ] Per-lint fixture tests: buggy in → fixed out; clean in → no edit.
- [ ] `--suggest` retrieves each new rule (latent + BM25 both find it).
- [ ] G3 no-regression: default `cargo test --lib` green.
- [ ] Bench note appended (corpus size before/after; suggest smoke time).

## Non-goals

- No type-aware proposers — the parse-only/verify split stands (Issue 015
  owns the verify path).
- No target of "all ~700 lints" — value is the frequent structural subset.
