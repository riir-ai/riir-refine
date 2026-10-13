# Research 261 — Batch 230: the clippy-1.99 new-lint wave (the toolchain-move re-arm)

**Status:** DISTILLED — batch landed 2026-10-13 (Plan 232)

## The trigger (why this batch exists)

The clippy-lane re-arm condition ("needs a toolchain move or a fresh t05
histogram wave" — the standing post-B228 state) FIRED on the first leg:
upstream Rust stable moved **1.98.1 → 1.99.0** (released 2026-09-28,
channel dated 2026-10-01). Cycles 63–67 never polled the Rust channel —
zero `1.99` adjudication anywhere in-repo (verified by grep across
AGENTS.md / HISTORY.md / `.research/` / `.distill/`). Clippy 1.99 ships
**8 new lints** + the #16972 UFCS `clone_on_copy` extension. This is the
**Batch-91 precedent class**: the 1.98 wave was mined as the P9
clippy-1.98 new-lint batch at the last pin move (2026-09-05).

Scope difference vs Batch 91: the workspace pin still rides **1.98.1**
(the bump is owner-deliberate per the rust-toolchain.toml law — untouched
here), so workspace populations are NOT measurable. The live surface is
the PUBLIC consumer set on stable 1.99 (out since 10-01). Probe-first
against the ISOLATED `+1.99.0` toolchain (box stable is 1.93.0 — also
untouched; `rustup toolchain install 1.99.0`).

## Adjudication of the 8 newcomers

| lint | group | verdict |
|---|---|---|
| `assert_is_empty` | pedantic | **LANDED** — the eq/ne empty-form with value-printing failures |
| `manual_bit_width` | pedantic | **LANDED** — `<Ty>::BITS - x.leading_zeros()` → the receiver-driven `bit_width()` fix |
| `mismatched_bit_width_type` | suspicious | **LANDED** — same fix form, receiver-driven (the LHS names the wrong width — a probable bug) |
| `nonnull_unchecked_on_box_ptr` | complexity | **LANDED** — `unsafe { NonNull::new_unchecked(Box::into_raw(p)) }` → `NonNull::from_mut(Box::leak(p))` |
| `block_scrutinee` | suspicious | **LANDED** — the un-brace of a single-expression scrutinee (help-only upstream — the identity fix is ours) |
| `definition_in_module_root` | restriction | SKIP — policy class (the standing restriction verdict) |
| `rest_pattern_accessible_field` | restriction | SKIP — policy class |
| `unnecessary_rest_pattern` | restriction | SKIP — policy class |

Plus the count-neutral rider: `clone_on_copy` += the UFCS path-call forms
(#16972) — `Clone::clone(&x)` / `Ty::clone(&x)` / `std::clone::Clone::clone(&x)`
on signature-proven by-value Copy scalars → `x`. The `<Ty as Clone>::clone`
qself spelling refuses by construction (documented narrower-than-clippy).

## The probe record (2026-10-13, isolated +1.99.0)

Round 1 discovered the changelog alone misleads: `assert_is_empty` fires on
`assert!(x.is_empty())` (NOT `assert_eq!(x.len(), 0)` — the lint's direction
is diagnostics-forward); `manual_bit_width`'s trigger is `BITS -
leading_zeros` (NOT `size_of::<T>() * 8`). The PR patches (`.patch` endpoint)
carry the verified test + `.fixed` shapes — read them before authoring:

- #17149 `assert_is_empty`: type-keyed fix forms (`&Vec` → `.as_slice()`,
  owned Vec → `[] as [T; 0]` cast, str-family → `""`); custom messages +
  macro-expanded stay silent.
- #16902 `bit_width.rs` (BOTH lints, one file): fix by RECEIVER kind —
  unsigned `x.bit_width()`, signed `x.cast_unsigned().bit_width()`, NonZero
  `.get()` variants; `bit_width()` stabilized **1.97** (MSRV-gated in
  clippy at `1,97,0 { ISOLATE_LOWEST_ONE, BIT_WIDTH }`) — the lint only
  fires where the fix compiles (the chunks_exact_to_as_chunks precedent).
- #17336 `nonnull_unchecked_on_box_ptr`: only `BOX_LEAK` MSRV-gated
  (ancient); `from_mut` compile-verified at BOTH 1.99.0 and 1.98.1.
- #16855 `block_scrutinee`: NO `.fixed` upstream (help-only); unsafe-block
  scrutinees ignored upstream.

All five fixed-form families compile-verified at BOTH 1.99.0 and 1.98.1
(probe crate, this session) — the workspace pin can adopt every fix text
today; only the DETECTION is 1.99-gated.

## The lint_level carve-out (the structural finding)

`LINT_GROUPS` derives from the PINNED driver's `-W help` — the pinned
1.98.1 driver cannot know 1.99 lints. The batch introduces
`AHEAD_OF_PIN_1_99` (lint_level.rs): the five rows derived from the
ISOLATED `+1.99.0` driver (`clippy-driver +1.99.0 -W help`; groups:
assert-is-empty/manual-bit-width pedantic, block-scrutinee/
mismatched-bit-width-type suspicious, nonnull-unchecked-on-box-ptr
complexity — per-lint default levels + group blocks read), the
driver-match test skips them BY NAME (asserting the pinned driver does NOT
know them), and a NEW test verifies them against the +1.99.0 driver when
installed (UNSEEN otherwise — never a pass). **The pin-bump commit
EMPTIES the list** (rows re-merge into the live driver check in the same
commit that bumps rust-toolchain.toml to ≥1.99) — the pin-bump follow-up
owes: (a) that merge, (b) the workspace population sweep (the Batch-91
evidence class), (c) the un-exclusion of the five atoms from the gen-2/3
freeze tables (the pinned oracle can then label them).

## Gates (all green at landing)

- lib default **2608/0** (prior 2538 — +70 test arms); feature posture
  (score_bench+latent_retrieval) **2972/0**
- clippy ×4 postures clean (default / all-features / all-targets both /
  no-default-features)
- v2 freeze regen: **94 verified / 18 newly excluded** (the five new atoms
  — unlabelable by the pinned 1.98.1 oracle, the designed AHEAD-OF-PIN
  state — plus the stride-window reshuffles), M4 PASS, record pasted,
  loudness invariants green
- v3 freeze regen: stratification PASS, record pasted
- retrieval: blended floors HELD (126/132/132 of 141 — byte-identical);
  **router annotation re-pinned 108→102 of 141** — the 5-entry pool growth
  flipped 6 spans' TF-cosine routing (the Batch-200 P7 cross-pool
  mechanism), measured 113→107, floor = measured − 5 per the Plan-030
  convention; the delta is recorded at the constant
- score-bench double-run: gen-1 **96.36% FLAT bit-identical** (created
  1.8%, parse safety 1.0 — the new rules ride the gen-2 windows; the gen-1
  floors untouched)
- FIXABLE_LINTS 84 → 89; corpus 103 → 108 (the count asserts + the
  fixture table 96 → 101 rows)

## Honest caveats

- The rules are LIVE for public 1.99 consumers, DORMANT internally until
  the owner moves the pin — by design (the corpus is the product's
  knowledge base; the healer's oracle is the TARGET's clippy).
- The 1.99 ENHANCEMENTS to pre-existing uncovered lints
  (`branches_sharing_code` match-arms, `approx_constant` +
  EULER_GAMMA/GOLDEN_RATIO) are watch-only: those lints carry standing
  corpus verdicts (never covered); nothing staged.
- The router annotation decay (−6 spans) is real and recorded — a 1.99
  follow-up batch that grows the clippy pool further owes the same
  re-measure discipline.
