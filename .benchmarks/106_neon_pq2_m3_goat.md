# Bench 106 — the NEON PQ2_0 M3-Max GOAT (Plan 204 T11)

> **Status:** COMPLETE — PARETO-PASS (2026-10-08, M3 idle loop)
> **Rule:** `neon-tbl-replicate-2bit-codec-sdot-block-rescale` (Batch 204, rust_perf simd — the PrismML-Eng/llama.cpp #265 NEON PQ2_0 vec_dot distill, pin `6bfcd79a2d`)
> **Bench:** `benches/bench_005_neon_pq2_m3_goat.rs` (self-contained, `harness = false`, no required-features — aarch64-only by loud refusal, exit 2 elsewhere)

## The verdict

**PARETO-PASS** — G1 exact, G4 zero-alloc, NEON strictly faster at every one of 25 sweep shapes.

The rule's NotBenchable annotation was an x86_64-host statement, never a no-bench statement: this gate is the M3-Max half of the staged GOAT, exactly as Plan 204 T11 specified — rust_perf Pareto validator + integer-exact G1 + the 256..4096 K × 32..512 rows GEMV sweep.

## Gates

| Gate | Result |
|---|---|
| **G1 integer-exactness** | **311 cases bit-identical** vs the naive per-element scalar reference (the B62 law: the reference IS the rule's buggy_code form — never a transposed-lane scalar, which would measure LAYOUT not codec). 256 random fixtures across all five power-of-two scale pairs (both axes) + 50 edge fixtures (every 2-bit code value via `0b11_10_01_00` / reversed pairs, q8 at BOTH extremes −128/127, all 25 scale pairs) + 5 multi-block chains (n ∈ {2,3,5,17,33} — the sumf accumulation chain incl. the odd-tail half-block layout). |
| **G2 GEMV sweep** | NEON strictly faster at **25/25 shapes**; speedup band **2.00×–2.35×** (see the grid below). Interleaved best-of-5 per shape per arm (the sequential-A/B box-noise lesson), correctness-coupled sinks (each arm's GEMV must equal the scalar reference bit-for-bit at setup — a deleted loop FAILS, never a fast zero). |
| **G4 allocation-free** | Zero allocations, both arms (counting global allocator armed around the measured vec_dot path; the fixed idx/shift/mask tables hoist before the loop). |

## The G2 grid (best-of-5 ms, interleaved; M3 Max)

| rows \ K | 256 | 512 | 1024 | 2048 | 4096 |
|---|---|---|---|---|---|
| **32** | 2.00× | 2.11× | 2.24× | 2.26× | 2.24× |
| **64** | 2.05× | 2.17× | 2.23× | 2.26× | 2.26× |
| **128** | 2.00× | 2.14× | 2.23× | 2.25× | 2.25× |
| **256** | 2.02× | 2.19× | 2.22× | 2.26× | 2.25× |
| **512** | 2.02× | 2.17× | 2.21× | 2.26× | 2.35× |

Absolute anchors (rows=512): neon 0.078 ms vs scalar 0.184 ms at K=4096; neon 0.006 ms vs scalar 0.012 ms at K=256. The win is FLAT across the grid (2.0–2.35×) — both arms are compute-bound with identical memory traffic, so the ratio IS the codec's op reduction, not a cache effect.

## Provenance (quote beside every number)

```
PROVENANCE: power=AC Power load=4.37 swap=6156.88M canary=skipped powermode=2(high)
```

`NEON_PQ2_BENCH_REQUIRE_PROVENANCE=1` gate posture held (exit-2 unless PASSED). High Power Mode is DISCLOSED on the line (the preflight's own posture check passed it; bench_004's citable run carried the same posture). Box otherwise quiet (no sibling cargo/rustc at run time — Protocol H verified).

## Honest readings

- **The source-reported 5.1×/4.6× (RK3588) is NOT reproduced here and was never a bar** — the rule text itself says why: "a WEAK-CORE scalar baseline handicaps the denominator; expect materially less on big cores (M3 Max)." Measured: **~2.2× typical, 2.00–2.35× band** — the honest M3-Max number. The rule's mechanism claim (integer-register decode + sdot + one rescale per block vs ~4 scalar ops/weight) is exactly what the flat ~2.2× shows.
- **Rust naming note (recorded for the next reader):** ACLE's `vsdotq_s32` is `vdotq_s32` in Rust stdarch — the corpus rule text carries the C name (correct for its source); the bench carries the Rust name. Same instruction (sdot, v8.2 asimddp, NOT i8mm).
- **std `f16` is still unstable on the pinned 1.98.1** — the corpus rule text is string data (never compiled), so the bench carries the scale as f16 BITS + an exact decoder; the fixtures use power-of-two normals (exactly representable). If the toolchain stabilizes f16, the bench can switch to the type wholesale.
- The healer landed 2 mechanical fixes (manual_is_multiple_of, unnecessary_cast) + 2 manual (missing # Safety doc, div_ceil) — clippy `-D warnings` clean on the bench target.

## What this gate does NOT claim

- No league-model claim: this is the RULE's mechanism priced on real shapes, not a Bonsai/Qwen pipeline number.
- No x86_64 statement: the rule's non-NEON fallback story is unchanged; this gate is aarch64-only by construction (exit 2 elsewhere, never a green zero).
- No corpus edit: entries_17.rs is untouched (the 4090 lane owns corpus surfaces — Issue-665); the verdict lands HERE + in Plan 204 T11 + the snapshot row.

## Queue after

Plan 204 T11 CLOSED (the plan is COMPLETE end to end). The corpus lane (B208 staged, 4090) is unaffected. Refine Layer 5c (all-features integration sweep, the 3600 s budget class) remains the standing M3 verification item.
