# Research 177 — Batch 150: the staged default-level + pedantic probe pack

**Status:** RECORD (batch landed — corpus 56 → 62, FIXABLE 41 → 45, VERIFY +2)

## Provenance

The staged pack from `.distill/001` (accumulation satisfied by Bench 094's
default-eco histogram — the "second input" rule): 3 pedantic candidates
(`unreadable_literal`, `cast_lossless`, `redundant_closure_for_method_calls`)
+ 5 default-level candidates (`write_with_newline`, `doc_lazy_continuation`,
`doc_overindented_list_items`, `partialeq_to_none`,
`legacy_numeric_constants`). Each was probed against real clippy FIRST
(P10 discipline: a rule without probe evidence does not land), suggestion
shapes + applicability captured from `cargo clippy --message-format=json`
at the pinned toolchain (rustc 1.98.1 / clippy 0.1.98 — the same toolchain
the coverage report derives with).

## Probe matrix (the scratch crate: `.raw/probe150/`, deleted at landing)

### Default level (plain `cargo clippy`)

| lint | FIRES | SILENT | suggestion |
|---|---|---|---|
| `print_with_newline` | `print!`/`eprint!` literal ending `\n` (with/without format args, inline `{x}` args); `print!("\n")` | `println!`/`eprintln!` (nothing to suggest); interior `\n`; raw strings; `"\\n"` endings | 2× MachineApplicable: macro name + `\n` strip |
| `write_with_newline` | `write!(f, "…\n")`; `write!(f, "\n")` | `writeln!` (nothing to suggest) | 2× MachineApplicable |
| `partialeq_to_none` | `x == None`, `x != None` (→ is_some), `r == &None`, `x == Option::None` | `x.is_none()`; custom `PartialEq<Option>` type (type-aware) | 1× MachineApplicable |
| `legacy_numeric_constants` | `std::T::CONST` paths (int MAX/MIN; float EPSILON/INFINITY/NEG_INFINITY/NAN/MIN_POSITIVE), `::std::`-leading, `use std::u32::MAX as X;` | modern `u32::MAX`; `std::f64::MAX` (no assoc-const twin) | 1× **MaybeIncorrect** (shadow hazard) |
| `doc_lazy_continuation` | table/quote case: `>= 1` row reads as blockquote, the `< 1` row is its lazy continuation → "doc quote line without `>` marker"; list case: unindented line after an item | blank-line-separated paragraphs (the obvious shape does NOT fire) | quote case: MachineApplicable `> ` insert; list case: **MaybeIncorrect** indent insert |
| `doc_overindented_list_items` | continuation line indented >2 past marker (6+ spaces) | correct 2-space continuation; blank-separated paragraphs | **MaybeIncorrect** (dedent to 2) |

### Pedantic (`-W clippy::pedantic`)

| lint | FIRES | SILENT | suggestion |
|---|---|---|---|
| `unreadable_literal` | ≥6-digit ungrouped decimal int/float (`999999`, `123456.789`, `1234567890123_i64`) | ≤5 digits (`10000` silent — the boundary is digit COUNT, not value); any `_`; radix/exponent | grouping rewrite, **MaybeIncorrect** (clippy's conservatism; grouping is value-preserving by construction, so the proposer doesn't care) |
| `cast_lossless` | every primitive widening cast incl. the sign change (`u32 as i64` — `From<u32> for i64` exists) and `f32 as f64` | narrowing; same-type (`unnecessary_cast`'s territory) | 1× MachineApplicable whole-expr |
| `redundant_closure_for_method_calls` | `|s| s.len()` on `&String` (→ `std::string::String::len` FULL PATH); `|s| s.clone()` → `std::clone::Clone::clone` | `|x| x.pow(2)` (by-value self → path arity mismatch); `&&String` deref chain; `to_string` (ToString blanket); annotated closures | MachineApplicable full path |

## Adjudications

1. **`print_with_newline` + `write_with_newline` — FIXABLE.** One proposer
   module (`print_write_with_newline.rs`), two dispatch names. The
   full-literal form produces the cascade TERMINAL directly
   (`print!("\n")` → `println!()`, deleting the literal token + its
   preceding comma — never the re-firing `println!("")`).
2. **`unreadable_literal` — FIXABLE.** Digit grouping needs no type
   knowledge — the only parse-only-PROVABLE pedantic row in the staged
   head, as the staging note predicted. Clippy's MaybeIncorrect does not
   matter: regrouping decimal digits cannot change a literal's value.
3. **`legacy_numeric_constants` — FIXABLE with a SHADOW GUARD.** Clippy
   marks its own suggestion MaybeIncorrect because path resolution
   prefers an in-scope ITEM named like a primitive (`mod u32 {}`,
   `struct u32;`, `use weird::u32;`, `use foo::bar as u32;`) — the
   rewrite would compile with a DIFFERENT meaning, which the compile
   gate cannot arbitrate. The proposer refuses such files wholesale
   (`shadows_primitive`), conservatively allowing only `use std::…`
   re-openings (semantics preserved by construction). The const table
   only names legacy constants that have assoc-const twins.
4. **`partialeq_to_none` — VERIFY.** Type-blind rewrite; a non-Option
   receiver with a custom `PartialEq<Option>` impl has no `is_none()`
   (E0599) — check-visible, the `--fix --verify` gate auto-reverts. This
   is the VERIFY tier's exact design (the `redundant_closure` demotion
   precedent). Clippy's MachineApplicable suggestion means the P21 oracle
   lane also consumes the shapes the receiver-shape guard refuses.
