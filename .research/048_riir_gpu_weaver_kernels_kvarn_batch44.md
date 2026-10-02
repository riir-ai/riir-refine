# Research 048 — riir-gpu Batch 44: Weaver GPU kernel substrate + KVarN quantization cluster

**Date:** 2026-08-15
**Batch:** 44 (corpus 325 → 334)
**Sources:** `riir-ai/crates/riir-gpu/src/weaver_gpu/mod.rs` (1,657 LOC) + `riir-ai/crates/riir-gpu/src/kvarn/` (`kvarn_zone_weights.rs` 23 KB + `kvarn_dequant.rs` 13 KB + `kvarn_quant.rs` 12 KB + `kvarn_dispatch.rs`/`hadamard.rs`/`var_norm.rs`/`mod.rs`)
**Method:** 2 parallel mining subagents (Lane A: weaver kernel substrate; Lane B: kvarn cluster incl. canonical `katgpt-kv` cross-verification) + coordinator re-verification of every quoted line (all exact; one off-by-one on a CubeDim line number 1341→1342, corrected before landing).

## Cluster context

**Lane A — `weaver_gpu/mod.rs`:** the CubeCL GPU port of the Weaver marginal
corrector's forward pass (Plan 436) — the 19-dispatch kernel substrate behind
the Batch 43 corrector. `GpuWeaverWeights` upload/download (with
`[in,out]→[out,in]` transpose for plane-GEMV layout), a dead Phase-1 scratch
struct, six kernels with unit-struct launchers (pos-emb add, batched SwiGLU,
causal MHA with smem softmax, indirect embedding gather, dot-per-row,
softmax-over-K with the Issue 468 `P0` `probs_offset` write-shift), four step
helpers, and the production entry `full_forward_gpu`. Consumed by
`GpuWeaverCorrector` on both the per-depth path (`seq_len=2`, `d_depth=1`,
run `depth ×`) and the batched path.

**Lane B — `kvarn/`:** the game-side CPU reference + zone-scale registry for
KVarN (Sinkhorn variance-normalized, Hadamard-rotated, asymmetric-RTN
bit-packed KV-cache quantization; Research 159, Plans 199-200). Post-Issue-042
the heavy primitives are re-exported from canonical `katgpt-kv`
(`rtn_quantize_rows`/`pack_value`/`unpack_value`, `hadamard_transform_inplace`,
`variance_normalize`), leaving 2D tile quantize APIs, per-position + legacy 1D
dequant, a CPU-only dispatch stub, and a `kvarn_game`-gated per-zone
precomputed-scale registry. Only production consumer: the `kvarn_npc_thinking`
example (2D tile APIs).

## The 9 rules (corpus 325 → 334; A 75, B 49, C 33, D 59, E 19, L 11)

| # | rule_id | cat | source |
|---|---------|-----|--------|
| 1 | `row-dot-split-inner-axis-block-reduce` | E | mod.rs L1283-1306 + L1332-1333 + doc L1259-1260 |
| 2 | `fuse-indirect-gather-into-dot-consumer` | C | mod.rs L1195-1196 + L1237 → L1303 + L1344 |
| 3 | `share-cube-size-const-between-kernel-and-launcher` | D | mod.rs L391/L983/L1180/L1283 vs L452/L751/L1125/L1234/L1342/L1493 |
| 4 | `stash-kernel-internal-intermediate-in-smem` | A | mod.rs L1413-1414 → L1429 + contract L1471 |
| 5 | `hoist-bit-address-out-of-packed-row-sweep` | B | kvarn_dequant.rs L41-47 + L51; canonical kv_cache.rs L444-461 |
| 6 | `fold-tile-transpose-into-dequant-scatter` | L | kvarn_dequant.rs L120-133 |
| 7 | `bit-cursor-unpack-to-width-specialized-byte-batch` | A | kvarn_dequant.rs L162-186; canonical kv_cache.rs L1345-1397 |
| 8 | `single-sweep-minmax-for-quant-scale` | B | kvarn_zone_weights.rs L67-68; canonical var_norm.rs L513 + kv_cache.rs L1155-1160 |
| 9 | `serialize-container-canonical-order` | D | kvarn_zone_weights.rs L284-287 |

### Rule highlights

1. **`row-dot-split-inner-axis-block-reduce` (E)** — the dot-per-row kernel
   maps one-thread-per-row: at production (D=4, K=32, h=2048) that is 128
   threads inside a SINGLE workgroup (one SM; the file's own doc presents this
   as fine), with adjacent threads reading 8 KB apart (every warp load = 32
   cache lines, ~16× read amplification). Fix: one workgroup per row, lanes
   stride `h` coalesced, block-reduce partials through smem. The INVERSE
   regime of `grid-over-output-thread-private-sweep` (B33).
