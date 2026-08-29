# Issue 056 — perf-fixture purity is SAMPLED, not enforced: two known violations are invisible to the test

**Status:** OPEN — filed 2026-08-29 off the Layer-5c red that `819f77c` fixed. The
instance is fixed; the CLASS is not. Two more `PERF_SPANS` fixtures carry a clippy
lint today and no test can see them, because the purity assertion only walks the 12
`CROSS_PAIRS` constituents rather than the whole const. Not urgent (nothing is red),
but it is a latent repeat of the exact break that just cost a CI cycle.

## What happened (the instance, already fixed)

Layer 5c (`--all-features --tests`) went red at HEAD `075dd3f`: 1588 passed, 2 failed.

```
draft::switch_cost::tests::cross_domain_constituents_carry_no_cross_domain_proposer
draft::switch_cost::tests::cross_domain_fixture_labels_verify_via_proposers
  -> "perf fixture for retain-inplace carries clippy lint manual_is_multiple_of"
```

`bf991f7` (2026-08-29 06:13) added the `manual_is_multiple_of` proposer. The
`retain-inplace` fixture in `src/draft/eval_fixtures_perf.rs` had contained
`id % 7 != 0` since long before — exactly that lint's pattern. Fixed in `819f77c`
by dropping the lint pattern while keeping the fixture's `filter().collect()`
shape. Floors re-verified: `draft::switch_cost` 5/5, `draft::retrieval_eval` 14/14
including `retrieval_precision_regression_gate` — no re-pin needed.

## The actual defect: the invariant is sampled

`cross_domain_constituents_carry_no_cross_domain_proposer` iterates `CROSS_PAIRS`
(12 pairs) and checks `perf_fixture(rule)` for each. `PERF_SPANS` is much larger,
so any fixture outside those 12 can carry a clippy lint indefinitely with every
test green. That is the same omission-SILENT design the Layer 5c allowlist was
deleted for — and its own removal comment says so:

> "That allowlist was an omission-SILENT design, and it had already rotted:
>  39 of 51 test binaries sat outside it."

## Known violations sitting there right now

Both in `src/draft/eval_fixtures_perf.rs`, both carrying `v % 2 == 0`
(`manual_is_multiple_of`):

- [ ] `filter-count-to-any`  — `.filter(|&&v| v % 2 == 0)`
- [ ] `filter-map-fuse`      — `.filter(|&&v| v % 2 == 0)`

They are invisible only because neither rule appears in `CROSS_PAIRS`. Add either
to `CROSS_PAIRS` and Layer 5c goes red again, for the same reason, with no warning.

## Proposed fix

- [ ] T1 — assert purity over **every** `PERF_SPANS` entry, not just the 12
      `CROSS_PAIRS` constituents. One loop, and the invariant stops being a sample.
- [ ] T2 — clean the two fixtures above so T1 can pass.
- [ ] T3 — the symmetric check for `CLIPPY_SPANS` (does a clippy fixture carry a
      perf rule?) — verify it is not sampled the same way.

## Why this was NOT bundled into `819f77c`

`PERF_SPANS` feeds `draft::retrieval_eval`, which holds **pinned** retrieval
floors, so every fixture text edit is a roll against those pins. One edit was
verified end-to-end and landed; three edits at once would have made a floor shift
un-attributable. T1+T2 should land together with a floor re-run, and the working
tree must COMPILE at the time — when this was filed, a sibling's in-flight
`src/domains/sec/proposers/secret_ct_eq.rs` had the lib failing E0432/E0599, so
no further verification was possible.

## Lesson

A green purity test proved only that 12 sampled fixtures are pure. "The test
passes" and "the invariant holds" are different claims whenever the test walks a
subset — and the subset is never written down as a limitation.
