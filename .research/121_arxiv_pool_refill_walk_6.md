# Research 121 — arxiv pool refill walk #6 (2026-09-03 window differential)

**Status:** RECORD — read-only intake walk; zero corpus delta. The strict window is
NOT FIRED for mineable content (one C-grade row). Value is in the new-to-program
residue: two kernel-shaped source-verification candidates QUEUED (2608.15764
offered-load knee, 2608.15531 FlashQuant sparse-dense fusion), one A− doctrine
citation (2608.12700 contract-grade kernel verifier), two rider candidates (Bole →
B82, rl-triton → B79). Next window: Fri 09-04 ~07:00 +07.

Walk executor: idle-loop session (read-only lane), 4090 box while the Plan-371
Stage-1 Sophia-vs-AdamW window runs GPU-resident (CPU/network-only unit, GPU
untouched). Predecessor: [Research 115](115_arxiv_pool_refill_walk_5.md)
(walk #5, frontier = Tue-09-01 announcement batch; spec-dec anchor: AceSpec).

## Method + window mechanics (honest notes)

- **Export API healthy this walk** — `export.arxiv.org/api/query` answered all
  four queries back-to-back with no 429/503 (walk #5 was forced onto the search
  UI). All rows below are quoted from fetched Atom output; timestamps are
  API-verified per row.
- **Standing fix APPLIED:** the spec-dec query ran UNRESTRICTED (`all:"speculative
  decoding"`, no `cat:cs.LG`) per walk #5's fix — and the lane's cs.CL-primary rows
  are all visible, confirming the fix works. The fix's own recall hole did not
  resurface this walk (no cs.CL-primary paper was found hidden behind the old
  constraint).
- **Announcement mechanics:** walk #5's frontier was the Tue-09-01 20:00 ET batch
  (submissions 08-31T18:00Z → 09-01T18:00Z). Walk #6's strict differential = the
  **Wed-09-02 20:00 ET batch** (submissions 09-01T18:00Z → 09-02T18:00Z, announced
  Thu 09-03 ~07:00 +07 — live at walk time). The Thu-09-03 batch lands Fri 09-04
  ~07:00 +07; nothing from it is visible yet.
- **Walk-#5 coverage miss found (honest):** SMELT (2609.01343, 09-01T14:52Z) and
  mzCache (2609.01338, 09-01T14:49Z) sit INSIDE walk #5's strict window (same
  Tue batch as OUTLETS/SFAD, which it did capture) and were not recorded there.
  Both are C-grade/not-mineable, so the miss is cheap — but it is a miss: walk
  #5's query set (spec-dec, KV+eviction, quant+inference) does not reach
  training-scaling or mobile-memory papers. Recorded here as walk-#5 residue
  corrections, not walk-#6 window rows.
- **Lanes run:** spec-dec (unrestricted), KV+eviction, quant+inference, and a
  fourth kernel lane (`abs:"GPU kernel"` × DC/PF/AR) — the fourth surface is
  what surfaced the two strongest candidates this walk. Standing addition: keep
  the kernel lane in future walks (walk #4/#5 ran PF/DC listings only; the
  keyword query out-recalled the category listings).

## Differential table

