# Research 391 — P120 rulewright-miss trio disposition (let_unit_value authoring + the two slice-math lanes verified)

**Status:** LANDED — Batch 347, carried by this landing's commit; corpus 117→118, fixable 96; verdict below.

## The intake

`.distill/001_mining_queue_snapshot.md` P120 (surfaced 2026-11-08 by the riir-grams wire-v3 GOAT lane, plan-513 T6): three clippy findings the fix lane mapped to 0 edits, each fixed manually + compile-gated — (1) `chunks_exact_to_as_chunks` + (2) `manual_is_multiple_of` on the b64 mask-bit math, (3) `let_unit_value` on the new `wire_goat` bench (`let _ = sealed.verify().expect(..)`, found only at the `--benches` posture). Remedy as written: "author/verify the three proposer rules; probe against real clippy per the intake discipline before any landing."

## The probe (pre-claim, /tmp/p120_probe, clippy 1.99.0)

- **(3) let_unit_value fires** on `let _ = sealed.verify().expect(..)` (the intake's exact shape), on unit std macro calls (`println!`/`assert!`/`assert_eq!`), and on underscore-NAMED patterns (`let _y = <unit-expr>` — referenceable). It stays SILENT on pure unit literals (`let _ = ();`, `let x = ();`). Diagnostics are HELP-ONLY: `sugg: null` on every child — the "omit the let binding" text carries NO machine-applicable replacement, so the P21 oracle lane cannot heal the class. The corpus proposer is the only fix path — the intake's "fixable half" framing CONFIRMED.
- **(1)(2) fire as documented** on the b64 shapes (`for chunk in bytes.chunks_exact(3)`, `bits % 8 == 0`), suggestions help-only → corpus-proposer path. Both proposers ALREADY EXIST (VERIFY-tier, registered in lint_level + corpus + dispatch + fix lists since the P7/Batch-91 era, probed vs clippy 1.97/1.98) — the intake's authoring half is STALE, the P116 precedent ("the needless_borrow half was STALE — the rule already existed").

## The root cause of the lane's 0-edits

The oracle set is `FIXABLE_LINTS` + (`VERIFY_LINTS` only under `--verify`, `src/bin/heal.rs` lint_set assembly). A plain `--fix` run without `--verify` excludes BOTH slice-math lints from the oracle set AND from the fix dispatch — the findings surface as `[no corpus rule]`-class unmapped spans exactly as the lane recorded ("the clippy class not mapped"). This is the DOCUMENTED VERIFY-tier behavior (type-blind rewrites ride the Issue-015 compile gate; the E0277/E0689 classes the plain check gate misses), not a bug. The correct lane posture for riir-grams-shaped targets is `cargo refine fix --verify`.

Empirical confirmation: the current binary's `fix --verify` run on the probe crate produced the correct compile-gated edits on the exact shapes — `bytes.as_chunks::<3>().0`, `flat.as_chunks::<2>().0`, `bits.is_multiple_of(8)`, `bytes.len().is_multiple_of(4)`. No proposer change needed for (1)(2); verified-existing, fixtures already standing (the mod.rs pseudocode triples + the proposer unit tests).

## The authoring — `let_unit_value.rs` (FIXABLE, Batch 347)

Shape: `let <pat> (: Ty)? = <unit-expr>;` → `<unit-expr>;` — the rewrite DELETES `let <pat> (: Ty)? =` and keeps the initializer verbatim (zero resampling of the expression's bytes; the original trailing `;` survives in place).

The unit-proof (parse-only; every arm's type is fixed by std or by the file-local definition):
- **Unit std macro allowlist** — `println`/`print`/`eprintln`/`eprint`/`assert`/`assert_eq`/`assert_ne`/`panic`/`unreachable`/`todo`/`unimplemented` (fixed expansion types, never must_use). NOT `writeln!`/`write!` — io::Result, the must_use-suppression class the compile gate CANNOT see (warnings are not errors; the gate would not revert it).
- **Unit-tail blocks** — `ExprBlock`/`ExprUnsafe` with no tail expr (the last stmt is `;`-terminated or the block is empty); `if` WITHOUT else (the branch type is `()` by law); `loop`/`while`/`for` (`!` or `()`, never must_use).
- **File-local unit fns** — unqualified single-segment calls resolving to the ONE file-wide fn def of that name, unit-returning (no arrow or `-> ()`), shadow-checked against every binding (PatIdent pre-pass), every `macro_rules!`, and every `use`-imported leaf name (globs refuse conservatively). Scope resolution: the def's scope path must be an ancestor-or-equal of the call site's item-chain (module-boundary prefix; the file root is every scope's ancestor — a call inside `fn probe`'s body resolves the file-level def; a def inside `mod m` does not resolve a root call). Duplicate defs (cfg alternatives) refuse.

Gates:
- **Pattern gate**: the wildcard `_` (syn parses `let _ = …` as `Pat::Wild` — verified, NOT `Pat::Ident("_")`) is unreferenceable by language law → fires free. NAMED patterns (including `_y`) require ZERO ident occurrences of the name after the initializer (one file-wide ident-positions walk in the pre-pass; conservative — any occurrence class refuses). Non-ident patterns (tuple/struct destructures) refuse: a unit value cannot destructure.
- **Refusals**: attributed locals (the cfg class), let-else (`diverge` arm — cannot coexist with an irrefutable pattern anyway), method chains / foreign fns / `?` / `.await` initializers (unprovable — the documented divergence; the intake's own bench shape `sealed.verify().expect(..)` is a method chain and stays manual), `if`-with-else, unit literals (clippy itself is silent there — no corpus value), interior comments in the removed span (the displacement guard).

Measured en-route (the probe-fix run's own disclosure): the perf domain's `shrink-to-fit-after-build` rule proposed a `shrink_to_fit()` on a Vec RETURNED from the probe fn — a false-positive on return-path shapes; recorded here for the perf lane's intake, NOT fixed this batch (different proposer, not in scope).

## Gates (the batch record)

- 26/26 proposer unit tests (fire arms ×10 + refusal arms ×16); lib default 3809/0; lib latent_retrieval 3949/0; the `fixable_pseudocode` triple row; clippy -D default + latent_retrieval + all-targets: zero on the delta; `all_buggy_codes_fire_their_lint` green.
- Retrieval floors HELD: 29/0 at the default posture + `retrieval_precision_regression_gate` green — no re-pin.
- Freeze regen (T7.5, owed on the clippy delta): `generation_v2_freeze` + `generation_v3_freeze` e2e both PASS with real clippy (39 s / 15 s); the new `let_unit_value` atom's specs printed clean (0 drift, 0 newly excluded — no exclusion-table rows owed, unlike the b62decef doc-lint pair).
- Score-bench appended (gen-1): heal_rate 0.9643 (prior 0.9618), parse_safety 1.0, decline_correctness 1.0 — no regression; gen-1 SATURATED (marginal +0.0006).
- `pins_manifest --write` (clippy_lints 118); the pin web: health_views (clippy 118, total 1604), AGENTS §Role, README domain row + corpus-tree line, mod.rs Scope lineage, corpus.rs size lineage.

## Divergences vs real clippy (documented, deliberate)

| Shape | clippy 1.99 | this proposer |
|---|---|---|
| `let _ = <unit method-chain>` | FIRES | silent (typeck-only) — the manual/`cargo clippy --fix` path |
| `let _ = writeln!(..)` (io::Result) | silent (must_use binding is intentional) | silent (refused — the must_use class) |
| `let _ = ()` / `let x = ()` | silent | silent (no corpus value) |
| `let _y = <unit-expr>` | FIRES | fires (named gate: zero later references) |
| unit std macros / unit-tail blocks / if-no-else / loops | FIRES | fires (provably-unit arms) |
| unique file-local unit fn call | FIRES | fires (uniqueness + shadow + scope-resolution proof) |
