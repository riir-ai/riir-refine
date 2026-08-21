# Bench 041 — Behavioural-equivalence instrument: the 2 silent semantic breaks are now caught automatically

**Verdict: INSTRUMENT VALIDATED — 9/9 harness cases PASS. Both silent semantic
breaks that [Bench 039](039_l4_vs_decline_reachable_set.md) scored as WINs are
now detected mechanically, and none of the 3 clean wins is falsely rejected.**

Date 2026-08-21. Closes the named sub-blocker inside riir-train Plan 336 **T5.1**
and **T5.3**. Harness `tests/t51_behavioural_equivalence.rs`, implementation
`src/behavioural_equivalence.rs`. CPU-only, no model, no GPU, no checkpoint.

## Why this exists

Bench 039 scored a candidate fix as a WIN on `cargo clippy` clean **AND**
contract-identifier preservation. At n=9 that gave 5 wins, and 2 were wrong:

```
option_if_let_else  None   : orig=0     l4=1          WRONG
manual_strip        "xyz"  : orig=None  l4=Some("z")  WRONG
```

`o.unwrap_or(0) + 1` adds 1 to the `None` branch; `s.get(2..)` drops the
`starts_with("ab")` guard. Both preserve **every** public identifier, so the
contract check cannot see them. Plan 336 T5.3 recorded the consequence: *"Any
future L4 promotion criterion must require behavioural equivalence, not just
contract preservation — otherwise `p_win` is an upper bound at any `n`."*

T5.1 recorded the fix and its cost: *"property tests would catch both failures
with a handful of random inputs; it is CPU-only and needs no model."* This is
that, built and measured.

## Method

Differential property testing on the **pair**. Parse the target fn's signature
with `syn`, derive a bounded input domain per argument type, emit one
dependency-free driver, compile with plain `rustc` (not cargo — no dep graph, no
shared target dir), and compare `{:?}` of the two returns over every input.
First divergence wins. Both sides run under `catch_unwind`, so "one panics and
the other does not" is itself a divergence.

Deterministic by construction: fixed literal corpora plus a fixed-seed LCG (the
same discipline as Plan 336 T3.4a's advantage-floor harness). Re-running cannot
change a verdict.

**Adversarial values come first in each corpus, on purpose.** `Option<Int>`
yields `None` as case 0; `&str` yields `"xyz"` as case 0. The two documented
breaks are therefore caught on the *first* comparison, not found by luck.

## Results — 9/9

| # | case | expected | got |
|---|---|---|---|
| 1 | `option_if_let_else` break — `o.unwrap_or(0) + 1` | Divergent | **Divergent** — input `None`, orig `0`, cand `1` |
| 2 | `manual_strip` break — `s.get(2..)` | Divergent | **Divergent** — input `"xyz"`, orig `None`, cand `Some("z")` |
| 3 | both breaks invisible to a name-based check | true | **true** — the contract check provably cannot see either |
| 4 | `or_fun_call` correct fix (`unwrap_or_else`) | Equivalent | **Equivalent** |
| 5 | `ptr_arg` correct fix (`&Vec<u8>` → `&[u8]`) | Equivalent | **Equivalent** (across the signature change) |
| 6 | `redundant_clone` correct fix (zero-arg) | Equivalent | **Equivalent** |
| 7 | correct fixes for both breaks (`map_or`, `strip_prefix`) | Equivalent | **Equivalent** |
| 8 | `box_collection` (user type, no bounded domain) | Unsupported | **Unsupported** — and `is_equivalent() == false` |
| 9 | non-compiling candidate | Unsupported | **Unsupported** |

Case 3 is the load-bearing one: it proves the *reason* behavioural equivalence
had to be added rather than the contract check tightened. Cases 4–7 are what
make the instrument usable — a checker that rejected the 3 clean wins would be
worthless in the other direction. Case 5 matters specifically because the
`ptr_arg` fix **is** a signature change, so the checker compares across
`&Vec<u8>` vs `&[u8]` by generating one owned `Vec<u8>` and handing each side
the borrow its own signature asks for.

`cargo clippy` **0 warnings** on the new module and its test (crate runs
`clippy::pedantic`). Runtime **1.7 s** for all 9 (rustc dominates; each driver is
dep-free).

## What this does NOT establish

- **It is a refuter, not a prover.** `Equivalent` means "no divergence over this
  bounded domain", never a proof. `Unsupported` is deliberately **not** a pass —
  `is_equivalent()` returns false for it, mirroring Bench 039's choice to
  exclude rather than flatter an unscoreable fixture.
- **Bench 039 is not re-scored end-to-end here.** Plan 336 records the verbatim
  L4 output for the 2 breaks but only the lint names for the 3 clean wins, so
  cases 4–6 use the *canonical* correct fix rather than the adapter's actual
  text. Re-deriving the real outputs needs the adapter — GPU + ~13 GB RAM —
  which was unavailable (measured 941 MB free of 64 GB, with a Plan 341 probe
  resident). **The instrument is validated; the adapter re-scoring is a separate,
  GPU-gated run.**
- **T5.1 and T5.3 remain unchecked.** This removes the *measurement* blocker, not
  the export bar: the adapter is still 4 regressions + 2 semantic breaks out of
  9. What changed is that "every WIN in this tree is an upper bound" no longer
  holds — the upper bound can now be tightened mechanically.

## Consequence for the gate

A promotion criterion can now require all three, where it previously had two:

1. `cargo clippy` resolves the lint,
2. contract identifiers preserved,
3. **behavioural equivalence over the generated domain** ← new.

Applied to Bench 039's 5 WINs, criterion 3 reclassifies 2 of them as breaks,
giving `p_win_clean = 3/9 = 0.33` mechanically rather than by hand-inspection.

## References

- [Bench 039](039_l4_vs_decline_reachable_set.md) — the n=9 run this instruments
- riir-train Plan 336 T5.1 / T5.3 — the tasks that named this blocker
- `src/behavioural_equivalence.rs`, `tests/t51_behavioural_equivalence.rs`
