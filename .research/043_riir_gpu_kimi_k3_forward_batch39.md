# Research 043 — riir-gpu `kimi_k3_gpu_forward.rs` mining (Batch 39)

**Date:** 2026-08-15
**Target:** `crates/riir-gpu/src/kimi_k3_gpu_forward.rs` (2,601 LOC — the KimiK3
full-model GPU decode forward; Issue 390 T3 / Plan 318 C13)
**Method:** Protocol A — 2 parallel mining subagents (dispatch/variants/hot-swap
lane + memory/readback lane), coordinator re-verification of every quoted line,
corpus dedup greps before landing.
**Corpus:** 294 → **303 rules** (A 55→60, C 31→32, D 51→53, K 4→5).

## The file

The largest single unmined file in riir-gpu. The KimiK3 full-model GPU forward:
composes the per-layer kernels (`GpuMlaCubeCL`, `GpuMoeCubeCL`, `GpuKdaCubeCL`)
into a full decoder stack. Hybrid CPU/GPU — GPU takes all GEMVs (MLA/KDA
projections, MoE router + experts, dense FFN, LM head); CPU takes elementwise
(attn-res mixing, prefix-sum, RMSNorm, softmax, SiTU, ShortConv1D, SSM
recurrence, RoPE). Sequential per layer; sync points at each GEMV boundary.

Structural inventory (the mining map):

- **7 forward entry-point variants** (evolutionary accumulation):
  `forward_token` (L528, naive per-GEMV readback) → `forward_token_batched`
  (L578, Issue 396) → `forward_token_batched_v2` (L627, Issue 398 GPU GEMM
  K/V) → `forward_token_fused_final_norm` (L704, Issue 401 Ph 1.1) →
  `forward_token_gpu_resident` (L857, Issue 401 Ph 1.2, feature-gated) →
  `forward_token_saved` (L1788, T6.6) → `forward_token_hidden_saved` (L2091,
  Plan 324 Ouro loop).
- **4 layer-loop bodies**: `forward_layer_gpu` (L1100) / `_batched` (L1212) /
  `_batched_v2` (L1324) / `_gpu_resident` (L937, Handle-in-Handle-out).
- **Dual weight-update paths**: `update_weights` (L1473, allocate-and-replace,
  = B25's hot-swap instance) + `update_weights_in_place` (L1579, Issue 402
  Phase 9 raw-buffer slot cache).
- **MLA KV cache rewind** for the Ouro looped forward (L2068).
- **4 dense SiTU FFN helpers** (L2316 gpu / L2381 gpu_batched / L2449
  gpu_fused / L2513 gpu_fused_with_saved) — standalone free fns extracted to
  avoid borrow-checker conflicts with `&mut self.ffn_layers` (L2308-2315).
- Cross-file substrate: `gemv_batched.rs` (`launch_gemv` / `launch_gemv_handle`
  / `read_batched` / `read_handle`).

## The 9 rules landed

