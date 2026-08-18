# Research 065 — Batch 58: the ternary_deltanet sibling-WIP unlock (cudarc forward twin), 8 rules, corpus 355→363

**Status:** RECORD (batch landed, closed)
**Date:** 2026-08-18
**Batch:** 58 (file mining — reopen trigger (a): the ternary_deltanet sibling-WIP unlock)
**Source:** `riir-ai/crates/riir-gpu/src/ternary_deltanet_gpu_forward_cudarc.rs` (5,691 lines)
**Corpus:** 355 → 363 (A 83→86, D 66→70, E 20→21) — `entries_gpu_07.rs`

## 1. Why this batch existed

The file was the LAST never-mined riir-gpu orchestration file, blocked since
Batch 29/42-era on active sibling WIP. The unlock fired when the Issue 717
(speculative seam, `d626fcee0`) + Issue 715 (cudarc attention barrier fix,
`93d853582`) work LANDED (tree clean, file committed Aug 18 11:30). The
CubeCL twin (`ternary_deltanet_gpu_forward.rs`, 5,416 lines) remains BLOCKED:
open riir-ai Issue 721's T3 will rewrite its chunked-recurrence kernels
(IntraOutput/CrossChunk/StateTransition at line 3676) — mining it now risks
mid-flight quote drift. Issue 721's Non-goals explicitly exclude the CUDA
port ("Metal first"), so the cudarc twin was collision-free.

## 2. Method

2 parallel subagents (Lane A: lines 1-3000 infra/forward/graph/profiling;
Lane B: 3000-5302 backward/speculative + the named seams at 1003-2000 — the
spec seam actually lives in the FIRST impl block, which Lane B flagged and
mined anyway). Heavy cross-lane convergence: 4 of Lane A's 6 and 4 of Lane
B's 6 proposals were the SAME mechanisms found independently — merged by the
coordinator into single rules (the two-agent convergence pattern B39/B40
previously rewarded). Coordinator re-verified every load-bearing quote
(lines 145-158, 261-268, 283-295, 785-838, 1359-1365, 1392-1398, 1408-1434,
1476-1517, 1767-1813, 1899-1928, 3284-3302, 3435-3483, 5238-5260, 4436-4448)
+ personally confirmed H1 (grep: only `spec_verify_dispatch` exists),
H3 (the `?` between begin/end capture), H4 (the `with_norm_x` tail skips the
argmax tail).

## 3. The 8 rules

1. **`async-htod-host-source-outlives-copy` (A)** — an async HtoD memcpy
   reads the host pointer at GPU-COMPLETION time; a stack-local source
   dangles and the GPU silently reads the dead stack slot (wrong token/pos,
   no error). Persistent host staging fields + `slice::from_ref`. The
   HOST-side complement of the readback-timing family (B26/B27 pin device
   transients for deferred DtoH; this pins host memory for deferred HtoD).
   [L261-268, L1408-1434]
2. **`captured-graph-bakes-buffer-addresses` (D)** — mutators split into
   address-stable (in-place `memset_zeros` → graph SURVIVES) vs
   identity-changing (alloc/free → MUST drop the graph for lazy re-capture,
   else replay runs against freed pointers). The inverse-facing half of the
   hot-swap-keep-caches rule (B25 keeps caches alive; this kills the graph).
   [L815-838, L1513-1517, L3288-3299]
3. **`output-buffer-rotation-pool-multi-forward` (A)** — K pipelined
   forwards writing the same output field: swap the field's handle with
   pool[i] per dispatch (O(1) ownership exchange, zero bytes moved); ONE
   sync; K+1 position-aligned reads. The field KEEPS the last buffer between
   cycles (never un-rotated mid-run) — that makes the next cycle's out[0]
   the live pre-speculation prediction. [L283-290, L661-675, L1767-1838]
4. **`mid-pipeline-device-alloc-is-implicit-sync` (D)** — `cuMemAlloc` can
   implicitly synchronize the device: an allocation inside a
   no-intermediate-sync dispatch sequence is a HIDDEN BARRIER. Pre-allocate
   the cycle's whole working set at construction sized to the static max
   (SPEC_MAX_K=8) — "nice-to-have" pre-allocation is a correctness
   precondition for pipelining. [L283-290, L632-635]
5. **`capture-region-dispatch-only` (D)** — the captured region records GPU
   work ONLY: host effects (pos += 1) run ONCE at capture, NEVER on replay
   (silent frozen position); syncs/dtoh are not capturable; preconditions:
   event tracking off BEFORE any allocation, non-default stream, pre-upload.
   [L1359-1365, L1392-1398, L1480-1509]
6. **`occupancy-grid-cap-refuted-uncapped-default` (E)** — the Issue 705 /
   Bench 684 measured NEGATIVE, never distilled at resolution time (B49 came
   from Issue 706): the occupancy-derived cap (768 on 4090) measured a 1-2%
   LOSS at every size (87.1 uncapped vs 84.9-86.2 capped); wavefront
   locality beats the wave tail; the z-class exact-1-wave counter-example
   (706 vs 854 GB/s) kills the theory twice over. Cap stays env-gated as
   measurement apparatus. [L145-158, L449-486]
