# Proposal 009 — Trained clippy drafter: the moka-PUCT transfer (bounded draft-and-verify for the healer)

Status: **draft** (proposal only — execution waits on the owner go + [Plan 049](../.plans/049_trained_drafter_phased_rollout.md))
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: moka PUCT (katgpt-rs Issues 204–207 / Bench 205) × [Research 001](../.research/001_self_evolving_latent_first_agent.md) "Model evolution" (the deferred category) × [Plan 017](../.plans/017_dflash2_selection_headroom_poc.md) headroom instrument × [Plan 014](../.plans/014_clippy_bench.md) score-bench × the `l4_lora` seam
Related: [Research 077](../.research/077_dflash2_draft_only_distillation.md) (DFlash2 draft-only distill), Bench 031 (fix_verify), Bench 043 (selection headroom negative), Bench 049 (fixer re-gate POC), riir-train Plan 336 (the 0/60 LoRA predecessor), katgpt-rs Plan 004 (Leviathan speculative-decoding distill)

## TL;DR

Train a **small domain-specific drafter** for the clippy healer — not a generative fixer. The
model learns `P(rule | span)` + fix-template slot bindings over the ~450-rule bounded corpus,
speculatively; the existing verifier stack (syn parse → `clippy_verify` → `fix_verify` compile
gate) makes correctness free, so draft quality is purely an **acceptance-rate** problem — the
same property that makes moka's PUCT player work (policy prior + value head over a bounded move
set, self-play trained, int8). This is a **riir-clippy invention** in mechanism (bounded
draft-and-verify over lint corpus) whose components are established prior art (speculative
decoding: Leviathan et al., distilled in-house as katgpt-rs Plan 004; policy+value search:
AlphaZero lineage, already ported as moka; SWE-RL-style model evolution from coding feedback:
surveyed in Research 001, where this proposal activates the deferred "Model" category). The cost:
one training lane in riir-train + an opt-in `trained_prior` consumer feature. The prize: the
L4 seam finally has an arm that is *shaped* to beat `decline`, and the Plan 017 type-blind
selection bound gets a type-aware challenger.

## The problem this solves