| # | Rule | Cat | Source evidence |
|---|------|-----|-----------------|
| 1 | `autotune-cache-extract-outlives-owner` | K | L296-302 (training-loop ctor doc: re-bench ~5-10 min vs upload ~64 ms), L324-329 (Arc inject contract), L344-346/352-354/393-395 (3 kernel ctors share the Arc), L1735-1743 (`into_autotune`) |
| 2 | `append-cache-rewind-overwrite-in-place` | A | L2061-2074 (`rewind_mla_caches_to`: "the cache grows T× per token"; KDA no-op — heterogeneous-aware), L2084-2089 (caller contract) |
| 3 | `slow-path-bootstraps-raw-buffer-hotswap-cache` | A | L1551-1558 (two-tier doc), L1585-1627 (boot: slow path + harvest + `extract_buffer` lazy→eager), L1630-1653 (fast path `write_in_place` + order-mirroring contract) |
| 4 | `entrypoint-variants-share-layer-body` | D | L566-576/L619-627 (each variant quantifies sync delta + parity class) + the 2 observed failures: L977-992 (resident fork initially called OLD kernels — Issue 401 Ph 2.1 catch-up) + the saved-forward divergence (L1862-1896 OURO dispatch vs L2162-2177/L2220 unconditional) |
| 5 | `elementwise-offload-needs-device-side-anchor` | C | L969-972 + L1011-1014 (unanchored per-layer norm sandwich: upload → GPU norm → read back), L930-935 (the file's own "honest intermediate state" label) vs the anchored contrast L737-762 (fused final norm → LM head = 1 sync eliminated) + L2492 (`launch_gemv_handle` enabler) |
| 6 | `tiny-param-buffer-rebuild-over-cache` | A | L2437-2441 (doc: "3 f32 = 12 bytes — cheaper to rebuild than to cache a Handle"), L2473-2476 + L2539-2542 (both fused variants rebuild per call) |
| 7 | `readback-destination-passing-free-fn` | A | L807-828 (`read_handle_into` — the GOOD dst-passing free fn, feature-gated) vs the naive shapes still live: L505-523 (`dispatch_gemv` `.to_vec()`), L613-614/662-663/763-764/914-915 (vocab logits fresh-Vec + copy), gemv_batched.rs L96-111 (`read_batched`/`read_handle`) |
| 8 | `variant-owned-output-buffer-clone-escape` | A | 6 clone sites (L1069/1192/1304/1416/1979/2240) + 4 fresh-Vec sites (L1072/1195/1307/1419); consumer forcing escape L1201-1204; the file's half-fix L2308-2315 (free fns extracted but out_buf still variant-owned → clones persist) |
| 9 | `pipeline-layer-boundary-device-handle` | D | L937 (Handle-in → Handle-out signature), L871-874 (loop threads the handle), L944-945/L1086-1087 (internal read/upload — host materialization is a layer-internal detail), L1018-1024 (the payoff: fused FFN landed with ZERO boundary changes) |

**Merged-from-both-agents rules (2):** #5 (found independently by both agents —
the convergent discovery) and #8 (agent 1 proposed the free-fn borrow split,
agent 2 the ownership placement; coordinator merged — the free-fn extraction
alone does NOT fix the clone because `out_buf` stays variant-owned; the two
mechanisms are inseparable halves of one fix).

## Falsifications + dedup kills (13 total)

| Candidate | Killed by | Evidence |
|---|---|---|
| `update_weights` as new hot-swap rule | B25 `hot-swap-weights-preserve-autotune-caches` | L1460-1467 doc = the B25 pattern verbatim |
| `OURO_*` per-layer env reads | B38 `env-flag-resolved-once-not-per-dispatch` (already cites this file) | L1862/1869/1892/1942 — up to 3 `env::var` per layer per token |
| inv_rms double-compute (6 sites) | B34 `backward-helper-expose-shared-norm-statistic` (forward-side instance) | L1848-1850, L1917-1918, L2032-2033, L2157-2158, L2198-2199, L2293-2294 — `simd_sum_sq` for the save, then `rmsnorm_with_gamma_eps` recomputes internally |
| 4-buffer single batch drain in `_fused_with_saved` | B36 `activation-checkpoint-as-handle-clone-deferred-batch-readback` + B26 `batched-downloads-single-sync-cycle` | L2549-2550 (`gate_h.clone()` into SiTU = the handle-clone medium), L2560-2561 |
| Hybrid per-layer CPU/GPU structure | B34 `hybrid-cpu-elementwise-gpu-throughput-sync-points` + B35 `kernelize-host-chain-after-sync-batching-floor` | L14-17 module doc |
| gate+up shared-input fan-out | B23 `fan-out-shared-input-single-sync` — the batched-READ half is applied; the upload half is MISSED (see H12) | L2400-2405 + gemv_batched.rs L49 |
| SAFETY size-contract comments | B29 `launch-unchecked-caller-owned-size-contract` | L752, L793, L2478 |
| `launch_gemv` vs `launch_gemv_handle` dual entry | B34 `split-dispatch-expose-fused-intermediate` | gemv_batched.rs L41-51 vs L65-86 |
| Option→[value,flag] params encoding | `optional-bias-branch` + B29 `pack-scalar-params-into-combined-buffer` | L2473-2476 |
| saved-forward `.clone()` captures as pure waste | FALSE — they're checkpoint captures (CPU values overwritten downstream); the B34 preallocated-slots INSTANCE applies, not waste | L1816-1837, L1992-2010, L2253-2270 |
| `read_batched(vec![lm_head_h]).remove(0)` single-item batching | FALSE — the LM head is the token's LAST op; nothing to batch with; one reader path | L611 |
| `fill(0.0)` vs loop | clippy-level, too thin for the corpus | L964-966 |
| "gpu_resident keeps hidden on GPU" | FALSE at data level (every layer reads back L945 + re-uploads L1087) — TRUE at signature level → became rule #9 | L837-841 (doc is honest) |

