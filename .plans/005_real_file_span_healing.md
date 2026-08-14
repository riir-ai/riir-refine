# Plan 005 — Real-file span-level healing for `rust_perf` + `kernel_opt`

**Proposal:** [002](../.proposals/002_real_file_span_healing.md)
**Branch:** `develop` (per global rule — no feature branches)
**Status:** proposed (awaiting go-ahead)

## Goal

Lift the `clippy_lints` proposer pattern (real `syn` AST rewrites) into `rust_perf` + `kernel_opt`, add a shallow-reasoning matcher (so perf/kernel detect their own patterns), and ship a unified `cargo heal` CLI that scans real `.rs` files + applies all three domains. This is the `quest_grammar` fusion (Proposal 002): the corpus entries become real `syn` rewrites, not string templates.

**This is a product-capability plan, not a corpus-expansion plan.** The GOAT gate (G5)
requires at least one real fix to land in a sibling repo.

> **BREAKTHROUGH REVISION (2026-08-14):** grep revealed the unified latent-retrieval
> substrate already ships across riir-rag (`AstChunker` + `RetrievalProvider` +
> `ModellessEmbedder`) + katgpt-core (`pick_domain<const N, const A>`). This collapses
> the original 6-phase plan: Phase 1 becomes "wire riir-rag" (not "write proposer
> surface"), cross-domain routing is IN SCOPE (N=3 instantiation), and `--suggest`
> mode ships with ZERO proposer code (the corpus ref_shapes are already indexable).
> The phases below are revised. See Proposal 002 §"Revised architecture" for the
> authoritative design. The proposer work (original Phase 2/3) is now Phase 4/5 —
> decoupled from the critical path.

## Substrate-first gate (pre-flight, MANDATORY)

Before implementing, verify the substrate exists:
- [x] **`AstChunker`** (riir-rag `ast_chunker.rs`) — chunks Rust files at fn/impl/struct
      boundaries WITH line ranges. "Put AST in riir-rag" = ALREADY IMPLEMENTED.
- [x] **`RetrievalProvider::retrieve(LatentQuery, budget)`** (riir-rag `query.rs`) —
      Clifford-wedge KNN over latent embeddings. IS the rule-matcher.
- [x] **`ModellessEmbedder`** (riir-rag `embedder.rs`) — BLAKE3+DFT+sigmoid, 8-D, no weights.
      (opt-in `BonsaiEmbedder` 768-D for higher accuracy.)
- [x] **`pick_domain<const N, const A>`** (katgpt-core `variable_rank_domain_expert.rs`) —
      generic MoE router. N=3 for cross-domain is a one-line instantiation.
- [x] **`LatentQuery::from_direction([f32;8])`** (riir-rag) — the query builder.
- [x] `clippy_lints/proposers/` — 10 `syn` AST codemods (the rewrite pattern, decoupled).
- [x] `RustAnalyzerPruner` — `syn`-based parse validation.
- [x] `KernelExpertRouter::route()` — within-kernel MoE (N=15); composes with cross-domain.
- [x] Corpus: 46 perf + 123 kernel + 10 clippy rules — the indexed knowledge base.

**Zero new substrate invented.** The entire revised pipeline composes existing pieces.

## Phase 1 — Wire riir-rag latent retrieval (the `--suggest` MVP, ZERO proposers)

The critical-path milestone. Index the corpus rules, embed real code chunks, retrieve
matching rules via Clifford-wedge KNN. This ships a WORKING `--suggest` mode with no
proposer code at all — the corpus `ref_shape`/`opt_pattern` text is the suggestion.

**Status: DONE 2026-08-14** (13 tests, clippy-clean, `latent_retrieval` feature).
One design deviation, empirically driven: retrieval is **fan-out across all
enabled domains** (`suggest_span` doc has the full account) — the plan's
gated route→retrieve starved the correct domain on fixture testing (the
8-D structural centroids mis-routed a `return expr;` fn to perf + a
linear-scan fn to clippy, swapped). The router verdict survives as an
annotation; re-gating is a Phase 3 tuning option (tuned centroids or the
768-D embedder). Second addition: **multi-scale indexing + queries** (whole
pattern + significant pattern lines; whole span + significant span lines) —
corpus patterns are 1-5 line excerpts while real spans are whole items, and
the single-scale retrieval missed the needle on the file fixture.

### Tasks

- [x] **T1.1** Add `riir-rag` as a dep of `riir-clippy` (behind a `latent_retrieval`
      feature gate; pulls `AstChunker` + `ModellessEmbedder` + `RetrievalProvider`).
- [x] **T1.2** `src/draft/corpus_index.rs` — index each domain's corpus entries as
      riir-rag chunks. The `ref_shape` text → `ModellessEmbedder.embed()` → `ShardIndex`.
      One index per domain (3 indices) OR one index with a domain tag.
      Done as one `RuleIndex` per domain (multi-scale: whole `buggy_code`/`ref_shape`
      + significant pattern lines, keyed `BLAKE3 → rule_id`).
- [x] **T1.3** `src/draft/latent_matcher.rs` — given a real code chunk: `AstChunker.chunk()`
      → `ModellessEmbedder.embed()` → `LatentQuery::from_direction()` → `retrieve()`.
      Returns `Vec<(rule_id, latent_score)>` ranked.
- [x] **T1.4** `src/draft/domain_router.rs` — `pick_domain::<3, 8>` with hand-crafted
      `DOMAIN_DIRECTIONS` (clippy/perf/kernel signatures). The 8-D activity = the code
      chunk embedding. Output: domain index 0/1/2.
      Deviation: corpus-derived centroids (L2-normalized mean of pattern embeddings)
      instead of hand-crafted directions, and a runtime-sized argmax mirroring
      `pick_domain`'s tie semantics (the enabled-domain set is feature-dependent —
      `pick_domain` is const-generic over N). Routing verdict is an annotation, not
      a gate (see the Phase 1 status note).
