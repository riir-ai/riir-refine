# Issue 022 — Corpus real-clippy drift: 5 Issue-016 entries' buggy_code does not trigger the named lint

**Filed:** 2026-08-16, out of the Issue 018 GOAT (blocked G7 demo re-run).
**Status:** OPEN.
**Type:** corpus defect (proposer/real-clippy drift — the Issue 016 GOAT
verified corpus buggy shapes against our parse-only proposers, not real
cargo clippy; the Issue 017 fixture sweep verified FIXTURES against real
clippy but not the corpus ENTRIES).

## Symptom

`cargo run --example self_evolve_real_world_demo --features clippy_verify`
fails G6 (corpus truthfulness) at **35/40** — 5 entries' `buggy_code` does
NOT trigger `clippy::<name>` under real cargo clippy 1.93 (the verifier
runs `-W clippy::<lint>` on a temp crate containing the buggy code):

| Lint | Note |
|---|---|
| `comparison_to_empty` | KNOWN — Bench 022 already recorded ours implements `.len() == 0` (real clippy's `len_zero`); real `comparison_to_empty` is `s == ""`. |
| `iter_next_loop` | Shape doesn't fire under `-W clippy::iter_next_loop`. |
| `iter_nth_zero` | Shape doesn't fire. |
| `single_match_else` | Shape doesn't fire (pedantic-group lint). |
| `while_let_on_iterator` | Shape doesn't fire. |

Verified PRE-EXISTING at baseline `a532b7c` (worktree run — identical 5
failures), i.e. NOT caused by Issue 018. `fixed_resolves=Y` on all 5 — only
the buggy side drifts.

## Impact

- The G7 self-evolution demo (real clippy feedback loop, 50%→100% healing
  validation) aborts at G6 — cannot serve as regression evidence until fixed.
- `--fix` proposers for these 5 lints fire on shapes real clippy would not
  flag (parse-only detection) — the edits are still sound (they compile +
  simplify) but "fixes a real clippy warning" is overclaimed for them.

## Fix sketch (small, ~1-2 hours)

For each of the 5: craft a `buggy_code` that ACTUALLY fires the lint under
`cargo clippy -- -W clippy::<name>` (verify against real clippy like the
Issue 017 fixture sweep), update the corpus entry + any lockstep row +
check the retrieval-eval fixtures for impact (floors may shift — re-measure
+ re-pin per the Bench 022 contract). For `comparison_to_empty` decide:
widen the proposer to the real `s == ""` shape (the recorded follow-up) or
rename our rule to `len_zero` semantics.

## Acceptance

- [ ] G6 demo passes 40/40 (buggy triggers + fixed resolves, real clippy).
- [ ] Retrieval-precision regression gate still passes (floors re-measured
      if the corpus change shifted retrieval).
- [ ] G7 demo re-runs end-to-end (50%→100% healing reproduced) — restores
      the oracle-present regression evidence path.
