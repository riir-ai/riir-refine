# Proposal 001 — Multi-Domain Modelless Code Healer (Meta-Architecture)

Status: **draft**
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: [Proposal 034](../../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) (clippy-healing — the original single-domain pipeline) × [Proposal 012](../../katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md) (kernel-heal domain — **supersedes its "new repo" conclusion**) × [`HealConflictDetector`](../../katgpt-rs/crates/katgpt-core/src/) (Issue 133 — heal-path validation trait precedent) × [`pick_domain`](../../katgpt-rs/crates/katgpt-core/src/variable_rank_domain_expert.rs) (Plan 558 — latent-MoE expert selection) × [`rust-optimize` skill](file:///Users/katopz/.agents/skills/rust-optimize/SKILL.md) (perf-domain L0 rule distillation)
Related: [Bench 009](../.benchmarks/009_bonsai_vs_modelless_e2e_tokps.md) (cache-vs-compute silent-miss lesson — cross-domain), [Plan 001](../.plans/001_multi_domain_framework_refactor.md) (execution plan)

## TL;DR

**Should riir-clippy generalize from a clippy-lint-only healer into a pluggable multi-domain modelless code healer, where each domain (clippy lints, rust perf, kernel opt, future) contributes a corpus + validator + optional expert set behind a `HealDomain` trait, sharing one domain-agnostic pipeline (drafter + pruner + ruliology + self-evolve)?**

**Yes.** The current pipeline (`ClippyCorpus` → `ClippyTernaryDrafter` → `RustAnalyzerPruner` → `HealPipeline`) is domain-agnostic in mechanism but clippy-hardcoded in vocabulary. The generalization is a trait extraction + rename, not a new architecture. Three domains ship behind feature gates: `clippy_lints` (current, default — back-compat preserved via re-export), `rust_perf` (rules distilled from the `rust-optimize` skill's checklist), `kernel_opt` (Proposal 012's design, **4090 LoRA path deferred**). Future domains drop in by implementing the trait + a feature gate.

**This SUPERSEDES Proposal 012's "new repo `riir-kernel-heal`" conclusion.** The user's correction (this session): same pipeline shape → same repo, domain as feature. The kernel domain is `feature = "kernel_heal"` in riir-clippy, not a separate repo. Proposal 012's *domain design* (thermal-tier routing + latent-MoE expert selection + numeric-compare validator) stands unchanged — only the *placement* changed. This is the substrate-first rule applied to our own prior proposal.

**This is NOT a novel research idea.** It is an in-house refactoring that generalizes an already-shipped pipeline. The only genuinely new abstraction is the `HealDomain` trait (the seam between domain-agnostic core + domain-specific corpus/validator). Everything else is rename + re-export + feature-gate.

## The problem this solves

### Problem 1: The pipeline is domain-agnostic in mechanism but clippy-hardcoded in vocabulary

`src/lib.rs` ships `ClippyCorpus`, `ClippyFixEntry`, `ClippyTernaryDrafter`, `build_seed_corpus` (clippy lints). The `RustAnalyzerPruner` + `HealPipeline` + `LatentFixMemory` + `RuliologySearch` are already domain-neutral in behavior — they operate on `(rule_id, buggy_code, fixed_code)` tuples + `syn` parse. The `Clippy` prefix is a vocabulary leak, not a capability boundary.

When the user asked "why can't this heal kernels?" (this session), the answer was "it can — the pipeline doesn't care what code it heals". The blocker is that the corpus type + the validator + the entry point are all named for clippy. Generalizing them is a trait extraction.

### Problem 2: Proposal 012 proposed a new repo; the user corrected to "same repo, domain as feature"

Proposal 012 (`katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md`) concluded "ship as a new repo `riir-kernel-heal`". The user's correction: *"why it can't be clippy? it's like clippy for kernel?"* — the boundary rule is about **dep surface** (katgpt-core only, no private siblings), not about **target domain**. The kernel healer preserves that dep surface exactly (pick_domain, thermal tier, ternary matvec all live in katgpt-core). The CubeCL compile validator is a subprocess behind `kernel_verify`, mirroring how `cargo clippy` is a subprocess behind `clippy_verify`.

A new repo for each domain (riir-kernel-heal, riir-perf-heal, ...) would duplicate the entire drafter+pruner+self-evolve pipeline N times. The right shape is ONE pipeline, N domain plugins.

### Problem 3: The `rust-optimize` skill is already a hand-written L0 corpus for the perf domain

The `rust-optimize` skill checklist IS the L0 rule distillation for a `rust_perf` domain:

| Skill checklist item | L0 rule (buggy → fixed) |
|---|---|
| Complexity: O(n) scan → O(1) lookup | `for item in &items { if item.key == k }` → `stats[key].rate()` |
| Allocations: `String` → `&'static str` | `.to_string()` in hot loop → `&'static str` |
| Layout: field reorder (u64→u32→u8) | padded struct → reordered struct |
| Concurrency: `Arc<RwLock<HashMap>>` → `papaya` | lock-contended map → lock-free map |
| SIMD: chunked loops, branch-free inner | scalar loop → `chunks_exact(8)` |
| Bevy: event loop spin | `WinitSettings::default()` → `reactive_low_power` |
| WASM FFI: batch API | N×M serial FFI calls → 1 batch call |

Each row is a `(rule_name, buggy_pattern, fixed_pattern)` triple — exactly the `FixEntry` shape. The skill is a domain corpus waiting to be encoded. Healing perf issues is structurally identical to healing clippy lints: draft a fix, validate it compiles + passes tests, optionally benchmark it.

### Problem 4: Bench 009's silent-miss is a Stage 1 limitation; Stage 2 is the modelless fix (2-stage drafter, `quest_grammar` precedent)

Bench 009 measured the clippy domain: modelless cache 1.8M tok/s, held-out quality 0/10, silent miss (L2 `parses=true` on wrong answers). [Issue 010](../.issues/010_drafter_cannot_generalize_heldout_code.md) root-caused this: the L1 drafter (`ClippyTernaryDrafter`) is a **corpus retriever with a hash-driven index**, not a code transformer — `buggy_code` only selects *which* stored fix to return; it never influences the returned text. The lesson — *"a perf gain on an incorrect answer is a speedup of a wrong result"* — applies to EVERY domain's **Stage 1 hot path**:

| Domain | Stage 1 silent-miss | Stage 2 ground-truth validator |
|---|---|---|
| clippy_lints | Wrong lint fix — usually harmless | `cargo clippy` re-run (binary, ~130 ms) — fine for tuning |
| rust_perf | Wrong "optimization" that breaks correctness or regresses perf | `cargo test` + `cargo bench` (seconds) |
| kernel_opt | **Wrong GPU output → desync + anti-cheat false positives + replay divergence** (Proposal 012 §Problem 4) | compile + run + **numeric compare vs reference** |

**The fix is a 2-stage drafter**, mirroring `quest_grammar`'s `plasma_draft` (Stage 1, always-on, ~µs, hash-selected template pool) + `adaptive_drafter` (Stage 2, optional override, loaded when a verified pool exists). See [Issue 010](../.issues/010_drafter_cannot_generalize_heldout_code.md) §"Fix direction" for the full pattern. The key point: **re-running the domain validator (e.g. `cargo clippy`) during the tuning phase is fine** — the ~130 ms cost is acceptable OFF the hot path; it is the mechanism that populates the Stage 2 pool. The hot path stays at Stage 1's ~µs latency and upgrades to Stage 2 only when a verified entry exists.

The validator MUST be domain-pluggable (the `HealDomain::validate` method) AND each domain's Stage 2 tuning loop MUST hold a ground-truth signal stronger than "it parsed". This is a **modelless generalization path** (per the modelless-first mandate, it must be exhausted before L4 LoRA).

## The proposed design

### The one new abstraction: `HealDomain` trait

```rust
/// One pluggable code-healing domain (clippy lints, rust perf, kernel opt, ...).
///
/// The domain-agnostic core (drafter + pruner + ruliology + self-evolve)
/// operates on the corpus entries this trait produces + the validator verdict
/// it returns. The domain owns: (1) what counts as a fixable rule, (2) how to
/// validate a proposed fix against ground truth.
pub trait HealDomain {
    /// Corpus entry shape: (rule_id, buggy_code, fixed_code, metadata).
    /// Metadata carries domain-specific signal (lint_group, perf_metric,
    /// kernel_shape_dtype_hw, ...).
    type Entry: Clone + Send + Sync + 'static;

    /// Validator verdict: did the proposed fix satisfy the domain's ground truth?
    /// Stronger than "parses" — clippy re-runs clippy, perf runs tests+bench,
    /// kernel runs numeric compare vs reference (Proposal 012).
    type Verdict: Clone + Send + Sync;

    /// Build the seed corpus for this domain (BLAKE3-committed, freeze/thaw-ready).
    /// Mirrors `build_seed_corpus()` for clippy; each domain ships its own.
    fn build_corpus() -> Vec<Self::Entry>;

    /// Validate a proposed fix against ground truth.
    /// This is the **Stage 2 tuning-phase validator** (NOT the hot-path guard).
    /// The tuning loop calls this OFF the hot path to build the Stage 2
    /// adaptive drafter pool. clippy re-runs clippy (~130 ms), perf runs
    /// tests+bench (seconds), kernel runs numeric compare vs reference
    /// (Proposal 012). Returns a verdict the pipeline uses for Stage 2 pool
    /// admission + L3 Pareto axes.
    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict;

    /// Extract the (rule_id, buggy_code, fixed_code) triple from an entry.
    /// The domain-agnostic core reads through this — it never indexes into
    /// domain-specific fields directly.
    fn triple(entry: &Self::Entry) -> (&str, &str, &str);
}
```

### 2-stage drafter (`quest_grammar` precedent — Issue 010 fix direction)

The drafter is **2-stage**, mirroring `QuestGrammarPipeline`'s `plasma_draft` + `adaptive_drafter`:

| Stage | quest_grammar analogue | riir-clippy | Latency | When |
|---|---|---|---|---|
| **Stage 1 (hot)** | `TernaryDraftModel` (`plasma_draft`) | `TernaryDrafter` (current L1) | ~µs | always-on; returns one corpus `fixed_code` per rule via BLAKE3→matvec→FNV-1a |
| **Stage 2 (tuned)** | `AdaptiveQuestDrafter` (`adaptive_drafter`) | `<Domain>AdaptiveDrafter` (new) | ~µs serve (after load) | optional override; serves verified fixes from a pool built by the tuning loop |

The routing is one method (mirrors `QuestGrammarPipeline::draft_text`):

```rust
pub fn draft_fix(&mut self, rule_id: &str, buggy_code: &str, corpus: &[D::Entry]) -> Option<DraftedFix> {
    if let Some(ad) = self.adaptive_drafter.as_ref() {
        if let Some(fix) = ad.lookup(rule_id, buggy_code) {
            return Some(fix);                       // Stage 2 if loaded + hit
        }
    }
    self.ternary_drafter.draft_fix(rule_id, buggy_code, corpus)  // Stage 1 otherwise
}
```

**The tuning loop** (where re-running the domain validator is fine) populates the Stage 2 pool OFF the hot path:

1. Stage 1 drafts a candidate.
2. `HealDomain::validate` re-runs the domain's ground-truth check (e.g. clippy's ~130 ms `cargo clippy`).
3. If the fix passes → it joins the Stage 2 pool, keyed by `(rule_id, hash(buggy_code))`.
4. If not → the tuning loop tries alternatives (identity guard decline, span-local AST edit) and verifies those.
5. Verified alternatives join the Stage 2 pool.
6. Subsequent hot-path calls route to Stage 2 if the pool has an entry; else Stage 1.

