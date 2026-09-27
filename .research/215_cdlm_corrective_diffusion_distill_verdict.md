# Research 215 — CDLM (Corrective Diffusion Language Models) distill verdict

**Status:** RECORD — verdict landed 2026-09-27 after a 3-round Claude verdict negotiation
(REVISE round 1 → AGREE round 2; the reviewer's reheat/generation corrections were
accepted and are folded through this note, the issue, and the riir-train plan).
**Instrument LANDED 2026-09-28**: generation v9 + the arming gate + the T5 POC
(outcome (c) infrastructure-ahead-of-consumer — the ranking separates CRB corruption,
relative gap +0.191, but no semantic-bug consumer exists) — measured record:
`.benchmarks/103_generation_v9_crb_corruption_localization.md` (commits `c2faf248`,
`5912e144`).
**Date:** 2026-09-27 (M3, AC power)
**Source:** [arXiv:2512.15596](https://arxiv.org/abs/2512.15596) "Corrective Diffusion
Language Models" — Zhang, Peng, Zhang, Pan, Chrysos (UW–Madison + Duke), v2 2026-01-29.
Code `zhangshuibai/CDLM` @ `12fe555aa815959fe1323a85f3d8a9656c480149` (MIT; shallow
clone `katgpt-rs/.raw/CDLM`, **removed at landing** — provenance pinned here per the
external-repo law).
**Related filings:** `riir-clippy/.issues/137_crb_compile_clean_test_fail_corruption_eval.md`
(modelless, PRIMARY) · `../riir-train/.plans/424_cdlm_correction_aware_dlm_posttrain.md`
(training, SECONDARY).

## Verdict

**GAIN on both tracks, never Super-GOAT.** The corrective-DLM training class is
published and crowded (see prior art below) — the objective is cited, never claimed.
Our value is the stack fusion, in two independent tiers:

| Track | Tier | Filing | Standing |
|---|---|---|---|
| (a) modelless instrument | GAIN (GOAT-tier fusion) | riir-clippy Issue 137 | PRIMARY — in-stack, cheap, measurable now |
| (c) trained weights | GAIN (GOAT-tier fusion) | riir-train Plan 424 | SECONDARY — training outside the hot path; consumer gated |

This mirrors the established Plan 414 (training half) / Plan 602 (measurement-primary)
split for the DLM lane.

## The paper in one paragraph

Standard MDLM training (`L_absorb`) supervises only masked positions. Visible-but-
incorrect tokens receive zero loss, so token-level confidence `c_i = max_v p(v|x)`
is uninformative about token correctness — the paper's Figure 1 shows a model equally
confident in a correct identifier and an undefined-bug identifier. Confidence-guided
remasking then overwrites correct tokens while leaving real errors. The fix is a
post-training mixture objective:
`L = (1/|M|) Σ_{i∈M} ℓ_i + λ_noise Σ_{i∈V} z_i ℓ_i`, `z_i ~ Bernoulli(α)` —
absorbing mask corruption (standard reconstruction) PLUS explicit cross-entropy on
uniformly-replaced VISIBLE tokens. Measured: clean-vs-error confidence gap grows
~1000× (their Sudoku control: 10,000× clean/noise confidence ratio vs 8.2× for
absorbing-only); Top-K error localization up; CRB Pass@1 0.225 vs 0.148 MDLM; and
even from-scratch codegen (HumanEval 0.22 vs 0.19) and high-entropy parallel decoding
(ParallelBench high-entropy remask 0.62 vs 0.47) improve. The α tradeoff is real:
gaps grow with α but Top-5 hit rate declines past α ≈ 0.7 (false positives on clean
tokens). Recipe constants: CDLM-0.5B = Open-dCoder-0.5B + 2000 steps on Nemotron;
16B = LLaDA-2.0-MINI + 6 epochs on 50k ParallelBench examples, block diffusion block
size 32. The **Code Revision Benchmark (CRB)**: type-preserving tokenizer-consistent
corruption (operator / identifier-from-same-scope / literal-same-type, token count
preserved), execution-validated (only programs that FAIL the grader are kept),
difficulty = |E| replaced tokens; metrics = confidence gap, Top-K hit rate, Pass@1
after T remask steps at threshold τ.

## Published prior art (§4 searches, two rounds)

The class "train DLMs to be error-aware/corrective" is contested by ~6 concurrent
works — this is why Q1 fails and the verdict is GAIN, not Super-GOAT:

- **GIDD** (von Rütte et al. 2025) — absorbing + uniform hybrid noise; motivation is
  generation quality, holistic updates (not remask-restricted refinement).
- **PRISM** (Kim et al. 2025, arXiv:2510.01384) — provable self-correction, internal
  remask probability head, joint optimization.
- **RemeDi** (Huang et al. 2025, arXiv:2509.23653) — self-reflective remasking, joint
  optimization, uniform artifact distribution.
- **BackPlay / DSC** (Amazon, arXiv:2601.06428) — **decoupled** correction head:
  freeze the base DLM, train a lightweight (~100M, 2–3 layer) head with BCE on
  model-generated artifacts + Future-Context Augmentation; Pareto-better
  accuracy-vs-iterations on MBPP/HumanEval/GSM8K. Closest published alternative to
  CDLM's objective; informs Plan 424's phase structure (a decoupled head is the
  cheaper pilot alternative) but touches no ternary backbone and no healer consumer.
- **"Denoising is not the End"** (Zhang, von Rütte, Ding, Hofmann — OpenReview
  uelrt017SR, Latent & Implicit Thinking workshop) — systematic evaluation of
  self-correction strategies for hybrid-noise DLMs; finds a quality/preservation
  tradeoff and ~equal compute split between denoising and self-correction.
- **ReMDM** (Wang et al. 2025) — training-free remasking with inference-time scaling.
- Corruption-instrument prior art (for the benchmark half): **Pydra** (synthetic
  clones + bugs for representation probing), **BugsInPy** (real Python bugs),
  classical mutation testing, and — closest for RUST — **cargo-mutants**
  (type-preserving compile-clean mutations; exactly the operator class our
  compile-clean bucket uses). Our delta is never the corruption concept: it is the
  healer-localization-eval fusion + the oracle-anchored real-error arm.

## Fusion — what our stack does with it

### Track (a) — modelless, PRIMARY (→ riir-clippy Issue 137)

The CRB instrument decomposes into three modelless pieces, all extractable without a
gradient:

1. **Compile-clean, test-failing Rust corruption fixtures.** The paper's
   execution-validated corruption, ported to Rust with the two-tier filter: corrupted
   program must still `cargo check` clean (a syntax break is not an error position —
   noise), then `cargo test` must FAIL (the program must actually be broken). Only
   the compile-clean/test-failing bucket is the headline: there NO oracle marks the
   error, so localization is a genuine open question. In Rust, type-preserving
   compile-clean swaps are mostly literal / operator / identifier-in-scope swaps —
   cargo-mutants' territory (cited, not claimed). Compile-FAILING corruption is the
   **control arm**: rustc/clippy already mark those spans, so hit@K there measures
   rustc's span accuracy, which is near-trivial (verdict-round-1 correction — the
   original draft wrongly headline'd that bucket).
2. **Known-position localization metrics.** Because corruption positions are known by
   construction, labels are free: confidence gap (mean scorer confidence on clean
   positions − mean on error positions), Top-K hit rate, false-positive rate, plus
   MRR as the low-variance continuous variant. Loud-zero laws: a constant-confidence
   scorer must produce gap = 0 and the gate must REFUSE (never read as "perfectly
   calibrated"); a shuffled-confidence control must fall to chance hit@K (proves the
   metric discriminates); per-|E|-level count floors so a silent generation
   regression empties the population loudly.
3. **The k-ladder already exists — this is a new generation, not a harness.**
   Verdict-round-1 correction: `score_bench/generations.rs` ships `generation_v2()`
   (deterministic k-ladder, issue 072 T5) and the `gen4-k*` rustc error-code ladder
   (Plan 093). "Difficulty = |E|" IS what that k already is. Issue 137 is framed as a
   new `--generation` row under the house append-refusal law (not vacuous, harder
   than the prior generation) — never a parallel system.
4. **Consumers, honestly split.** PRIMARY: the DLM remask confidence gate as an
   arming check — CDLM's causal finding is "uninformative confidence makes
   confidence-gated loops actively harmful", and the surfaces that actually gate on
   confidence are the d2f remask (`katgpt-forward` d2f / diffusion_sampler) and the
   Plan 414 block-diffusion sampler's remask step. (The original draft aimed this at
   `reheat_fix_pass`; withdrawn — that loop is `MAX_PASSES = 4` fixed with no
   confidence gate at `riir-clippy/src/bin/heal.rs:11838`.) SECONDARY: healer span
   ranking — a pre-registered negative-result POC covering BOTH "no separation" (the
   existing retrieval scores may not separate corrupted from clean spans) and "no
   consumer" (the healer has no semantic-bug fixer that could consume a span ranking
   today). If neither separation nor a consumer materializes, the recorded finding is
   infrastructure-ahead-of-consumer, and the instrument stands ready for the DLM
   remask gate.
