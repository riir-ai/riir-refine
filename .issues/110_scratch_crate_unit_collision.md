# Issue 110 (2026-09-16) — every scratch crate shares ONE cargo unit: the verifier passes a bad fix and the perf bench times the wrong code

**Status:** RESOLVED 2026-09-16 — all three sites repaired, proven both
directions against the same known answer that exposed it
**Severity:** HIGH — the failure is a **silent false PASS** in both
verification oracles and a **fabricated number** in the perf bench. No error,
no warning, exit 0.
**Sites:** `src/domains/clippy_lints/verify.rs` (`clippy_verify`),
`src/domains/rust_perf/verify.rs` (`perf_verify`),
`src/domains/rust_perf/bench.rs` (`perf_bench`)
**Found by:** riir-train Issue 555 — that repo's `ClippyLintOracle` is a
**port of `ClippyVerifier`**, carried the same defect, and was measured red
there. Filed here because the port's parent is here, and because two of these
three sites are load-bearing for `cargo heal --verify`.

---

## The defect

All three `prepare_crate` implementations BLAKE3-key the scratch
**directory** and write a **fixed package name**:

| site | package name | what a wrong verdict means |
|---|---|---|
| `clippy_lints/verify.rs:187` | `clippy_verify` | `lint_resolved = !lint_present` — a probe that reports no lints reads as **"the fix is good"** |
| `rust_perf/verify.rs:218` | `perf_verify` | same shape, `cargo test` instead of `cargo clippy` |
| `rust_perf/bench.rs:442` | `perf_bench` (package AND `[[bin]]`) | the timing loop runs a **binary built from other code** |

⛔ **Cargo's unit metadata for a ROOT package does not separate two
identically-named ones by manifest path.** The moment two scratch crates
share a target directory they are ONE cargo unit: the second is "fresh", and
cargo replays the first's diagnostics or re-runs the first's binary.

The directory key buys nothing against this, and the existing arm
`prepare_crate_different_code_different_dir` asserts exactly the property
that is *not* the one that matters — which is why this survived. Same for
riir-train's copy.

## Measured, twice, with known answers (outside either repo)

**Lib / diagnostics** — crate B is not valid Rust:

```text
$ export CARGO_TARGET_DIR=$PWD/T
$ cargo clippy --manifest-path a/Cargo.toml   # "fn f() {}"
    Checking clippy_verify v0.0.0 (…/a)
    Finished `dev` profile in 0.20s
$ cargo clippy --manifest-path b/Cargo.toml   # "no code fence here"
    Finished `dev` profile in 0.00s
    exit 0                                    ⛔
```

A crate that **cannot compile** reports success in 0.00s and hands back
crate A's (empty) diagnostic set. For `ClippyVerifier` that is
`present_lints = []` → `lint_resolved = true` → **the healer accepts a fix
that was never verified**.

**Bin / timing** — same name, two different programs:

```text
$ cargo run --manifest-path a/Cargo.toml   →  I AM CRATE A
$ cargo run --manifest-path b/Cargo.toml   →  I AM CRATE A      ⛔  exit 0
```

`perf_bench` therefore times crate A's harness and attributes the
nanoseconds to rule B. **That is worse than a false PASS: it is a
measurement that never happened, reported as one.**

## Exposure — what has to be true

The trigger is **two scratch crates sharing one target directory**, which
happens when `CARGO_TARGET_DIR` is set in the environment the verifier runs
in (or `build.target-dir` in a reachable `.cargo/config.toml`). Without one,
each scratch crate gets its own `<dir>/target` and the units do not meet.

⚠ Read that honestly and in the uncomfortable direction: an isolated
`CARGO_TARGET_DIR` is the idiom this workspace's own gate notes recommend
for taking a measurement on a busy box — so **the more carefully somebody
isolates a run, the more reliably these oracles lie to them.** In riir-train
the same class was a 2-in-3 *flake* on a bare box and a **deterministic**
red under `CARGO_TARGET_DIR`.

⛔ **UNDETERMINED, deliberately not assumed either way:** whether any shipped
number — a `fix_verify` verdict, a corpus label, a `.benchmarks/` perf row —
was produced under a shared target dir. Nothing recorded the environment, so
this cannot be answered from the artifacts. It is open, not closed clean.

