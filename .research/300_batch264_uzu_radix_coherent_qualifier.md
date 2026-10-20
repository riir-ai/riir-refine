# Research 300: uzu #994 — the cross-threadgroup coherent-qualifier face (Batch 264)

**Status:** RECORD — Batch 264 landed (Plan 268); count-neutral rider, corpus stands 923.

## Source

- Repo: trymirai/uzu (MIT) @ `32cd07c3f648d65e77063f3105f36c7c85bf5d81`
  — #994 "Radix top-k cross-threadgroup coherency" (2026-10-20 05:49 +0400,
  author uuuvn).
- Diff: ONE file, +1/−1 — `crates/uzu-engine/src/backends/metal/kernel/radix_top_k_small.metal`,
  `RadixTopKSmallCollect` kernel signature, line 164:
  `device ulong* selected_keys` → `device coherent(device) ulong* selected_keys`.
- Verified from the vendor clone (`E:/git/vendor/uzu`, fetched this session by the
  league watch) — `git show 32cd07c` diff --stat read + both spellings at the pin.

## The atom

Metal's default `device` address space is **non-coherent**: the compiler may cache
accesses through a bare `device` pointer with only per-threadgroup coherency, so
**plain (non-atomic) vector stores** into a buffer that ANOTHER threadgroup reads —
same dispatch or a later pass on the same command stream — carry no cross-threadgroup
visibility guarantee. The `coherent(device)` qualifier opts the buffer into
device-wide coherency for every access through that pointer.

The protocol's ATOMIC surface never needed it: `arrive_last` / `reset_arrival`
(quoted verbatim in the host rule since Batch 260) carry explicit memory orders and
device-scope fences. The defect was on the PLAIN-STORE surface (`selected_keys` is
written with vector stores in the collect phase, read across threadgroups downstream).

## Why a rider on gpu_85 (and not a rule)

The host `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` (Batch 223,
lithos `static_barrier_*.metal` family) already carries the design clause verbatim:
**"Coherent pointers and device fences publish intermediate data across groups"**
(static_barrier_serial.metal:1-3). uzu #994 is that clause's upstream DEFECT
INSTANCE — a shipped radix top-k that had the fences but not the qualifier. The
Batch-260 face (#991) and this face (#994) are the same file, same protocol, two
independent visibility defects five days apart:

- **#991 = the ORDERING axis** — barrier scope (`mem_threadgroup | mem_device`) +
  fence placement on the merge window.
- **#994 = the QUALIFIER axis** — `coherent(device)` on the cross-threadgroup
  plain-store buffer.

LAW: a cross-threadgroup handshake is correct only when BOTH axes hold — the pointer
qualified coherent AND the ordering fences/barriers at device scope; either alone can
serve stale or unordered state. Composition, never replacement.

## Dedup (3+ vocabulary)

- `coherent` corpus grep: entries_gpu_30 (sentinel protocol — mentions "remote
  stores arrive coherent" as a scoped premise, does not own the qualifier),
  entries_gpu_35 (Metal function-constant cache keys — the word in a different
  sense), entries_gpu_86 (steal-claimant rule — its probe RAN "spec-compliant
  coherent(device) + seq_cst fences" as a measured posture, never states the
  defect law). NONE owns "cross-threadgroup plain-store handshake buffer must be
  qualified coherent".
- `selected_keys` / `radix_top_k` grep: only the Batch-260 face (same host) —
  which is where this face belongs.
- Not a rule: the atom is one word wide; the decision content (qualifier + scope
  ordering compose) rides the host where both faces live.

## Disposition

- Count-neutral rider on gpu_85; needle gate `batch264_rider_landed` (7 verbatim
  needles + the 923 count witness), FIRST-RUN PASS.
- Gates: kernel_opt lib 3193/0 (+1), default lib 2715/0, clippy −D ×2 postures
  clean, retrieval floors HELD 21/21 no re-pin (291.1 s, isolated CARGO_TARGET_DIR
  removed). Freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary
  rebuild NOT owed (kernel_opt outside the release set).
- CRLF hazard: entries_gpu_85.rs is CRLF-native (349); the rider spliced inline
  byte-exact (0 line-ending churn — python byte splice, not an editor rewrite).

## Watch rows from the same sweep (no corpus)

- llama.cpp 4 past pin `7adf160c4` → new pin `3bb63cbcbf`: #30713 (CI skills lint,
  NULL), #30754 (WASI CI/docs, NULL), #30736 (mtmd Qwen-VL resize, out-of-lane),
  **#30430** (sycl get_rows on reordered quantized weights — REAL law: a
  shared-weight layout-transform optimization must remain correct for every consumer
  of the weight (tied embeddings read raw rows); the `test_get_rows_after_mul_mat`
  ordering shape. Graded WATCH/lane-intel, NOT authored: SYCL-lane fix, and our
  lane's weight transforms (bonsai2_hadamard folds) happen at LOAD time, before any
  consumer — the in-place per-op transform class has no surface today. Re-opens if a
  per-op weight-transform lane lands).
- bevy `ddd047d56..0691fcd66`: ONE commit #26242 (Feathers font families +
  KeySymbols font) — UI lane, NULL.
- vllm 2 past the B263 merge `3c9beb82`: #60373 (DCP strided-LSE direct-A2A
  reduction — distributed-attention lane, NULL), #52670 (pluggable KV-cache dtype
  feature — architecture watch).
- prism: NO new scored tag (ladder top b10787); fork tip unchanged past the B261
  window → ai-971 absorption NO-FIRE; the queued decode re-pin bout stands
  (GPU-gated, StartLux resident).
- cubecl `ec701b5fe` / TensorFold `d1018b318` / splash `be3b083c5` / wgpu
  `e084b66aa` — ALL static vs pins. mlxfast `0c7c392` NO-FIRE (weekly floor ~10-24).
- INSTRUMENT: `F:/llama.cpp` on this box was a STALE SHALLOW graft (HEAD tag b10567,
  `7adf160` not an ancestor) — the league watch's llama.cpp listing from it is
  unreliable; refreshed before the watch artifact re-lands in riir-ai.
