# Bench 105 — Plan 199 T3.4: the WASM validator lane's three-arm PoC gate

**Status:** PARTIAL — G-safety floor + G-value quantification LANDED; G-cost rows PROVISIONAL (the citable quiet-box run is OWED — this record updates in place when it lands).

**Instrument:** `benches/bench_004_wasm_validator_goat.rs` (`cargo bench --bench bench_004_wasm_validator_goat --features wasm_validator`)
**Date:** 2026-10-07 · **Box:** M3 Max (this workstation), shared with sibling builds.
**Provenance (the run quoted below):** `REFUSED — load average 6.65 exceeds MAX_LOAD=6.0 — a sibling session is on the box` (an earlier same-day run read `PASSED — power=AC Power load=5.13 … powermode=2(high)`; the G-cost rows below are from the REFUSED run and are labeled accordingly).

## G-safety — the hard floor (posture-independent, a correctness claim)

**PASS.**

| family | result |
|---|---|
| planted violations (byte-0 / last-byte braces, `{{` / `}}` escapes, anonymous `{}`, unicode/CJK/emoji adjacency, all-256-byte closed form, 4 KiB buried brace, capacity-exact violating) | **11/11 caught** — one false pass panics the bench permanently |
| clean family (plain literals, lookalike punctuation `[({<|;:`, UTF-8 continuation bytes, capacity-exact clean) | **4/4 passed** (no false flag) |
| score cell (brace count, little-endian at the ABI address) | **15/15 exact** |
| determinism | 15 fixtures × 2 runs byte-identical |
| over-capacity input (32 769 B > 32 768 B ABI region) | `Trap::InputTooLarge` (a named refusal, never a verdict) |

The all-256-byte-values case is the closed form: exactly `0x7B`/`0x7D` are braces — a UTF-8 continuation byte can never read as one.

## G-value — the committed-artifact hot-swap cost

| path | measured (two same-day runs, load states differ — ranges disclosed) |
|---|---|
| install (admission + BLAKE3 + atomic write) | 299–511 µs |
| load (BLAKE3 re-verify + re-admit) | 49–72 µs |
| byte-identical reinstall | 22–26 µs (the `AlreadyCurrent` no-op) |
| **bytes→serving (install + load)** | **349–583 µs** |

The native equivalent of "change the predicate" is a host rebuild (edit → `cargo build` → redeploy) — measured separately at the citable run for the T3.5 verdict.

## G-cost — the 10⁴-span fixture set ⚠ PROVISIONAL (do not cite)

Seeded xorshift64* set (seed `0x1990_0040_0002_0001`), 10 000 spans, 5 000 violating, generator truth verified by an independent scan; the timing is correctness-coupled (verdict/score sinks must equal generator truth × 7 passes or the bench FAILS — Σverdicts 35 000, Σbraces 516 614 ✓).

| arm | best-of-5 total | per-verdict mean | chunk p50 | chunk p99 |
|---|---|---|---|---|
| wasm interpreter (committed artifact, production store-load path, `vm.reset` per verdict) | 61.36 ms | 6.14 µs | 6 327 ns (support 51) | 7 035 ns (support 2) |
| native predicate (the honest baseline, zero boundary cost) | 215.8 µs | 21 ns | 22 ns (support 51) | 25 ns (support 2) |

**Boundary cost: wasm/native ≈ 292×** under a REFUSED provenance — the shape (native wins by two orders of magnitude, as the plan predicted) is readable; the NUMBERS are not citable until the quiet-box run.

## The clippy re-run baseline (env-gated, validated separately)

`WASM_VALIDATOR_BENCH_CLIPPY=1`: the real oracle over a 64-finding zero-dep scratch crate — 2.1 s cold, 178 ms warm, **64/64 `clippy::useless_format` findings** counted by diagnostic rows. The oracle prices at BATCH time (one invocation covers the crate); the pre-gate prices at EDIT time (ns/verdict). Different units — the plan's G-value question is whether the µs-class pre-gate earns its boundary cost over both.

## Provenance discipline (landed with the bench)

- preflight is SPAWNED every run: `PASSED` → G-cost rows citable; `REFUSED` → every G-cost row printed PROVISIONAL; script absent / exit 2 → UNAVAILABLE — provenance is never skipped.
- `WASM_VALIDATOR_BENCH_REQUIRE_PROVENANCE=1` = the citable-run posture: exit 2 unless preflight PASSED (the fire path proven with `MAX_LOAD=0.1`).

## Owed before T3.4 closes

1. The quiet-box citable run (`WASM_VALIDATOR_BENCH_REQUIRE_PROVENANCE=1`, quote the PROVENANCE line here).
2. The native host-rebuild wall measurement (the G-value contrast number).
3. Then the plan's T3.4 checkbox closes and T3.5 consumes the verdict.
