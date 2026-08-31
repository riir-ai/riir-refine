# Research 107 — ktransformers distill: graph-stable GEMM + serving-graph discipline (Plan 548 feed)

**Status:** RECORD -- external-source distill landed 2026-08-31. Corpus candidates 1-12
LANDED as Batch 92 (11 rules, `entries_gpu_09.rs`, commit in git history; the repack rider
folded into `cpasync-multistage-tile-pipeline`). The residue (13-14 + the 8 host
candidates) DISPOSITIONED same day as Batch 93 (`.plans/054`, zero corpus delta):
14 was already inside Batch 92's `quant-group-scale-fetch-gating` (the cu:464-472
alignment precondition); 13 killed (no source anchor); host candidates killed on
dedup/bar grounds or routed to the Plan 548 intel lane. The ktransformers lane is
CLOSED. Plan 548 lanes + the league ledger fed. Read-only mining; raw clone at
`.raw/ktransformers` (gitignored, disposable -- the clone is not the record, this note is).

## Source

- `kvcache-ai/ktransformers` @ `21095b9912f844b4cf1a55097f9f66c0bbbd3de5` (2026-08-31, HEAD at
  clone time), shallow clone to `riir-clippy/.raw/ktransformers`.
- What it is today: the repo has pivoted to **kt-kernel** — CPU-optimized kernels (Intel
  AMX/AVX512/AVX2, INT4/INT8/BF16/FP8/FP4/mxfp8) for CPU-GPU heterogeneous MoE inference,
  with serving delegated to an SGLang integration (adapter converter + install checker only —
  `third_party/sglang` is EMPTY in the clone, fetched at build time). The OLD full framework
  (server, models, placement, balance_serve) lives in `archive/ktransformers`; the OLD CUDA
  kernels live in `archive/csrc`.
- Mining method: 3 parallel read-only subagent sweeps (CUDA kernels / CPU-host / serving-arch)
  + coordinator spot-verification of the load-bearing claims before writing this record.

## Coordinator spot-checks (all claims below verified in-file, not trusted from the agents)

- `archive/csrc/custom_marlin/gptq_marlin/gptq_marlin.cu:1410-1457` — `Marlin_wrapper` takes
  `const int* __restrict__ prob_m_ptr` (L1417) and reads `int prob_m = *prob_m_ptr;` (L1422)
  INSIDE the kernel, caps `min(prob_m, 1024)`, derives `thread_m_blocks = min(div_ceil(prob_m,16),
  template_thread_m_blocks)` at runtime, dispatches {1,2,3,4}-m-block template instantiations via
  an if-chain. Launch geometry is M-independent (`grid = sms`).
- `archive/ktransformers/server/balance_serve/inference/model_runner.py:154-156` — graphs are
  captured with `pool=self.graph_memory_pool`, then `self.graph_memory_pool = graphs[i].pool()`
  — the memory pool is CHAINED across all bucket graphs (one arena).
- `kt-kernel/examples/test_mxfp8_moe_m3.py` — EXISTS (the CPU agent wrongly reported it missing);
  it is a **MiniMax-M3-Preview model** test (AMX MXFP8 + swigluoai), NOT Apple M3. The agent's
  substantive conclusion stands: there is NO Apple-silicon path anywhere in the repo
  (`DetectCPU.cmake` reads `/proc/cpuinfo`, x86-only; AMX is an Intel tile ISA).

## Verdict

| Axis | Grade | Why |
|---|---|---|
| kernel_opt mining source | **A−** | The modified Marlin (`archive/csrc/custom_marlin`) is a complete, working, ktransformers-extended W4A16 GEMM — the single richest kernel in-repo — plus a GGUF dequant family. The rest of the CUDA surface is thin (no attention kernels at all). |
| rust_perf / host-scheduling source | **C+** | task_queue/worker_pool/shared_mem_buffer are x86+NUMA-flavored; the portable patterns (arena-rebind, K-block sizing rules, intensity-gated twins, pipelined barrier) are real but mostly overlap instincts our corpus already encodes. |
| Serving-arch intel for Plan 548/556 | **B** | The balance_serve graph discipline + the Marlin graph-stable GEMM are directly actionable for the lanes (concurrency) lane; placement/overlap ideas are MoE-offload-shaped (N/A premise). |
| Adopt wholesale? | **NO** | Their premise is CPU-offload MoE serving on multi-GPU; ours is a full-GPU-resident dense/hybrid model. Everything is technique-level cherry-pick. |

