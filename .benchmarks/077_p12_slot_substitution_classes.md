# Bench 077 — P12 slot-substitution classes: mutator header, mutable-iter exclusivity guard, template-less family-macro descent

**Status:** PASS — all three actionable P12 classes landed with sound parse-only fixes; the intake's "parse-only undecidable" premise FALSIFIED for every class; both pre-registered predictions (series stability) met exactly. One NEW broken-output class (C4) found + fixed beyond the intake.
**Date:** 2026-09-07
**Pre-registration:** `.plans/100_p12_slot_substitution_classes.md` (T0 evidence + design + predictions committed BEFORE the series re-runs).
**Probe rig:** `/tmp/p12_probe` scratch crate; global `cargo-heal` binary verified fresh against the last proposer commit (`784b673`) before probing; healed outputs compiled with `cargo check` (E0596/E0382 reproduced, then eliminated).

## What was measured (T0 — the design's ground truth)

| # | Input | Before this unit | After |
|---|---|---|---|
| C1 | owned `mut v` + `v[i].push(1)` | emits `for __elem in &v` + `(*__elem).push` → **E0596** | `for __elem in &mut v` — compiles |
| C4 (NEW) | `&mut v` param + non-slot body use (`v.len()`) | emits bare iter + `v.len()` → **E0382 borrow of moved `v`** | refuses (the exclusive-borrow shape cannot heal with another body use) |
| C2 | `assert_eq!(v[i], 2)` | silent refuse (missed heal) | `assert_eq!(*__elem, 2)` — compiles (expansion `&(*__elem)` = `&u32`) |
| C2b | `assert!(v[i] > 2)` (template absent) | silent refuse | `assert!(*__elem > 2)` — compiles |
| C3 | `let x = v[i]; if x > 2` | `let x = *__elem` — **already compiles** | unchanged; pinned (the intake's residual here was stale — the P11 universal splice already covered it; the input universe is Copy-only because non-Copy `let x = v[i]` is E0507 in the ORIGINAL) |

C4 is NOT in the intake — found by the T0 probe (`&mut` params were folded into
"shared" by the old `param_is_reference`; a bare iter of `&mut` MOVES the
borrow, so any non-slot body use breaks). Bonus heal measured en route: the
tracing named-target form (`log::info!(target: t, "at {}", v[i])`) now heals
(the old conservative walk refused it); a named capture hidden in ANY string
literal of a template-less family macro refuses (the new descent's
load-bearing capture guard — the old walk never looked inside literals).

## The fixes (root cause, in `needless_range_loop.rs`)

1. **T1 mutator header**: `MUTATOR_METHODS` (37 known `&mut self` receivers,
   consuming methods deliberately excluded — E0507 in every header) triggers
   `needs_mut`; `param_iter_form` splits Shared / Mut / Owned{mut_decl}; an
   owned-`mut` param emits `&mut v` under the trigger; shared/non-mut owned
   refuse defensively (unreachable inputs); `Mut` params keep the bare iter
   (C1b — already correct).
2. **T1 exclusivity guard**: `collection_uses − exact.len()` counts non-slot
   body uses of the collection; ANY mutable-iter shape (`&mut v` emission OR a
   `&mut` param) with a non-slot use refuses. This is the C4 fix.
3. **T2 template-less family descent**: when the family table matches but the
   template position holds no plain string literal (eq-family operands;
   message-less `assert!`/`ensure!` conditions), `scan_macro` descends
   `all_expr_args` (pre-existing exporter) instead of the conservative
   token-walk refusal; a hidden named capture in any literal token refuses.

Place-write refusal, `&v[i]` address-of refusal, complex-index and
other-collection-index refusals: byte-unchanged (`gen7-ref-nrl-write`
territory intact — confirmed by the series re-runs below).

## Validation (all first-shot green except where noted)

- lib default: **1156/0** (floor 1142 held; +14 new fixtures, 30→44 nrl tests)
- lib + score_bench: **1357/0/8i**
- integration trio (goat_gate_004, latent_suggest_goat, stage2_goat): **13/0/2i**
- clippy `-D warnings`: lib default clean; lib+score_bench clean AFTER fixing
  the pre-existing `generations.rs:823` doc-markdown straggler (one-backtick
  mechanical, landed by `784b673`'s record, empty-diff before this unit);
  `--all-targets` default: 0 findings
- gen-7 series ×3: **87.5% heal / 12.5% created / parse 100%** — deterministic,
  IDENTICAL to the Plan-099 landing (prediction 1: unchanged — met)
- v1 score-bench: **96.2% heal / 1.9% created / parse 100%** (E4 gate holds —
  prediction 2: met; jsonl rows appended, UTC timestamps 21:05–21:06Z = 04:05–04:06 +0700)
- Healer e2e re-probe (rebuilt `--features fix_verify,clippy_verify` release
  binary): all four healed shapes compile; both broken shapes now refuse.

## Honest caveats

- The mutator set is NON-exhaustive by design: a custom `&mut self` receiver
  still emits the shared shape (`--verify` auto-reverts it in production).
  This residual is the intake's irreducible core — unlistable parse-only.
- The template-less capture guard scans literal TOKENS of the invocation
  only; a macro whose args build a format string at runtime (rare) is out of
  scope for a parse-only proposer.
- Bench numbers from the isolated `CARGO_TARGET_DIR=/tmp/p12_target` runs
  (sibling session building in riir-chain concurrently; no shared target dir).
- `score_history.jsonl`/`svg` re-rendered by the appended rows; committed with
  this unit.

**Records:** Plan 100 (→ RECORD), snapshot P12 → DONE (+C4 amendment row).