- [x] **T1.5** Compose: chunk → embed → route(domain) → retrieve(from that domain's index)
      → ranked rules. This is the `--suggest` pipeline.
      Done as fan-out (all enabled domains), router verdict annotated.
- [x] **T1.6** Tests: index a small fixture corpus, embed a matching code span, confirm
      the top-1 retrieved rule is the correct one (per domain).
      13 draft tests: index build/multi-scale/top-1/determinism, centroid
      unit-norm/separation/routing/panics, matcher span/file/recall.

### Phase 1 exit criteria
- Latent retrieval works end-to-end: real code chunk → correct rule retrieved, for all
  3 domains. **ZERO proposer code written.** `--suggest` mode is functionally complete
  (surfacing rules + opt_patterns); `--fix` needs Phase 4/5 proposers.

## Phase 2 — `cargo heal --suggest` CLI

Ship the user-facing binary that runs Phase 1's pipeline over real files.

**Status: not started.**

### Tasks

- [ ] **T2.1** `src/bin/heal.rs` — args: `--suggest`, `--domain <auto|clippy|perf|kernel>`
      (default `auto` = dMoE routing), `[path...]`.
- [ ] **T2.2** File scanner: walk `path`, `AstChunker.chunk()` each `.rs` file.
- [ ] **T2.3** For each chunk: embed → route(domain) → retrieve → print top-K rules with
      scores + the `opt_pattern` snippet for manual review.
- [ ] **T2.4** Domain gating: `--domain perf` requires `--features rust_perf`; etc.
      `--domain auto` runs all enabled feature-gated domains via the dMoE router.

### Phase 2 exit criteria
- `cargo heal --suggest src/` prints real suggestions on real files. The product exists
  (suggest mode). `--fix` is Phase 4/5.

## Phase 3 — GOAT gate for the latent path (G1-G4, `--suggest` mode)

Measure the retrieval quality + latency before investing in proposers.

**Status: not started.**

### Tasks

- [ ] **T3.1 (G1 retrieval precision)** On a fixture of known anti-patterns, the top-1
      retrieved rule is correct ≥80% of the time (per domain). Record in `.benchmarks/`.
- [ ] **T3.2 (G2 latency)** Chunk + embed + route + retrieve < 1ms per chunk (Clifford
      wedge KNN is O(log N); Bench 575 measured 123µs p50 @ N=1000).
- [ ] **T3.3 (G3 no-regression)** All existing riir-clippy tests pass.
- [ ] **T3.4 (G4 alloc-free hot path)** Reuse scratch buffers in the retrieve loop.

### Phase 3 exit criteria
- G1-G4 PASS for `--suggest` mode. If G1 (precision) is <80%, tune the domain directions
  or switch to `BonsaiEmbedder` (768-D, opt-in) before proceeding to proposers.

## Phase 4 — Top-5 perf proposers (the `--fix` upgrade for perf)

Proposers enable `--fix` (auto-apply). Start with perf (clippy already has 10).

**Status: not started.** (Same as original Plan 005 Phase 2 — unchanged by the revision.)

### Tasks
- [ ] **T4.1-T4.7** The 5 perf proposers (`vec-with-capacity`, `entry-vs-contains-insert`,
      `static-str`, `bool-to-bitpack`, `fxhashmap`) + `propose_fix` dispatch + fixture tests.
      (See original Plan 005 Phase 2 for details — unchanged.)

## Phase 5 — Top-5 kernel proposers (the `--fix` upgrade for kernel)

**Status: not started.** (Same as original Plan 005 Phase 3 — unchanged.)

### Tasks
- [ ] **T5.1-T5.6** The 5 kernel proposers (launch/dispatch/layout only, outside `#[cube]`).
      (See original Plan 005 Phase 3 — unchanged.)

## Phase 6 — `cargo heal --fix` + final GOAT gate (G5 real-file value)

**Status: not started.**

### Tasks

- [ ] **T6.1** Wire `--fix` mode: for each retrieved rule, if a proposer exists + latent
      score > threshold (0.8), apply the rewrite (span-preserving); else fall back to
      `--suggest` output for that rule.
- [ ] **T6.2 (G5 real-file value)** At least ONE real fix lands in a sibling repo
      (riir-ai or riir-gpu): the latent matcher detects a real anti-pattern, the proposer
      rewrites it, the change compiles + tests pass.
- [ ] **T6.3** `.benchmarks/016_real_file_heal_goat.md` — full G1-G5 measurement.

### Phase 6 exit criteria
- G5 PASS. `cargo heal --fix` is production-usable for the 10 shipped proposers +
  `--suggest` covers the rest of the corpus.

## Phase 7 — Self-healing closed loop (MAG online EMA + LatentFixMemory)

The fusion layer from Proposal 002 §"Fusion layer". The healer improves its own
retrieval + routing from every fix outcome. **All substrate ships** (`LatentSkillEvolution`
+ `LatentFixMemory` in riir-clippy Issue 004/008); this phase generalizes the key from
per-lint to per-domain + wires the outcome recorder into the `--fix` path.

**Status: not started.**

### Tasks

- [ ] **T7.1** Generalize `LatentSkillEvolution` key from `lint_name: &str` to
      `domain_key: &str` (where domain_key = "clippy"/"perf"/"kernel" OR the
      domain+rule_id composite). The EMA mechanism is domain-agnostic; only the key shape
      changes. Back-compat: existing per-lint callers pass `lint_name` as the key.
- [ ] **T7.2** Wire `HealPipeline::heal()` to record the outcome after validation:
      `evolution.record_outcome(domain, fix_direction, validated)`. The fix_direction =
      `ModellessEmbedder.embed(fixed_code)`. This evolves the DOMAIN_DIRECTIONS used by
      `pick_domain<3,8>` — the dMoE router gets smarter with use.
- [ ] **T7.3** Wire `LatentFixMemory` into the retrieval path (step 4↻ in the pipeline):
      before the corpus index retrieve, check if a past trajectory matches (same domain +
      similar code span). If a high-confidence past fix exists, prefer it (it was
      validated). This is the 3-axis retrieval (latent + BM25 + trajectory) already shipped.
- [ ] **T7.4** The freeze/thaw: evolved directions + memory persist across runs (via
      `freeze()`/`thaw()` on `Corpus` + the existing BLAKE3 commitment). The healer's
      learned directions are reproducible + auditable.
- [ ] **T7.5** GOAT gate (self-healing): after N=100 fix cycles on a fixture corpus, the
      retrieval precision (G1 from Phase 3) improves ≥10% vs the cold-start hand-crafted
      directions. Record in `.benchmarks/`. This is the MAG "does unsupervised direction
      mining help" gate — modelless, online.

### Phase 7 exit criteria
- Self-healing loop ships: `--fix` records outcomes → directions evolve → next `--fix`/
  `--suggest` retrieves better. The dMoE router + corpus retrieval both improve with use.
  GOAT: ≥10% precision improvement after 100 cycles.

## Phase 8 — WASM pruner fast-validation hook (Proposal 011, gated on rubrc)

The fast `--fix` validator. Blocked on `rubrc` maturity (Proposal 011 Phase 3) — this
phase is SKETCHED, not started. Until it lands, `cargo test` (slow) + `RustAnalyzerPruner`
(syntax-only, fast) are the validators.

**Status: blocked on rubrc (Proposal 011).**

## Phase 9 — Calibrated UQ layer (Deep fusion A: conformal + select_best_belief + clr_vote)

Replace the uncalibrated `DraftedFix.confidence = sigmoid(mean(matvec_output))` with
calibrated, distribution-free confidence intervals. Three DEFAULT-ON katgpt-core
primitives fuse: `ConformalIntervalCalibrator<F>`, `select_best_belief`, `clr_vote`.
All three are verified to ship via direct grep (2026-08-14). All are floor-tested.

**Status: not started. Substrate verified.**

### Tasks

- [ ] **T9.1** Define `FixForecaster: PointForecaster` impl that maps
      `(rule_id, buggy_hash) → f32` predicted-correctness score. The forecast
      target = 1.0 if the fix was actually correct (post-verify), 0.0 otherwise.
      Wrap `TernaryDrafter`'s existing `confidence` output as the point forecast.
- [ ] **T9.2** Wrap with `ConformalIntervalCalibrator<FixForecaster>` behind a
      `calibrated_uq` feature gate. Per-rule residual ring buffer (capacity 64).
      The calibrated output replaces `DraftedFix.confidence` with a true 95%
      interval `[lo, hi]`.
- [ ] **T9.3** Replace the `0.6·evolution + 0.4·success_rate` linear blend in
      `HealPipeline::heal` with `select_best_belief(candidates, ε, incumbent)`.
      Each candidate carries `(successes, failures)` as Beta posterior counts.
- [ ] **T9.4 (clr_vote, opt-in `clr_voting`)** Generate K=8 drafted fixes
      (currently 1) via QMC perturbation of the ternary matvec, cluster by
      syntactic similarity (Levenshtein on AST node-kind sequence), run L2
      pruner on each, `clr_vote` picks the most-reliable cluster's winner.
- [ ] **T9.5** GOAT gate (calibrated_uq): empirical coverage of the 95%
      interval ≥ 90% on a fixture corpus; `select_best_belief` selection-regret
      ≤ the linear-blend baseline (Bench 010 T5 floor). Record in
      `.benchmarks/017_calibrated_uq_goat.md`.

### Phase 9 exit criteria
- `DraftedFix.confidence` is a calibrated interval, not a raw sigmoid.
- `HealPipeline::heal` selects fixes via Beta-posterior, not linear blend.
- GOAT G1 (coverage) + G2 (regret vs linear blend) PASS.

## Phase 10 — Sleep-cycle consolidation layer (Deep fusion B: mine_motifs_at_sleep_cycle + LocalKvStore + can_freeze)

The healer stops re-learning from scratch each session. Three substrates compose:
`mine_motifs_at_sleep_cycle` (overnight motif mining over fix trajectories) →
`ConsolidationPipeline::can_freeze` (gate) → `LocalKvStore` (BLAKE3-committed
persistence of frozen motifs). All three verified via direct grep (2026-08-14).

**Status: not started. Substrate verified.**

### Tasks

- [ ] **T10.1** Generalize `FixTrajectory` to PTG-shape (primitive transition
      graph) — the input shape `mine_motifs_at_sleep_cycle` expects. Each fix
      becomes a 3-node PTG: `(retrieve_rule) → (draft_fix) → (verify_outcome)`.
- [ ] **T10.2** Persist fix trajectories via `LocalKvStore::put_pod<FixOutcome>`
      keyed by `BLAKE3(lint_name ∥ rule_id ∥ buggy_hash)`. Survives restart.
      Native-only (no WASM — `LocalKvStore` uses std::fs).
- [ ] **T10.3** Implement `cargo heal --sleep` CLI: drain accumulated trajectories
      → `mine_motifs_at_sleep_cycle(miner, admitter, dl_old_bits)` →
      `SleepCycleClosureReport` → admit high-PRI motifs via `MotifAdmitter`.
- [ ] **T10.4** Gate motif graduation with `ConsolidationPipeline::can_freeze`
      semantics: only freeze motifs with `n_wake_events ≥ d AND flatness < τ`.
      Prevents premature freezing of noisy patterns.
- [ ] **T10.5** Freeze admitted motifs via the existing `freeze()`/`thaw()` on
      `Corpus` (BLAKE3-committed). The healer's learned skill library is
      reproducible + auditable.
- [ ] **T10.6** GOAT gate (sleep_consolidation): on a fixture corpus with N=100
      fix cycles, `cargo heal --sleep` extracts ≥1 motif that the next `--fix`
      cycle retrieves (precision improvement ≥10% vs cold-start). Record in
      `.benchmarks/018_sleep_consolidation_goat.md`.

### Phase 10 exit criteria
- `cargo heal --sleep` mines motifs from past trajectories, freezes them,
  persists them across restart.
- The healer's skill library improves overnight.

## Phase 11 — Poincaré fix-imagination layer (Deep fusion C: imagine_belief_into + ViableManifoldGraph)

**THE NOVEL ANGLE.** The healer imagines the fix in Poincaré space BEFORE the
expensive L2 validate. Two substrates compose: `poincare_navigate_into`
(single-step imagination) + `ViableManifoldGraph` (safe fix-neighborhood graph).
Both verified via direct grep (2026-08-14).

**Status: not started. Substrate verified. Honest risk documented (Plan 497 Phase 2 quality REFUTE for NPC HLA; latency win stands; code-healing nonlinearity bar is lower but not zero).**

### Tasks

- [ ] **T11.1** Collect (pre-fix embedding, post-fix embedding) trajectory
      pairs from past `--fix` cycles. The corpus needs trajectory shape — the
      current flat `FixTrajectory` is insufficient (blocker for T11.2).
- [ ] **T11.2** Fit `PoincareAdapter` offline via `fit_poincare_adapter(
      z_samples, target_samples, latent_dim=8, target_dim=8, ...)`. Closed-form
      PCA + ridge regression, no GD. The adapter is BLAKE3-committed + frozen.
- [ ] **T11.3** Define `score_fix_goodness(z_out: &[f32;8]) -> f32` —
      `dot(z_out, goodness_direction) + sigmoid` onto a learned "correct code"
      direction. The direction is mined from past verified fixes.
- [ ] **T11.4** Build `SafeManifoldGraph` from a corpus of compile-passing code
      embeddings. Predicate = "does this state compile?". Curiosity walk =
      "what's the nearest compile-passing neighbor of the proposed fix?".
- [ ] **T11.5** Compose: chunk → embed → route → retrieve → draft fix →
      **IMAGINE via `poincare_navigate_into(z_src, src_pos, dst_pos, z_out)`** →
      score → if score < τ, reject + try next retrieved rule → if score ≥ τ,
      L2-validate. The imagination gate is µs-scale; the L2 pruner is ms-scale;
      net throughput rises if imagination rejects ≥20% of bad fixes.
- [ ] **T11.6** GOAT gate (poincare_imagination): on a fixture corpus,
      imagination rejects ≥30% of "would-fail-L2" fixes (recall) while accepting
      ≥90% of "would-pass-L2" fixes (precision). Latency improvement ≥1.5× vs
      no-imagination baseline. Record in `.benchmarks/019_poincare_imagination_goat.md`.
- [ ] **T11.7** Multi-step imagination (`poincare_multi_step_into`) for large
      fixes that span multiple chunks: split the fix into N sub-steps, imagine
      each, reject if any sub-step diverges.

### Phase 11 exit criteria
- Imagination gate ships behind `poincare_imagination` feature gate (opt-in).
- GOAT G1 (recall ≥30%) + G2 (precision ≥90%) + G3 (latency ≥1.5×) PASS.
- If quality REFUTED (like Plan 497 Phase 2 for NPC HLA): document honestly,
  keep the latency win, demote the quality claim, do NOT promote.

## Phase 12 — Trajectory-archetype classifier (Deep fusion E: SweTrajectoryFreezer)

Each fix attempt is classified into an archetype (`converged_correct`,
`committed_wrong`, `oscillation`) via `SweTrajectoryFreezer<N, D>`. Verified
via direct grep (2026-08-14, Bench 013 G3 + Bench 014 G5 PASS).

**Status: not started. Substrate verified.**

### Tasks

- [ ] **T12.1** Define the 3 archetypes as a `FixArchetype` enum.
- [ ] **T12.2** Fit `SweTrajectoryFreezer<3, 8>` from past `--fix` trajectories
      labelled by outcome (converged_correct = post-fix passes L2; committed_wrong
      = post-fix fails L2; oscillation = drafter flip-flops across cycles).
- [ ] **T12.3** Wire `freezer.freeze_attempt(trajectory) → archetype` into the
      `--fix` path as a complementary confidence signal to the conformal one
      (Phase 9).
- [ ] **T12.4** GOAT gate: cross-archetype discrimination (Bench 013 G3
      methodology) PASS on the fixture corpus. Record in
      `.benchmarks/020_trajectory_archetype_goat.md`.

### Phase 12 exit criteria
- `committed_wrong` + `oscillation` archetypes escalate to human review.
- `converged_correct` archetype unlocks the high-confidence auto-apply path.

## Phase 13 — ConvexTok AST vocabulary (Deep fusion D: research-gated, NOT a plan yet)

**THE SUPER-GOAT.** Apply the ConvexTok LP-graph to AST node-kind sequences →
learn a globally-optimal AST multi-node token vocabulary. Genuinely novel — no
code-domain analog exists (grep confirmed 2026-08-14).

**Status: NOT STARTED. This is a research question, not a plan.** The LP
infrastructure (`GraphBuilder`, `ConvexSolver`, `Rounder`, `Certifier`) is
100% reused from katgpt-tokenizer. The open question is the colouring scheme
(what is a "colour" for an AST node?).

### Tasks (research, not implementation)

- [ ] **T13.1** File `.research/NNN_convex_ast_vocab.md` — design the AST-node
      colouring scheme (node-kind vs kind+arity vs kind+child-shape). Cite
      ConvexTok paper. Sketch the encoding.
- [ ] **T13.2** Build a 1000-fix-corpus AST node-kind sequence dataset (extract
      via `syn` visitor from the existing `ClippyCorpus` + perf + kernel rules).
- [ ] **T13.3** Run `GraphBuilder::build(ast_sequences, max_len=8)` +
      `ConvexSolver::solve(graph, budget_k=64)` + `Rounder::det` on the dataset.
      Measure: how many recurring AST motifs does the LP find? What's the
      compression ratio vs the flat per-rule corpus?
- [ ] **T13.4** If T13.3 shows ≥2× compression: file a `.proposals/` for the
      ConvexTok-AST vocabulary as a Phase 14 implementation plan. Else:
      document the negative result + close.

### Phase 13 exit criteria
- Research note filed. Either a follow-up plan is opened (T13.4 positive) or
  the avenue is documented as a negative result (T13.4 negative).

## Phase 14 — Verified additional substrates (HebbianKernelMemory + DeltaMemoryState + ExperienceReuseHook)

The deep-fusion harvest surfaced three more substrates. Originally marked "unverified",
they are NOW CONFIRMED via direct grep (2026-08-14):

- **`DeltaMemoryState`** — `katgpt-core/src/delta_mem/state.rs` (DEFAULT-ON, Plan 408).
  Rank-r associative bandit memory. `read(key) → value`, `write(key, value)` (δ-update),
  `mean_prediction_error()` (drift signal). Composition: `MemorySteeredPruner<P>`
  wraps an inner pruner and adds memory-steered corrections: `corrected_relevance =
  base + α·memory.read(context)`. **Use as the drafter's correction layer** — key =
  `(lint_name_hash, buggy_code latent)`, value = fix-direction correction.
- **`HebbianKernelMemory<D>`** — `katgpt-core/src/hebbian_kernel_memory.rs` (Plan 559,
  G1-G5 PASS, DEFAULT-ON). Closed-form fact-storing MLP. Bilinear sketched-K₂ feature
  map + ridge-whitened readout. **Store verified (rule_id, buggy_code) → fixed_code
  mappings as Hebbian facts.** Retrieval is O(1) associative recall. Replaces the
  current `HashMap<String, Direction>` in `LatentSkillEvolution`.
- **`ExperienceReuseHook` + `query_priors_dual`** —
  `riir-engine/src/arg_runtime/experience_reuse.rs`. Latent-seeded NS-traversal over
  a fix-trajectory DAG. Dual-stream returns BOTH `ReuseCandidate` (positive priors,
  UCB-scored) AND `AvoidCandidate` (negative priors, Q-negative failure signal).
  **Replaces the flat `Vec<FixTrajectory>` scan with graph-indexed prior lookup.**
  Dual-stream = "these fixes worked" + "these fixes caused cargo build breakage —
  avoid". The natural sink for `cargo clippy`/`cargo build` verify feedback.

**Status: substrate verified. Not started.**

### Tasks (additive — these compose on top of Phases 9-12)

- [ ] **T14.1** Wrap `TernaryDrafter` in `MemorySteeredPruner<P>` so each draft
      is corrected by `DeltaMemoryState::read(context)` keyed on
      `(lint_name_hash, buggy_code_latent)`. The `write()` happens on each
      verified fix (success → +α toward the correction, fail → -α away).
- [ ] **T14.2** Replace `HashMap<String, Direction>` in `LatentSkillEvolution` with
      `HebbianKernelMemory<8>` for the verified-fix fact store. O(1) associative
      recall vs O(N) hash scan. The fact corpus = (rule_id, buggy_hash) → fixed_code.
- [ ] **T14.3** Replace the flat `Vec<FixTrajectory>` in `LatentFixMemory` with
      `ExperienceReuseHook`. The hook gives graph-indexed prior lookup +
      dual-stream (positive + negative priors) for free.
- [ ] **T14.4** Wire `cargo build` failure outcomes into the dual-stream's
      `record_observation_outcome_dual`: `cargo build` fail = strong negative prior.
      `cargo clippy` fail = weak negative prior. `cargo test` pass = strong
      positive prior. This is the modelless RL signal layer.
- [ ] **T14.5** GOAT gate (verified_substrates): on the fixture corpus,
      memory-steered drafting (T14.1) + Hebbian fact recall (T14.2) + graph-indexed
      priors (T14.3) collectively improve G1 precision ≥5% over Phase 9 alone.
      Record in `.benchmarks/021_verified_substrates_goat.md`.

### Phase 14 exit criteria
- All three substrates wired. The healer's memory layer is no longer flat +
  in-memory; it's Hebbian fact store + graph-indexed dual-stream + memory-steered
  drafter.
- GOAT G1 PASS (≥5% improvement over Phase 9).

### Tasks (sketch — not started)

- [ ] **T8.1** `WasmPruner: ConstraintPruner` impl (katgpt-rs) — compiles the test suite
      to WASM, runs the proposed fix's token sequence against it, returns valid/invalid.
- [ ] **T8.2** Wire into `cargo heal --fix` as the fast validator (replaces `cargo test`
      subprocess for the common case). Fallback to `cargo test` if WASM pruner declines.
- [ ] **T8.3** GOAT: WasmPruner validation < 10ms (vs ~seconds for cargo test).

### Phase 8 exit criteria
- BLOCKED. Reopens when rubrc lands (Proposal 011 Phase 3). Until then, the slow path
  (`cargo test`) is the validator; the self-healing loop (Phase 7) works with either.

## Out of scope (per Proposal 002 revised + deep fusion layer)

- Full proposer coverage (all 34 perf + ~30 kernel) — decoupled; `--suggest` covers them.
- CubeCL `#[cube]` macro-body proposers — `--suggest` still works (embedding captures the
  signature); only `--fix` is limited.
- L4 LoRA — deferred across all domains.
- `--watch` mode — deferred.
- Domain-direction mining via `CommittedFieldBlend` — hand-crafted vectors ship first.
- **ConvexTok-AST implementation** — Phase 13 is research-only. The implementation
  plan (Phase 15+) opens only if the encoding design + corpus experiment (T13.3)
  shows ≥2× compression.

## Risks

1. **Span-preserving patch is harder than it looks.** `syn` spans are byte-offsets; re-emitting only the changed span while keeping surrounding bytes intact needs care. Mitigation: `prettyplease` fallback (T1.3) ships first; span-preserving is the upgrade.
2. **Visitor false positives.** A `vec-with-capacity` visitor might match a vec that's intentionally un-sized. Mitigation: visitors are conservative (only rewrite when the size is statically known); uncertain cases go to `--suggest`.
3. **CubeCL parse failure.** A real riir-gpu file with `#[cube]` macros won't parse with vanilla `syn`. Mitigation: the scanner skips files that fail to parse (with a warning); proposers run only on parseable files. Kernel proposers are tested on EXTRACTED snippets, not whole kernel files.
4. **G5 might fail.** If no real anti-pattern exists in the sibling repos for the 10 shipped rules, G5 can't pass. Mitigation: the rule selection (Phase 2/3 candidates) is chosen BECAUSE the patterns are common; if G5 fails, expand the rule set or target a different repo.

## Numbering discipline

- Proposal 002: `riir-clippy/.proposals/002_real_file_span_healing.md`
- Plan 005: this file.
- Highwater: `.plans/.highwater` = `5`, `.proposals/.highwater` = `2` (written back after this plan lands).
