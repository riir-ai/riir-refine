# Issue 111 (2026-09-16) — the platform-audit mod-row lattice resolves NOTHING on Windows, so the audit invents platform-dead findings

**Status:** RESOLVED 2026-09-16 — one normalisation at the index boundary,
9 arms proven by perturbation
**Severity:** HIGH on Windows, **zero elsewhere** — and that asymmetry is the
whole finding. `files_modrow_gated` reads **0**, no error, no warning.
**Site:** `src/platform_audit.rs`, `ModRowLattice::build`
**Found by:** running this crate's own `--lib` suite on the Windows
workstation while landing Issue 110. **8 of this module's own fixtures were
red on clean `HEAD`** and nobody had seen it.

---

## The defect

`ModRowLattice::build` resolves each `#[cfg(...)] mod x;` row to its child
file by constructing a candidate path and looking it up in a set keyed by the
source strings:

```rust
let candidates = [row.dir.join(format!("{}.rs", row.name)),
                  row.dir.join(&row.name).join("mod.rs")];
let key = candidate.display().to_string();
if key_set.contains(key.as_str()) { … }
```

`Path::join` emits the **platform** separator. Every key in `key_set` uses
`/` — they come from `git ls-files` (P25) or from a caller's literal
`src/util/win.rs`. On Windows the two never match, so **no mod row resolves
at all**.

⛔ **An unresolved mod row does not error.** The lattice silently degrades to
empty, every mod-row-gated item's root context reads as ungated, and the
audit then reports items as **platform-dead that are perfectly alive** — a
false finding, in the direction that costs a reader time and credibility.
`files_modrow_gated`, the instrument's own report of how much folding it did,
reads 0 and looks like a tree with no gated mod rows in it.

⚠ A real walk makes it worse than a clean either/or: `git_derived_rust_files`
joins the repo toplevel onto git's `/`-separated relative paths, so
production keys are **MIXED** — `E:\repo\src/util/mod.rs`. Normalising only
the candidate side is not enough; that was the first repair attempted and it
still misses.

## Why nothing caught it

The fixtures exist and they are good: 8 of them exercise exactly this
lattice. **No lane in this workspace runs this crate's tests on Windows** —
`full_gate` is macOS/aarch64, the CI runners are `ubuntu-latest`. This is the
katgpt-rs AGENTS.md platform axis one level in from where that document
applies it: not "a `--target` you never pass is a platform you never
compile", but *a platform you never EXECUTE on is a set of assertions you
have never read*.

The 8 reds had presumably been there since `12392811` (the commit that added
the fixtures), through `2eadba4e` and `d5223688`, both of which recorded
green suites — correctly, on the platform they were run on.

## The repair

One normalisation, at the index boundary rather than at the comparison, so
both sides are covered and every other map here stays keyed by the file
string as the caller spelled it:

```rust
let norm = |s: &str| s.replace('\\', "/");
let key_set: HashMap<String, &str> =            // normalised → ORIGINAL
    parsed_files.iter().map(|(f, _)| (norm(f), f.as_str())).collect();
…
if let Some(orig) = key_set.get(&norm(&candidate.display().to_string())) {
    incoming.entry((*orig).to_string())…
}
```

## Verification

`cargo test --lib platform_audit` — **37 passed, 0 failed** (was 28 / 8).
Full suite with the gated features
(`clippy_verify,rust_perf,rust_perf_bench,rust_perf_verify`): **1594 passed,
0 failed, 11 ignored**. Clippy `--all-targets`: 0 findings (the only warnings
on this box are the exFAT incremental-cache hard-link notices).

**Perturbation** — `norm` replaced by identity, which is the pre-repair
behaviour: **9 failed**, every mod-row fixture plus the new one. The arms are
canaries, not decoration.

## ⚠ The new arm is PLATFORM-DEPENDENT and that is a limit, not an oversight

`mod_rows_resolve_through_mixed_path_separators` feeds the lattice a mixed
key set (`E:\repo\src/util/mod.rs`) and asserts the row still folds. It reds
under perturbation **on Windows** and is **VACUOUS on POSIX**, where `\` is
an ordinary filename character and `Path::join` reproduces the `/` form
whatever the input.

The obvious stronger arm — an all-`\` key set — is worse than useless: it
passes on Windows and reds on POSIX for the *opposite* reason, so it is a
false failure rather than a canary. **There is no separator fixture that is
sound on both platforms**, and pretending otherwise would put a red in CI
that says nothing about the defect. Written down here because the next person
to look at that test will want to "fix" its platform-dependence.

## The standing gap this does not close

Nothing automatic runs this crate's tests on Windows, so the next
separator-shaped defect lands the same way. Two honest options, neither taken
here because both are the owner's call: a `windows-latest` job (real
coverage, real cost), or an explicit written-down decision that Windows is
unsupported for this crate — which is currently the *de facto* state without
anyone having chosen it.
