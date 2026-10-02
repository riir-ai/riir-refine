# Research 033 — riir-gpu Batch 31: the training-substrate cluster (lora + optimizer + dataloader + buffer)

**Date:** 2026-08-15
**Batch:** 31 (corpus 209 → **217**)
**Sources:** `riir-ai/crates/riir-gpu/src/lora/mod.rs` (1512 lines) + `optimizer.rs` (1712) + `dataloader.rs` (1166) + `buffer.rs` (687) — the substrate UNDER the dllm training loop mined in Batch 30. Two parallel mining subagents (A: lora+optimizer, B: dataloader+buffer) + synthesizer re-verification of every load-bearing quote.

## The headline: a CRITICAL production bug, not just rules

`AdamWOptimizer::step()` (optimizer.rs L291-365) wrote each dispatch's `param_count` to ONE shared `uniform_adamw` via `queue.write_buffer`, then recorded all A/B compute passes into a single submit. Under wgpu queue semantics **every recorded pass sees the LAST write's `param_count`** — the codebase's own documented hazard (`backward/dispatch.rs` L65-69, Issues 366/368/671), and the shader bounds each dispatch with `if (i >= opts.param_count) { return; }` (`optimizer.wgsl` L23). With the standard target order `[Q,K,V,O,Mlp1,Mlp2]` the final write is Mlp2's `b_count = n_embd·rank`, so **Mlp1.B and Mlp2.A (`mlp_hidden·rank`) silently trained only their first `n_embd/mlp_hidden` fraction — 25% at the usual 4× SwiGLU ratio**; params AND moments frozen for the rest. Attention adapters (equal dims) were unaffected — training still converged, which is why the bug survived every gate. `step_ia3` carries the same defect (per-target `adapter.dim` differs); `step_moa` benign by luck (equal counts); `step_domain_latent` single-dispatch.

