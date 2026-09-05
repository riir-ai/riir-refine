# 137 — Distill verdict: ggml-org/llama.cpp v0.4.0 delta (Batch 113 candidate)

**Status:** RECORD — Batch 113 LANDED 2026-09-05 (Plan 086; 4 rules + 18 riders, corpus 518 → 522, executed ahead of the soft arxiv-walk slot since the walk is time-blocked; batch record in riir-clippy AGENTS.md). Mine-time verification: a dedicated read lane re-read all five surfaces with exact line numbers; THREE attribution corrections — the XOR-swizzle (#25635) and MoE-fusion (#25952/#27621) commits are CUDA-side (zero 'moe'/'swizzle' in ggml-metal at this pin); --lazy-mode is APP-LAYER (common/arg.cpp:2737, include/llama.h:217, src/llama-model-loader.cpp:1082-1088), absent from ggml/src; the Metal-4 gate is has_tensor/MTLGPUFamilyMetal4_GGML + a name gate (M5/M6/A19/A20) with MEASURED reasons in-source (M2 Ultra ~5% slower, M4 no significant difference) — richer than the verdict's gating claim. fastdiv_L is byte-spaced 8-slot with a 6-bit mask (not tightly-packed 6-bit fields) and carries no in-source commit comment. Candidate #4 (sparse-FA) LANDED AS A RULE in category I (the T1 read confirmed the sentinel/clamp/single-source contracts); #20 FWHT revert is in-tree (restore post-pin) — regime evidence only.

- **Source:** https://github.com/ggml-org/llama.cpp/releases/tag/v0.4.0 (release notes span v0.3.0 → v0.4.0, ~150 commits)
- **Inspection date:** 2026-09-05
- **Pin:** ggml-org/llama.cpp @ `5266f24da75dc449bd56cbed7addb9c8e4a6a73e` (v0.4.0; tag object `5ac847190e979e0da7c4a21806630805f396d487`), MIT license. Shallow clone read at this pin (`.raw/llama-cpp-v040`), quotes below verified byte-accurate, clone removed same session per the .raw discipline.
- **Prior contact:** Batch 104 (Research 126 / Plan 074) mined the fa-vec tuning infrastructure at pins `efeda76b9` / `8963a9bdc` / `d230ddd76` — **all three are INSIDE this release window**, so this is a DELTA RE-MINE question, not a first verdict. Batch 96 (Research 114) mined the PrismML fork @ `e311ed38f` (which does NOT carry fa-vec — see the league flag below).

## Verdict

**YES — grade B+.** Route `kernel_opt` (the delta is GPU-kernel dominated; the few host-side items are thin instances of existing rust_perf rules). Mine as ONE delta batch (**Batch 113**) led by a two-rule pair from the Mamba-2 chunked SSD-MMA kernel, plus a shader-fastdiv rule, a large rider sweep (~12–15 external confirmations), and one league flag.

The center of mass is confirmations, not novelty — most mechanism commits in this window are external instances of rules the corpus already holds (B92, B96/B44, B98, B104, B105, B107, B110, B76, B65, B71, B22, B23, B38, B46, B57, B58). That is rider gold, and it is also why the grade is B+ and not A: only 3–4 genuinely new rule shapes exist here.

## GOAT lead (read and verified at the pin)

`kernel_ssm_scan_ssd_mma_f32` — `ggml/src/ggml-metal/kernels/ssm.metal:300-473` + the host gate in `ggml/src/ggml-metal/ggml-metal-ops.cpp` (~L1790-1830, commit `11cd988` #26647 "chunked SSD MMA for Mamba-2 prefill optimization"). The kernel header states the mechanism:

> "Chunked SSD SSM scan via Metal simdgroup MMatrix Multiply-Accumulate (simdgroup_float8x8) fast path. One threadgroup per (head, sequence) and tokens are processed in chunks. C*B^T computed in each chunk one time and reused across the head_dim channel tiles."

Yields TWO rules, not one:

1. **`chunk-transition-consumed-by-mma`** (category G, the recurrence-strategy ladder's fifth member). The chunked SSM recurrence where the heavy products ride the matrix unit: the per-chunk Gram `C·Bᵀ` is built ONCE per chunk (simdgroup MMA over 8×8 tiles) and REUSED across every head-dim channel tile (the B110 m-group shared-operand shape at chunk granularity); intra-chunk output = masked `(C·Bᵀ ∘ exp(acs_i − acs_j))·dtX` via MMA (the decay weights fold into the stored Gram after the matmul — a placement decision: mask-and-scale the Gram once, not per output tile); inter-chunk output = `C·S_prev` via MMA; the carried-state update `s' = chunk_decay·s + Σ_t state_decay_t·B_t·dtX_t` stays SCALAR (see rule 2). Eligibility is a five-clause gate (B46 external confirmation — the gate names every cost factor): `mma_tokens > 0 && ne30 == 1 && props_dev->has_simdgroup_mm && d_state % 8 == 0 && d_inner == OP_SSM_SCAN_SSD_HD` — hw capability + scalar-decay shape + tile alignment + head-dim SPECIALIZATION, each commented. The tail is the load-bearing dispatch shape: `mma_tokens = ((n_seq_tokens - snap_reserve) / CHUNK) * CHUNK` ("largest multiple of CHUNK that leaves snap_reserve for the tail") — run the MMA kernel over the chunk multiple, then fall back to the SEQUENTIAL kernel family for the remainder (a kernel-FAMILY fallback, not padding; `snap_reserve` exists because the sequential kernel owns the rollback snapshots — a B71/B58 interop constraint). GOAT criteria: benchable (batch gates; optional measured A/B via the riir-train SSM lane); semantics = chunk OUTPUTS reassociate → tolerance-gated, stated; magnitude = O(CS²·nc + CS·nc·HD) work moved from scalar lanes to simdgroup MMA, upstream ships it as the Mamba-2 prefill optimization; deterministic (fixed chunk/tile order, deterministic tail split).

2. **`carried-state-reduction-stays-scalar`** (category D — the tolerance-asymmetry partition). The source's own contract, ssm.metal:454-456:

> "Keep the carried-state reduction in token order. Reassociating this particular product with MMA compounds rounding differences at every chunk boundary; CB, y_diag, and C*S remain on the matrix unit."

This is a per-product PLACEMENT decision with a numerics rationale, and it is the sharper novelty of the pair: products feeding PER-CHUNK OUTPUTS (non-compounding — each output consumed once) may ride the matrix unit; products feeding the CARRIED STATE (compounding — last-bit drift crosses every chunk boundary through the recurrence) stay scalar in token order even though the same MMA unit is idle-ready. Distinct from B95 `reduction-restructure-needs-source-owned-comparator` (that pins a restructured reduction bit-identical against an oracle; this DECIDES WHICH products may reassociate at all — the oracle-half and the partition-half compose), from B79 (gates the whole recurrence's parallelization strategy), and from exact-op-parity (forward/backward consistency). Generalizes to any chunked-recurrence MMA implementation: GDN, RWKV, SSM all must make this partition.

## Ranked candidate table

| # | Candidate | Source @ pin | Grade | Disposition |
|---|---|---|---|---|
| 1 | `chunk-transition-consumed-by-mma` (+ per-chunk Gram reuse) | ssm.metal:300-449 + ops.cpp use_mma | A- | NEW RULE (G). GOAT lead, half 1. |
| 2 | `carried-state-reduction-stays-scalar` | ssm.metal:454-468 | A- | NEW RULE (D). GOAT lead, half 2 — the sharper content. |
| 3 | `shader-fastdiv-mulhi` | ggml/src/ggml-vulkan/vulkan-shaders/utils.glsl:1-37 (commit `c9ca51c` #27526) | B | NEW RULE (N/arithmetic). `fastdiv(n, mp, L) = (umulExtended-mulhi(n, mp) + n) >> L` — the round-up magic-multiplier form (B59's mechanism) in shader form, where 64-bit multiply does not exist so the mulhi intrinsic IS the construction; plus `fastdiv_L` packing eight 6-bit shift amounts into one u32, plus `fastdiv_small(a,b) = (a < b) ? 0 : a / b` — the quotient-zero branch-skip (in coordinate decomposition the leading divisions usually have quotient 0; skip the hardware div). Adjacent: rust_perf B59 (CPU, u64 widening) + kernel_opt `strength-reduce-div-pow2` (pow-2 only) — the GPU mulhi + round-up + L-pack + zero-guard cluster is unclaimed. |
| 4 | sparse-attention KV budget (index-list FA) | ggml/src/ggml-metal/kernels/fa.metal:1077-1176 (`7bb0fc1` #28098, `8e93a97` #27970, `cb30059` #27453, `daef7b6` #28032) | B- | Rule-or-rider, decide at T1: the kernel consumes a PRE-COMPUTED selected-position list, `-1` sentinel-padded to `n_kv_max_padded` (a multiple of the NCPSG tile), with a clamp-on-hint-violation fallback ("if the hint is violated, keep only the first n_kv_max entries"). Model-arch adjacent; the mineable idiom is sentinel-padded index-list attention + the hint clamp. Adjacent B107/B97. |
| 5 | metal top-k radix | ops.cpp:5249-5422 (`2a74817` #28073) | rider | B107 `radix-select-ordered-key-topk` confirmation (order-preserving float→unsigned key map + 4 radix-8 histogram passes + compact). The NEW bit is the gate: "radix-select has a fixed single-workgroup-per-row cost (~50-60us)" → `use_radix` shape gate, and ops.cpp:5249 "where the single-workgroup-per-row radix-select cannot reach enough parallelism" — a fixed-cost-vs-row-length threshold (B77 twins / B104 tables instances). |
| 6 | XOR-swizzle FA K,V smem fp16 tiles | `e4b9af0` #25635 | rider | B92 `xor-swizzle-smem-static-index-tables` confirmation — same mechanism, attention-kernel family instead of GEMM. |
| 7 | MoE weighted expert reduction fused + specdec extension | `3466812` #25952 + `41ef91f` #27621 | rider | B98 `moe-routed-down-sum-single-accumulate-pass` confirmation; #27621 ("earlier MOE glu fusion and topk-router fusion were restricted to 1 token") is a regime-extension data point for B98's decode-only precondition. |
| 8 | five-clause `use_mma` eligibility gate | ops.cpp use_mma | rider | B46 `size-gate-covers-every-cost-factor` confirmation — the gate names hw capability, alignment, AND the head-dim specialization ("mma kernel is specialized for the Mamba-2 head dim"). |
| 9 | cross-simdgroup state-buffer handoff barriers | ssm.metal:452, 470-471 | rider | B57-adjacent instance: `mem_device\|mem_threadgroup` barriers BOTH sides of the `s_buff` reuse ("All simdgroups must finish reading s_buff before any thread overwrites it"). |
| 10 | capability+shape route with sequential-tail fallback | ops.cpp dispatch tail | rider | B110 row-parallelism-ladder / B105 capability-route instance (kernel-FAMILY fallback, not a parameter change). |
| 11 | lazy tensor reading (`--lazy-mode`) | `fac889f` #27794, `bebc935` #27969, `2578138` #27837, `b19cbe9` #27869 | rider | B76 `eager-pointer-collect-lazy-fetch` confirmation at model-load scale (an entire feature = the rule's pattern). |
| 12 | view-alias + alloc deps in graph_optimize | `b387ddf` #27812 + `57291f2` #27301 | rider | B105 `hazard-tracked-aliasing-scratch-allocator` real-world bug instance (missing view-alias dependencies = the exact hazard the rule tracks). |
| 13 | Metal 4.0 tensor API gated M5+/A19+ | `d5d993a` #27461 | rider | B105 `cooperative-tensor-mma-capability-route` confirmation (capability probe + fallback tier). |
| 14 | f16 KV in-place bind (oneDNN SDPA) + SYCL TILE quantized KV decode | `be87620` #27468 + `d077b4c` #26689 | rider | B105 `tensor-op-strided-device-operand-binding` + B77 twins instances. |
| 15 | MMVQ→MMQ crossover SM87 / Strix Halo mat-vec rows / SYCL Q4_K multi-column share | `8c1a251` #28285 + `2cdae80` #27909 + `4aa6ffb` #27062 | rider | B104 swept tables + B77 twins + B110 `m-group-shared-weight-scan-register-relief` instances. |
| 16 | quantizer row-slab stream (anti thread starvation) | `1b89a43` #27830 | rider | B65 `channel-send-batching-unpark` instance at the loader (chunked producer so the quantize worker pool never idles). |
| 17 | qwen4exp recurrent rollback + snap_reserve | `0eadefe` #28123 + ops.cpp | rider | B71 rotated-ring-undo / B58 write-before-read family confirmation (recurrent-state rollback interop constrains the MMA path's tail). |
| 18 | DFlash encoder fused into KV injection + non-fused GDN/LID disabled | `662a0b0` #27310 + `8663224` #27877 | rider | B22 `fused-activation-quantize-for-gemv-consumer` / B19 fusion-family confirmations (upstream DISABLES the non-fused path — the fused-path-preference posture). |
| 19 | mm_ids_helper for any n_expert_used + KleidiAI init off dispatch | `f1793c1` #27978 + `b81c99b` #27891 | rider | B23 hardcoded-fast-path + B38 env/init-once instances. |
| 20 | non-pow2 Kronecker FWHT (384/640/768/1280) | `1f3d318` #28016 + `c845263` #28184 (**REVERTED**) | C | DROP as wisdom; optionally mine the REVERT REASON at T1 (a reverted mechanism is regime evidence, not a rule). B96's FWHT rule stays power-of-2. |
| 21 | n-gram position index / kv-cells early-stop / non-contiguous KV restore | `b356fa2` #28040 + `62acc89` #28011 + `2d8d612` #27991 | drop | Thin host-side instances of existing rust_perf scan/index rules + B94 coalesce. |
| 22 | AVX2 IQ large-batch + OpenCL/HIP/Hexagon/WebGPU platform items | `85c5522`, `95ef7fc`, `010be96`, `c841aee`, `43d87ff`, `0b5be7e`, `e422148`, `960dffa`, `f027c4f`, `8e53fce`, … | drop | No bounded Rust fix-space / no consumer surface. (`e422148` HIP Q2_0 gfx1201 is the B96 HIP-rider family — rider note only.) |
| 23 | server / UI / mtmd / API / CI / model-loaders | misc | drop | Not kernel/perf patterns. |

## Dedup-gate evidence

- Grepped `riir-clippy/src/domains/kernel_opt/**` for `radix|swizzle|xor|fwht|hadamard|fastdiv` → hits: `xor-swizzle-smem-static-index-tables` (B92), `vectorized-smem-write-no-bank-padding` (B79), `fwht-shuffle-xor-butterfly-register-resident` (B96), `byte-perm-lut-via-permutation-immediate` (radix mention). NO shader-division rule exists → candidate 3 passes (adjacencies B59 + `strength-reduce-div-pow2` declared above).
- Grepped for `moe-routed|down-sum|expert-major|wide-row|row-dot-split|ssd|mamba|lightning|indexer|sparse-fa` → hits: `moe-routed-down-sum-single-accumulate-pass` (B98), `wide-row-copy-tile-rows-across-workgroups` (B96), `row-dot-split-inner-axis-block-reduce` (B44), `first-order-affine-recurrence-associative-scan` (B79 — its regime note NAMES "chunking" for matmul-combine recurrences but no chunk-MMA rule exists), `whole-sequence-zero-barrier-single-dispatch-scan` (B95), no SSD/Mamba/sparse-FA rules → candidates 1/2/4 pass at the rule level; adjacencies to declare at authoring: B79 (strategy ladder — the new rule is the chunk family's MMA member), B95 (the comparator/ethos), B110 (per-chunk Gram reuse), B46 (eligibility), B57 (barrier windows).
- Grepped `lazy|eager-pointer|starvation|slab|coalesce|early-exit|alias` → B76 / B39 / B40 / B41 / B42 confirmations only — no new-rule collision.

## Yield estimate

After ~50% coordinator falsification: **2–4 rules** (the lead pair + fastdiv are strong; sparse-FA may collapse to a rider) **+ 12–15 riders** mapped into existing name-fields, + 1 league flag. Comparable shape to B104 (2 rules) with B111-style rider mass.

## Honest caveats

- Grades 1–3 verified by reading ssm.metal / ops.cpp / utils.glsl at the pin; grade 4 verified at grep/comment level; rider grades (5–19) are from the changelog + commit titles — **re-verify at mine time** (T1).
- The release notes span v0.3.0 → v0.4.0; B104's pins sit inside the window. **Do NOT re-mine the fa-vec infra** (B104 territory; table rows are data, not rules — the mechanism claim was settled by B104 and its post-pin prediction held: this window's fa-vec commits are rows for more SKUs: M1/M1 Max/M1 Ultra/M2/M2 Max/M2 Pro/M3/M3 Pro/M3 Ultra/M4/M4 Pro/M5/M5 Pro/A18 Pro + `8f83678` "remaining M3 Max").
- Platform-specific work (OpenCL/HIP/Hexagon/SYCL/WebGPU/Snapdragon) is dropped — no consumer surface in our stack; a few are rider-note material only.
- Sparse-FA and SSD-MMA are model-architecture-serving; the mineable content is the kernel idiom + the eligibility/dispatch shape, never the architecture.
- **LEAGUE FLAG (not mining):** upstream v0.4.0 completes the fa-vec tuning table for M3 Max (`8f83678` #28373 "add remaining fa-vec tunings for M3 Max") — our league fork pin `e311ed38f` does NOT carry fa-vec (upstream-only, per B104's record). A future opponent re-pin to upstream ≥ v0.4.0 moves the opponent's M3 prefill cells. Record in the league doc's opponent-watch (Plan 086 T11).
- `snap_reserve` (ops.cpp): the MMA path leaves `K` tokens for the sequential kernel's rollback snapshots — a spec-decode interop constraint on chunked fast paths (B71/B58 family); fold into rule 1's dispatch shape.

## Queue slot

After the time-blocked arxiv walk #8 (Mon 09-07 ~07:00 +07) or as the next CPU-only unit on a quiet box. One batch (113), plan-first: `.plans/086_llama_cpp_v040_delta_batch113.md`.
