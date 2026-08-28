# Research 102 — arXiv pool-refill walk (cs.PF + cs.AR + cs.DC + cs.OS + the KV-quantization lane)

**Status:** RECORD — pool refilled; no corpus batch (strategic/algorithm-level yields; the two live batches B86/B87 stay build-gated).

**Filed:** 2026-08-28, idle loop (queue items 9/11 — online unblock search + perf search, the standing arxiv-API fallback while the MCP search quota is dead until 09-15).
**Method:** `fetch` on `export.arxiv.org/api/query` — cs.PF+cs.AR (40 recent), cs.DC+cs.OS (30 recent), and one targeted lane query (cs.LG × "quantization" × "KV cache", 15). Titles+abstracts scanned; grades below are VERDICT-TIME (abstract-level), not source-read. Full source reads happen at dispatch, plan-FIRST, per Protocol A step 0.

**Why zero-build this cycle:** box load 30/35/47 with the sibling mid-build in riir-clippy's own tree (docker-domain batch, 26+ modified files, `katgpt_types` compiling) AND sibling sessions active in riir-ai (41 dirty) / riir-game-sdk (14) / riir-dapps (6) / katgpt-rs (1 untracked). Plan 337: no builds, no benches. This unit is fetch+read+write only.

---

## P1 — 2608.08081 RotaryQuant (B+ strategic, riir-ai KV lane)

Fused compressed-space attention on **Metal** (four-kernel pipeline, attention directly on packed 3-bit tensors without materializing f32 KV state), Gemma-4-26B/Qwen3-30B in 16 GB, Nemotron-H 120B in 32 GB, ΔPPL ≤ +0.0012, 100% retrieval at 32K.

