# Issue 182 — the healer's verify is blind to module-decl gating: a `kat_migrate`-gated file healed to a green zero, `--all-features` broke E0599

**Status:** OPEN — filed 2026-10-27 (zed-m3 session, measured during the
dapps MIN_REDACT_VERSION 5→6 coordinated bump; the bad edits REVERTED in the
same session, no bad state shipped).

## What happened (measured)

1. `cargo refine --fix src/kat/migrate_sweep.rs` (riir-dapps, invoked at
   DEFAULT features, no `--verify-args`) — the clippy surface reported
   `unnecessary_get_then_check` ×4; the healer applied
   `self.store.get(&k).is_some()` → `self.store.contains_key(&k)` and
   reported **"fixed 1 file(s) / 4 edit(s) (verified — compile-gated)"**.
2. The verify was a GREEN ZERO. `pub mod migrate_sweep;` is
   `#[cfg(feature = "kat_migrate")]`-gated — at its DECLARATION SITE in
   `src/kat/mod.rs:327`, a DIFFERENT file. At default features the module
   compiles to nothing, so the verify compiled none of the edits.
3. The consumer's own `--all-features` gate broke:
   `error[E0599]: no method named 'contains_key' found for struct
   'riir_neuron_db::LocalKvStore'` ×3 — `LocalKvStore` has no
   `contains_key` anywhere in riir-neuron-db (the only hits are unrelated
   `BTreeSet` calls in kg_triple tests). `get(&k).is_some()` was correct.
4. Manual revert (all four edits), the file's two PRE-EXISTING
   `needless_borrow` sites fixed by hand; final tree clippy-clean and
   test-green at BOTH postures. No bad state committed.

## Root cause

The P20 post-verify cfg audit ("kept edits inside cfg-gated regions it
never compiled carry a per-edit `cfg audit` tail naming the exact
`--verify-args` re-run") is FILE-LOCAL: it scans the edited file's
internals for cfg regions. Module-level gating lives on the `mod`
declaration in the PARENT file — one hop away — so a whole module compiled
out at the verify posture reads as an ordinary ungated file and the verify's
green is taken at face value. P20 closed "the edit site is inside a gated
region of the file I edited"; the surviving class is "the FILE ITSELF is a
gated module of its parent". Same family as P43 (required-features rows
skip targets silently), one layer down: the target compiled, its content
did not.

The corpus rule is NOT the defect — `unnecessary_get_then_check` is a fine
rewrite when the receiver has `contains_key`; the healer cannot know the
receiver type and the verify exists precisely to arbitrate that. The defect
is that the arbiter was blind at the one posture that mattered, and nothing
said so.

## The fix space (for the implementing lane)

- **(a) The load-bearing half — module-decl lookup:** when the edited file
  is (or lives under) a module whose `mod`/`inline-mod` declaration in the
  parent file carries a `#[cfg(...)]`/`#![cfg(...)]`, resolve that gate
  against the verify posture; unsatisfied ⇒ the P20 tail MUST print (naming
  the exact `--verify-args` re-run), or `--fix` refuses without
  `--verify-args` (the stronger arm — a verify that provably compiled
  nothing should not be marketed as compile-gated). One hop covers the
  measured class (`src/kat/mod.rs` gating `src/kat/migrate_sweep.rs`);
  the walk can go transitively, but the measured case is one hop.
- **(b) Trajectory honesty:** the self-evolve record for this heal logs a
  verified fix; it was a green zero. When (a) lands, backfill the class so
  verify-verdicts carry the posture they compiled (the same disclosure the
  AGENTS.md full-gate section demands of every other lane).
- **(c) Non-goal:** teaching the corpus rules type inference. The verify
  posture is the arbitration layer; rules stay type-blind by design.

## Reproduce

```sh
cd riir-dapps   # at 428a2f76+ (pre-bump tree carries the same shape)
git stash; git checkout <pre-fix migrate_sweep.rs>  # restore get().is_some()
cargo refine --fix src/kat/migrate_sweep.rs             # green, no cfg tail
cargo clippy --all-targets --all-features               # E0599 ×3
```

Consumer-side lesson recorded: a heal of a file you have not personally
posture-audited deserves `--verify-args "--features <set>"` when ANY
ancestor mod is gated — and the consumer's `--all-features` gate stays the
arbiter of record either way (it is what caught this).

Session: zed-m3-refine179-hygiene, 1793100000
