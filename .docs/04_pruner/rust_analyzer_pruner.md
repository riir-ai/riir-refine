# RustAnalyzerPruner — syn-based syntax validator

The L2 layer of the clippy-healing pipeline. Validates that a drafted
fix parses as valid Rust syntax. Always-on, modelless, in-process.

## The two validation levels

### Fast path (always-on): `validate()`

```rust
let pruner = RustAnalyzerPruner::new();
let result = pruner.validate(&drafted.code, "needless_return");
if result.is_valid() {
    // Apply the fix
}
```

Parses the code with `syn`. Corpus entries are often single items (fns,
structs) or expressions — the pruner tries `parse_file` first, then
falls back to wrapping the snippet in `fn _wrap() { <code> }` for
expression/statement snippets.

**`lint_resolved` is always `true` in the fast path** — this is a
documented limitation. The syntax check is the load-bearing validation.

### Slow path (opt-in `clippy_verify`): `validate_with_clippy()`

```rust
let pruner = RustAnalyzerPruner::new();
let result = pruner.validate_with_clippy(&drafted.code, "needless_return");
// result.lint_resolved is now truthful
```

Shells out to `cargo clippy` in a temp crate. `lint_resolved = true` only
when cargo clippy confirms the lint did NOT fire. ~130ms/verify. See
[05_clippy_verify](../05_clippy_verify/README.md).

## ValidationResult

```rust
pub struct ValidationResult {
    pub parses: bool,           // truthful: syn parsed the code
    pub lint_resolved: bool,    // fast path: always true; slow path: truthful
    pub diagnostics: Vec<String>, // parse errors (empty on success, bounded on failure)
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.parses && self.lint_resolved
    }
}
```

## Snippet-wrapping fallback

`parse_rust()` tries multiple strategies:

1. `syn::parse_file(code)` — works for full files + single items (fns,
   structs, impls).
2. If that fails, wrap in `fn _wrap() { <code> }` — works for
   statements + expressions.
3. If both fail, return the first error (most informative).

This makes the pruner robust to snippet-shaped corpus entries that
aren't standalone items.

## Parse-file utility

```rust
let pruner = RustAnalyzerPruner::new();
let file: syn::File = pruner.parse_file_ast(code)?;
```

Direct AST access for callers that need the parsed syntax tree (e.g.
the L3 ruliology search uses this for AST delta-size computation).

## Pattern source

Conceptually implements the `ConstraintPruner` trait from `katgpt-core`,
but ships its own `validate()` API (returns `ValidationResult`, not the
trait's `bool`). The trait's shape didn't fit the two-level (syntax +
lint) validation model. See [honesty_contract.md](honesty_contract.md).

## GOAT gate results

| Gate | Target | Measured | Status |
|---|---|---|---|
| G1 (correctness) | 16/16 deterministic parse classifications | 16/16 | ✅ PASS |
| G2 (latency) | p50 < 100µs | **p50 = 5.459µs** | ✅ PASS |
| G3 (no-regression) | real parse, not stub | valid=true, invalid=false | ✅ PASS |
| G4 (alloc-bounded) | diagnostics bounded | success empty, failure ≤1 | ✅ PASS |

See [`.benchmarks/001_goat_consolidated.md`](../../.benchmarks/001_goat_consolidated.md).

## See also

- [`honesty_contract.md`](honesty_contract.md) — what the fields actually mean
