# Research 187 — Batch 160: the frontier-lead mining pack VI

**Status:** RECORD (batch landed 2026-09-18; Plan 151 — the Batch 159
re-arm's leads consumed, families graduated, frontier re-armed).

## Source

The Batch 159 re-arm's committed frontier leads (rows in
`.benchmarks/frontier_miner_rows.jsonl`, probe-verified by
`scripts/plan150_frontier_probe.py`): `from_str_radix_10` +
`assign_op_pattern` — both consumed as FIXABLE rules this batch, both
families graduated to the heldout loop, frontier re-armed with `unit_arg`
+ `string_lit_as_bytes` (probe-verified by
`scripts/plan151_frontier_probe.py`).

## The leads → rules

### `from_str_radix_10` — `u32::from_str_radix(s, 10)` → `s.parse::<u32>()`

**Construction identity**: std defines `impl FromStr for {every int
primitive}` as literally `from_str_radix(src, 10)` — `parse::<T>()` calls
THAT function through `FromStr::from_str` with the SAME `&str` input and
returns the SAME `Result<T, ParseIntError>`. The turbofish re-spells the
type the original call already named, making the rewrite
RESOLUTION-IDENTICAL — no NEW name resolution is introduced, so no shadow
guard is needed (unlike `legacy_numeric_constants`, which INTRODUCED the
bare primitive path and could compile under shadowing with a different
meaning; here both sides spell `TY` in type position, resolving the same
way under any shadow).

The receiver is arg0's text VERBATIM (paren-wrapped when not
postfix-safe): `from_str_radix`'s `&str` param accepted the receiver via
deref coercion, and method resolution reaches `str::parse` through the
same autoderef chain — every receiver shape the original compiled on, the
rewrite compiles on.

Guards: exactly 2 args; bare 2-segment `TY::from_str_radix` path (no
qself / leading colon / path arguments); `TY` ∈ the 12 int primitives
(exactly where both `from_str_radix` and the radix-10 `FromStr` impl
exist); radix an int LITERAL whose base-10 value is 10 (suffix rides — a
compiling original's literal already unifies with the `u32` param); attrs
refuse; comments in the deleted glue refuse (keep-list = arg0).

Probe (plan151): signed receivers + suffixed radix both fire; group =
**style / default-on** (pinned by the default-groups arm — the plan150
probe had run the fixture only under the full `-W all -W pedantic -W
nursery` union, which cannot distinguish the group).

### `assign_op_pattern` — `y = y + 1` → `y += 1`

**Builtin-op construction identity**: for BUILT-IN ops on primitives,
compound assignment is DEFINED as the binary op + store — same op, same
overflow checks, same wrapping. The overloaded-operator hole (a custom
`Add`/`AddAssign` pair may differ) is closed by TYPE DERIVATION.

**The exactly-once binding derivation** (new substrate, this batch): a
name is PROVEN when bound EXACTLY ONCE in the whole fn (params + lets +
match/closure/for bindings, NESTED ITEMS INCLUDED — a second binding
anywhere kills it) with a type from one of:
- (a) a fn/impl-method param annotation (`mut y: i32`);
- (b) an annotated local (`let mut y: i64 = ..` — the annotation folds
  into `Pat::Type` in syn, unwrapped);
- (c) an unannotated local `let mut y = <already-proven ident>` (the
  init-transfer leg — processed in SOURCE order so the init resolves to
  the one binding the source name has; chains work: param → a → y);
- (d) a SUFFIXED int/float literal init (`0i32`, `0.0f64`). Bare `0` is
  the `{integer}` E0689 ambiguity class; bare `0.0` can still unify with
  `f32` through a later constraint — both refuse.

Assignments never change a local's type, so plain/compound assignment
sites do not kill the derivation — only RE-BINDINGS do. A span-order
guard requires the assignment after the local's binding end (closes the
module-shadow-before-let hole). Nested fns get their OWN derivation pass
via the take/restore discipline (fns cannot capture — an inner fn's `y`
is a different variable; its assignments match against ITS map only);
closures stay in the outer walk (they capture the same variable).

