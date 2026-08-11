# Seed corpus — built-in lint examples

`build_seed_corpus()` returns a `ClippyCorpus` with ~10 entries covering
the most frequent clippy lints across style, complexity, and perf groups.

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
