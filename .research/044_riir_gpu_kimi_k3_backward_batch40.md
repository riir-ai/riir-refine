# Research 044 — riir-gpu `kimi_k3_gpu_backward.rs` mining (Batch 40)

**Date:** 2026-08-15
**Target:** `crates/riir-gpu/src/kimi_k3_gpu_backward.rs` (4,332 LOC — the
KimiK3 GPU backward; Issue 390 T6; the backward half of the Batch 39 forward)
**Method:** Protocol A — 2 parallel mining subagents (handle/build/update
infrastructure lane + gradient-compute lane), coordinator re-verification of
every quoted line (incl. the full RMSNorm dot algebra), corpus dedup greps
before landing.
**Corpus:** 303 → **310 rules** (A 60→64, D 53→54, H 13→14, N 17→18).

## The file

The largest file in riir-gpu (176 KB). The KimiK3 GPU backward pass — hybrid
CPU/GPU gradient computation mirroring the `gemma2_cubecl_train` pattern: GPU
takes the transposed GEMVs (`W^T @ grad` via pre-transposed weight handles +
`GemvAutotune::launch`); CPU takes the elementwise backward ops (SiTU
derivative, weight-gradient outer products). Four subsystem backwards (dense
FFN, LM head, MoE, MLA) + KDA. All derived weight handles are transposed on
host and uploaded once; per-step updates go through the `WeightBufferSlot`
raw-buffer cache (the B39 bootstrap pattern, correctly implemented ×4).

Structural inventory (the mining map):

- **Read helpers**: `read_handle` (L77, `#[allow(dead_code)]`, fresh-Vec form)
  re-exporting `gemv_batched::{launch_gemv, read_batched}` (Issue 396).
- **Dense FFN**: `DenseFfnBackwardHandles` (L94) + `transpose_f32` /
  `transpose_f32_into` (L131/L149, blocked-tiled + destination-passing) +
  `transpose_and_upload` (L177) + `build_dense_ffn_handles[_from_cpu]`
  (L201/L265) + `update_from_cpu_in_place` (L308) — the Phase 9f/9g/9j
  coalesced-scratch + rayon path.
- **LM head**: `LmHeadBackwardHandle` + `build_lm_head_handle[_from_cpu]` +
  `lm_head_backward_gpu` (L457) — production-dead (the sequence backward has
  its own 16×-cheaper CPU pass).
- **MoE**: `ExpertBackwardHandles` / `MoeBackwardHandles` + `update_in_place`
  ×2 + `build_moe_handles[_from_cpu]` (L1122/L1239) + `moe_backward_gpu`
  (L1378) + `expert_backward_into` (L1665) + `router_backward_cpu` (L1737).
- **MLA**: `MlaBackwardHandles` + `update_from_cpu_in_place` (L1884) +
  `build_mla_handles[_from_cpu]` + `mla_backward_gpu` (L2394 — the largest
  single fn, ~410 lines) + `compute_forward_matmul_cpu` (L2807) +
  `rmsnorm_backward_into` (L2823 — the CORRECT gamma-scoped form).
- **KDA**: `KdaBackwardHandles` + `build_kda_handles` + `kda_backward_gpu`
  (L3031) — production-dead (the sequence backward routes KDA through the CPU
  path).
- **Tests** (L3275+): five GPU-matches-CPU parity tests built on a shared LCG
  test-data generator (L3362) — the generator is broken (see Issue 693 H3).

## The 7 rules landed

