# Research 359 — Batch 300 landing addendum: the ConstexprCore/perfect_hash rust_perf micro-layer

**Status:** LANDED (Batch 300, 2026-10-31, Plan 305) — converts the YES B− verdict into two benchable `rust_perf` rules (`entries_57`, corpus 287 → 289); Research 341 stays the verdict record, this is the landing delta.

## T0b — the LLVM-fusion falsification probe (the batch's main kill risk)

**Verdict: NOT the oxc positive-instance kill — candidate 1 survives as the lead.**

Probe posture: scratch crate `/tmp/b300_probe` (rustc 1.99.0, x86_64, `-O`),
`--emit=llvm-ir`, arms `#[inline(never)]`:

- **Buggy arm** (`eq_slice`: len-dispatch + `iter().zip().all()`): lowers to a
  **scalar byte loop with a per-byte early-exit branch** — NO `memcmp`/`bcmp`
  call at this size class, NO vectorization (len-dependent trip count).
- **Folded arm** (`eq_fold`: `u16` head/tail + len into one u64): 2× `u16`
  loads + shift/or chain + ONE `u64` compare.
- Interleaved A/B (chunks ×6 after warmup, the sequential-AB law): buggy
  1.92–2.06 ns/verify vs folded 0.76–0.83 ns/verify ≈ **2.6×**.
- G1 equivalence: exhaustive over all 1..=4-byte keys from an 8-char alphabet
  (both orders) + 200k LCG probes over a 6-char alphabet — identical verdicts
  on every pair.

The oxc negative (`keyword-map-to-match`, entries_03) killed a *perfect-hash
dispatch* vs `match`; THIS rule owns the *verify compare at a known slot* —
a different unit, and LLVM demonstrably does not fuse it.

## The official Pareto verdicts (the `rust_perf_bench` validator, `--release`, 4090 box, idle AC desktop, 2026-10-31)

| rule | verdict | f/s/t | notes |
|---|---|---|---|
| `fold-key-len-into-one-word-compare` | **ParetoDominant** | 2/0/1 | n=4096 0.97× tie / n=65536 **2.84×** / n=262144 **1.54×**; allocs 0 vs 1 (the fixed arm builds the packed lane per loop — disclosed, not hidden), bytes 0 vs 2 MiB; det stable ×3 |
| `fused-value-in-spare-lane-bits` | **ParetoDominant** | 2/0/1 | n=4096 1.02× tie / n=65536 **2.55×** / n=262144 **1.27×**; allocs 1/1 (both arms build lanes once); det stable ×3 |

The n=4096 tie cell is the small-table floor (the loop is dominated by the
lane build both arms amortize away at scale); the win is the large-table
verify loop, exactly the rule's stated regime.

**Harness-shape lesson (recorded for the next benchable-rule author):** the
first draft measured a *linear-scan membership* shape and the fixed arm LOST
(the fold helps the compare, not the scan — the added `Option` encode per
probe is pure overhead there). The corrected shape measures the rule's actual
regime: **verify at a KNOWN slot** (the hash's slot pick is outside the
measurement, both arms pay the same lookup). A benchable rule's harness must
isolate the law's unit, not a neighbor's. Also: the validator's `calibrate`
closure requires `run_buggy`/`run_fixed` to return `()` (fold your counter
into `black_box`) and the G1 `check()` needs its own counter helpers.

## What landed

- `entries_57`: the two rules above, both BENCHABLE with a `bench_spec` each
  (the first benchable `rust_perf` rules since Batch 305's interleave rule).
- Candidates 3–7 dropped per Research 341: candidate 3's safe-arm law reduces
  to "pad before you lane-load" (owned by the simd-chunk/swar safety notes);
  4 needs address placement a fixture can't honestly control; 5 is a face on
  the owned PH rule (cited in `minimal-perfect-hash-static-lookup`'s
  kinship space); 6 thin; 7 owned (find-first-of's 256-entry table).
- Count pins: tests.rs len 287→289 (5 multi-line era-witness sites + the
  headline pin), annotated 234→236 (3 sites), with_spec 122→124,
  without_spec 165 unchanged (noted); AGENTS.md + README + the domain doc
  re-pinned; batch300 needle + parse gates appended (2 tests).
- The stale-pin sweep found ZERO additional 287s outside the known sites —
  the B318-era pins were current.

## Defers

- [-] T-D1 (inherited from plan 305 T6's M3 posture note): the GOAT row's
  box-state provenance is THIS box (4090, Windows, AC, idle) — the plan's
  example posture named the M3; the validator verdict is box-stamped
  either way, no re-run owed.
- [-] Fixture standalone-compile+execute as separate `rustc --test` passes:
  subsumed — the validator itself compiles AND executes both arms 3× per
  size (the G1 `check()` gate IS the execution), a stronger form of T3.
