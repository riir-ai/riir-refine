# Research 001 — Self-Evolving Coding Agents → Latent-First Self-Evolution Loop for riir-clippy

**Paper:** Zhou, Hu, Shang, Zhang. "Self-Evolving Coding Agents." arXiv:2608.03392 [cs.SE], Aug 2026.
**Verdict:** **Gain** — survey reveals riir-clippy's gap (zero self-evolution); the fix is a latent-first loop borrowing patterns from riir-agents + riir-rag, implemented locally (boundary-compliant).
**Date:** 2026-08-11

> **PASS-Redirects (synthesis):** Ning, Tieu, Fu, Wei et al. [arXiv:2605.18747 "Code as Agent Harness: Toward Executable, Verifiable, and Stateful Agent Systems"] (UIUC+Meta+Stanford, 102-page survey, 2026-05) — the field-level taxonomy arriving AFTER this note's loop shipped: `cargo heal` IS an instance of its thesis (§3.4 PEV loop = `fix_verify` compile gate; §3.5 AHE deep telemetry/Evolution Agent/governed mutation = TrajStore + LatentSkillEvolution + EvidenceTier + score-bench; §5.2.1 oracle adequacy = the clippy oracle + Bench 041 premise probe; §5.2.3 self-evolution-without-regression = Issue-037 E1–E5 + A1 cadence + absorbing-Withdrawn tiers). PASS — the 2608.03392 gap it would have revealed is closed; its un-distilled citations (AutoHarness/Meta-Harness/AHE) are the AIDE² LLM-dependent class per [katgpt-rs/.research/440](../../katgpt-rs/.research/440_AIDE2_Recursive_Self_Improvement_PASS.md).

## TL;DR

The survey organizes self-evolving LLM coding agents into a 5-category taxonomy
(framework / memory / skills / tools / model evolution). riir-clippy currently
has **zero** of these evolution loops — the corpus is frozen at 10 seed lints,
the drafter's direction vectors never update from feedback, and there's no
memory of past fix attempts.

The actionable insight: implement the survey's **memory + skill** evolution
categories as a **latent-first** loop — using `dot(fix_dir, context_dir) +
sigmoid` scoring (the riir-agents `ModellessReasoner` pattern) + 3-axis
retrieval (the riir-rag pattern: latent KNN + BM25 + graph k-hop), all
modellessly. This is the "latent first agent" the substrate was built for.

The survey's systems all evolve at the **LLM-token level** (reflection,
trajectory summarization, skill distillation via LLM). A modelless latent-first
approach — where memory retrieval and skill scoring happen via dot-product +
sigmoid, no LLM, no backprop — is the novel angle our stack enables.

## The survey's taxonomy (5 evolution objects)

| Object | What evolves | Representative systems | riir-clippy applicability |
|---|---|---|---|
| **Framework** | Agent edits its own scaffold/code | SICA, SIFT, STOP, Darwin Gödel Machine | **N/A** — riir-clippy is modelless, not an LLM agent loop that rewrites itself |
| **Memory** | Accumulate fix trajectories, retrieve for future tasks | SWE-Exp, EvoCoder, Repository Memory, EvoRepair, SAGE | **YES — the primary gap** |
| **Skills/tools** | Distill trajectories into reusable procedures/tool bank | CODESKILL, GSkill, Socratic-SWE, EffiSkill, Live-SWE-Agent | **YES — drafter direction vectors should evolve** |
| **Model** | SFT/RL updates from coding feedback | Self-play SWE-RL, Agent-RLVR, ReVeal, CURE, ZeroCoder | **Deferred** — this is L4 LoRA, already deferred to riir-train |
| **Workflow/topology** | Multi-agent collaboration graphs adapt | SEMAG, EvoMAC, AgentConductor, SEW, AFlow | **N/A** — riir-clippy is single-agent |

Two categories apply: **memory** + **skills**. The other three are either
N/A (framework, workflow — we're modelless single-agent) or already deferred
(model — L4 LoRA).

## The gap: riir-clippy has no evolution loop

Current state (per Proposal 034 + README):

```
clippy lint → L0 corpus (static 10-lint seed) → L1 drafter (frozen weights) → L2 pruner → fix
                                         ↑ no feedback ↑ no memory ↑ no skill update ↑
```

What's missing (per the survey's taxonomy):

