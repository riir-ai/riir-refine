# Issue 261 — rulewright divergence: the deref→unsize trait-object rewrite class compiles to NOTHING at the verifying posture (riir-chain E0277 + a dropped let-binding casualty)

**Status:** Open — filed 2026-11-28 (amo idle lane zed-eb03c1fb@amo) from the riir-chain heal; both consumer-side casualties are ALREADY FIXED (riir-chain `45cce96a`); this issue owns the proposer-side root cause + the posture hole.

## The two casualties (same rulewright session, zed-8fa4839b@amo 2026-11-24, commits 3180cdf4 + 52529a02)

1. **`explicit_auto_deref` rewrote `&*self.nft_registry` → `&self.nft_registry` at an `&dyn RecipientRegistry` parameter** (`src/forensic/attribution_agent.rs:324`). Rust does NOT chain deref→unsize for trait objects in argument position — `&Arc<S>` does not coerce to `&dyn T` unless `Arc<S>: T` itself (standalone repro: `f(&arc)` with `impl T for S` is E0277, `required for the cast from &Arc<S> to &dyn T`). Result: `cargo check --all-features` has been red on riir-chain develop since 11-24. The mechanical was *lint-driven by clippy itself* — clippy fires `explicit_auto_deref` there because it models the deref, not the unsize target — so the class will RE-FIRE wherever a `&*x` feeds a `&dyn Trait` param through an `Arc`/`Box` smart pointer.
2. **The unused-variable family dropped `let status = ` from a `loop { break s }`** (`crates/riir-chaind/tests/e2e_crash_recovery.rs:268`) feeding the file's own shutdown-canary assertions — E0425 in the e2e test binary. The test's doc comment says those assertions ARE the fix ("The one-line fix is not what keeps it fixed. This is."); the mechanical lobotomized the guard.

## Why the compile gate certified both as green

- Casualty 1's call site is gated `chain_asset_fingerprinting` → compiled to NOTHING at the default-posture verify (the green-zero class). Casualty 2's test binary compiles only under the posture that arms chaind e2e features — never default.
- **The open question this issue owns: did the P20 cfg audit fire?** The verify's cfg audit (edits inside cfg-gated regions → `verified=false` down the recorder) shipped before 11-24. Either the 11-24 binary predates a working P20, the audit's cfg detection missed a mid-file `#[cfg]` on the module decl one hop up (`forensic/mod.rs` gates `attribution_agent` — the MODULE-DECL audit, issue-182 class), or it disclosed and the batch landed anyway. Re-run today's binary on the healed tree's pre-state (git checkout the 11-24 parents in a scratch worktree, replay `cargo refine fix` dry) and adjudicate which. If P20 works today, the residue is just the old binary; if not, the module-decl hop detection needs a fixture.

## Fix space (root cause, in priority order)

1. **Refuse the deref→unsize class in `explicit_auto_deref`'s fixer:** when the borrow's coercion target is a trait object (`&dyn …` — resolvable at the call site's parameter type) AND the receiver is a smart pointer (`Arc`/`Box`/`Rc`), the rewrite is compile-breaking → decline + record the divergence. Fallback posture if the target type is not resolvable parse-side: annotate `explicit_auto_deref` as verify-only (never `--fix`) for smart-pointer receivers.
2. **Fixture regression:** standalone crate with `impl T for S` + `f(&Arc::new(S))` + `f(&*arc)` — assert the fixer declines both rewrites.
3. **The P20/module-decl adjudication above** (may be a no-op if today's binary is clean).

## Provenance

- Fixed consumer-side: riir-chain `45cce96a` (this repo's heal landed first; the comment at the call site names the class).
- Standing rule honored: "wrong suggestion → fix the proposer at root cause; NEVER suppress, NEVER hand-edit past it without a fixture."