2. **`fuse-indirect-gather-into-dot-consumer` (C)** — the gather kernel
   materializes `D*K*h` rows whose ONLY consumer is the dot kernel re-reading
   the whole buffer. Fix: delete the gather; compute `topk_ids[di*k + ki]`
   once per row and read `embedding[id*h + j]` in the dot loop. Removes a
   dispatch, both directions of the round-trip, and the scratch.
3. **`share-cube-size-const-between-kernel-and-launcher` (D)** —
   `let cube_size = 256u32;` in 4 kernels vs `CubeDim::new_1d(256)` in 6
   launchers: a silent stride contract (change one side → wrong results, no
   error, `launch_unchecked`). Fix: one shared const.
4. **`stash-kernel-internal-intermediate-in-smem` (A)** — `softmax_k_f32`
   writes `corrected_logits` to GLOBAL then re-reads it in the next loop of
   the same thread; the buffer is contractually write-only externally
   (verified: the corrector only reads `corrected_probs`). Fix: smem stash +
   drop the buffer from the signature.
5. **`hoist-bit-address-out-of-packed-row-sweep` (B)** — per-row
   `unpack_value(packed_row, pos_in_tile, bits)` re-derives an identical bit
   address (div/mod + cross-byte branch + 2 bounds checks) because
   `pos_in_tile` never varies. Fix: decompose once per position; the
   canonical `dequantize_key_into` is the proven form.
6. **`fold-tile-transpose-into-dequant-scatter` (L)** — materialize
   position-major + a second full transpose pass (2 reads + 2 writes per
   element, `actual_cols + 2` allocs) → scatter `out[ch*cols + pos]` inside
   the producing sweep. Sibling of `tiled-host-transpose-before-quantize`
   (B37, the inverse direction).
7. **`bit-cursor-unpack-to-width-specialized-byte-batch` (A)** — a software
   bit cursor unpacks ONE value per iteration (per-element bounds check +
   data-dependent byte-advance branch + push). For pow2 widths a per-width
   arm makes the shifts compile-time-constant (canonical `unpack_row`:
   `match bits { 8/4/2/1 => ... }`). Also encodes the drifted-fork hazard:
   the inline re-implementation lacks the canonical `bits > lo_bits`
   cross-byte handling.
