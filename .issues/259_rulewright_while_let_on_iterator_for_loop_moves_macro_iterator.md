# Issue 259 — the rulewright's `while_let_on_iterator` rewrite moves an iterator the loop body still reads (E0382)

**Status:** OPEN — filed 2026-11-25 (katgpt-rs-dc@amo) from a wrong heal found while unblocking riir-train Plan 342 B5; unclaimed.

## The wrong heal (measured)

riir-train `aad54860` (2026-11-24, a rulewright batch over 113 files) rewrote

```rust
let mut argv = std::env::args().skip(1);
while let Some(arg) = argv.next() {
    match arg.as_str() {
        "--gguf" => args.gguf = PathBuf::from(next_str!("--gguf")), // next_str! = argv.next()...
```

to `for arg in argv {`. Each arm pulls its value from the SAME iterator through a local
`macro_rules!` (`next_str!` / `next_usize!` expand to `argv.next()`), so the `for` loop's
by-value move makes every macro call a use-after-move — **E0382; three examples stopped
compiling on develop**: `bonsai_clippy_l4_eval.rs`, `bonsai_clippy_l4_margin_probe.rs`,
`bonsai_clippy_l4_sft_train.rs` (riir-train-engine, feature `bonsai_clippy_l4`). Repaired by
hand in riir-train `1a6a4fb8`. riir-train Issue 470 records the same shape in the RLVR trainer.

clippy's own `while_let_on_iterator` lint does not fire here when the body uses the iterator
(it suggests `for x in iter.by_ref()` in that case) — the rewrite either ignored macro
expansion when checking body uses, or dropped the `.by_ref()`.

## Why the compile gate did not catch it

The three examples are `required-features = ["bonsai_clippy_l4"]`; a `cargo refine fix`
verify at default features skips them (the green-zero class). The batch needed
`--verify-args "--features bonsai_clippy_l4"` — or the rule must not fire unless it can see
the whole body.

## Tasks

- [ ] T1 — the rule's body-use check expands (or conservatively refuses on) `macro_rules!`
  invocations defined in the same fn that mention the iterator ident; when any body use
  exists, emit `for arg in argv.by_ref()` (clippy's own suggestion) or do not fire.
- [ ] T2 — regression fixture: the three examples' exact shape (local macro reading the
  iterator), asserting no rewrite or the `.by_ref()` form, and that the result compiles.
- [ ] T3 — batch verify: a rulewright batch touching a `required-features` target verifies at
  that target's features (derive from the manifest), never default-only.