5. **`cast_lossless` — VERIFY via the SAFE-LET gate.** The operand type is
   only followed from an EXPLICIT annotation (unique `let x: T = …`, or a
   uniquely-named fn param); the rewrite emits only on std's primitive
   `From` table (explicit matrix). Any second `let` of the same name
   refuses (shadow). All other operands (fields, generics, complex exprs)
   are the oracle lane's residue; a wrong fire is E0277 — check-visible.
6. **`redundant_closure_for_method_calls` — DECLINED (no bounded proposer
   path).** The fix needs a typeck-RESOLVED full path
   (`std::string::String::len`); parse-only cannot name the receiver type,
   and guessing paths is unbounded. The staging note's E0631 adjacency
   worry resolved POSITIVELY for clippy itself — real clippy is SILENT on
   exactly the danger shapes (by-value self, deref chains) — but that
   clippy-side safety does not give US a fix space. Oracle-lane residue
   only; the corpus-lockstep gate (every table name needs a proposer +
   entry) correctly forbids a proposer-less VERIFY row.
7. **`doc_lazy_continuation` + `doc_overindented_list_items` — STAGED
   follow-up (probe complete, authoring unblocked, NOT landed this
   batch).** The probe matrix above is the design input: the lint's real
   trigger is a LINE-STATE machine over doc-comment lines (list item /
   blockquote-marker tracking + lazy-continuation detection), the same
   complexity class as the doc_markdown and manual_range_patterns ports —
   a full batch of its own on the next quiet window. Oracle coverage
   would be partial (only the quote case is MachineApplicable), so a
   proposer port is the only full route.

## Generator lessons (for the next batch)

- **A bare top-level `let` is not a Rust FILE.** Every corpus fixture and
  inline test snippet must be fn-wrapped or `syn::parse_file` refuses it
  before the proposer ever runs — three fixtures + six test cases failed
  on exactly this.
- **Numeric suffixes contain DIGITS** (`i64`, `u128`): a suffix scanner
  must walk the trailing ALPHANUMERIC run and find its first ALPHABETIC
  character — a plain "trailing alphabetic run" scan stops at the digit
  inside the suffix and mis-splits `1234567890123_i64`.
- **Cargo's test-binary cache can serve a STALE suite after a failed
  compile** — a 1694-test run that predates the new modules. `--list`
  grep for the new test names is the cheap staleness check.
- clippy-driver rejects `--message-format` AFTER `--`; the cargo-clippy
  flag goes before it (`cargo clippy --message-format=json -- -W …`).

## Gates at landing

- lib default **1711/0** (incl. `all_buggy_codes_fire_their_lint`
  lockstep, `corpus_size_is_62`, `FIXABLE_LINTS.len() == 45`, the
  62-row `all_lints_fire_and_parse` table, the 6 new fixture sets with
  fire + silence shapes per P11)
- latent_retrieval **1863/0** incl. the 14 retrieval-precision gates
- all-features `cargo check` green (1m24s)
- clippy `--all-targets` **0 warnings** on the write set (cfg(test)-only
  findings hand-fixed — the healer's `--verify` gate compiles without
  `--tests` and would have flagged them unverified)
- score-bench **96.4%** (was 96.2% — slight improvement, no regression;
  rows #125-126 appended)
- `--corpus-stats` clippy_lints **62** (live dogfood)
- `--coverage-report`: covered 56 → 62, fix-set-only stays 0, the
  default-firing evidenced-uncovered set 74 → 71 (−3 = exactly
  write_with_newline, partialeq_to_none, legacy_numeric_constants — the
  three evidenced lints this batch covered)

## Riders landed in the same commit

- `corpus_coverage/tests.rs`: the Bench 094 test
  `the_three_source_precedence_is_documented_not_accidental` was landed
  WITHOUT its `#[test]` attr — dead code that never ran, and the moment
  it ran it FAILED (`def.rank()` asserted 8 against a `FiredDefault(1)`
  binding). Fixed (attr + the assertion's true intent: rank mirrors the
  binding's own count). An uninvoked assertion is *unknown*, not passing —
  this is the katgpt-rs Issue-734-adjacent lesson landing in this repo.
