# Research 338 — vLLM lucianommartins Gemma-4 window (10 PRs, 2026-10-05..10) — distill verdict

**Status:** DISTILLED — pending owner decision (nominal Batch 297 / plan 303; commit DEFERRED — see §commit-state)

## Source

- Search: `vllm-project/vllm` PRs, `author:lucianommartins`, `created:2026-10-05..2026-10-10` — 10 PRs (GitHub search API, fetched live 2026-10-27, zed-m3).
- Inspection date: 2026-10-27. Provenance: **PR bodies only — diffs NOT read this session** (disclosed per the unread-files rule; T0 re-read at pins is a plan task). All bodies are agent-authored ("AI assistance was used") with in-body measurement harnesses (parity metrics incl. bit-exact/1-ULP censuses, CUDAGraph-replay timing, L2-flush decode timing) — unusually rigorous provenance, still never an adoption bar.
- All numbers are NVIDIA A100-SXM4-80GB, source-reported — foreign silicon, provenance, never bars.

| PR | Title (short) | State | Headline claim (source-reported) |
|---|---|---|---|
| 60254 | EmbeddingGemma2 support | MERGED 10-06 | new-model (not perf) |
| 60289 | EmbeddingGemma2 config streamline + Triton prefill head-512 smem cap | MERGED 10-07 | fixes `OutOfResources: shared memory` (294,912 B > 166,912 B on A100) via `if Lk >= 512: BLOCK = min(BLOCK, 32)` → 98,304 B |
| 60317 | Triton unified attention head_dim=512 tiling + SWA decode | OPEN | up to 1.74x prefill kernel, up to 4.29x SWA decode (ctx=8192, W=512), E2E 1.02–1.13x |
| 60445 | KV-sharing (YOCO) fast prefill enable + metadata specialization | OPEN | E2E prefill up to 2.06x / long-context 2.10x (E2B); metadata builder 4.22–5.28x; pure-decode prep 5.33–5.67x |
| 60478 | Batch vision pooler + fused vision/audio RMSNorm + 2D RoPE | OPEN | pooler batching up to 27.6x (32 frames); RMSNorm 2.83–9.40x; 2D RoPE 3.21–6.83x; E2E mm prefill 1.13–1.35x |
| 60504 | Tuned FusedMoE Triton configs E=128 (N=704/352/176/88) | OPEN | TP8 M=128 19.59x (register-spill cliff 4.865→0.250 ms); TP8 decode up to 1.83x; E2E up to 1.20x |
| 60643 | Gemma4Unified 2D posemb vectorize + batch image/video embedder | MERGED 10-22 | batch embedder 4.28–6.94x; E2E 1.01–1.04x (encoder-free, Amdahl-capped) |
| 60663 | DiffusionGemma fused self-conditioning + zero-sync batching | OPEN | component 1.72–1.85x (19→5 launches); multi-request batching up to 34.34x at B=32 (host-loop-dominated); E2E 1.14x |
| 60674 | Fused Triton QKV norm/RoPE, pre/post-FFN norms, PLE, MoE routing | OPEN (needs-rebase) | 113/113 cases ≥1.0x (1.06–4.98x); E2E prefill up to 1.90x (E2B) |
| 60689 | MTP spec-decode fusion + sparse gather-GEMV LM head | OPEN (needs-rebase) | 2.84x CUDA-graph on the MTP head; 99.6% HBM reduction; monotone-softcap argmax bypass; relaxed MTP verification (lossy, opt-in) |

## Verdict

**YES B corpus / lane-intel MODERATE / product-coverage MODERATE.**

Two of the four strongest-looking patterns died at the dedup gate as positive instances of EXISTING corpus rules (evidence below) — which is the gate working — and the remaining slate is a lean 2–4 rule kernel_opt batch led by a launch-failure-grade capacity law and an analytic-bounds law.

## Domain routing

