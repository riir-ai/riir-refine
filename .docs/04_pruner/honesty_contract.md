# Honesty contract — what `parses` and `lint_resolved` actually mean

The L2 pruner ships with a documented honesty contract so callers know
exactly what each field guarantees.

## Field semantics

### `parses: bool`

**Truthful.** This is `true` only when `syn::parse_file` (or the
snippet-wrapping fallback) succeeds. This is a real syntax check —
the load-bearing validation.

### `lint_resolved: bool`

**Depends on which method was called:**

| Method | `lint_resolved` meaning | Truthful? |
|---|---|---|
| `validate()` (fast path) | Always `true` | **No** — documented limitation |
| `validate_with_clippy()` (slow path, `clippy_verify` feature) | `true` only when `cargo clippy` confirms the lint did NOT fire | **Yes** |

### `diagnostics: Vec<String>`

Empty on success. Bounded by the number of parse errors (typically 1)
on failure.

## Why the fast path lies about `lint_resolved`

The fast path doesn't shell out to `cargo clippy` because:

1. **Latency**: `cargo clippy` takes ~130ms/verify (temp dir + cargo
   build). The fast path takes ~5.5µs. A 24,000× difference.
2. **External dependency**: the fast path requires only `syn` (in-process).
   The slow path requires `cargo` + `clippy` on PATH.
3. **Use case**: the fast path is for the inner loop (draft → check →
   refine). The slow path is for final verification before applying a fix.

The honesty contract documents this so callers don't mistake the fast
path's `lint_resolved = true` for a truthful claim.

## The correct usage pattern

```rust
let pruner = RustAnalyzerPruner::new();

// Fast path: inner loop (draft → check → refine)
let result = pruner.validate(&drafted.code, lint_name);
if !result.parses {
    // Syntax broken — reject immediately
    continue;
}

// Slow path: final verification before applying (opt-in feature)
#[cfg(feature = "clippy_verify")]
{
    let verified = pruner.validate_with_clippy(&drafted.code, lint_name)?;
    if !verified.lint_resolved {
        // Lint still fires — don't apply this fix
        continue;
    }
}

// Apply the fix
```

## See also

- [`rust_analyzer_pruner.md`](rust_analyzer_pruner.md) — the pruner API
- [`../05_clippy_verify/clippy_verifier.md`](../05_clippy_verify/clippy_verifier.md) — the slow path
