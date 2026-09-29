# Plan 188 — Batch 191: `mem_replace_with_default` (the coverage-report re-arm, pack XVIII)

**Status:** COMPLETE (2026-09-30 — Batch 191 LANDED; all gates green; two en-route gate catches both fixed at source: the lint_level driver-match refusal of my perf-group read [the driver says style] + the freeze e2e's latent-E0308 in the corpus atom [`sink.push(0)` into a `Vec<Vec<u32>>` sink] — see Research 223)

- [x] T0 — the fresh derivation + adjudication (DONE in-session, recorded here):
      `cargo heal --coverage-report` at clippy 0.1.98, corpus 99 → the
      frequency-ranked head fully adjudicated against the standing verdicts:
      the Research-191 skip list (doc-authoring / typeck / error-invention /
      architectural / type-blind), the B168 overflow-class REJECTS
      (`manual_div_ceil`, `int_plus_one`), the B185 MTA probe-REJECT, the
      B171 INERT (`useless_borrows_in_formatting`) + DUPLICATE (`len_zero`),
      the B184 type-blind refuse (`double_ended_iterator_last`), the
      wave-4 anecdote record (`obfuscated_if_else` 10/1 — recorded not
      staged), the B156 silent probe (`manual_strip`), the B185 structural
      DEFER (`derivable_impls` — semantic-eq + cross-item attr edit), and
      the receiver-collision refuses (`unwrap_or_default`, `search_is_some`,
      `get_first` — custom-method collisions, the `double_ended_iterator_last`
      class). **The one unadjudicated, bounded, eq-by-construction row:
      `mem_replace_with_default` (count 4, spread 3, perf, warn-by-default).**
- [x] T1 — the probe (`scripts/plan188_pack18_probe.py`, the plan163/179/180
      envelope): firing arms (`std::mem::replace(x, Default::default())` +
      the `use std::mem` imported spelling), silence arms (turbofish
      `<T as Default>::default()`, non-default second arg, 1/3-arg calls,
      local `mem` shadow), fixed forms (compile + silent), the
      suggestion-applicability dump (MachineApplicable expected — measured
      in-session on the smoke probe), and the co-fire census (the corpus
      atom must fire EXACTLY its target — the `let_and_return` tail-shape
      artifact found by the smoke probe is designed OUT of the atom).
- [x] T2 — author on PASS: proposer `proposers/mem_replace_with_default.rs`
      (syn visitor over `Expr::Call`; callee path `std::mem::replace` or
      `mem::replace` with the import verified + no local `mem` shadow — the
      `legacy_numeric_constants` shadow-guard pattern; exactly 2 args; the
      second EXACTLY the bare `Default::default()` path call; replacement
      `<path-prefix>take(<arg0>)` — clippy's own MachineApplicable
      whole-call shape) + the six registration surfaces (proposers/mod.rs:
      mod decl + FIXABLE_LINTS + dispatch + parse-gate fixture row;
      corpus.rs ClippyFixEntry; lint_level.rs group row) + module tests.
- [x] T3 — count pins walked: corpus.rs `corpus_size_is_82` 99→100; the
      FIXABLE_LINTS.len() 80→81 pin + the fixture-table doc line 92→93.
- [x] T4 — freeze regen (OWED — clippy corpus delta moves the stride
      windows, plan 137 T7.5): **v2 98 verified / 0 newly excluded / 19
      pre-excluded + v3 98 / 0 / 23, both green, 2-run converged**; the
      +2 delta is the atom's k1 rows in FROZEN_LABELS. The regen detour
      is the recorded lesson (Research 223): the broken-fixture run's
      exclusion row was pasted, then REMOVED once the banner showed the
      atom verifying — a pasted exclusion row SKIPS the spec, "0 newly
      excluded" after a paste is the skip talking, not a verify.
- [x] T5 — GOAT: G1 parse/syn + all_buggy_codes_fire_their_lint (7/7); G2 lib
      default **2398/0** at the new count; G3 clippy `-D` clean at default /
      no-default / all-features (1 doc_markdown backtick fix); retrieval
      **21/21 floors HELD** (202.1s, isolated target dir); score-bench
      **row #173, gen-1 heal 0.96363634 FLAT**, created 1.8%, the frontier
      snapshot appended at the bench tail.
- [x] T6 — docs: Research 223 (born-complete RECORD), AGENTS.md clippy
      corpus count 99→100 + the B191 clause, README.md row (the stale
      80/61 head counts repaired to 100/81 — they predate this batch),
      `.distill/001` queue row, global binary rebuilt (release set).
- [x] T7 — commit `feat: batch 191: mem_replace_with_default (mem::take
      definition identity)` + push; queue snapshot updated in the SAME
      commit.

## The rule (the identity, the guards, the divergence record)

`std::mem::replace(dst, Default::default())` → `std::mem::take(dst)` —
FIXABLE by STD'S OWN DEFINITION IDENTITY: `mem::take` is literally defined
as `pub fn take<T: Default>(dest: &mut T) -> T { replace(dest, T::default())
}` (the std source). The strongest equivalence class the corpus carries
(the `from_str_radix_10` / `bytes_count_to_len` construction-identity
precedents). The input's own compilation proves `T: Default` — zero
type-blindness.

Guards (the bounded fix-space):
- The callee path is `std::mem::replace` (3 segments) or `mem::replace`
  (2 segments, with `use std::mem` verified in the file and NO local `mem`
  item shadowing it — the `legacy_numeric_constants` shadow-guard class).
- Exactly 2 args; the second is EXACTLY the bare `Default::default()` path
  call (no args, no turbofish). The `<T as Default>::default()` turbofish
  spelling is SILENT upstream (smoke-probe measured) — parity refusal.
- No attributes on the call; no comments inside the replaced span.
- Bare `replace(...)` (via `use std::mem::replace`) refuses v1 — the
  one-import-fn ambiguity (a local `fn replace` collision); documented
  narrower-than-clippy divergence if upstream fires there (probe records).

## Evidence (the concentration law, honored)

default-eco histogram (Bench 094 t04, 663 crates, clippy 0.1.98):
count 4, crate_spread 3 — thin but REAL (not the single-crate anecdote
class: `obfuscated_if_else` 10/1, `upper_case_acronyms` 8/1). No local
Bench-061 row. The staging justification is the RULE CLASS (definition
identity, MachineApplicable, default-firing perf lint) — the count is
supporting, not primary. Recorded honestly in the Research note.

GROUP CORRECTION (measured, the gate's own catch): the lint is
**style**-group, not perf — the installed driver's derivation refused my
perf read at test time (`table_and_corpus_match_installed_driver`).
Additionally: the rule also unlocks the P21 oracle-edits lane for this
lint (corpus membership is that lane's gate — MachineApplicable
whole-call suggestion, probe-measured).
