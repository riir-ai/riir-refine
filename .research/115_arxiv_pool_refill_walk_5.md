# Research 115 — arxiv pool refill walk #5 (2026-09-02 window differential)

**Status:** RECORD — read-only intake walk; zero corpus delta. One A− candidate
queued for source-verification; two lane records filed. Next window: 09-03+
(Wed-09-02 submissions land Thu ~07:00 +07).

Walk executor: idle-loop subagent (read-only lane). Coordinator intake + this
record: same session. Predecessor: [Research 113](113_arxiv_pool_refill_walk_4.md)
(walk #4, frontier snapshot 2026-08-31T15:55Z, spec-dec lane anchor: DBloom).

## Method + window mechanics (honest notes)

- **Surface substitution:** `export.arxiv.org/api/query` returned 429/503 for the
  first six attempts (burst + cooldown); recovered for one successful spec-dec
  pass, then throttled again on the `id_list` confirmation. Primary surface
  became the `arxiv.org/search/` UI (same corpus, different endpoint) +
  `cs.DC`/`cs.PF` recent listings. All three task queries executed; every row
  below is quoted from fetched output — nothing fabricated.
- **Announcement mechanics:** no new announcement day passed since walk #4
  (Tue-09-01 20:00 ET batch; the Wed-09-02 batch lands Thu 09-03 ~07:00 +07), so
  papers *submitted* 09-02 do not exist publicly yet. The genuine differential
  vs walk #4: (a) submissions 08-31T15:55Z → now, (b) the **Sept-ID (2609.x)
  wave**, which walk #4's export-API snapshot structurally could not see, (c)
  two NEW query surfaces (KV+eviction, quant+inference — walk #4 ran
  PF/DC/AR/spec-dec only), exposing 08-2x papers no walk ever captured.
- **Query-coverage gap found in walk #4:** its spec-dec query carried
  `AND cat:cs.LG`, which hid cs.CL-primary papers (VAT, ReTrace) *older than its
  own frontier*. Standing fix: drop the `cat:cs.LG` constraint from the spec-dec
  query in future walks.
- Exact UTC timestamps verified via export API for the spec-dec lane; three
  quant-lane papers' dates rest on search-UI display (API throttle) — flagged
  per row.

## Differential table

| # | ID | Title (short) | Window | Lane | Grade | Mechanism (abstract-only) | Composes with |
|---|---|---|---|---|---|---|---|
| 1 | **2609.00363** | Deterministic LLM Inference Across GPU Kernels: PoT INT8 Scales & Limits of Tolerance-Based Conformance | Sept-ID wave ⚠ search-UI says "submitted 24 Aug" — announcement-hold artifact; not visible to any walk | **kernel_opt / measurement doctrine** | **A−** | Fault-injection audit of INT8 GEMM conformance (9 faults × 8,232 cells, Qwen3-1.7B): five *epilogue* faults (scale precision, double rounding, mult order, truncation, fused ordering) each move output ≤1 bf16 spacing → a 1-spacing tolerance is **blind to the class by construction**; accumulator-precondition/operand-sharing breaks are always caught. Fix: requantize every weight scale to nearest **power of two** → CUTLASS and Triton agree **bitwise** at every linear layer (196/196, 252/252 vs 8/196, 10/252), byte-identical token streams at 1.7B/8B/14B; ppl ≤ +0.76% at 14B. A prior +157% ppl claim was a probe artifact (scales rewritten without requantizing weights). | **Issue 776/779 → Benches 802/804** (our flat 5e-3 f16-KV tolerance verdict — this paper argues flat tolerances are structurally blind to an epilogue-fault class); **Bench 728** (Vulkan `OpFDiv ≡ div.full` — a live cross-kernel rounding divergence of exactly this class); **lossy-surface doctrine** (PoT = a lossless-ification transform making cross-backend bit-identity *achievable*); B55/B92 quant-kernel corpus |
| 2 | **2609.00066** | OCGQuant: Outlier-Companion Grouping for NVFP4 | Sept-ID wave (submitted 08-30 per search UI ⚠) | kernel_opt (quant layout) | **B+** | NVFP4 microscaling: one large activation dominates the block scale → "Collateral Quantization Error" on everything sharing the scale. Fix (PTQ, calibration-side): **re-group channels** so outliers pair with low-magnitude companions within blocks; RTN-speed prefill, lowest WikiText-2 ppl among evaluated PTQ. | **B55 `outlier-span-escapes-quantization-via-lossless-sidecar`** (Issue 716 q8kv sink guard — same failure physics, weight/activation side; *grouping* is the un-tested third family after sidecar + Hadamard rotation); **KVarN**; ⚠ composition constraint: channel grouping = permutation → **svcca hot-swap gate detects Permuted 8/8** (Bench 836) |
| 3 | **2609.00355** | GLANCE: One-Pass Block Drafting for Lossless SD in VLMs | ⚠ boundary: submitted 08-31T20:50Z, 3h9m past walk-#4 frontier (API-verified); unambiguously outside walk-#4's captured set | spec-dec league lane | **B−** | Block-diffusion head reads the target's already-fused vision-language state (zero drafter-side vision cost), fills a block in one pass, tree-verified in one target pass; greedy-exact. "Accepted length is set by the target's next-token entropy"; grounded workloads enter a **verbatim-copy regime** where block drafting wins. | DBloom (walk #4's lane anchor), Benches 746/747 (DFlash2 loses composed), Bench 754 doc-repro (our lookup lane 15.15/16 acceptance IS the verbatim-copy regime — owned training-free); reinforces the ceiling-bin histogram telemetry adoptable (walk #4) |
| 4 | 2609.01068 | OUTLETS: Output-Length Prediction from SD Backbones | ✅ strict window (09-01T11:00Z, API-verified) | serving/scheduler | C — not-mineable | EAGLE-3 draft latents predict output length; regression head on the SD backbone; P99 −34.8% under disaggregated serving scheduling. | none (vLLM scheduling; no bounded fix-space for our corpus) |
| 5 | 2609.00796 | SFAD: Speculative Factuality-Aware Decoding | ✅ strict window (09-01T06:46Z, API-verified) | not ours (alignment) | C — not-mineable | DPO-trained faithful drafter + "epistemic friction" hallucination gate + logit steering; 2.48×. | none (trained faithfulness machinery; riir-train adjacent at most) |
| 6 | 2609.00450 | HBQ: Hierarchical Block Quantization (MICRO) | Sept-ID wave (submitted 08-31 ⚠) | hardware/ASIC | C+ — not-mineable | Large-block BQ + "significand (SIG)" second-level scaling; W4A5 ≈ W4A16 accuracy; 28nm ASIC incl. KV BQ. Noted for the direct tension with #1 (SIG-vs-PoT scaling — determinism vs accuracy per-bit trade). | none in-software; contrast note vs 2609.00363 |
| 7 | 2609.00718 | Closed-Loop Capability-Loss Eval of Compressed Driving Policies | ✅ strict window (submitted 09-01) | doctrine echo only | C — no batch | Stage-wise closed-loop eval of pruning→distillation→quantization; aggregate scores hide per-stage capability loss. | external echo of our lossy-surface deployed-path doctrine (Issue 750 T3) — cite-class only |
| 8 | 2608.30386 | DASC: Decay-Aware State Compression (GDN/KDA serving) | ⚠ query-gap item, not window: submitted 08-31 (pre-frontier), invisible to walk #4 because it ran no KV query; surfaced by the new KV+eviction surface | **GDN league lane** | **B−** | Derives **retention horizons** per head/channel from GDN/KDA decay structure; keeps only long-horizon units in ragged state checkpoints; 2.63× checkpoint compression, TTFT −42.6% at fixed budget; suffix-refresh on reuse. | **Bonsai GDN lane directly** (our gated-delta kernels, Benches 734/726/831, TreeWY Batch-82 distill); serving-system grade — research-note routing only, no corpus fix-space |

**Residue (out-of-window 08-2x, new-to-program via the two new query surfaces):**
VAT 2608.30135 (verification-aware draft training — same riir-train routing as
walk-#4's 2608.30252; also evidence of the `cat:cs.LG` recall hole); ReTrace
2608.29748 (DFlash rejected-trajectory conditioning — LiLiCorr/xPress/DBLAST
family); SemKV 2608.28911 (KV quality-cliff under a multi-seed statistical
protocol — the *instrument class* of our lossy-surface promotion rule);
Trust-the-Mass 2608.25230 (largest-weights near-optimal; published eviction
margins come from **byte accounting** — measurement-doctrine echo); TwinKV
2608.27128; Speculative Probing 2608.28099; ASD v2 2608.03447 (lossy acceptance
under a regret budget — anti-lossless, recorded as the doctrine's boundary).

## Shortlist (coordinator decisions)

1. **2609.00363 (A−)** — the only A-grade of the window. Source verification
   needed before any distill: (a) is the PoT requantization a deterministic
   per-tensor weights-only procedure we can mirror; (b) does the ≤1-spacing
   blindness argument transfer from INT8-weights to **f16-KV** (our Issue
   753/776 dtype); (c) whether it upgrades Bench 802's "flat budget stands"
   into a *classification* of which faults a flat budget can/cannot see. If it
   holds: composes with the cross-backend bit-identity gates (Bench 728) as a
   modelless determinism lever. **QUEUED — research-skill distillation when a
   session picks it up (arxiv-walk #6 window or the next idle mining cycle).**
2. **2609.00066 (B+)** — one source question decides it: is the grouping a
   **pure channel permutation** (load-time gather, zero hot-path cost → cheap
   A/B against the B55 sidecar on our q8kv/KVarN fixtures) or
   calibration-search-coupled (drops to C)? The svcca Permuted-detection
   constraint must be designed around either way. **QUEUED BEHIND #1.**
3. **2608.30386 DASC (B−)** — **lane record only** (GDN serving context);
   no batch.

**Window verdict (honest):** the *strict* 09-01-submission window contains only
OUTLETS + SFAD (both not-mineable) → read alone, the window would be "NOT
FIRED." The differential's value came from the Sept-announcement wave (2609.x)
that walk #4's snapshot structurally could not see — one A− and one B+.

## Queue state after this walk

- Corpus: **UNCHANGED** (426 kernel_opt + 97 rust_perf + 45 clippy_lints +
  12 sec + 31 docker; Batch 96 = the corpus head). Zero batches queued by this
  walk; the two queued items above are RESEARCH-VERIFICATION tasks, not corpus
  batches until their source checks pass.
- Lane state: spec-dec league lane gains GLANCE + the VAT/ReTrace residue; GDN
  lane gains DASC context; the ceiling-bin-histogram adoptable (walk #4)
  unchanged — still rides the next verify-loop bench touch in riir-ai.
- Next re-walk window: **Thu 09-03 ~07:00 +07**. Standing fix: drop
  `cat:cs.LG` from the spec-dec query.