The healer's L1 drafter is modelless and correct-but-shallow: retrieval top-1 is
fixture-saturated (~96% on the composite set with the Structural re-ranker) yet **real-repo
reachable misses are closed at 0% by decline** — Bench 049 measured the incumbent general-LLM
arm (`decline` 0/9, CF llama-70B 5–6/9 strict-keep) and riir-train Plan 336 measured the
generative-LoRA arm at **0/60 degenerate** (Bench 467). Both failures share one shape: they
asked an unconstrained generator to emit fixes. Meanwhile [Plan 017](../.plans/017_dflash2_selection_headroom_poc.md)
proved the *selection* axis is precision-bounded, not ordering-bounded — `decompose_headroom()`
found **0 of 101 spans** selector-addressable because parse-only latent features carry no type
information ("a pairwise selector over the same parse-only latent features carries no type
information with which to break that bound" — Plan 017 §Outcome). The gap: **no trained,
span-conditioned, type-aware prior exists anywhere in the healer**, and the L4 seam
(`src/l4_fallback.rs`, armed, miss-only reachability contract) sits UNARMED with no candidate
shaped to beat decline. Without this, the healer's misses stay permanently declined and the
armed L4 seam stays dead weight.

## The proposed design

**Reframing (load-bearing):** the 0/60 failure was a *generative fixer* (free-text emission).
The trained component here is a *drafter* in the speculative-decoding sense — bounded output
space, verifier-gated correctness. Draft quality sets acceptance rate, never correctness.

**Model (Phase 1 artifact): a two-tower bi-encoder.** Span tower (span tokens / AST-DFS
sequence through the existing GDN/DeltaNet ternary kernels — reuse over novelty) + rule tower
(rule doc tokens); score = `dot(span_emb, rule_emb) → sigmoid` (house bridge pattern, sigmoid
never softmax). Contrastive training on (span, rule-id) pairs. This is literally "latent
retrieval with learned embeddings instead of the modelless hash embedder" — the minimal
trainable delta on the shipped `LatentMatcher` seam. Slot-filling heads (which identifier/
range fills which template hole) are Phase 2; the corpus templates ARE the output space.

```text
span tokens ──► GDN tower ──► s (d-dim) ─┐
                                          ├─ dot → σ → P(rule|span)  (replaces/augments
rule docs  ──► GDN tower ──► r (d-dim) ─┘   fan-out + Rerank scoring)
P + V(+fix template) ──► [PUCT over top-k × proposer variants] ──► candidate fix
        │                                                    │
        └── trained on ──► verify oracle outcome ◄── syn/clippy/compile gates (EXISTING)
```

**Search (Phase 3, the moka transfer proper):** PUCT over `(rule, proposer-variant)` actions
with the trained prior P and value head V(passes-verify), only on contested/composite spans —
the Board→span, move→(rule,fix) mapping of `katgpt-moka-wasm/src/puct.rs`. Self-play loop =
healer runs over the workspace repos; real `cargo clippy` (`src/draft/clippy_oracle.rs`) is
both reward and eval — the uncontaminated-oracle discipline from T3.5.5 is load-bearing.

**Weight format + boundary:** train in riir-train (tiny model — M3 Metal first; no 4090
needed), ship as a **freeze/thaw artifact** — BLAKE3-committed NeuronShard-style snapshot
(riir-neuron-db freeze envelope precedent) loaded at runtime as DATA. `riir-clippy`'s dep tree
does not move: weights are bytes, not a cargo dep (default build stays katgpt-core-only).

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **Plan 017's negative is the primary risk.** The selection band on parse-only features was
   EMPTY (0/101 selector-addressable). The trained bi-encoder's entire value proposition is
   adding type/semantic signal the parse-only features lack — if it cannot move
   `decompose_headroom()`'s addressable band on REAL-REPO spans, it is a duplicate of Plan 017's
   refuted selector and must be killed at Phase 0 (the instrument is committed and re-runs on
   every corpus batch).
2. **The reachable set is small; the win margin may be 2–3 cases.** Bench 049's bar: decline
   0/9, CF-70B 5–6/9. A domain-trained drafter should beat a general 70B on domain data
   (the classic small-specialist-beats-large-general regime), but a 2-case win on a 9-case set
   is within noise of harness drift — the reachable set must be re-measured fresh at gate time
   (corpus has grown since 049), and G2 requires the refreshed denominator.
3. **Starved-pool selection washes have precedent** (Issue 026 entropic, Issue 039 Elo) — both
   selection upgrades on thin evidence measured flat. The trained prior is a different signal
   class (supervised function vs accumulated counters), but it must pass the same adversarial
   harnesses or be rejected by the same rule.
4. **Fixture saturation means the fixtures cannot be the primary eval.** Composite sits at 96%
   with the Structural re-ranker AT pool-recall ceiling — a trained prior that merely recovers
   pool recall on fixtures is a TIE, not a win. The real headroom is real-repo spans (Layer:
   oracle-label fresh spans across the workspace repos before training; dataset frozen
   BLAKE3-committed).
5. **Modelless-first mandate tension, resolved by the three-track rule.** This ships trained
   weights into a modelless-first repo. The stack's three-track system (modelless inference +
   self-adaptive runtime + model-based trained weights) legitimizes it, and the L4 precedent
   sets the posture: opt-in feature, `decline` stays the default answer, promotion only on
   measured win over decline, never-wrong preserved. The burden of proof is on the model.
6. **Prior-art honesty.** Draft-and-verify for code repair is an established literature shape
   (SWE-RL / CURE / ZeroCoder class, surveyed in Research 001 §taxonomy "Model" category;
   Leviathan speculative decoding distilled in katgpt-rs Plan 004; DFlash2 in Research 077).
   The invention claimed here is narrow: bounded rule-id + template-slot drafting over a lint
   corpus with verifier-gated acceptance, fused with PUCT-style search. code2vec (Alon et al.,
   AST paths) and TBCNN (Mou et al., tree-conv) are cited from knowledge — the MCP search
   quota was exhausted at filing time (resets 2026-09-15); the in-house distills above carry
   the load. No paper is re-attributed as ours.

## Fusion lineage

