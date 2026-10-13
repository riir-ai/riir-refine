# Plan 232 — Batch 230: the clippy-1.99 new-lint wave (the toolchain-move re-arm)

**Status:** LANDED 2026-10-13 — all gates green (Research 261)

## T0 — the trigger (DONE)

Upstream Rust stable moved **1.98.1 → 1.99.0** (released 2026-09-28, channel
dated 2026-10-01). Never polled by cycles 63–67 (zero `1.99` adjudication
anywhere in-repo — verified by grep across AGENTS.md / HISTORY.md /
`.research/` / `.distill/`). Clippy 1.99 ships **8 new lints** + the #16972
UFCS `clone_on_copy` extension. This is the **Batch-91 precedent class**: the
1.98 wave was mined as the P9 clippy-1.98 new-lint batch at the last pin move
(corpus.rs line ~33).

Scope difference vs Batch 91: the workspace pin still rides **1.98.1** (the
bump is owner-deliberate per the rust-toolchain.toml law — NOT touched here),
so workspace populations are NOT measurable; the live surface is the PUBLIC
consumer set on stable 1.99 (out since 10-01). Probe-first against an
isolated `+1.99.0` toolchain (box stable is 1.93.0 — also untouched).

## The candidates (changelog-derived, dedup-clean)

| lint | group | fix class | note |
|---|---|---|---|
| `assert_is_empty` | pedantic | fixable | `assert_eq!(x.len(), 0)` → `assert!(x.is_empty())` (#17149) |
| `manual_bit_width` | pedantic | verify | manual bit-width arithmetic → `T::BITS` (#16902; BITS is u32 vs size_of*8 usize — type-width divergence risk) |
| `mismatched_bit_width_type` | suspicious | suggest-only | bit-width/type mismatch in shifts/bit ops (#16902) |
| `nonnull_unchecked_on_box_ptr` | complexity | TBD at probe | #17336 — shape read at probe |
| `block_scrutinee` | suspicious | verify/suggest | block-scrutinee temporary-lifetime trap (#16855); hoist-to-let fix changes drop order |
| `clone_on_copy` rider | complexity | fixable (count-neutral) | += UFCS path-call forms `T::clone(&x)` / `Clone::clone(&x)` per #16972 — the existing proposer explicitly refuses them today |

OUT (pre-adjudicated): `definition_in_module_root`,
`rest_pattern_accessible_field`, `unnecessary_rest_pattern` — restriction
(policy class, not corpus material; the standing skip-ledger verdict).

Dedup: all 5 + rider target verified zero-hit in corpus + proposers (the
lone `block_scrutinee` grep hit is an unrelated let-else TEST NAME — false
positive, read and dismissed).

## Tasks

- [x] T1 Probe: `rustup toolchain install 1.99.0` (isolated); probe crate + the PR `.patch` test shapes (round 1 decoded the REAL trigger shapes — assert_is_empty is diagnostics-forward, manual_bit_width is `BITS - leading_zeros`); every candidate verified firing + every fixed form compile-verified at BOTH 1.99.0 and 1.98.1. Probe artifacts: the session record (Research 261).
- [x] T2 Author: 4 proposer modules (assert_is_empty.rs, bit_width.rs serving BOTH bit-width lints, block_scrutinee.rs, nonnull_unchecked_on_box_ptr.rs — 3 authoring subagents over disjoint files) + the `clone_on_copy` UFCS rider (count-neutral, this session). All corpus ATOMS compile-verified.
- [x] T3 Standing gates: fixture table +5 rows (96→101); `cargo test --lib` 2608/0 default + 2972/0 feature posture; clippy ×4 postures clean.
- [x] T4 Freeze regen: v2 (94 verified / 18 newly excluded — the five atoms are AHEAD-OF-PIN-unlabelable by the 1.98.1 oracle by design + the stride reshuffles; M4 PASS) + v3 (stratification PASS); both records pasted; loudness invariants green.
- [x] T5 Score-bench: double-run gen-1 96.36% FLAT bit-identical (records 179+180).
- [x] T6 GOAT: G1 parse/syn (fixture table green); G2 lib tests; G3 clippy 0 all four postures.
- [x] T7 Docs: Research 261; AGENTS.md count line 103→108 + the B230 row; corpus count asserts (108, FIXABLE 89); plan ticked (this file); snapshot landing row (same commit). No .docs/11_domains/clippy_lints.md exists (B228 precedent: AGENTS.md only).
- [x] T8 Commit + push (`feat: batch 230: ...`).
- [x] T9 (added at gate time): the router-annotation floor re-pin 108→102 of 141 (the 5-entry pool growth flipped 6 spans' TF-cosine routing — the Batch-200 P7 mechanism; measured 113→107, floor = measured − 5 per the Plan-030 convention; delta recorded at the constant). The lint_level AHEAD_OF_PIN_1_99 carve-out + the ahead-of-pin driver test (the pinned-driver test asserts the rows' absence).

## Honest caveats

- No workspace population evidence (pin lags) — the rules are LIVE for
  public 1.99 consumers, DORMANT internally until the owner moves the pin;
  the Batch-91 population sweep re-arms at the pin-bump follow-up.
- The 1.99 enhancements to pre-existing uncovered lints
  (`branches_sharing_code` match-arms, `approx_constant` new constants) are
  watch-only: those lints carry standing corpus verdicts (never covered);
  the changelog note is recorded in the research note, nothing staged.
