# Issue 259 — the rulewright's `while_let_on_iterator` rewrite moves an iterator the loop body still reads (E0382)

**Status:** OPEN — filed 2026-11-25 (katgpt-rs-dc@amo) from a wrong heal found while unblocking riir-train Plan 342 B5. T1+T2 LANDED 2026-11-25 (zed-efbee190@amo); T3 (verify at required-features) stays open.
**Claimed:** zed-efbee190@amo, 1795617389, expires 2026-11-27T14:30Z

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

- [x] T1 — DONE 2026-11-25 (zed-efbee190@amo): the rule's use-check now covers macro expansion — a locally-defined `macro_rules!` whose definition tokens mention the iterator ident (with `$`-metavariable placeholders skipped, same-token-nesting-level adjacency) or any body invocation whose argument tokens mention it (`println!("{}", argv)` — the same hole, also fixed). Post-loop half: any mention of the ident after the loop in the enclosing fn (text scan + hidden macro uses) also declines — a post-loop use breaks under the move with the same E0382, and the `.by_ref()` form that would serve there is NOT emitted (probed: `for x in it.by_ref() { it.next(); }` compiles — rustc accepts the by_ref+body shape — but `it.by_ref()` post-loop breaks the non-`mut` `&mut I` binding shape, E0596 — so decline is the sound answer everywhere; T1's "or do not fire" branch). Probe receipts: E0382 reproduced on the old rewrite of the real incident shape; by_ref probes recorded above.
- [x] T2 — DONE same session: seven-fixture battery in the proposer's test mod — the exact `aad54860` `next_str!` shape (no rewrite), macro-arg mention (no rewrite), post-loop use (no rewrite), sequential while-lets (FIRST refuses, SECOND converts — the asymmetric coordination, rustc-verified: the mixed while+for pair compiles), post-loop hidden macro use (no rewrite), clean-std-macro body still converts (over-refusal guard), plus the five pre-existing tests unchanged. Full lib 3998/0; `cargo clippy --lib -- -D warnings` clean; fmt gate PASS; score-bench appended (heal 0.964 / decline_correctness 1.0 / created 0.018); e2e: the real incident shape through `cargo refine fix --no-verify` fixed 0 edits (`for arg in argv` count 0, file byte-identical).
- [ ] T3 — batch verify at required-features: still OPEN (a fix_verify behavior change, separable from the rule fix). The P43 audit already NAMES the gap per kept edit; what remains is auto-deriving the feature set from the manifest and verifying at it, instead of only disclosing.

## Discovered adjacent residue (this session, not this landing)

- `src/domains/rust_perf/corpus/tests.rs` carries 6 pre-existing clippy findings under `--all-targets` (3x doc-list-indentation @13407-13409, missing backticks @13595, 2x too_many_lines @13602/@13721) — test-cfg only (the `--lib` posture is clean), tree-verified pre-existing at HEAD (the file is untouched by this landing). The batch lanes' "clippy -D x3 postures" do not include `--all-targets` — a coverage gap for the corpus-lane follow-up, not this issue's fix.
- The maintained binary was rebuilt after the fix landed (the Issue-153 release feature set; `--version` prints the post-commit hash).