**Re-running the domain validator during tuning is fine.** The cost (~130 ms clippy / seconds perf / minutes kernel) is acceptable for a batch/background tuning process; it is NOT the hot-path guard. The hot path stays at Stage 1's ~µs and upgrades to Stage 2's ~µs serve only when a verified entry exists. This is a **modelless generalization path** — it must be exhausted before L4 LoRA (per the modelless-first mandate + caveat 4).

See [Issue 010](../.issues/010_drafter_cannot_generalize_heldout_code.md) §"Fix direction" for the clippy-domain concrete design.

### Domain-agnostic core (shared, renamed)

The current modules drop the `Clippy` prefix where it's misleading, keep the behavior:

| Current | After | Why |
|---|---|---|
| `ClippyCorpus` | `Corpus<D: HealDomain>` | Generic over domain's `Entry` type |
| `ClippyFixEntry` | (domain-specific, per-domain) | Each domain defines its own `Entry` |
| `ClippyTernaryDrafter` | `TernaryDrafter` | Already domain-agnostic (ternary matvec on embeddings) |
| `RustAnalyzerPruner` | `RustAnalyzerPruner` (unchanged) | Already domain-agnostic (syn parse) |
| `RuliologySearch` | (unchanged) | Already domain-agnostic (Pareto front) |
| `HealPipeline` | `HealPipeline<D: HealDomain>` | Generic over domain |
| `build_seed_corpus` | `<domain>::build_corpus()` via `HealDomain::build_corpus` | Per-domain |

