# Research 392 — Batch 348: the P111 approx_constant + P112 useless_vec corpus gaps (plan 359)

**Status:** RECORD (the batch's own research note — landed with the batch)

## Intake provenance

- **P111** (2026-10-31, the riir-refine idle sweep's oracle-anchored lane on
  `riir-shader/crates/riir-shader-effects/src/clipping.rs:62`): `const T: f64 =
  1.618033988749895;` — the golden-ratio icosphere literal. Fixed manually at
  the time (`std::f64::consts::GOLDEN_RATIO`, to_bits-identical) for want of a
  rule. P94 (2026-10-28) had already probed the lint's shape; this batch
  re-verified the probe at the current driver.
- **P112** (2026-11-01, the seal-render clippy sweep, session
  zed-seal-continuation): `crates/seal-scene/src/mip.rs:121,125` — test-fixture
  `&vec![7_u8; N]` → `&[u8; N]`; fixed manually with `const` arrays. The row
  itself ordered the triage: "probe whether P21's verbatim-oracle lane already
  covers it before authoring" — the P120 half-stale lesson applied.

## Probe record (clippy 1.99.0, /tmp/b348_probe fixture matrix, removed at close)

### approx_constant

- **Deny-by-default**, fires in every literal position (const items, let
  bindings, call arguments) — 12 shapes in the probe matrix.
- **Help-only diagnostics**: zero suggestion children (applicability `None`,
  empty snippets) — the P21 verbatim oracle lane cannot heal the class. The
  corpus rule is the only fix path. (P94's finding, re-confirmed at 1.99.0.)
- Message format: `approximate value of \`f{32, 64}::consts::GOLDEN_RATIO\`
  found` (f64 → `f{32, 64}::consts::X`, f32 → `f32::consts::X`).
- **The table** (every entry verified to fire): E, EULER_GAMMA, FRAC_1_PI,
  FRAC_1_SQRT_2, FRAC_2_PI, FRAC_2_SQRT_PI, FRAC_PI_2, FRAC_PI_3, FRAC_PI_4,
  FRAC_PI_6, FRAC_PI_8, GOLDEN_RATIO, LN_10, LN_2, LOG10_2, LOG10_E, LOG2_10,
  LOG2_E, PI, SQRT_2, TAU — 21 constants. **SQRT_3 is deliberately ABSENT**:
  std carries it (`1.7320508075688772`) but clippy 1.99 does NOT fire it
  (probed) — a proposer const upstream is silent on would rewrite code the
  target crate's own clippy calls clean (the Issue-135 inversion).
- **Tolerance, not bit-identity**: the truncated `3.14159265358979` fires
  upstream (correcting the earlier "NEAR_MISS silent" misread of the first
  probe pass — the JSON parse's level filter had dropped error-level
  diagnostics; the truncated shape DID fire in every pass). Negated literals
  fire (`-0.6931471805599453` as LN_2). E-notation fires
  (`6.283185307179586e0` as TAU). Unsuffixed literals in untyped positions
  fire (typeck-inferred f64).

### useless_vec

- **Warn-by-default** (perf group), 2 shapes in the probe matrix:
  - `&vec![1, 2, 3]` at a call-argument position → help `you can use a slice
    directly: \`&[1, 2, 3]\`` — **MachineApplicable** (the run's own tail:
    "run \`cargo clippy --fix\` to apply"), the span covering the whole
    `&vec![…]` expression;
  - `&vec![0; 8]` (repeat with a LITERAL length) → `&[0; 8]`, same
    applicability.
- **Silent** on: `&vec![0; n]` (variable length — an array length must be
  const), the let-bound-then-borrowed shape (`let v = vec![..]; f(&v)` —
  typeck territory at 1.99), mutated vecs, Vec-typed returns, `Vec::new()`.
- **The P21 triage verdict** (the intake's own question): the suggestion is
  machine-applicable, but the oracle lane consumes suggestions only for lints
  the corpus CARRIES (the P51 corpus-rule precondition — the same
  let_unit_value finding). Authoring the corpus rule is what unlocks BOTH the
  oracle lane (fed diagnostics) and the standalone scan path. Not
  P21-covered — the gap is real.

## Authoring decisions

- **approx_constant — FIXABLE tier.** The fire surface is parse-provable by
  construction: a suffixed literal pins its own type (the value parses
  DIRECTLY at that type — `base10_parse::<f32>` reproduces rustc's rounding,
  no double rounding); an unsuffixed literal fires only inside a declared
  `: f64` annotation (const/static item or let pattern). Every rewrite is
  bit-identical by the match gate. Disclosed divergences (all
  upstream-fires-we-refuse): truncated values, negated literals, unsuffixed
  in untyped/f32-annotated positions (the double-rounding class), and the
  `no_std` residual (the emitted `std::f64::consts::NAME` path is
  std-dependent; the compile gate reverts there). Macro interiors never
  reach the visitor (syn does not parse macro token streams) — the macro-arg
  class is structurally out of reach, not a guard.
- **The table sources the std consts themselves**
  (`KnownConst { name, value: std::f64::consts::E }`) — never re-typed
  digits: a transcription slip would fail the bit pins, and the crate's own
  denied lints (`approx_constant`, `unreadable_literal`) would fire on our
  own table (measured: the digits draft tripped 21 approx_constant errors at
  `-D warnings`). The f32 arm derives `(value as f32).to_bits()` at match
  time; `table_matches_std_consts_bit_for_bit` pins EVERY entry's f64 bits
  AND derived f32 bits against the actual `std::fXX::consts` items (the
  double-rounding question is answered by the test, not assumed).
- **useless_vec — VERIFY tier** (the `ptr_arg` precedent: type-blind
  consumer — a callee taking `&Vec<T>` exactly makes the array form E0308;
  check-visible, the compile gate reverts). The rewrite splices the macro
  interior VERBATIM (elements, separators, comments, interior whitespace
  survive byte-identically — the interior is located by the FIRST `[` and
  the call's final `]`, so a `]` inside a string literal cannot derail the
  splice; the replacement covers only the `vec![…]` call — the caller's `&`
  survives in place). Guards: `&mut vec![..]` refuses; the repeat length
  must be one integer-literal token (underscore groupings and integer
  suffixes both valid); qualified paths (`std::vec![..]`) refuse.

## Freeze-table regeneration (the clippy_lints delta moved the seeded corpus)

- The two new atoms reshuffled every stride window past them: 68 relabel
  rows landed (36 V2 + 32 V3) — the k1 pair rows + every k2/k3/k4 composite
  the stride windows rebuilt. En-route: a first bulk-merge attempt mangled
  the multi-line rows (my rebuild dropped rows my one-line regex didn't
  match — caught by the coverage gate red on `gen2-k1-doc_overindented_list_items`,
  restored via `git checkout`, re-merged with a row-preserving parser). One
  label correction: the persist atom's expect lint (`persist_useless_format`
  → `useless_format`) — labels are the EXPECT lints, never the atom ids.
- Both freeze e2es re-ran GREEN with the landed rosters (v2 12.9s, v3 12.7s,
  real-clippy oracle, 0 drift); `freeze_status_holes_are_loud` ×2 green (the
  always-on coverage gate reads labels ∪ exclusions covering every spec, and
  each label equals the spec's union gold).

## Gates (all green)

- Proposer tests: approx_constant 15/15, useless_vec 12/12 (fire + refusal
  arms; the untouched-bytes invariant).
- Corpus round-trip: `all_buggy_codes_fire_their_lint` + `floor_corpus_integrity`
  (3/3) + `generated_corpus_spot` (2/2) — every corpus fixture fires its own
  proposer and re-parses.
- Real-clippy parity: the buggy fixtures fire (4 diagnostics in the parity
  probe), the healed forms are byte-clean.
- Lib: default 3836/0 · latent_retrieval 3976/0 (768s — includes the
  fixture-label cross-fire walk; the two new proposers cross-fire nothing) ·
  score_bench 4157/0.
- Clippy `-D warnings`: default / clippy_lints / score_bench / no-default /
  all-features — CLEAN.
- Retrieval floors: HELD at both postures (the latent run's gates green, no
  re-pin).
- Freeze: v2 PASS 12.9s + v3 PASS 12.7s + the two coverage gates.
- Score-bench gen-1 appended: heal 0.9643 / created 0.0179 / parse 1.0 —
  FLAT (the 0.9643 floor held; n_healable 31).
- pins_manifest --write (clippy_lints.rules 118→120, the landing's own
  diff); AGENTS.md + README.md count rows (120 = 97 fix + 20 verify + 3
  self_evolve-only, derived from source and confirmed by the split count);
  health_views pins (corpus line 118→120, the tree total 1604→1606).
- Release binary rebuild owed (the corpus compiles into the shipped binary)
  — executed at the landing HEAD (the `--version` build hash == HEAD check).
- dual_allocation gate: 0 twin / 0 independent / 0 counter.

## Yield

Corpus 118→120 (fixable 96→97, verify 19→20). Both intake gaps were real —
the probe-first discipline confirmed the P120 lesson's value on the OTHER
side too (the P21-unlocks note in P112's row was the correct reading, not a
stale premise).
