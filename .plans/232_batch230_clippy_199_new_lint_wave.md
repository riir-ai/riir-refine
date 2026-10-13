# Plan 232 — Batch 230: the clippy-1.99 new-lint wave (the toolchain-move re-arm)

**Status:** IN PROGRESS — claimed 2026-10-13 ~21:4x +07 (claim row `66f9e41a`, pushed)

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

- [ ] T1 Probe: `rustup toolchain install 1.99.0` (isolated); temp crate
  with triggering shapes per candidate; `cargo +1.99.0 clippy
  --message-format=json` — verify each lint FIRES, capture the suggestion
  text + applicability (MachineApplicable / MaybeIncorrect / help-only) +
  co-fire census + silence arms. A candidate whose probe contradicts the
  fixable read re-tiers or drops (probe evidence is the landing authority —
  Batch 33 precedent).
- [ ] T2 Author: 5 corpus entries + proposers (per probe shapes, bounded
  guards per house style) + the `clone_on_copy` UFCS rider (proposer edit,
  count-neutral); entry ATOMS rustc-verified (the Batch-183 E0308 lesson:
  `fixed_code` must COMPILE, not just parse).
- [ ] T3 Standing gates: extend `batch230_pseudocode_parses` in the
  syn-parse gate; `cargo test --lib` green; clippy ×4 postures green
  (default / all-features / all-targets / no-default-features).
- [ ] T4 Freeze regen (clippy corpus delta — plan 137 T7.5): regenerate
  gen2/gen3 freeze tables in this batch (the `#[ignore]`d
  `generation_v2_freeze`/`generation_v3_freeze` e2e).
- [ ] T5 Score-bench: `cargo refine --score-bench` (append-only history).
- [ ] T6 GOAT: G1 parse/syn (all new rules); G2 lib tests; G3 no-regression
  (clippy 0 both feature states + `./scripts/ci_feature_guard.sh`).
- [ ] T7 Docs: research note 261; AGENTS.md clippy count line + batch row;
  `.docs/11_domains/clippy_lints.md` B230 row; snapshot landing row (same
  commit); corpus count via `cargo refine --corpus-stats` (derived, never
  hand-typed).
- [ ] T8 Commit + push (`feat: batch 230: ...`); CRLF ritual (new files
  LF; the distill file consistent-CRLF).

## Honest caveats

- No workspace population evidence (pin lags) — the rules are LIVE for
  public 1.99 consumers, DORMANT internally until the owner moves the pin;
  the Batch-91 population sweep re-arms at the pin-bump follow-up.
- The 1.99 enhancements to pre-existing uncovered lints
  (`branches_sharing_code` match-arms, `approx_constant` new constants) are
  watch-only: those lints carry standing corpus verdicts (never covered);
  the changelog note is recorded in the research note, nothing staged.
