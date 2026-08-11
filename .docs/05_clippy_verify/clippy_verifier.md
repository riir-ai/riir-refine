# ClippyVerifier — truthful lint resolution via cargo clippy

The Phase 3 verification layer. Shells out to `cargo clippy` in a temp
crate to check whether a drafted fix actually resolves the target lint.

## How it works

```text
1. prepare_crate(fixed_code)
   → creates a temp dir (BLAKE3-hashed for determinism)
   → writes a minimal Cargo.toml + src/lib.rs containing the fixed code
   → dir path: work_root / blake3(fixed_code)[:16]

2. run_clippy(dir)
   → cargo clippy --message-format=json
   → parses JSON output for clippy lint messages
   → returns the set of lints that fired

3. verify(fixed_code, lint_name)
   → returns ClippyVerifyResult {
       lint_resolved: !present_lints.contains(lint_name),
       ran: true/false,
       error: Option<String>,
     }
```

## ClippyVerifyResult

```rust
pub struct ClippyVerifyResult {
    pub lint_resolved: bool,    // true = lint did NOT fire on fixed code
    pub ran: bool,              // false = cargo clippy couldn't run
    pub error: Option<String>,  // error message if ran=false
    pub present_lints: Vec<String>, // lints that fired on fixed code
}
```

## Temp-crate determinism

The temp dir path is `work_root / blake3(fixed_code)[:16]`. Same code →
same dir → cached `target/` dir → faster re-runs. Different code →
different dir → fresh crate. This is the `prepare_crate_stable_dir_for_same_code`
+ `prepare_crate_different_code_different_dir` invariants tested in
the test suite.

## API

```rust
pub struct ClippyVerifier {
    cargo_bin: String,
    work_root: PathBuf,
}

impl ClippyVerifier {
    pub fn new() -> Self;  // cargo from PATH, work_root = system temp
    pub fn with_cargo_bin(cargo: impl Into<String>) -> Self;
    pub fn with_work_root(root: impl Into<PathBuf>) -> Self;
    pub fn verify(&self, fixed_code: &str, lint_name: &str) -> ClippyVerifyResult;
    pub fn is_cargo_available(&self) -> bool;
}
```

## When to use this

- **Final verification** before applying a fix (the slow path after the
  fast L2 check passes)
- **Test fixtures** that need truthful lint resolution
- **CI gates** that verify the corpus's `fixed_code` entries actually
  resolve their target lints

## When NOT to use this

- **Inner loop** (draft → check → refine) — use the fast L2 `syn` check
  instead (~5.5µs vs ~130ms)
- **Headless / no-cargo environments** — `is_cargo_available()` returns
  false; the verifier gracefully reports `ran = false`

## GOAT gate results

| Gate | Target | Measured | Status |
|---|---|---|---|
| G1 (correctness) | JSON parsing identifies target lint | correctly parses | ✅ PASS |
| G2 (latency) | documented, not hot-path | **~130ms/verify** | ✅ PASS (documented) |
| G3 (no-regression) | default features still pass | green | ✅ PASS |
| G4 (alloc-free) | N/A — runs once per verify | N/A | ✅ PASS (documented N/A) |

## See also

- [`../04_pruner/honesty_contract.md`](../04_pruner/honesty_contract.md) —
  the fast vs slow path distinction