**FIXED as riir-ai Issue 679A** (commit `acc7a0392`): per-slot uniform buffers (`uniform_slots`, grow-only 64 B pool; every bind group binds its OWN slot; `dispatch_adamw` takes `uniform_slot`). Regression test asserts ZERO frozen elements in Mlp1.B/Mlp2.A — verified to fail under the old aliasing with exactly `96/128 never updated`. Not a new corpus rule (the fix pattern IS Batch 26's `per-slot-uniform-buffer-write-ordering` — this is its highest-impact confirmation).

## Rules landed (8)

| Rule | Cat | Source | Concept |
|---|---|---|---|
| `ragged-pack-offset-table-fused-dispatch` | D | lora/mod.rs L308-325 + adapter_merge.rs L775-884 | 12 per-adapter GEMV dispatches → packed contiguous buffers + prefix-sum offset table, ONE launch over Σ out_dim. Ragged segments need offsets — `i % n` cannot identify them. |
| `twin-projection-shared-delta-adapter` | L | lora/mod.rs L84-92, L488-517 | GQA K/V shape-twins share ONE adapter pair: adapters 6→5/layer, params −33%, KV delta cache halves at inference. |
| `min-rank-inert-adapter-zero-alpha` | M | lora/mod.rs L218-219, L666-689 | Structurally-required-but-inert adapters: rank=1 (zero-size buffers fail validation) + alpha=0 (annihilated by scale, not contents). Distinct from Batch 30's VALUE-contract rule (B=0). |
| `debug-only-gpu-label-macro` | M | optimizer.rs L17-28 | cfg-split label macro: debug = format!'d capture labels, release = typed `Option::<String>::None`. Zero release alloc/format/retention. |
| `sync-map-result-stack-cell` | A | buffer.rs L9-45, L159-164 | `poll(Wait)` IS the barrier → the map callback parks its result in `Arc<UnsafeCell<Option<_>>>` (scoped `unsafe impl Sync`) instead of `Arc<Mutex<...>>`. Per-sync lock+alloc removed from the readback critical path. |
| `staging-reuse-grow-only-map-prefix` | A | buffer.rs L47-93, L219-220, L457-458 + optimizer.rs L81-83, L496-498 | Loop-owned grow-only MAP_READ staging (~50-500μs driver alloc → zero steady-state); map only the copied PREFIX; the same CPU buffer serves download + norm + re-upload. MERGED from two subagent proposals (A5 + B2 — same concept from the optimizer + buffer sides). |
| `mapped-at-creation-init-upload` | A | buffer.rs L95-112 | One-shot uploads via `create_buffer_init` (mapped memcpy): no encoder, no submit, no queue-timeline ordering hazard. The tell: the unused `_queue` param. |
| `dtype-normalize-once-at-upload` | N | buffer.rs L388-404 | bf16→f32 is a pure bit-shift — normalize ONCE at ingestion instead of per-dispatch forever. The host-side sibling of the in-kernel dequant hoist; complementary to the packed-weights regime. |

Category deltas: A 29→32, D 31→32, L 6→7, M 7→9, N 12→13. Total 217 across 15 categories.

## Bugs found (filed as riir-ai Issue 679, B1-B10 — all unfixed)

1. **B1** `ensure_lora_bg_cache` keyed on COUNT only — a same-count `GpuLoraBuffers` swap trains zombie buffers (the live lora never updates, no error). `invalidate_lora_bg_cache` has ZERO production callers.
2. **B2** `total_packed_size()` returns hardcoded `0` ("Actual measurement done in benchmark") — a public G5-comparison accessor that lies. Stale `#[allow(dead_code)]` masks live `packed_b` usage.
3. **B3** `GpuLoraPackedBuffers::new`: n_layer×12 GPU→host→GPU round trips with `.expect` panics inside a constructor (~384 blocking syncs for a 32-layer model) — device→device encoder copies would do it with zero host round trips.
4. **B4** `load_lora_from_bin` trusts the header `rank` over per-block `a_rows/a_cols` — a blake3-valid file with disagreeing fields makes `param_count()` disagree with buffer sizes (OOB dispatch or truncation).
5. **B5** `DataLoader::from_jsonl` drops malformed lines silently (both IO + parse errors) — a half-corrupt corpus trains on half the data with zero warning.
6. **B6** ragged final batch: `chunks(batch_size)` yields a partial tail the doc contract (`[batch_size * seq_len]`) and `num_batches` (floor) both disown.
7. **B7** padding tokens included in the loss on the rationalization "grad will be ~0" — false early in training; every pad position biases toward pad-emission.
8. **B8** dFlash draft conditions on `tokens[block_size-1]` but targets `tokens[..block_size]` — trained to predict tokens that PRECEDE-or-equal its anchor; the doc says the opposite.
9. **B9** adaptive-span widths not recoverable from `span_starts` (region tails discarded) + hardcoded `decay_rho = 0.9` vs the parameterized `build_mux_batch`.
10. **B10** `download_f32` discards the poll result; the failure path `take().unwrap()` panics far from the cause (device-lost → panic, not `GpuError`).

## Candidates dropped (dedup, with the covering rule)

- **Device→device repack instead of host round trip** (B3's fix) — triply covered (Issue 018 lineage; the source comment itself cites it). Filed as a bug, not a rule.
- **Per-slot uniforms / encoder-ordered staging copies** (the 679A fix) — `per-slot-uniform-buffer-write-ordering` + `staging-packed-uniforms-encoder-ordered-copies` (Batch 26/27).
- **Bind-group caching lazy build + incremental resume** (optimizer.rs L238-280) — `prebuilt-bind-groups-fixed-buffer-combos` (Batch 26); the hazard became B1.
- **One encoder + single submit per step** — `chain-dispatches-single-sync-point` (Batch 19).
- **clip_gradients contiguous multi-buffer download** — `batched-downloads-single-sync-cycle` (Batch 26; this file cites the same ~1.28 ms/sync measurement).
- **4-lane f64 reduction accumulator** — host math; `upcast-before-reduction` + `dual-accumulator-single-reduction-sweep`.
- **v2 export `cast_slice` vs v1 per-element `to_le_bytes`** — host serialization (rust_perf bulk-copy family).
- **NF4 two-nibbles-per-byte + branch-free 4-chunk LUT scan** — host quantization math (`int2-pack-weights` family / `branchless-select`).
- **`packed.clear()` capacity reuse** — rust_perf `reuse-allocation-clear`.
- **Eval-split derived seed** (dataloader.rs L99-113) — concept-covered by `counter-derived-seed-stateless-noise` (Batch 30): seed = pure function of structural params, never mutable RNG state.
- **N-downloads-one-sync offset packing** (buffer.rs L300-375) — `batched-downloads-single-sync-cycle`.
- **Device-side 4-byte scalar download** (`download_u32`) — `device-side-argmax-minimal-download` (Batch 25).
- **Empty-workset short-circuit** — `predicate-gated-skip-empty-workset-step` (Batch 30).
- **Fixed-shape pad/truncate flattening** — `varlen-packing` / `ragged-chunk-pad`.
- **`Vec<bool>` role mask bitpacking** — rust_perf `bool-to-bitpack` (cross-domain, excluded).
- **3 RePlaid controller patterns** (per-component LR plumbing) — out of kernel_opt scope (host math, no GPU axis).

## GOAT gate

- **G1 (parses):** `batch31_pseudocode_parses` — 8/8 rules' ref_shape + opt_pattern syn-parse. PASS.
- **G2 (tests):** kernel_opt lib suite 102 tests PASS (0.13s) incl. the new batch31 gates.
- **G3 (no-regression):** default `cargo check` + clippy clean; full test suite to run at close.
- **G4:** unchanged (no hot-path additions — corpus is static data).
- **G5 (corpus):** 217 flat → G5-drafter reduction maintained.

## State

`riir-gpu` mined: ~54 of ~200 files. Next candidates: `edge_lora/`, `kernels/` WGSL dispatch wrappers, `ternary_deltanet_gpu_forward.rs` (234K — blocked on the sibling's active WIP), `forward/` remnants.
