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

**Status: not started.**

### Tasks

- [ ] **T1.1** Add `riir-rag` as a dep of `riir-clippy` (behind a `latent_retrieval`
      feature gate; pulls `AstChunker` + `ModellessEmbedder` + `RetrievalProvider`).
- [ ] **T1.2** `src/draft/corpus_index.rs` — index each domain's corpus entries as
      riir-rag chunks. The `ref_shape` text → `ModellessEmbedder.embed()` → `ShardIndex`.
      One index per domain (3 indices) OR one index with a domain tag.
- [ ] **T1.3** `src/draft/latent_matcher.rs` — given a real code chunk: `AstChunker.chunk()`
      → `ModellessEmbedder.embed()` → `LatentQuery::from_direction()` → `retrieve()`.
      Returns `Vec<(rule_id, latent_score)>` ranked.
- [ ] **T1.4** `src/draft/domain_router.rs` — `pick_domain::<3, 8>` with hand-crafted
      `DOMAIN_DIRECTIONS` (clippy/perf/kernel signatures). The 8-D activity = the code
      chunk embedding. Output: domain index 0/1/2.
- [ ] **T1.5** Compose: chunk → embed → route(domain) → retrieve(from that domain's index)
      → ranked rules. This is the `--suggest` pipeline.
- [ ] **T1.6** Tests: index a small fixture corpus, embed a matching code span, confirm
      the top-1 retrieved rule is the correct one (per domain).

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

## Out of scope (per Proposal 002 revised)

- Full proposer coverage (all 34 perf + ~30 kernel) — decoupled; `--suggest` covers them.
- CubeCL `#[cube]` macro-body proposers — `--suggest` still works (embedding captures the
  signature); only `--fix` is limited.
- L4 LoRA — deferred across all domains.
- `--watch` mode — deferred.
- Domain-direction mining via `CommittedFieldBlend` — hand-crafted vectors ship first.

## Risks

1. **Span-preserving patch is harder than it looks.** `syn` spans are byte-offsets; re-emitting only the changed span while keeping surrounding bytes intact needs care. Mitigation: `prettyplease` fallback (T1.3) ships first; span-preserving is the upgrade.
2. **Visitor false positives.** A `vec-with-capacity` visitor might match a vec that's intentionally un-sized. Mitigation: visitors are conservative (only rewrite when the size is statically known); uncertain cases go to `--suggest`.
3. **CubeCL parse failure.** A real riir-gpu file with `#[cube]` macros won't parse with vanilla `syn`. Mitigation: the scanner skips files that fail to parse (with a warning); proposers run only on parseable files. Kernel proposers are tested on EXTRACTED snippets, not whole kernel files.
4. **G5 might fail.** If no real anti-pattern exists in the sibling repos for the 10 shipped rules, G5 can't pass. Mitigation: the rule selection (Phase 2/3 candidates) is chosen BECAUSE the patterns are common; if G5 fails, expand the rule set or target a different repo.

## Numbering discipline

- Proposal 002: `riir-clippy/.proposals/002_real_file_span_healing.md`
- Plan 005: this file.
- Highwater: `.plans/.highwater` = `5`, `.proposals/.highwater` = `2` (written back after this plan lands).
