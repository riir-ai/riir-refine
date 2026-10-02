# Research 047 — riir-gpu weaver_gpu Batch 43: Weaver speculative-decode GPU corrector

**Date:** 2026-08-15
**Batch:** 43 (corpus 320 → 325; A 73, B 47, D 57)
**Targets:** `crates/riir-gpu/src/weaver_gpu_corrector.rs` (1,449 LOC — Plan 436
Phase 4 T4.1, `GpuWeaverCorrector` + `GpuWeaverBatchedScratch` + the per-depth
`correct_marginals` and batched `correct_marginals_batched` entry points) +
`crates/riir-gpu/src/weaver_gpu_dflash.rs` (576 LOC — T4.2,
`dflash_predict_with_weaver_gpu`, the DFlash draft + GPU correction orchestration).
**Bug yield:** riir-ai Issue 697 (17 hazards: the 3-layer silent no-correction chain,
the release-mode embedding OOB window, ~29 alloc requests/call, test-only reachability).
**Status:** the weaver orchestration pair is now MINED. The `weaver_gpu/` directory
(mod.rs ~62 KB + tests.rs ~57 KB — the 19-dispatch kernel implementations) is a
SEPARATE cluster, still unmined.

## What the cluster is

Weaver speculative decoding drafts with DFlash, then a small corrector network re-weights
the draft marginals over each step's top-K candidates. The GPU port (Plan 436 Phase 4):
CPU per-depth top-K selection → CPU packing of `raw_buf` (verifier row + drafter row) →
GPU upload → `full_forward_gpu` (19 dispatches: conditioning → QKV → attn → out → SwiGLU
→ gather → dot_per_row → softmax_k) → single batched readback (Issue 468 P0) → CPU
scatter + renormalize in-place. Two entry points: per-depth (the G1 CPU-parity reference —
depths CANNOT be batched into one causal forward without letting deeper queries attend
shallower drafter positions) and batched (seq_len=depth+1, the latency win, 1.01×–1.59×
vs CPU parallel, with documented cross-depth divergence).

The two paths sit side by side as a **native naive/opt contrast**: the per-depth path
allocates its inputs per iteration; the batched twin packs flat + uploads once. The
Issue 468 P0/P2 work already shipped several corpus rules' fixes — driving an unusually
high falsification count (23).

## The 5 rules

### 1. `mutable-input-scratch-residency-write-per-call` (A — Memory access)

A hot path with scratch-resident intermediates often still allocates per-call INPUTS
fresh — `create_from_slice` per loop-iteration makes the device allocator service a new
request every iteration while re-uploading a sub-row that never changes (the constant
verifier row). Fix: inputs get the same residency discipline — handles allocated once in
the scratch struct, contents written per call with `client.write(handle, bytes)`;
flat `[depth × K]` per-call buffers replace per-iteration `[K]` allocations when the
whole upload set is known pre-loop. Size honesty: below the tiny-param threshold the
rebuild form stays correct.

- Source: `weaver_gpu_corrector.rs` L461 (per-call `raw_flat`), L464-466 (verifier row
  re-packed EVERY depth), L468-477 (three `create_from_slice` per depth), L438-439 (two
  `vec![0.0f32; k]` per depth). Positive contrast in the same file: L686-687 + L729-737.
  Scratch precedent L168-189 (18 persisted intermediate handles).

### 2. `topk-select-nth-over-full-vocab-sort` (B — Compute)

Host-side top-K feeding a GPU upload materializes the full (id, score) pair collection
(16 B/entry) and fully sorts it — O(V log V) — then keeps K. When K << V (speculative
top-K over a 256k vocab with K=8-64), `select_nth_unstable_by` partitions in O(V) and
sorting only the K-prefix costs O(K log K); a u32 index buffer quarters the moved bytes.
Composes with `order-stable-topk-via-index-sort` (its content is the ordering contract;
both its forms fully sort — this rule is the selection cost). Parity caution: CPU and GPU
paths must run the SAME host selection or tie order shifts under a G3 gate.

- Source: `weaver_gpu_corrector.rs` L421-430 + duplicated L694-703; only `n_take ≤ K`
  consumed (L432-436). At V=256k/K=8: ~4.6M comparisons + 4 MB pair movement per depth.

### 3. `post-scatter-renorm-support-set-only` (A — Memory access)