**Why it matters to us — the counter-evidence shape:** its IsoQuant KV axis applies a Walsh–Hadamard transform + **block-diagonal SO(4) rotations** to isotropize activations before 3-bit scalar quantization at **O(d log d) ops and 256 stored params/head** (vs O(d²)/16,384 for dense rotations). Our Issue 716 sink-guard A/B REJECTED the Hadamard arm on a CPU-simulated "19.6× reduction but nonzero + strictly larger kernel cost" verdict — that simulation had NO cheap-rotation construction and NO fused kernel. RotaryQuant (with SPECTRA/NOVA-KV/OptR, below) is the measured counter-family: rotation-before-quantization is now the dominant KV-compression pattern, and the O(d log d) parametrized rotation invalidates the "rotation cost is strictly larger" premise at kernel level. **Action when the riir-ai KV lane reopens (716 follow-up / 753 long-ctx / 769): re-arm the Hadamard/SO(4) arm against the f32 sidecar with (a) the parametrized rotation (256 params/head) and (b) a fused packed-quantized attention kernel, not the CPU simulation.** Also note the "different execution model, not just a quantization scheme" framing — attention over packed low-bit KV is a candidate execution shape for our qwen38 verify path (Issue 742's C axis) if the owner re-opens quality-tolerance work there.

Source-read TODO at dispatch: the four-kernel split, the 3-bit pack layout, whether the SO(4) block-diagonal is per-head or shared, the outlier-channel handling vs our sink rows (their isotropization may make sink-guard sidecars unnecessary — or not; MA channels survive rotations differently than the Issue 716 synthetic MA injection measured).

## P2 — 2608.23658 Elastic KV Cache (B strategic, riir-ai KV lane, honest-negative)

Builds an elastic prefill-reserve reclaim mechanism (pure-userspace CUDA VMM: two physical handles in one contiguous virtual range per layer, decommit ms / recommit tens-of-ms, CUDA-graph compatible, never OOMs), then **premise-tests it and reports the honest negative**: chunked prefill already closes the gap (median TTFT differs ~1% between chunk 8192 and 32768), lowering `max_num_batched_tokens` recovers more KV at near-equal latency, and the reserve dilutes 16% → 2.7% of KV from TP1 → TP4. Releases the mechanism as a reusable userspace elastic-VMM allocator anyway.

**Why it matters:** (a) the methodology IS the house style — build the mechanism, test the premise, publish the negative with the precise reopen conditions ("we state precisely when reclaiming the reserve could still help"); (b) the numbers feed our KV-sizing reasoning wherever a fixed reserve is assumed (Bench 751's KV accounting, Issue 753 f16-KV headroom claims); (c) the "static commit of the same memory is unsafe, crashing on prefill bursts" finding is a hazard class worth remembering if we ever pool KV scratch. Note-only — no Rust fix-space.

## P3 — 2608.21693 MoEXBench (B strategic citation, the Issue 750 doctrine)

Systematic benchmark of COMPOSABLE MoE compression (10 models, expert pruning × weight quant × KV precision, individually AND stacked). Headline findings: **composable compression cannot be predicted from standalone techniques; compression rate alone does not predict quality loss; expert pruning is the dominant degradation source; average quality hides workload/architecture-specific failures.**

**Why it matters:** external, pre-registration-grade validation of our lossy-surface doctrine — the Issue 750 T3 rule ("aggregate perplexity can be flat while family-conditional behavior flips", adopted 2026-08-28) and Bench 696's KVarN sink-guard gate both say exactly this. Cite it as the independent arrival alongside Research 502 + Bench 696 in any future lossy-surface promotion argument. Also note "expert pruning is the dominant degradation source" as a prior for any future MoE work.

## P4 — weak pointers (no action, recorded for completeness)

- **2608.25185 RCheck** (pgvector per-query recall adaptation: raise effort below target recall, lower above): strategic pointer for riir-rag IF Plan 524 phases 7–9 ever unblock — per-query adaptive k/effort on `ShardIndex::retrieve_diverse` is the transplantable idea. Not corpus (no bounded Rust fix-space).
- **2608.25683 psRL** (RL training-time prefix sharing, KV block reuse across samples): riir-train lane pointer (GRPO/RLVR prefix redundancy). Routes via the research skill, not here.
- **2608.25469 HSMA-TRSM** (GPU TRSM, 2.05× over cuBLAS): **DROP** — TRSM has zero in-tree consumers (no LU/Cholesky in any served path); the techniques (seven-stage smem pipeline for double-complex, diagonal block decoupling) are TRSM-shaped; the "offline profiling → compile-time config selection" half is already in-tree (autotune + the Bench 641 record). Below the B79 precedent bar (that rule generalized to our GDN recurrence; this does not generalize to anything we run).
- **2608.18147 ECASQ** (entropy-constrained stochastic quantization): **DROP** — unbiased STOCHASTIC quantization contradicts the determinism contract on every surface we gate (bit-identity G1s); no consumer.
- **2608.14191 Anatomy of a Quantized Agent** (honest-negative: closed-form VRAM models beat learned in weight-dominated regimes; complex predictive models unjustified): methodology note only — same shape as P2, useful the next time someone proposes an ML VRAM/latency predictor.
- **2608.11212 Route-flip causal apparatus** (pre-registered RMF attribution for 4-bit KV damage in MoE): methodology citation for lossy-surface forensics; the "detecting a flip is easy, sign-attribution is at chance" result is a caution for any selective-repair idea. Note-only.

## Lane-level observation (the walk's meta-yield)

The KV-compression literature has converged on **rotation-before-quantization** (SPECTRA's statistics-derived rotation past the 2-bit cliff; RotaryQuant's SO(4); NOVA-KV's attention-preserving non-orthogonal key transform; OptR's output-aware rotation through W_O) as THE mechanism that buys low-bit viability — while our stack's only in-tree rotation comparison is Issue 716's CPU-simulated Hadamard loss. If any riir-ai KV work re-opens, the first-class question is: does a parametrized O(d log d) rotation + fused packed-attention change the 716 verdict? Secondary observation: the algorithm-level KV wave (PuzzleKV page-wise low-rank, AnchorKV anchor-residual, JoLT Tucker, QEvict recoverable eviction) is rich but has no Rust/kernel fix-space for us — it belongs to model/algorithm owners, not the healer corpora.

## Dispositions

| Candidate | Grade | Lane | Disposition |
|---|---|---|---|
| 2608.08081 RotaryQuant | B+ | riir-ai KV (716-follow-up/753/769) | POOLED — re-arm evidence for the rotation arm |
| 2608.23658 Elastic KV Cache | B | riir-ai KV sizing | POOLED — honest-negative + hazard note |
| 2608.21693 MoEXBench | B | Issue 750 doctrine citation | POOLED — cite in future lossy-surface gates |
| 2608.25185 RCheck | B- | riir-rag (blocked Plan 524) | note-only |
| 2608.25683 psRL | B- | riir-train | note-only pointer |
| 2608.25469 HSMA-TRSM | C+ | — | DROPPED (zero in-tree consumers) |
| 2608.18147 ECASQ | C | — | DROPPED (stochastic ≠ our determinism contract) |
| 2608.14191 / 2608.11212 | note | methodology | note-only |

**Corpus deltas: NONE.** Zero rules added — all yields are strategic/algorithm-level. The pool (next corpus-grade external candidates) remains: B86 Big Pineapple (Plan 047) and B87 TTT-KVB (Issue 054), both queued and build-gated; this walk's strategic entries ride in THIS note until their lanes re-open.
