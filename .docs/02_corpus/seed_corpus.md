# Seed corpus — built-in lint examples

`build_seed_corpus()` returns a `ClippyCorpus` with 51 entries: the 10
seed lints (style, complexity, perf, nursery groups) + the 30 Issue 016
structural-expansion lints (frequency-ranked selection — see
[`.benchmarks/021`](../../.benchmarks/021_clippy_structural_expansion_goat.md))
+ the P7 post-mining verify row (`manual_is_multiple_of`, 2026-08-29) +
2 Batch 89 + 2 Batch 90 + 4 Batch 91 + the 2 P10 post-mining intake rows
+ the P11 post-mining intake row
(2026-09-13).

## The 10 seed entries

| Lint | Group | Pattern |
|---|---|---|
| `needless_return` | style | `return 42;` → `42` |
| `redundant_closure` | style | `\|v\| double(v)` → `double` |
| `needless_lifetimes` | style | `fn first<'a>(s: &'a String) -> &'a str` → `fn first(s: &String) -> &str` |
| `redundant_field_names` | style | `Point { x: x, y: y }` → `Point { x, y }` |
| `manual_map` | complexity | `if x.is_some() { Some(...) } else { None }` → `x.map(...)` |
| `unnecessary_unwrap` | complexity | `if opt.is_some() { opt.unwrap() } else { 0 }` → `opt.unwrap_or(0)` |
| `needless_range_loop` | complexity | `for i in 0..v.len()` → `v.iter().sum()` |
| `useless_conversion` | perf | `let y: i32 = x.into();` → `let y: i32 = x;` |
| `unnecessary_mut_passed` | perf | `&mut s` → `&s` when callee takes `&str` |
| `needless_borrow` | nursery | `take_ref(&x)` → `take_ref(x)` when x is already `&i32` |

## The 30 Issue 016 entries (2026-08-16)

Selected frequency-ranked from kornel's 100K-crate clippy run + Li et
al.'s 8.09M-warning crates.io study ∩ clippy's `MachineApplicable` axis ∩
parse-only provability. 27 are `--fix` eligible (`FIXABLE_LINTS`, 6→33);
3 are `--verify` tier (`VERIFY_LINTS`, 1→4 — type-blind shapes whose
structural detection is still worth attempting under the Issue 015
compile gate):

