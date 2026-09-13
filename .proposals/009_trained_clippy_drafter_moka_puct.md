# Proposal 009 — Trained clippy drafter: the moka-PUCT transfer (bounded draft-and-verify for the healer)

Status: **REVIEWED 2026-08-29 — verdict REFRAME (not CONDITIONAL GO).** Phase 1 (bi-encoder pool prior) is NO-GO on committed evidence (Bench 043 raw table 3: selector-addressable band 0, pool@16 100%); Phase 2 (proposer coverage) is promoted; Phase 0 re-points at the proposer-coverage band and needs two unlisted plumbing tasks. Three premises measurably wrong (corpus ~450→41 relevant; L4 "unarmed" stale per Bench 049; prior-art gap TFix/Getafix/CodeCureAgent/PredicateFix). See §Review verdict at the end.
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: moka PUCT (katgpt-rs Issues 204–207 / Bench 205) × [Research 001](../.research/001_self_evolving_latent_first_agent.md) "Model evolution" (the deferred category) × [Plan 017](../.plans/017_dflash2_selection_headroom_poc.md) headroom instrument × [Plan 014](../.plans/014_clippy_bench.md) score-bench × the `l4_lora` seam
Related: [Research 077](../.research/077_dflash2_draft_only_distillation.md) (DFlash2 draft-only distill), Bench 031 (fix_verify), Bench 043 (selection headroom negative), Bench 049 (fixer re-gate POC), riir-train Plan 336 (the 0/60 LoRA predecessor), katgpt-rs Plan 004 (Leviathan speculative-decoding distill)

## TL;DR

Train a **small domain-specific drafter** for the clippy healer — not a generative fixer. The
model learns `P(rule | span)` + fix-template slot bindings over the ~450-rule bounded corpus
(sizing corrected 2026-08-29 — §F2: **577 rules total, only 41 clippy + 94 perf default-on**),
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
span-conditioned, type-aware prior exists anywhere in the healer, and the L4 seam
(`src/l4_fallback.rs`, armed, miss-only reachability contract) sits UNARMED with no candidate
shaped to beat decline. **(STALE 2026-08-29 — §F3: Bench 049 resolved — the CF-70B arm already
measures 5–6/9 strict-keep vs decline 0/9; the re-plan below arms the seam with that measured
arm instead of training a new one.)** Without this, the healer's misses stay permanently declined and the
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

The combination produces what none alone can: the corpus is bounded (577 rules total,
of which **41 clippy lints** are the classes a clippy drafter classifies over — the
corrected F2 figure), too small to train a generator, exactly right to train a
classifier+slot-filler; the verify stack makes
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

> **SUPERSEDED 2026-08-29** by §Review verdict §Recommended re-plan (Phase 1 killed on Bench 043;
> Phase 2 promoted; Phase 0 re-pointed at the proposer-coverage band + two unlisted plumbing
> tasks). Kept as the filed record.

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
7. Drozdova et al., "Diffusion as a Training Curriculum for Timestep-Free Iterative
   Reasoning" (arXiv:2609.01449) — **distilled in-house** as riir-train Research 449 + Plan
   387 (the `riir-bench-algo` Sudoku-Extreme/Maze-Unique bench, Phase 1 landed 2026-09-05).
   Feeds the settle-refiner fusion lane (§Addendum 2026-09-05 below) and the Phase-3 PUCT
   prior-art challenge.

## TL;DR

**Verdict: CONDITIONAL GO** — train the bounded drafter (bi-encoder first, PUCT later), behind
an opt-in feature with decline as default, gated on G1–G6 with the Plan 017 instrument as the
Phase-0 kill switch. Next action: run [Plan 049](../.plans/049_trained_drafter_phased_rollout.md)
Phase 0 (refresh + measure the trainable band) before any training code is written.

---

## Review verdict — 2026-08-29 (adversarial re-read, evidence re-grepped + online prior-art sweep)

**Verdict: REFRAME — not CONDITIONAL GO as filed.** Phase 1 is **NO-GO on already-committed
evidence**; Phase 2 is the only phase with a non-empty denominator and should be promoted to
Phase 1; Phase 0's kill switch points at the wrong band AND is not executable as written.
Three of the proposal's load-bearing premises are measurably wrong.