Back-compat: `pub use` re-exports preserve `ClippyCorpus` / `ClippyTernaryDrafter` / `build_seed_corpus` as type aliases / thin wrappers so existing call sites + examples compile unchanged. This mirrors the SDK facade re-export pattern (riir-game-sdk Issue 002).

### Domain modules (feature-gated)

```text
src/
├── corpus.rs           # Corpus<D: HealDomain> — generic
├── drafter.rs          # TernaryDrafter (Stage 1) — generic
├── adaptive_drafter.rs # AdaptiveDrafter<D> (Stage 2) — generic, quest_grammar precedent
├── pruner.rs           # RustAnalyzerPruner — generic
├── ruliology_search.rs # (unchanged)
├── self_evolve.rs      # (unchanged — hosts the Stage 2 tuning loop)
├── domain.rs           # HealDomain trait
└── domains/
    ├── clippy_lints/   # feature "clippy_lints" (default)
    │   ├── mod.rs      # ClippyLintDomain: HealDomain
    │   ├── corpus.rs   # ClippyFixEntry, build_seed_corpus (current code)
    │   └── verify.rs   # ClippyVerifier (current clippy_verifier.rs) — Stage 2 validator
    ├── rust_perf/      # feature "rust_perf" (opt-in)
    │   ├── mod.rs      # RustPerfDomain: HealDomain
    │   ├── corpus.rs   # PerfRule entries from rust-optimize skill checklist
    │   └── verify.rs   # cargo test + cargo bench subprocess — Stage 2 validator
    └── kernel_opt/     # feature "kernel_heal" (opt-in, 4090 deferred)
        ├── mod.rs      # KernelOptDomain: HealDomain
        ├── corpus.rs   # KernelRule entries (Proposal 012 §Phase 0 — popcorn distillation, DEFERRED)
        ├── router.rs   # pick_domain<N,A> — latent-MoE expert selection (Plan 558)
        ├── thermal.rs  # Plasma→Hot→Warm→Cold→Freeze (FlashAR/QGF pattern)
        └── verify.rs   # compile + run + numeric compare vs reference (Proposal 012) — Stage 2 validator
```