| # | Rule | Cat | Source evidence |
|---|------|-----|-----------------|
| 1 | `host-scratch-mirrors-slot-order-for-coalesced-writes` | A | POSITIVE: L109-116 (dense `batch_scratch` doc — "holding ALL 3 transposed weights contiguously ... coalesce contiguous GPU buffer slots into the minimum number of queue.write_buffer calls") + L346-365 (split_at_mut carve → `data_slices` → `write_slots_batched`), L1963-1966/2033-2035 (MLA sizes to the SUM: `slot_lens.iter().sum()`). ANTI-instance in the same file: L973-986 (MoE latent path — `max_len` scratch + 2 sequential `write_in_place` calls that can never coalesce) |
| 2 | `write-plan-derived-from-cache-slots-not-recomputed` | A | L1907-1909 (boot pushes the `w_g` slot iff the HANDLE exists) vs L1952-1954 (fast-path `slot_lens` includes it iff handle AND source exist — the guards drift), L1961 (`debug_assert_eq!` — the ONLY tripwire, compiled out in release; downstream weight_buffer_cache.rs L290 is also debug-only). Release failure mode: shifted writes into WRONG buffers + tail OOB |
| 3 | `cpu-source-builder-skips-device-roundtrip` | A | L253-256 (the `from_cpu` doc: "skips the synchronous `client.read_one()` round-trip (one per weight matrix) that dominates the per-step ... cost"); the un-batched GPU-source twins: dense L225-237 (3 sequential `read_f32`), MoE L1153-1195 (~195 sequential syncs at the 63-expert config), MLA L2179-2225, KDA L2965-2994; trainer routes per-step through from_cpu (kimi_k3_train.rs L2320-2331) |
| 4 | `expert-loop-two-phase-split-batch-across-iterations` | H | L1682-1684 (`d_act` readback inside the per-expert helper) + L1707-1710 ("Issue 393: launch BOTH independent GEMVs, then read both in ONE batched sync" — within-iteration batching landed, cross-iteration did not); stage-A inputs loop-entry-ready: L1563-1564 (`d_expert_out` from `d_latent_prenorm`, computed pre-loop L1522-1542). 2+2·k_r+2 syncs → ~3 |
| 5 | `rmsnorm-backward-gamma-scope-correction-term` | N | WRONG (production MoE latent norm): L1537 `dx[i] = norm_w[i] * r * (d_latent_postnorm[i] - x_i * r2 * dot * inv_d_moe)` — gamma outside the parenthesis. RIGHT (same file): L2839 `dx[i] = r * (gamma[i] * d_y[i] - x_raw[i] * r2 * dot * inv_n)`. katgpt-attn mla_backward.rs L681-684 names the exact incorrect form. Coordinator verified the full algebra (the dot is correct via `dot_ydy / r` with `latent_postnorm = γ·x·r`; the ONLY error is the gamma scope) + the finite-difference counterexample (γ=(1,2): true dx₂=−0.5, slipped form −1.0) |
| 6 | `per-position-scalar-factored-gemv-collapse` | D | The CORRECT form ships twice: L2557-2559 (W_UV factoring) + L2596-2598/L2621-2623 (W_UK twin, consumer correctly re-applies ds). The FAILURE MODE is live: L2576-2584 — `let scaled = aw * base_uv[i]; d_ckv_j[i] += scaled; s += scaled * c_kv_j[i];` feeds the UNSCALED consumer (`d_attn_w`) from the scaled term (Issue 693 H1) |
| 7 | `foreign-accumulator-bridge-copy-hoist-to-outer-loop` | A | L3082 (`CpuKdaGradients::zeros_like` — fresh 14-Vec alloc per token) + L3085-3098 (fourteen `copy_from_slice` in) + L3256-3269 (fourteen out); the justifying comment L3079-3081 names FOUR fields while the code copies all fourteen both ways |

**Dropped (1):** agent A's rayon-parallel-transforms proposal — the
kernel_opt-specific facet (buffer-granularity unit + carved scratch) is thin
against the rust_perf generic rayon guidance; the missed MLA instance
(sequential transposes while dense rayons its 3 — L1984-2020) is recorded in
Issue 693 H11 instead.

**Dedup kills / falsifications (highlights):**