8. **`single-sweep-minmax-for-quant-scale` (B)** — two `fold`s scan the chunk
   twice for the `(lo, hi)` pair every RTN/AWQ scale prep needs. Canonical
   katgpt-kv fixed exactly this and documented it in-source ("Single-pass
   min/max fold (was two separate `fold` calls scanning `vals` twice).").
9. **`serialize-container-canonical-order` (D)** — `serialize_all` iterates
   `HashMap::values()` (RandomState order): identical content → different
   bytes per process → any BLAKE3 commitment over the serialization is
   unreproducible. Fix: sorted keys / BTreeMap; the wire format unchanged.

## Falsifications (22 total)

**Lane A (14):** per-launch params-handle rebuild (adjudicated
`tiny-param-buffer-rebuild-over-cache`: rebuilds BEAT caching below a cache
line — proposing the opposite would contradict an adjudicated rule); naive
`transpose_weight` (instance of `tiled-host-transpose-before-quantize`);
transpose-at-upload (positive instance of `pre-transposed-consumer-layout`);
QKV/gate-up sibling GEMVs (`fuse-gate-up-gemv-activation` +
`fan-out-shared-input-single-sync`); 3-dispatch norm chains (`fuse-add-rms-norm`
+ `epilogue-at-register-write-phase`); `probs_offset` write-shift (the file
implements the FIX — positive instance); thread-0 sequential softmax at K ≤ 512
(regime-honest documented trade-off); naive sigmoid (benign here); MHA smem +
2D grid (covered ×3); `download()` 12 sequential `read_one` (test-only path +
`batched-downloads-single-sync-cycle`); CPU-precomputed `scale`
(`derived-scalar-host-computed-in-uniform`); written-twice buffer choreography
(thin `inplace-output-alias` + same-queue variant); runtime div/mod
(`strength-reduce-div-pow2` needs compile-time divisor); natural `exp` vs
`exp2` at tiny K (thin).

**Lane B (8):** per-tile `to_vec` scratch (caller-owned-split-scratch family);
per-position `Vec` return (`readback-destination-passing-free-fn`); standalone
`s_col` hoist (folded into rule 5); FMA affine (`factor-affine-quant-scale`);
pow2-gate skipping Hadamard on partial tiles (hazard, not rule);
`from_tile_sequence` reimplementing canonical scale stats
(`superseded-primitive-machinery-zombie` — hazard); `bpr` from `s_col.len()`
(`derive-shape-from-payload` — hazard); canonical-side positives (nothing
in-target to mine in `variance_normalize_into`/`apply_scales_into`).

## Bug yield — riir-ai Issue 698

**Lane A (12 hazards):** H1 HIGH the dot-per-row single-workgroup/16×
amplification perf cliff (rules 1-2 are the fix); H2 MED the dead
`GpuWeaverScratch` with known-wrong sizing (its own doc + the corrector's doc
confirm; any future wiring = GPU OOB writes); H3 MED cube-size literal drift
(rule 3); H4 MED unchecked embedding-gather ids (kernel-side half of Issue
697's OOB window); H5-H12 LOW/INFO (idle 255/256 threads in softmax_k, stale
`#[allow(dead_code)]` × 11, module header doc-lies, test-only `download()`
perf, one-time naive transpose, benign sigmoid, dead `_ki` calc, load-bearing
`assign_op_pattern` workarounds).

**Lane B (13 hazards):** H1 HIGH the legacy KVarN round-trip DROPS the VarN
inversion (`kvarn_quant.rs` L160: `let _var_scales = variance_normalize(...)`
— scales discarded; `kvarn_dequantize_cpu` has no var_scales param) →
systematic scale error (~3.67× computed for the L208 test ramp), masked by
vacuous tolerances (max < 15.0 / avg < 5.0 on data spanning ~12.7); H2 HIGH
the CPU/GPU auto-route doc-lie (mod.rs L6 + `kvarn_dispatch.rs` ignores
`_batch_tiles`, always `used_gpu: false`, zero production callers); H3 HIGH
`kvarn_dequantize_key_tile` dead (zero callers + zero tests); H4 MED-HIGH the
inline bit cursor cannot read cross-byte values (canonical `unpack_value`
handles `bits > lo_bits`; `bits: u8` unrestricted → 3/5/6/7 silently corrupt
while quant packs correctly); H5 MED `hot_swap_zone` sets `active_zone` before
lookup (unknown zone → inconsistent state); H6 MED HashMap serialization
nondeterminism (rule 9); H7 MED the (mean,std)/(scale,zp) doc drift ×3 +
vacuous consistency test; H8 MED the drifted unpack fork (H4's root);
H9 MED quant/dequant Hadamard contract asymmetry + gate mismatch; H10 MED
8 Sinkhorn iterations on a degenerate 1×N tile whose scales are then
discarded; H11 LOW missing `hadamard_cols_into` re-export; H12 LOW
`memory_bytes()` off-by-4 per bundle; H13 LOW dead pub surface (dispatch,
zone registry, legacy pair — test-only, but `pub` API).

## GOAT

| Gate | Result |
|---|---|
| G1 (syn-parse of all 9 new rules' ref_shape + opt_pattern) | ✅ PASS (348 kernel_opt tests incl. `batch44_pseudocode_parses`) |
| G2 (lib tests) | ✅ PASS — 348 kernel_opt (345+3) + 201 default |
| G3 (no-regression) | ✅ PASS — clippy 0 warnings in all 4 states (default / kernel_opt / rust_perf / latent_retrieval, `--lib`) |
| Count pins | ✅ all sites in sync at 334 (entries.len(), KERNEL_CATEGORIES + per-category comments, Plan-003 expected table, grouped totals ×2, router doc ×2 + comment + assert, batch-list comment, 2 stale-corpus comments) |

## Landing notes

- One-shot Python generator, dry-run first: **first anchor pass clean**; one
  post-write defect (`lit()` omitted `.to_string()` → 36 E0308) caught by the
  first compile, fixed in the generator + full revert + re-run (clean abort,
  zero pollution — the revert-verify discipline held).
- One concepts_covered case-sensitivity fix post-run ("TWO" vs "two" — the
  Batch 39 lesson class, now third occurrence: assert the EXACT case of the
  prose).
- 2 `doc_markdown` backtick fixes pre-commit (`seq_len` in `<= 16` prose +
  `VarN` — the CamelCase-adjacent family again).

## Next targets (Batch 45 candidates)

`hypernet/` (~3,300: meta_lora 25 KB + train_meta_lora 20 KB + m2p_transformer
21 KB + gpu_m2p 20 KB + shine 15 KB + routing 8 KB + types 8 KB), `maxsim/`
(spectralquant.rs 20 KB + dispatch.rs 14 KB — NOTE: distinct from the mined
`spectralquant/` dir), `kimi_k3_gpu_backward_batched.rs` (LOW value — shares
Issue 693's H1/H2 + broken LCG). `weaver_gpu/tests.rs` test-only, not mined.
