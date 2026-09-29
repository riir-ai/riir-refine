# Research 219 — arxiv pool refill walk #12 close-out (2026-09-29)

**Status:** RECORD — walk executed 2026-09-29 00:09–02:4x UTC (07:0x–09:4x +07,
M3, the due clock; the window opened Tue 09-29 00:00Z as Research 217
predicted). **Zero corpus delta: 0 rule-batches — the third consecutive
zero-delta walk.** 5 substantive lane records + 1 doctrine datum + 1
measurement-echo + counted skip classes. Differential base 2609.23388
(2026-09-20T06:11Z) → window 09-20 06:11Z → 09-29 02:00Z; 79-row union
(21 rows submitted 09-26/27 — the weekend wave, visible only after the Tue
00:00Z announce, exactly as the walk-#11 arithmetic predicted); 47 fresh past
the walk-#11 base after dedup against Research 192 + 201 + 217 + the six
early-catches.

## Index-state instrument notes (the walk's own traps, recorded)

- **Throttle:** 3 of 5 lanes 429'd on the first pass (00:09 UTC); retries at
  15-min spacing landed all 5. The walk-#11 "first-try clean" path did not
  reproduce — the API rate-limiter is aggressive in the 00:00–02:30 UTC
  post-announce window (shared-IP throttling suspected). Future walks: 8s+
  inter-query spacing, 30s retry backoff, and expect 429s in the first hour.
- **Index roll:** the first reads returned a SHRUNK result set (plain-KV 12 vs
  walk-#11's 31 same-window) — verified NOT a query defect (three query forms
  agree; `totalResults` matches entries; no-date sanity 718, month 42). The
  index fills over ~2h after the announce: plain-KV 12→19, cs-PF 21→33,
  spec-dec 2→5 across re-runs, and the 09-26/27 pub-dated rows appeared only
  in the 02:0x reads. **The walk is graded on the 02:0x state; earlier
  partial-pass numbers were instrument readings, not graded rows.**
- **Lane-form note:** the quant+inference lane (`abs:"quantization" AND
  abs:"inference"`) returns 3 — walk #11's 27-row lane profile implies the
  broader `abs:quantization` form (stable 21 this window); walk #12 grades the
  broadened form and records the delta so no session re-derives the query.

## Graded rows

| # | ID | Title (short) | Lane | Grade | Reading |
|---|---|---|---|---|---|
| 1 | **2609.33135** | MpFA: Train-Free QK4V8 FlashAttention on Blackwell | gpu-kernel | **B+ lane record (kernel-lane intel HIGH)** | NVFP4-QK + FP8-PV mixed FA: "fully FP4 attention often fails to translate throughput into end-to-end speedups due to non-matmul costs" (online quant after softmax, tensor/smem movement, softmax-path contention); **rank-one smoothing compensation implemented as an ADDITIONAL Tensor Core MMA** recovers accuracy without stressing softmax; async pipeline + tensor-memory reuse + adaptive prefill/decode partitioning; 2.81× e2e over BF16 FA4 (B200, 16K–128K). Blackwell-only, FP4-class — no 4090/M3 consumer, no corpus rule; the quantized-attention format-regime datum for the watch. |
| 2 | **2609.33477** | SuffixReplay: Prefix Caching via Suffix Replay (hybrid LLMs) | cs-PF/plain-KV | **B+ lane record (GDN serving intel)** | Hybrid (full-attn + linear-attn) prefix reuse WITHOUT recurrent-state checkpoints: replay a bounded recent SUFFIX of input hidden states (retained anchors) to reconstruct the linear-attention recurrent state at any page boundary — "just let linear states forget the distant past" (decay/gating attenuates old inputs). The system-level answer to the exact GDN prefix-cache shape our serving lanes carry (the recurrent state cannot roll back to arbitrary boundaries). APPROXIMATE, not bit-exact — the 846 DRAM-floor / verify-posture stands; this is the mechanism reference if prefix-cache-on-GDN opens. |
| 3 | **2609.29397** | Baseline Shape Decides the Verdict (ternary @60K) | quant-broad | **B lane record (ternary doctrine)** | Controlled re-exam of the routed-ternary-beats-transformer claim: at 60K params the BASELINE spans 22.6% val loss by depth/width alone (≥ the published 22% margin — a baseline-shape effect); at 130M bytes a plain gated diagonal-SSM beats the routed ternary by 9.1% and the router itself prefers the recurrent path. Benchmark-vs-baseline shape control is the lesson (echoes the walking-seeds class); our league ternary lanes are fixed-model-anchored — record only. |
| 4 | **2609.33923** | Quantization Error Is Spectrally Flat (random probe) | quant-broad | **B (measurement-instrument datum)** | One random Gaussian probe = unbiased per-tensor quant-error norm estimator (RTN error spectrally flat); data-free mixed-precision knapsack under exact byte budget; **propagated probe rank-correlates 0.81–0.83 with the GPTQ objective, isolated uncorrelated (−0.01)**; "at matched bytes on Qwen3.8-27B a block-output probe beats a vendor IQ3_M mix" — the league model family, quant axis. riir-train PTQ-lane citation candidate; no code-shape. |
| 5 | **2609.33889** | Where Activation and KV-Cache Sparsity Cross | plain-KV/cs-PF | **B− (measurement echo)** | Byte-crossover model predicts activation-vs-KV sparsity speedup crossings within 4.1K tokens; **"timing the dense baseline with masked instead of split-K attention inflates the apparent speedup of the same KV policy about fivefold"** — the baseline-kernel-choice IS the measurement (the aa-control/B109 class, external datum). Analysis, no code-shape; echo recorded. |
| 6 | **2609.32902** | Linger and Lose (knowledge collapse, low-bit) | quant-broad | **B− doctrine datum (training-track)** | From-scratch ternary retains as little as 6% factual capacity vs fp16 at near-par loss; the deficit is an LR-dwell instability localized in the output head; "loss and downstream accuracy conceal a much larger failure" — the per-family-conditional framing of the lossy-surface law, one lane over. From-scratch ≠ our PTQ'd league models; record only. |
| 7 | **2609.31980** | Coding Agents as Perf Engineers (FRNN) | cs-PF | **C+ ecosystem record** | Off-the-shelf coding agents autonomously optimize CUDA FRNN (1.6×, exact-correctness contract, profiling-gated acceptance); an independent rerun followed different hypotheses to better perf. Ecosystem validation of this workspace's pre-registration + twin-discard discipline; no corpus content. |
| 8 | **2609.33465** | GSM: Shared Global State LM | plain-KV | **C** | Encoder-side shared fixed-window state; decoder reads it without per-step KV growth. Architecture class, no league consumer. |
| 9 | **2609.33652** | Tsubame (tree replay, diffusion spec-dec) | spec-dec | **C** | Freeze tree topology from draft scores, repopulate nodes by sampling in pass 2; lossless under compatible sampling. Spec-dec lane closed-negative (717 G3a) stands; ecosystem record. |
| 10 | **2609.33887** | Redline (block-diffusion risk-budget serving) | quant-broad/spec | **C** | Distribution-free reference-relative-risk budgets select serving operating points; explicitly applies to speculative acceptance rules. External formalization of the T2-certification instinct; serving-recipe class. |
| 11 | **2609.33245 / 2609.26796 / 2609.28270 / 2609.31009 / 2609.32237** | ASR spec-dec · Flash-dLLM · PTQ price · G²PTQ · Apple-M6 FFT | various | **C/skip** | Off-model or training-track records, counted individually in the note's skip ledger. |

## Skip classes (counted, ~32 rows)

- Quant-domain off-lane: 23585 (BOS-sink 4-bit), 23587 (efficiency-safety),
  24538 (protein QLoRA), 24799 (medical evidence PTQ), 26823 (Qwen2-Audio),
  27408 (VLM readout), 28739 (Whisper temporal taxation), 28757 (low-vision),
  29371 (Bangla speech), 30009 (regulatory RAG), 30820 (looped transformers),
  30846 (conformer NPU), 31341 (IE energy), 32042 (agentic PTQ replication).
- plain-KV 746-family: 27233 (distilling sequential computation), 28250
  (activation vs parametric memory), 32353 (visual token reduction), 32610
  (KV-Lingo translators), 32663 (Distance-KV), 33334 (draft-guided eviction),
  33759 (positions are not facts).
- cs-PF off-lane: 28855 (healthcare orchestration), 29067 (TileBench),
  29765 (sparse Cholesky), 31551 (EAServe multimodal disagg), 31938 (Conv3D
  cache-aware lowering), 32197 (SparSP video DiT), 32396 (FA-Bench ASR),
  32814 (associative algebra layers), 32848 (HFT design principles), 32978
  (petabit associative arrays), 33874 (JET justification eval).

## Yield

0 rule-batches · 2 B+ lane records (MpFA, SuffixReplay) · 1 B ternary-doctrine
record · 1 B measurement-instrument datum (spectral-flat probe) · 1 B−
measurement echo + 1 B− doctrine datum · 1 C+ ecosystem record · the rest C /
skip-class counted. **Walk #12 DRY for the corpus — the staged queue is
unchanged (still EMPTY post-B187); mining cadence-HOLD continues until the
next staged lead lands.**