### F1 — Phase 1's band is not "unmeasured", it is *already decomposed and empty*

Proposal §design: "the trained bi-encoder's entire value proposition is adding type/semantic
signal the parse-only features lack." [Bench 043](../.benchmarks/043_dflash2_selection_headroom_negative.md)
raw table 3 already attributes every residual miss:

| cause | spans | recoverable by |
|---|---|---|
| type-blind exclusion | 4 | `RerankMode::All` — **already shipped**, declined on precision (Bench 022/027) |
| pool depth | 1 | `RERANK_POOL` 12→16 — a constant |
| unfixable target (no proposer) | 2 | **writing the proposer** |
| selector-addressable | **0** | — |

Bench 043 also measures **zero retrieval misses at any depth** — `pool@16 = 100.0%` on every
surface. A tower whose output is *a score over a pool that always contains the answer* has a
structural ceiling of 4 fixture spans, and those 4 are not a missing-signal problem: `All`
already promotes them and was rejected because it over-matches on production code
(`created_rate`). So Phase 1's real ask is "be more precise than a blunt on/off rule on 4
spans" — that is Plan 017's refuted selector wearing trained weights, which is exactly the
failure mode caveat #1 warns about. It should be killed on the existing evidence, not
re-litigated with a GPU.

### F2 — the corpus-sizing argument is off by ~10× in the direction that matters

Proposal: "the corpus is frozen at ~450 rules (too small to train a generator, exactly right
to train a classifier+slot-filler)." Measured `.benchmarks/corpus_history.jsonl` 2026-08-29:

```
clippy_lints 41 · rust_perf 94 · sec 11 · docker 2 · kernel_opt 400   (total 548)
```