## The repair (the riir-train fix, ported back)

1. **The package name carries the key.** `clippy_verify_<blake3[..16]>`,
   the same stem as the directory. Two probes can no longer BE one cargo
   unit, in any target dir, ever. For `bench.rs` the `[[bin]]` name must be
   keyed too — that is the half that decides which binary runs.
   ⚠ Hash the manifest TEMPLATE, not the rendered manifest: the rendered one
   carries the key, so hashing it would be circular.
2. **Pin the probe's `CARGO_TARGET_DIR`** to `<crate_dir>/target` rather than
   inheriting one. Belt and braces with (1): the name makes the units
   distinct, this makes them not share a directory at all.
3. **An arm that asserts the thing that matters.** The existing
   `prepare_crate_different_code_different_dir` is not it. The new arm reads
   both rendered manifests and asserts the **package names differ** and each
   carries its own directory's key — pure, no cargo needed, because the
   collision is decided in the manifest.

## Related, and worth a separate look

`clippy_lints/verify.rs`'s manifest has **no empty `[workspace]` table**.
riir-train's copy added one after a scratch crate rooted under a repo's
`target/` was auto-adopted by the enclosing workspace manifest and cargo
hard-errored. Whether that bites here depends on where `work_root` points;
not part of this issue's claim, but it is the same file.

## Cross-repo

riir-train's copy is FIXED — `61d85d62` (Issue 555 there: root cause, five
arms, 3/3 green where the class was 2-of-3 failing). The two repairs are the
same two lines of idea; this issue exists so the parent does not keep the
defect its child already shed.

## Verification (2026-09-16, shikuwa)

**The known answer, re-run with keyed names in ONE `CARGO_TARGET_DIR`** — the
direction that matters, because the defect's whole signature is a silent pass:

```text
$ cargo clippy --manifest-path aaaa/Cargo.toml   # "fn f() {}"        → clean
$ cargo clippy --manifest-path bbbb/Cargo.toml   # "no code fence here"
    |    ^^^^ expected one of `!` or `::`
    error: could not compile `clippy_verify_bbbb` (lib)               ⛔→✅
```

Before: `Finished` in 0.00s, exit 0, no diagnostics. After: the compile error
the probe exists to find. Same tree, same target dir, one line of difference.

**Arms** (3 new, none needs a toolchain — the collision is decided in the
manifest, which is why it was assertable all along):

| arm | asserts |
|---|---|
| `clippy_lints::verify::…::prepare_crate_different_code_different_package_name` | names differ; each carries its own dir key; identical code still reuses one crate |
| `rust_perf::verify::…::prepare_crate_different_code_different_package_name` | same, `perf_verify` side |
| `rust_perf::bench::…::prepare_crate_keys_the_package_and_bin_name` | **both** names — package AND `[[bin]]`. A keyed package with an unkeyed bin still runs the wrong executable, so the bin is the load-bearing half |

**Suites** (`--features clippy_verify,rust_perf,rust_perf_bench,rust_perf_verify`):

```text
--lib                    1585 passed;  8 failed   ← see below, PRE-EXISTING
--lib -- --ignored       11 passed;    0 failed   ← the real-cargo oracle tests
--lib -- prepare_crate    9 passed;    0 failed
clippy --all-targets      0 findings
```

⚠ **The 8 failures are `platform_audit::tests` and are NOT from this change**
— measured by stashing: they fail identically on clean `HEAD`
(`e8f5425e` + 33 fetched commits, `28 passed; 8 failed`). Recorded here
because it was found here, not adjudicated here. Filed and FIXED as
**Issue 111**: `ModRowLattice::build` resolves mod rows by `Path::join`,
which emits `\` on Windows against a `/`-keyed source set, so the whole
lattice silently resolved NOTHING there and the audit invented platform-dead
findings. One normalisation; 37/37, and 9 arms proven by perturbation.

⛔ **These modules are behind `clippy_verify` / `rust_perf_bench` /
`rust_perf_verify`, so a plain `cargo test --lib` compiles them to NOTHING**
— `-- prepare_crate` reported `running 0 tests` with a green exit at default
features, the green-zero trap. Any gate row covering this code must name the
features.
