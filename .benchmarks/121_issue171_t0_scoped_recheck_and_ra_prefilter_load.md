# Bench 121 — Issue 171 T0/T1: scoped re-check GOAT + the RA-prefilter load-cost refutation

**Status:** DONE 2026-10-25. T0b GOAT **PASS → promoted** (`fix_verify` implies `scoped_recheck`). T1: the RA pre-filter **cannot pay** on measured load cost, so T2–T5 are **DEFERRED** with the numbers below.
**Issue:** [171](../.issues/171_ra_prefilter_oracle_for_verify.md) · **Session:** claude-amo-171

## Box state (disclosure)

amo (M5 Ultra, 96 GB RAM). Load average 5.2–5.7: a sibling session was running `cargo test -p riir-gpu … bench_831_decode_long --release` (a Metal decode bench) the whole time. Free pages about 6.5 GB. AC power. Every A/B below ran on the same box back to back. The ratios that decide anything (8.5×, 2.3×) are far larger than the ±22% sequential-arm noise this workspace has measured under load.

## T0a — round-count census

`n_checks` is not persisted anywhere. The census is every recorded `verify: N cargo check(s)` line in the workspace's text files (all 31 repos, `.md/.jsonl/.txt`):

| recorded run | n_checks | packages | rounds after baseline |
|---|---|---|---|
| 2× `kept 1 file / 1 edit, reverted 0` | 2 | 1 | 1 (clean) |
| `kept 1 file / 6 edits, reverted 0` | 2 | 1 | 1 (clean) |
| `kept 6 files / 12 edits, reverted 2` | 3 | 1 | 2 (dirty → clean) |
| katgpt-rs B89–B91 adoption heal (`2cb97410`, Research 108) — "42 cargo checks/pass", 20 reverts | 42 | multi (25 touched) | not decomposable from the record |
| Bench 019 §G5 run 1 (30 edits, 6 errors / 4 files) | not recorded | 1 | ≥ 2 |