> **Correction 2026-08-29 (author's LGTM pass was right; my arithmetic was not).** Both that
> jsonl row AND my cross-check grep undercount `docker`: its rules are built through a
> `rule(...)` helper in `src/domains/docker/corpus.rs`, so a `DockerRule {` struct-literal
> grep sees **1** and the projection records **2**, while `entries()` actually returns **31**
> (`rule(` call count, measured). Corrected totals: **577 total, of which 400 (69%) are
> `kernel_opt`.** Two consequences: (i) **fix the projection** — `corpus_history.jsonl`'s
> docker series is wrong, and any domain using a builder helper is equally suspect, so the
> count must be derived from `build_*_seed_corpus().len()` at runtime, never by grep
> (this note included); (ii) **F2's conclusion is unchanged** — `kernel_opt` still dominates
> and the corpus a clippy drafter would classify over is still **41 clippy lints**, which is
> the number the sizing argument actually turns on.

**400 of 577 (69%) are `kernel_opt`** — a default-OFF GPU-kernel domain that the decline/L4
lane does not serve. The corpus the clippy drafter would actually classify over is
**41 clippy lints** (+94 perf rules). "41 classes, pool recall already 100%@16" is a
materially weaker training proposition than "~450 rules". Fix the number before re-arguing
the design.

### F3 — the motivating claim "the L4 seam sits UNARMED with no candidate shaped to beat decline" is stale

[Bench 049](../.benchmarks/049_fixer_regate_poc.md) resolved 2026-08-26: CF llama-3.3-70B
strict-keep **5/9 (arm A) and 6/9 (arm B)** vs **decline 0/9** — "**RESOLVED — the falsifiable
question answers YES**". An arm shaped to beat decline exists and has already met the bar; it
is simply not wired into `HealPipeline`. The cheapest path to closing real-repo misses is
**arming the seam with the measured arm**, not training a new artifact to re-win a bar
something else already won.

### F4 — Phase 0 is not executable as written (concrete plumbing gap)

- `decompose_headroom()` (`src/draft/headroom_probe.rs:466`) takes **no arguments** and reaches
  `run_headroom_probe_with(RerankMode)` → `labeled_eval_spans()`
  (`src/draft/retrieval_eval.rs:480`), which iterates two hardcoded consts
  (`eval_fixtures_clippy::CLIPPY_SPANS`, `eval_fixtures_perf::PERF_SPANS`). There is **no span-set
  injection point**. Plan 049 T0.2 says "re-run `decompose_headroom()`" on a refreshed real-repo
  set — that cannot be done without first parameterizing the span source.
- `ClippyOracle` (`src/draft/clippy_oracle.rs`) exposes `lints_for(&str)` only — a
  single-snippet API. T0.1's "extend the batch path" is writing a batch path, not extending one.

Both are ordinary work, but they are **unlisted tasks on the critical path of a kill switch**,
and the plan reads as if the instruments already accept the new inputs.

### F5 — prior-art gap: the four closest papers are missing, and the search excuse does not hold

The proposal cites code2vec (2018) + TBCNN (2014) "from knowledge — MCP search quota exhausted
(resets 2026-09-15)". Plain `WebSearch` answered on the first try today; the quota claim only
covered the MCP path. The uncited competitors are each closer to this design than either
representation paper:

| Paper | Why it is load-bearing here |
|---|---|
| **TFix** (Berabi/He/Raychev/Vechev, ICML 2021) | *Literally Phase 2.* Fine-tuned T5 on linter warnings, 52 ESLint error types, **~67% correct fixes**. The "small specialist over bounded lint corpus" bet has a published number to beat — and TFix used **free text**, not bounded templates, contradicting the proposal's framing that free-text emission is the doomed shape. |
| **Getafix** (Bader et al., OOPSLA 2019, deployed at Meta) | *Literally the modelless L1 + a learned ranker.* Anti-unification + hierarchical clustering of past fixes, context-based ranking; **top-1 exact human fix 12–91% by category**, 1,268 fixes. Shows the ranking win comes from edit-pattern generalization, not from a learned span embedding. |
| **CodeCureAgent** (arXiv:2509.11787, 2025) | *Riir-clippy's verifier stack, already SOTA without a trained drafter.* Three-step approval — build → warning gone with no new warnings → tests pass — i.e. `clippy_verify` + `fix_verify`. **96.8% plausible / 86.3% manually-confirmed correct on 1,000 SonarQube warnings, 291 rules, $0.029 and ~4 min per warning.** This is the honest bar for "arm the L4 seam", and it is a general LLM in an agent loop. |
| **PredicateFix** (arXiv:2503.12205, 2025) | The closest thing to the rule tower — and it wins by retrieving **the rule's predicates as a bridge**, a *symbolic* artifact, not a learned rule embedding: **+27.1–69.3% correct repairs over RAG baselines** (CodeQL, GoInsight). Suggests the cheap win on our side is richer rule-side symbolic evidence, which is what `RerankMode::Structural` already is. |
| **Sorald** (arXiv:2103.12033) | 30 hand-written SonarQube template fixers — the `FIXABLE_LINTS` analogue, and evidence that proposer coverage is the ordinary, effective lever. |

Phase 3's PUCT novelty claim also needs narrowing: tree search over partial patches with a
policy prior is published (FLAMES; value-guided MCTS decoding). The transferable claim is the
in-house *port*, which the proposal already correctly frames — but it should say so against
these, not against AlphaZero alone.

### What survives

The **reframing** survives and is genuinely good: bounded output + verifier-gated correctness
turns quality into an acceptance-rate problem, and that is the right response to the 0/60
(Bench 467) generative failure. The **modelless-first tension resolution** and the **decline-is-
default posture** survive. What does not survive is the phase order, the sizing, and the
choice of kill-switch band.

### Recommended re-plan (supersedes §Phased rollout)

1. **Kill Phase 1 (bi-encoder pool prior).** Bench 043 is the negative; do not spend a GPU hour
   re-deriving it. Re-open only if a *retrieval* miss (not selection) is ever measured.
2. **Re-point the Phase-0 kill switch** from the selection band (known 0) to the
   **proposer-coverage band**: of oracle-labeled real-repo spans, what fraction have the correct
   rule at top-1 and **no proposer**? Bench 043 puts it at 2/101 on fixtures — nonzero, and the
   only band Bench 043 says is real ("Writing the proposer is the fix, not selecting better").
3. **Modelless-first before trained-first.** Write the missing deterministic proposers
   (`dyn-to-enum`, `shrink-to-fit-after-build`, the Getafix/Sorald shape — zero training), then
   re-measure. Only the residual after that is an honest training denominator, and it must be
   compared against TFix's ~67% and Getafix's 12–91%.
4. **Arm the L4 seam with the arm that already passed** (Bench 049 CF-70B, 5–6/9) plus
   CodeCureAgent's iterate loop. This is the largest measured win available and needs no new
   artifact. *Guard (2026-08-29 LGTM pass): the ENABLE flip stays owner-gated on leak-posture
   sign-off (`.docs/10_self_evolve/l4_fallback.md` §5) — wiring may land opt-in-dark; and the
   loop's keep criterion is the FULL gate (warning gone + no new warnings + compile + tests),
   never "warning cleared" alone.* (This arm is also the prerequisite
   [Proposal 011](011_kat_pay_request_qr_genesis_and_onboarding.md) §Baz names — "the CF-llama
   arm awaiting the owner's leak-posture sign-off" — so arming it is the shared next step for
   both proposals.)
5. **Add the unlisted plumbing as explicit Phase-0 tasks**: parameterize the headroom span
   source; add a `ClippyOracle` repo-walk batch labeler. Nothing downstream can run without them.
6. **Fix the corpus number** (577 total / **41 clippy** relevant — the corrected figure in the F2 blockquote) wherever "~450" appears.

**Bottom line:** the mechanism is sound and the discipline is exemplary, but as phased this
plan spends its first (and most expensive) phase attacking a band its own committed instrument
already measured at zero, while the band that instrument says is real — proposer coverage — is
deferred to Phase 2 and is addressable **modellessly**. Reframe, do not cancel.

---

## Addendum — 2026-09-05: the settle-refiner fusion lane (DEFERRED — re-plan priorities UNCHANGED)

Source: riir-train [Research 449](../riir-train/.research/449_Diffusion_Curriculum_Timestep_Free_Iterative_Reasoning.md)
(arXiv:2609.01449) + riir-train [Plan 387](../riir-train/.plans/387_sudoku_maze_algo_reasoning_bench.md)
(Sudoku-Extreme/Maze-Unique bench, `riir-bench-algo` Phase 1 landed). Owner direction
recorded: training around this is acceptable (tiny params, hours-class on M3) but **lower
priority than the re-plan above**; posture is freeze/thaw (weights as committed DATA), and
the design should FUSE with the healer's existing stack — the moka+PUCT precedent, not a
wholesale port.

### The fusion (moka+PUCT shape, one layer over)

moka+PUCT fused policy/value search with draft-and-verify. This fuses the paper's
**single-trajectory settle solver** with the same verifier stack:

- **Mechanism transposed.** The paper's solver holds a persistent hidden state, injects
  fresh noise on the non-clue cells each step, and SETTLES on the answer — no parallel
  rollouts, no selection head (it explicitly names PTRM/GRAM's parallel-trajectory +
  selection-head design — PUCT's family — as machinery it does not need; 99.90% at K=10k).
  Transposed: a **settle-refiner drafter** — persistent state over the span, each step emits
  a bounded `(rule, slot)` action, noise = exploration over the action space, settle = the
  EXISTING verify gates pass. Our verifier is CHEAPER than the paper's internal one and our
  fix space is multi-solution, so the open hypothesis the POC measures: does
  sample-until-verify with a settle-style refiner match or beat tree ordering (PUCT) at
  lower per-fix cost?
- **Curriculum transposed — SPECULATIVE, flagged as such.** "Violation = corruption, fixed
  span = clean target": denoising-style training on template-perturbed spans (annealed
  corruption during training, max-noise + patience at inference). The paper's corruption
  path is CONTINUOUS diffusion; our action space is discrete — this port is a hypothesis to
  test, not a result to cite.
- **Weight posture.** Tiny model (~250k class), trained ONCE in riir-train, shipped as a
  BLAKE3-committed freeze artifact consumed as DATA — the proposal's existing boundary
  ("weights are bytes, not a cargo dep") unchanged. Updates = re-freeze a new snapshot,
  never in-place; fine-tuning = a new snapshot on top.

### Next actions (in order — an agent picking this up starts here)

1. **Do NOT start here.** The re-plan above (steps 1–6) owns the critical path: modelless
   proposers for the proposer-coverage band first, arm the L4 seam with the Bench 049 arm,
   land the §F4 plumbing (parameterized headroom span source + `ClippyOracle` batch
   labeler). This lane activates ONLY on the residual training denominator that re-plan
   step 3 defines — the POC below CONSUMES the same batch-labeler plumbing.
2. **Capability-gate the trainer first (riir-train Plan 387 Phase 2).** The tiny-reasoner
   trainer must reproduce published-class numbers on the `riir-bench-algo` harness
   (Sudoku-Extreme ≥ 99% on the 10k ablation subset @ K=10k; Maze-Unique ≥ 98% @ K=100)
   BEFORE any clippy-side training. One trainer, two consumers — do not build a second.
3. **POC before training (defend-wrong, `riir-poc`).** Three arms on oracle-labeled
   real-repo spans: (a) decline baseline; (b) one-shot prior + verify loop
   (sample-until-verify, re-ranker as prior — the modelless-reachable shape); (c)
   settle-refiner loop (persistent state + noise injection + patience, verifier as settle
   criterion). Proof bar: (c) strict-keep ≥ (b) AND directionally vs the Bench 049 CF-70B
   arm at lower per-fix cost; G1 never-wrong preserved (verifier-gated keeps only, decline
   stays default). POC harness may reuse the paper's settle classifier
   (`riir-bench-algo::eval::classify_settling`) as the stability instrument.
4. **Training (only after POC pass).** Denoising curriculum on frozen oracle-labeled spans
   (BLAKE3-committed, per §evaluation-validity risk); M3 Metal, hours-class. G5 gains the
   acceptance-vs-depth axis + the settle classification (settled-and-stayed /
   settled-then-wandered / never-settled). G2/G3/G6 bars unchanged.
5. **Phase 3 (PUCT) impact.** PUCT stays the contested-span search layer, but arm (c) is
   now its measured comparator — arXiv:2609.01449 is the prior-art datapoint that
   single-trajectory settle matched parallel+selection in its domain; ours differs (cheap
   external verifier, multi-solution), so this is a measurement obligation, not an
   assumption either way.

## Addendum — 2026-09-11: HRM-Text second-pass recipe feeds the TRAINER lane (stop rule UNCHANGED)

Owner asked whether Plan 082b's HRM-Text distill (which "got really good results for 1B")
fuses further into this proposal. Answer, honest: **not into the drafter lane — into the
trainer.** Plan 049's stop rule has FIRED (in-vocab band 0.94pp < 2.0pp) and the settle-refiner
prerequisite (trainer capability gate, next-action 2 above) FAILED with riir-train Plan 387
Phase 3 (2.0%/0.0% exact-match, representational plateau). A second-pass HRM-Text re-mine
(katgpt-rs [Research 547](../../katgpt-rs/.research/547_HRM_Text_Second_Pass_Training_Recipe.md),
pinned `aaa948ea`) landed the missing recipe — sum-reduction accounting, two-level bp
allocation, Adam-atan2+EMA wiring with the v1-divergence stress arm, LPT packing, learned
carry init — as **riir-train Plan 394** (OPEN, NOT SCHEDULED), plus a modelless queue in
katgpt-rs Issue 744. This proposal's reopen trigger stays corpus-side (the fired-histogram
roadmap); Plan 394 only shortens the path IF the corpus band ever clears 2pp and the
capability gate is re-run. The one live takeaway for THIS repo: modelless extraction #1
(one-cumsum segmented reduction) has `AstChunker`-stream + oracle-metric consumers with no
dependency on any training lane.