| # | ID | Title (short) | Window | Lane | Grade | Mechanism (abstract-only) | Composes with |
|---|---|---|---|---|---|---|---|
| 1 | **2609.02514** | AceSpec: Asymmetric Edge-Cloud Collaborative Inference | ✅ strict window (09-02T12:18Z) — the ONLY strict-window row | spec-dec serving | **C — not-mineable** | Edge drafter + cloud verifier over WAN; probabilistic state cache turns rejection rollbacks into O(1) local lookups; asymmetric uplink (indices) / downlink (sparse distributions). | none (edge-cloud serving; no bounded fix-space in our single-box kernels) |
| 2 | 2609.01343 | SMELT: Scaling Laws for Looped MoE Transformers | ⚠ walk-#5 window MISS (09-01T14:52Z, Tue batch) | training/architecture | C — not ours | Middle-half layers looped twice at FLOP/param/KV-matched budgets; −6.8–18% training FLOPs on the compute-optimal frontier. | none (training scaling laws; riir-train routing at most) |
| 3 | 2609.01338 | mzCache: On-Device LLM Memory under Multitasking | ⚠ walk-#5 window MISS (09-01T14:49Z, Tue batch) | edge serving | C — not-mineable | Restoration-oriented memory mgmt under OS eviction pressure; partial eviction + concurrent CPU restoration on unified mobile memory; 2.1–5.5× TTFT vs storage offload. | none (Android/llama.cpp edge lane) |
| 4 | **2608.15764** | Concurrency Response of Plain Global Loads on H100 | new-to-program (08-16T14:33Z; surfaced by the NEW kernel lane) | **kernel_opt / measurement** | **A− — QUEUED for source verification** | Clean-room microbench on 3× H100 SXM5: attained `ld.global` bandwidth PEAKS at ~2 outstanding loads/thread and declines ~35% by K=8 (survives fixed-work control, sweep-order, allocation-size 512 MB→20 GB); DRAM bytes constant while L2-sector traffic rises; `cp.async` 2.1–2.9× over plain loads at high offered depth. Little's-Law accounting. | **cudarc CUDA kernel family directly** (`gemv_ternary_cuda_raw`, dp4a GEMV, the B20/B22 unrolled-load rules — "more outstanding loads per thread" is NOT free past the knee); **B98 `dequant-byte-pairs-as-aligned-u16-loads`** (load-width vs load-count tension gets a measured per-thread-depth axis); our `software-prefetch-storage-bound` annotation (prefetch ≠ the only async path); Bench-734 T9.15/T9.16 float4/`__ldcs` load-shape work |
| 5 | **2608.15531** | FlashQuant: Sparse-Dense Fusion for Outlier-Aware W4A16 | new-to-program (08-16T04:57Z; quant + kernel lanes both) | **kernel_opt** | **B− — QUEUED behind #4** | Fuses the dense GEMM path and the high-precision outlier SpMM path into ONE kernel (they share activations + outputs); sparse-dense tiling aligns outlier processing with GEMM tiles; Tile-COO encoding cuts smem bank conflicts; pipelined scheduling. 2.74–4.18× over cuBLAS BF16, up to 1.53× over the strongest UNFUSED outlier-aware baseline (the honest comparable). | **B58 `same-input-gemv-segments-single-launch`** (the heterogeneous-path member of the same-input family — FlashQuant is the sparse+dense twin); **B98 `moe-routed-down-sum-single-accumulate-pass`** (fusing heterogeneous compute into one output space); B15 fusion family; the B92 stripe/semaphore epilogue rules |
| 6 | **2608.12700** | A Contract-Grade Verifier for LLM-Generated GPU Kernels (+ native Blackwell GDN backward) | new-to-program (08-13T01:25Z, v2 08-17) | **measurement doctrine** | **A− — doctrine citation, no batch** | Twelve adversarial gates (several TOLERANCE-FREE) over 2,638 LLM-generated kernels a standard harness had accepted: 39.5% broken beyond any tolerance argument, 62.1% ≥1 violation; the field's standard test accepts 1,487 kernels the verifier rejects vs 14 the other way. Also: first native Blackwell tcgen05 GDN training backward, verified against a double-precision oracle. | **Bench 802/776 → the fault-class classification doctrine** (walk #5's 2609.00363 verdict: tolerance gates certify boundedness + preconditions + operand provenance ONLY) — this is the external 2,638-kernel measured confirmation of exactly that class; Bench-728's div.full invisible-class instance; the G4/alloc-budget doctrine citations; riir-train GDN lanes (Benches 734/726/831) |
| 7 | **2608.01651** | Bole: Tree Speculation for Hybrid-Attention Models | new-to-program (08-03T03:43Z) | GDN league lane | **B− — rider candidate on B82** | Transforms the linear-attention recurrence into a TREE-STRUCTURED closed form; losslessly encodes speculative state updates as token-level factors; reconstructs ONLY the sampled state; 82–99× transient-state reduction; 3.4–7.7× linear-attn tree verify; integrated into SGLang. | **B82 `wy-transform-tree-verify-state-free`** — external independent confirmation of the same mechanism family (TreeWY's pseudo-value matrix ↔ Bole's token-level factors) + the memory-side win measured where memory binds (our Issue-721 closed-negative was COMPUTE-side; Bole reconfirms the rule's own "win CONDITIONAL on memory pressure" clause); DASC/DAMP lane (same recurring-state territory) |
| 8 | **2608.17641** | rl-triton: associative scan for RL credit assignment | new-to-program (08-18/19) | B79 breadth | **B− — rider candidate on B79** | Recasts GAE, V-Trace, Retrace(λ), TD(λ), discounted returns, eligibility traces, episodic prefix sums as ONE first-order linear recurrence solved by associative scan in O(log T); algebraic verification of the operator; 1.6–5.7× over torch-compile in the many-env regime. | **B79 `first-order-affine-recurrence-associative-scan`** — breadth confirmation: the affine-recurrence-is-a-scan mechanism applies far beyond attention state (7 RL estimators are the same operator); riir-train's GRPO/PPO lanes are the potential consumers, though ours are not Triton-shaped |
| 9 | 2608.27513 | DAMP: Decay-Aware Mixed-Precision Recurrent-State Quant | new-to-program (08-27T09:11Z; KV lane) | GDN league lane | **B− — lane record** | First PTQ study of GDN/KDA recurrent states: INT8 uniform already degrades complex reasoning; error energy concentrates in few channels; decay-persistence is prompt-stable → offline calibration picks high-risk channels for higher precision, rest INT8; 69.1% state storage cut, 2.01× state-update kernel, −10.9% TPOT. Same group as DASC (walk #5 row 8). | **B55 `outlier-span-escapes-quantization-via-lossless-sidecar`** (same failure physics, recurrent-state side — DAMP is the state-tensor analog of the q8kv sink guard); Bonsai GDN state (Benches 734/726); H-Scale below (scale-selection axis) |
| 10 | 2608.28113 | H-Scale: Hessian-Guided NVFP4 Scale Refinement | new-to-program (08-28T09:22Z; quant lane) | quant (scale selection) | **C+/B− — lane record** | Post-processing refinement of NVFP4 per-group scales via a diagonal second-order (Hessian-proxy) criterion from calibration activations; drop-in replacement for RTN scale selection; offline-only, zero inference overhead. | the quant scale-AXIS triplet: **Issue 845 PoT** (which values are representable) + **OCGQuant 2609.00066 grouping** (which channels share a scale) + H-Scale (which scale VALUE to pick) — three independent axes over the same per-group-scale surface (q8kv, Q4_K, ternary groups, KVarN); B92 `quant-group-scale-fetch-gating` |
| 11 | 2608.22761 | DRY: stopping verbatim loops at sampling time | new-to-program (08-24T03:33Z; quant query, off-lane hit) | league-adjacent sampling | C+ — lane record | Suffix-extension penalty at sampling time; sequence breakers protect formatting tokens; −47% loop rate at preserved quality; ADOPTED by llama.cpp, ExLlamaV2, text-generation-webui. | league opponent surface (llama.cpp sampler feature — a parity note for the perf-rematch league, not a perf cell); no kernel/corpus fix-space |
| 12 | 2608.30963 | Universal Context-Reuse (cross-model KV sharing) | walk-#5 miss, pre-frontier (08-31T15:28Z) | KV systems | C — not-mineable | Cross-model KV translation (trained translator); "context mobility" abstraction; 67% target-prefill cut at 4K in-family. | none (heterogeneous serving; trained component) |
| 13 | 2608.30295 | CateKV: sequential-consistency head eviction | walk-#5 miss, pre-frontier (08-31T06:02Z) | KV eviction | C+ — lane record | CV-based classifier finds sequentially-consistent attention heads; retain critical tokens only for those heads, adaptive hybrid elsewhere; 2.72× memory / 2.18× decode. | riir-train eviction research lane (H2O family, katgpt-rs R523); no bounded fix-space |
| 14 | 2608.23296 | Sigmoid attention as eviction substrate | walk-#5 miss, pre-frontier (08-24T14:21Z) | doctrine echo | **C+ — doctrine citation** | Controlled 2×2×2 (attention type × learned gating × position enc): sigmoid-gated models delete KV entries with negligible PPL change vs their own references; softmax gates do NOT uniformly beat post-hoc H2O/KeyDiff. | **the house sigmoid-not-softmax rule** — external measured confirmation in a NEW regime (learned-eviction soft→hard transfer); cite-class only |
| 15 | 2608.27591 | Prox: training-free FFN activation sparsity | new-to-program (07-30; quant query) | FFN sparsity | B− — lane record | Two-stage training-free SwiGLU sparsity: the channel MASK (not values) is constructed from magnitude ranking of a quantized-proxy intermediate; exact compute on selected channels only; 1.99× e2e at 70% FFN sparsity. | Bench-763 finding (FFN = #1 GPU decode time on M3); the token-local-only-approximate-accelerator boundary (B84) applies: Stage-1's shared mask is input-derived and cross-projection WITHIN a token — a lossy compute surface needing our lossy-surface promotion gate; no current consumer |

**Residue (out-of-window, new-to-program or doctrine echoes, recorded without rows):**
LLMscope 2608.25321 (optical side-channel weight extraction — sec lane, C);
Hydra 2608.25053 (phase-aware edge characterization — instrument echo of our
decode/prefill stage decomposition, C+); FAMPWQ 2608.24945 (RL-based bit-width
allocation — not deterministic/modelless, C); NeuroPrefetcher 2608.22643
(82–85% neuron persistence across tokens — echoes the closed oMLX
expert-streaming pool, C); Anatomy of a Quantized Agent 2608.15117
(closed-form VRAM models + 2 constants suffice — measurement-doctrine echo,
C+); UnionSparse 2608.09291 (payload-to-metadata ratio as the sparse-edge
bottleneck metric, C+); AdaMX 2608.03867 (heterogeneity-aware microscaling,
ASIC, C); AnchorKV 2608.02901 (anchor-residual KV, C+); LightRot 2607.27704 +
GyRot 2607.27694 (FHT rotation units; GyRot's fully-integer dequantization —
external hardware echo of the rotation-TAX concern our Bench-716 Hadamard
refutation and Batch-96 fwht rule price, C+); MXSens 2607.17733 (MX mixed
mantissa, C); PolyQ 2607.14618 (CPU fractional-bit compiler co-design, C);
Julia Kernel Tuner 2608.21227 / HIERA 2608.21157 / KernelArc 2608.17071 /
SparseDitto 2608.05033 (LLM-based kernel generation — C, though KernelArc's
static config tables + shape-gated fusion echo B23); rl-triton's sibling
X-Stage 2607.23264 (post-issue comm-compute overlap, distributed, C);
TileSight 2607.22432 (tile-centric perf model, C); SPDP 2607.21985
(static-dynamic pruning Tiled-CBC, C+/B− sparse lane); Celty 2608.01536
(dual-sparsity spMspV + SIMT co-design, C).

## Shortlist (coordinator decisions)

1. **2608.15764 (A−)** — the strongest candidate of the walk and the first
   MEASUREMENT-shaped kernel_opt intake since Batch 49 (the B49 path: measured-GOAT
   distillation from an external measured source). Source questions, preregistered
   before any read: (a) does the K≈2 knee survive on non-Hopper parts (our 4090 is
   Ada — the paper is H100-only; the rule may need an architecture caveat or our own
   probe); (b) does the mechanism (L2-sector traffic rising while DRAM bytes stay
   constant) transfer to our GEMV shapes, or is it an Hopper-L2-sectorization
   artifact; (c) does the cp.async 2.1–2.9× high-depth result combine with or
   contradict our B20/B22 staged/unrolled-load rules. If it holds on (a) or scopes
   honestly: a kernel_opt rule "offered-load-knee-plain-loads" + possibly a
   measured probe on our cudarc GEMV. **QUEUED — next idle mining batch (plan-first
   discipline applies).**
2. **2608.15531 FlashQuant (B−)** — the corpus-shaped fusion member. Source
   questions: (a) is the fused kernel's win at our batch-1 decode regime or only
   batched prefill; (b) Tile-COO's bank-conflict fix vs our B92/B79 smem rules —
   same mechanism?; (c) is the outlier-path structure reachable from OUR weight
   formats (ternary/Q4_K have no high-precision outlier sidecar in production —
   B55's q8kv sink guard is the closest). If (c) fails, the rule distills as the
   heterogeneous-path member of the B58/B98 same-input family with the outlier
   instantiation as evidence only. **QUEUED behind #1.**
3. **2608.12700 (A−)** — NO batch (eval instrument, not a fix-space). Cite in the
   doctrine surfaces: the lossy-surface promotion rule, Bench-802's fault-class
   classification, G4/tolerance budget docs. One-line citation riders, batched with
   the next doc-sync touch — not worth a standalone commit.
4. **Bole → B82 rider + rl-triton → B79 rider** — rider-class (external
   confirmations). Ride the next B82/B79-adjacent batch touch per the standing
   rider discipline (Batch 96/98 precedent: riders land WITH a batch, never
   standalone).

**Window verdict (honest):** the strict Wed-09-02 window contains exactly ONE row
(AceSpec, C — not-mineable) → **NOT FIRED**, same shape as walk #5's strict window
(whose value also came from residue, not the window). The walk's yield is the
fourth (kernel) query surface — it out-recalled every category-listing surface
used by walks #4/#5 and surfaced both queued candidates. Standing addition for
walk #7: keep `abs:"GPU kernel"` × (DC/PF/AR) as a permanent fifth lane.

## Queue state after this walk

- Corpus: **UNCHANGED** (453 kernel_opt + 99 rust_perf + 45 clippy_lints + 12 sec
  + 31 docker; Batch 101 = the corpus head). Zero batches landed by this walk; two
  research-verification candidates queued above are RESEARCH tasks until their
  source checks pass, then plan-first mining batches.
- Lane state: GDN league lane gains DAMP + Bole (the recurring-state surface keeps
  consolidating — DASC walk-#5, DAMP, Bole all 08-2x/09-x, same physics family);
  quant scale-axis triplet recorded (845-PoT / OCGQuant-grouping / H-Scale-value);
  league lane unchanged (no Flash-Next-class + native-MTP opponent movement —
  re-arm condition not met).
- Walk-#5 miss corrections: SMELT + mzCache recorded as walk-#5 residue rows here.
- Next re-walk window: **Fri 09-04 ~07:00 +07** (Thu-09-03 batch). Standing lanes:
  spec-dec (unrestricted), KV+eviction, quant+inference, GPU-kernel (new),
  cs.DC/PF listings.