7. **`write-before-read-state-skips-checkpoint` (A)** — a speculative
   checkpoint backs up ONLY read-modify state; write-before-read append-only
   caches (attention KV at position p: decode reads [0..=p], the append for
   p overwrites the stale entry before any read) are repaired by
   re-execution itself. Async dtod, ZERO CPU↔GPU sync (stream FIFO order);
   host bookkeeping snapshotted by the caller. [L1899-1928, L292-295]
8. **`same-input-gemv-segments-single-launch` (D)** — same-input GEMV groups
   (qkv+z+a+b; gate+up; q+k+v) batched into ONE multi-segment launch (≤4
   segments); nsys measured 497 GEMV launches/token with m=48 projections at
   the ~5.6 µs kernel-latency floor (540 µs/token for 6.6 MB); pad unused
   slots with segment 0's pointers (m=0 matches no rows — never
   dereferenced). The generic middle between shared-upload-N-launches (B25)
   and the bespoke fused pair (B15). [L3435-3483]

## 4. Dropped candidates (23 across both lanes — the dedup filter held)

Highlights: devpos scalar buffers (dup `cuda-graph-scalar-via-device-pointer`
B22), env-resolved-once (dup B38), residual-fold GEMV (dup B19),
fused-quantize fan-out (dup B20/B25), device argmax (dup B25), dtoh
dst==src len workaround (adjacent-thin to B55), standalone
checkpoint/rollback (dissolved: FIFO ordering = B21, KV-no-backup = folded
into rule 7, pre-allocation = rule 4), argmax-memset-capturable (B27),
save-5-recompute-8 (B35/B36), CPU-stays-elementwise (B34),
spec-checkpoint-clone (B36), SectionProfile harness (diagnostic apparatus,
not decision content), split-borrow struct shapes (ambient Rust API).

## 5. Hazard yield → riir-ai Issue 722 (13 hazards)

H1 `spec_restore_buffers`/`spec_unrotate` doc-lie (HIGH — referenced but
nonexistent; the real graph×rotation mitigation lives only in the consumer);
H2 `set_lora` release-mode silent no-op (debug_assert-only layer checks; the
LoRA never fires for non-DeltaNet targets with zero diagnostic); H3
capture-error-path stream leak (the `?` between begin/end capture leaves the
stream capturing — every subsequent op fails); H4 `last_argmax` stale-read
contract break (the `with_norm_x` variants skip the argmax tail); H5 conv1d
backward full-buffer download to recover host-owned data (byte-identical
direct-upload fix); H6 attention-backward per-token alloc storm (~10 fresh
allocs/token/layer, violates the file's own no-mid-pipeline-alloc rationale);
H7 host zero-vec + htod instead of `memset_zeros`; H8 uncached
`dequant_transpose(&lm_head)` per backward call (GB-class at 27B); H9
borrow-staging via PCIe round-trip where dtod suffices; H10
layer-capture hand-duplicated loop (drift family); H11 zero in-file spec-seam
tests; H12 triplicated capture+launch body ("keep them in sync"); H13 stale
gemv_multi doc (says the grid "caps at full residency" — contradicts the
measured-UNcapped default 30 lines above).

## 6. GOAT

- **G1**: 3/3 batch58 gates; 16 ref/opt pseudocode pairs syn-parse.
- **G2**: kernel_heal **769/0/1** (B57's 766 + exactly 3 new gates); default
  **571/0/0** (unchanged).
- **G3**: clippy **0** both states. One post-write fix round: the B37
  line-initial `+ `/`- ` lesson struck a 5TH time (3 trigger lines in the
  batch58 bullet cascading 33 doc-list warnings) + 6 backtick items
  (`ternary_deltanet`, `CubeCL`, `HtoD`, `set_lora`, `lm_head`,
  `dequant_transpose`). Also the B51 case-sensitivity lesson struck
  (concepts test fragments vs sentence-initial caps — fixed in the test).
- **Retrieval**: 13/13, floors HELD with zero decay — clippy 89.6/95.8/95.8,
  perf 96.2/98.1/98.1, kernel self-variants **100/100/100**, blended
  94.6/97.7/97.7 (identical to B53/B55/B56/B57 — sixth consecutive batch).

## 7. State after this batch

- Corpus: **363 kernel rules** (A 86, B 52, C 33, D 70, E 21, F 10, G 19,
  H 14, I 4, J 5, K 5, L 11, M 9, N 20, O 4) + 62 rust_perf.
- `entries_gpu_07.rs` now ~500 lines (10 rules; room for more).
- The riir-gpu corpus is COMPLETE-minus-one: only the CubeCL twin
  (`ternary_deltanet_gpu_forward.rs`) remains, blocked on Issue 721 T3.
- The git-log re-scan discovery channel (B57's lesson) paid off AGAIN: the
  unlock itself was found by re-scanning sibling git logs for fix commits
  touching the blocked file.