### Cross-domain routing: NOT in scope

Each domain is selected by the caller enabling its feature flag — there is no "this code has both a lint AND a perf issue, which domain heals it?" router. That is over-engineering for now; revisit when a concrete need (a unified `cargo heal` that auto-detects domain from code shape) materializes. The `pick_domain` substrate is used **within** a domain that has multiple experts (kernel_opt: coalesce/tile/fuse experts), not across domains.

## Honest caveats — READ BEFORE IMPLEMENTING  (MANDATORY)

1. **The `HealDomain` trait is the one new abstraction; everything else is rename.** The risk is over-abstracting the trait too early. The three domains (clippy, perf, kernel) have similar shapes NOW, but a fourth domain (e.g., security audit healing, API migration healing) might not fit the `validate(rule_id, proposed_fix) -> Verdict` signature cleanly. Mitigation: ship the trait with exactly three impls (the domains we know), document that the signature is provisional, and revisit if a fourth domain forces a change. Do NOT speculatively add trait methods for hypothetical domains.

2. **The validator cost asymmetry is real, crosses domains, and is a TUNING-phase cost, NOT a hot-path cost.** clippy's `cargo clippy` re-run is ~130 ms (acceptable for the Stage 2 tuning loop — re-running clippy during tuning is fine per Issue 010). perf's `cargo test + cargo bench` is seconds. kernel's `compile + run + numeric compare` is multi-seconds to minutes (Proposal 012). These costs are all **off the hot path** — they populate the Stage 2 adaptive drafter pool; the hot path stays at Stage 1's ~µs. The tuning loop's economics differ per domain: clippy can iterate 1000× in a test, perf maybe 100×, kernel maybe 10× overnight on the 4090. The trait does NOT enforce a latency budget — each domain documents its own. Do not assume the tuning loop works the same way across domains, and do NOT put any validator on the hot path.

