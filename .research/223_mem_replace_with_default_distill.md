# Research 223 — Batch 191: `mem_replace_with_default` (the coverage-report re-arm, pack XVIII)

**Status:** RECORD (batch landed 2026-09-30, closed)

## Source

The standing coverage-report pipeline (Issue 119's `cargo heal
--coverage-report` — the derivation at clippy 0.1.98, corpus 99): the
fresh frequency-ranked head at the post-B185 state. Every row above the
lead carried a standing verdict (the Research-191 skip list, the
B168/B171/B184/B185 recorded rejects/refuses/defers, the wave-4 anecdote
record, or the receiver-collision class) — the adjudication is recorded
in [Plan 188](../.plans/188_mem_replace_with_default_take.md) T0.

## The rule

`clippy::mem_replace_with_default` (style-group, warn-by-default — the
installed driver's own derivation; my first read said perf and the
`lint_level` driver-match gate REFUSED it, caught at test time):
`std::mem::replace(dst, Default::default())` → `std::mem::take(dst)`.

**The identity class: std's own definition.** `mem::take` is literally
`pub fn take<T: Default>(dest: &mut T) -> T { replace(dest,
T::default()) }` — the strongest equivalence justification the corpus
carries (the `from_str_radix_10` / `bytes_count_to_len` construction-
identity precedents). The input's own compilation proves `T: Default` —
zero type-blindness.

## The probe (scripts/plan188_pack18_probe.py — the plan163 envelope)

- FIRING (3/3): `std::mem::replace(x, Default::default())` + the
  `use std::mem` imported spelling + the nested-in-larger-expr form —
  all fire, all whole-call `MachineApplicable` `std::mem::take(dst)`
  suggestions.
- SILENT (3/3): the `<T as Default>::default()` turbofish (parity
  refusal), a non-default second arg, take-already-present.
- POSTURE (recorded, never gated): a local `mod mem` shadow — upstream
  SILENT (typeck resolves it; our shadow-guard is parity); bare
  `replace(..)` via `use std::mem::replace` — upstream FIRES
  (`MachineApplicable`) but our v1 refuses (a syntax-only proposer
  cannot prove the one-name import — a local `fn replace` would make
  the rewrite broken code — the SAFER-than-clippy divergence class);
  `<_>::default()` — upstream FIRES, v1 refuses (the documented
  narrower-than-clippy divergence, the bounded-space call).
- FIXED forms (2/2): both path spellings rewritten to take() compile
  clean + stay silent.

## Evidence (the concentration law, honored)

default-eco histogram (Bench 094 t04): count 4, crate_spread 3 — thin
but real (not the single-crate anecdote class). No local Bench-061 row.
The staging justification is the RULE CLASS (definition identity +
`MachineApplicable` + default-firing), the count supporting — recorded
honestly. The rule also unlocks the P21 oracle-edits lane for this lint
(corpus membership is that lane's gate).

## The landing (corpus 99 → 100, FIXABLE 80 → 81)

- Proposer `proposers/mem_replace_with_default.rs` (13 module tests) +
  the six registration surfaces (mod decl / FIXABLE_LINTS / dispatch /
  parse-gate fixture / corpus entry / lint_level row).
- **Two en-route catches, both by the batch's own gates:**
  1. The `lint_level` driver-match gate refused my "perf" group read —
     the installed driver says **style** (the -W help visual grouping
     misled me; the gate's derivation is the authority).
  2. The **freeze e2e caught a latent E0308 in my corpus atom** — the
     liveness statement `sink.push(0)` pushed an integer into a
     `Vec<Vec<u32>>` sink (the B183 latent-E0308 class; my probe
     fixtures compiled, the corpus atom I wrote separately did not).
     Fixed at source (`sink.push(Vec::new())`); the atom then genuinely
     verified — the exclusion-row detour (pasted from the broken-fixture
     run, then REMOVED once the banner showed the atom verifying) is the
     recorded lesson: **a pasted exclusion row SKIPS the spec from
     verification — "0 newly excluded" after a paste is the skip
     talking, not a verify. Read the failure banner, paste, then REMOVE
     stale rows when the underlying defect is fixed at source.**
- Freeze regen (owed — clippy corpus delta): **v2 98 verified / 0 newly
  excluded / 19 pre-excluded + v3 98 / 0 / 23**, both green, converged
  (2-run byte-stable); the +2 table delta is the atom's k1 rows in
  FROZEN_LABELS (both lanes) — the healthy state.
- Gates: lib default **2398/0** (+13 over the 2385 baseline); clippy
  `-D` clean at default / no-default / all-features (1 doc_markdown
  backtick fix in the new file's doc); retrieval **21/21 floors HELD**
  (202.1s, isolated target dir); score-bench **row #173, gen-1 heal
  0.96363634 FLAT** (the frozen set is frozen), created 1.8%, the
  frontier snapshot appended at the bench tail (the 089 clock).
- Count pins walked: `corpus_size_is_82` 99→100, `FIXABLE_LINTS.len()`
  80→81, the 93-row fixture-table doc line.

## Queue after B191

Staging pool EMPTY again — the next pool: the coverage-report re-arm at
the next batch slot (the head's remainder is now fully skip-listed or
recorded — the next fresh derivation needs a clippy version move or new
evidence), item-11 hourly envelopes, walk #13 ~10-05, nnethercote
re-test ~10-01. Owner-gated set unchanged. NOTHING INVENTED.
