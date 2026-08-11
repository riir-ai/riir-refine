# Issue 002 — L2 pruner: replace stub with real `syn`-based syntax validation

**Status:** RESOLVED — L2 `syn`-based syntax validation wired. GOAT G1-G4 ALL PASS.
**Type:** Feature (make the L2 stub honest — wire real Rust syntax parsing)
**Scope:** `riir-clippy/src/pruner.rs` — `validate` method

## Problem

The L2 pruner's `validate` method is a **lie**:

```rust
pub fn validate(&self, _code: &str, _lint_name: &str) -> ValidationResult {
    // POC stub — actual implementation in Phase 2
    ValidationResult {
        parses: true,         // ← claims parse success without checking
        lint_resolved: true,  // ← claims lint resolution without running clippy
        diagnostics: vec![],
    }
}
```

It claims `parses = true` without ever checking if the drafted fix is valid Rust.
Every drafted fix is unconditionally accepted — even syntactically broken code.

## Design

Implement real syntax validation using `syn` (already in Cargo.lock as a
transitive dependency — zero new dep cost). The modelless-first mandate is
honored: `syn` is a pure Rust parser with no ML.

### Architecture

Two-tier validation:

1. **Syntax check (always, fast, in-process)**: `syn::parse_file(code)` —
   catches syntax errors without process spawning. This is the load-bearing
   check: if the code doesn't parse, nothing else matters.

2. **Lint resolution check (Phase 3, deferred)**: would scaffold a temp crate
   + run `cargo clippy --message-format=json`. Verifies the specific lint is
   gone. Requires process spawning (slow, environment-dependent) — stays as a
   documented TODO for now.

### Snippet handling

Corpus `fixed_code` entries are often single items (fns, structs) or
expressions, not full files. The pruner tries multiple parse targets:

1. `syn::parse_file(code)` — full file (most corpus entries are full items)
2. If that fails, try wrapping in `fn _wrap() { <code> }` for expression snippets
3. If both fail, `parses = false` + collect the diagnostic

### Honesty contract

- `parses = true` → the code ACTUALLY parsed with `syn` (real check, not stub)
- `lint_resolved = true` → STILL a known limitation (Phase 3 TODO), documented
  in the return + in the struct's doc comment. The honest scope of this issue
  is making `parses` truthful.

## Substrate (verified)

- `syn = "2.0.119"` is in Cargo.lock (transitive via katgpt-core → ... → syn)
- `syn::parse_file(&str) -> Result<File, Error>` — parses a full Rust file
- `syn::Error` gives span + message for diagnostics

## GOAT gates

- **G1** (correctness): invalid syntax → `parses = false`; valid syntax → `parses = true`
- **G2** (latency): parse check < 100µs for typical snippet sizes (< 1KB)
- **G3** (no-regression): existing tests pass; the stub test `poc_stub_always_validates`
  is replaced with real-parse tests
- **G4** (alloc-bounded): diagnostics Vec is empty on success, bounded on failure

## Results (M3 Max, release mode)

| Gate | Metric | Result |
|---|---|---|
| **G1** | 8 valid snippets parse, 8 invalid rejected, all deterministic | ✅ PASS |
| **G2** | p50 @ ~40-byte snippet, 10K iters | **6.792 µs** (target < 100 µs, 14.7× headroom) |
| **G2** | p99 | 12.042 µs |
| **G3** | real parse check (not stub) | ✅ PASS |
| **G4** | success diagnostics all empty (1000 iters); failure bounded ≤ 1 | ✅ PASS |

24 lib tests pass (13 pre-existing + 11 new L2 tests). All new code is clippy-clean
(zero warnings in `pruner.rs`; pre-existing pedantic warnings in `corpus.rs`/`drafter.rs`
are unchanged).

**What changed in `src/pruner.rs`:**
- Deleted the stub `validate` (always returned `parses=true, lint_resolved=true`)
- Wired real `syn::parse_file` with snippet-wrapping fallback for expressions/statements
- Added `parse_rust` (multi-strategy parse) + `format_syn_error` + `parse_file_ast`
- The `lint_resolved` field is honestly documented as a Phase 3 TODO (needs `cargo clippy`)
- The `parses` field is now truthful: it's `true` only when `syn` actually parsed the code

**The L2 pruner is now honest:** `parses = true` means the code actually parsed with `syn`.
The old stub returned `parses = true` unconditionally — even for garbage input.