3. **The `rust_perf` domain's validator is the weakest link.** clippy has binary ground truth (lint satisfied: yes/no). kernel has numeric ground truth (output matches reference within tolerance). perf has **no single ground truth** — "faster" depends on workload, hardware, input size. A perf "fix" that's faster on the benchmark input might be slower on production input. Mitigation: the perf validator MUST run `cargo bench` across multiple input sizes + report a Pareto verdict (faster on N of M sizes), not a binary pass/fail. This is harder than the other two domains and might land as a degraded validator (syntax + tests pass, perf delta reported but not gated) in the first cut.

4. **4090 LoRA path (L4) is deferred across ALL domains, not just kernel — BUT Stage 2 is a modelless generalization path that must be exhausted first.** Bench 009 proved the modelless Stage 1 cache is 110,000× faster but 0% on held-out. **Stage 2 (the clippy_verify-backed adaptive drafter, Issue 010 fix direction) is the modelless generalization path**: the tuning loop re-runs the domain validator (e.g. `cargo clippy`, ~130 ms — fine for tuning) to build a pool of verified fixes, and the adaptive drafter serves those on subsequent hot-path calls. Per the modelless-first mandate, Stage 2 MUST be exhausted before L4. L4 (LoRA fallback, the "Expert 2" / Warm-Cold tier) requires GPU training (riir-train) + a real model (Ternary-Bonsai-27B or smaller) and is the conservative last resort for every domain. This proposal does NOT ship L4 for any domain; it ships the framework that L4 plugs into later. Each domain's L4 is a separate riir-train plan.

5. **Superseding Proposal 012's repo placement is a user-directed correction, not a re-evaluation.** Proposal 012 argued "new repo" with a reasoned boundary analysis. The user overrode it ("why it can't be clippy?"). This proposal does NOT re-argue the boundary — it cites the user's correction as the deciding factor. If a future re-evaluation finds that kernel_opt's CubeCL dep (behind `kernel_verify`) genuinely conflicts with riir-clippy's clean dep surface, the domain can still be extracted to a new repo later. The trait abstraction makes that extraction a move, not a rewrite.

6. **Domain corpora are deferred.** This proposal ships the FRAMEWORK (trait + domain-agnostic core + three domain scaffolds with structure but minimal corpus). Each domain's real corpus (clippy: already shipped; perf: rust-optimize skill encoding; kernel: popcorn distillation per Proposal 012 §Phase 0) is a follow-up plan. The GOAT gate for THIS proposal is "framework compiles + clippy domain still passes its existing tests + new domain scaffolds compile", not "domain X heals N% of held-out cases".

## Fusion lineage