`kernel_opt`. The tempting wrong domain is `rust_perf` for the host-side metadata/launch-overhead rules (PR 60445's arange specialization, the batching loops) — the routing table's "host-orchestration around GPU" row puts them here; the pure-Python memo/hoist candidates die at dedup against the rust_perf hoist family anyway. Nothing here routes to `sec` or `clippy_lints`.

## Ranked candidates

| # | Candidate | Grade | Source | Notes |
|---|---|---|---|---|
| 1 | `head-dim-smem-tile-cap` | **A−** | 60289 (merged) + 60317 | dtype-default tile sizing overflows the platform SMEM ceiling at head_dim ≥ 512 — the failure is a LAUNCH REFUSAL (`OutOfResources: shared memory`, 294,912 B requested vs 166,912 B available), not a slowdown; fix caps the tile by head_dim (`min(BLOCK, 32)` → 98,304 B). 60317 extends: SMEM-tiered stage/warp ladder across SM70–SM120 (≥160 KB → 1–2 stages; 96–160 KB → TILE 32 + 1 stage; <96 KB → TILE 16), `TILE_SIZE_PREFILL` 64/128 gated on `q_tokens*num_kv_heads >= 2048`, and the MEASURED small-shape fallback (0.86–0.99x on `B=1, seq=512` shapes — deliberately routed back to upstream heuristics). Two-regime: capacity-derived tiles vs dtype-default; the anti-regime half is carried in-rule by their own honest negative. Distinct from `dynamic-shared-memory-runtime-sized` (launch-time smem SIZING for runtime dims — not tile capping against a ceiling) and from `hardcoded-fast-path-jit-specialization` (config MATCHING, not capacity DERIVATION). |
| 2 | `swa-evicted-prefix-tile-analytic-skip` | **B+** | 60317 | The first in-window tile is DERIVED analytically from the window algebra — `start_tile = max(0, (seq_len − q_len − window + 1) // TILE)` — so the attention scan never touches evicted prefix tiles; plus the zero-length guard `active_tiles = max(0, num_tiles − start)` (prevents uninitialized reads) and the 3D→2D decode switch at `num_seqs*num_kv_heads >= 128` (occupancy). Up to 4.29x at long-context SWA decode. Distinct from `mask-block-classification-skip-placement` (that rule classifies tiles by READING mask bytes, data-driven; this derives bounds analytically — zero mask reads, static per shape — and clamps against the degenerate-window edge). **Main falsification risk: the T3 adjacency adjudication against that rule.** |
| 3 | `analytic-constraint-prefix-sum-arange` | **B+** | 60445 | Pigeonhole specialization: `max_logits_per_req == 1 && num_logits_indices == num_reqs` ⟹ every request contributes exactly one decode token ⟹ `decode_query_start_loc ≡ arange(0, num_reqs+1)` — one arange replaces the bucketize/zeros/ones_like/scatter_add_/fill_/cumsum pipeline (6 GPU launches) + 1 PCIe D2H transfer. Measured 4.22–5.28x on the metadata builder (~0.17 ms/prefill step), pure-decode prep 5.33–5.67x (~0.04 ms/step). The multi-logit fallback keeps the general path at measured 1.00x parity — the two-regime shape the corpus favors. Dedup: no owner (greps below); `lm-head-loss-positions-only` owns dispatch ELISION under a consumer-set predicate (different mechanism: elision vs analytic construction of a known output); `pairwise-distance-decay-matrix` uses arange for a decay matrix, unrelated. |
| 4 | `moe-tokens-per-expert-block-m-cap` | **B−** | 60504 | Tile-sizing invariant from routing arithmetic: `tokens_per_expert = M·top_k/E`; at M=256/E=128/top_k=8 that is 16 ⟹ `BLOCK_SIZE_M ≤ 32` + `GROUP_SIZE_M = 1` (avoids zero-padding waste and L2 thrash across scattered token streams). The 19.59x row is really the register-spill CLIFF (`BLOCK_M=64 × BLOCK_N=128` accumulator @ 4 warps > 255 regs → local-memory spill; 4.865→0.250 ms at TP8 M=128) — the warp-guard half may be owned by the splash Spill-series rust_perf rules (spill-tier victim-exclusion landed B270); T3 adjudicates rider-vs-own. MoE-only regime (this stack serves no MoE); product-coverage value for cargo-refine's GPU consumers. |

**Yield estimate:** 3–4 authored → 2–3 land after ~50% coordinator falsification. Lead with candidate 1 (merged-PR provenance, launch-failure half + measured-negative half).

## Dedup grep evidence (2026-10-27, corpus at Batch 295, kernel_opt 981+ rules)

Greps over `src/domains/kernel_opt/**` under both vocabularies:

- **`monotone-transform-elision-under-argmax` EXISTS** (entries_gpu_02, B25) — "a strictly monotone post-transform (softcap cap·tanh(x/cap)…) does not change the argmax — skip it entirely when the only consumer is greedy decoding", WITH the Batch-170 rider extending elision to the OUTPUT PLANE (mlxfast LogitslessGreedyHeadV2). → **PR 60689's tanh-softcap argmax bypass is a POSITIVE INSTANCE. DROP as own rule** (rider possible at most: `_suppress_idx` mask preservation + `use_local_argmax_reduction`).
- **`fuse-indirect-gather-into-dot-consumer` EXISTS** (entries_gpu_06) — "an indirect gather materializes rows to a global buffer and the ONLY consumer is a per-row dot kernel… delete the gather kernel, pass the index array into the consumer, read directly inside the dot loop". → **PR 60689's sparse gather-GEMV is a POSITIVE INSTANCE. DROP as own rule** (rider possible: 2D `(T, cdiv(S, BLOCK_S))` grid + FP32 register accumulators + the 99.6% HBM-allocation figure).
- Sliding-window greps (`sliding|window…clamp|skip|evict|prefix|start_tile`): all hits are the ring-layout/residency/undo family (`mirror-ring-store-sliding-window-contiguity`, `kv-window-device-resident-ring-append`, `append-cache-rewind-overwrite-in-place`, `rotated-ring-undo-via-rebind-reference-snapshot`, `draft-context-ring-window-bounded`, `fused-conv-state-shift-dot-silu`) — storage and lifecycle, NOT scan-bounds algebra; plus `mask-block-classification-skip-placement` (data-driven). No owner of the analytic clamp → candidate 2 clears.
- SMEM/head-dim greps: `dynamic-shared-memory-runtime-sized` (runtime smem sizing — different mechanism) and `hardcoded-fast-path-jit-specialization` (config matching). No SMEM-ceiling/capacity-derivation owner → candidate 1 clears.
- Pigeonhole/analytic greps (`pigeonhole|analytic…construct|arange|scatter_add|cumsum…skip|histogram…known`): no owner → candidate 3 clears.
- Candidate 4 NOT grepped this session (MoE tile/warp vocabulary) — T1 re-grep owed; the `moe-*` family (B268: lone-tile gemv, zero-expert routing) and the splash Spill series are the expected adjacencies.

## Drops (each with its reason)

| Dropped | Source | Reason |
|---|---|---|
| Monotone softcap argmax bypass | 60689 | positive instance of `monotone-transform-elision-under-argmax` + its Batch-170 output-plane rider |
| Sparse gather-GEMV row-stream | 60689 | positive instance of `fuse-indirect-gather-into-dot-consumer` |
| Fused norm/RoPE/PLE/MoE-routing epilogues, GELU-tanh-mul | 60674, 60663 | the fusion family is heavily owned (fusion-scope-by-barrier-cost, fused-epilogue shapes); these are instances, not new laws |
| Batched pooler / image / video / zero-sync host loops | 60478, 60643, 60663 | host-loop batching in Python idiom; the 27.6x/34.3x/4.28x rows are loop-overhead-dominated; the Rust shape ("batch per-item ops into one") is adjacent to existing batched-op families — weak own-rule |
| `lru_cache` metadata-class + backend-factory memoization | 60445 | the rust_perf hoist/memo family owns it (`regex-hoist-lazylock`, `dispatch-handler-pair-hoist`, `mask-memo-hoist-to-owner-lifetime` cited in-rule as owners) |
| Frozen full-attn layer-index set + memoized prefix clear | 60478 | hoist/precompute family |
| Pure-decode skip guard (`max_query_len > 1`) | 60445 | adjacent to `predicate-gated-skip-empty-workset-step`; thin rider at best |
| KV-sharing ring/config/YOCO auto-enable, piecewise CUDAGraph wiring | 60445 | ring family 6+ owners; the config auto-enable is vLLM-framework plumbing, not a portable law |

## Lane mapping (lane-intel axis)

**MODERATE — no league-cell mover, and our own kernels already do the headline trick.**

- The league lanes are Bonsai-27B GDN on M3-Metal/4090-CUDA; these PRs are Gemma-4 (SWA + head_dim 512/256 global/SWA mix) + EmbeddingGemma-2 on A100/Triton. No scored-cell implication, no config recipe to adopt, no decode/prefill re-pin trigger.
- Our Metal flash-attention pass 4 already window-predicates key ranges (`LAYA_METAL_FLASH`, recorded in the reflex lane) — candidate 2's value here is documentation-grade: it names the analytic-bounds law + the zero-length clamp hazard (uninitialized reads) that our windowed kernels must honor.
- Candidate 1 is documentation-grade for our CubeCL tile configs (`attention_cubecl` head_dim=256 posture): tile config must be derived from head_dim × platform SMEM budget, never from dtype defaults alone.
- The honest negatives in the PR tables (0.86–0.99x small shapes kept on the old path; 0.87–0.90x eager regressions disclosed) are the same discipline as our league's FAIL-honest rows — good provenance signal, nothing to adopt.

## Product coverage

**MODERATE.** This continues the established vllm mining lane (B265 vllm Mamba2 host-overhead, B267 vllm MLA-prefill rider, the hardcoded-fast-path vllm riders). The patterns transcribe into the corpus's existing fixture languages (CubeCL/WGSL/CUDA/Metal) — exactly what cargo-refine's GPU consumers ship. Follow-up pool (accumulation clause applies): vllm perf PRs (standing watch), `triton-lang/triton` python/triton ops + tutorials, flashinfer, SGLang kernels.

## Caveats

1. **Diffs unread** — graded from PR bodies (which carry root-cause + implementation + measured tables). T0 re-read at pins is mandatory; grades are provisional until then.
2. 7 of 10 PRs are OPEN; 60674/60689 carry `needs-rebase` — pin PR-head shas at clone; merged PRs (60254, 60289, 60643) pin at main.
3. Agent-authored PR bodies: the measurement harnesses are included and detailed, but every quote must be machine-verified against the diff at the pin (the standing law).
4. The giant multipliers decompose: 19.59x/4.29x are real kernel-shape cliffs; 27.6x/34.3x/4.28x are host-loop-overhead-dominated; E2E rows sit at 1.0–2.1x. Never quote a headline without its class.
5. Candidate 4's dedup was not run this session (T1 owes it). Candidate 2's adjacency vs `mask-block-classification-skip-placement` is the batch's main falsification risk.

## Commit-state (measured this session — why nothing is committed yet)

- The M3 riir-refine checkout is **held by a concurrent lane**: HEAD detached at `2d9cd070` ("fix: restore the queue snapshot's native CRLF line endings", Session: zed-amo-housedelta, epoch 1793069897) with THEIR uncommitted WIP on `.distill/001_mining_queue_snapshot.md` + `.issues/.highwater`; `origin/develop` has moved to `86c69fab`.
- Numbers verified UNALLOCATED on origin (`.research/.highwater` = 334, `.plans/.highwater` = 300 at fetch): this note was 335, the plan was 301 — RENUMBERED on landing (338/303): upstream took 335 (walk17) + 301 (the cubecl batch) in the allocation race while this lane's commit was deferred.
- Committing here would capture or endanger the sibling's in-flight hunks; switching branches would drag their WIP. **Deferred** — the next free cycle: re-fetch, confirm the checkout returned to `develop` (or the sibling's row landed), splice the queue row below into the snapshot (byte-level, line-ending aware — the file is natively CRLF), stage the three named paths only (`git add .research/338_… .plans/303_… .distill/001_…`), commit `docs:` and push.