## Bug yield — filed as riir-ai Issue 692 (14 hazards)

Headline items (full list in the issue):

- **H2**: dead ~640 MB `embed_handle` uploaded at construction (L415) + retained
  forever — the Issue 402 Phase 9 comment (L1536-1540) proves the team knows
  it's dead ("Uploading 640MB... was pure waste") but that fix only removed the
  PER-STEP re-upload; the constructor still pays once + holds the VRAM.
- **H3**: per-layer-per-token clone/fresh-alloc in every FFN dispatch arm (6 + 4
  sites) — the enum-variant borrow escape (rule #8's own instance) across ALL
  forward variants. G4 alloc-free violation class.
- **H4**: the two saved forwards diverged (~310 duplicated lines):
  `forward_token_saved` has the OURO 3-way env dispatch; its clone
  `forward_token_hidden_saved` lost it (unconditional batched) — silent
  behavioral divergence in kernel selection under the same env vars.
- **H1**: contradictory sync-count docs — L573-575 ("7→3 MLA, 8→3 KDA") vs
  L1210-1211 ("3→1 MLA/KDA") about the same batched kernels.
- **H12**: gate+up duplicated GPU upload (doc claims "sharing input h" but
  `launch_gemv` uploads per call — the missed upload-half of B23).
- **H13**: vocab-sized logits double-copy per token (fresh Vec + copy, 4+ sites).
- + H5 (doc UAF language), H6 (order-keyed cache drift-fragility), H7
  (ungated `cpu_expert` CPU weight duplication), H8 (dead cfg(not) branches),
  H9 (panic in hot path), H10 (saved-forward alloc storm), H11 (inv_rms ×6),
  H14 (unconditional `Instant::now()` per layer).

## Process notes

1. **Two independent convergences in one batch** — both agents found the
   sandwiched-offload rule (#5) and two halves of the clone-escape rule (#8).
   The coordinator merge for #8 required reading the actual borrow structure:
   the file's existing free-fn extraction (L2308-2315) did NOT fix the clones —
   proving the free-fn form alone is insufficient; ownership placement is the
   load-bearing half. Encoded as one rule with both mechanisms.
2. **The generator anchor discipline held**: 25 anchors verified in a dry-run
   BEFORE the real run (all unique), one post-run fix needed (a concept
   substring asserted lowercase while the text had caps — the concepts_covered
   test caught it immediately). The `ensure_ascii=False` rule now standing.
3. 4 doc_markdown backtick fixes pre-commit (`KimiK3`, `SiTU`, `O(max_seq)`,
   `O(iters × max_seq)` — the O() expressions are a new gotcha variant).
4. The variants have real callers (tests/benches + the Ouro loop path) —
   unlike Batch 38's zero-caller backward. No dead-file headline this time.

## GOAT

- **G1**: `batch39_pseudocode_parses` — all 9 rules' ref_shape + opt_pattern
  syn-parse. PASS.
- **G2**: 333 kernel_heal (330 + 3 new) + 201 default lib tests. PASS.
- **G3**: clippy 0 warnings in all 4 states (kernel_heal/default ×
  lib/all-targets). PASS.
- Orphan `rule_id:` scan: 0/303. All count-pin sites in sync at 303 (the
  count asserts are enforced by the passing test suite).

## Next targets

Batch 40 candidates: the remaining `kimi_k3_gpu_backward.rs` (4,332 LOC — the
largest file in the crate) + `kimi_k3_gpu_backward_batched.rs` (1,394) +
`kimi_k3_gpu_backward_sequence.rs` (1,328) — the backward cluster this file's
saved activations feed; `gemma2_d2f*` (mod 1,579 + _sc 800); `weaver_gpu*`
(dflash 576 + corrector 1,449); `kvarn/`; `hypernet/`; `maxsim/`.
`ternary_deltanet_gpu_forward.rs` (+ `_cudarc.rs`) still BLOCKED on sibling WIP.
