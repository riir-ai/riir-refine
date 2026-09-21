# Plan 164: Post-Mining Intake Sweep — the three 09-21 healer-defect classes (proposer root-cause repairs)

**Status:** COMPLETE — all four repairs landed; GOAT green (lib 2152/0 · clippy -D ×2 states · score-bench 96.4% FLAT, row #155).

**Source:** `.distill/001_mining_queue_snapshot.md` §Post-mining intake (three INTAKE 2026-09-21
entries) + the riir-chain `b7e1ad8` / riir-ai `e4e17b4be` sweep sessions that recorded them.

**Discipline:** probe against real evidence FIRST (done — see per-class ground truth), fix the
proposer at root cause, fixture regression test per repair, re-run `--score-bench`, mark the
intake rows DONE with the commit hash. Proposer-only change — NO corpus delta ⇒ the gen2/gen3
freeze tables do NOT move (the plan-137 T7.5 law).

## Ground truth established (pre-implementation probes)

- **Class 1 (chunks_exact)**: live evidence riir-chain `src/consensus/curator_bridge.rs:110` —
  `let bytes_per_dim = 32 / effective_dim;` (a runtime let) reached `as_chunks::<bytes_per_dim>()`
  → E0435. The proposer's `Expr::Path(_)` arm accepts ANY path; the old test
  `variable_size_still_emits_but_is_verify_tier` pinned emit-and-let-VERIFY-revert.
- **Class 2 (excessive_precision/unreadable_literal)**: the five live constants in riir-ai
  `e4e17b4be` were **UNSUFFIXED** literals in an `[f32; N]` array inside `mod tests`. Our
  `excessive_precision` refuses unsuffixed by documented scope (type-blind f32/f64 — a
  wrong-width rewrite CHANGES the f64 value; the refusal is correct and stays). The intake's
  "exponent-mismatch guard" hypothesis is DISPROVED by direct probe: `{:?}` on
  `5.960464477539063e-08f32` prints `5.9604645e-8` (exponent form KEPT — the suffixed e-notation
  path already fires; existing test `exponent_form_shortens_in_place`). The REAL defect is the
  cross-fire: `unreadable_literal` regrouped `1.0009765625` → `1.000_976_562_5` (digits unchanged)
  on a literal clippy's excessive_precision flags — masking, not fixing.
- **Class 3 (orphan-report wire_const)**: live evidence riir-kat — all in-repo construction
  sites for `ClaimRequest.queued_unix` + `ShopSeedItemWire.{price_micro, edition_cap, nft_track}`
  are `#[cfg(test)]` fixtures with literal constants, while the PRODUCTION constructors
  (`riir-clippy/src/bin/heal.rs`, dapps routes) live outside the scanned set. The class's premise
  ("serialized-but-constant = schema-dead weight") is unsupported by test-only evidence.
- **Class 2b (auto-oracle posture)**: recorded in the riir-chain intake — the auto-oracle spawns
  `cargo clippy --workspace --no-deps` WITHOUT `--all-targets`, so `mod tests` spans (where
  fixtures live — the excessive_precision five were exactly there) are never anchored. A knob is
  not a repair ("a flag nobody passes is not a repair") ⇒ `--all-targets` becomes the DEFAULT
  posture with a kill-switch.

## Tasks

- [x] T1 — `chunks_exact_to_as_chunks` const-arg guard: pre-pass collects runtime binding
      idents (every `PatIdent`) + file-local `static` idents (statics are not valid const args);
      a SINGLE-segment Path size arg refuses when it names either. Multi-segment paths
      (`Foo::N`) can never be locals ⇒ keep emitting (VERIFY tier). Update the pinned test
      (runtime size now REFUSES); add fixtures: runtime-let (the live shape), fn-param,
      const-generic param fires, qualified path fires, file-local static refuses.
- [x] T2 — `unreadable_literal` cross-fire guard: expose `excessive_precision`'s decision
      (`shortest_round_trip`) as `pub(crate)` and SKIP the regroup when that lint owns the
      literal — suffixed: at its width; unsuffixed under a DIRECT `let _: f32/f64` (or
      `[f32; N]` array) annotation: at that width; array-unification via the first SUFFIXED
      element (the live `[0.0f32, …]` shape); plain unsuffixed: the f64 inference default
      (Rust's own default — keeps every probed firing fixture green). The existing f64-annotation
      fixture (`3.14159265358979`) stays firing (verified: f64-shortest not shorter).
- [x] T3 — auto-oracle `--all-targets` default + kill-switch
      `RIIR_HEAL_AUTO_ORACLE_NO_ALL_TARGETS=1` (restores the lib-only posture); status lines
      name the knob; knob-parse tests mirror the existing idiom.
- [x] T4 — `orphan_report` `wire_const` test-mod masking: `SiteEntry.in_test` bit (attrs
      `cfg(test)` + `#[test]` fns + tests/-path files — the config_audit helpers, consumed
      not copied); zero NON-test sites ⇒ render NOTHING for the (ty, field); classification
      runs on non-test sites only. Fixtures: the riir-kat live shape (test-only const sites ⇒
      no row), test-const + prod-var ⇒ cleared, test-var + prod-const ⇒ wire_const.
- [x] T5 — GOAT: `cargo test --lib` default **2152/0** (+15 fixtures); clippy `-D warnings`
      default lib + `--all-features --all-targets` both clean (the healer took its own 7
      doc_markdown mechanicals + 1 shrink-to-fit en route — compile-gated);
      `--score-bench` **96.4% FLAT** (row #155 appended; the 2 quantile_balance_router drift
      warnings are the known pre-existing pair); module-doc updates on all four surfaces;
      NO corpus delta ⇒ freeze tables unmoved (the plan-137 T7.5 law).
- [x] T6 — docs: mark the three intake rows DONE with the commit hash in
      `.distill/001_mining_queue_snapshot.md`; HISTORY.md entry; commit + push.
