# Research 104 — Batch 88: sec Phase 2 (unsafe hygiene + crypto hygiene)

**Status:** RECORD (batch landed 2026-08-29; Plan 018 T2.1 complete)

## Provenance

- **Source:** Plan 018 T2.1's candidate list (the Phase-2 gate opened when
  Phase 1 landed 6/6 — Bench 046). Owner-directed execution 2026-08-29
  (the same owner call that promoted `rustc_errors`).
- **Probe protocol:** every candidate shape probed against real clippy
  **1.97.0** BEFORE authoring (Proposal 004 caveat 4 — the boundary is
  re-measured per batch, never inherited). Probe crate: default +
  pedantic + restriction + nursery + suspicious + correctness + perf +
  style groups, JSON diagnostics parsed for code + machine-applicability.

## Probe verdict table (clippy 1.97.0)

| Candidate | Clippy firings on the buggy form | Verdict |
|---|---|---|
| `unsafe impl Sync` for a type w/ `RefCell<Rc<str>>` field | `undocumented_unsafe_blocks` only (restriction; wants a COMMENT, not a redesign — no equivalent fix) | OPEN — genuine Rudra `unsafe-impl-send-sync` territory |
| `#[no_mangle] pub extern "C" fn` (VALUE-return) w/ indexing panic | `indexing_slicing` only (restriction, no fix) | OPEN — Phase-1 `ffi-extern-fn-catch-unwind` owns UNIT-return only |
| `Sha256::digest` / `Sha1::digest` / `Md5::digest` | nothing hash-related (only `redundant_import`-class style noise) | OPEN — clippy has NO weak-hash lint; cargo-audit owns vulnerable CRATES, not misuse |
| `fn hash_password` w/ bare `Sha256::digest` | silent | OPEN |
| `a == b` on secret-named slices | silent (even under restriction) | OPEN |

Clean shapes (blake3 / `ct_eq` already used) drew nothing relevant — no
false-positive pressure on the good side.

## Rules landed (corpus 6 → 11)

1. **`unsafe-impl-send-sync-unsound-field`** (`UnsafeHygiene`, suggest-only) —
   the fix is a type-level Arc/Mutex redesign; every constructor/usage site
   ripples (the `rc-backedge-field-prefer-weak` refusal class). Corpus fix
   shape shows the redesign; the rule is detection + guidance.
2. **`ffi-export-panicking-body`** (`Ffi`, suggest-only) — the VALUE-return
   complement of Phase-1's catch-unwind: a panic here needs a caller-chosen
   sentinel value, a semantic decision, never mechanical.
3. **`weak-hash-to-blake3`** (`CryptoHygiene`, suggest-only, dep-gated) —
   MD5/SHA1 broken + SHA256 the house loser → blake3. Output TYPE changes
   (`[u8; N]` → `blake3::Hash`) — signature-changing, no proposer.
4. **`password-hash-argon2`** (`CryptoHygiene`, suggest-only, dep-gated) —
   fn named `*password*` + bare digest → argon2 (memory-hard KDF). Name-
   based trigger: false-NEGATIVES honest and expected (parse-only cannot
   see data flow).
5. **`constant-time-secret-compare`** (`CryptoHygiene`, **proposer**) —
   `a == b` → `a.ct_eq(b).into()`. Signature-preserving (bool → bool).
   Gates: file imports `subtle` (the fxhashmap dep-gate — any
   `use ... subtle ...` form); the **enclosing fn's name OR an operand
   name** hits the secret vocabulary (`secret|token|mac|signature|sig|
   key|hmac|nonce|tag|digest`, `_`-segment word-boundary); both operands
   plain path idents; `#[cfg(test)]` mods skipped; `!=` out of scope.

## The en-route finds (G1 doing its job)

- **Corpus-lockstep gate caught the R5 fixture**: `every_fixable_rule_fires_
  on_its_fixture` FAILED on the first run — the corpus `buggy_code` lacked
  the `use subtle::ConstantTimeEq;` import, and the dep gate (correctly)
  refused to fire. The fixture now carries the import: the honest real-
  world shape is "subtle is already a dep and imported, but THIS comparison
  missed it".
- **The fn-name signal was designed in, not assumed**: the first fixture
  (`check_token(provided, expected)`) has operands WITHOUT secret names —
  the vocabulary signal had to include the ENCLOSING FN name (a walker
  stack over `ItemFn`/`ImplItemFn` idents). Real code names fns
  `verify_token`/`check_mac` with generic operands; operand-name-only
  detection would have been near-useless.
- **The corpus kebab-case gate rejected digits** (`password-hash-argon2`);
  relaxed to allow ASCII digits (the rust_perf convention —
  `usize-index-to-u32` exists there).
- **The boundary-gate probe crate needed the crypto deps**: fixtures that
  import sha1/sha2/blake3/argon2/subtle fail to COMPILE in the bare probe
  crate, and compile errors are not clippy diagnostics — the scaffold
  manifest now declares them. Two new pinned allowed overlaps
  (`indexing_slicing` on the FFI-panic fixture, `undocumented_unsafe_blocks`
  on the unsound-impl fixture — both fire with NO machine fix, so our
  fixes stay additive).

## Honest caveats

- **R5 is shape-based, not type-aware**: a `String`-typed session token
  FIRES (name + plain-ident gates pass) and then `--fix --verify`
  auto-reverts the E0599 (subtle has no `ConstantTimeEq` for `String`).
  Pinned by fixture `string_operands_fire_but_compile_gate_owns_them` —
  the compile gate is the type backstop; every revert is review noise by
  design (never-wrong outranks never-noise).
- **R4's trigger is name-based**: a weak password hash inside a fn NOT
  named `*password*` is a false negative (expected, documented). A fn
  named `hash_password` that hashes non-passwords is a false positive —
  the concept text carries the reasoning; the review line names the rule.
- **R1's field-type match is textual** (segment match for `Rc`/`RefCell`/
  `Cell` in same-file struct defs). Aliased imports (`use std::rc::Rc as
  Alias`) are missed (false negative, safe). Generic impls are skipped
  (unprovable).

## GOAT (G1–G4)

- **G1**: 10 R5 proposer fixtures (fire ×3 incl. aliased import + fn-name
  signal; silent ×6: no-import, non-secret names, non-plain operands,
  `!=`, test mod, word-boundary) + corpus-lockstep green + boundary gate
  green with the 3 pinned allowed overlaps (1 Phase-1 + 2 Phase-2).
- **G2**: rides the shared index+scratch suggest path; the R5 proposer is
  file-local syn visiting, same class as the Phase-1 proposers.
- **G3**: sec 939/0 · sec+kernel+latent 1267/0/2i · all-features
  1599/0/15i · clippy `--all-targets` 0 in sec and sec+kernel+latent
  states · retrieval floors 14/14 HELD (no re-pins) + sec self-variants
  still top-1 with 11 rules.
- **G4**: dep-tree unchanged (`cargo tree` diff empty — syn-only, the
  `rust_perf` gating pattern). E2E: `--fix --write --verify --domain sec`
  on a scratch victim healed both secret compares compile-gated and left
  the non-secret `a == b` untouched.
