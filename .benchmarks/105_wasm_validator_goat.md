# Bench 105 — Plan 199 T3.4: the WASM validator lane's three-arm PoC gate

**Status:** G-safety floor + G-value + G-cost CITED — the gate is COMPLETE (all three arms measured; the citable run's PROVENANCE line quoted below).

**Instrument:** `benches/bench_004_wasm_validator_goat.rs` (`WASM_VALIDATOR_BENCH_REQUIRE_PROVENANCE=1 WASM_VALIDATOR_BENCH_CLIPPY=1 cargo bench --bench bench_004_wasm_validator_goat --features wasm_validator` — exit 0)
**Date:** 2026-10-07 · **Box:** M3 Max, shared with sibling builds; preflight-arbitrated.
**Provenance (the CITABLE run):** `PROVENANCE: power=AC Power load=5.26 swap=6268.94M canary=skipped powermode=2(high)` — preflight PASSED (its own MAX_LOAD=6.0 arbiter; an earlier same-day attempt at load 6.65 was REFUSED and discarded, the gate working as designed).

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

## G-cost — the 10⁴-span fixture set (CITABLE)

Seeded xorshift64* set (seed `0x1990_0040_0002_0001`), 10 000 spans, 5 000 violating, generator truth verified by an independent scan; the timing is correctness-coupled (verdict/score sinks must equal generator truth × 7 passes or the bench FAILS — Σverdicts 35 000, Σbraces 516 614 ✓).

| arm | best-of-5 total | per-verdict mean | chunk p50 | chunk p99 |
|---|---|---|---|---|
| wasm interpreter (committed artifact, production store-load path, `vm.reset` per verdict) | 59.90 ms | **5.99 µs** | 6 028 ns (support 51) | 6 795 ns (support 2) |
| native predicate (the honest baseline, zero boundary cost) | 210.5 µs | **21 ns** | 21 ns (support 51) | 23 ns (support 2) |

**Boundary cost: wasm/native ≈ 285×.** Same-day REFUSED-run shape agreed (292× at load 6.65) — the ratio is stable across load states.

## G-value — the citable run's values

| path | citable run |
|---|---|
| install (admission + BLAKE3 + atomic write) | 304.8 µs |
| load (BLAKE3 re-verify + re-admit) | 53.2 µs |
| byte-identical reinstall | 21.1 µs (the `AlreadyCurrent` no-op) |
| **bytes→serving (install + load)** | **358 µs** |

The native equivalent of "change the predicate" is a host rebuild (edit → `cargo build` → redeploy) — minutes-class on this tree; recorded as T3.5 context, not a gate input.

## The clippy re-run baseline (arm 3, measured in the citable run)

`cargo clippy --message-format=json --quiet` over a 64-finding zero-dep scratch crate — **178 ms warm wall, 64/64 `clippy::useless_format` findings** counted by diagnostic rows (2.1 s cold on the first same-day run). The oracle prices at BATCH time (one invocation covers the crate); the pre-gate prices at EDIT time (ns/verdict). Different units — the plan's G-value question is whether the µs-class pre-gate earns its boundary cost over both.

## Provenance discipline (landed with the bench)

- preflight is SPAWNED every run: `PASSED` → G-cost rows citable; `REFUSED` → every G-cost row printed PROVISIONAL; script absent / exit 2 → UNAVAILABLE — provenance is never skipped.
- `WASM_VALIDATOR_BENCH_REQUIRE_PROVENANCE=1` = the citable-run posture: exit 2 unless preflight PASSED (the fire path proven with `MAX_LOAD=0.1`).

## T3.4 verdict inputs (for T3.5)

1. **G-safety: PASS** — the floor is posture-independent and permanent (one false pass fails the gate).
2. **G-cost: native wins by ~285×** (5.99 µs vs 21 ns per verdict) — the plan predicted exactly this shape; the wasm lane cannot win on raw verdict cost.
3. **G-value: the hot-swap costs 358 µs bytes→serving** with zero host rebuild — the capability exists and is cheap; whether a hot-swap CONSUMER materializes is T3.5's question, not a measurement.
4. **The clippy re-run baseline: 178 ms warm / 2.1 s cold per invocation** at batch granularity.
