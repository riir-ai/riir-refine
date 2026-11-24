# Research 441: vLLM GDN decode-fusion family — the third+fourth instances of the whole-step law

> **Source:** vLLM three-commit window, all read at the pinned blobless clone `.raw/vllm-gdn` (2026-11-24, amo, zed-fleet@amo — REMOVED at close):
> - `78f20a5a3` — "[Kernel][GDN] Faster fused CUDA MTP decode for 16-bit recurrent state" (#69065, 2026-11-21; Apache-2.0)
> - `34c653886` — "[ROCm][Perf] Fuse GDN decode (conv1d + delta rule + gated norm) into AITER's gluon kernel (Qwen3.8-27B)" (#61428, 2026-11-20)
> - `92b52792d` — "[Perf][GDN] Drop the prefill copies of the fused spec-decode path" (#67733, 2026-11-16)
> Staged by the perf-rematch weekly watch (2026-11-24, riir-ai `d0b745763`) as lead family (1); this verdict is the T0 the watch row owed.
> **Date:** 2026-11-24
> **Status:** LANDED 2026-11-24 (Batch 407, Plan 406, zed-e7e471a3@amo) — rust_perf +1 rule, kernel_opt +8 count-neutral faces, same commit. **R1 DEMOTED at landing (NotBenchable-by-platform):** the GOAT falsifier fired — the Pareto validator priced the pair ParetoDominated (0 faster / 3 slower / 0 tie) and the isolated probe (identical indexing loops, only the allocation differing) read 0.967/0.979 twice, inside the tie band: the Rust class `vec![0.0; n]` rides `alloc_zeroed`, whose zero-page path serves the zeros without a separate memset pass on this platform posture, so the pass the rule teaches deleting (torch.zeros pays a real memset) is already absent in the Rust allocation class. The LAW stays decision content (the dead-store mechanism + regime condition + canary discipline; the torch win is real where it was measured). The 16-bit-state lever (T8): LOSSY-GATED lane intel — never a bit-identity arm; any future decode-state-traffic cell is governed by the lossy law's per-family retention gate. DROPS + widen finds recorded in §6 below.
> **Related Research:** 438 (mlx-4927, the Metal sibling family), 421 (anemll GDN numerics), 429 (maya inkling-mini)
> **Related Plans:** 406 (this verdict's batch plan, unclaimed)
> **Classification:** Public

---

## TL;DR

The watch pass's prediction confirmed at source: vLLM's GDN window is the **third+fourth independent instances of `whole-step-gdn-decode-window-one-launch`** (entries_gpu_126, B278) — 34c65388 re-fuses conv1d+delta-rule+gated-RMSNorm into one launch (ROCm/AITER form + the typed capability-envelope refusal), 78f20a5a carries the S-token window arm (kMaxMtpTokens=9, recurrence in-register across tokens) with the states variant (per-token spec-slot store). Positive instances are faces, never rules. The NEW atoms: (1) the **dead-pre-zero elimination** (`torch.zeros` → `torch.empty`, "the kernels write every actual row" — the inverse polarity of `vec-macro-zero-fill`, benchable, the GOAT candidate); (2) the **packed-16-bit recurrent-state store with f32 compute** (uint4 pair load → f32 math → packed store — face-or-rule on gpu_51's native-dtype-at-the-boundary family, state-side polarity; fold decision at authoring); (3) the **12-per-8 padded smem column map** (`qk_column` — integer-padded bank-conflict-free layout, face-or-thin-rule on the swizzle family). Five more faces on existing owners. Yield: 1 rule + 4-6 faces after falsification.

## 1. What the commits actually do (all quotes read at the pin)

### 78f20a5a — the 16-bit MTP decode kernel (#69065)

A dedicated 16-bit-state variant of vLLM's fused GDN decode kernel (`csrc/gdn/fused_gdn_decode_16bit_kernel.cu`, 450 lines):

- **Packed state I/O:** `load_state8` reads the bf16/fp16 recurrent state as one `uint4` (four `__nv_bfloat162`/`__half2` pairs → eight f32 values), the delta-rule recurrence runs in f32 registers (`h[4][4]` float2 accumulators, `__ffma2_rn`), and `store_state8` rounds back through `__floats2bfloat162_rn` at the store point. State traffic halves vs the fp32-state kernel; the compute precision does not change (the state's own dtype is the checkpoint dtype — this is NOT the lossy class: the reference path stores the same bf16 state).
- **The MTP window:** `kMaxMtpTokens = 9` — one launch carries up to 9 accepted draft tokens per request; warps stage tokens into shared memory (`shared_q/k/v/decay/beta`), the recurrence steps across tokens with the state resident in registers, each token's result lands in its `destination_slot` (`state_indices[request * width + t]`, store gated `destination_slot > 0`), and the source state loads from `accepted - 1`'s slot.
- **The in-kernel fallback:** `source_slot <= 0 || num_tokens > kMaxMtpTokens` → the kernel zero-fills its own output (the guard arm keeps one launch, never a host-side branch).
- **The 12-per-8 padded column map:** `qk_column(dim) = (dim >> 3) * 12 + ((dim & 3) << 1) + ((dim >> 2) & 1)` — the shared q/k rows are laid out 12 words per 8 dims so each lane's `float4` reads land in distinct banks (integer-padded static index remap, data-independent — a different mechanism from XOR swizzle).
- **`reduce_scatter`:** a keep/send butterfly (`lane & 8/4/2` selection with `__shfl_xor_sync` at offsets 8/4/2, then a 3-step cross-lane fold) specialized to 16 lanes × 8 partials.
- **Fused gated-RMSNorm epilogue:** rstd + weight × sigmoid/silu gate in the same kernel, `SigmoidGate` a template arm.

### 34c65388 — the ROCm/AITER whole-step fusion (#61428)

- One launch per layer for conv1d + delta-rule + gated-RMSNorm via AITER's gluon kernel (`torch.ops.vllm.qwen_gdn_attention_core_fused_norm_rocm`).
- **The typed capability-envelope refusal:** `_aiter_fused_gdn_decode_unsupported_reason` returns a reason STRING naming the exact envelope — "K=V=128, 2 or 3 value heads per key head, conv width 4, SiLU conv and gate activations, a BF16 convolution cache, a BF16 or FP32 recurrent state, an unquantized out_proj and no speculative decoding" — and the caller logs `info_once` and keeps the decomposed path. Note the envelope includes the CONSUMER constraint: "The kernel emits the normed output, so out_proj cannot quantize it" — the admission names the downstream consumer's requirement, not just the kernel's own geometry.
- **Zero-bias padding:** the fused kernel requires a conv bias, so `bias is None` registers a zeros buffer — pad the shape to the kernel's contract instead of branching.
- **The signature version gate:** `inspect.signature(fused_gdn_decode_qkvz)` must carry `qkvz_layout`/`query_start_loc`/`out` ("added in ROCm/aiter#6488") — capability detection by parameter introspection.

### 92b52792 — drop the prefill copies (#67733)

- **`.contiguous()` eliminated:** `b=b.contiguous(), a=a.contiguous()` → pass as-is; the layernorm-guard Triton kernel learns `groups_per_row` strides and reads the strided rows in place ("A row per token, a group per head: reads the strided gate in place").
- **The dead-pre-zero elimination:** `core_attn_out = torch.zeros(...)` + `.zero_()` → `torch.empty(...)`, and the test canary `fused_out = torch.full_like(output_gate, torch.nan)` (NaN-poison as the writes-every-row coverage detector). The commit's own law: "No zero fill in either branch: the kernels write every actual row, and padding rows only feed row-wise ops (same as forward_cuda)."
- **The shared-weight stride-0 lane:** `self.norm.weight.expand(num_heads, head_dim)` + `stride_w_group = 0 when all groups share one weight` — one weight row serves every group through a zero stride instead of a per-group copy.

## 2. Domain routing

`kernel_opt` for the CUDA/ROCm window + state-traffic atoms (the tempting WRONG routing: none — this IS kernel_opt family ground); `rust_perf` allocation for the dead-pre-zero elimination (the Rust shape: `vec![0.0; n]` before a full-overwrite fill = wasted memset; `Vec::with_capacity` + write-every-row). The transcription law holds: fixtures are our own Rust reproductions.

## 3. Dedup gate — evidence per atom (3+ vocabularies, corpus at 35c219d4)

| Atom | Greps | Verdict |
|---|---|---|
| whole-step fusion / window / states variant | `gdn\|deltanet\|delta-rule` (7 owners) · read `whole-step-gdn-decode-window-one-launch` in full (entries_gpu_126) | **OWNED** — all four mechanisms (one-launch whole step, S<=16 window + register residency, capability gate, per-token states for rewind) are the rule's own clauses; vLLM = the CUDA+ROCm instances. Faces. |
| packed 16-bit state store | `uint4\|half2\|packed.*state` · `state`-owner list (27 rules) · gpu_51 `dtype-native-source-read...` read | **ADJACENT-DISTINCT-OR-FACE** — gpu_51 owns load-side WIDENING of inputs; this is store-side NARROWING of the recurrent state across a recurrence loop. Fold decision at authoring (face likely; the uint4-pair packing axis is its only rule-shaped delta). |
| 12-per-8 padded smem column map | `bank\|swizzle\|pad` (18 owners: xor-swizzle, circular-reorder, no-bank-padding, shift-row-stride) | **FACE-OR-THIN** — integer-padded static column remap is a fourth swizzle-family mechanism (XOR vs circular vs shift vs integer-pad); likely a face on `xor-swizzle-smem-static-index-tables` (the static-index-tables kin). |
| dead-pre-zero elimination | `zero\|fill\|empty\|init` kernel_opt (no owner) · rust_perf: `vec-macro-zero-fill` = the INVERSE polarity (need zeros → calloc); no owner of "don't pre-zero what a full-overwrite kernel writes" | **NEW RULE** — dedup-clean both corpora. |
| per-token spec-slot state store | `slot\|spec\|accepted` · `gdn-state-snapshot-per-position-verify-early-exit` (B337) | **FACE** — the CUDA instance of B337's per-position snapshot law (slot-indexed form). |
| capability-envelope typed refusal | `fa-fast-path-validated-exact-shape-admission` + the B398/B401 admission family | **FACE** — well-owned shape; the consumer-constraint clause ("out_proj cannot quantize it") is the one new shading. |
| signature version gate | B404 `dependency-version-known-wrong-window-refusal` | **FACE** — the inspect-signature variant of the version-gate family. |
| shared-weight stride-0 | `gqa-zero-stride-batch-broadcast` (B197) | **FACE** — the weight-side instance. |
| contiguous-drop stride generalization | `alignment-gated-zero-copy-view` + `fa-...-v-as-strided-view` | **FACE** — second instances. |
| reduce_scatter butterfly | `warp-reduce\|butterfly` — standard shapes | **DROP** — thin, no bounded decision content beyond standard shuffles. |

## 4. GOAT candidate

**`output-buffer-prezero-when-kernel-writes-all-rows`** (rust_perf allocation): an output buffer pre-zeroed (`vec![0.0; n]`, `torch.zeros`) then fully overwritten by the producer wastes a full memset pass; allocate uninitialized (`Vec::with_capacity` + set-len-on-full-write / `MaybeUninit` / `torch.empty`) when every row is written, and prove the coverage claim with the NaN/poison canary in the TEST (the commit's own device — `torch.full_like(..., torch.nan)` fails loud if any row is missed). Benchable on the Pareto validator + exact alloc-count axis (alloc+memset vs alloc-only), exact semantics (the overwritten values are identical; only the pre-write contents differ and are never read), deterministic, magnitude story = one full-buffer memset eliminated (O(n) write pass). The regime condition is the load-bearing half: EVERY row written by ONE producer before any read — padding rows "only feed row-wise ops" is the named anti-regime boundary (a partially-written buffer read elsewhere = UB class, the reason `vec-macro-zero-fill` exists as its inverse). The 92b52792 test canary is the fixture's falsification arm shape.

## 5. Lane mapping (moderate)

- **League decode side:** 78f20a5a targets exactly the Qwen3.5/3.8 GDN recurrent-state class our league rows decode (Bonsai-2-27B GDN tiers; the decode campaign riir-infer 1139→1145, ffn_gate_up 28.1% the named lever). The two transferable LEVERS for our decode lanes: (a) MTP multi-token one-launch — our `whole-step-gdn-decode-window-one-launch` already owns the shape (S<=16); the vLLM instance corroborates the window law at production scale, nothing to file; (b) **16-bit recurrent-state traffic** — our GDN tiers store f32 state; storing bf16 state halves decode state traffic. NOT a free win: bf16 state is the LOSSY class on our stack (the lossy-surface law, per-family retention gate; riir-infer issue 879 quant-certification gates state precision). The honest disposition: a lossy-gated lane-intel line only — if a decode-state-traffic cell ever opens, it opens under the lossy law's per-family retention gate, never as a bit-identity arm. Nothing filed this session (no open decode lane names state traffic as its current blocker; the named blocker is ffn_gate_up).
- **NOT transferable:** ROCm gfx950 AITER gluon kernels; the CUDA bf16 intrinsics; their TRT-style spec-decode integration; absolute ms figures (H100/MI-class provenance).

## 6. Product coverage

None — vLLM-specific kernels, no famous-lib Rust family seeded.

## 7. Honest caveats

- Verdict from a three-commit window, not a repo walk — vLLM's GDN surface beyond these commits is UNREAD (the mamba/gdn directory has more files; a batch T0 re-pins and may widen the yield).
- Blobless clone: quotes read from fetched blobs at the pinned shas; batch T0 re-clones + byte-verifies (fast-moving repo, the standing caveat).
- The faces' home-rule attribution needs the batch lane's in-rule adjudication reads (this verdict read the one controlling owner in full; the face hosts named from rule_ids + memory of the batch records).
- Co-authored-by Claude Opus 5.5 / Claude Fable 5.1 on two commits — provenance note only.

## 8. Bottom line

**Mine as ONE face-heavy batch** (nominal Batch 407, Plan 406 — staged-unclaimed behind live Plans 404/405): lead `output-buffer-prezero-when-kernel-writes-all-rows` (rust_perf, the benchable GOAT candidate), the 16-bit-state atom as face-or-rule on gpu_51 (fold at authoring), the padded-column map as face-or-thin, and 5 count-neutral faces on their owners. Nothing INVENTED — every quote read at the pinned clone this session. Clone removed at close.

Session: zed-fleet@amo, epoch 1795466000
