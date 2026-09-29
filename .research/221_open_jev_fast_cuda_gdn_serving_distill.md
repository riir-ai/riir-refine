# 221 — open-jev-fast: fused-CUDA GDN + tree-attention serving distill (lyuyiqi)

**Status:** DISTILLED — pending owner decision (nominal Batch 189 staged behind the sibling's 188; conversion cargo-deferred per Plan-337 load discipline; reflex-side lane candidate filed as riir-reflex Issue 054)

## TL;DR

`lyuyiqi/open-jev-fast` is a hand-written-CUDA inference backend for Open-Jev-27B (Qwen3.8-27B base, GDN+FA hybrid — the SAME architecture family as our league lane's qwen3.8-27B/Bonsai models): 256.0 → 17.3 ms per multi-candidate scoring request on one B300 (14.8× over PyTorch, 6.0× over PyTorch+FLA), JevBench mean 258 → 42 ms at 197/231 accuracy (one near-tie flip). Verdict: **YES, B corpus (kernel_opt) / LANE INTEL HIGH** — the source carries (a) a measured two-chunk state-independent overlap scheduling rule for serial chunked recurrences (zero corpus hits, bit-identical by construction, 11 vs 24 barriers), (b) a bit-visibility block-skip tree-attention kernel with a MEASURED dispatch cross-over vs cuDNN SDPA (≤384 rows wins; 576 rows cuDNN wins), (c) exact full-bf16-domain activation LUTs, plus a documented measured-negative list (gated-RMSNorm-into-GDN-epilogue SLOWER, L2 weight prefetch +0.18 ms, PDL rejected, exhaustive cuBLASLt search unnecessary, FP8 rejected on numerics) and a league-mapped GDN prefill lead for riir-ai.

## Source + provenance

| artifact | pin | license |
|---|---|---|
| `github.com/lyuyiqi/open-jev-fast` | `c52b8bb958c1f0d241d4eb7fce4ecd8d885bf1e4` | MIT (code); docs page CC BY-SA 4.0 |
| `github.com/Zefan-Cai/Open-Jev` (the served upstream, wire schema only) | `3308a15ccd7eea1df7a37d6ddc39b023b801ba16` | MIT |

Inspected 2026-09-29 (M3, this session). Author: Yiqi Lyu, Northwestern (Zhaoran Wang's group). Project page: <https://yiqilyu.me/open-jev-fast/>. Both clones were `.raw/` ephemerals; deleted after this note committed; quotes below re-verified at the pins before deletion.

Repo shape (complete, not WIP): `src/kernels.cu` (4,775 L — six fused elementwise kernels + the GDN kernel family `gdn_fused…6` + tree attention `fattn_k`/`fattn2_k` + split-K RMSNorm-reduce kernels), `src/fastmodel.py` (482 L — weight merge, prefix tree `gtree_build`/`forward_gtree`, env-knobbed variant selection with in-source rejection notes), `src/lt.cpp` (190 L — cuBLASLt per-shape algo timing wrapper), `src/server.py` (160 L — shape-bucketed CUDA-Graph cache + jev.server shim), `tests/` (correctness vs the original at kernel / 64-layer / probability / JevBench levels), `bench/logs/r4/` (the recorded negatives), `results/` (per-task JevBench rows). 79 stars, 15 commits — young but COMPLETE at the pin; every module read for this verdict is finished code.

## What the source does (the ladder)

256.0 ms (PyTorch default) → 103.7 (PyTorch+FLA) → 32.4 (phase 1: LoRA merge, RMSNorm fuse, 2 CPU-sync removals, whole-model CUDA Graph, torch.compile) → 20.1 (phase 2: six fused kernels, GEMMs 9→4/layer, ~5000→945 launches) → **17.3 ms** (phase 3: hand-written GDN kernel 74→36.5 µs/layer, tree attention 21.4→12.7 µs/layer, GVA q/k prep 26→12.5 µs, LUT activations, 642 launches). Two-level prefix tree (root/question/candidate) computes shared text once — 574 → 277 computed rows on the example request; linear-attention layers run each candidate over its full path (recurrence semantics preserved); full-attention layers use an ancestor mask; long single-question prompts hand the prefix's final recurrent state to each candidate (`fla_mode="state"`). Correctness discipline: kernel-level element diffs vs PyTorch/FLA/cuDNN references, 64-layer hidden-state diffs, probability drift 0.0021 @539 tok → 0.019 @10,722 tok with decisions unchanged, JevBench 197/231 vs original 198/231.

## Dedup gate (both vocabularies, kernel_opt corpus @ Batch-187 state, 675 rules)

| candidate pattern | source vocab greps | corpus vocab greps | verdict |
|---|---|---|---|
| two-chunk state-independent overlap | `two.chunk`, `side.by.side`, `state.pass`, `barrier` | `chunk.boundary`, `cross.chunk`, `prologue`, `overlap` — only hits: `cross-chunk plane_sum` bug record (different class), `prologue` constant-fill (delta-0 operands, different mechanism) | CLEAR — novel |
| ancestor-mask bit block-skip tree attention | `tree.attention`, `ancestor`, `mask.bits` | `block.skip`, `skip.block`, `mask.bits`, `bitmask`, `visibility` — ZERO hits | CLEAR — novel |
| exact bf16-domain activation LUT | `lut`, `65536` | `lut`, `lookup`, `activation.table`, `bit.exact` — hits are 16-entry ternary subset-sum LUTs, `lut16-pshufb` byte expansion, `fused-vector-site-transcendental-form-nonportable`, `bit-exact-dispatch-fusion-boundary-emulation` | CLEAR — same family (table substitution), different decision content (exact-transcendental replacement keyed on the full 16-bit input domain); re-read the two bit-exact hits at conversion time |
| shape-bucketed graph cache LRU | `graph cache`, `bucket` | `cuda-graph-scalar-via-device-pointer` — the copy-in-at-replay discipline is a POSITIVE instance of that rule; the bucketing+LRU+capture-amortization decision is new | PARTIAL — keep as dispatch rule, cite the positive instance |
| split-K partials reduced in consumer epilogue | `splitk`, `rmsnorm` | `multiplexed-grid-axis-partition-modes-exclusive` (CloudRift split-K), `split-k-two-pass-attention-decode-long-context` | ADJACENT — the fold-at-consumer placement may read as a rider, not a standalone rule; grade at conversion |
| GDN chunked recurrence core | `gdn`, `delta.rule`, `deltanet` | the linear-attention recurrence family is the corpus's LARGEST cluster (17 rules; popcorn `gated_delta_rule`/`gated_delta_product`; `row-owned-register-blocked-recurrence`; `first-order-affine-recurrence-associative-scan`; `deltanet_chunked`) | the CORE is covered — only the SCHEDULING delta (candidate 1) clears; the kernel itself is a reference implementation, not a rule |

## Ranked candidates

| # | pattern (proposed rule id) | source file | grade | decision content |
|---|---|---|---|---|
| 1 | `two-chunk-state-independent-overlap-serial-chunk-recurrence` | `kernels.cu` `gdn6_item` (round 4 C6d) | **A−** | In a one-block-per-(sequence, head) chunked serial recurrence, every per-chunk step that does not depend on the carried state (gate cumsum, beta scaling, K·Kᵀ, the (I+A)⁻¹ solve, u/w, the output attention matrix) for chunk n+1 executes inside chunk n's barrier interval — the two chunks' serial solve chains run side by side; only the state handoff is sequential. Measured: 11 barriers vs 24; bit-identical ("skipped MMA k-steps only ever added exact zeros"); covers 64+1..32-row paths. Anti: single-chunk paths; smem must stage both chunks (R=96); regime = chunked recurrences with a state-carried dependency (GDN/GLA/RWKV family) |
| 2 | `ancestor-mask-bit-visibility-block-skip-tree-attention` | `kernels.cu` `fattn_k` (round 4 C8) | **B** | Tree mask arrives as per-row bit words (`vbits[n][w]`); the CTA unions visibility across its 16 query rows and skips 32-key blocks no row can see; FA-2-style online softmax in exp2 domain; fused sigmoid gate. **Measured dispatch arm: beats cuDNN SDPA only ≤384 packed rows (288 rows: 12.7 vs 21.4 µs; 576 rows: 35.2 vs 34.2 µs — cuDNN wins above)** — the cross-over IS the rule's regime boundary, source-measured |
| 3 | `exact-bf16-domain-activation-lut` | `kernels.cu` `act_tables_k` + `silu_mul_lut_k` | **B** | bf16 inputs span exactly 65,536 values → build SiLU (bf16 out) + sigmoid (f32 out) tables once at load; the hot kernel is a gather, bit-exact to the reference ("sigmoid_f inline: identical to the table entry"). Regime: scalar unary transcendental ops on 16-bit inputs; anti: f32 inputs (table infeasible); measured anti-data: folding the gated RMSNorm into the GDN epilogue with inline sigmoid was REJECTED as slower (GDN_VER 5) — the LUT wins where the op stands alone, loses where folding removes the op |
| 4 | `shape-bucketed-graph-cache-lru` | `server.py` `Entry`/`EntryState` + `_graphs` | **B−** | CUDA Graphs cached per (path, row-bucket 32, candidates, path-bucket 16) key, LRU-capped at 512; inputs copied in at replay (a positive instance of `cuda-graph-scalar-via-device-pointer`); capture cost 1–3 s amortized across the bucket |
| 5 | split-K fp32 partials reduced inside the residual+RMSNorm epilogue | `add_rmsnorm_sk2_k`, `SK_LIN_OUT`/`SK_DOWN` = 2 | **C+** | Fold-at-consumer placement (no separate reduce launch); split-K 2 beats 4 by ~0.2 ms at M≈288 (source-measured). Likely a rider row on the CloudRift split-K rule rather than standalone |
| — | measured negatives (annotation feed, NOT rules) | `fastmodel.py` env comments + `bench/logs/r4/` | — | gated-RMSNorm-in-GDN-epilogue (GDN_VER 5 "rejected: slower") · L2 weight prefetch "+0.18 ms, no GEMM gain" (PF) · programmatic dependent launch rejected · persistent kernel not the winner · exhaustive cuBLASLt algo search rejected ("the large GEMMs were already optimal" — heuristic timing suffices) · FP8 GEMMs measured (14.6→7.8 ms) rejected on numerics · prep-fused-into-attention (fattn2 "rejected: 36.7 us") |

## GOAT candidate test (candidate 1)

1. **Benchable:** yes — measured-GOAT distillation shape (the B49/B52/B54/B55 precedent): our GDN chunked prefill kernels (riir-gpu CUDA on the league lane; riir-infer deltanet CubeCL/Metal lanes) can adopt the overlap and re-measure in the league prefill matrix (pp2048/pp4096 rows). Not a Pareto-validator rule; annotated accordingly.
2. **Exact semantics:** bit-identical claimed by the source with the mechanism argument (skipped k-steps add exact zeros) AND verified by their two-level test suite (kernel element diff + 64-layer hidden state + end-to-end probabilities + JevBech). Our port must re-prove on our kernel.
3. **Magnitude story:** barrier count 24 → 11 per 2-chunk item; per-layer 74 → 36.5 µs on B300 — a mechanism-level serialization argument (the state-carried chain is 1–4 warps deep; everything else is state-independent), not "probably faster".
4. **Deterministic:** same input, same output — bit-identity is the design constraint.

All four pass → the GOAT lead. NOTE: it is a scheduling pattern in CUDA/PTX source (not a bounded syntactic fix), so it lands as decision-content + reference-implementation citation (the B136 shape), graded on the measured-GOAT re-measurement offer.

## Lane-intel axis (MANDATORY — this axis is the verdict's center of mass)

**(a) Perf league (riir-ai, investment #3) — MAPPED, HIGH.** The league's qwen3.8-27B/Bonsai lane is a GDN+FA hybrid on the SAME base architecture (Qwen3.8-27B). The 2026-09-24 re-pin record itself names the mechanism this source attacks: *"the real 4090 delta sits between +1.3% pp512 and the +6% Ada-4070 headline, longer prompts weight the serial GDN recurrence"* — the league's prefill row (GREEN 1.185×) is capped by exactly the serial chunked-GDN recurrence candidate 1 pipelines. Transferable: the scheduling idea + the bit-identity discipline + the negative list (gated-RMSNorm-into-GDN fusion is SLOWER — an anti-lead that saves a fork experiment; PDL rejected; exhaustive algo search unnecessary). NOT transferable: absolute numbers (B300 sm_103 @1,100 W vs our sm_89 4090; `-gencode` port untested upstream), the multi-candidate prefix-tree shape (league is single-stream), FP8 (rejected by the source itself). **Recommendation (detection-only, the TensorFold-220 precedent): the riir-ai-owning session should read candidate 1 against `riir-gpu`'s GDN chunked prefill as a measured-GOAT re-arm for the prefill row; reference = `open-jev-fast @c52b8bb9` `gdn6_item` + `bench/logs/r4/v6_e2e.log`.** The league watch row should also note the source as an OPPONENT-ADJACENT reference (not an opponent itself — not in the league matrix).

**(b) riir-reflex (the session's named target) — TWO mappings, filed as Issue 054.**
1. **New lane candidate `openjev`: Open-Jev-27B-v1.1 on typed_decisions.** The wire is PINNED compatible: Open-Jev `POST /v1/{inference,systemone}` takes `{"state", "questions":[{"id","type":"choice"|"score"|"noul",...}]}` — the exact decision_wire vocabulary; noul expands to `["no","yes"]`; score = 2–10 descriptive levels; answers carry `probabilities` + `confidence`. A reflex lane is a thin translation shim (agentjev sends `type:"boolean"` for noul and an options OBJECT; Open-Jev wants `type:"noul"` and a descriptions list). Value: does the 27B Jev model beat AgentJev-0.6B 0.7715 / laya-typed 0.7445 on OUR gold labels? **BLOCKED on hardware: ~98 GiB bf16 (the MODEL, not the backend — the original jev.server needs it too); our 4090 = 24 GB; the source rejects FP8/quantization on numerics. Re-arm: any ≥98 GB GPU window (cloud B300/H200 class) or an owner-sanctioned quantized serve.**
2. **Prefix-state handoff serving lead (latency, accuracy-neutral).** reflex's agentjev lane already batches all questions of a case into ONE request because the state is shared; OUR laya lane re-encodes the shared case state per question on typed_decisions (5-question cases). open-jev-fast's `fla_mode="state"` path computes the shared prefix once and hands its final GDN recurrent state to each candidate — exact (the recurrence is sequential; no SuffixReplay-style approximation), and laya IS GDN-family (riir-infer deltanet substrate). Applies to riir-infer-laya serving + the harness's per-question round trips. NOT urgent (comparison lanes, not production serving) — filed for the owning sessions.

**(c) riir-infer (substrate) — recommendation only.** The source is the GDN lane's complete reference implementation for chunked-prefill scheduling + bit-exactness discipline (preserve every bf16 rounding point of the FLA reference — the G5-parity analog), complementing the TensorFold EXL3 reference (Research 220) on the other lane family. Detection-only per precedent.

**(d) League/lane-intel queue line:** recorded in `.distill/001_mining_queue_snapshot.md` this commit.

## Product-coverage axis

N/A — single-model CUDA serving source; no famous-lib family (tokio/rayon/serde/bevy) in play. Stated for completeness.

## Honest caveats

- **B300-only measurements.** Every number (17.3 ms, 36.5 µs, 12.7 µs, the 384-row cross-over) is one GPU (sm_103, ~1.1 kW). The 384-row cross-over especially is a B300/cuDNN-relative bar — our 4090 + our SDPA alternatives must re-measure before adopting candidate 2's arm.
- **Single-model specialization.** Qwen3.8-27B shapes are hardcoded (HD=256, head counts, 64 layers, `layer_types`); rules must be authored shape-generic with the source as reference, never ported verbatim (fixture hygiene: original minimal reproductions only).
- **Language mismatch.** Source is Python + CUDA-C++ (torch `load_inline`); corpus fixtures are Rust/CubeCL/Metal — rules land as pattern + reference citation, and candidate 1 is explicitly a scheduling decision (not a syntactic rewrite), the corpus's established NotBenchable-annotated class when no validator hosts the win metric.
- **GDN corpus density.** The recurrence family is the corpus's largest cluster; candidates 1–2 cleared dedup greps at the Batch-187 state, but the final read of the four kinship rules (`row-owned-register-blocked-recurrence`, `first-order-affine-recurrence-associative-scan`, `bit-exact-dispatch-fusion-boundary-emulation`, `multiplexed-grid-axis-partition-modes-exclusive`) is a Plan-185 conversion task.
- **Wire schema of Open-Jev verified at `3308a15` only** (api.py + server.py); a future lane implementation re-pins.
- The 0.6B AgentJev model the reflex lane serves is a DIFFERENT model — no serve swap exists for the current agentjev lane; open-jev-fast does not serve it.

## Queue slot

Nominal **Batch 189** (staged behind the sibling's in-flight 188 — TensorFold/MLX, Plan 184). Conversion plan: `.plans/185_open_jev_fast_kernel_pack.md`. Corpus conversion needs cargo — deferred per Plan-337 load discipline to the next quiet slot, exactly the Plan-184 precedent. Reflex side: `riir-reflex/.issues/054_openjev_lane_and_prefix_state_handoff.md`.
