# Issue 003 — Phase 3: verify lint resolution via `cargo clippy` re-run

**Status:** RESOLVED — Phase 3 lint resolution verification shipped behind `clippy_verify` feature. GOAT G1-G4 ALL PASS.
**Type:** Feature (make `lint_resolved` truthful — wire real cargo clippy verification)
**Scope:** new `src/clippy_verifier.rs` behind feature `clippy_verify` + `RustAnalyzerPruner::validate_with_clippy`

## Problem

The L2 pruner's `lint_resolved` field is a **documented lie**:

```rust
// src/pruner.rs line 107
lint_resolved: true, // Phase 3 TODO — see struct doc
```

It always returns `true` even when the fix doesn't actually resolve the lint.
The syntax check (`parses`) is truthful (Issue 002), but the lint resolution
check has never been wired. This is the Phase 3 TODO.

## Design

Feature-gated (`clippy_verify`, default-off) because:
- Slow: seconds per invocation (process spawn + cargo resolution + compile)
- Requires cargo + clippy on PATH
- Temp dir I/O
- Not appropriate for the modelless fast path

### Architecture

New `ClippyVerifier` struct in `src/clippy_verifier.rs`:

1. Create a temp crate under `std::env::temp_dir()/riir_clippy_verify/<blake3_hash>/`
   - Stable dir name (BLAKE3 of code) → cargo reuses build cache → faster on repeat
2. Write `Cargo.toml` + `src/lib.rs` (the code being verified)
3. Run `cargo clippy --message-format=json -- -W clippy::<lint_name>`
4. Parse JSON lines for `compiler-message` with `message.code.code == clippy::<lint_name>`
5. If the lint appears → NOT resolved. If absent → resolved.

### API

```rust
#[cfg(feature = "clippy_verify")]
pub struct ClippyVerifier { ... }

#[cfg(feature = "clippy_verify")]
pub struct ClippyVerifyResult {
    pub lint_resolved: bool,
    pub ran: bool,                // false if cargo/clippy missing
    pub error: Option<String>,
    pub present_lints: Vec<String>,
}
```

Composition with the existing pruner:

```rust
#[cfg(feature = "clippy_verify")]
impl RustAnalyzerPruner {
    pub fn validate_with_clippy(&self, code: &str, lint_name: &str) -> ValidationResult
}
```

### Temp crate

```toml
[package]
name = "clippy_verify"
version = "0.0.0"
edition = "2021"

[lib]
path = "src/lib.rs"
```

### JSON parsing

Each line from cargo clippy stdout is a JSON object. Parse as
`serde_json::Value`, filter for `reason == "compiler-message"`, extract
`message.code.code`. Check if it matches `clippy::<lint_name>`.

## Substrate

- `serde_json` — already a dep ✓
- `std::process::Command` — stdlib ✓
- `std::fs` / `std::env::temp_dir` — stdlib ✓
- `blake3` — already a dep ✓
- NO new dependencies

## GOAT gates

- **G1** (correctness): `needless_return` buggy code → `lint_resolved=false`;
  fixed code → `lint_resolved=true`
- **G2** (latency): document honestly (seconds, not µs). First run builds
  deps; cached runs are faster. Not a hot-path gate.
- **G3** (no-regression): feature off by default → existing 29 tests pass
  unchanged
- **G4** (alloc): N/A for I/O bound — documented honestly