After zero-fill + K-scatter, renormalizing sums the ENTIRE row (V-K exact zeros) and
rescales all V. Accumulate the sum DURING the scatter, rescale only the support.
**Bit-identical by IEEE**: +0.0 addition is exact and 0 × inv = +0.0 — no numerical
risk, unlike most reduction reshapes. The zero-fill stays (the in-place contract needs
it); only the read-back sweep and re-scale sweep are deleted. At V=256k, K=8, depth=4:
~2M memory ops → ~64.

- Source: `weaver_gpu_corrector.rs` L547-563 + byte-identical L797-812 (the duplication
  itself is Issue 697 H9).

### 4. `stack-array-row-view-bridge` (A — Memory access)

Bridging a flat capture buffer to a `&[&[f32]]` API: the naive form collects a
`Vec<&[f32]>` (heap per call — a live anti-instance in the weaver bench itself); the
idiom is a fixed `[&[f32]; 64]` stack array filled for `count = steps.min(64)`, passed
as `&arr[..count]` — 1 KB of stack writes, zero heap, zero copy. Recurs at 5+ in-tree
sites (katgpt-rs `build_marginals_view` returns the array by value as the shared pub
helper; rollback + qwen tree steps mirror it; the engine dflash wrapper inlines it).

- Source: `weaver_gpu_dflash.rs` L134-146 (positive); anti-instance
  `bench_weaver_gpu_436.rs` L499-500.

### 5. `empty-slice-sentinel-launders-shape-violation` (D — Dispatch)

A view builder guards each slice with `else { &[] }` — the empty row is TYPE-correct
but SHAPE-invalid, so it sails past the borrow checker into the consumer's validation,
which returns an `Err`... which an orchestration `let _ =` swallows. The live 3-layer
chain: undersized capture → sentinel row → `HiddenShape` Err → swallowed → pipeline
continues on UNCORRECTED marginals, zero log lines. Fix: delete the defensive branch
and slice hard — indexing IS the assertion — fronted with a `debug_assert!` carrying
`steps × row_len <= buf.len()`. The negative space of
`launch-unchecked-caller-owned-size-contract` (B29).

- Source: `weaver_gpu_dflash.rs` L140-144 → `weaver_gpu_corrector.rs` L636-644 →
  `weaver_gpu_dflash.rs` L247. Same sentinel in katgpt-rs `step_gdn_tree.rs` L56-60 +
  engine `dflash.rs` L239-243; the CPU swallow is inherited (L358).

## The 23 falsifications (the anti-pattern museum effect)

The Issue 468 P0/P2 work already shipped several rules' fixes, so most "obvious"
candidates were already done:

1. Batch the per-depth forwards → SHIPPED as `correct_marginals_batched` + documented
   divergence tradeoff + P2 test.
2. Collapse depth readbacks into one sync → SHIPPED (Issue 468 P0, `probs_offset =
   di * k`) — `flat-activation-slots-as-readback-target` applied, POSITIVE evidence.
