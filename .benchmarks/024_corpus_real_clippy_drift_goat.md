# Bench 024 — Issue 022: corpus real-clippy drift fix (5 entries) + G7 demo restored

**Date:** 2026-08-16.
**Issue:** 022 (resolved + removed; found during the Issue 018 GOAT).
**Scope:** correct the 5 Issue-016 corpus entries whose `buggy_code` did not
trigger the named lint under real cargo clippy **1.97** (the toolchain
upgraded from 1.93 since Issue 016/017 authoring). Unblocks the G6/G7
self-evolution demo (the oracle-present evidence path for Issue 018).

## Root causes (each verified empirically — temp-crate + `cargo clippy --message-format=json` probes)

| Lint | Root cause | Fix |
|---|---|---|
| `iter_next_loop` | Corpus code **didn't compile** — `.next()` needs `&mut self`; the missing `mut it` was invisible to parse-only proposers | `mut it` both sides |
| `while_let_on_iterator` | Same missing-`mut` compile failure | Same |
| `iter_nth_zero` | `.iter().nth(0)` on a SLICE receiver routes to clippy 1.97's more-specific `iter_nth`; `iter_nth_zero` fires on non-slice iterators | `mut lane: IntoIter<u8>` + `lane.nth(0)` (mirrors the Issue 017 fixture shape) |
| `single_match_else` | A bare-default else routes to `manual_unwrap_or[_default]`; the lint fires only when the else arm is a STATEMENT BLOCK (10 shapes probed — the Issue 017 fixture had the firing form all along) | arm-expr + statement-block else → if-let-else |
| `comparison_to_empty` | The recorded Bench 022 drift: ours implemented `.len() == 0` = real clippy's `len_zero`; the REAL shape is `s == ""` | Corpus → `s == ""` → `s.is_empty()`; proposer WIDENED to detect both forms (the len form stays useful for workspace healing — same fix either way) |

## GOAT

| Gate | Result |
|---|---|
| G6 (corpus truthfulness, real clippy) | ✅ **40/40** (was 35/40) — buggy triggers + fixed resolves, all 40 entries |
| **G7 (self-evolution, real cargo clippy feedback)** | ✅ **baseline 50.0% → evolved 100.0% (40/40, +50.0pp)** — the full Issue 018 oracle-present validation, now on the 40-lint corpus (Issue 008's original G7 ran the 10-lint corpus) |
| G3 (no-regression) | ✅ 408 default / 429 latent_retrieval lib tests; clippy 0 warnings in 4 states; `ci_feature_guard.sh` ALL LAYERS (incl. the 652-test combined suite); lockstep `all_buggy_codes_fire_their_lint` green on the new shapes; proposer suite +4 tests (`== ""` forms + non-empty negative) |
| Retrieval gate | ✅ **floors RE-PINNED per the Issue 017 contract** (recorded, never silent): clippy 58.3/75.0/87.5 → **54.2/70.8/83.3**, blended 2-domain 71.6/84.1/92.0 → **69.3/81.8/89.8**, 3-domain 78.8/88.1/94.1 → **77.1/86.4/92.4**, perf + kernel unchanged (kernel 100/100/100 — the crowding canary holds). The −2 clippy spans are the 2 len-shaped `comparison_to_empty` fixtures (rank 3 + rank 7) — the truthful corpus text no longer lexically anchors the len shape. Truthfulness wins: a corpus that misrepresents lint behavior poisons `self_evolve` learning. |

## Bonus root-cause fix (found by the determinism test)

`eval_report_is_deterministic` flaked IN-PROCESS (79 vs 78 top-5) after the
corpus change put a score TIE exactly on the top-5 boundary — the matcher's
ranking resolves ties by backing-map iteration order. Fix (eval-local, the
measurement owns its tie-break): `canonical_rank` re-sorts the matcher's
returned top-8 by (`score` desc via `total_cmp`, then `rule_id` asc) before
any rank/counting — 3 consecutive runs identical. A substrate-level
deterministic tie-break in riir-rag remains a documented follow-up.

## Method notes

- The probe harness gotcha: when a lint is requested on the command line,
  rustc renders the code HYPHENATED (`clippy::needless-return`) in human
  output — underscore greps see nothing. Use `--message-format=json` (the
  `"code":{"code":"clippy::..."` field is underscore-stable).
- Fixture-2 experiment (rejected): rewriting the len-shaped fixture to
  `pending == []` improved top-5 but broke proposer verification (`== []`
  not detected — array-literal form out of scope) + still missed top-1;
  reverted to the original len shape with a documenting comment instead.
- The `single_match_else` firing shape came from the Issue 017 fixture
  (statement-block else) — the corpus + fixtures now agree on it.