- The `update_from_cpu_in_place` ×4 family = correctly-implemented instance
  cluster of B39 `slow-path-bootstraps-raw-buffer-hotswap-cache` (order-keyed,
  two-tier, harvest-on-first-call) — NOT new rule material; the two facets
  layered on top (coalesced-write scratch #1, slot-plan derivation #2) are the
  only new content.
- `transpose_f32_into` is ALREADY blocked/tiled (L153-168, BLOCK=64) + the
  destination-passing form — B37's `tiled-host-transpose-before-quantize` and
  B39's dst-passing rules have no violation here.
- Per-head W_UK/W_UV slicing is structurally required (GEMV needs contiguous
  row-major per-head operands; a full-matrix transpose interleaves heads) —
  not a missed fusion.
- `situ_backward` exact-op parity CORRECT (standard tanh/sigmoid both sides;
  the `1/beta` from tanh′ cancels the outer `beta`); no
  saturating-recovery-from-cached-output hazard (derivative recomputed from
  saved RAW `gate_inter`).
- `rmsnorm_backward_into` exposes the norm statistic by PARAMETER (takes
  forward-saved `inv_rms`) — B34's statistic-recompute hazard does not fire
  on the MLA path (ironically the MoE latent path fails the same discipline:
  Issue 693 H10).
- No `transposed-consumption-accumulate-not-gather` violations (all CPU outer
  products stream row-major with the row scalar hoisted).
- Issue 393's within-iteration batching landed where claimed (gate+up pairs,
  cross-head base GEMVs, seq-wise W_KR/W_DKV) — the gaps are the
  cross-iteration (#4) and cross-family (Issue 693 H4/H5) dimensions.
- `f32::from_bytes(&bytes).to_vec()` in read helpers is ONE necessary copy,
  not two (`from_bytes` borrows).

## Bug yield — riir-ai Issue 693 (the headline)

Two CRITICAL wrong-math bugs on the production training path, both
coordinator-verified line-by-line + algebraically:

1. **MLA `d_attn_w` carries a spurious attention-weight factor** (L2576-2584 +
   the batched twin L1157-1161). For `attn_out = Σⱼ wⱼ·vⱼ`,
   `dL/dwⱼ = dot(d_o, vⱼ)` — the code computes `wⱼ·dot(d_o, vⱼ)` because the
   Issue-393 scalar-factoring refactor reused the aw-scaled `d_ckv` term for
   the unscaled consumer. Every attention-score gradient (→ d_q_c, d_q_r,
   d_k_r → W_UQ/W_QR/W_UK/W_KR grads) is wrong in BOTH backward variants.
   One-line fix ×2 files.
2. **MoE latent RMSNorm backward over-applies gamma to the correction term**
   (L1537 + batched L407 + the katgpt-rs CPU reference L627). The correct
   sibling ships in the SAME FILE (L2839) and the workspace's own
   `mla_backward.rs` NOTE names the exact incorrect form. Masked by γ init
   1.0±0.1 under the 1.5e-2 grad-check tolerance. Corrupts the entire routed
   half of every MoE layer's gradient. One-line fix ×3 files.
3. **The parity suite is NaN-vacuous** (L3362 LCG generator: `((rng >> 33) as
   u32) | 0x40000000` leaks random bits into the f32 EXPONENT field — the
   "[2,4)" comment is false; values span ~1e0..1e32 with periodic NaN), both
   max-statistics silently skip NaN (`if diff > max` / `f32::max`), so 4 of 5
   parity tests "pass" vacuously, and the KDA test FAILS with diff 4.95e27 —
   empirically observed by the mining agent (M3 Max Metal). The feature is in
   no CI gate. The two wrong-math bugs above were masked by exactly this
   blindness + the two-wrongs-agree parity structure.

Plus 15 more hazards (MLA slot-plan drift, production-dead KDA/LM-head
subsystems, MoE zip without length assert, MLA sequential transposes,
per-step CPU-copy clone storms, dead doc-links, doc-lies, KDA bridge
ping-pong, ...). Full inventory + a fix order in `.issues/693` (riir-ai).

## Generator / process notes

- The dry-run anchor verification (B39's new standing step) caught ONE bad
  anchor before the real run (the header anchor text mismatch) — second
  consecutive clean landing.
- **New pruner root-cause fix**: `parse_rust`'s wrap strategy was
  `format!("fn _wrap() {{ {code} }}")` — a snippet ENDING IN A LINE COMMENT
  glues the closing `}` into the comment text → the wrapped fn never closes →
  false parse failure. Batch 40's rule 7 (statements-only + trailing comment)
  was the first corpus entry to hit it. Fixed by adding boundary newlines
  (`fn _wrap() {{\n{code}\n}}`) — strictly-more-permissive, no behavior change
  for well-formed snippets. This is the L2 substrate hardening class: the
  corpus found a pruner bug, not the other way round.
- doc_markdown gotcha list grows: `2+2k_r`-style math expressions with
  subscript underscores need backticks in `//!` prose (the `k_r` token reads
  as an identifier to clippy).

## GOAT

- **G1**: `batch40_pseudocode_parses` PASS (7/7 after the pruner fix).
- **G2**: 336 kernel_heal + 201 default lib tests PASS.
- **G3**: clippy 0 warnings in 4 states (default / kernel_heal /
  kernel_heal+latent_retrieval / rust_perf).
- Count pins: all 20+ sites in sync at 310 (the generator updated corpus.rs ×19
  + router.rs ×4; enforced by the passing count tests).