## Addendum — 2026-09-13: the fly-connectome wave as external evidence on this proposal's architecture (re-plan priorities UNCHANGED)

Source: the awesome-fly landscape dig (3 subagents; 10 repos cloned at pinned SHAs + 44
README-swept; full record in [riir-ai Research 379](../../riir-ai/.research/379_fly_connectome_fixed_wiring_reservoir.md)
§10). The wave ran this proposal's exact architecture — frozen substrate + tiny trained
readout — a dozen times independently, mostly with exemplary controls. What it says here:

1. **External corroboration of the F1 kill (four independent negatives of the same shape).**
   In every controlled instance, the substrate-advantage-over-cheap-features claim FAILED:
   FLM's parameter-matched direct-input control (no graph) slightly **beat** the
   278k-param adapter over the 25.6M-edge connectome; neuroterrarium's 20-shuffled-graphs
   null (full ≈ shuffled; no-recurrence slightly better, disjoint train/val/test);
   neurocraft's trained readout **95.66×/403.70× worse** than a matched direct controller;
   fly-craftax's DN rates "indistinguishable from any noisy interoceptive feature vector".
   Same shape as pool@16 = 100% locally: a learned prior over features that already contain
   the answer adds nothing. Re-plan step 1 stands with external support.
2. **The surviving branch is the modelless-substrate branch** — external validation of
   re-plan step 3: the wave's clean positives came from building the substrate, not the head
   (flyjump's circuit is anatomy-selected; flyhard 0/100→100/100 came from the embodiment
   interface). And doomfly's six in-substrate plasticity versions ALL failed their
   predeclared gates (causal-but-harmful memory; shuffled-timing learned the same harmful
   change) — freeze-the-corpus, train-at-most-a-bounded-head is the empirically surviving
   split, matching the 0/60-vs-draft-and-verify reframing.
