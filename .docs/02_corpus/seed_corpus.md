# Seed corpus — built-in lint examples

`build_seed_corpus()` returns a `ClippyCorpus` with 40 entries: the 10
seed lints (style, complexity, perf, nursery groups) + the 30 Issue 016
structural-expansion lints (frequency-ranked selection — see
[`.benchmarks/021`](../../.benchmarks/021_clippy_structural_expansion_goat.md)).

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
| `needless_bool_assign` | complexity | fix | `x = if c { true } else { false }` → `x = c` |
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

The full per-lint fixture tables live in
[`corpus.rs`](../../src/domains/clippy_lints/corpus.rs) + the per-lint
proposer test modules under
[`proposers/`](../../src/domains/clippy_lints/proposers/).

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
