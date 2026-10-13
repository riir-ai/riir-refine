# Bench 116 — the AVX2 Q2_0 x86 GOAT (the P53 rider, batch 231)

> **Status:** COMPLETE — PARETO-PASS (2026-10-14 ~03:2x +07, 4090 idle loop); annotation tie-in CLOSED 2026-10-14 (batch 231 landed `9d8ca6490` — the rider carries this bench's numbers)
> **Rule:** the P53 x86 SIMD face rider on kernel_opt `lut-free-bit-spread-vadd4-unpack` (entries_gpu_33) — the llama.cpp `0a731cce` #30447 AVX2 Q2_0 vec_dot distill (batch 231, claim `68be73c34`, plan 233 `11e1a8ae9`, in flight on the M3 lane at run time — this gate is the STAGED 4090 half, run AHEAD of the landing so the numbers exist the moment the rider does)
> **Bench:** `benches/bench_007_avx2_q2_x86_goat.rs` (self-contained, `harness = false`, no required-features — x86_64+avx2+fma only by loud refusal, exit 2 elsewhere)

## The verdict

**PARETO-PASS** — G1 exact, G4 zero-alloc, AVX2 strictly faster at every one of 25 sweep shapes.

The exact mirror of the bench_005 lane split (Batch 204's NEON PQ2_0 rule: NotBenchable-on-4090, M3-Max GOAT at bench 106): the M3 box is aarch64, every x86 intrinsic compiles to nothing there, so the rule's NotBenchable-on-M3 note is an M3-host statement — this gate is the x86 half, exactly as the claim row staged it ("the x86 GOAT staged for the 4090 lane").

## Gates

| Gate | Result |
|---|---|
| **G1 integer-exactness** | **311 cases bit-identical** vs the naive per-element scalar reference (the B62 law: the reference IS the rule's buggy_code form). 256 random fixtures across all five power-of-two scale triples (x + both y halves) + 50 edge fixtures (every 2-bit code value via `0b11_10_01_00` / reversed pairs, q8 at BOTH extremes −128/127, all 25 scale pairs) + 5 multi-block chains (nb ∈ {2,3,5,17,33} — the TWO FMA-chain accumulators + the odd-tail shape; the exactness regime is order-free, so chain-vs-sequential accumulation is bit-identical when no addition rounds). |
| **G2 GEMV sweep** | AVX2 strictly faster at **25/25 shapes**; speedup band **2.20×–2.94×** (see the grid below). Interleaved best-of-5 per shape per arm (the sequential-A/B box-noise lesson), correctness-coupled sinks (each arm's GEMV must equal the scalar reference bit-for-bit at setup — a deleted loop FAILS, never a fast zero). |
| **G4 allocation-free** | Zero allocations, both arms (counting global allocator armed around the measured vec_dot path; the constant vectors hoist before the loop). |

## The G2 grid (best-of-5 ms, interleaved; i7-13700K, quiet box)

| rows \ K | 256 | 512 | 1024 | 2048 | 4096 |
|---|---|---|---|---|---|
| **32** | 2.82× | 2.94× | 2.90× | 2.64× | 2.26× |
| **64** | 2.68× | 2.75× | 2.52× | 2.70× | 2.20× |
| **128** | 2.59× | 2.58× | 2.60× | 2.48× | 2.41× |
| **256** | 2.65× | 2.39× | 2.51× | 2.27× | 2.29× |
| **512** | 2.44× | 2.34× | 2.40× | 2.53× | 2.31× |

Absolute anchors (rows=512): avx2 0.634 ms vs scalar 1.463 ms at K=4096; avx2 0.035 ms vs scalar 0.086 ms at K=256. The win is FLAT across the grid (2.2–2.9×) — both arms are compute-bound with identical memory traffic, so the ratio IS the codec's op reduction, not a cache effect. The soft slide toward 2.2–2.3× at high K is the scalar arm's sequential sumf chain getting some store-forwarding help at larger working sets — the band's honest shape, not a defect.

## Provenance (quote beside every number)

```
PROVENANCE: power=AC scheme=high load=0 swap=2339MB gpu=20 %, 498 MiB canary=skipped
```

`AVX2_Q2_BENCH_REQUIRE_PROVENANCE=1` gate posture held — exit 0 on this citable run. The gate FIRED once during the session (a transient load spike to 10.8 vs ceiling 6 → exit 2, correctly refused; re-ran quiet at load=0). The Windows sibling preflight (`riir-reflex/scripts/bench_preflight.ps1`, Issue 065 T3(b)) is the provenance instrument; `canary=skipped` is the ps1's own disclosure (no pinned Windows canary — the .sh canary is laya-Metal-specific).

## The pattern (the PR's three facets, transcribed verbatim)

1. **OR-spread bit unpack** — `v |= v<<12; v |= v<<6; v &= 0x03030303` after `cvtepu8_epi32(loadl_epi64(8 bytes))`: four consecutive Q2_0 weights per byte land in byte lanes — **OR, never add** (the overlapping copies only touch masked-off bits).
2. **maddubs int algebra** — `sum((v-1)·y) = maddubs(v,y) − maddubs(1,y)`, exact for every int8 y (no i16 saturation: |8·3·127| = 3048 < 32767), then `madd(p,1)` → 8×i32 → `cvtepi32_ps` (each lane an exact integer ≤ 8128).
3. **Two FMA-chain accumulators** — interleaved Q2-block pairs each into their own `__m256` chain (FMA latency hiding), odd tail onto chain 0, one final `hsum(acc0+acc1)`.

## Honest readings

- **The rule's mechanism claim prices at ~2.2–2.9× on a big-core x86** — the same honest band shape as the NEON PQ2_0 GOAT's ~2.2× (bench 106: "a WEAK-CORE scalar baseline handicaps the denominator; expect materially less on big cores"). The i7-13700K scalar baseline is a big-core baseline; no source-reported figure was a bar here (the PR reports whole-kernel numbers, not this vec_dot's isolated speedup).
- **Bit-identity holds through the two-chain FMA structure** — the per-lane FMA chains and the scalar's sequential `sumf +=` agree bit-for-bit in the power-of-two-scale regime because every intermediate is an exact dyadic (no addition rounds); the 311-case battery + 25 sweep sinks are the proof, not an assumption.
- **The PR's block pairing is QK2_0=64** — one Q2 block (16 qs bytes → 64 weights, two 8-byte halves) consumes TWO Q8_0 blocks (32 int8 each, own f16 scales); the bench transcribes this layout exactly (the `y + 2*i` addressing).
- **Rust stdarch naming** — `_mm256_maddubs_epi16(a, b)` takes the UNSIGNED operand first (the C intrinsic's argument order); `_mm256_slli_epi32::<12>` is the const-generic shift spelling. Same instructions as the C.
- **std `f16` is still unstable on the pinned 1.98.1** — scales ride as f16 BITS with an exact decoder (the bench_005 note, unchanged; power-of-two normals are exactly representable).

## What this gate does NOT claim

- **No corpus edit at run time** — entries_gpu_33 was the M3 lane's surface mid-batch-231 (the claim row owns it); the verdict landed HERE + in the snapshot row. **Tie-in CLOSED 2026-10-14:** batch 231 landed `9d8ca6490` and the rider's NotBenchable-on-M3 note now carries this bench's numbers inline ("the staged x86 GOAT RAN: BENCH 007/116 … G1 311 bit-identical, G2 25/25 at 2.20x-2.94x, G4 zero-alloc, provenance PASSED") — verified at the file post-landing; no further action.
- **No league-model claim** — this is the RULE's mechanism priced on real shapes, not a Bonsai/Qwen pipeline number.
- **No aarch64 statement** — the rule's NEON/other-arch faces are unchanged; this gate is x86_64-only by construction (exit 2 elsewhere, never a green zero).
- **No measured-vs-source figure dispute** — the PR quotes no isolated vec_dot speedup; 2.2–2.9× is this box's honest measurement of the pattern against the naive per-element form.

## Queue after

The x86 GOAT half of batch 231's staging is DONE (ahead of the landing). ~~poll batch 231's `feat:` landing → the annotation tie-in~~ **BOTH CLOSED**: batch 231 landed `9d8ca6490` (verified: `git log -1 -- src/domains/kernel_opt/corpus/entries_gpu_33.rs`), the rider annotation carries this bench's numbers (verified by file read 2026-10-14). Standing: the watches (971 merge-moment protocol re-armed; dated leads mlxfast ~10-17, walk #15 ~10-20, nnethercote ~11-01).
