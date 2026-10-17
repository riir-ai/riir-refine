# Issue 168 — five corpus rules key lints clippy 0.1.98 no longer ships

**Status:** open — corpus hygiene, toolchain-move follow-up (the Batch-230 re-arm class, inverted)

`cargo refine --coverage-report` discloses per run:

```
⚠ 5 corpus rule(s) name a lint this driver does not ship (renamed/removed):
  assert_is_empty, block_scrutinee, manual_bit_width, mismatched_bit_width_type,
  nonnull_unchecked_on_box_ptr
```

The warning is the instrument being honest; the follow-up nobody filed is the
KEYS themselves. Five clippy_lints corpus rules are keyed to lint names current
clippy (0.1.98) does not emit — renamed or removed upstream since the rules were
authored. On the current toolchain they are dead keys: the auto-oracle lane never
surfaces a diagnostic carrying those names, so the rules never rank. (The
patterns may still heal via span matching — the coverage report's name-keyed
buckets are the only thing that goes blind, plus oracle-fed ranking.)

| corpus rule file | stale key | likely current successor |
|---|---|---|
| `proposers/assert_is_empty.rs` | `assert_is_empty` | removed upstream (the `.assert(!x.is_empty())` shape; verify before retiring) |
| `proposers/block_scrutinee.rs` | `block_scrutinee` | renamed/removed (verify in clippy history) |
| `proposers/bit_width.rs` | `manual_bit_width` | `manual_bits` (verify) |
| `proposers/bit_width.rs` | `mismatched_bit_width_type` | verify — possibly folded into `manual_bits` family |
| `proposers/nonnull_unchecked_on_box_ptr.rs` | `nonnull_unchecked_on_box_ptr` | verify (box/nonnull rework upstream) |

## Fix shape (the Batch-230 precedent, inverted)

Batch 230 handled upstream ADDING lints (AHEAD_OF_PIN list, emptied at pin bump).
This is the inverse: upstream renamed/removed lints our corpus keys on. Per
stale key: verify the rename/removal in clippy's history, re-key to the current
name if one exists (a re-key = corpus CONTENT delta -> the plan-137 T7.5
freeze-table regeneration rides the SAME batch), or retire the key and keep the
rule pattern-first (annotated, like the divergence classes) if none exists.

## Constraints

- Any re-key moves `build_seed_corpus()` output -> gen2/gen3 freeze tables MUST
  regenerate in the same batch (plan 137 T7.5) + `--score-bench` after.
- Verify each successor name by running the real clippy (probe-crate discipline),
  never by memory — a wrong rename re-key is worse than the dead key.
- Must not collide with the B250 lane's corpus work: sequence after it lands
  (its freeze regen would otherwise need this folded in).

## Verification

- The ⚠ line's list shrinks to only keys VERIFIED absent-with-no-successor
  (retired keys are a deliberate disclosure, not silence).
- Freeze gates green at the new tables; score-bench appended.
