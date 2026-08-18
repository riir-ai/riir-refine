# M3 Max ANE hardware findings (h15g)

> Measured 2026-08-19 on the T0 box: M3 Max (Mac15,8), 64 GB, macOS 26.5.2.
> Probe: riir-poc `ane_t0` P1-P6 (`ANE_T0_PROBE=all cargo run -p riir-poc
> --example ane_t0_spike --release`). Full record: riir-ai `.issues/726`.

## Device identity

One `H11ANEIn` IOService (single die — unlike M1/M2/M3 Ultra which expose
two). ioreg `DeviceProperties`:

```json
{"ANEDevicePropertyANECPUSubType": 6,
 "ANEDevicePropertyANEVersion": 176,
 "ANEDevicePropertyANEMinorVersion": 18,
 "ANEDevicePropertyANEHWBoardType": 224,
 "ANEDevicePropertyNumANECores": 16,
 "ANEDevicePropertyTypeANEArchitectureTypeStr": "h15g"}
```

`_ANEDeviceInfo aneSubType` reports "h15" (same string as an M4 — the
maderix M4 bench prints the same subType on this M3 Max box; h15 is the
generation, the "g" suffix is board variant).

## The headline: ONE 16-core fabric, not pinnable instances

**Instance hints (`kANEFAneInstanceHint` 0-4) are all accepted but do not
create hardware parallelism on M3 Max.** Two independently-compiled models
evaluated concurrently on two threads:

| Pair | solo A / B (ms) | serial pair (ms) | concurrent pair (ms) | overlap |
|---|---|---|---|---|
| A(hint=1) vs B(hint=2) | 6.9 / 8.0 | 15.9 | 6.7 | **2.37×** |
| control A(hint=1) vs B(hint=1) | 8.9 / 9.3 | 18.9 | 6.6 | **2.87×** |
| control A(nohint) vs B(nohint) | 9.2 / 9.0 | 20.4 | 7.6 | **2.69×** |

(512ch conv, 256×256 spatial, ~34 GFLOP/eval, 30 iters/leg; rerun
confirmed the same shape.)

The CONTROLS overlap as much as the pinned pair: the scheduler multiplexes
concurrent programs across the 16-core array on its own. omlx's "two
instance-pinned submissions beat one unpinned" (57.90 → 41.51 ms) is an
**M3 Ultra property** (two dies surfaced as instances 1/2) — on M3 Max the
pinning constraint is unnecessary: **just submit concurrently and the
fabric parallelizes**. This is BETTER for hybrid-split designs (no pinning
bookkeeping, no per-instance programs needed for parallelism).

## Throughput observations

- Single 512ch × 64×64sp fp16 conv (2.1 GFLOP): ~0.5 ms/eval ≈ **4.2
  TFLOPS** — per-eval overhead dominated (dispatch floor + small op).
- Same box, maderix bench, deep conv chains (64-128 chained 512ch convs):
  **18.45 TFLOPS fp16 sustained** — the fabric's usable peak for this
  pattern. Single small ops see ~25% of it; concurrency fills the rest.
- Concurrent pair at 34 GFLOP/eval: combined ~12 TFLOPS (near saturation).
- Their W8A8 int8 variant: 1.24× over fp16 at 256ch, 1.00× at 512ch —
  **int8 is NOT automatically faster at large channel counts on h15**;
  the int8 win shows at smaller channels where fp16 is Issue-bound.

## Resident-model budget

**≥256 loaded models** (probe cap; all compiled+loaded at 64ch/64sp,
unloaded cleanly, reload-after-unload OK). omlx's ~121-handle cap was
measured with multi-GB weight banks on M3 Ultra — handle scarcity is not
the constraint at M3 Max; per-layer single-procedure programs are viable
(see `mil_and_blob_formats.md` for why that matters — banks are rejected).

## Memory math for the Bonsai hybrid (T0's budget task)

Ternary-Bonsai-27B (qwen35-GDN): 48 GDN layers (full_attention_interval=4),
per GDN layer `in_proj_concat` = 5120 × (10240 qkv + 6144 z + 48 a + 48 b)
= 5120×16480, `gate_up_proj` = 5120×34816 → **262.7 M weights/layer**.

ANE int8 copies at fraction f: `f × 262.7 MB × 48`:

| f | ANE int8 residency | + originals (Q2_0 ≈ 0.266 B/w, same weights) |
|---|---|---|
| 0.5 | 6.3 GB | + 3.4 GB |
| 1.0 | 12.6 GB | + 3.4 GB |

Whole model Q2_0 ≈ 7 GB resident regardless; IO surfaces ~0.3 GB reused.
**Fits 64 GB at any f** (default GPU wired limit applies to Metal side;
ANE-side residency is plain unified memory). Ternary → int8 requant is
**lossless** (weights ∈ {−1,0,1} exactly representable, no scale blobs —
strictly better than omlx's q4→int8 at cosine 0.9992).

## Shape constraints seen elsewhere (context, not measured here)

- maderix issue #42 (M3 Ultra, macOS 15.7.4): their conv bench accepted
  **only exactly 512 channels** (-4/-3 for every other count). NOT
  reproduced on M3 Max/macOS 26.5 — the same bench ran 512/256/384ch fine
  here. Treat per-arch channel quirks as possible, verify per box.
- Activation SRAM working set is ~2 MB (Research 377) — 2048-token × 5120ch
  fp16 inputs (21 MB) stream from DRAM; the conv pattern tolerates it.
