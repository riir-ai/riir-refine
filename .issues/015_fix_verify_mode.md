# Issue 015 — `--fix` verify-mode: make the parse-only gates self-verifying instead of conservative

**Filed:** 2026-08-14, out of the Bench 019 §G5 addendum session (T6.2).
**Status:** OPEN.
**Type:** tool improvement (proof-of-value follow-up, not a bug — the current
conservative gates are CORRECT, they just leave value on the table).

## Background

The G5 real-file run (katgpt-rs `katgpt-pruners`, commit `8c331cd9`) proved
the compile gate is the real safety net: run 1 wrote 30 edits, 6 errors in 4
files; the three proposer bug classes were fixed at root cause; run 2 (gated
proposers) landed 12 files / 20 edits clean. But the NEW gates are
deliberately conservative because `--fix` is parse-only:

1. **vec-with-capacity usize gate** — plain-path capacity bounds (consts,
   associated consts, locals like `min_len`/`n`/`len`) are skipped even when
   they are `usize` in context. On the G5 batch this skipped 8 in-context-
   correct sites (`hydra_budget` `n`, `collapse_detector` `N_TRACES`,
   `rosetta` `len`, `episode_pruner` `min_len`, `monopoly` `Strategy::COUNT`,
   ...). The scan found 62 vec-with-capacity candidates repo-wide; the
   whitelist landed 18.
2. **fxhashmap dep blindness** — 422 of 498 katgpt-rs candidates were
   fxhashmap rewrites, ALL unusable because no katgpt-rs crate depends on
   `rustc-hash` (the emitted `use rustc_hash::FxHashMap` fails to resolve).
   Documented per-rule, now with a measured repo-wide instance.

## Proposal — verify-mode

Add a `--fix --verify` mode that closes the loop IN the tool:

1. Apply edits (current behavior).
2. Run `cargo check -p <owning crate>` (or `cargo check` at the workspace
   root with a filter) against the edited tree.
3. On compile error: map errors back to files (rustc's `--error-format
   json`), revert ONLY the offending files (or the offending spans), re-check,
   report what was kept vs reverted.

With verify-mode, the gates can be loosened: path-typed capacity bounds can
be attempted + auto-reverted on E0308; redundant_closure can be attempted +
auto-reverted on E0631 (restoring it to `--fix` under `--verify`); fxhashmap
can additionally read the target crate's `Cargo.toml` for a `rustc-hash`
dependency and hard-skip when absent (cheap, no cargo invocation).

This is the `rust_perf_verify`/`clippy_verify` pattern (already shipped,
opt-in) applied to the edit path instead of the draft path.

## Acceptance

- [ ] `--fix --verify` exists (opt-in feature, e.g. `fix_verify`; implies
      `latent_retrieval`).
- [ ] Cargo.toml dep gate for fxhashmap (no cargo run needed — text scan of
      the owning crate manifest).
- [ ] Revert-on-error proven on a fixture repo: intentionally-bad edits
      (u8 capacity path, fn-path coercion closure) are auto-reverted; good
      edits survive.
- [ ] GOAT: re-run the katgpt-pruners G5 batch under verify-mode — the 8
      skipped in-context-correct vec-with-capacity sites land (compile
      confirms `usize`), zero regressions in `cargo test`.
- [ ] Bench note appended to 019 (or a new bench if numbers warrant).

## Non-goals

- No type checking inside the proposer (syn-only stands; verify-mode uses
  real rustc).
- No auto-adding the `rustc-hash` dependency to healed crates — that is a
  consumer decision, the tool can suggest it.