| Lint | Group | Tier | Pattern |
|---|---|---|---|
| `semicolon_if_nothing_returned` | pedantic | fix | unit fn + bare tail stmt → append `;` (41,601× in kornel's run) |
| `collapsible_if` | style | fix | `if a { if b { c } }` → `if a && b { c }` |
| `collapsible_else_if` | style | fix | `else { if b { c } }` → `else if b { c }` |
| `single_match` | style | fix | `match x { P => e, _ => () }` → `if let P = x { e }` |
| `single_match_else` | pedantic | fix | `match x { P => e, _ => f }` → `if let P = x { e } else { f }` |
| `redundant_pattern_matching` | style | fix | `if let Ok(_) = r` → `if r.is_ok()` |
| `question_mark` | style | fix | `if x.is_none() { return None; }` → `x?;` |
| `range_plus_one` | style | fix | `0..n+1` → `0..=n` |
| `uninlined_format_args` | pedantic | fix | `format!("{}", x)` → `format!("{x}")` |
| `identity_op` | complexity | fix | `x * 1` → `x` (IEEE-754 float-safety split) |
| `needless_bool` | complexity | fix | `if b { true } else { false }` → `b` |
| `needless_bool_assign` | complexity | fix | `if c { x = true } else { x = false }` → `x = c` (real-clippy shape, Issue 017 fix — the assign-RHS form is `needless_bool`) |
| `match_bool` | pedantic | fix | `match b { true => a, false => c }` → `if b { a } else { c }` |
| `needless_match` | complexity | fix | identity `match x { Some(v) => Some(v), .. }` → `x` |
| `redundant_static_lifetimes` | style | fix | `&'static str` in static/const type → `&str` |
| `double_parens` | complexity | fix | `((x))` → `(x)` |
| `unnecessary_lazy_evaluations` | style | fix | `.unwrap_or_else(|| pure)` → `.unwrap_or(pure)` |
| `map_unwrap_or` | pedantic | fix | `.map(f).unwrap_or(d)` → `.map_or(d, f)` |
| `filter_next` | complexity | fix | `.filter(p).next()` → `.find(p)` |
| `iter_nth_zero` | style | fix | `.nth(0)` → `.next()` |
| `manual_unwrap_or` | complexity | fix | `match x { Some(v) => v, None => pure }` → `x.unwrap_or(pure)` |
| `unused_unit` | style | fix | `fn f() -> () {}` → `fn f() {}` |
| `useless_format` | complexity | fix | `format!("lit")` → `"lit".to_string()` / `String::new()` |
| `deref_addrof` | style | fix | `*&x` → `x` |
| `wildcard_in_or_patterns` | complexity | fix | `A \| _ =>` → `_ =>` |
| `unnecessary_sort_by` | perf | fix | `.sort_by(\|a, b\| a.cmp(b))` → `.sort()` |
| `iter_next_loop` | correctness | fix | `for x in it.next()` → `for x in it` (intended semantics fix) |
| `comparison_to_empty` | style | verify | `v.len() == 0` → `v.is_empty()` (12 live workspace hits) |
| `while_let_on_iterator` | style | verify | `while let Some(x) = it.next()` → `for x in it` |
| `single_char_pattern` | style | verify | `.contains("x")` → `.contains('x')` |

## The 2 Batch 89 entries (2026-08-31)

From the Plan 049 T0.1 real-code fired histogram (Bench 061) — what real
clippy fires on our own repos that the corpus did not cover:

| Lint | Group | Tier | Pattern |
|---|---|---|---|
| `use_self` | style | fix | inside inherent `impl V`: `-> V` / `V { .. }` / `V::K` → `Self` (inherent impls only; skips nested items, bounds, attrs, macros; generic args must match the impl head) |
| `suboptimal_flops` | perf | verify | `x.powf(0.5)` → `x.sqrt()`, `x.powf(N.0)` → `x.powi(N)`, `x.log(2.0)` → `x.log2()`, `x.log(10.0)` → `x.log10()` (literal args only; `powf(1/3)` is nursery `imprecise_flops` territory) |

**The killed third lead:** `missing_const_for_fn` (723 instances — the
histogram's #1) was implemented as a VERIFY-tier allowlist proposer, then
KILLED at the gates: a broad const-ability proposer fires on virtually every
trivial fn, cross-firing every eval fixture (the single-anti-pattern
invariant) and the clean-span set, and polluting the instantiation
re-ranker's structural signal. The `needless_borrow` trio precedent
classifies such over-matching proposers as not-fix-eligible; the
corpus-lockstep gate forbids a corpus entry without a proposer; and real
`cargo clippy --fix` already applies the lint's own MachineApplicable
suggestion. Record: `.plans/050` + the `VERIFY_LINTS` note.

## The 2 Batch 90 entries (2026-08-31)

The same fired-histogram lead, pedantic tail — both FIXABLE, both
applicability `Unspecified` under clippy 1.97 (cargo-fix does NOT cover
them, so the proposers are genuinely additive):

| Lint | Group | Tier | Pattern |
|---|---|---|---|
| `items_after_statements` | pedantic | fix | block item (`fn`/`const`/`static`/`struct`/…) hoisted above the statements (order-independent resolution; guards: macro items, leading-gap + same-line-trailing comments) |
| `manual_let_else` | pedantic | fix | `let v = match x { P => v, Q => diverge }` → `let P = x else { diverge }` (visible divergence only: return/break/continue/panic-family; skips binding-use fallbacks, non-identity arm-1 bodies, 3-arm/guard shapes — all probe-matched to real clippy's silent set) |

**The refuted third lead:** `option_if_let_else` (38 instances) — nursery-
group pure style with zero perf/correctness content; its let-match half is
already covered by `manual_unwrap_or`/`manual_unwrap_or_default`; and its
rewrite output (`match x { Some(v) => v, None => 0 }` in statement position)
re-enters `single_match_else`'s Issue-029-T3 std-ctor territory → fixpoint
ping-pong with our own FIXABLE proposer. No corpus entry. Record: `.plans/051`
+ the `FIXABLE_LINTS` note.

## The 4 Batch 91 entries (2026-09-05)

The P9 clippy-1.98 new-lint batch — populations measured workspace-wide by
the 1.98.1 re-gate sweep. All four suggestion-only in real clippy
(MaybeIncorrect / Unspecified — `--fix` applied 0/17 on the dao run), which
is why the proposers exist:

| Lint | Group | Tier | Pattern |
|---|---|---|---|
| `byte_char_slices` | style | fix | `[b'S', b'H', ..]` → `*b"SH.."` (bit-identical; every element a byte literal, non-empty) |
| `manual_slice_fill` | style | fix | `for v in &mut p { *v = val; }` → `p.fill(val)` (pure-value gate; `&mut place` iterable form only — `iter_mut()` silent in real clippy) |
| `manual_checked_ops` | complexity | verify | `if n == 0 { v } else { total / n }` → `total.checked_div(n).unwrap_or(v)` (unsigned-only in real clippy; provably-signed divisors refused) |
| `chunks_exact_to_as_chunks` | style | verify | `bytes.chunks_exact(4)` → `bytes.as_chunks::<4>().0` (chunk type `&[u8]` → `&[u8; 4]`; MSRV 1.88 floor) |

## The 2 P10 post-mining intake entries (2026-09-13)

The 089-T3 usage artifact — both lints needed by hand on `src/bin/heal.rs`
while the mechanical fixer healed around them; both probe-verified
on-by-default in clippy 1.98 (probe crate, 2026-09-13):

| Lint | Group | Tier | Pattern |
|---|---|---|---|
| `filter_map_bool_then` | complexity | verify | `filter_map(\|x\| p(x).then(\|\| b(x)))` → `filter(\|&x\| p(x)).map(\|x\| b(x))` — MachineApplicable in real clippy but the `\|&x\|` pattern binds only Copy items (String items silent — probed); E0507 is check-visible so the verify gate backs it |
| `vec_init_then_push` | style | verify | `let mut v = Vec::new(); v.push(a); v.push(b);` → `let v = vec![a, b];` — help-only in real clippy (`--fix` applied 0/3 even on the plain shape, probed); refuses attributed/cfg'd pushes (a `vec![]` literal cannot carry cfg'd elements — the documented divergence; comment-guarded `allow` is the correct fix for that shape) and every parse-visible later mutable use |

Probe note (the intake's premise corrected by the probe, the
probe-first-law working): the intake described `filter_map_bool_then` as a
"filter+map split" lint — the probe showed the reverse: it fires on
`bool::then` INSIDE `filter_map`, and the split `.filter(pred).map(f)` is
the FIX. The `.filter(p).map(f)` forms in `src/bin/heal.rs` never fired.

The full per-lint fixture tables live in
[`corpus.rs`](../../src/domains/clippy_lints/corpus.rs) + the per-lint
proposer test modules under
[`proposers/`](../../src/domains/clippy_lints/proposers/).

## The 1 P11 post-mining intake entry (2026-09-13)

The Bench-088 usage artifact (`format_collect` fixed by hand on
`corpus_gen::compose_multi` after `--fix --write --verify` applied 0 edits
— the rule did not exist):

| Lint | Group | Tier | Pattern |
|---|---|---|---|
| `format_collect` | pedantic | verify | `.map(\|x\| format!(..)).collect::<String>()` → `.fold(String::new(), \|mut out, x\| { let _ = write!(out, ..); out })` + a `use std::fmt::Write as _;` insertion (skipped when the trait is already imported) — the FIRST two-span clippy rule; help-only in real clippy (probed — no machine-applicable suggestion); the anonymous `as _` import cannot collide (E0252) and cannot be ambiguous (`String` implements `io::Write` nowhere in std — probed E0599) |

Same day, the intake's OTHER row — the `needless_borrow` method-receiver
shape gap — landed as a proposer EXTENSION (no new corpus entry): `(&x).m()`
→ `x.m()` for m in {as_slice, as_mut_slice, len, iter, as_ptr}, full
borrow-peel (clippy's own `(&&v)` → `v` suggestion, probed),
mutability-agnostic, postfix-safe base only. Real clippy fires BROADER
(`(&v).to_vec()` too — probed); the whitelist keeps the strip parse-only
decidable. Tier unchanged (self_evolve-only): the free-fn shape's
over-match on every necessary `f(&local)` borrow is the documented
BY-DESIGN exclusion from the structural set — shape-split tiers would
need pseudo-lint names that fragment the retrieval vocabulary and break
the oracle's name matching.

Fallout repaired (the corpus +1 re-fired `pool_depth_rescue_guard`, the
designed prompt): the Issue-024 casualty's raw rank moved 15 → 16
(first-found depth 17, measured by the guard's discovery sweep — now
first-class diagnostics that print BEFORE the assert); `RERANK_POOL`
16 → 20, `HEADROOM_DEPTHS` gained a 24 rung, `PROD_DEPTH_IDX` 2 → 3
(production keeps a rung above it).

## Honesty caveat

These examples are **illustrative** — the L2 syntax pruner validates
that the `fixed_code` parses as valid Rust, but does NOT verify the lint
is actually resolved in the fast path. For truthful lint resolution,
enable the `clippy_verify` feature and use `validate_with_clippy` (runs
real `cargo clippy`).

The `buggy_code` and `fixed_code` represent the *pattern* the tool
learns from, not verified trigger/fix pairs in the fast path.

## Test coverage

- `seed_corpus_is_nonempty` — corpus has entries + commitment verifies
- `seed_corpus_has_unique_lint_names` — each lint appears exactly once
- `seed_corpus_covers_common_groups` — at least style + complexity + perf
- `all_fixed_code_parses` — every `fixed_code` parses as valid Rust (G1)
- `seed_corpus_freeze_thaw_roundtrip` — freeze/thaw preserves commitment

## Extending the corpus

Add entries by constructing your own `ClippyCorpus`:

```rust
use riir_clippy::{ClippyCorpus, ClippyFixEntry};

let corpus = ClippyCorpus::new(vec![
    ClippyFixEntry {
        lint_name: "my_custom_lint".to_string(),
        lint_group: "style".to_string(),
        buggy_code: "fn foo() { /* buggy */ }".to_string(),
        fixed_code: "fn foo() { /* fixed */ }".to_string(),
    },
]);
```

Or start from the seed corpus and append:

```rust
use riir_clippy::{build_seed_corpus, ClippyCorpus, ClippyFixEntry};

let seed = build_seed_corpus();
let mut entries = seed.entries;
entries.push(ClippyFixEntry { /* ... */ });
let extended = ClippyCorpus::new(entries);
```

## See also

- [`clippy_corpus.md`](clippy_corpus.md) — the corpus type