## Queue row (splice verbatim, after the `# Mining queue snapshot` header line)

```text
> **DISTILL VERDICT 2026-10-27 (zed-m3/M3, user-directed @distill: vllm-project/vllm PRs author:lucianommartins created 2026-10-05..2026-10-10 — 10 PRs, the Gemma-4/EmbeddingGemma-2 optimization window; 3 merged [60254, 60289, 60643], 7 open [60674/60689 needs-rebase]; Apache-2.0; graded from PR BODIES — diffs unread, T0 re-read at pins owed).** Verdict: **YES B corpus [2-4 own kernel_opt rules, NotBenchable-annotated class] / lane-intel MODERATE [no league-cell mover: Gemma-4 A100/Triton vs our Bonsai GDN Metal/CUDA; our Metal flash-attn pass 4 ALREADY window-predicates — the SWA law documents the analytic-bounds + zero-length-clamp hazard for our own kernels; the head-dim SMEM-cap law is documentation-grade for our CubeCL tile configs] / product-coverage MODERATE [continues the established vllm lane — B265 Mamba2, B267 MLA rider; transcribes to CubeCL/WGSL/CUDA/Metal for cargo-refine GPU consumers; follow-up pool = vllm perf PRs + triton-lang ops + flashinfer + SGLang].** Ranked: A− `head-dim-smem-tile-cap` (60289 MERGED + 60317: dtype-default tiles overflow the platform SMEM ceiling at head_dim>=512 — the OOR specimen is a LAUNCH REFUSAL [294,912 B requested vs 166,912 available → `OutOfResources: shared memory`], fix caps BLOCK by head_dim [`min(BLOCK, 32)` → 98,304 B]; 60317 adds the SMEM-tiered stage ladder SM70–SM120 + TILE_SIZE_PREFILL 64/128 gated on `q_tokens*num_kv_heads >= 2048` + the MEASURED 0.86-0.99x small-shape fallback carried in-rule as the anti-regime half; up to 1.74x kernel / 1.02-1.13x E2E; distinct from dynamic-shared-memory-runtime-sized [launch-time smem SIZING vs TILE CAPPING] + hardcoded-fast-path-jit-specialization [config MATCHING vs capacity DERIVATION]) · B+ `swa-evicted-prefix-tile-analytic-skip` (60317: the first in-window tile DERIVED analytically from the window algebra — `start_tile = max(0, (seq_len − q_len − window + 1) // TILE)` + the zero-length clamp `max(0, num_tiles − start)` + the 3D→2D decode switch at `num_seqs*num_kv_heads >= 128`; up to 4.29x at ctx=8192 W=512; distinct from mask-block-classification-skip-placement [data-driven mask reads vs analytic derivation] + the ring-layout family [storage, not scan bounds] — the adjacency adjudication is the batch's main falsification risk) · B+ `analytic-constraint-prefix-sum-arange` (60445: the pigeonhole conjunct — `max_logits_per_req == 1 && num_logits == num_reqs` ⟹ every request contributes exactly one token ⟹ decode_query_start_loc ≡ arange — replaces bucketize/zeros/ones_like/scatter_add/fill/cumsum [6 launches] + 1 PCIe D2H with ONE arange; 4.22-5.28x on the metadata builder, pure-decode prep 5.33-5.67x; the multi-logit fallback keeps the general path at 1.00x parity = the two-regime shape; no corpus owner) · B− `moe-tokens-per-expert-block-m-cap` (60504: BLOCK_M ≤ M*topk/E + GROUP_SIZE_M=1 anti-thrash; the 19.59x TP8-M=128 row is the register-spill CLIFF [64×128 accumulator @ 4 warps > 255 regs] — the warp half may be owned by the splash Spill-series rust_perf rules, T3 adjudicates; MoE-only regime, product-coverage value). DROPS (dedup evidence, greps run 2026-10-27 at corpus Batch 295): the tanh-softcap argmax bypass (60689) = POSITIVE INSTANCE of `monotone-transform-elision-under-argmax` [gpu_02 + its Batch-170 LogitslessGreedyHead output-plane rider]; the sparse gather-GEMV (60689) = POSITIVE INSTANCE of `fuse-indirect-gather-into-dot-consumer` [gpu_06; rider possible: 2D (T, cdiv(S,BLOCK_S)) grid + FP32 register accumulators + 99.6% HBM reduction]; fused norm/RoPE/PLE/GELU epilogues [60674/60663 — the fusion family]; batched pooler/embedder/video host loops [60478/60643/60663 — Python-idiom, the 27.6x/34.3x/4.28x rows are loop-overhead-dominated]; lru_cache metadata-class/backend-factory memo [60445 — the hoist/memo family]; frozen full-attn index set [60478 — hoist family]; the KV-ring/config bits [ring family 6+ owners]. Yield: 2-3 land after ~50% falsification. **COMMIT STATE (measured): the M3 checkout is HELD by a concurrent lane — detached @ 2d9cd070 [zed-amo-housedelta's CRLF-restore fix, today] with THEIR uncommitted WIP on this snapshot + .issues/.highwater; origin/develop @ 86c69fab; numbers verified UNALLOCATED on origin [research 334 / plans 300] — Research 338 + plan 303 (then numbered 335/301) written to disk, the row splice + commit DEFERRED to the next free cycle [recipe in Research 338 §commit-state].** Queue after: Batch 297 = this window (claim plan-first); B279 glm + the standing watches unchanged. NOTHING INVENTED — every claim a fetched PR body this session; diffs unread and marked.
```

- **LANDED 2026-10-27 (zed-m3-175-verdict):** the deferred commit executed by the next cycle — renumbered 338/303, batch 296→297 (the cubecl #1922 batch took 296 upstream mid-race); the queue row below spliced into the snapshot with the new numbers; the P93 healer intake row (issue 182) landed beside it. The checkout-side blocker (the held detached HEAD) resolved by owner call — "sibling is done or leftover".