3. Hoist CPU work out of the dispatch window → SHIPPED (Phase A pre-computes all
   depths' top-K, documented).
4. Cache embedding/weights on device → SHIPPED (`set_embedding` once, `GpuWeaverWeights::upload`
   in `new`; the 2 GB rationale in the module doc) — `immutable-operand-upload-at-set-creation`.
5. 31 handle clones per depth → FALSIFIED (Arc bumps; the file documents "cheap,
   Arc-backed").
6. readback destination-passing → FALSIFIED as rule-worthy (cubecl's `read_one` is an
   external API; 128 B–2 KB payloads).
7. NaN-unsafe comparator → FALSIFIED (finite filter before sort).
8. Unstable-sort tie order breaks parity → FALSIFIED (G3 asserts same vid SET + 1e-3).
9. `top_pairs.reserve(vocab_size)` per depth allocates → FALSIFIED (`clear()` retains).
10. Entry-point variants should share the body → covered by B39; filed as hazard H9.
11. `ln()` per candidate is a storm → FALSIFIED (K×depth = 32 transcendentals).
12. Same-queue ordering hazard on reused scratch → FALSIFIED (sequential single-client
    submit; `same-queue-implicit-ordering-no-sync` applied CORRECTLY).
13. Verifier row should fan-out-upload once → real, folded into Rule 1 (too close to
    `fan-out-shared-input-single-sync` standalone).
14. Per-launch tiny params in softmax_k → adjacent-file instance of
    `tiny-param-buffer-rebuild-over-cache`, applied correctly.
15. Padding loop redundant → PARTIALLY (the dflash half is a real write; topk half
    redundant — micro hazard H14).
16-23. (Lane B) batched-path selection → B34/B26; embedding cache → B33/B26; scratch
    reuse → ambient family; cycle-constrained orchestration → repo architecture;
    GPU-vs-CPU error divergence → inherited shared lineage, not a regression;
    "mirrors CPU verbatim" doc-lie → VERIFIED TRUE; feature-combo hazard → disproven
    (Cargo feature implies `cubecl_runtime`); step-invariant re-upload in dflash.rs →
    FALSIFIED (the file is clean — the waste lives in corrector.rs + shared helpers).

## Hazard yield → riir-ai Issue 697 (17)

Headline: **H1 the 3-layer silent no-correction chain** (the sentinel rule's live
instance), **H2 the release-mode embedding OOB window** (`set_embedding`'s length check
is `debug_assert!`-only; release builds accept an undersized buffer the gather kernel
reads as `[V×h]` — up to ~2.1 GB OOB window at V=256k), **H3 test-only reachability +
no CI row** (`dflash_predict_with_weaver_gpu` is called only by the same file's tests;
no `weaver_gpu` row in `ci_feature_guard.sh`'s pinned suites — the Layer 1.5-1.7
precedent has no weaver entry), **H4 ~29 alloc requests/call** (17 host + 12 device at
depth=4 — the Rule-1 fix), H5 depth=0 unvalidated, H6 GOAT assertion stricter than its
own doc (`pct <= 0.05` at max_len ≤ 5 tolerates ZERO diff while the comment promises
"differ by at most 1"), H7 inconsistent failure modes (panic on missing embedding vs
silent on shape bug), H8 the un-asserted 64-row cap, H9 the ~120-line entry-point
duplication, H10 `read_one` over-read, H11 panic paths in a `Result` API, H12
`tree_logits` ~8.4 MB/step allocs in shared helpers, H13 the `-69.07` magic constant
quadruplicated (= ln(1e-30), coupled to the `1e-30` guard), H14 the redundant topk-half
padding, H15 dead f32 payload in `depth_top_pairs`, H16 dead `Ok(steps)` payload, H17
the "behavior-identical to CPU" doc overclaim.

## GOAT

- **G1** (syn-parse): `batch43_pseudocode_parses` PASS — all 5 rules' ref_shape +
  opt_pattern parse.
- **G2**: 345 kernel_opt (342 + 3 new) + 201 default tests PASS.
- **G3**: clippy 0 warnings in all 4 states (default / kernel_opt / rust_perf /
  latent_retrieval; the single latent_retrieval warning = the sibling's pre-existing
  WIP `heal.rs:415`, untouched).
- Count pins: all sites in sync at 325 (A 73, B 47, D 57; C 32, E 18, F 10, G 19, H 14,
  I 4, J 5, K 5, L 10, M 9, N 18, O 4), enforced by passing tests.

## Generator notes

Two anchor bugs caught by the dry-run (zero pollution, both clean aborts):

1. **Header line-start assumption**: the anchor assumed the previous batch's header
   ends on a line starting with its last words — but wrapped continuation lines share
   the `//!   ` prefix and the anchor must match MID-LINE (from the distinctive tail
   text, not from the line start).
2. **The fn-closing brace before the string-literal quote** (a 4th anchor-class
   variant): the pseudocode's final `}` (closing the fn body) sits between the last
   `\n` escape and the Rust string's closing quote — `...walk\n}".to_string(),`. An
   anchor written `...walk\n".to_string(),` (missing the `}`) matches nothing. The
   mixed-escape tail anchor must account for EVERY byte between the pseudocode's last
   escape and `.to_string(),`.

Plus 2 doc_markdown fixes pre-commit (`T4.1`/`T4.2` and `DFlash` — the
CamelCase-adjacent family; `DFlash` needs backticks even in backticked-adjacent prose).

## Next

- `weaver_gpu/` dir (mod.rs ~62 KB + tests.rs ~57 KB) — the 19-dispatch kernel
  implementations, a separate cluster.
- `kvarn/` (~1,838), `hypernet/` (~3,300), `maxsim/` (~1,049).
- `kimi_k3_gpu_backward_batched.rs` (1,394) LOW value (shares Issue 693's H1/H2).
