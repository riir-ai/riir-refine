# Research 116 — Source verification: 2609.00363 (PoT determinism) + 2609.00066 (OCGQuant)

**Status:** RECORD — both walk-#5 queued papers source-verified full-text (abs + arxiv HTML,
2026-09-02). All three 2609.00363 questions CONFIRMED; the 2609.00066 runtime-cost question
REFUTED in the B+ direction (static permutation, zero GEMM cost). One lever FILED
(riir-ai Issue 845 — PoT-scale determinism), one lever QUEUED (OCG grouping — see
`.distill/001_mining_queue_snapshot.md`). Zero corpus delta.

Predecessor: [Research 115](115_arxiv_pool_refill_walk_5.md) (walk #5, which queued both
with pinned source questions). Both papers read in full — every claim below is quoted or
paraphrased from fetched text, nothing inferred from abstracts alone.

---

## 1. arXiv:2609.00363 — "Deterministic LLM Inference Across GPU Kernels: Power-of-Two INT8
Quantization Scales and the Limits of Tolerance-Based Conformance" (T.-R. Chen, Krixvon,
submitted 2026-08-25; companion to arXiv:2608.13756 "The Integer Alibi"; RTX 4090,
pre-registered protocol with append-only amendments)

### Q(a): Is the PoT requantization a deterministic per-tensor weights-only procedure we can mirror?

**YES, with one precision caveat recorded.**

The construction (§V-A):
- From the **parent bfloat16 weights** (not the int8 payload): each channel's scale computed
  by the checkpoint's own rule (÷127.5, stored bf16, the quantizing division itself in bf16
  — eight mantissa bits), then **constrained**: snap to `2^round(log2 s)` (nearest) or
  `2^ceil(log2 s)` (ceiling); the int8 weights are **recomputed** under the constrained
  scale. Powers of two are exactly representable in bf16, so no further rounding intervenes.
- **The gate discipline is the transferable instrument**: the requantization pipeline must
  reproduce the committed baseline checkpoint **byte for byte under the unconstrained rule**
  BEFORE any constrained arm is built. Their first attempt (division in f32 instead of
  bf16) left 6.42% of int8 weights different and was REJECTED by exactly this gate. Our
  Bench 692 LOGITS_HASH pinning is the same discipline applied informally — this paper
  formalizes it as a *precondition of comparison*, not a nicety.

The caveat: the formal required-equality condition (eq. 2: `rnd(x)·2^k = rnd(x·2^k)` —
multiplication by a power of two commutes with rounding) is stated for **all scales**
being PoT, while the requantization procedure describes **weight scales** explicitly. The
studied vLLM W8A8 activation path carries per-token dynamic scales (not PoT), and the
result (196/196 per-layer bitwise + 8/8 byte-identical greedy prompts at 1.7B/8B/14B) is
empirical, not forced. Any mirror of this lever on OUR stack must first pin OUR scale
inventory: which scales are ours to snap (weight-side group scales — yes; runtime-computed
block scales — snap-at-quantize is the analog move), and where the epilogue ordering
freedom actually lives. Do not assume "snap the weight scales" alone forces equality on an
arbitrary pipeline.

Cost data (for any lever gate): nearest-PoT perplexity point estimates +0.32% / −0.28% /
+0.48% at 1.7B/8B/14B (90% paired cluster-bootstrap intervals; upper ends +0.71% / +0.76%;
interval excludes zero only at 14B). Throughput at 1.7B: −9.6%..+1.4% prefill, −1.6%..+0.9%
decode. **Ceiling-PoT cost MORE than nearest (+0.54%)** — the pre-registered hypothesis
inverted: clipping a few outliers in exchange for finer resolution is mildly beneficial.
Nearest is the arm to mirror.

### Q(b): Does the ≤1-spacing blindness argument transfer from INT8-weights to f16-KV?

**YES as doctrine; NO as mechanism. Both halves recorded.**

Does NOT transfer: the *integer alibi* itself. The paper's accumulator is INT32-exact and
order-independent under a verified no-overflow bound (max product 128×127, so
`max|acc| ≤ 16256·K < 2^31`) — the accumulator is *above suspicion* and all cross-kernel
difference localizes to the epilogue. Our attention path has **no such accumulator**: dots
are fp32 accumulation, order-dependent by nature, and the drift is multi-ULP (Bench 802's
f16-ARITHMETIC envelope grew to 8.3e-1 in ops mode at 16K). That drift is
tolerance-VISIBLE — which is exactly why our flat-budget gates work at all. The alibi is
INT8-specific; nobody should import it.