**Reading:** no recorded single-package run exceeds 2 rounds. n = 5 is thin, and the multi-package run cannot be decomposed into rounds. So the census alone does NOT defer (per the verdict's Q3 amendment). T1 decides.

## T0b — package-scoped re-checks (GOAT)

**Mechanism:** in round k+1, a package is re-checked only if it was dirty in round k, OR a file rewritten by round k's reverts lives in it, OR it depends on such a package transitively. Dependencies come from the full `cargo metadata --all-features` resolve, which covers every dependency kind and platform, `[patch]` redirects, and sibling-workspace path deps. Anything unknown is re-checked, as before T0b. The graph is built lazily: only when there are ≥ 2 packages and a round finishes with a mix of clean and dirty packages, so a single-package run pays nothing.

**Fixture:** `tests/scoped_recheck_goat.rs`, a real 3-package workspace. `a` gets 1 good + 1 bad edit, `b` is independent, and `c` depends on `a`. There are two cases. In the first, the bad edit is in `a`'s test code, so `c` is clean in round 1 but must still re-check because it depends on `a`. In the second, the bad edit is in `a`'s lib, so `c`'s check is dirty too. The run uses `--release` with 3 interleaved pairs that alternate which arm goes first, after one warm run.

| gate | result |
|---|---|
| G1 soundness + yield | **PASS**: final tree bytes, kept edits/files and the reverted set are identical on vs off, in both cases and on every pair |
| G2 cost | **PASS (count)**: n_checks 9 → 8 with skipped = 1 (`b`), and on + skipped = off. Wall time: median off 0.36s vs on 0.36–0.37s, metadata included. On a 3-package toy one skipped no-op check costs about the same as the metadata call |
| G3 dependency clause | **PASS**: `c` was never skipped in either case |
| G4 no regression elsewhere | **PASS**: `fix_verify`/`compile_oracle` lib tests 39/39 + the new graph unit test; the 2 real-cargo `fix_verify_e2e` tests; `p12_suppress_reverted_e2e` 1/1 |

**Real-workspace costs (katgpt-rs, 35 members):** `cargo metadata --format-version 1 --all-features` takes **0.18s** warm (1.10s cold), paid once per verify run. A no-op `cargo check --all-targets` on a leaf crate (`katgpt-types`) takes **0.08s**, and non-leaf crates cost more. Break-even is about 2–3 skipped package-checks. A katgpt-rs-wide heal touches about 25 packages and its later rounds are usually dirty in a few, so roughly 20 checks are skipped per extra round. **Verdict: GOAT, promoted.** Kill-switch: `RIIR_REFINE_SCOPED_RECHECK=0`.

## T1 — the installed rust-analyzer binary as the pre-filter oracle

The substrate-first rung: `rust-analyzer diagnostics <dir>` from the 1.98.1 rustup component (added this session; it is not in `rust-toolchain.toml`'s component list).

**Span-matching works:** the output gives file, `LineCol` start/end and the code (`RustcHardError("E0308")`). A planted E0308 + E0425 were both reported at the right lines. **The `ra_ap_*` escalation is not needed for span-matching.**

**Load cost: the decisive number.**

| target | `rust-analyzer diagnostics` | one cargo `--all-targets` round | RA ÷ round |
|---|---|---|---|
| 1-file, 1-package workspace (the floor) | **3.17s** | ~0.2s | ≈ 15× |
| riir-refine (733 files) | **11.63s / 11.80s** warm | **1.38s** incremental (3 runs, 1.37–1.38) | **8.5×** |

**Arithmetic (issue §"honest arithmetic" + verdict Q3):** the pre-filter saves at most (rounds_off − 1) cargo rounds and costs one RA load. With every class trusted (the best case), it pays only when RA < rounds_off − 1 rounds. At 8.5× that needs ≥ 10 rounds per run. The census tops out at 2 rounds, and the G2 bar (≥ 2× on multi-round cases) would need about 19. Even the floor (3.17s on a 1-file crate, which is mostly sysroot plus crate-graph load) is above a riir-refine round. So an in-process `ra_ap_*` lane that diagnosed only the touched files would still pay that load. The only way to amortize it is a persistent daemon, which is the external-daemon shape the issue already rejected (blaze).

**Class trust — also negative on the classes that matter:** RA reported **113** distinct (file, code) hard errors on a riir-refine tree that cargo compiles clean:

| code | count | where | cause |
|---|---|---|---|
| E0432 | 34 | examples/tests/benches/src-bin | feature posture (required-features targets) |
| E0282 | 30 | examples, `src/bin/fixer_regate.rs` | inference gap / posture |
| E0425 | 26 | examples, benches | posture |
| E0599 | 9 | examples, src | posture / inference |
| **E0308** | **8** | **katgpt-types `simd/*` (7 files) + katgpt-core `diversity/temp.rs`**, reached through deps | **real inference false-red** on platform-intrinsic code |
| E0277 | 5 | `src/bin/fixer_regate.rs` | inference gap |

E0308 and E0425 are the healer's own top error classes (Bench 019 §G5: E0425 ×3, E0308 ×1). E0308 shows false-reds on clean code, so it cannot be a TRUSTED (final) class without the rescue round. And every rescued class gives back the round the pre-filter was supposed to save.

## Verdict

- **T0b: GOAT, promoted** (`fix_verify = [..., "scoped_recheck"]`).
- **T0a + T1: DEFER T2–T5.** This meets the verdict-tightened defer condition: the recorded corpus is ≤ 2 rounds, AND no class set can pay, because RA load is 8.5× a cargo round. That holds even before class trust, and the most common healer class is not trustable anyway. **Re-open trigger:** RA's per-run cost falls below one incremental cargo round on a real package (for example a future in-process API that skips the sysroot reload), OR a recorded corpus shows runs of ≥ 10 rounds.