5. **Panel extras adopted as instrument rows in Issue 137** (No-GD advocate): the
   loop-harm A/B detector (paired non-inferiority: does the loop fix planted errors
   without breaking previously-correct positions — the paper's measured failure mode,
   decomposed); the oracle-anchored-vs-heuristic gap audit (the paper's deepest
   generalizable claim — supervision confined to one position class breeds an
   uninformative signal on the other class — maps onto our two span classes, reported
   per class, never pooled); Bernoulli re-draws as variance reduction (k seeded
   corruption draws per program → CI-width disclosure; born compliant with the
   unseeded-global-RNG gate).

### Track (c) — training, SECONDARY (→ riir-train Plan 424)

The mixture objective is a training-loop innovation (loss placement), so no modelless
analog is required — but it is implementable at near-zero infra cost: riir-train's
`mask_indicators` per-position loss-mask machinery (Plan 074's interventional-SFT
infrastructure, `riir-train-engine/tests/bench_dllm_training.rs`) extends from one
boolean gate to two weighted sets (absorbed-M at 1/|M|, z-selected visible at
λ_noise). The pilot is a **three-arm** design (verdict-round-2 addition):
**arm C = lr=0** (a confidence gap that exists before any training cannot be scored
as a gain — and it subsumes the sampler no-op detector: if the mixture arm's gap ≈
the lr=0 arm's gap, nothing trained), **arm B = absorbing-only control** (the paper's
own isolation protocol), **arm A = mixture** (α sweep 0.2/0.35/0.5; the paper's α≈0.7
false-positive cliff may arrive EARLIER on GDN — recurrent state smearing makes clean
positions downstream of a corruption read low-confidence, a confound the pure-attention
backbone never needed to split for). Ternary/GDN-specific risks pinned in the plan:
embedding/LM-head treatment in the quant grid (the likely failure point — if
`embed_tokens` is frozen-ternary, the mask-token embedding cannot adapt), LoRA rank
capacity for a global recalibration, and the weirdness-detector confound (uniform
replacement is heavily OOD — the oracle-anchored real-error arm from the healer's
`--clippy-json` feed is the decisive axis, not synthetic CRB). **The closed loop:**
the measured confidence gap feeds the DLM remask-gate arming check from Track (a) —
the training lane's concrete consumer, which answers the "thin consumer" weakness.
The paper's honest ceiling datum stands: even CDLM self-revision on its own generated
programs is modest (Appendix D, 0.20→0.34) — sell calibration and targeting, never
miracle fixes.