DOES transfer: the **one-spacing blindness**. Across all 5,880 epilogue cells, five fault
families (scale precision, double rounding, multiplication order, output truncation, fused
ordering) produced maximum ULP distance of exactly 0 or 1 — never 2 — while one spacing is
the smallest tolerance admitting any legal floating-point difference. The fault class and
the legal class **occupy the same interval**: a tolerance of any size ≥ 1 spacing is blind
to the class *by construction*, not by bad threshold choice. Four of the five were detected
by NO check at ANY severity (including corrupt-every-element); the fifth (truncation) only
under PoT scales via the required-equality check. Operand-mismatch faults reached distances
68–35,571 in the same metric — the contrast is the result: tolerance gates reliably certify
**preconditions, operand provenance, and one-band boundedness; nothing else**.

**Live instance in our stack:** Bench 728's Vulkan `OpFDiv ≡ div.full` (vs div.rn)
cross-backend divergence is precisely this fault class — a ≤1-spacing rounding divergence.
It was caught by bit-exact comparison, never by any tolerance — consistent with the paper's
exit: the fix for cross-backend determinism claims is an *equality check under a
determinism-enabling construction*, not a tighter tolerance.

### Q(c): Does it upgrade Bench 802's "flat budget stands" into a fault-class classification?

**YES.** Bench 802's verdict ("flat 5e-3 budget STANDS; f16-STORAGE deviation dilutes with
context; f16-ARITHMETIC envelope grows") now reads as an instance of the paper's general
law: the flat budget certifies one-band boundedness of the storage deviation and is
structurally silent about any fault class that stays inside the band. The upgrade is
actionable in two directions:

1. **Gate design**: cross-backend determinism claims (Vulkan vs MSL vs CUDA arms of the
   same kernel) cannot be certified by any relative-error budget; they are certified by
   bit-identity gates under constructions that remove rounding freedom. Bench 804's gate 9
   (bit-identical across p ∈ {1,4,16}) and Issue 779's planted-poison control already did
   this empirically — the paper supplies the theory of *why tolerance gates cannot do this
   job* and the taxonomy of what they can.
2. **Experiment discipline**: the +157%-artifact story (the companion's probe rewrote
   stored scales without requantizing weights — 99.8% of the degradation attributed to that
   weight–scale mismatch, an operand-mismatch-family fault) is a standing warning for any
   future scale-snapping experiment on our quantized paths: **requantize from parents,
   never rewrite scales in place; byte-exact reconstruction gate before the constrained
   arm; report both matrices if any prediction changes post-hoc** (their two-stage-pinned
   77-cell prediction matrix with three disclosed corrections is the honest-A/B template).

**Verdict: CONFIRMED A− → actionable.** Doctrine upgrade (this note) + lever filed
(riir-ai Issue 845).

---

## 2. arXiv:2609.00066 — "OCGQuant: Outlier-Companion Grouping for NVFP4 Quantization"
(Yao et al., Intellifusion + SCUT, submitted 2026-08-30, EMNLP 2026 Main; code
github.com/Eshamont/OCGQuant)

### Q: Is the grouping a pure channel permutation (load-time, zero hot-path cost) or calibration-search-coupled (drops to C)?

**REFUTED in the "drops to C" direction — the runtime mechanism is a static permutation
with zero GEMM cost. The DECISION is offline and deterministic. Both halves verified.**

- **Decision (§3.2.2)**: layer-wise input-channel permutation from calibration activations.
  Rank channels by RMS over calibration tokens; greedy loop: largest-RMS unassigned channel
  = outlier candidate; its companions = the G−1 lowest-RMS unassigned channels (eq. 7);
  adaptive packing stop rule (eq. 8: pack while `Δ(o) = r_o² − Σ_{f∈F_ref} r_f² > 0`,
  where `F_ref` is the G−1 globally-lowest-RMS channels); remaining channels **preserve
  their original relative order**. No RNG anywhere; ~5 min per 8B model total (RMS profiling
  43.5 s + companion selection 0.55 s + reorder construction 1.97 s + GPTQ reconstruction
  250 s). Deterministic-given-calibration = modelless-compatible (same status as any
  one-time quant calibration).