## Findings vs Plan 548 lanes

### P0-1 / U7 — Bonsai prefill GEMM (owner-gated, cp.async warp specialization = the named unlock)

The **ktransformers Marlin is the concrete complete reference** for exactly the technique the
league ledger names as the remaining lever (`perf_rematch_league.md` Bonsai-prefill lane:
"the one live lever — cp.async warp-specialized GEMM, ~2,270–2,437 projected, gate-red either
way — queued owner-gated as Issue 780 U7"). Anatomy, all file:line verified
(`archive/csrc/custom_marlin/gptq_marlin/gptq_marlin.cu` = 2034 lines; provenance: Neural
Magic/vLLM lineage per header L18-24, **ktransformers-extended**):

- **4-stage cp.async pipeline**, 16 B `cp.async.cg`, predicate-carrying predicated copies for A
  (`cp_async4_pred`, `gptq_marlin.cuh:45-55`); wait is `cp_async_wait<stages-2>` + syncthreads
  (cu:807-814); the next prefetch group is issued **mid-tile** — inside the k-unrolled inner
  loop at `k == b_sh_wr_iters-2` (cu:1247-1253) — so DRAM latency hides under dequant+mma,
  not just under mma.
- **Register double-buffer**: `frag_a[2][m_blocks]`, `frag_b_quant[2]`, `frag_s[2]`;
  `fetch_to_registers(k+1)` issued before `matmul(k)` (cu:1244-1254); even-length pipelines so
  every smem access is static/unrolled.
- **ldmatrix**: A loaded `ldmatrix.sync.m8n8.x4` straight into tensor-core fragment layout
  (cu:118-127); B read as raw int4 from smem, dequantized **in registers** (cu:829-833).
- **XOR-swizzle smem** with read+write transforms precomputed into static per-thread index
  arrays (`a_sh_wr_trans` / `a_sh_rd_trans`, cu:644-665) — the unrolled loop sees constant
  indices.
- **Dequant recipes** (for a hypothetical fp16-mma W4A16 arm — NOTE: ours is int8-mma):
  int4→half2 via ONE `lop3` with LUT `(0xf0&0xcc)|0xaa` exponent-embedding `(q&0x000f000f)|
  0x64006400`, symmetric −8 zero-point fused as `SUB=0x64086408`, second nibble pair via the
  shifted `b_quant >> 8` (cu:163-208); int8→half2 via two `prmt` byte-perms + one `__hsub2`
  vs magic `0x64806480` (cu:221-240).
- **matmul order n-outer/m-inner** "to encourage overlapping dequantization and matmul"
  (cu:960-1014). **Scale-fetch gating**: grouped scales cp.async'd only when a tile starts a
  new quant group (cu:780-796). **Multi-pointer dependency break**: B uses an array of
  pre-offset pointers instead of one advancing pointer (cu:667-674).
- **Scheduling/reduction**: grid = `sms` exactly (cu:1723); ragged shapes striped so all SMs
  stay busy (cu:474-521); K-slice warps reduce in smem (log reduction, cu:1021-1072);
  cross-block stripes reduce **in the L2-resident fp16 output buffer** behind a per-column
  lock-free semaphore — `ld.global.acquire.gpu` spin + `fence.acq_rel` + `red.relaxed.gpu
  .global.add` (cu:315-345, 1074-1137). `max_par=16` M-stripes.
- **Config table** (cu:1489-1507): M>16 → {64,256,256}-threads wide-N first; smem-validity
  formula `(a+b)·stages < 0.95·(max_smem − scales_cache)` (cu:1509-1580); launched with
  `cudaFuncAttributeMaxDynamicSharedMemorySize` opt-in (cu:1466-1473).

**Honest architecture-point caveat**: Marlin is **fp16-mma W4A16** (weights dequantized in
registers; activations never quantized). Our U7 arm is int8-mma. The PIPELINE structure
(stages, mid-tile issue, register double-buffer, ldmatrix, swizzle, striping, semaphore)
transfers 1:1 to an int8-mma arm; the dequant LUT recipes apply only if we ever ship a
fp16-mma W4A16 arm (sm89 supports `m16n8k16` + cp.async — a real alternative point that
avoids activation-quantization error entirely).

### P0-2 / verify-C — mma-arm fix reference (NOT a kernel to port)

**NOT FOUND: no attention kernel exists in this repo** (MLA lives on CPU; the old GPU attention
was external flash-attn; grep `mla|flash_attn|fmha` over `*.cu/*.cuh` = zero hits). Nothing to
port for the LDS-issue-bound score phase. But the league's reopen condition (b) names "a
Marlin-class repack PROVEN ≥1.15×", and the Bench-786 KNOWN-BROKEN score-mma arm died on
**fragment assembly** — Marlin is a clean, complete, working `m16n8k16` fragment-assembly
reference: `FragA=Vec<half2,4>` / `FragB=Vec<half2,2>` / `FragC=Vec<float,4>` /
`FragS=Vec<half2,1>` (`gptq_marlin_dtypes.cuh:22-26`), the inline
`mma.sync.aligned.m16n8k16.row.col.f32.f16.f16.f32` PTX (cu:86-114), and ldmatrix A-loads
(cu:118-127). Fed to the league ledger's reopen row as the pinned reference.

### P1-2 / lanes-concurrency (Plan 556) — the strongest transferable cluster

1. **Device-side-M graph-stable GEMM** (`Marlin_wrapper`, cu:1417-1424 — ktransformers' own
   extension; upstream Marlin has host-side M): M read from a DEVICE pointer inside the
   kernel, runtime m-block dispatch, launch geometry M-independent ⇒ **one captured graph
   serves every batch size**. `archive/csrc/custom_marlin/test_cuda_graph.py` proves the
   contract: capture with a max-size input buffer + an int32 device `bsz_tensor`, mutate the
   batch OUTSIDE the graph, replay, compare `out_buf[:cur_batch]` vs fresh eager. **Caveat
   found en route**: `min(prob_m, 1024)` — M > 1024 needs host-side chunking; the wrapper is
   decode/verify-oriented (for our P=2048 prefill, chunk; for lanes batch sizes ≤ 1024, free).
   This is the key enabler for Plan 556 lane packing: GEMM shape-freedom under capture removes
   the per-batch-shape capture explosion.
2. **Graph bucket discipline** (`model_runner.py`): bucket list
   `[1,2,3,max_batch,64,256,512,chunk_size]` + 1024-multiples, dedup+sort (L58-68); tokens
   padded to 64 (L53); **shared graph memory pool chained across all buckets** (L154-156);
   **11 eager warmup iterations on a side stream before capture** (L190-198 — cubin/JIT
   warm-up; codifies our Issue-749 first-arm cold-start lesson); **static pinned staging
   buffers** per graph (`page_idx_buf`/`page_offset_buf`/`features_buf`/`bsz_tensor_buf`)
   `copy_`'d at replay — shape baked in, data streamed in (L294-321).
3. **Graphs encode prefill+decode MIXES**: bucket graphs store
   `prefill_query_length = (bucket − max_decode_bs) / max_prefill_bs` (L164) — one replay
   advances multiple requests at different phases. Measured **+130% throughput @ 4-way
   concurrency** (`doc/en/balance-serve.md:18`; honest caveat: their bottleneck was CPU expert
   compute — our GPU-bound case differs, but it is direct evidence the mixed-graph serving
   shape pays).
4. **Graph-safe CPU↔GPU event handshake inside captured graphs**
   (`archive/ktransformers/operators/experts.py:293-344`): pinned per-graph-index buffers +
   `submit_with_cuda_stream` — a host-threadpool task that records a CUDA event on the
   *capturing* stream when done, with a `torch.cuda.is_current_stream_capturing()` branch
   (L325) — the only pattern found for capturing host-completion into a replayable graph.
5. **Mutable-scalar-outside-graph precedent**: the old backend calls
   `cache.change_seq_length(1)` before each replay — mutating a length scalar between replays
   while the graph holds the append into a fixed cache (`ktransformers.py:165-184`) —
   independently validates our KV-append-into-fixed-cache graph design (the Issue-746
   append-only contract).
6. **Deferred-execution knob**: `--kt-max-deferred-experts-per-token` = defer 1-4 layers'
   work across the loop, sync later; 1-4 sweet spot, 5-7 documented accuracy loss
   (`kt-kernel/README.md:507,537-542`) — bounded-lookahead with an honest trade table.

### P2-3 / chat drafter — confirmatory only

Their MTP is **delegated to SGLang EAGLE**: `--speculative-algorithm EAGLE
--speculative-num-steps 3 --speculative-eagle-topk 1 --speculative-num-draft-tokens 4`
(`doc/en/DeepSeek-V4-Flash.md:188-200`) — measured ~1.2× single-request decode, 26.5 → 32.74
tok/s (8×5090), **90% accept at chain depth 1**. Two uses for us: (a) confirms the league's
chat-row decomposition — even the reference EAGLE stack lands only ~1.2× e2e at depth-1
chaining, so our P2-3 trained-drafter GOAT gate arithmetic gets an external anchor; (b) their
in-archive MTP support is a stub (`# TODO support MTP`, `model_runner.py:386`; ngram arg stub
`args.py:72`) — no verify-loop machinery to transplant.

### M3 lane — N/A confirmed

No Apple-silicon path exists (every "M3" in-tree = the **MiniMax-M3 model**; `FindSIMD.cmake`
compiles x86 intrinsics only). Their GDN-hybrid handling (`doc/en/Qwen3-Next.md:69-70`, old
stack): **"CUDA Graph optimization is not yet support"** for linear attention — our graphed
GDN decode is ahead of their archive path; nothing to adopt.

### Ingest / prefix cache — minor hardening ideas

Their real 3-tier (GPU/CPU-Disk) prefix cache is `kvc2` (ARCHIVED, `archive/ktransformers/
kvc2/`): 256-token pages keyed by **XXH64 token-slice hash + an independent-seed check hash
before trust** (`hasher.hpp:9-11`), `MatchStatus{Exact,Partial}` with **`recompute_ratio=0.2`**
(matched fraction below threshold ⇒ recompute, don't fetch), **batched eviction
(`evict_count=20`)** (`kvc2.h:37-43`). Our GDN+KV prefix cache already leads theirs in the
league rows; the dual-seed hash and the recompute-ratio guard are optional hardening riders,
queued not planned.

## Measured numbers found (honest inventory)

**Zero kernel-level numbers are committed anywhere in the repo** (benches print at runtime; no
result JSONs; `L2_size = 72 MB` and `sms=56` in `test_cuda_graph.py` are H100-class harness
pins, not results). All numbers found are serving-level, in docs: +130% @4-way (balance_serve,
CPU-bound caveat); Qwen3-Next-80B-A3B-FP8 4×4090 TP4 decode 0%→100% GPU experts 53 → 112-114
tok/s with dynamic redistribution dominating at low ratios (`experts-sched-Tutorial.md:176-190`);
DeepSeek-V4-Flash 20+ tok/s + MTP ~1.2× (8×5090); Kimi-K2-Thinking layerwise prefill 53/184/290
tok/s @2k/8k/30k (1×4090-48GB, weight-streaming — not comparable to our resident-weights
league); 347 tok/s AMX prefill / 418 tok/s e2e (Xeon4-class CPU-offload, N/A premise); 4-5 min
startup dominated by weight load + graph capture. None of these are comparable to our
scoreboard rows; they are context, not bars.

## N/A verdicts (do not port)

- MoE offload premise (CPU experts, NUMA pools, expert placement ratios, layerwise weight
  streaming): our model is fully GPU-resident — N/A. The threshold-with-VRAM-pricing policy
  pattern (layerwise prefill switch, `kt-kernel/README.md:543-546`) generalizes as a
  documentation template only.
- Multi-GPU layer split / TP / NPU graphs / custom all-reduce: N/A.
- `moe_topk_softmax_kernels.cu` (verbatim vLLM v0.7.3 ← TRT-LLM): MoE routing, N/A.
- Marlin act-order machinery (permute_cols + g_idx staging): coded but never instantiated —
  dead here; N/A.
- GGUF dequant family (`kt-kernel/cuda/custom_gguf/dequant.cu`): load-time converter; we
  consume packed W4A16 directly — import-tool reference only (note: the archive copy carries a
  hardcoded `{32,256}` output-alloc bug that kt-kernel fixed to `ele_per_blk` — cited as a
  corpus lesson candidate, not a port).
- WDDM-specific handling: NOT FOUND.
- x86 AMX/oneDNN/AOCL/llamafile CPU kernels: ISA/library-specific — techniques-only (K-blocking
  rules queued below).
- Their kt-kernel `KVCache` block-retrieval + unhashed dump format: strictly below our
  BLAKE3-committed prefix cache — nothing to adopt.

## Corpus candidates → mining queue (queued, not landed)

kernel_opt (the Marlin cluster is the value; each needs our own fixture + bench at batch time):
1. `device-side-m-graph-stable-gemm` — per-batch relaunch/recapture → M via device pointer,
   grid=f(sms), runtime m-block dispatch, M-cap chunking (headline; feeds lanes).
2. `cpasync-multistage-tile-pipeline` — tile-boundary waits → 4-stage pipeline, wait<stages-2>,
   next group issued mid-tile.
3. `reg-double-buffer-inner-pipeline` — single operand register set → k%2 double buffer,
   fetch(k+1) before mma(k).
4. `ldmatrix-mma-layout-loads` — lane-math shared→register loads → ldmatrix.m8n8.x4 in
   fragment layout.
5. `lop3-int4-to-half2-dequant` — per-nibble shift/mask → single lop3 exponent-embed + fused
   zero-point.
6. `prmt-int8-to-half2-dequant` — per-byte int8→fp16 math → two __byte_perm + one __hsub2 magic.
7. `xor-swizzle-smem-static-index-tables` — differing read/write smem layouts → XOR swizzle
   precomputed into static index arrays.
8. `multi-pointer-dependency-break` — serially-advancing load pointer → array of pre-offset
   pointers.
9. `quant-group-scale-fetch-gating` — scales refetched every tile → cp.async only at
   group-boundary tiles.
10. `stripe-schedule-l2-lockfree-reduction` — cross-block K-split via atomics on accumulators →
    blockIdx stripes + acquire/release semaphore + partials reduced in the L2-resident output.
11. `mma-fragment-assembler-m16n8k16` — the working FragA/B/C + f32-accum PTX reference shape
    (for our Bench-786 arm fix).
12. `pipeline-machinery-for-pure-movement` — naive repack copy loops → reuse the GEMM
    cp.async pipeline (their `gptq_marlin_repack.cu` runs the same machinery at 8 stages).
13. `k-block-half-of-l2` (host/CPU-origin, transfers) — K_BLOCK sized so the weight slab ≈ ½ L2.
14. `k-block-aligned-to-quant-group` — K blocking = quant group ⇒ one scale application per block.

rust_perf/host (lower priority; x86-flavored; dedup-check against existing corpus at batch
time — e.g. `alignas(64)` status structs are B65 `align-64-false-sharing` covered):
`grow-only-arena-pointer-rebind` (shared_mem_buffer's {binding fn, size} registry over one
arena), `intensity-gated-mat-vec-twins` (tile path above ~4 tokens/expert, vector path below),
`fuse-independent-gemms-one-task-space` (gate+up as one flattened task list),
`allow-n-pending-sync` (barrier permitting N outstanding tasks), `host-callback-on-gpu-stream`,
`dual-seed-page-hash` + `recompute-ratio-guard` + `batched-evict-count` (kvc2 hardening riders),
`decode-prefill-kernel-split` (mode-selected micro-kernels — likely corpus-adjacent).

## Fusion (the novel combinations this source enables)

1. **Marlin pipeline × our cudarc int8-mma GEMM family** = the U7 arm, fully specified, IF the
   owner gates it (projected 2,270-2,437 tok/s, gate-red per B803 — this source does not
   change the closure arithmetic, it removes the implementation-risk excuse).
2. **Device-side-M GEMM × our graphed GDN+attention decode × Plan 556 lanes** = a
   batch-size-invariant captured serving graph: GDN recurrence is already shape-pinned in our
   graphs; making the GEMM legs shape-free via device-M + per-bucket static staging is the
   missing piece for lane packing without per-shape capture blowup (their graph mix encoding
   shows one replay can carry mixed-phase requests).
3. **Their EAGLE depth-1 acceptance anchor (90%, 1.2× e2e) × our chat-drafter GOAT gate**
   (P2-3 / Issue 780 U3 / riir-train Issue 493): the external bar our trained drafter must
   beat is now anchored — a K=16 lookup drafter at 15.15/16 acceptance already beats the
   EAGLE-chain acceptance class; the open question stays chat-domain generalization, which
   matches their delegation of MTP to a trained model.

## Provenance + verification

- 3 parallel read-only subagent sweeps (CUDA / CPU-host / serving-arch), coordinator
  spot-verified: Marlin_wrapper device-M (read cu:1400-1460 directly), graph pool chaining
  (grep), `test_mxfp8_moe_m3.py` (read header — MiniMax-M3 model). Two agent-report errors
  corrected in-record: (a) CPU agent claimed the m3 example file missing — it exists;
  (b) measured-numbers claims are serving-level only, no kernel numbers exist anywhere
  (verified by tree search + bench-file reads).
- Clone is disposable (`.raw/` gitignored this same commit); every load-bearing claim in this
  note carries its file:line so the note survives the clone's deletion.