1. **moka PUCT** (`katgpt-moka-wasm/src/puct.rs`, Issues 204–207, Bench 205): policy prior +
   value head over a bounded action set, self-play trained, int8-forward. Contributes the
   search layer + the training loop shape (self-play with oracle reward).
2. **[Research 001](../.research/001_self_evolving_latent_first_agent.md)**: the self-evolving
   agents survey explicitly deferred the "Model" evolution category ("SFT/RL updates from
   coding feedback — Self-play SWE-RL, Agent-RLVR... Deferred — this is L4 LoRA"). This
   proposal is that category's activation, scoped to beat its failed first attempt.
3. **The L4 seam + Bench 049 harness** (`src/l4_fallback.rs`, `tests/` fixer-regate): the
   reachability contract (misses only), the decline-is-correct bar, and the strict-keep
   partial-credit metrics — the evaluation frame the trained arm must win in.
4. **[Plan 014](../.plans/014_clippy_bench.md) score-bench + T3.5.5 oracle**: the acceptance-rate
   harness (heal/created/safety/decline axes) and the uncontaminated labeling instrument —
   repurposed as training val set + RL reward.

The combination produces what none alone can: the corpus is frozen at ~450 rules (too small to
train a generator, exactly right to train a classifier+slot-filler); the verify stack makes
correctness free; and the oracle supplies dense labeled data across 15 repos of real code.

## GOAT gate

Gated per-phase; the full promotion gate before any default flip:

| Gate | Bar | Instrument |
|---|---|---|
| G1 correctness | never-wrong preserved: verifier-gated keeps only; decline stays default; oracle-labeled eval only (no proposer-verified circularity — the T3.5.4 lesson) | `clippy_oracle` + uncontaminated composite |
| G2 heal-rate | beats `decline` on the REFRESHED real-repo reachable-miss set; bar to beat = max(Bench 049 70B-arm, refreshed) | Bench 049 harness re-run at gate time |
| G3 no-regression | score-bench axes: heal_rate ≥ run#3 (98.1%), created ≤ 1.9%, safety 100%, decline 100%; default suites count-identical with feature OFF | Plan 014 `--score-bench` + CI guard |
| G4 latency/alloc | suggest path stays in the Bench 018 budget shape (µs-ms bi-encoder dot); PUCT only on contested spans; counting-allocator stable | `SuggestScratch` pattern |
| G5 acceptance-rate curve (moka axis) | drafter acceptance on held-out real-repo spans monotonically improves per training epoch — the self-play loop demonstrably learns | training telemetry, frozen dataset |
| G6 headroom axis | moves `decompose_headroom()`'s selector-addressable band on real-repo spans from 0 to nonzero — the Plan 017 instrument, positive this time | `src/draft/headroom_probe.rs` |

## What ships now (riir-clippy) vs deferred (riir-train)

### Ships now — riir-clippy (consumer + data + eval)
- Opt-in `trained_prior` feature (implies `latent_retrieval`): loads the frozen bi-encoder
  artifact at runtime, scores the fan-out pool, feeds the Structural re-ranker slot.
- Dataset engine extension of `src/draft/clippy_oracle.rs`: batch-label real spans across the
  workspace repos → frozen, BLAKE3-committed dataset snapshots.
- Eval integration: score-bench rows + Bench 049 harness arm for the trained drafter.
- (Phase 3) PUCT search module over contested spans, modelless-fallback preserved.

### Deferred — riir-train (training lane)
- The bi-encoder training loop (GDN towers, contrastive loss, M3 Metal first), slot-head
  training, freeze-snapshot export. Tiny model; no 4090 needed unless scaling the tower.

### Explicitly NOT shipped by this proposal
- No generative fix emission (the 0/60 shape) — rule-id + bounded template slots only.
- No LoRA on a general-purpose LM — the CF llama arm keeps its own leak-posture gate; this is
  a separate, domain-trained artifact.
- No default-on promotion without G1–G6; `decline` remains the default answer throughout.
- No new cargo dependency for riir-clippy — weights are runtime data (freeze/thaw), never a
  path dep; riir-train trains, riir-clippy consumes bytes.

## Phased rollout (sketch — [Plan 049](../.plans/049_trained_drafter_phased_rollout.md) expands)

### Phase 0 — kill-or-proceed (the Plan 017 instrument, trained arm)
- [ ] T0.1 Refresh the real-repo span set via the oracle; re-run `decompose_headroom()`; measure the trainable band. Empty → record negative, close, keep decline.

### Phase 1 — dataset + bi-encoder prior (feature `trained_prior`, opt-in)
- [ ] T1.1 Frozen dataset (oracle-labeled, BLAKE3-committed). T1.2 Train in riir-train (M3).
      T1.3 Freeze/thaw export. T1.4 Integrate as pool scorer behind the feature.

### Phase 2 — drafter heads for the reachable misses (via the existing L4 seam)
- [ ] T2.1 Slot-filling heads (template instantiation, bounded). T2.2 Bench 049 harness arm:
      strict-keep vs decline vs CF-70B on the refreshed set. Reachability contract unchanged.

### Phase 3 — PUCT search on contested spans (the moka transfer proper)
- [ ] T3.1 Policy+value heads, PUCT over top-k × proposer variants. T3.2 Self-play loop:
      oracle reward, dataset re-freeze per epoch, G5 curve.

## Risks

1. **Correctness risk:** low — the verifier stack gates every keep; the failure mode is wasted
   compute, not bad merges (never-wrong is structural, same as the L4 contract).
2. **Perf risk:** medium — the bi-encoder must stay µs-ms per span; a miss forces the
   modelless path unchanged (feature-off = byte-identical).
3. **Architectural risk:** low — runtime-loaded weights cross no sync boundary and add no
   cargo edge; the artifact is versioned data with a BLAKE3 commitment (stale-weights
   detection is a hash check, the corpus-commitment pattern).
4. **Evaluation-validity risk:** high if ignored — every eval must be oracle-labeled and
   uncontaminated; a single circular eval (proposer-verified) repeats T3.5.4's +12pp lie.

## Out of scope

- Retraining the corpus itself or expanding rule counts (that's ordinary batch mining).
- Serving the trained model as a daemon (the `bonsai_clippy_l4_eval --serve` transport stays
  as-is; the bi-encoder loads in-process).
- KAT/billing for the training lane; GPU budget beyond M3 Metal for Phase 1.
- Any public-surface change (source secrecy rules apply — the artifact is private, like the
  binary).

## References

1. Leviathan, Kalman, Matias. "Fast Inference from Transformers via Speculative Decoding"
   (arXiv:2211.17192) — **distilled in-house** as katgpt-rs Plan 004 (DFlash/SpeculativeVerifier
   lineage; the draft-verify acceptance-rate frame this proposal borrows).
2. DFlash2 (inco.ai blog; llama.cpp PR #27342) — **distilled in-house** as Research 077 +
   Plan 017 (the selection-headroom instrument and its negative).
3. Zhou, Hu, Shang, Zhang. "Self-Evolving Coding Agents" (arXiv:2608.03392) — **distilled
   in-house** as Research 001 (the 5-category taxonomy; "Model" category = this proposal).
4. Silver et al., AlphaZero (Science 2018) — cited-only; the PUCT policy+value lineage, already
   ported in-repo as moka (Issues 204–207, Bench 205) — we cite the port, not re-derive it.
5. Alon et al., code2vec (arXiv:1806.09473); Mou et al., TBCNN (arXiv:1409.5718) — cited from
   knowledge (AST-path / tree-conv representation prior art; MCP search quota exhausted at
   filing time — re-verify at plan-execution time if the AST-path refinement phase opens).
6. Bench 049 (fixer re-gate POC: CF llama 5–6/9, decline 0/9), Bench 031 (fix_verify 14.3%
   first-pass rejection — the compiles-per-heal perf axis), Bench 043 (headroom negative),
   Bench 463/467 + riir-train Plan 336 (the 0/60 generative-LoRA failure this design avoids).

## TL;DR

**Verdict: CONDITIONAL GO** — train the bounded drafter (bi-encoder first, PUCT later), behind
an opt-in feature with decline as default, gated on G1–G6 with the Plan 017 instrument as the
Phase-0 kill switch. Next action: run [Plan 049](../.plans/049_trained_drafter_phased_rollout.md)
Phase 0 (refresh + measure the trainable band) before any training code is written.