- **Runtime (§3.3)**: channel reordering is **absorbed into the NVFP4 activation-quantize
  kernel** — fused RMSNorm + reorder + quantize, reading activation elements directly in
  reordered channel order, **no materialized reordered tensor**. The OCG permutation's
  structure gives a **contiguous-segment fast path**: non-packed channels keep original
  relative order, so most reordered groups correspond to contiguous source segments — the
  kernel loads one base index and derives `base+offset`, avoiding per-element lookup;
  non-contiguous groups fall back to gather. Weights are permuted **offline** (+ GPTQ
  reconstruction in the reordered space — "adding no inference-time overhead").
- **Measured overhead**: prefill throughput −1.5%..−2.9% vs fused-RTN (both arms fused for
  fairness); peak decode memory **identical to RTN** at every batch size. Correctness
  anchor (eq. 5): `XW^T = (XP)(WP)^T` — the permutation is exact only when applied
  consistently to BOTH operands.

**Verdict: stays B+.** Grouping is confirmed as the un-tested third outlier-mitigation
family after sidecar (B55) and rotation (Hadamard/QuaRot class).

### Composition constraints and the block-invariance insight

- **svcca detects Permuted 8/8** (Bench 836): any hot-swap equivalence gate will flag an
  OCG-permuted adapter as Permuted — *by design*; the gate's job is exactly to catch
  inconsistent application. An OCG adoption must ship the permutation as an explicit,
  committed transformation (both operands) so the gate's alarm is a known-cause trip, not
  a mystery.
- **The block-invariance boundary** (from riir-train Bench 556's negative control):
  *per-channel* quantization is permutation-INVARIANT (reordering channels cannot change
  independent per-channel error), while *block-shared-scale* quantization is
  permutation-SENSITIVE (channel order determines block composition, hence the scale, hence
  collateral error — the paper's "Collateral Quantization Error", eq. 6). **OCG therefore
  applies to our BLOCK-quantized surfaces** — q8kv 32-blocks, Q4_K 32-blocks, ternary group
  scales, KVarN — and NOT to any per-channel-scaled surface. This boundary kills whole
  classes of pointless A/Bs before they run.
- **Relation to B55 (q8kv sink guard)**: complementary, not competing. The sidecar is a
  *per-row lossless escape* for known massive-activation rows (exact, 6,850× error
  reduction on guarded rows, bit-identical at S=0). OCG is a *global block-composition
  improvement* that reduces collateral error on ALL scale-dominated blocks. A fusion
  (grouping + sidecar for residual sink rows) is the untested combination.

### OCG lever — QUEUED, not filed

Entry condition (recorded so the queue is falsifiable): a measured collateral-error case on
one of our block-quantized surfaces that the sidecar does not already fix at lower cost.
The sidecar is exact and cheaper where it applies (known sink rows); grouping only pays
when MANY blocks are scale-dominated, not a handful of rows. Build sketch if entered:
RMS-ranked greedy permutation (G=32 for q8kv) + permuted weight columns + fused gather in
the quantize kernel (contiguous-segment fast path first — it is most of the win) +
svcca-Permuted-aware equivalence gate. Track: `.distill/001_mining_queue_snapshot.md`.

---

## 3. What changed in the program

| Item | State |
|---|---|
| riir-ai Issue 845 — PoT-scale determinism lever (ternary group scales, q8kv block scales; kills the Bench 728 div.full divergence class by construction) | **FILED** |
| OCG grouping lever | **QUEUED** (entry condition above; snapshot updated) |
| Bench 802 doctrine framing | **UPGRADED by this note** (boundedness-only certification; bit-identity is the determinism instrument) — no doc edit made to the sibling benchmark file; this note is the canonical reference |
| Corpus (kernel_opt 426 + rust_perf 97 + clippy 45 + sec 12 + docker 31) | **UNCHANGED** |

Dedup note: grep for `power-of-two|pow2|dyadic` across program notes/benches returned only
unrelated dyadic families (Hadamard dyadic anchors, dyadic bootstrap, dyadic reveal grids);
grep for outlier-companion/channel-reordering returned riir-train Research 430 / Bench 556
(TV-permutation smoothing — training-side reordering for cross-channel structure fits, a
different purpose) and Research 504's orthogonal-block quantization note. Neither covers
PoT-scale determinism or block-composition grouping. Both mechanisms are new-to-program.

Prior art carried honestly: PoT/dyadic quantization scales for *accuracy* is old
(HAWQ-V3 2021, RAPQ 2022 — cited in-paper); the paper's contribution is the *determinism
objective* + the blindness classification. Channel reordering for quantization is old
(RPTQ 2023, Atom 2024 — cited in-paper); OCG's delta is FP4-grid-aware companion selection
+ the block-composition framing. Our notes cite both lineages; nothing here claims
mechanism-level novelty over them.