1. **Proposal 034** (`riir-ai/.proposals/034_clippy_healing_drafter_pruner.md`) — the original single-domain pipeline. This proposal generalizes its vocabulary from clippy-specific to domain-pluggable.
2. **Proposal 012** (`katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md`) — the kernel-domain design (thermal-tier + latent-MoE + numeric validator). This proposal adopts its domain design unchanged, supersedes its repo placement, and absorbs it as the `kernel_opt` domain module.
3. **`HealConflictDetector`** (katgpt-core Issue 133) — the precedent for heal-path validation traits. The `HealDomain::validate` method mirrors its shape (domain-specific verdict type, stronger than "parses").
4. **`quest_grammar` 2-stage drafter** (`riir-ai/crates/riir-games-quest/src/quest_grammar/`) — the **Stage 1 + Stage 2 drafter pattern** this proposal adopts. `QuestGrammarPipeline` holds `plasma_draft` (Stage 1, `TernaryDraftModel`, always-on, ~µs) + `adaptive_drafter` (Stage 2, `AdaptiveQuestDrafter`, optional override loaded via `load_seal_corpus`). The `draft_text` routing function is the direct precedent for `draft_fix`. See [Issue 010](../.issues/010_drafter_cannot_generalize_heldout_code.md) §"Fix direction" for the riir-clippy mapping.

The combination produces: one pipeline (Stage 1 drafter + Stage 2 adaptive drafter + pruner + ruliology + self-evolve tuning loop) × N domains (clippy/perf/kernel/future) × M validators (per-domain ground truth, Stage 2 tuning-phase). None of the four alone gives this — Proposal 034 is single-domain single-stage, Proposal 012 is single-domain-new-repo, `HealConflictDetector` is validation-only, `quest_grammar` is single-domain (game quests) but ships the 2-stage pattern this proposal generalizes.

## GOAT gate

This is a **framework refactor**, not a perf-gain primitive. The GOAT gate is:

| Gate | Criterion |
|---|---|
| **G1 (correctness)** | clippy domain: all existing tests pass bit-identical after the refactor (the `ClippyCorpus` re-export preserves the old API). New domain scaffolds compile. |
| **G2 (perf)** | clippy domain: Stage 1 drafter latency unchanged (the rename + trait dispatch adds ≤1 virtual call; Bench 009's 17µs p50 must not regress >5%). Stage 2 adaptive drafter serve latency ≤ Stage 1 (hash lookup, no matvec). |
| **G3 (no-regression)** | `cargo test --all-features` passes. `./scripts/ci_feature_guard.sh` passes (the 5-layer CI guard). |
| **G4 (alloc-free)** | clippy domain hot path: 0 steady-state allocations (the trait dispatch + Stage 1→2 routing must not introduce boxing). Domain scaffolds: compile-only, no alloc gate yet. |
| **G5 (Stage 2 generalization, DEFERRED to Issue 010 follow-up)** | clippy domain: Stage 1 + identity guard + Stage 2 adaptive drafter must beat 0/10 on the held-out fixture (`benches/fixtures/e2e_cases_heldout.json`). This is the Issue 010 GOAT target — NOT gated by this framework proposal (which ships the container, not the Stage 2 pool). |

Promotion: `clippy_lints` stays default-on (it IS the current behavior under a new name). `rust_perf` + `kernel_heal` stay opt-in until their corpora + validators pass their own domain-specific GOAT gates (separate plans per domain).

## What ships now (riir-clippy) vs deferred

### Ships now — framework + clippy back-compat
- `HealDomain` trait (`src/domain.rs`)
- Domain-agnostic core rename: `Corpus<D>`, `TernaryDrafter` (Stage 1), `AdaptiveDrafter<D>` (Stage 2, quest_grammar precedent), `HealPipeline<D>` (back-compat re-exports preserve `ClippyCorpus` etc.)
- `domains/clippy_lints/` — current code reorganized behind `feature = "clippy_lints"` (default-on); `ClippyVerifier` is the Stage 2 tuning-phase validator
- `domains/rust_perf/` — scaffold: `RustPerfDomain` impl + minimal corpus (5–10 rules from rust-optimize skill) + `cargo test` validator (Stage 2; perf-bench validator deferred per caveat 3)
- `domains/kernel_opt/` — scaffold: `KernelOptDomain` impl + empty corpus + `pick_domain` router wiring + thermal-tier stub. Numeric validator behind `kernel_verify` (Stage 2; compile-only stub; full numeric compare deferred per Proposal 012 §Phase 0)
- **Stage 2 tuning loop scaffold** — the self-evolve loop gains a Stage 2 populate path (draft → `HealDomain::validate` → verified fix joins the `AdaptiveDrafter<D>` pool). The Stage 2 pool starts EMPTY; it fills as the tuning loop runs. The hot path routes Stage 1 → Stage 2 only on pool hits. (Full Stage 2 generalization — span-local AST edits, identity guard — is Issue 010's follow-up, NOT this framework proposal.)

### Deferred — per-domain follow-up plans
- **clippy corpus expansion** — already shipped; future lints added incrementally
- **rust_perf real corpus** — encode full rust-optimize skill checklist (~20 rules); perf-bench validator (Pareto verdict across input sizes); separate plan
- **kernel_opt corpus** — popcorn distillation (Proposal 012 §Phase 0, ~2 weeks reading job); full numeric validator; separate plan
- **L4 LoRA fallback (ALL domains)** — riir-train plan, 4090, only if modelless path plateaus below the domain's G1 gate; **DEFERRED** per modelless-first mandate + caveat 4

### Explicitly NOT shipped by this proposal
- Cross-domain routing (auto-detect domain from code shape) — caveat in §Proposed design
- A unified `cargo heal` CLI — each domain exposes its own entry point for now
- L4 LoRA for any domain — deferred (caveat 4)
- Real corpora for rust_perf / kernel_opt — framework ships scaffolds; corpora are follow-up plans

## Phased rollout (sketch — Plan 001 expands this)

### Phase 1 — Framework extraction (modelless, no new domains)
- Extract `HealDomain` trait
- Generalize `Corpus<D>`, rename `ClippyTernaryDrafter` → `TernaryDrafter`
- Back-compat re-exports (`ClippyCorpus` = `Corpus<ClippyLintDomain>` etc.)
- clippy domain reorganized under `domains/clippy_lints/` behind `feature = "clippy_lints"` (default-on)

### Phase 2 — rust_perf domain scaffold
- `RustPerfDomain: HealDomain` impl
- Minimal corpus (5–10 rules from rust-optimize skill)
- `cargo test` validator (perf-bench deferred per caveat 3)

### Phase 3 — kernel_opt domain scaffold
- `KernelOptDomain: HealDomain` impl
- `pick_domain` router wiring (Plan 558 substrate)
- Thermal-tier stub (FlashAR/QGF pattern)
- Compile-only validator behind `kernel_verify` (full numeric compare deferred)

### Phase 4 — GOAT gate + docs
- G1–G4 per the gate table above
- `.docs/` book update: domain-pluggable architecture section
- README update: three domains, feature matrix

### Phase 5 — L4 LoRA (DEFERRED, all domains)
- [-] riir-train plan, 4090, only if modelless path plateaus

## Risks

1. **Trait over-abstraction** (caveat 1) — the `HealDomain` trait might not fit a future domain. Mitigation: ship exactly three impls, document the signature as provisional.
2. **Validator cost asymmetry breaks the Stage 2 tuning loop economics** (caveat 2) — the tuning loop works at clippy speed (~130 ms/iterate) but not at kernel speed (minutes). Mitigation: each domain documents its Stage 2 validator latency; the tuning loop gets a per-domain iteration budget. The hot path is unaffected (Stage 1 ~µs always-on; Stage 2 serve is a hash lookup).
3. **rust_perf validator weakness** (caveat 3) — "faster" is workload-dependent. Mitigation: Pareto verdict across input sizes, not binary pass/fail; first cut ships degraded (syntax+tests only).
4. **Back-compat breakage** — the rename + trait extraction touches the public API. Mitigation: `pub use` re-exports preserve old names; G1 gate enforces existing tests pass bit-identical.
5. **Scope creep into cross-domain routing** — tempting to add "auto-detect domain" early. Mitigation: explicitly out of scope (§Proposed design); revisit only with a concrete consumer.

## Out of scope

- Cross-domain auto-routing (caller picks domain via feature flag)
- Unified CLI (`cargo heal`) — each domain has its own entry point
- L4 LoRA for any domain (deferred per caveat 4)
- Real rust_perf / kernel_opt corpora (follow-up plans per domain)
- A new repo for any domain (supersedes Proposal 012's "new repo" — same repo, domain as feature)
- Cross-domain corpus sharing (a clippy rule doesn't help a kernel heal; domains are isolated)

## References

1. **Proposal 034** (`riir-ai/.proposals/034_clippy_healing_drafter_pruner.md`) — the direct precedent. Same pipeline shape, single domain (clippy lints).
2. **Proposal 012** (`katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md`) — the kernel-domain design this proposal absorbs as `kernel_opt`. Its references (KernelBench, Sakana AI CUDA Engineer, AutoKernel, LatentMoE, popcorn, Bench 009) are the full prior-art survey — not duplicated here.
3. **`HealConflictDetector`** (katgpt-core Issue 133) — heal-path validation trait precedent.
4. **`pick_domain`** (katgpt-core Plan 558 / Research 453, arXiv:2601.18089 LatentMoE) — latent-MoE expert selection substrate.
5. **`rust-optimize` skill** (`~/.agents/skills/rust-optimize/SKILL.md`) — the L0 rule distillation for the `rust_perf` domain (its checklist maps 1:1 to corpus entries).
6. **Bench 009** (`riir-clippy/.benchmarks/009_bonsai_vs_modelless_e2e_tokps.md`) — the cache-vs-compute silent-miss lesson, cross-domain (Stage 1 scope-limit).
7. **riir-game-sdk Issue 002** — the facade re-export pattern this proposal uses for back-compat (`pub use` preserves old names after the underlying type moves).
8. **popcorn** ([`tilde-research/popcorn`](https://github.com/tilde-research/popcorn), upstream origin) — PyTorch kernel dispatcher, 96 kernels × ~100 impls × 7 backends. The L0 corpus source for the `kernel_opt` domain (distilled, not ported — see Proposal 012 §Phase 0). Referenced as the methodology source (reference-check + benchmark records + agent loop); the Python/Triton backend code is not portable to the Rust stack.
9. **Issue 010** (`riir-clippy/.issues/010_drafter_cannot_generalize_heldout_code.md`) — the L1 drafter 0/10 held-out root cause + the 2-stage drafter fix direction (Stage 1 scope-limit, Stage 2 clippy_verify-backed adaptive drafter). This proposal ships the container; Issue 010 ships the Stage 2 generalization.
10. **`quest_grammar` 2-stage drafter** (`riir-ai/crates/riir-games-quest/src/quest_grammar/plasma_draft.rs` + `mod.rs`) — `TernaryDraftModel` (Stage 1) + `AdaptiveQuestDrafter` (Stage 2) + `QuestGrammarPipeline::draft_text` (the routing function). The direct structural precedent for the 2-stage drafter this proposal adopts.

## TL;DR

**Verdict: ship — the multi-domain framework refactor generalizes the existing pipeline behind a `HealDomain` trait, adopts the `quest_grammar` 2-stage drafter pattern (Stage 1 = `TernaryDrafter` always-on ~µs; Stage 2 = `AdaptiveDrafter<D>` loaded by the clippy_verify-backed tuning loop, re-running the domain validator off the hot path), preserves clippy back-compat via re-exports, and scaffolds two new domains (rust_perf from the rust-optimize skill, kernel_opt absorbing Proposal 012). 4090 LoRA path deferred across all domains — Stage 2 is the modelless generalization path that must be exhausted first (Issue 010). Next action: execute [Plan 001](../.plans/001_multi_domain_framework_refactor.md).**