3. **Mandatory controls for any future trained-prior gate (the import list):**
   - **direct-feature control** (FLM): matched head over cheap features (rule frequency,
     lint density, file shape) — without it, a "corpus as substrate" claim is unfounded;
   - **exact-disconnect silencing** (FLM zero-init bias-free adapter ⇒ bit-identical base,
     unit-tested; flyjump silenced 99→0): untrained/silenced prior MUST degenerate to
     corpus-only behavior bit-identically — externally validates opt-in-dark + G1
     never-wrong;
   - **null ladder** (openfly's 4 trading controls incl. shuffled-label; FlyDoom's
     ER-matched + Maslov–Sneppen degree-preserving + weight-shuffle): shuffle rule↔span
     labels before believing any trained-prior gain;
   - **full-gate keep criterion stays** (fly-craftax's PPO head became "a clock with a small
     drive-dependent jitter", farmed self-generated `wake_up` achievements while scoring
     well, open-loop alternation beat it on survival): externally validates the LGTM guard
     (warning gone + no new warnings + compile + tests — never "warning cleared" alone).
4. **Settle-refiner stop-rule datapoints** (the 09-05 addendum's lane): flyjump's champion
   landed at generation 32/80 (the other 48 added nothing); fly-craftax's return plateaued
   at update ~40/150 ("the remaining 110 updates lowered entropy without raising return").
   The useful signal arrives in the first fraction of any fitting budget — patience/early-stop
   belongs in that POC harness from day one.
5. **PUCT half: still zero external instances.** No project in the wave runs search over
   the substrate (MCTS/PUCT/beam — zero in 44+ repos). Phase 3 remains the in-house moka
   lineage bet exactly as §F5 narrowed it; the wave neither supports nor refutes it.
6. **Tonic-feature guard** (doomfly's constant lamina bias drove actions under black input,
   black beat live vision 3:1; fly-craftax's floors ended up setting the gain): normalize/
   floor features so constant channels (language, file length) cannot dominate a prior.

**Verdict: evidence addendum only.** The 2026-08-29 re-plan and the 09-05/09-11 addenda
priorities are UNCHANGED (stop rule still fired; reopen trigger still corpus-side). If the
corpus band ever clears and a trained-prior gate runs, §3's controls list is the import.
Pinned SHAs: FLM `7251a89` · fly-craftax `2fe4145` · flyjump `c08c86b` · FlyDoom `a9d9dba`
· AxonWeave `106d00e` · connectome_interpreter `d212f86` · RuVector `8667f1f` (the only
MIT-derivable Rust LIF kernel).
