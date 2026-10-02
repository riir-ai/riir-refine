# Research 052 — riir-gpu hypernet second half (Batch 47)

**Date:** 2026-08-15 (evening)
**Cluster:** `hypernet/` second half — `shine.rs` (402) + `routing.rs` (268) + `types.rs` (269) + `m2p_transformer.rs` (611) + `gpu_m2p.rs` (568) + `kernels/m2p_attention.wgsl` (173). Completes the hypernet module alongside Batch 45's meta-LoRA pair (`meta_lora.rs` + `train_meta_lora.rs` — sibling WIP during this batch, untouched; quotes drawn only from clean files).
**Method:** 2 parallel subagents (Lane A: the M2P compute pair + WGSL; Lane B: shine/routing/types) + coordinator re-verification of every quoted line (all Lane A/B quotes personally re-read in the clean files; the bench parity section read directly).
**Consumers:** bench-only (`alternating_2d_bench.rs`) + `rim_routing.rs` doc references + tests — **zero production wiring**; all hazards latent-until-adopted (Issue 700 precedent).

## Rules landed (corpus 340 → 343)

| rule_id | cat | one-line |
|---|---|---|
| `per-key-projection-hoist-out-of-query-sweep` | B | K/V projections re-derived inside the query loop multiply projection work by seq (O(seq·H²) → O(seq²·H²)); the reference twin (`m2p_transformer.rs` `self_attention` L313-323) projects ALL QKV once at sweep entry |
| `multi-head-out-proj-consumes-head-concat` | E | the attention out-projection O[d] = Σₖ attn_out_full[k]·wo[d][k] needs the head CONCAT; a per-head thread degenerates to scalar-times-rowsum (`attn_val × Σₖₖ wo[d_out·h+kk]`), masked by identity-weight fixtures |
| `shared-producer-prefix-extract-once-per-input` | D | two sibling methods sharing an expensive pure producer (`extract_memory_states` — a full frozen-backbone forward) pay it twice under one composed call (`context_to_lora_with_span_width` L229-230); extract once, thread the artifact via `*_from_grid` variants |

Counts: A 79, **B 51**, C 33, **D 61**, **E 20**, F 10, G 19, H 14, I 4, J 5, K 5, L 11, M 9, N 18, O 4.

## The wrong-math find (the class that matters)

**H-C — the output projection is WRONG in both WGSL kernels AND the CPU fallback.** `m2p_attention.wgsl` L92-98/L167-172 + `cpu_m2p_attention` L134-141 compute `proj_val = Σ_kk attn_val · wo[d_out·h+kk]` where `attn_val` is INVARIANT in `kk` — that is `attn_val × rowsum(wo_row)`, not the matvec. The CPU twin adds a second bug (`let wo_row = d_out;` — head-local index, so heads 1..n re-read wo rows [0, head_dim)). The correct form ships in the sibling file (`m2p_transformer.rs` L371-381, sums the full attn_out row). Masking: the identity-weight fixtures give every row rowsum = 1.0 (all 3 tests pass vacuously), and the bench's "Correctness check" (`alternating_2d_bench.rs` L515-553) compares cpu-fallback vs WGSL — **both wrong the same way** — printing the cosine similarity with NO assert. Rule 2 (`multi-head-out-proj-consumes-head-concat`) is the design rule that would have prevented the class; Issue 701 H-C is the live instance.

## Dedup / falsification ledger (~17 dropped)

| candidate | verdict |
|---|---|
| `@workgroup_size(1,1,1)` + `dispatch(seq, heads, 1)` — 288 single-thread workgroups, ~155M serial MACs/thread at default | DEDUP composite: B44 row-dot-split (split the reduction axis) + B23 fold-query-tokens (occupancy) + B28 workgroup-equals-plane (sizing). The literal is a detection heuristic, not a decision. |
| `gpu_dispatch` re-uploads wqkv (3H² = 201 MB) + wo (H² = 67 MB) + fresh bind group + submit + download PER column/row — M round-trips/layer (~114 TB weight uploads per row layer at default M) | DEDUP composite: B33 immutable-operand-upload + B28 token-flattened collapse (the dead `grid_stride` is the fossil of the intended fix) + B26 batched-downloads + B40 loop serialization |
| the `rows*num_heads < threshold` gate (L307/L356) | DEDUP — B46's pre-recorded 2nd instance |
| per-(pos,head) `vec![0.0; seq_len]` scores; per-column `vec![0.0; rows*h]`; per-position `to_vec()` + gate/up/down Vecs | DEDUP — B45 scratch-depth instances |
| sinusoidal `powf` freq per (i,d) (m2p_transformer.rs L108/L118) | DEDUP — cos/sin-table 3rd-strike (dropped B17 + B36-F4 + B37-F1) |
| routing: pre-normalize expert embeddings at registration; query norm argmax-invariant | DEDUP — `norm-decoupled-low-bit-scoring` (+ B17 kin) |
| routing: `find_best_match` first-max ties | judged fine (determinism wanted; B32c is exploration-flavored) |
| shine: `GeneratedLoRA` combined-layer metadata contradicts payload | DEDUP rule-wise — `derive-shape-from-payload-not-cached-field` (B32b); hazard H-M |
| shine: `flat_map(\|w\| w.a.clone())` per-module clones | DEDUP — avoid-clone family; hazard H-M secondary |
| shine: loop-invariant `total_params` per layer | THIN — dropped (7-int sum ×36; hazard note only) |
| shine: unconditional `vec![0.0; total_params]` + copy when `flat.len() >= total_params` | THIN — folded into H-M |
| the 251 GB default config | HAZARD, not rule — Issue 700 precedent (maxsim 800K doc example) |
| empty-registry `(0, -inf)` sentinel | HAZARD — B43 empty-slice-sentinel family instance |
| gate/up sharing the `hidden` walk (forward twin) | DEDUP-adjacent — B34 backward-specific twin; CPU reference, dropped |
| per-thread weight re-reads, no smem staging | subsumed — Rule 1's hoist removes the weight from the inner loop; B35's gate says don't stage at reuse=1 (fix the workgroup shape first) |
| the two WGSL kernels byte-identical modulo names | anti-instance of B39 entrypoint-variants-share-layer-body; hazard H-G |