1. **No memory** — successful/failed fixes are not stored. The next time the
   same lint appears on different code, the drafter starts from scratch. This
   is the SWE-Exp / Repository Memory gap: "a memoryless coding agent must
   rediscover such information for every new issue."

2. **No skill evolution** — the drafter's ternary direction vectors are frozen
   at load time. If a direction vector produces bad fixes, there's no
   correction. If it produces good fixes on a new lint variant, that success
   doesn't reinforce the direction. This is the CODESKILL / GSkill gap:
   "skill management itself [should be] a learnable policy."

## The latent-first design (the "latent first agent")

Borrow two patterns from riir-ai (boundary-compliant — reimplemented locally,
not imported):

### Pattern 1: `dot + sigmoid` latent scoring (from `ModellessReasoner`)

riir-agents' `ModellessReasoner` (riir-ai/crates/riir-agents/src/modelless.rs)
scores task relevance via:
```rust
score = sigmoid(dot(task_dir, context_dir) / temperature)  // ∈ [0, 1]
```
where `task_dir` and `context_dir` are 8-dim latent direction vectors. The
reasoner already lists "code refactor" as a target task kind.

**riir-clippy adaptation:** score fix candidates the same way:
```rust
fix_score = sigmoid(dot(fix_direction, lint_context_direction) / temp)
```
where `fix_direction` is a BLAKE3-deterministic embedding of the (lint_name,
buggy_code, fixed_code) tuple, and `lint_context_direction` embeds the current
(lint_name, buggy_code) context.

### Pattern 2: 3-axis retrieval (from riir-rag)

riir-rag (riir-ai/crates/riir-rag/) retrieves context via 3 additive axes:
- **Latent KNN**: cosine(query.direction, chunk.embedding)
- **BM25 lexical**: exact symbol/token match
- **Graph k-hop**: lint→lint structural similarity (optional)

**riir-clippy adaptation:** retrieve past fix trajectories the same way:
- **Latent**: `dot(embed(current_lint), embed(past_fix))` — "similar lint+code"
- **BM25**: exact `lint_name` match — "same lint, different code"
- **Graph**: lint taxonomy adjacency — "related lint family" (e.g.,
  `needless_return` ↔ `needless_question_mark`)

### The self-evolution loop

```
clippy lint enters
    │
    ▼
┌─────────────────────────────────────────────────────────┐
│ L_memory: LatentFixMemory (NEW)                          │
│                                                          │
│ Query: { direction: embed(lint+code),                   │
│          symbols: [lint_name],                           │
│          text: buggy_code }                              │
│                                                          │
│ Retrieve top-K past fix trajectories via:                │
│   - latent KNN: dot(embed(query), embed(past_fix))+sig   │
│   - BM25: lint_name + code tokens                        │
│   - graph: lint→lint k-hop (optional)                    │
│                                                          │
│ Returns: Vec<FixTrajectory { lint, buggy, fixed,         │
│             success_count, fail_count, direction }>      │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│ L1 drafter (EXISTING, enhanced)                          │
│                                                          │
│ PlasmaPath ternary matvec (frozen weights)              │
│   + retrieved FixTrajectories as additional candidates   │
│ Score ALL candidates: sigmoid(dot(candidate_dir,         │
│                             context_dir))                │
│ Pick top-scoring valid candidate                         │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│ L2 pruner (EXISTING)                                     │
│ syn parse check → valid/invalid                          │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
              [ fix applied ]
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│ L_evolve: LatentSkillEvolution (NEW)                     │
│                                                          │
│ On outcome (success/fail):                               │
│   1. Store trajectory in LatentFixMemory                 │
│   2. Update drafter direction vectors (modelless):       │
│      On success:                                         │
│        direction[lint] += α·(fix_dir - direction[lint])  │
│      On fail:                                            │
│        direction[lint] -= β·(fix_dir - direction[lint])  │
│      (vector arithmetic — NOT gradient descent)          │
└─────────────────────────────────────────────────────────┘
```

### Why this is modelless (not training)

The direction-vector update is pure vector arithmetic:
```
direction_new = direction_old + α · (observed_direction - direction_old)
```
This is an **exponential moving average** — the same modelless latent update
pattern used by `CommittedFieldBlend` (Plan 321) and `PersonalityWeightedComposition`
(Plan 297) in katgpt-rs. No loss function, no gradient, no backprop. The
drafter's frozen ternary weights get a **deterministic reader-side correction**
applied at retrieval time — the freeze/thaw invariant is preserved (the base
snapshot never mutates; the correction is an overlay).

