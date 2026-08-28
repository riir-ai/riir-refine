# Issue 055 — `rust_perf` corpus count pins are 2 rules stale; Layer 5b + 5c are RED at HEAD

Status: **OPEN — filed 2026-08-28, not fixed here (owner: whoever landed the two post-B83 rules)**

## What

`cargo test --lib` fails at HEAD (`e40cb6e`) — 5 failures on default features,
7 on `--all-features`. Every one is a stale count pin in
`src/domains/rust_perf/corpus.rs`, and they move together:

| Test (line) | pinned | actual | delta |
|---|---|---|---|
| `seed_corpus_has_eighty_eight_entries_across_twelve_categories` (7462) | 89 | **91** | +2 |
| `seventy_nine_rules_have_bench_specs` (8097) | 79 | **80** | +1 |
| `ten_rules_are_not_benchable` (8110) | 10 | **11** | +1 |
| `twenty_eight_annotated_rules_carry_context_notes` (8159) | 32 | **33** | +1 |
| `unannotated_rules_have_no_context_notes` (8201) | 57 | **58** | +1 |
| `context_notes_backward_compatible_serde` (8213) | 32 | **33** | +1 |
| `drafter_propagates_context_notes_from_perf_corpus` (8247) | 32 | **33** | +1 |

Internally consistent with **two rules landed**: one annotated + benchable,
one unannotated + not-benchable. The pins, the test NAMES (which spell their
numbers out) and the per-category doc comment were not bumped with them.

## Why it is not fixed in this commit

Bumping seven integers is mechanical, but a correct fix also renames the tests
and updates the doc comment's per-category attribution ("Batch NN added …") —
and that needs the batch that landed the two rules, which its author knows and
a reader cannot infer. `git log` after B83 (`5225957`, "rust_perf 88->89")
shows only a **1-line** insertion into `corpus.rs` (`e40cb6e`), so the two
extra entries are not visible as a corpus commit at all. Guessing here would
paper over whatever produced that.

## Not caused by the docker promotion

Verified before filing: the failures reproduce with `docker` removed from the
default list (`--no-default-features --features clippy_lints,rust_perf,self_evolve`
and with the default line reverted). This is a pre-existing red.

Note that `e40cb6e`'s message claims "all-features lib 1505/0 + full CI guard
ALL LAYERS PASS". That claim does not hold at HEAD now — worth understanding
which of the two is stale before the next batch lands on top.

## Fix shape

1. Identify the two post-B83 rules.
2. Bump the 7 pins, rename the 4 count-named tests, update the category
   comment's attribution line.
3. Re-run `./scripts/ci_feature_guard.sh` Layers 5b + 5c.