## Hazard ledger (filed as riir-ai Issue 701)

- **H-A** release-mode stack OOB on the DEFAULT path: `cpu_m2p_attention` L83-91 `[f32; 128]` + `debug_assert!(head_dim <= 128)` — default `head_dim = 4096/8 = 512` → 384 floats past the array; default column attention (288 < 512 threshold) takes this path.
- **H-B** WGSL fixed arrays under unbounded runtime bounds: `var q: array<f32, 128>` (hd=512 default) + `var scores: array<f32, 256>` (row attention seq = M ≥ 416 for any non-toy config; default 425,984 → ~1.7 MB past a 1 KB private array per thread).
- **H-C** the wrong out-projection math (above) + no parity test vs `M2PTransformer` (the only "parity" is print-only and compares the two wrong arms).
- **H-D/H-E** B45 anti-instance (`scores[t]/sum_exp` in the V loop) + depth instances (dedup, noted).
- **H-F** vestigial `grid_stride` uniform (set 0, never read — the fossil of the intended batched grid-strided dispatch).
- **H-G** the two WGSL kernels byte-identical modulo variable names (one entry + a stride param would halve the surface).
- **H-H** row-attention default dispatch `dispatch_workgroups(425_984, 8, 1)` exceeds wgpu's default `max_compute_workgroups_per_dimension` (65,535) → validation failure on most backends (adapter-dependent).
- **H-I** no test compiles either WGSL kernel or `gpu_dispatch` (5 unit tests cover only the CPU fallback + constants; the kernels are reachable only via the ungated bench); `cpu_m2p_attention`'s "Mirrors WGSL exactly" doc is accurate — it faithfully mirrors the wrong math.
- **H-J** default config allocation-infeasible + TRIPLE-inconsistent provision: `compute_memory_length` M = 13·r·H = 425,984 (test-pinned!) → grid 36×425,984×4096×4B ≈ **251.3 GB** + `MemoryEmbeddings::random` 6.98 GB + `block_size` 524,288; the constant is 13H² (formula) vs ~13.33H² (doc) vs **19Hr** (`LoRAModule::dims` actual: intermediate = (4·H·2)/3 = 8H/3) → at default, 99.96% of the extracted grid is discarded (1,744,830,464 elements available vs 622,576 consumed).
- **H-K** silent zero-padding produces structurally-dead modules at toy scale: `usable = min(flat.len(), total_params)` clamps — at the test config (H=16, r=2, M=4) flat=64 < total=604, Q consumes ALL 64 real values, K/V/O/gate/up/down get all-zero A and B (identically-zero LoRA deltas), undetectable by the existing tests (zeros are finite; Q's weights differ across contexts).
- **H-L** empty registry returns `(0, -inf)` (a valid-looking id pointing at nothing); `register()` never validates `embedding.len() == hidden_dim` (panics later inside `cosine_similarity`, far from the site).
- **H-M** combined-layer metadata contradicts payload: `a: flat_map(|w| w.a.clone()).collect()` concatenates 7 modules' A matrices (different I dims; down is 8H/3) labeled `in_dim=hidden_dim` — 294,912 elements at default claiming I·r = 32,768 (**9× off**); module boundaries invisible; plus double materialization (7 `to_vec` then 14 clones).
- **(new, minor)** `estimate_difficulty`'s hand-rolled sigmoid (Issue-684 family).

## GOAT

- **G1** parse/syn: `batch47_pseudocode_parses` — 3/3 rules' ref_shape + opt_pattern parse ✓ (inside the suite).
- **G2** lib tests: **357 kernel_opt** (354 + 3 new) + **201 default** — all pass ✓.
- **G3** no-regression: clippy **0 warnings in both feature states** (10 doc warnings fixed pre-commit: my line-initial `+` RECURRED — the B37 class, 4th occurrence — + `head_dim` backtick + 7 pre-existing `KERNEL_CATEGORIES` slice-header backticks left by the B46 split).
- **G4** n/a (corpus data batch).

## Generator lessons (4th/5th occurrence logging)

- The line-initial `+ ` in `//!` bullet continuations struck AGAIN (the `+ the triple-inconsistent` line) — this is now a 4-time recurred class; the generator template should lint its own output for `^//!\s*\+ ` before write.
- 7 pre-existing `KERNEL_CATEGORIES` doc-markdown warnings surfaced from the B46 split's slice headers — the split commit's "clippy 0" claim was scoped to its own diff; a whole-crate clippy run after ANY corpus-layout change is the standing check.

## Next

- Queue: Batch 48 — the newly-unblocked cudarc cluster was RE-BLOCKED mid-batch (sibling WIP on `ternary_deltanet_gpu_forward_cudarc.rs` + `cudarc_kernels/mod.rs` again, mtime Aug 15 18:38). Verify before mining.
- `kimi_k3_gpu_backward_batched.rs` (1,394) remains LOW value (shares Issue 693's H1/H2 + broken LCG).
