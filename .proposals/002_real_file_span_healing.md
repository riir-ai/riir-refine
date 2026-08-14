# Proposal 002 — Real-file span-level healing for `rust_perf` + `kernel_opt` (the `quest_grammar` fusion)

**Status:** proposed (awaiting user go-ahead to execute via Plan 005)
**Date:** 2026-08-14
**Supersedes (partially):** Proposal 001 §"Product vision: `cargo heal` CLI (design sketch)" — this proposal makes the sketch concrete + scopes the first real milestone.
**Related:** [Plan 001](../.plans/001_multi_domain_framework_refactor.md) (framework DONE), [Plan 005](../.plans/005_real_file_span_healing.md) (execution).

## TL;DR

> **BREAKTHROUGH ADDENDUM (2026-08-14, post user pushback):** the original TL;DR below
> framed a two-layer matcher (hand-coded visitors + ternary-matvec suggester) + kept
> cross-domain routing out of scope. **Grep revealed both are unnecessary limitations.**
> The substrate for a unified, clippy-optional, latent-retrieval architecture already
> ships across 3 crates:
>
> 1. **`AstChunker`** (riir-rag) — chunks Rust files at semantic boundaries (fn/impl/struct)
>    WITH line ranges. "Put AST in riir-rag" = already implemented.
> 2. **`RetrievalProvider::retrieve(LatentQuery, budget)`** (riir-rag) — Clifford-wedge KNN
>    over latent embeddings. This IS the rule-matcher: embed the code span, query the
>    corpus index, retrieve the matching rule. Works UNIFORMLY for all 3 domains.
> 3. **`pick_domain<const N, const A>`** (katgpt-core) — generic MoE router. Currently N=15
>    for kernel categories; **N=3 for cross-domain routing is a one-line instantiation.**
>
> The revised architecture collapses the two-layer matcher into ONE latent retrieval
> path, promotes cross-domain routing from "out of scope" to "ships", and makes clippy
> **optional verification** (not the matcher). The asymmetry the original TL;DR flagged
> (clippy has a matcher, perf/kernel don't) disappears — all three domains use the same
> latent retrieval path. **This is the dMoE-for-multi-domain + AST-in-riir-rag fusion the
> user identified.**
>
> The revised pipeline:
> ```
> real .rs file
>   → AstChunker.chunk()                      [riir-rag: EXISTS]
>   → ModellessEmbedder.embed() → [f32;8]      [riir-rag: EXISTS, modelless]
>   → pick_domain<3,8>(activity, dirs)           [katgpt-core: EXISTS, N=3 trivial]
>       → domain 0/1/2 (clippy/perf/kernel)
>   → RetrievalProvider.retrieve(query)          [riir-rag: EXISTS, Clifford wedge KNN]
>       → ranked rules from THAT domain's corpus
>   → proposer.rewrite() (if one exists)         [clippy_lints: EXISTS; perf/kernel: Plan 005]
>   → RustAnalyzerPruner                         [riir-clippy: EXISTS]
>   → optional: clippy_verify / cargo test / cubecl compile
> ```
>
> The original TL;DR + design sections below are retained for lineage; the addendum
> at §"Revised architecture (dMoE + AST-in-riir-rag)" is the authoritative design.
>
> **Original TL;DR (superseded by the addendum on key points):**

The three heal domains today have **asymmetric real-file capability**:

| Domain | Corpus | Drafter | Pruner | **Real-file rewrite (proposer)** | Matcher (what tells you the rule applies?) |
|---|---|---|---|---|---|
| `clippy_lints` | 10 lints | ternary matvec | `RustAnalyzerPruner` | ✅ **10 `syn` AST codemods** (Stage 2 proposers) | ✅ clippy emits the lint_name |
| `rust_perf` | 46 rules | ternary matvec | `RustAnalyzerPruner` | ❌ **pseudocode template match only** | ❌ none |
| `kernel_opt` | 123 rules | ternary matvec + MoE router | `RustAnalyzerPruner` | ❌ **pseudocode template match only** | ❌ none |

The asymmetry is the gap. `clippy_lints` already proves the pattern: a per-rule `syn` visitor (`fn rewrite(file: &mut syn::File) -> bool`) detects the buggy pattern in real code + rewrites it in place. The drafter picks the rule; the proposer applies it; the pruner validates. This is **`quest_grammar` in the Rust domain** — drafter + pruner + corpus, but the corpus entries are real `syn` rewrites, not string templates.

**This proposal lifts that proposer pattern into `rust_perf` + `kernel_opt`**, adds a **shallow-reasoning matcher** (so perf/kernel can detect their own patterns — clippy isn't there to hand them a rule_id), and ships a **unified `cargo heal` CLI** that scans real `.rs` files + applies all three domains.

The result: `cargo heal --fix` gives you clippy lints + perf wins + kernel-launch/dispatch fixes on **your actual code**, modellessly. Optional model-based (L4 LoRA) stays deferred.

## The problem this solves

### Problem 1: `kernel_opt`/`rust_perf` heal pseudocode, not your code

Right now, `kernel_heal_demo` + `perf_heal_demo` draft a fix from a corpus *template* (`ref_shape` → `opt_pattern`) and validate it *parses*. They do **not** read `riir-gpu/src/gemv_q4k_batched_rmsnorm_cubecl.rs` and say "this launch has 8 redundant planes — here's the 1-plane rewrite." The 123 kernel rules + 46 perf rules are a knowledge base that nothing applies to real files. That's the gap.

### Problem 2: clippy is already ruliology — our value-add must be "clippy + shallow reasoning"

`cargo clippy --fix` already handles the common case (it IS a rule-based healer). Our `clippy_lints` domain adds value in the tail: the Stage 2 proposers do AST rewrites clippy can't (they learn from fixtures + emit caller-idiomatic fixes). The same asymmetry should hold for perf + kernel: **there's no `cargo perf --fix` and no `cargo kernel --fix`** — that entire space is ours. The shallow-reasoning matcher (ternary matvec + `syn` visitors) IS the product for those two domains.

### Problem 3: the unified CLI is a sketch, not a plan

Proposal 001 §"Product vision" describes `cargo heal` as a design sketch + flags span-level patching as the "Key technical challenge." It's actually not that hard — `syn` parses the file, the proposer rewrites in place, you write back. The real work is the **proposer coverage** (writing the visitors + rewrites) + the **matcher** (detecting which rule applies without an external diagnostic). This proposal scopes both.

## Revised architecture (dMoE + AST-in-riir-rag) — the authoritative design

> This section supersedes the original "proposed design" sections (Layer 0-3) on
> three points: (1) the matcher is latent retrieval, not hand-coded visitors;
> (2) cross-domain routing via `pick_domain<3,8>` is IN SCOPE; (3) clippy is optional
> verification, not the matcher. The proposer pattern (Layer 0) still stands — it's
> the rewrite mechanism, orthogonal to the matcher.

### The unified latent-space heal path (one pipeline, all domains)

The breakthrough: **all three domains share ONE retrieval path.** The asymmetry
(clippy has clippy-as-matcher; perf/kernel have nothing) collapses because the
matcher becomes riir-rag's latent retrieval, which is domain-agnostic.

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. CHUNK    AstChunker.chunk(file) → Vec<RawChunk> [line ranges]    │
│             [riir-rag ast_chunker.rs — EXISTS]                      │
├─────────────────────────────────────────────────────────────────────┤
│ 2. EMBED    ModellessEmbedder.embed(chunk) → [f32; 8]               │
│             [riir-rag embedder.rs — EXISTS, BLAKE3+DFT+sigmoid]     │
│             (opt-in: BonsaiEmbedder 768-D for higher accuracy)      │
├─────────────────────────────────────────────────────────────────────┤
│ 3. ROUTE    pick_domain<3, 8>(activity, domain_directions) → 0|1|2  │
│             [katgpt-core variable_rank_domain_expert.rs — EXISTS]   │
│             0=clippy_lints, 1=rust_perf, 2=kernel_opt               │
│             (the dMoE — domain-level Mixture of Experts)            │
├─────────────────────────────────────────────────────────────────────┤
│ 4. RETRIEVE RetrievalProvider.retrieve(LatentQuery, budget)         │
│             → ranked rules from the routed domain's corpus          │
│             [riir-rag query.rs — EXISTS, Clifford-wedge KNN]        │
│             The corpus rules (ref_shape text) are indexed chunks.   │
├─────────────────────────────────────────────────────────────────────┤
│ 5. REWRITE  proposer.rewrite(file) for the top rule (if one exists) │
│             [clippy_lints proposers — EXISTS; perf/kernel: Plan 005]│
├─────────────────────────────────────────────────────────────────────┤
│ 6. VALIDATE RustAnalyzerPruner (parse check — always)               │
│             [riir-clippy pruner.rs — EXISTS]                        │
│             optional: clippy_verify / cargo test / cubecl compile   │
└─────────────────────────────────────────────────────────────────────┘
```

### Why this is better than the original two-layer matcher

| Dimension | Original (Layer 1a/1b) | Revised (latent retrieval) |
|---|---|---|
| Matcher for perf/kernel | hand-coded visitors + ternary suggester | **riir-rag retrieve (Clifford wedge KNN)** — one path |
| Cross-domain routing | out of scope (`--domain` flag) | **`pick_domain<3,8>`** — automatic, MoE-routed |
| clippy dependency | clippy emits lint_name (required for matching) | **optional** — clippy becomes verify-only |
| New substrate needed | visitors per rule (~64 to write) | **zero** — riir-rag + katgpt-core already ship it |
| Proposer coverage | the bottleneck (must write 64 proposers) | **decoupled** — retrieval works WITHOUT proposers (`--suggest` mode surfaces the rule + opt_pattern for manual review); proposers are the `--fix` upgrade |

The proposers are still valuable (they enable `--fix` auto-apply), but they're
no longer on the critical path. **The retrieval-only product (`--suggest`) ships
first, with ZERO proposer code**, because the corpus ref_shapes are already
indexed-able chunks. This is a massive scope reduction for Plan 005 Phase 1.

### dMoE — the domain-level Mixture of Experts

`pick_domain<const N, const A>` is already generic over N. The kernel_opt
router uses `N=15` (15 categories). Cross-domain routing is `N=3`:

```rust
// The 3 domain-direction vectors (8-D, hand-crafted initially — mined later):
//   clippy:  high on style/complexity signals (short fns, closures, returns)
//   perf:    high on hot-path signals (loops, allocations, HashMap)
//   kernel:  high on GPU signals (CubeDim, CubeCount, #[cube], launch)
const DOMAIN_DIRECTIONS: [[f32; 8]; 3] = [
    /* clippy  */ [ ... ],  // mined from clippy lint signatures
    /* perf    */ [ ... ],  // mined from perf anti-pattern signatures
    /* kernel  */ [ ... ],  // mined from kernel launch signatures
];
let domain = pick_domain::<3, 8>(&activity, &DOMAIN_DIRECTIONS);
```

The activity vector is the code chunk's 8-D embedding (step 2). The domain
selection is a dot-product argmax — modelless, zero-allocation, O(3×8)=24
FLOPs. This IS the "dMoE technique for multi-domain" the user identified: the
MoE router that was within-kernel-only is instantiated at the domain level.

### AST in riir-rag — the fast + accurate retrieval

The corpus rules become indexed chunks:
- `ref_shape` (the buggy pattern pseudocode) → embedded via ModellessEmbedder
- The embedding → indexed in `ShardIndex` (Clifford wedge KNN) or `DenseEmbedIndex`
- At heal time: the real code chunk is embedded → `LatentQuery::from_direction()`
  → `retrieve()` → ranked rules by latent similarity

This is faster + more accurate than waiting for clippy because:
- **Faster:** Clifford-wedge KNN is O(log N) lookup (Bench 575: 123µs p50 @ N=1000);
  clippy is ~130ms (1000× slower).
- **More accurate:** latent similarity catches semantic equivalents (a `Vec::push`
  in a `loop` matches the `vec-with-capacity` rule even if the variable names differ);
  clippy only catches its hardcoded lint patterns.
- **Uniform:** works for ALL 3 domains; no per-domain matcher code.

### clippy becomes optional verification

The `clippy_verify` feature (already shipped) re-runs `cargo clippy` to confirm a
proposed fix resolves the lint. In the revised architecture, this is the ONLY role
clippy plays — verification, not matching. A user can run `cargo heal --fix` without
clippy installed (the latent matcher works standalone); clippy is the belt-and-suspenders
confirm step. This also means the `clippy_lints` domain can use the SAME latent
retrieval path as perf/kernel — no special-casing.

## Fusion layer — MAG self-healing + WASM pruner + KVarN-normalized retrieval

Four MORE substrates (beyond the 3 in the TL;DR addendum) compose onto the pipeline.
All confirmed to exist via grep:

### MAG unsupervised direction mining (Research 316) → mines the dMoE directions

The `DOMAIN_DIRECTIONS` for `pick_domain<3,8>` start hand-crafted. **They don't have
to stay that way.** riir-clippy already ships `LatentSkillEvolution` (Issue 004/008,
`src/evolve.rs`):

```rust
// record_outcome: EMA update of the direction vector per-key
// success: dir += α · (fix_dir - dir)   (move toward)
// fail:    dir -= β · (fix_dir - dir)   (move away)
fn record_outcome(&mut self, key: &str, fix_direction: &Direction, success: bool)
```

This IS the modelless MAG: unsupervised direction mining from fix outcomes. Currently
keyed per-lint; the fusion generalizes the key to per-DOMAIN. Each `cargo heal --fix`
cycle records (domain, fix_direction, success) → the dMoE `DOMAIN_DIRECTIONS` evolve.
The router gets smarter with use — no hand-crafted centroids, no offline mining job.

**This closes the "centroid mining deferred" gap from the kernel_opt router.** The
`CommittedFieldBlend` (Plan 321) was the offline path; `LatentSkillEvolution` is the
online modelless path that ships TODAY. MAG's 8 operators (Research 316 §"8 MAG
operators") are the richer offline variant for when the online EMA plateaus.

### Self-healing closed loop (Issue 004/008) → the pipeline composes

`HealPipeline` (already shipped, G1-G7 PASS) already composes:
1. heal(lint, code) → proposes a fix
2. validate (clippy_verify / cargo test) → outcome (success/fail)
3. `LatentSkillEvolution.record_outcome()` → evolve the direction
4. `LatentFixMemory.insert()` → store the trajectory for 3-axis retrieval
5. NEXT heal query → `LatentFixMemory.retrieve()` finds similar past fixes → better proposal

In the revised architecture, this loop runs at the DOMAIN level (not per-lint). The
`--fix` mode records (domain, code_span, fix, outcome); the `--suggest` mode retrieves
past trajectories. The healer gets better with every fix it applies. **This is
self-healing** — the system improves its own retrieval + routing from its own outcomes.

### WASM pruner (Proposal 011) → fast `--fix` validation

The current validators are slow: `clippy_verify` (~130ms), `rust_perf_verify` (cargo
test, seconds), `kernel_verify` (cubecl compile, deferred). Proposal 011's WasmPruner
compiles the test suite to WASM + runs it as an in-process `ConstraintPruner` —
millisecond-scale validation, no subprocess spawn.

`ConstraintPruner` (katgpt-core `traits.rs`) is the trait:
```rust
pub trait ConstraintPruner {
    fn is_valid(&self, depth: usize, token_idx: usize, parent_tokens: &[usize]) -> bool;
}
```

The WasmPruner impl interprets a proposed fix as a token sequence + validates it against
the WASM-compiled test suite's pass/fail. This is the fast path for `--fix`'s "does the
fix break anything" check. The trait already exists; the WasmPruner impl is the new work
(gated on `rubrc` maturity per Proposal 011 Phase 3). Until then, `cargo test` is the
slow fallback.

### KVarN variance normalization (Research 049) → accurate Clifford-wedge retrieval

The Clifford-wedge KNN in riir-rag's `NeuronDbRetriever` works on L2-normalized 8-D
latent vectors. The `FixedProjection` (riir-rag `bonsai_embedder/dense_projection.rs`)
already uses variance=1/n Gaussian (the JL construction) + `normalize_l2`. KVarN's
Sinkhorn variance normalization is the refinement: per-dimension variance equalization
before the L2 norm, so high-variance dimensions don't dominate the dot product. This is
an embedding-quality upgrade, not a new substrate — it composes into `ModellessEmbedder`
or `BonsaiEmbedder` as a pre-normalize step. Defered until retrieval precision (G1) needs
it; the L2-normalized baseline ships first.

### The full 7-substrate fusion (the complete product architecture)

```
real .rs file
  1. CHUNK     AstChunker.chunk()                         [riir-rag]
  2. EMBED     ModellessEmbedder.embed() → [f32;8]         [riir-rag] (+ KVarN normalize, opt-in)
  3. ROUTE     pick_domain<3,8>(activity, DOMAIN_DIRS)      [katgpt-core]  (dMoE)
  4. RETRIEVE  RetrievalProvider.retrieve(LatentQuery)       [riir-rag]    (Clifford KNN)
     ↻ MEMORY  LatentFixMemory.retrieve(past trajectories)  [riir-clippy] (3-axis: latent+BM25+traj)
  5. REWRITE   proposer.rewrite(file) (if exists)           [clippy_lints; perf/kernel: Plan 005]
  6. VALIDATE  RustAnalyzerPruner (always)                  [riir-clippy]
              WasmPruner (fast, Proposal 011) OR cargo test (slow fallback)
  7. EVOLVE    LatentSkillEvolution.record_outcome()         [riir-clippy] (MAG online EMA → DOMAIN_DIRS)
```

Steps 4↻ (memory) + 7 (evolve) form the **self-healing closed loop**. The healer
improves its retrieval (memory) + its routing (evolved directions) from every fix
it applies. This is the MAG self-healing fusion — modelless, online, no training.


## Deep fusion layer — calibrated UQ + sleep-cycle consolidation + Poincaré imagination + ConvexTok AST + trajectory-archetype (2026-08-14)

The 7-substrate pipeline above ships `--suggest` + `--fix` with modelless
self-healing. A deeper substrate harvest (vocabulary-translated grep across
all 7 repos, subagent-parallel) surfaced **five more fusion layers** that the
healer does NOT yet consume but that ship in katgpt-core / katgpt-rs / riir-ai
TODAY. Each is verified to exist by direct grep (not subagent claim).

The unifying reframe: **the healer is a *system* not a *pipeline* — it has
perception (UQ), memory (consolidation), imagination (Poincaré roll-out),
language (AST vocabulary), and post-hoc shape classification (trajectory
archetype).** The 7-substrate version had no perception, no imagination, no
language, and no shape classification. The deep-fusion version does.

### Deep fusion A — calibrated UQ (the "which fix is correct" layer)

`DraftedFix.confidence` is currently `sigmoid(mean(matvec_output))` — a raw
scalar in (0,1). **This is uncalibrated** (the exact failure mode Bench 010
documents for BoMSampler: 5-15% empirical coverage vs nominal 95%). Three
DEFAULT-ON katgpt-core primitives close this:

1. **`ConformalIntervalCalibrator<F: PointForecaster>`** (`katgpt-core/src/conformal/mod.rs`)
   — distribution-free confidence intervals with empirical-coverage guarantees.
   Wrap the drafter's confidence: collect residuals
   `(predicted_correct, actual_correct)` per rule_id after each `--fix` cycle,
   calibrate the raw scalar into a true 95% interval on "is this fix correct".
   The standing floor (`ConformalIntervalCalibrator<SeasonalNaiveForecaster>`,
   Bench 010) is the regression test — the calibrator MUST beat it.
2. **`select_best_belief(candidates: &[(u32,u32)], ε, incumbent)`** (`best_belief.rs:284`)
   — Beta(1+S, 1+F) posterior ε-quantile selector. **Proven 15-30% selection-
   regret reduction vs MLE floor** (Bench 010 T5). Replace the linear blend
   `0.6·evolution + 0.4·success_rate` in `HealPipeline::heal` with this. Each
   candidate fix carries `(successes, failures)`; pick the Bayesian winner.
   **This directly answers "fix A vs fix B"** — the question that broke the
   original 7-substrate pipeline.
3. **`clr_vote<T, E, V>` + `ClrConfig`** (`katgpt-claim/src/clr/vote.rs:79`)
   — Claim-Level Reliability: sample K candidate fixes, cluster by outcome,
   M-verifier vote per cluster, pick the most-reliable cluster's representative.
   The nonlinear `(mean_m v_k,m)^5` gate sharpens cluster rankings vs majority
   vote (Bench 284 GOAT: 100% win rate vs 22% majority). Generate K=8 drafted
   fixes (currently 1), cluster by syntactic similarity, run the L2 pruner on
   each, vote.

**Why this is novel.** Existing code healers (clippy, ruff, eslint --fix) emit
ONE fix with no confidence. They cannot answer "how sure are you?". Calibrated
UQ on candidate fixes is the missing product surface — and the substrate ships
TODAY, fully floor-tested.

### Deep fusion B — sleep-cycle consolidation (the "overnight learning" layer)

`LatentFixMemory` is per-trajectory, in-memory only. The healer re-learns from
scratch each session. THREE substrates close this (all verified via grep):

1. **`mine_motifs_at_sleep_cycle(miner, admitter, dl_old_bits)`**
   (`katgpt-core/src/closure/mining.rs:101`) — offline motif mining at the
   sleep-cycle boundary. Mines recurring subgraphs (PTGs — primitive transition
   graphs) over recent trajectories, computes PRI (Primitive Reuse Index) per
   motif, admits high-PRI motifs into a frozen skill library. Returns
   `SleepCycleClosureReport { motifs, pri, ptg_count, admitted_count }`.
   Rayon-parallel, modelless, zero GD. **Direct drop-in for overnight
   consolidation of fix-pattern motifs.**
2. **`LocalKvStore`** (Warm tier, `riir-neuron-db/src/local_kv.rs`) — encrypted
   WAL-backed KV with BLAKE3-keyed XOF. `put_pod<T: Pod>`, `pod_view<T>`,
   `commit_value`/`verify_commitment` for tamper-evidence. **Persist fix
   outcomes overnight.** Key = `BLAKE3(lint_name ∥ rule_id ∥ buggy_hash)`,
   Value = `FixOutcome { fixed_code, verified, blake3_root }`. Survives restart.
3. **`ConsolidationPipeline::can_freeze` + `FreezeGateReport`**
   (`riir-neuron-db/src/consolidation.rs`) — the two-sided gate that decides
   "is this batch ready to consolidate into a frozen snapshot?" (n≥d samples
   AND flatness < τ). **Gate skill-library commits** — don't freeze a motif
   until enough trajectories observed AND the direction has converged.

The composed loop: each `--fix` records (rule, code, fix, outcome) → in-memory
`LatentFixMemory` → at sleep cycle, `mine_motifs_at_sleep_cycle` extracts
recurring fix-pattern motifs → `can_freeze` gates which motifs graduate →
admitted motifs persist via `LocalKvStore` + freeze/thaw envelope. The healer
literally sleeps, dreams about past fixes, and wakes up with new skills.

### Deep fusion C — Poincaré fix-imagination (the "simulate before apply" layer) — THE NOVEL ANGLE

**The single most novel fusion.** Existing healers apply a fix and check the
result. The deep-fusion healer *imagines the fix* in latent space *before*
touching the file, scores the imagined post-fix state, and only applies if the
score is good.

Two substrates compose:

1. **`fit_poincare_adapter` + `poincare_navigate_into` + `poincare_multi_step_into`**
   (`katgpt-core/src/poincare.rs:680`, `lib.rs:2256`) — given a fitted
   `PoincareAdapter { φ, W, W† }` (offline-fit via PCA + ridge regression,
   no GD), source latent state `z_src`, and source/target raw positions, writes
   the imagined destination latent `z_out = z_src + φ⁻¹(φ(z_src) + W†·Δtarget)`.
   Zero-allocation, single matvec + MLP eval. **Game-runtime wrapper exists**:
   `riir-engine/src/poincare_bridge/imagine.rs::imagine_belief_into`.
2. **`ViableManifoldGraph` + `manifold_curiosity_walk` + `manifold_geodesic`**
   (`katgpt-core/src/viable_manifold_graph.rs`, DEFAULT-ON, G1-G7 PASS,
   7.10 ns/step) — builds a discrete safe-subgraph from samples where each node
   + edge satisfies a viability predicate + Jacobian pullback volume threshold.
   **The "safe fix neighborhood" graph** — fixes never leave the viable set
   (compile-passing code states).

**The translation table** (NPC HLA state → code healer):

| Game concept | Healer concept |
|---|---|
| NPC HLA state `z_src` | Current code-state embedding (8-dim, from ModellessEmbedder) |
| Source position `src_pos` | Current file hash / context signature |
| Target position `dst_pos` | Proposed fix hash / target signature |
| Imagined HLA `z_out` | Predicted post-fix code-state embedding |
| `compute_animal_emotions(z_out)` | `score_fix_goodness(z_out)` (dot + sigmoid onto a learned direction) |
| Anti-cheat invariant (never substitute for raw position) | **Never substitute imagined fix for applied fix** — imagined fixes never get committed to git |

The healer's loop becomes: chunk → embed → route → retrieve rule → draft fix →
**IMAGINE the fix in Poincaré space → score → if score < τ, reject the fix and
try the next retrieved rule** → if score ≥ τ, apply + validate with the L2
pruner. **The imagination gate prevents wasted L2 calls** (the L2 pruner is
the slow step; imagination is µs-scale).

**Honest caveat (must be in any plan that consumes this):** Plan 497 Phase 2
REFUTED on quality axis for NPC HLA (linear model can't fit nonlinear emotion
evolution). **The latency win stands (102× speedup)** and for code healing the
nonlinearity bar is LOWER — fix locality is closer to linear than NPC emotion
curves. But this is an honest risk, not a guaranteed win. The fusion ships
behind an opt-in feature gate; quality is benchmarked against a "no-imagination"
baseline (the L2 pruner alone) before any promotion.

### Deep fusion D — ConvexTok LP-graph for AST nodes (the "globally optimal vocabulary" layer) — THE SUPER-GOAT

**The genuinely novel fusion. No code-domain analog exists** (grep for
`convex.*code|ast.*convex|abstract_syntax.*lp` returns zero hits across all
7 repos).

The substrate ships in katgpt-tokenizer (opt-in `convex_tok`):
- `GraphBuilder::build(&[Vec<u8>], max_len)` (`convex_graph.rs`) — coloured
  DAG from byte-corpus; vertices = positions, free edges = single bytes,
  priced edges = candidate multi-byte tokens coloured by substring.
- `ConvexSolver::solve(graph, budget_k)` (`convex_solver.rs:24`) — LP via
  `good_lp`/HiGHS, picks fractional token selection maximizing compression.
- `Rounder::{det, bias, int}` (`convex_rounding.rs`) — LP → integral vocab.
- `Certifier::certify(sol, rounded)` (`convex_certify.rs`) — optimality gap.

12/12 GOAT PASS (test_127_convex_tok_goat.rs).

**The fusion:** feed `GraphBuilder::build()` AST-node-kind byte sequences
instead of literal bytes. Each lint-fix corpus entry becomes a sequence of
AST node kinds (via `syn` visitor: `[ExprMatch, Arm, Pat, Path, ...]`). The LP
then picks a vocabulary of K "AST multi-node tokens" that maximize fix-
template compression. **This gives the healer a globally-optimal AST vocabulary**
— far more structured than the current 1-fix-per-rule corpus.

Why this matters: today's `ClippyCorpus` is a flat list of (lint_name, ref_shape,
opt_pattern, fix). The vocabulary is implicit (one entry per rule). ConvexTok-
AST extracts the **recurring AST sub-patterns across rules** — e.g., a
`swap-remove` motif (3 AST nodes) might appear in 5 different perf rules.
With a learned K=64 AST vocabulary, the drafter composes fixes from these
reusable atoms rather than memorizing per-rule templates. **This is the
language-model analog for the healer — its own token vocabulary, learned
from its own corpus.**

**Honest research question (NOT a quick fusion):** ConvexTok's LP is over a
coloured DAG where colours = byte-substrings. AST nodes are not bytes — the
encoding design (what is a "colour" for an AST node? node-kind? kind+arity?
kind+child-shape?) is the research question, not the LP. File as
`.research/NNN_convex_ast_vocab.md` first; the LP infrastructure is 100% reused.

### Deep fusion E — SweTrajectoryFreezer (the "fix-attempt shape classifier" layer)

`SweTrajectoryFreezer<N, D>` (`katgpt-core/src/swe_trajectory_freeze/`, Bench
013 GOAT G1-G4 PASS, Bench 014 G5 cross-model PASS) freezes a fix-generation
trajectory into a D-dim geometry summary, classifies into N archetypes
(`converged_correct`, `committed_wrong`, `oscillation`), `gates()` returns
per-archetype match scores.

**Fusion:** each fix attempt IS a trajectory (draft → prune → apply → verify).
Freeze it, classify the archetype:
- `converged_correct` → high confidence, ship
- `committed_wrong` → low confidence, escalate to human review or stronger pruner
- `oscillation` → the drafter is flip-flopping, abort + log for self_evolve

This is a **trajectory-shape confidence signal**, complementary to the
residual-based signal from `ConformalIntervalCalibrator` (Deep fusion A).

### The revised 12-substrate fusion (the SUPER-GOAT product architecture)

```
real .rs file
   1. CHUNK     AstChunker.chunk()                                [riir-rag]
   2. EMBED     ModellessEmbedder.embed() → [f32;8]              [riir-rag] (+ KVarN normalize, opt-in)
   3. ROUTE     pick_domain<3,8>(activity, DOMAIN_DIRS)           [katgpt-core]  (dMoE)
   4. RETRIEVE  RetrievalProvider.retrieve(LatentQuery)            [riir-rag]    (Clifford KNN)
      ↻ MEMORY  LatentFixMemory.retrieve(past trajectories)       [riir-clippy] (3-axis)
   5. REWRITE   proposer.rewrite(file) (if exists)                [clippy_lints; perf/kernel: Plan 005]
   6. IMAGINE   poincare_navigate_into(z_src → z_out)             [katgpt-core] (Poincaré) — NEW
      SCORE     dot(z_out, goodness_dir) + sigmoid; if < τ → next rule
      VIABLE    manifold_geodesic(z_now, z_target) feasibility     [katgpt-core] (ViableManifoldGraph) — NEW
   7. VALIDATE  RustAnalyzerPruner (always)                       [riir-clippy]
               WasmPruner (fast, Proposal 011) OR cargo test (slow fallback)
   8. SELECT    select_best_belief(candidates, ε)                  [katgpt-core] (Beta posterior) — NEW
               clr_vote(K candidates, M verifiers)                 [katgpt-claim] (Claim-Level Reliability) — NEW
   9. CALIBRATE ConformalIntervalCalibrator wraps DraftedFix.confidence [katgpt-core] — NEW
      FREEZE    SweTrajectoryFreezer.classify(attempt → archetype) [katgpt-core] — NEW
  10. EVOLVE    LatentSkillEvolution.record_outcome()               [riir-clippy] (MAG online EMA → DOMAIN_DIRS)
  11. SLEEP     mine_motifs_at_sleep_cycle(miner) — overnight batch [katgpt-core] — NEW
      FREEZE    ConsolidationPipeline::can_freeze gate              [riir-neuron-db] — NEW
      PERSIST   LocalKvStore.put_pod(FixOutcome) — WAL-committed    [riir-neuron-db] — NEW
  12. VOCAB     ConvexTok AST vocabulary (offline, optional)         [katgpt-tokenizer] — NEW (research-gated)
```

**This is no longer "clippy with extra rules."** It is a self-imagining,
calibrated, sleep-consolidating code healer with its own learned AST vocabulary.

### What's NEW vs the 7-substrate version

- **Calibrated confidence (steps 8, 9):** the healer now answers "how sure am
  I?" with a 95% coverage guarantee, not a raw sigmoid. Beta-posterior selection
  + CLR voting replace the linear blend.
- **Imagination before apply (step 6):** the healer simulates the fix in
  Poincaré space + checks the safe-manifold graph BEFORE the expensive L2
  validate. Wasted L2 calls drop, throughput rises.
- **Sleep-cycle consolidation (step 11):** the healer mines motifs from past
  fixes overnight + freezes them into a versioned skill library (BLAKE3-
  committed via LocalKvStore).
- **Trajectory-archetype classification (step 9 FREEZE):** each fix attempt is
  classified (converged_correct / committed_wrong / oscillation) — a shape-based
  confidence signal complementary to the residual-based conformal one.
- **Learned AST vocabulary (step 12):** ConvexTok LP over AST nodes gives a
  globally-optimal multi-node token vocabulary. Research-gated — the encoding
  design is the open question.

### Honest caveats (deep fusion layer, 2026-08-14)

1. **Poincaré imagination REFUTED on quality axis for NPC HLA** (Plan 497 Phase
   2). The latency win stands. For code healing the nonlinearity bar is lower
   but this is an honest risk — ship behind opt-in feature gate, benchmark
   against no-imagination baseline before any promotion.
2. **ConvexTok-AST encoding is a research question**, not a fusion. File
   `.research/NNN_convex_ast_vocab.md` before any plan. The LP infrastructure
   is 100% reused; the colouring scheme is the open question.
3. **`select_best_belief` proven on Bench 010 T5** (heteroscedastic regime).
   The lint-fix domain may not be heteroscedastic enough to see the 15-30%
   regret reduction. Honest benchmark required.
4. **`mine_motifs_at_sleep_cycle` needs PTG-shaped trajectories.** The healer's
   current `FixTrajectory` is not PTG-shaped (it's a flat (rule, code, fix)
   record). Generalize the trajectory type before mining.
5. **Three substrates from the harvest remain UNVERIFIED** — `HebbianKernelMemory`,
   `ExperienceReuseHook`, `DeltaMemoryState`. Subagents claimed them but my direct
   grep did not confirm. File `.issues/` to verify before any plan consumes them.
   **UPDATE (2026-08-14, same session): all three VERIFIED via direct grep.**
   `HebbianKernelMemory` in katgpt-core Plan 559 (DEFAULT-ON, G1-G5 PASS),
   `DeltaMemoryState` in katgpt-core Plan 408 (DEFAULT-ON), `ExperienceReuseHook`
   in riir-engine arg_runtime (dual-stream TD learning). Plan 005 Phase 14 wires
   them as additive fusions on top of Phases 9-12.
6. **Feature-gate discipline holds.** Every deep-fusion layer ships behind an
   opt-in feature (`poincare_imagination`, `sleep_consolidation`, `convex_ast_vocab`,
   etc.). Promotion to default requires the GOAT gate to PASS, per global rule.


### Honest caveats (revised)

1. **CubeCL `#[cube]` macro bodies** — still out of scope for proposers (syn can't parse
   inside). BUT: the latent matcher STILL works on them (AstChunker falls back to
   whole-file chunk on parse failure; the embedding still captures the signature).
   So `--suggest` works for kernel-body rules; only `--fix` (proposer) is limited.
   This is better than the original caveat — the retrieval path covers more ground.
2. **Latent false-matches** — the Clifford-wedge KNN will sometimes retrieve the wrong
   rule. Mitigation: `--suggest` mode surfaces the top-K rules with scores for human
   review; `--fix` requires EITHER a confirmed proposer match OR a high-confidence
   (>0.8) latent score AND a proposer. The safety contract: never auto-apply a
   low-confidence latent match.
3. **Domain-direction mining** — the initial `DOMAIN_DIRECTIONS` are hand-crafted (like
   kernel_opt's category centroids). Mining via `CommittedFieldBlend` (Plan 321) is
   the same deferred path; the hand-crafted vectors are the modelless baseline.
4. **Proposer coverage still voluminous** for `--fix` — but decoupled from the critical
   path. `--suggest` ships first (zero proposers); proposers expand `--fix` coverage
   incrementally.
5. **Cross-domain auto-routing is NOW IN SCOPE** (was caveat 5 in the original). The
   `pick_domain<3,8>` substrate makes it trivial. The CLI defaults to auto-route; `--domain`
   overrides for users who want to force a specific domain.

## The proposed design (ORIGINAL — partially superseded by the revised architecture above)


### Layer 0 — the proposer pattern (generalize from `clippy_lints`)

Each domain gets a `proposers/` module mirroring `clippy_lints/proposers/`:

```rust
// src/domains/rust_perf/proposers/mod.rs
pub fn propose_fix(rule_id: &str, code: &str) -> Option<String> {
    let mut file: syn::File = syn::parse_file(code).ok()?;
    let changed = match rule_id {
        "vec-with-capacity" => vec_with_capacity::rewrite(&mut file),
        "entry-vs-contains-insert" => entry_vs_contains::rewrite(&mut file),
        // ... one arm per Pareto-dominant rule (Bench 013: 34/38)
        _ => false,
    };
    if !changed { return None; }
    Some(quote_file(&file))
}
```

Each proposer is ~50-100 LOC: a `syn::visit::Visit` detector + an in-place rewrite. Same shape as `needless_return.rs`. **No trait change needed** — the `propose_fix` free function mirrors `clippy_lints` exactly; the `HealDomain` trait stays provisional (Proposal 001 caveat 1).

### Layer 1 — the shallow-reasoning matcher (the NEW piece)

For `clippy_lints`, clippy hands you the `lint_name`. For perf/kernel, **nobody hands you the rule_id** — you have to find the pattern yourself. Two complementary mechanisms:

**1a. Hard-coded `syn` visitors (precise, high-frequency rules).**
Each proposer's detector IS a visitor. For the top-N Pareto-dominant rules (Bench 013 winners), the visitor walks the AST looking for the specific anti-pattern. Examples:
- `vec-with-capacity`: find `Vec::push` in a `for`/`while` loop where the vec was built with `vec![]` / `Vec::new()` (no `with_capacity`).
- `entry-vs-contains-insert`: find `if map.contains_key(k) { map.insert(k, v) } else { ... }` → rewrite to `map.entry(k)`.
- `single-plane-sufficient-reduction` (kernel): find `CubeDim::new_1d(N)` where N > PLANE_DIM and the body is a single `plane_sum` → flag the redundant-plane pattern.

These are precise (no false positives) but require one visitor per rule.

**1b. Ternary-matvec shallow-reasoning suggester (the "clippy + shallow reasoning" tail).**
For rules WITHOUT a dedicated visitor, the `TernaryDrafter` (already shipped, generic over `E: HealEntry`) embeds the candidate code span + matches it against the corpus `ref_shape`s. The closest match (above a confidence threshold) SUGGESTS the rule. If a proposer exists for that rule, it applies; if not, the suggestion is surfaced for manual review (the CLI's `--suggest` mode). This is the **shallow reasoning**: latent-space nearest-neighbor over a BLAKE3-embedded code span, not deep semantic analysis. It's exactly `quest_grammar`'s drafter mechanism, applied to Rust spans.

The two compose: visitors handle the confident cases; the ternary matvec handles the long tail. This is the "clippy + shallow reasoning" positioning from Proposal 001 §"Shallow reasoning is the honest positioning".

### Layer 2 — the MoE router (kernel_opt only, already shipped)

`KernelExpertRouter::route(activity) -> expert_index` (Plan 003/004, `pick_domain<15, 8>`) narrows 123 rules → ~8 category candidates before the proposer runs. This composes with the matcher: the router picks the CATEGORY (memory/fusion/dispatch/...), then the visitor/ternary matvec picks the RULE within the category. 15× narrowing already measured.

### Layer 3 — the unified `cargo heal` CLI

```
cargo heal [--fix] [--domain clippy|perf|kernel|all] [--suggest] [path...]
```

- Scans `.rs` files under `path` (default: current crate `src/`).
- For each file, runs the enabled domains' matchers (Layer 1).
- For each match, applies the proposer rewrite (Layer 0).
- Validates the result parses (`RustAnalyzerPruner`).
- `--fix`: writes the changes in place. Without `--fix`: prints a diff (review mode).
- `--suggest`: for rules without a proposer, surfaces the ternary-matvec suggestion + the corpus `opt_pattern` for manual review.
- `--domain`: selects which domain(s) to run. Default `all` (all enabled feature-gated domains). **Cross-domain auto-routing stays out of scope** — the user picks the domain(s), the CLI doesn't infer "this is a perf issue not a clippy issue." (That inference is a future enhancement, not this proposal.)

This is the `cargo riir-clippy --fix` end state the user asked about — scoped, honest, modelless.

## Fusion lineage (the novelty)

This proposal is a **composition of four existing substrates**, not a new invention:

1. **`quest_grammar`** (riir-games-quest) — drafter + pruner + corpus pattern. The `TernaryDraftModel` + `SealQuestCorpus` + `ConstraintPruner` shape. Borrowed into clippy_lints already; this extends it to perf + kernel.
2. **`clippy_lints` Stage 2 proposers** (riir-clippy) — real `syn` AST rewrites (`fn rewrite(&mut syn::File) -> bool`). The proof that the pattern works on real Rust code.
3. **`katgpt-core` ternary matvec** — the shallow-reasoning embedder. `TernaryDrafter` is already generic over `E: HealEntry`; the matcher reuses it on code spans.
4. **`kernel_opt` MoE router** — `pick_domain` (Plan 558) category narrowing. Already shipped; composes with the matcher.

No new substrate invented. The novelty is the **composition** (real-file healing for perf + kernel, grounded in latent-space matching) + the **honest scope** (CubeCL macro bodies are out of scope; launch/dispatch/layout Rust code is in scope).

## Honest caveats — READ BEFORE IMPLEMENTING

1. **CubeCL `#[cube]` macro bodies are OUT OF SCOPE for kernel proposers.** `syn` cannot parse inside the `#[cube]` proc macro. Kernel proposers operate on the Rust code OUTSIDE the macro: launch params (`CubeDim::new_1d(...)`), dispatch (`CubeCount::Static(...)`), buffer setup, the `launch` function. Many of the 123 rules ARE about this outer Rust (dispatch, layout, autotune, reduction-split, single-plane, CPU-offload) — but the kernel-body rules (flash-attention tiling, linear-attn recurrence) are not proposable on real files until we have a CubeCL-aware parser. **This is a hard scope limit, not a deferral.** Documented per-proposer.

2. **Proposer coverage is voluminous.** 34 Pareto-dominant perf rules (Bench 013) + ~30 kernel-launch/dispatch rules = ~64 proposers. Each is ~50-100 LOC. Plan 005 phases this: top-5 per domain first (prove the pattern), then expand. Full coverage is multi-week.

3. **`quote_file` is NOT formatting-preserving.** The existing `quote_file` uses `ToTokens` (token-stream → string), which reformats. A `--fix` CLI that reformats the user's whole file is hostile. Plan 005 T1.3 adds `prettyplease` OR a span-preserving patch emitter (only rewrite the matched span, leave the rest byte-identical). The latter is the honest production choice; the former is the fast MVP.

4. **The ternary-matvec suggester can false-match.** Latent-space nearest-neighbor over code embeddings will sometimes suggest the wrong rule. The `--suggest` mode makes this explicit (human reviews); `--fix` only applies rules with a CONFIRMED visitor match (not ternary suggestions). This is the safety contract: visitors are precise; the matvec is advisory.

5. **Cross-domain auto-routing stays out of scope.** The CLI takes `--domain`. It does not infer domain from code shape. Revisit only with a concrete consumer + a trained classifier (that's L4 territory).

## What ships (Plan 005) vs deferred

### Ships (Plan 005 scope)

**Original 7-substrate pipeline (Phases 1-7):**
- Phase 1-3: riir-rag latent retrieval (the `--suggest` MVP, ZERO proposers).
- Phase 2: `cargo heal --suggest` CLI.
- Phase 3: GOAT gate for `--suggest` mode (G1-G4).
- Phase 4-5: top-5 perf + top-5 kernel proposers (the `--fix` upgrade).
- Phase 6: `cargo heal --fix` + GOAT G5 (real-file value).
- Phase 7: self-healing closed loop (MAG online EMA → DOMAIN_DIRECTIONS).

**Deep fusion layers (Phases 9-14, all substrate verified via direct grep 2026-08-14):**
- Phase 9: calibrated UQ (`ConformalIntervalCalibrator` + `select_best_belief` + `clr_vote`).
- Phase 10: sleep-cycle consolidation (`mine_motifs_at_sleep_cycle` + `LocalKvStore` + `can_freeze`).
- Phase 11: Poincaré fix-imagination (`poincare_navigate_into` + `ViableManifoldGraph`).
- Phase 12: trajectory-archetype classifier (`SweTrajectoryFreezer`).
- Phase 13: ConvexTok AST vocabulary (**RESEARCH ONLY** — encoding design first).
- Phase 14: verified substrates (`DeltaMemoryState` + `HebbianKernelMemory` + `ExperienceReuseHook`).

### Deferred (separate plans)
- WASM pruner (Plan 005 Phase 8) — blocked on `rubrc` maturity (Proposal 011).
- ConvexTok-AST implementation (Phase 15+) — opens only if Phase 13 research shows ≥2× compression.
- Full proposer coverage (all 34 perf + ~30 kernel rules) — Plan 005 proves the pattern with 10; expansion is mechanical follow-up.
- CubeCL macro-body proposers — blocked on a CubeCL-aware parser (research, not impl).
- L4 LoRA drafter weights — multi-step path via quest_grammar training + distill_attention.

## GOAT gate (Plan 005 §Phase 6)

- **G1 (correctness):** each proposer round-trips on a fixture file — detects the pattern, rewrites, output parses, `cargo test` passes (semantics preserved).
- **G2 (perf):** matcher + proposer latency < 10ms per file (syn parse + visitor walk + rewrite). Measured on a 1000-LOC fixture.
- **G3 (no-regression):** all existing tests pass (the framework + clippy + corpus tests); the new CLI adds tests, breaks none.
- **G4 (alloc-free hot path):** the visitor walk reuses a scratch buffer; no per-file allocation in steady state (mirror the drafter's alloc-free contract).
- **G5 (real-file value):** at least ONE real fix lands in a sibling repo (riir-ai or riir-gpu) — a perf or kernel rule detects a real anti-pattern + the proposer rewrites it + the change compiles + tests pass. This is the "does it actually help" gate.

## References

- [Proposal 001](001_multi_domain_modelless_code_healer.md) — the multi-domain meta-architecture + the `cargo heal` sketch this concretizes.
- [Plan 001](../.plans/001_multi_domain_framework_refactor.md) — the framework refactor (DONE).
- [Plan 005](../.plans/005_real_file_span_healing.md) — the execution plan for this proposal.
- `src/domains/clippy_lints/proposers/` — the 10 proposer codemods this generalizes.
- `katgpt-core` `TernaryDrafter` — the shallow-reasoning embedder.
- [Bench 013](../.benchmarks/013_*.md) — the perf-rule Pareto verdict (34/38 dominant) that prioritizes proposer coverage.
- `quest_grammar` (riir-games-quest) — the drafter+pruner+corpus precedent.