### L4 boundary (non-negotiable, carried into Plan 424)

The L4 lesson (191 s/fix, 0/60 exact-match) bars any generative fixer from the heal
hot path. CDLM's remask-refine loop IS generative multi-pass decode — same cost
class, same verdict. What survives by cost class: the trained error-aware confidence
as a RANKER (one forward pass, ms-class) feeding existing modelless gates; and
off-hot-path generative lanes only if the probe proves out (`--fix-compile`'s
multi-attempt loop; instinct's serving lane at 118–187 ms p50, where a remask-refine
pass fits budget). `riir-clippy` consumes at most as an opt-in client with a
kill-switch; it never hosts the weights.

### Modelless analog already shipped (validation, not novelty)

The healer's P39 `--repo-posture` mode (edit only where the target's clippy flagged —
`src/edit_emitter.rs`, verified shipping) is the modelless analog of CDLM's
"explicitly supervise visible incorrect tokens": edits land only at oracle-marked
spans, preserving correct code. The paper validates this design at the training
level (models given explicit incorrect-token supervision gain discriminative
confidence); recorded as validation, never claimed as ours.

### Game-context reframe (honest, weakest)

No direct NPC-cognition mechanism transfers: NPC perception has no softmax-over-
tokens whose calibration the paper addresses. The metric-only port: inject synthetic
corruptions at known ticks into NPC observation streams in the RAW physical domain
(tick-exact, replay-deterministic — never a latent stand-in for position), grade
existing modelless anomaly signals with Top-K hit rate. A perception-gate test
instrument, not a cognition primitive. Recorded; no files filed.

## What this is NOT

- NOT a claim on the mixture objective — published (CDLM, and the GIDD hybrid-noise
  family before it).
- NOT a claim on corruption-based benchmarking — published (Pydra, BugsInPy, mutation
  testing, cargo-mutants).
- NOT a claim on k-laddered difficulty — the healer's own score-bench generations
  already implement it.
- NOT Super-GOAT — Q1 (no prior art) fails on both tracks; the fusion axes
  (ternary/GDN backbone + healer consumer + remask-arming consumer loop) are the
  delta.

## Validation protocol summary

- Track (a): new generation row; shuffled-confidence control → chance hit@K;
  constant-scorer gap = 0 → gate REFUSES; per-|E| floors; compile-clean filter
  proven by a planted no-op corruption being rejected; negative result pre-registered
  two ways (no separation / no consumer).
- Track (c): three arms (lr=0 / absorbing-only / mixture), α sweep, oracle-anchored
  organic arm decisive, gap-vs-both-controls as the GOAT gate, clean-passage
  regression ≤1% as G3, and the 4090 box-state preflight on every quoted latency.