This is the **freeze/thaw-over-fine-tuning** rule (AGENTS.md constraint #3)
applied to the clippy healer: the base weights are frozen, the direction
corrections are versioned overlays.

## Boundary compliance (non-negotiable)

riir-clippy's AGENTS.md says: "Only public dep: `katgpt-core`. No dependency on
`riir-engine`, `riir-games`, or any game crate."

riir-agents and riir-rag are in riir-ai (private game runtime). riir-clippy
**CANNOT** depend on them. The riir-ai AGENTS.md explicitly sanctions the
pattern-borrowing path:

> "A new project may borrow an architectural pattern from a riir-ai crate
> without needing a code-level dependency. If the pattern can be reimplemented
> in ~200 LOC and the new project has no other game-domain coupling, it belongs
> in a new repo, not as a new module in riir-ai."

**Canonical precedent:** riir-clippy itself — it already borrows the
`TernaryDraftModel` + `SealQuestCorpus` + `ConstraintPruner` pattern from
`quest_grammar` without a code-level dep.

**What riir-clippy borrows (pattern, not code):**
- `dot + sigmoid` scoring (riir-agents `ModellessReasoner::score`) — ~10 LOC
- 3-axis retrieval shape (riir-rag's latent+BM25+graph) — ~50-100 LOC for
  latent KNN + BM25 over fix trajectories (graph k-hop is optional, defer)
- BLAKE3-deterministic embedding (riir-rag `ModellessEmbedder`) — ~30 LOC
  using katgpt-core's `blake3_hash` + `simd_sigmoid`

**What riir-clippy uses from katgpt-core (the legal dep):**
- `simd_dot` / `simd_sigmoid` — the latent op primitives
- `blake3` hashing — deterministic embedding seed
- `ConstraintPruner` trait — the existing L2 pruner substrate

**Estimated local impl:** ~200-300 LOC for the memory + scoring + evolution
loop. Well within the "pattern borrowing" threshold.

## Fusion lineage

This fuses three substrate pieces + the survey's taxonomy:

1. **Survey taxonomy** (arXiv:2608.03392) — the 5-category "what evolves" map.
   Provides the design blueprint: memory + skill evolution are the applicable
   categories.

2. **riir-agents `ModellessReasoner`** (Plan 523, riir-ai) — the `dot + sigmoid`
   latent scoring pattern. Already lists "code refactor" as a target. Borrowed
   as the fix-candidate scoring substrate.

3. **riir-rag 3-axis retrieval** (Plan 524/526, riir-ai) — the latent KNN +
   BM25 + graph k-hop shape. Borrowed as the fix-trajectory memory retrieval
   substrate.

4. **`CommittedFieldBlend` / `PersonalityWeightedComposition`** (Plans 321/297,
   katgpt-rs) — the modelless EMA direction-vector update pattern. The
   evolution mechanism (direction += α·(observed - current)).

The combination produces what none alone can: a coding agent that evolves
its fix memory + drafter directions **modellessly**, using latent-space
operations — no LLM calls for evolution, no GPU training, no backprop.

## Novelty check

**Q1: No prior art?** The survey's systems ALL use LLMs at the token level for
evolution (LLM reflection, LLM trajectory summarization, LLM skill
distillation). A modelless latent-first approach using dot+sigmoid for both
memory retrieval and skill scoring is NOT in the surveyed literature.

**Q2: New class of behavior?** A coding agent that evolves modellessly — no
GPU, no LLM calls for the evolution loop. The drafter's direction vectors
update from successful fixes via EMA vector arithmetic. This is a different
mechanism class from all surveyed systems.

**Q3: Product selling point?** "Our clippy healer learns from every fix
without GPU training or LLM calls — the evolution loop is sub-microsecond
latent-space ops."

**Q4: Force multiplier?** Connects riir-clippy + riir-agents pattern +
riir-rag pattern + katgpt-core latent primitives.

**Honest caveat:** novelty is on the MECHANISM (modelless latent-first
evolution), NOT on the capability (clippy fixing — RustAssistant hit 75%
with LLM training). The GOAT gate must prove the latent-first loop
measurably improves healing rate over the static corpus before claiming
the evolution mechanism works.

## GOAT gate sketch

The POC issue (Issue 004, resolved + removed per noise-reduction; canonical record is `.docs/10_self_evolve/` + `.benchmarks/008_real_world_validation.md`) defined the gate:

| Gate | What | Target |
|---|---|---|
| **G1** | LatentFixMemory retrieval relevance — does `dot+sigmoid` rank past fixes correctly? | Top-1 retrieved fix is the same lint family ≥80% on a held-out set |
| **G2** | Retrieval latency — latent KNN + BM25 over N trajectories | <50µs per query at N=1000 (the riir-rag p50=123µs baseline, scaled down for smaller corpus) |
| **G3** | No-regression — existing L0-L2 tests still pass | All existing tests green |
| **G4** | Alloc-free steady state — the evolution loop doesn't allocate per-fix | 0 bytes steady-state (reuse scratch buffers) |
| **G5** | **The headline gate** — does self-evolution improve healing rate? | After N=100 fix attempts, healing rate on held-out lints improves by ≥5pp vs static-corpus baseline |

**G5 is the modelless-gain gate.** If the latent-first loop can't measurably
improve healing rate after accumulating fix trajectories, the evolution
mechanism doesn't work and we revert to the static corpus (L0-L2 as-is).

Per the UQ-bearing "Report the Floor" rule: this is NOT a UQ-bearing
primitive (no probability distribution / interval / coverage claim), so the
conformal-naive floor gate does not apply.

## Verdict: **Gain**

The survey reveals an actionable gap (zero self-evolution in riir-clippy) +
provides the design blueprint (memory + skill categories). The latent-first
implementation is a novel fusion of existing patterns. The POC (Issue 004)
will validate whether the mechanism works.

Not Super-GOAT because: (1) novelty is mechanism-level (modelless evolution),
not capability-level (clippy fixing is proven); (2) the GOAT gate is
unvalidated; (3) no cross-pillar connection (riir-clippy is a standalone
dev tool, not a game pillar amplifier).

## Distillation: what ships where

| Component | Repo | What |
|---|---|---|
| `LatentFixMemory` (3-axis retrieval over fix trajectories) | riir-clippy | ~150 LOC, borrows riir-rag pattern |
| `LatentSkillEvolution` (EMA direction update) | riir-clippy | ~50 LOC, borrows CommittedFieldBlend pattern |
| BLAKE3-deterministic `ModellessEmbedder` for (lint, code) | riir-clippy | ~30 LOC, uses katgpt-core blake3 |
| Fix-candidate scoring (`dot + sigmoid`) | riir-clippy | ~10 LOC, uses katgpt-core simd_dot/simd_sigmoid |
| Feature flag: `self_evolve` (opt-in, default-off until G5 passes) | riir-clippy | gates the whole loop |

**Nothing ships in katgpt-rs** — the primitives (`simd_dot`, `simd_sigmoid`,
`blake3`) already exist there. **Nothing ships in riir-ai** — the patterns
are borrowed, not imported.

## Deferred / future work

Validated + shipped (G1-G7 ALL PASS). Three items deferred pending
concrete triggers — recorded here so they're not lost when the issue
tracker is pruned.

### D1 — `self_evolve` stays opt-in (decision, not a defect)

**Trigger to revisit:** a deployment environment where `clippy_verify` is
on the default path (e.g. a CI bot with cargo always on PATH).

The mechanism is validated against real `cargo clippy` feedback (G7:
50.0pp improvement, 50% → 100%). The opt-in gate is a deployment-
environment constraint, not a correctness concern: `clippy_verify` is
itself opt-in (slow, ~130ms/verify, requires cargo+clippy on PATH), and
`self_evolve`'s feedback loop depends on it. Promoting `self_evolve` to
default-on without `clippy_verify` available means the loop can't run.

### D2 — Graph k-hop retrieval axis (research candidate)

**Trigger to revisit:** a concrete use case where latent + BM25 retrieval
is demonstrably insufficient (missed a fix that a lint→lint taxonomy
adjacency would have found).

The current `LatentFixMemory` has 2 retrieval axes:
1. **Latent** — 8-D vector KNN via BLAKE3-deterministic embedding
2. **BM25** — exact symbol match on lint_name + code tokens

A 3rd axis would be **graph k-hop** over a lint→lint taxonomy (e.g.
`needless_return` is adjacent to `needless_return_loop` is adjacent to
`redundant_closure`). The substrate exists in `riir-rag`'s `graph_rag`
feature (Plan 526) but adds ~2MB of dep weight (`kg_triple + syn`).

Defer until the 2-axis retrieval shows a concrete gap. TheGOAT G5/G7
results show 2 axes is sufficient for the current 10-lint seed corpus.

### D3 — Production-scale profiling (perf follow-up)

**Trigger to revisit:** `LatentFixMemory` grows beyond 10K trajectories,
or the `success_rate_for` lookup shows up in a profile.

The `success_rate_for(lint_name, fixed_code)` lookup added in Issue 008
(G7 fix) is a linear scan over all trajectories. Measured at ~6µs at
N=1000 (within the G2 latency budget). At production scale (10K+ 
trajectories accumulated over weeks of CI runs), this may become a
bottleneck.

**Mitigation when triggered:** add a `HashMap<(lint_name_hash,
fixed_code_hash), Stats>` index alongside the `Vec<FixTrajectory>`. 
The hashes are already computed for the latent axis; the index is a
one-line insert on `record_outcome`. No change to the retrieval API.

## Related Research

- [Proposal 034](../../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) — the clippy-healing design this extends
- [Plan 523 (riir-agents)](../../riir-ai/.plans/523_riir_agents_latent_space_task_reasoning.md) — the `ModellessReasoner` whose `dot+sigmoid` pattern we borrow
- [Plan 524/526 (riir-rag)](../../riir-ai/.plans/524_riir_rag_latent_first_retrieval_facade.md) — the 3-axis retrieval pattern we borrow
- [katgpt-rs `.research/302`](../../katgpt-rs/.research/302_FAME_Sampling_Invariant_Per_Entity_MoE.md) — `CommittedFieldBlend`, the EMA direction-update pattern
- [katgpt-rs `.research/276`](../../katgpt-rs/.research/276_Personality_Weighted_Latent_Layer_Composition.md) — `PersonalityWeightedComposition`, the sigmoid-gated composition pattern
- [katgpt-rs `.docs/06_game_arenas/heuristic_learning.md`](../../katgpt-rs/.docs/06_game_arenas/heuristic_learning.md) — the codebase's native "self-evolving via code updates" framing (HL)

> **Survey cross-reference:** Zhou, Hu, Shang, Zhang. "Self-Evolving Coding Agents." arXiv:2608.03392 [cs.SE], Aug 2026. This note distills the survey's 5-category taxonomy into the 2 categories applicable to riir-clippy (memory + skills), implemented modellessly via latent-first ops.
>
> **Related survey (Gain, distilled in riir-train):** Yang et al. [arXiv:2511.18538 "From Code Foundation Models to Agents and Applications: A Comprehensive Survey and Practical Guide to Code Intelligence"] — the broader parent survey covering the full Code LLM lifecycle (data curation → pre-training → SFT → RL → agents + probing experiments). SFT/RL/scaling-law sections are actionable for the system's existing LoRA training pipelines (`quest_grammar/grammar_training.rs`, `quest_grammar/quest_training.rs` — QLoRA rank=16-32, alpha=32-64) that feed ternary draft models (`TernaryDraftModel`). Distilled in `riir-train/.research/414`.
>
> **PASS-Redirects (synthesis):** Tu, Feng, Wang, Xie, Yan [arXiv:2608.18027 "Chain-of-Experience for Continual LLM Improvement"] — external systematic study of the exact loop this note designed: (attempt, feedback) trace accumulation + history-conditioned retry across a four-type feedback spectrum (none / executor / model-judge / correctness). Its findings validate the shipped shape rather than extend it: full experience trails beat compressed memory (DC/SimpleMem underperform) confirms keeping full `TrajRowDoc` rows; gains saturate in the first ~20 iterations confirms bounded fixpoints (`--max-iters`); base ability ↔ improvement capacity (avg Pearson r=0.50) confirms the starved-pool doctrine (R083); correctness/executor feedback dominates model-judge feedback on accuracy confirms the verified-gated promotion (`EvidenceTier`, only `(success && verified)` promotes). Verdict PASS — every CoE component ships here or is inapplicable to deterministic verified proposers (SelMV answer aggregation presumes noisy sampled outputs; our candidate set is deterministic and compile-gated).