**The swapped-operand boundary is probe-measured, not assumed**: `y = 1 + y`
and `y = 2 * y` FIRE assign_op_pattern; `y = 1 - y` is SILENT (clippy
admits the swapped form only for COMMUTATIVE ops). The proposer mirrors
exactly: the swapped arm fires for `+`/`*` only (`b + y ≡ y + b ≡ y += b`
on integers and IEEE floats), refusing `y = 1 - y` where `y -= 1` would
be different math. (The pre-probe assumption that `y = 1 + y` fires
`misrefactored_assign_op` was WRONG — measured round 1.)

The evaluation-order swap (the y-read moves after the RHS operand in the
compound form) is unobservable: a Copy scalar read has no effects, and an
operand that could observe `y` concurrently would need `&mut y` — a
borrow error in BOTH forms.

Admitted ops: the 10 assign-op pairs (`+ - * / % & | ^ << >>`);
`&&`/`||` have no compound form and refuse. Admitted scalars: the 12 int
primitives + `f32`/`f64` (`bool`'s `&`/`|`/`^` shapes are real but
vanishingly rare — the documented divergence). Group = **style /
default-on** (pinned by the plan151 default-groups arm — the family_cases
"default-level" comments written at Batch 159 were claims; this batch
measured them).

## The graduation + re-arm

- Both families flipped `covered: true` — the 12th + 13th heldout family
  loops (both FIXABLE, the FOURTH consecutive all-FIXABLE pack).
- The 2 consumed frontier rows ARCHIVED (the Batch-159 measured lesson:
  a null-`solving_rule` row blocks its family's fresh admission through
  the KnownSet family-done guard).
- Re-armed with `unit_arg` (the plan150-verified spare, third turn — the
  side-effect-block dodge clearing both plan149 dirt classes) +
  `string_lit_as_bytes` (`"abc".as_bytes()` → `b"abc"`, perf-group,
  Bench-094-family default-level). Verified-but-not-armed alternates
  recorded in the probe: `manual_string_new`, `bool_comparison`,
  `bytes_count_to_len`, `needless_late_init` (whose naive seed
  crossfires OUR OWN `let_and_return` — the twice-used-binding dodge
  `x + x` clears it; recorded as the next spare).

## Gates (GOAT)

- G1: module tests (9 fsr arms + 15 aop arms) + `all_lints_fire_and_parse`
  rows 76→78 + corpus `corpus_size_is_78` + `all_buggy_codes_fire_their_lint`.
- Freeze regen v2/v3 (atom table 57→59): v2 82 labels / 13 excluded,
  v3 83/16 (the count movement vs Batch 159's 78/15 + 80/17 is the
  window shuffle — 0 newly excluded, 0 drift on both e2es proves it
  benign); v2 e2e 34.1 s PASS (assertions = M4 + label fences), v3 e2e
  193.5 s PASS.
- Frontier e2e VERDICT ✓: 32 seeds clean, 13 family loops, 2 fresh
  heldout admissions, the 2 re-arm leads admitted (`lints_before` exactly
  their targets), 11 committed rows verified 0 drift, 4 rows appended,
  band fold 13/13.
- score-bench row #136: heal_rate 96.36% FLAT (vocab 59), frontier
  snapshot generation 7 appended.
- lib default 1917/0 · latent_retrieval 2070/0 · frontier_miner+score_bench
  2234/0 · clippy 0 in 4 states (2 doc_markdown findings in the new
  miner docs healed by our own healer mid-batch — the dogfood loop).

## Authoring lessons (the `.docs/02_corpus` discipline)

1. **syn's `Local` has no `ty` field** — the let annotation folds into
   `Pat::Type(PatType { pat, ty })`; unwrap it for annotated-let
   derivations.
2. **A `visit_item` blanket skip kills TOP-LEVEL fns too** —
   `visit_file` dispatches through `visit_item`. Per-fn scoping belongs
   to the take/restore discipline (a nested `Item::Fn` gets its own
   complete pass; its assignments match against ITS derivation only).
3. **The pty corrupts `cargo run > file` redirects on this box** (rows
   circularly rotated, headers lost) — a regen example must WRITE ITS OWN
   OUTPUT FILE (`fs::write`), never rely on the shell redirect; invoke
   the built exe directly for verification runs.
4. **Probe the boundary, don't recall it**: `y = 1 + y` firing
   `assign_op_pattern` (not `misrefactored_assign_op`) and `y = 1 - y`
   being SILENT were both measurements, both different from prior
   expectations.
