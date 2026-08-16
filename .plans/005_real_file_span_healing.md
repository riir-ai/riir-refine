# Plan 005 — Real-file span-level healing for `rust_perf` + `kernel_opt`

**Proposal:** [002](../.proposals/002_real_file_span_healing.md)
**Branch:** `develop` (per global rule — no feature branches)
**Status:** Phases 1-4, 6 SHIPPED (**Phase 3 CLOSED 2026-08-16** — G1 re-met
after the full T3.5 contingency, Bench 027) · Phase 5 DEFERRED ·
Phase 7 partially shipped (Issue 018) · Phase 8 BLOCKED (rubrc) ·
Phases 9-14 not started
**Last reviewed:** 2026-08-16 — see [Issue 023](../.issues/023_plan005_design_question_poc.md)
for the POC that settled the Phase 9 and Phase 11 design questions.

> **Read this first if you are picking the plan up.** Three things are not
> where the phase bodies below imply:
> 1. **Phase 3 is CLOSED** (2026-08-16). Its G1 died at scale (58.3%) and was
>    recovered by the full T3.5 contingency to **87.5%** on the committed eval
>    and **96.0%** on an uncontaminated composite set. Quote the second number:
>    the committed clippy rows are inflated by the fixture's single-lint shape.
> 2. **Phase 7 is partially shipped** via Issue 018 — the outcome recorder and
>    cross-run persistence already exist. Do not rebuild them.
> 3. **Phase 11's latency justification is dead** (measured 1.07× ceiling vs a
>    1.5× gate, Issue 023). It survives as a quality-only phase, demoted below
>    Phase 12 and dependent on Phase 10 T10.1.

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

**Status: DONE 2026-08-14.** Smoke-verified on this repo's own `src/` (19
files, ~0.4s total including index build). Bin name `cargo-heal` (cargo
subcommand convention → `cargo heal` once installed). No-threshold note:
every span prints suggestions (scores are relative); a minimum-score
cutoff is a Phase 3 G1 concern — the precision measurement drives it.
**Follow-up (same day, Phase 3 addendum): the cutoff was measured and
REFUTED** — see Phase 3 + Bench 018 addendum. Every-span output is the
permanent shape of `--suggest`.

### Tasks

- [x] **T2.1** `src/bin/heal.rs` — args: `--suggest`, `--domain <auto|clippy|perf|kernel>`
      (default `auto` = dMoE routing), `[path...]`. + `--top-k <N>` (default 3).
- [x] **T2.2** File scanner: walk `path`, `AstChunker.chunk()` each `.rs` file.
      Std-only recursive walker (no new dep) — skips hidden/`target`/`vendor`.
- [x] **T2.3** For each chunk: embed → route(domain) → retrieve → print top-K rules with
      scores + the `opt_pattern` snippet for manual review.
      Fan-out across enabled domains (Phase 1 deviation); router verdict
      annotates each span. Snippet = first significant line of the rule's
      `fixed_code` (via the new `LatentMatcher::rule_book`).
- [x] **T2.4** Domain gating: `--domain perf` requires `--features rust_perf`; etc.
      `--domain auto` runs all enabled feature-gated domains via the dMoE router.
      Explicitly requesting a feature-off domain is a hard error with the
      enabled list in the message (verified on `--domain kernel` without
      `kernel_heal`).

### Phase 2 exit criteria
- `cargo heal --suggest src/` prints real suggestions on real files. The product exists
  (suggest mode). `--fix` is Phase 4/5.
  **MET 2026-08-14** — smoke run on this repo's `src/`: real spans with scores
  + fixed-pattern snippets, ~0.4s wall for 19 files incl. index build.

## Phase 3 — GOAT gate for the latent path (G1-G4, `--suggest` mode)

Measure the retrieval quality + latency before investing in proposers.

> **REOPENED 2026-08-16 — G1 IS DEAD AT SCALE.** Issue 017 / Bench 022
> re-measured top-1 retrieval precision on **130 spans against the full
> 40-rule corpus** (vs the 12- then 24-item hand-built fixture below) and
> got **clippy 58.3%** — under this phase's own ≥80% bar. Perf holds at
> **87.5%**; kernel self-variant probes are 100% but are a weaker test
> (self-variants, not independent instantiations). Root cause is
> nearest-neighbour crowding concentrated in tiny-token patterns
> (`identity_op` alone tops 5 misses) — the same text-similarity-vs-rule-
> instantiation limit that refuted the min-score threshold below.
> `AGENTS.md` records the claim as "dead at scale".
>
> **This phase's own contingency is therefore live and MUST be executed
> before Phase 9+ starts:** *"If G1 (precision) is <80%, tune the domain
> directions or switch to `BonsaiEmbedder` (768-D, opt-in)."* Tracked as
> **T3.5** below. G2/G3/G4 are unaffected and remain PASS.

**Status (superseded — kept as the record of the fixture-scale gate):
DONE 2026-08-14 — ALL FOUR GATES PASS** (Bench 018):
G1 100% (12/12, target ≥80%), G2 349µs/chunk (target <1ms, 2.9× headroom),
G3 104 lib tests, G4 scratch stable. Notable: the first G1 run FAILED at
75% — audit showed the FIXTURE mis-instantiated 3 rules (method-closure vs
fn-closure, value-borrow vs ref-borrow, iterator-chain vs canonical
`.into()`); faithful rewritten variants scored 12/12. Fixture-faithfulness
audit precedes retriever tuning (recorded in Bench 018).

**Follow-up (2026-08-14, same day): minimum-score threshold REFUTED.** The
Bench 018 follow-up list called for threshold calibration via a
negative-example fixture. The fixture shipped (14 adversarial clean snippets
— the CORRECT version of each corpus anti-pattern — in
`tests/latent_suggest_goat.rs::NEGATIVE_FIXTURE`) + two calibration tests
as the permanent instrument. Measured: positives [3.19 ..= 34.71] vs
negatives [6.08 ..= 69.37] — **OVERLAP**; the correct `Vec::with_capacity`
fn scores 34.16 on `vec-with-capacity` (higher than the positive's 33.57);
whole-span-only ≡ multi-scale (the line-accumulation hypothesis refuted
too). Root cause: the fused latent+BM25 score is text similarity, not rule
instantiation — a one-token difference (`Vec::new()` vs `Vec::with_capacity`)
is invisible. No threshold shipped; `--suggest` stays a ranked review
queue + the proposers stay the only instantiation gate. Full record: Bench
018 addendum.

### Tasks

- [x] **T3.1 (G1 retrieval precision)** On a fixture of known anti-patterns, the top-1
      retrieved rule is correct ≥80% of the time (per domain). Record in `.benchmarks/`.
      Bench 018: 12/12 = 100% (6 clippy + 6 perf renamed-identifier variants).
      **Expanded 12 → 24 (2026-08-14, Bench 018 Addendum 2): 23/24 = 96%** — all
      10 clippy rules + 14 perf rules covered; single miss = `retain-inplace` →
      `collect-avoidance` (near-miss sibling pair, same text-similarity root
      cause as the threshold refutation). Gate still PASS **on the fixture**.
      **SUPERSEDED at scale (2026-08-16, Issue 017 / Bench 022): clippy 58.3%
      top-1 on 48 spans / 40-rule corpus — FAIL vs the ≥80% bar.** Perf 87.5%
      PASS; kernel self-variants 100% (weaker test). The 24-item fixture was
      too small and too clean to expose nearest-neighbour crowding.
- [x] **T3.2 (G2 latency)** Chunk + embed + route + retrieve < 1ms per chunk (Clifford
      wedge KNN is O(log N); Bench 575 measured 123µs p50 @ N=1000).
      Measured 349µs/chunk end-to-end (multi-scale + 2-domain fan-out), release.
- [x] **T3.3 (G3 no-regression)** All existing riir-clippy tests pass.
      104 lib tests green (91 pre-existing + 13 draft).
- [x] **T3.4 (G4 alloc-free hot path)** Reuse scratch buffers in the retrieve loop.
      `SuggestScratch` threaded through `suggest_*_into` (split-field borrow);
      capacities stable across 50 repeat scans. Remaining allocs bounded +
      documented (Bench 018 §G4).

- [x] **T3.5 (G1 recovery — the live contingency, blocks Phase 9+)** Lift clippy
      top-1 to ≥80%. **DONE 2026-08-16 (Bench 027) — the bar is MET on both a
      contaminated and an uncontaminated measurement.** Clippy top-1 54.2% →
      70.8% (tokenizer) → **87.5%** on the committed eval, and **96.0%** on the
      uncontaminated composite set; perf 87.5% → 97.9%; blended 70.8% → 92.7%.** (Baseline reads 54.2% not Bench 022's 58.3% because the
      fixture grew 88 → 96 labeled spans in between; all T3.5 numbers use the
      96-span fixture throughout.)

      **The route mattered as much as the number.** T3.5.4's re-ranker appeared
      to give +25pp; that was proven *circular*. T3.5.5 built an independent
      real-`cargo clippy` oracle, which showed the labels were fine but the
      *sample* was degenerate (0/43 multi-lint), authored the missing
      multi-lint spans, and re-measured a real **+12.0pp**. Both the
      contaminated eval (87.5%) and the honest one (96.0%) clear the 80% bar,
      and the honest one clears it by more.

  - [x] **T3.5.1 Re-fit the domain centroids — REFUTED BY ARCHITECTURE, no
        change shipped.** Two independent reasons this option could not work,
        both verified rather than argued:
        (a) **It cannot move G1 at all.** Retrieval is fan-out and the router
        is annotation-only — `run_eval` ranks within the span's *own* domain
        (`retrieval_eval.rs:441`) and the router verdict feeds only
        `router_correct`. Centroid quality cannot change any top-1/3/5 number
        by construction. (Bench 022 said the same from the miss side: "0
        misses caused" by wrong-domain routing.)
        (b) **The centroids were never stale.** `DomainRouter::with_embedding`
        recomputes each centroid from the *live* corpus at every construction;
        there is no cached centroid file, so "re-fit after the 10 → 40 growth"
        was already in effect on every run. The premise was wrong.
        The fit space was also already swept by Issue 020 (Bench 025, still
        re-runnable): best available is dft8-global at 75.0%, under the 85%
        gating bar, so gating stays killed and the router stays an annotation.
  - [x] **T3.5.2 Attack tiny-token crowding — ROOT-CAUSED, FIXED, PROMOTED.**
        The whole gain came from here. Root cause: the substrate default
        `CodeTokenizer` splits on `!alphanumeric && != '_'` and **discards all
        punctuation** — but the clippy corpus is 40 minimal generic functions
        whose entire signal IS punctuation (`identity_op` = `* 1`,
        `unused_unit` = `-> ()`, `deref_addrof` = `&*`, `double_parens` = `((`).
        Stripped of it, every rule collapses to the same `fn f x` bag and the
        most generic one wins — `identity_op` topped 7 of the 22 misses. This
        also explains why perf/kernel never had the problem (identifier-rich).
        Fix: `src/draft/op_tokenizer.rs` — identifier tokens byte-identical to
        `CodeTokenizer` (pinned by test, so identifier-rich domains cannot
        regress) plus **one token per maximal punctuation run**, plus
        operator-adjacency bigrams (needed because `op:()` is ubiquitous while
        `op:->|op:()` is rare). Runs rather than a hand-picked operator table
        so **BM25's own IDF** decides what discriminates — no tuned weights.
        Injected via `NeuronDbRetriever::with_bm25_tokenizer`, a designed
        substrate hook; zero substrate code reimplemented.
        **Secondary REFUTATION:** the latent fusion weight is an inert lever —
        clippy top-1 is identical at 1.0/0.5/0.25/0.0, because BM25 dominates
        and latent-only hits never win. `DEFAULT_LATENT_WEIGHT` stays at 1.0
        so the promotion carries no gratuitous behaviour change.
  - [x] **T3.5.3 `BonsaiEmbedder` 768-D — MEASURED, REJECTED ON COST.**
        The option as written was a no-op: `BonsaiEmbedder::embed` (the
        `Embedder` trait) returns **8-D**, i.e. exactly the term T3.5.2 proved
        inert; the 768-D vector is on the separate `DenseEmbedder` trait. Wired
        the real dense-rerank path instead (`bonsai_retrieval` feature,
        `RetrievalConfig::dense_weight`, `index_chunks_dense` + `rerank_dense`).
        **Cost probe: 166 s model load, 16.6 s per short embed, 48.1 s per long
        embed** — the full eval needs ~1018 embeds/config = **~13.6 h/config**,
        ~41 h for a 3-weight sweep, on CPU. There is no Metal path for
        `BonsaiEmbedder`; adding one is a riir-ai project.
        Measured on the largest matched subset that is feasible instead
        (clippy only, whole-span scale on both arms, dense vectors cached once
        so the weight sweep is free) — `tests/bonsai_retrieval_probe.rs`.
  - [x] **T3.5.4 Instantiation re-ranker — BUILT + BENCHMARKED; gain proven
        UNMEASURABLE on the then-current oracle. (Promotion followed in
        T3.5.5, once an independent oracle existed.)** Shipped as
        `src/draft/instantiation_rerank.rs` behind `RerankMode` (default
        `Off`): retrieve a pool of 8 → run the candidate rules' `propose_fix`
        visitors over the span (one parse, K visitor walks) → a firing visitor
        outranks a non-firing one → truncate. This is rule *instantiation*
        rather than text similarity — the exact distinction Bench 018's
        threshold refutation named as the ceiling.
        **Measured: clippy top-1 70.8% → 87.5% (`Structural`) → 95.8%
        (`All`), blended 84.4% → 92.7% → 96.9%. These are NOT effect sizes.**
        `clippy_fixture_labels_verify_via_proposers` (committed, passing)
        asserts every clippy fixture fires its OWN proposer and **zero** of the
        other 40 cross-fire — precisely the property the re-ranker exploits, so
        the oracle that defined the labels is the oracle doing the ranking.
        `rerank_circularity_diagnostic` states the falsifiable version and it
        holds **exactly**: `All` top-1 = pool recall @8 = 95.8% = 46/48. The
        table measures "was the right rule retrieved", not "did re-ranking pick
        it". Per the repo GOAT rule (a gain on a biased oracle is not a
        modelless gain) the gate did NOT pass, so the default stays `Off` —
        guarded by an assertion so the table cannot be acted on by itself.
        **Established non-circularly:** safety (`clean_spans_fire_no_lint` — the
        structural proposers do not fire on 12 real production excerpts, an
        oracle unrelated to the fixtures), coherence (`--fix` already gates on
        the same visitor, Phase 6 T6.1), and cost (`Off` short-circuits before
        the parse, so the shipped path pays nothing).
  - [x] **T3.5.5 Uncontaminated G1 oracle — BUILT; GOAT PASS; re-ranker
        PROMOTED.** `src/draft/clippy_oracle.rs` asks real `cargo clippy`
        (all three lint groups, JSON, with compile-failure detection) what
        fires on a span. It knows nothing about our corpus or proposers.
        **It overturned half of T3.5.4's diagnosis:** the labels were never
        wrong — real clippy confirms **43/43** of the compiling fixtures. The
        defect is **sample selection**: **0/43** trip more than one corpus
        lint, so exactly one rule can possibly be right and a ranker has
        nothing to decide. Re-running G1 against real labels reproduced the
        degeneracy exactly (`all` top-1 97.7% == pool recall), proving the
        fixture *shape* was the blocker, not the labels.
        Fixed by authoring the missing sample:
        `src/draft/eval_fixtures_composite.rs`, 28 spans tripping several
        anti-patterns at once, **labeled by the oracle rather than by hand**
        (hand-labeling would restore the coupling). 27 compile, 25 are
        genuinely multi-lint; a second batch pairs non-`return` rules so the
        result is not an artifact of the first batch's `needless_return` bias.
        **Uncontaminated G1: off 84.0% → `Structural` 96.0% (+12.0pp)**, and
        the degeneracy check now fails to fire — `all` top-1 96.0% ≠ pool
        recall 100.0%, i.e. top-1 is an ordering decision retrieval did not
        make. G2 +52-64µs/span (+5.6-7.2%); G3 415/461/481/471 lib tests green
        across four feature states, clippy 0; G4 one `syn::File` per span;
        modelless throughout. **→ `RerankMode::Structural` promoted to
        default**, pinned by assertion.
        Side finding recorded, not fixed: 5 of the 48 original fixtures do not
        compile, so no independent oracle can validate them.

### Phase 3 exit criteria
- G1-G4 PASS for `--suggest` mode. If G1 (precision) is <80%, tune the domain directions
  or switch to `BonsaiEmbedder` (768-D, opt-in) before proceeding to proposers.
  **MET 2026-08-14 on the 24-item fixture (Bench 018); NOT MET at scale
  (2026-08-16, Bench 022 — clippy 58.3%); RE-MET 2026-08-16 after the full
  T3.5 contingency (Bench 027).** Final: clippy top-1 **87.5%** on the
  committed eval and **96.0%** on the uncontaminated composite set — both over
  the ≥80% bar — with perf 97.9% and blended 92.7%. G2/G3/G4 remain PASS.
- **PHASE 3 CLOSES.** The "blocks Phase 9+" clause is discharged: the
  retriever is measured at scale, the measurement is committed as a regression
  gate, and the one gate that was failing now passes on evidence independent
  of our own proposers.
- Two honest caveats carried forward, neither blocking: (a) the committed
  clippy rows are inflated by the fixture's single-lint shape and are a
  regression floor, not an effect size — the uncontaminated composite set is
  the one to quote; (b) 5 of the 48 original fixtures do not compile, so no
  independent oracle can validate them.

## Phase 4 — Top-5 perf proposers (the `--fix` upgrade for perf)

Proposers enable `--fix` (auto-apply). Start with perf (clippy already has 10).

**Status: DONE 2026-08-14.** `src/domains/rust_perf/proposers/` — the 5
proposers (`vec-with-capacity`, `entry-vs-contains-insert`, `static-str`,
`bool-to-bitpack`, `fxhashmap`) + the `propose_fix` dispatch + 36 fixture
tests (140 lib total, clippy-clean). Design notes:
- **Capacity-source safety** (`vec-with-capacity`): only unambiguous
  capacity sources rewrite — `0..n` ranges, `0..=n` (+1), `&path`, and
  `path.iter()/iter_mut()`. Bare-path iterables (`for x in items`) are
  SKIPPED: the binding could be an iterator without `.len()`, and the
  parse-only pruner can't catch the broken code a wrong `.len()` emits.
- **Statement-level rewrite** (`entry-vs-contains-insert`): the if-stmt
  produces `()`; replacing it with the semicolon'd
  `map.entry(k).or_insert(v);` preserves that — the bare method call in
  value position would be `&mut V` (type change) and wouldn't re-parse
  mid-block without the semi. Walks ALL stmts file-wide (match arms,
  closures, loop bodies), not just fn-top-level.
- **Signature-visible changes documented per-rule** (`static-str` return
  type, `bool-to-bitpack` param type — same exposure class as the clippy
  proposers' rewrites; `rust_perf_verify` is the semantic gate).
- **Use-group surgery** (`fxhashmap`): plain use swapped in place; group
  uses slimmed (HashSet siblings preserved) + separate
  `use rustc_hash::FxHashMap;` appended; glob/`pub use`/fn-local forms
  skipped (public-semantics or sibling-item surgery hazards).
- `for_each_fn_mut` shared helper covers free fns + impl methods +
  nested inline `mod` blocks (beyond the clippy proposers' file-level scan).

### Tasks

- [x] **T4.1-T4.7** The 5 perf proposers (`vec-with-capacity`, `entry-vs-contains-insert`,
      `static-str`, `bool-to-bitpack`, `fxhashmap`) + `propose_fix` dispatch + fixture tests.
      (See original Plan 005 Phase 2 for details — unchanged.)

## Phase 5 — Top-5 kernel proposers (the `--fix` upgrade for kernel)

**Status: DEFERRED 2026-08-14 — corpus is concept-grade, not proposer-grade.**
Scope audit (this session): only 8/164 kernel rules even mention real API
symbols (`CubeDim`/`CubeCount`/`launch`/`client`/`Handle`/...) in their
`ref_shape`s — and all 8 need kernel-body knowledge (inside `#[cube]`, which
syn cannot parse — Proposal 002 caveat 1) or cross-call context (framework
buffer pools, queue-ordering proofs, graph capture). The other 156 are
pseudo-code concepts with imaginary helpers (`gemv_dispatch_sync`,
`empty_buf`) — retrieval-grade knowledge, not rewrite targets.

The kernel corpus was built for the LATENT path (`--suggest`, Bench 018 G1
100% — embedding similarity over concepts works); proposers require
syntactic determinism that only the perf/clippy corpora provide (exact
buggy→fixed idiom pairs).

**Reopen trigger:** mine ≥5 real outer-Rust launch/dispatch/layout rules from
riir-gpu with literal buggy→fixed code pairs (like Batch 14+ did for
concepts, but proposer-grade). Until then `--fix` covers perf + the clippy
structural set (T6.4) and kernel stays suggest-only.

### Tasks
- [-] **T5.1-T5.6** The 5 kernel proposers (launch/dispatch/layout only, outside `#[cube]`).
      **DEFERRED** — no honest 5 exist in the current corpus (see status).
      (See original Plan 005 Phase 3 — unchanged.)

## Phase 6 — `cargo heal --fix` + final GOAT gate (G5 real-file value)

**Status: T6.1 DONE 2026-08-14 (Bench 019); T6.4 DONE 2026-08-14; T6.2/T6.3 DONE 2026-08-14 (Bench 019 G5 addendum).**
The `--fix` wiring shipped with the **span-preserving patch emitter** (the
proposal's "honest production choice", not the prettyplease MVP): the 5
rust_perf proposers were refactored from AST-mutation to edit-collecting
(`SourceEdit` byte ranges via `proc-macro2` span-locations; replacement
text built from the matched nodes' ORIGINAL source), and `apply_edits`
splices them back-to-front. Everything outside a matched span stays
byte-identical — verified on a real 700-line file (1 line changed).

**T6.4** migrated the 10 clippy_lints proposers to the same emitter
(shared substrate extracted to `src/edit_emitter.rs`, feature-neutral):
`propose_fix` keeps its signature for `self_evolve`, but now
parse → collect_edits → splice instead of mutate → `ToTokens`
re-emission (the old path destroyed the caller's formatting). `--fix`
now also runs the **structural** clippy lints (detection alone guarantees
semantics: needless_return, redundant_field_names,
manual_map, unnecessary_unwrap, needless_range_loop, needless_lifetimes —
6 as of the T6.2 G5 run, which demoted redundant_closure after it caught
an E0631 fn-path coercion break); the **type-blind** ones
(needless_borrow, useless_conversion, unnecessary_mut_passed,
redundant_closure) stay
self_evolve-only (their rewrites are only correct when types line up;
parse-only validation can't check). The migration also fixed four
latent correctness bugs in the mutation-based detection (see T6.4
notes). Fix application iterates to a bounded fixpoint so nested fixes
compose (`return Cfg { x: x };` → `Cfg { x }` in two passes).

The proposer's visitor is the `--fix` gate (caveat 4): latent suggestions
never auto-apply; retrieval reports/prioritizes. The latent-score
threshold from the original T6.1 text is therefore NOT used to gate
application — the visitor IS the confirmation. `--fix [--write] [--quiet]`
composes with the suggest pass.

T6.2 is **DONE (2026-08-14, Bench 019 G5 addendum)**: the gate landed in
**katgpt-rs** (`katgpt-pruners`, commit `8c331cd9`, pushed) — riir-ai
remained sibling-active (dirty riir-net/qvu_lora/cudarc WIP) and
katgpt-rs is a sibling repo per this repo's layout, clean + quiet. The
real-file run CAUGHT three latent proposer bug classes (use-before-decl
capacity, non-usize capacity bounds, fn-path coercion in
redundant_closure) — all fixed at root cause in the same session, the
gated re-run compiled + passed 1233 tests in one pass. `redundant_closure`
was demoted from `FIXABLE_LINTS` (7 → 6 structural lints).

### Tasks

- [x] **T6.1** Wire `--fix` mode: for each retrieved rule, if a proposer exists + latent
      score > threshold (0.8), apply the rewrite (span-preserving); else fall back to
      `--suggest` output for that rule.
      **DONE with a design correction** — the proposer visitor (not a latent-score
      threshold) is the application gate (caveat 4); the span-preserving emitter
      shipped (Bench 019 G1-G4 PASS; G5 landed in the same bench's addendum).
- [x] **T6.2 (G5 real-file value)** At least ONE real fix lands in a sibling repo
      (riir-ai or riir-gpu): the latent matcher detects a real anti-pattern, the proposer
      rewrites it, the change compiles + tests pass.
      **DONE 2026-08-14** — landed in **katgpt-rs** instead (riir-ai sibling-active;
      katgpt-rs is a sibling repo, clean + quiet): `katgpt-pruners` 12 files / 20 edits,
      commit `8c331cd9` (pushed). Run 1 caught 6 compile errors across 4 files →
      3 proposer bug classes fixed at root cause (scope gate, usize gate,
      redundant_closure demotion) → run 2 compiled + passed 1233/1233 tests.
      See Bench 019 §G5 addendum.
- [x] **T6.3** `.benchmarks/016_real_file_heal_goat.md` — full G1-G5 measurement.
      Renumbered to **Bench 019** (016 was taken by the rust_perf v2 corpus batch);
      G1-G4 recorded there, **G5 recorded in the §G5 addendum (2026-08-14): PASS**.
- [x] **T6.4** Migrate the clippy_lints proposers to the edit-splice emitter
      (extends `--fix` to the clippy domain; preserves `propose_fix` for `self_evolve`).
      **DONE 2026-08-14.** Shared emitter extracted to `src/edit_emitter.rs`
      (feature-neutral; rust_perf re-exports preserve the old import path).
      All 10 proposers migrated to immutable-walk + `SourceEdit` collection;
      `FIXABLE_LINTS` = the structural rules (`--fix` eligible; 7 at T6.4
      time, 6 after the T6.2 G5 run demoted `redundant_closure`), the
      type-blind ones stay `self_evolve`-only. Fix application iterates to a bounded fixpoint
      (4 passes + unchanged-output guard) so nested fixes compose.
      Four latent detection bugs fixed by the migration (found by real-file review
      + type-checking the spliced output):
      1. `needless_lifetimes` — old rule ("one declared lifetime") was unsound:
        elided `&` inputs are input lifetimes too, and `'a` inside `FnMut(&'a S)`
        args / `Index<'a>` / `impl T + 'a` / receivers dangles. Now guarded by a
        single elision walker (exactly ONE input ref slot total; target only in
        strippable `&`-annotation positions).
      2. `needless_range_loop` — emitted `&v` unconditionally: for `v: &Vec<_>`
        that iterates `&&Vec` (NOT an iterator). Now resolves the collection
        against fn params (ref param → `v`, owned → `&v`, unresolvable → skip)
        + blocks slot writes (`v[i] = x`, `v[i] += x`) and address-of (`&v[i]`).
      3. `needless_range_loop` — token-substring usage counting false-positived
        on `v[v[i]]` (rewrote to `v[__elem]`) and false-negatived on `minimum`
        containing `i`. Replaced by exact `Expr::Index`/path analysis.
      4. `unnecessary_unwrap` — didn't check that the `unwrap` receiver matches
        the `is_some` receiver (`if a.is_some() { b.unwrap() }` → `a.unwrap_or`).
        Now receiver-text-matched.

      **Post-landing polish (same day):** per-edit review line numbers now
      reference the FINAL written source, not per-pass intermediates — each
      pass records its edits in input coordinates + `apply_edits_traced`
      returns the splices actually applied, then `map_through_splices` remaps
      the recorded positions through every pass's applied splices (own pass
      included — safe by the `start >= p` break) before line-counting in the
      final source. Without this, edits before a line-count-changing fix
      (manual_map's multi-line collapse, fxhashmap's inserted `use`) reported
      drifted lines. Same change fixed duplicate reporting: pass-1 edits
      dropped by overlap selection but re-applied on pass 2 were being
      counted twice; the bin now reports only edits in the applied-splices
      set. 4 new emitter tests (incl. the deletion-then-insertion coordinate
      regression that caught the first `map_through_splices` draft's frame
      bug + a two-pass composition case).

> **Count refresh (2026-08-16).** The narrative above records the state at
> T6.4 landing. Since then Issue 015 shipped `--fix --verify` (the
> `fix_verify` feature: apply → `cargo check --all-targets` → revert only
> the offending edits → re-check) and Issue 016 expanded the structural set.
> Current shipped surface, verified against source:
> `FIXABLE_LINTS` = **33** (the 6 seed + 27 from Issue 016,
> `src/domains/clippy_lints/proposers/mod.rs:174`), `VERIFY_LINTS` = **4**
> (`redundant_closure` restored here under the compile gate), plus the **5**
> rust_perf proposers. Read "6 structural lints" and "10 shipped proposers"
> below as historical, not current.

### Phase 6 exit criteria
- G5 PASS. `cargo heal --fix` is production-usable for the shipped proposers
  (**33 structural clippy lints + 4 verify-tier + 5 perf**, as of 2026-08-16) +
  `--suggest` covers the rest of the corpus.
- **Public distribution (if this milestone ships to users) is BINARY-ONLY via a
  separate public dist repo** — source stays private per Research 003. See
  [Proposal 003](../.proposals/003_binary_only_distribution.md) (brew tap /
  curl|sh / install.ps1 / scoop; never `cargo publish`, never source in the
  release assets).

## Phase 7 — Self-healing closed loop (MAG online EMA + LatentFixMemory)

The fusion layer from Proposal 002 §"Fusion layer". The healer improves its own
retrieval + routing from every fix outcome. **All substrate ships** (`LatentSkillEvolution`
+ `LatentFixMemory` in riir-clippy Issue 004/008); this phase generalizes the key from
per-lint to per-domain + wires the outcome recorder into the `--fix` path.

**Status: PARTIALLY SHIPPED (corrected 2026-08-16) — do not rebuild the
recorder.** Issue 018 already landed the outcome-recording half and cross-run
persistence, via `src/recorder.rs` (`SelfEvolveRecorder`) rather than the
`HealPipeline::heal` entry point this phase assumed:
`src/bin/heal.rs:637,659` calls `evolve_rec.record_outcome(...)` on both
reverted and kept edits, and `:672` saves to `memory.json` (path overridable
via `RIIR_HEAL_MEMORY`). Issue 018 also promoted the memory half to
unconditional, with the EMA half degrading gracefully (one log line) when the
`clippy_verify` verdict oracle is absent. Issue 021 added the `EvidenceTier`
trust axis to `LatentFixMemory`.

**What actually remains** is the *domain-level* half — the recorded outcomes
currently evolve **per-lint** directions, not the `DOMAIN_DIRECTIONS` the dMoE
router reads, so the router does not yet get smarter with use:
`src/evolve.rs:67` is still `directions: HashMap<String, Direction>` and
`:132` is still `record_outcome(&mut self, lint_name: &str, ...)`.

### Tasks

- [x] **T7.1 Domain-keyed evolution — ALREADY SUPPORTED; MEASURED; REFUTED as
      a router improvement.** The "generalization" turned out to be a no-op:
      `LatentSkillEvolution` stores `HashMap<String, Direction>` and
      `record_outcome` takes `&str`, so a domain key works today — what was
      missing was a *consumer*, not a key shape.
      Benchmarked before wiring one (`tests/t71_evolved_directions.rs`), both
      arms trained on the same signal (98 corpus patterns) and scored on 96
      spans neither saw:
      | estimator | routing accuracy |
      |---|---:|
      | corpus centroid (shipped) | **53.1%** |
      | EMA-evolved, domain-keyed | 52.1% (**−1.0pp**) |
      **Evolution does not beat the centroid**, and the centroid reproduces
      Issue 020's baseline-8d 53.1% exactly (harness cross-check). The reason
      is structural: for cosine routing the L2-normalized class mean IS the
      least-squares optimum, so an online EMA over the same distribution is a
      *noisier estimate of the same quantity*, never a better one. Its only
      theoretical value is adapting to distribution SHIFT (a user codebase
      unlike the corpus), which a static fixture set cannot measure.
- [x] **T7.2 (recording half SHIPPED via Issue 018; router half REFUTED on
      two stale premises).** Recording is already wired into `--fix` through
      `SelfEvolveRecorder` (`src/bin/heal.rs:637,659`), not `HealPipeline::heal`.
      The remaining "evolve the DOMAIN_DIRECTIONS" half rests on two premises
      that no longer hold:
      1. **"the dMoE router gets smarter"** — the router cannot affect G1 at
         all; retrieval is fan-out and the verdict is display-only
         (`heal.rs:241`), established in T3.5.1. It can only move a decorative
         annotation metric.
      2. **"the DOMAIN_DIRECTIONS used by `pick_domain<3,8>`"** — Issue 020
         replaced the 8-D `pick_domain` baseline with `TfCosine` 128-D
         centroids (68.8% vs 53.1%). The 8-D directions this task would evolve
         are **not on the shipped router path at all**.
      Combined with T7.1's measurement (evolution −1.0pp vs centroid, both far
      under the 85% gating bar), there is nothing here worth wiring. The
      per-lint EMA that DOES work stays as Issue 018 shipped it (Bench 024 G7:
      50% → 100% on real cargo clippy over the 40-lint corpus).
- [x] **T7.3 Trajectory-memory retrieval axis — MEASURED, REFUTED, NOT BUILT.**
      Probed before wiring (`tests/t73_memory_axis_headroom.rs`), with the
      populate/evaluate split held out to avoid repeating the T3.5.4 mistake:
      memory populated from the 48 single-lint fixtures (48 CERTIFIED
      trajectories — the strongest tier, so the axis is measured at its best),
      evaluated on the 25 held-out multi-lint composites.
      **Result: zero headroom and real risk.**
      - shipped ranking already correct: **24/25 (96.0%)**
      - cases memory COULD fix: **0/25 (0.0%)**
      - cases memory would OVERRIDE a correct top-1 with a false rule:
        **19/25 (76.0%)**
      There is not a single ranking on held-out spans that a memory axis could
      improve, and giving it authority would break three quarters of the ones
      that are already right — memory is lint-keyed, so it returns a hit for
      any rule with history regardless of span relevance.
      **T7.3 was refuted by the SUCCESS of T3.5.5**: shipping a strong
      evidence axis (proposer instantiation, 96.0% top-1) consumed the
      headroom a weaker one would have needed. Reopen only if the instantiation
      axis is removed or retrieval precision regresses materially.
- [ ] **T7.4** The freeze/thaw: evolved directions + memory persist across runs (via
      `freeze()`/`thaw()` on `Corpus` + the existing BLAKE3 commitment). The healer's
      learned directions are reproducible + auditable.
      **Partially shipped:** cross-run persistence exists as a plain JSON
      save/load (`memory.json`, Issue 018). What remains is the *committed*
      form — routing it through `Corpus::freeze()` so the learned state carries
      a BLAKE3 commitment and is auditable, not just durable.
- [x] **T7.5 GOAT gate — RUN, and it FAILS on an unreachable premise.** The
      gate as written ("retrieval precision (G1) improves ≥10% after N=100
      cycles vs cold-start directions") is **not reachable by the mechanism it
      gates**: direction evolution moves the router, and the router cannot
      move G1 (T3.5.1). Measured anyway on the metric it *can* move — router
      annotation accuracy — evolution scores **−1.0pp**, not +10% (T7.1).
      And the retrieval half it was meant to certify (T7.3) has **zero
      headroom**: 0/25 held-out rankings improvable, 19/25 breakable.
      **Verdict: the self-healing loop does not improve retrieval, and the
      honest gate result is FAIL.** Recorded rather than quietly rescoped.

### Phase 7 exit criteria
- Self-healing loop ships: `--fix` records outcomes → directions evolve → next `--fix`/
  `--suggest` retrieves better. The dMoE router + corpus retrieval both improve with use.
  GOAT: ≥10% precision improvement after 100 cycles.

## Phase 8 — WASM pruner fast-validation hook (katgpt-rs Proposal 011, gated on rubrc)

The fast `--fix` validator. Blocked on `rubrc` maturity
([katgpt-rs Proposal 011](../../katgpt-rs/.proposals/011_rust_swe_bench_latent_space_via_wasm_pruner.md)
Phase 3 — a *sibling-repo* proposal, not one of this repo's three) — this
phase is SKETCHED, not started. Until it lands, `cargo test` (slow) + `RustAnalyzerPruner`
(syntax-only, fast) are the validators.

**Status: blocked on rubrc (katgpt-rs Proposal 011).**

> **Sizing note (Issue 023, 2026-08-16).** The measured `RustAnalyzerPruner`
> fast path is **~21-23 µs per fix** — so "replace the slow validator" only
> describes the `clippy_verify` / `fix_verify` *subprocess* paths, not the
> default one. T8.3's "<10ms vs ~seconds for cargo test" bar is the right
> comparison; do **not** justify this phase against the default L2.

### Tasks (sketch — not started)

- [-] **T8.1** `WasmPruner: ConstraintPruner` impl (katgpt-rs) — compiles the test suite
      to WASM, runs the proposed fix's token sequence against it, returns valid/invalid.
- [-] **T8.2** Wire into `cargo heal --fix` as the fast validator (replaces `cargo test`
      subprocess for the common case). Fallback to `cargo test` if WASM pruner declines.
- [-] **T8.3** GOAT: WasmPruner validation < 10ms (vs ~seconds for cargo test).

### Phase 8 exit criteria
- BLOCKED. Reopens when rubrc lands (katgpt-rs Proposal 011 Phase 3). Until then, the slow path
  (`cargo test`) is the validator; the self-healing loop (Phase 7) works with either.

## Phase 9 — Calibrated UQ layer (Deep fusion A: conformal + select_best_belief + clr_vote)

Replace the uncalibrated `DraftedFix.confidence = sigmoid(mean(matvec_output))` with
calibrated, distribution-free confidence intervals. Three DEFAULT-ON katgpt-core
primitives fuse: `ConformalIntervalCalibrator<F>`, `select_best_belief`, `clr_vote`.
All three are verified to ship via direct grep (2026-08-14). All are floor-tested.

**Status: not started. Substrate verified. Warm-up cost + false-confidence
failure mode MEASURED — see Issue 023.**

> **Issue 023 POC (2026-08-16) — two corrections before this phase starts.**
>
> 1. **No new dependency is needed.** `conformal_predictive_intervals` is
>    DEFAULT-ON in katgpt-core and riir-clippy already deps katgpt-core with
>    default features. The `calibrated_uq` gate is a riir-clippy-side feature
>    only; it pulls nothing new.
> 2. **A cold calibrator emits a ZERO-WIDTH interval, not a wide one.**
>    `weighted_quantile_pair` returns `(0.0, 0.0)` on an empty pool, so an
>    unwarmed key silently claims *perfect* confidence — measured **0.0%
>    coverage at n=0 and n=1**. This is a false-confidence signature, the exact
>    thing the floor rule exists to catch. The implementation MUST refuse to
>    emit an interval below the warm-up count rather than emit the degenerate one.
>
> Measured warm-up curve (5 seeds, 4000 held-out evals/point, 3 repeat runs,
> all agreeing): coverage crosses T9.5's ≥90% bar at **exactly n = 19
> residuals** — identical to the analytic split-conformal bound
> `⌈(n+1)(1−α)⌉ ≤ n`. Full table in Issue 023.
>
> | n | 0 | 1 | 2 | 4 | 8 | 16 | **19** | 24 | 64 |
> |---|---|---|---|---|---|---|---|---|---|
> | coverage | 0.0% | 0.0% | 27.6% | 59.0% | 80.7% | 88.9% | **92.2%** | 95.8% | 96.3% |
>
> **Consequence: per-rule keying is the wrong default.** 19 residuals × 38
> `--fix`-eligible rules = **~722 verified fixes** before T9.5 is measurable at
> all. T9.2 is amended to key **per-domain** (3 keys → ~57 fixes); per-rule
> becomes an opt-in upgrade once the trajectory corpus is deep enough.

### Tasks

- [ ] **T9.1** Define `FixForecaster: PointForecaster` impl that maps
      `(rule_id, buggy_hash) → f32` predicted-correctness score. The forecast
      target = 1.0 if the fix was actually correct (post-verify), 0.0 otherwise.
      Wrap `TernaryDrafter`'s existing `confidence` output as the point forecast.
      Supply the point via `interval_from_point_into` (the caller-supplied-ŷ
      path); `forecast_into` stays a stub.
- [ ] **T9.2** Wrap with `ConformalIntervalCalibrator<FixForecaster>` behind a
      `calibrated_uq` feature gate (riir-clippy-side only — katgpt-core already
      ships the primitive default-on). **Key per-DOMAIN, not per-rule**
      (`n_channels = 3`), residual ring capacity 64. Per-rule keying is a
      documented opt-in upgrade, gated on ≥19 verified fixes per rule.
      The calibrated output replaces `DraftedFix.confidence` with a true 95%
      interval `[lo, hi]`.
- [ ] **T9.2b (warm-up guard — blocks T9.5)** Below `MIN_RESIDUALS = 19` for a
      key, return `None`/`Unknown` and fall back to the raw sigmoid confidence
      with an explicit "uncalibrated" marker. **Never emit the degenerate
      zero-width interval.** Add a test asserting a cold key does not produce
      a `[point, point]` interval — the Issue 023 regression witness.
- [ ] **T9.3** Replace the `0.6·evolution + 0.4·success_rate` linear blend in
      `HealPipeline::heal` with `select_best_belief(candidates, ε, incumbent)`.
      Each candidate carries `(successes, failures)` as Beta posterior counts.
- [ ] **T9.4 (clr_vote, opt-in `clr_voting`)** Generate K=8 drafted fixes
      (currently 1) via QMC perturbation of the ternary matvec, cluster by
      syntactic similarity (Levenshtein on AST node-kind sequence), run L2
      pruner on each, `clr_vote` picks the most-reliable cluster's winner.
- [ ] **T9.5** GOAT gate (calibrated_uq), measured **only on keys past the
      T9.2b warm-up count** (a cold key's 0.0% coverage would otherwise sink
      the gate for the wrong reason):
      - **G1 (coverage)** empirical coverage of the 95% interval ≥ 90%.
      - **G2 (regret)** `select_best_belief` selection-regret ≤ the
        linear-blend baseline (Bench 010 T5 floor).
      - **G3 (conformal-naive floor)** beat
        `ConformalIntervalCalibrator<SeasonalNaiveForecaster>` (m=1, plain
        split conformal) on CRPS / coverage / Winkler. This is katgpt-rs's
        "Report the Floor" rule — not binding on riir-clippy, but this phase
        ships exactly the claim class the rule exists for, and the floor is
        already in-tree. Cheap to include; expensive to retrofit.
      - **G4 (no false confidence)** zero degenerate zero-width intervals
        emitted, at any pool size (the Issue 023 failure mode).
      Record in the next `.benchmarks/` slot (`.benchmarks/.highwater` + 1).

### Phase 9 exit criteria
- `DraftedFix.confidence` is a calibrated interval, not a raw sigmoid —
  or an explicit "uncalibrated" marker when the key is below warm-up.
- `HealPipeline::heal` selects fixes via Beta-posterior, not linear blend.
- GOAT G1-G4 PASS.

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
      the next `.benchmarks/` slot (`.benchmarks/.highwater` + 1).

### Phase 10 exit criteria
- `cargo heal --sleep` mines motifs from past trajectories, freezes them,
  persists them across restart.
- The healer's skill library improves overnight.

## Phase 11 — Poincaré fix-imagination layer (Deep fusion C: imagine_belief_into + ViableManifoldGraph)

**THE NOVEL ANGLE — but a QUALITY one, not a latency one.** The healer imagines
the fix in Poincaré space before the L2 validate. Two substrates compose:
`poincare_navigate_into` (single-step imagination) + `ViableManifoldGraph` (safe
fix-neighborhood graph). Both verified via direct grep (2026-08-14).

**Status: not started, RESCOPED 2026-08-16. Substrate verified. The latency
justification is DEAD (measured, Issue 023). Quality claim survives, unproven.**

> **Issue 023 POC (2026-08-16) — the latency premise is false by ~43×.**
>
> T11.5 assumed "the imagination gate is µs-scale; the L2 pruner is ms-scale".
> Measured (median of 200 reps over 5 real spans, 3 repeat runs):
>
> | stage | per fix | share of pipeline |
> |---|---:|---:|
> | L2 (`RustAnalyzerPruner` fast path = in-process `syn` parse) | **21-23 µs** | ~6% |
> | imagination gate (`poincare_navigate_into` + dot + sigmoid) | **0.09-0.11 µs** | ~0% |
> | retrieval (Bench 018 G2) | 349 µs | **~94%** |
>
> L2 is **µs-scale, not ms-scale** — it is not a subprocess, it is a `syn`
> parse. Imagination only avoids an L2 call on the fixes it *rejects*, so
> `speedup = T_base / (T_base − base_rate·recall·T_L2 + T_imag)`. Even at a
> **100% bad-fix rate with 100% recall the ceiling is 1.0655×**, against
> T11.6's ≥1.5× gate. At a realistic 5% bad-fix rate it is 1.0007×.
>
> **The gate cannot pass. This is the wrong denominator, not a tuning
> problem.** The ≥1.5× latency gate is REMOVED. Any future latency work on
> this pipeline must target **retrieval** (349 µs, 94% of budget) — which is
> also where the live Phase 3 G1 problem lives.
>
> Two further defects the POC surfaced:
> - **T11.5 cited an API that does not exist.** Real signature is
>   `poincare_navigate_into(z_src, delta_target, adapter, z_out, phi_scratch,
>   hidden_scratch)` — a *delta* target plus a fitted adapter and two scratch
>   buffers, not a `(src_pos, dst_pos)` pair. Corrected in T11.5 below.
> - **T11.6 mislabelled its own metric** — "accepting ≥90% of would-pass-L2"
>   is specificity / TNR, not precision. Corrected below.
>
> **Ordering:** this phase now depends on **Phase 10 T10.1** (T11.1 needs the
> PTG-shaped trajectory that T10.1 produces) and is demoted below Phase 12 —
> Phase 12 answers a cheaper version of the same "will this fix hold?"
> question from data this phase would have to collect anyway.

**Prior-art risk (unchanged):** Plan 497 Phase 2 REFUTED the quality claim for
NPC HLA. That refutation kept its latency win; here the latency win is measured
away, so **quality is the only surviving claim**. If G1/G2 below fail, this
phase closes as a negative result rather than shipping opt-in.

### Tasks

- [ ] **T11.1** Collect (pre-fix embedding, post-fix embedding) trajectory
      pairs from past `--fix` cycles. The corpus needs trajectory shape — the
      current flat `FixTrajectory` is insufficient (blocker for T11.2).
      **Hard dependency on Phase 10 T10.1**, which produces exactly that
      PTG-shaped trajectory. Do not start this phase before T10.1 lands.
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
      **IMAGINE via `poincare_navigate_into(z_src, delta_target, &adapter,
      &mut z_out, &mut phi_scratch, &mut hidden_scratch)`** → score → if
      score < τ, reject + try next retrieved rule → if score ≥ τ, L2-validate.
      (Signature corrected per Issue 023 — a *delta* target plus the fitted
      adapter and two scratch buffers, not a src/dst position pair. Scratch
      lengths are `adapter.phi_out()` / `adapter.phi_hidden()`; hoist both out
      of the loop per the alloc-free rule.)
      **The rationale is quality, not throughput** — measured, imagination
      saves at most 1.07× even in the best case (Issue 023). The reason to
      reject early is to try the *next retrieved rule* instead of burning the
      fix attempt, which matters because Phase 3's at-scale top-1 is 58.3%
      on clippy: the right rule is often rank 2-3, not rank 1.
- [ ] **T11.6** GOAT gate (poincare_imagination) — **quality only, latency gate
      removed per Issue 023**:
      - **G1 (rejection recall)** imagination rejects ≥30% of "would-fail-L2" fixes.
      - **G2 (specificity / TNR)** it accepts ≥90% of "would-pass-L2" fixes.
        (Previously mislabelled "precision" — it is TNR.)
      - **G3 (rank recovery)** on spans where top-1 is wrong but a top-3 rule
        is right, the reject-and-advance path lands the correct rule ≥X% of
        the time. Set X from the Bench 022 rank distribution once T3.5 lands.
      - **~~latency ≥1.5×~~** — REMOVED. Measured ceiling is 1.0655× at a 100%
        bad-fix rate (Issue 023); the gate was arithmetically unreachable.
        A no-regression check (imagination costs <5% wall-clock) replaces it.
      Record in the next `.benchmarks/` slot (`.benchmarks/.highwater` + 1).
- [ ] **T11.7** Multi-step imagination (`poincare_multi_step_into`) for large
      fixes that span multiple chunks: split the fix into N sub-steps, imagine
      each, reject if any sub-step diverges.

### Phase 11 exit criteria
- Imagination gate ships behind `poincare_imagination` feature gate (opt-in).
- GOAT G1 (rejection recall ≥30%) + G2 (specificity ≥90%) + G3 (rank recovery) PASS.
- **If quality is REFUTED** (as Plan 497 Phase 2 was for NPC HLA): close this
  phase as a **negative result**. There is no latency win to fall back on —
  Issue 023 measured it away — so a quality refutation leaves nothing to ship.
  Document it and do NOT keep an opt-in flag alive for its own sake.

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
      the next `.benchmarks/` slot (`.benchmarks/.highwater` + 1).

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
      Record in the next `.benchmarks/` slot (`.benchmarks/.highwater` + 1).

### Phase 14 exit criteria
- All three substrates wired. The healer's memory layer is no longer flat +
  in-memory; it's Hebbian fact store + graph-indexed dual-stream + memory-steered
  drafter.
- GOAT G1 PASS (≥5% improvement over Phase 9).

## Out of scope (per Proposal 002 revised + deep fusion layer)

- Full proposer coverage (all perf + kernel rules — 58 perf and 282+ kernel as of
  2026-08-16, not the 34/~30 this plan was written against) — decoupled;
  `--suggest` covers them.
- CubeCL `#[cube]` macro-body proposers — `--suggest` still works (embedding captures the
  signature); only `--fix` is limited.
- L4 LoRA — deferred across all domains.
- `--watch` mode — deferred.
- Domain-direction mining via `CommittedFieldBlend` — **corpus-derived centroids**
  ship first. (Corrected 2026-08-16: the original "hand-crafted vectors ship
  first" never happened — T1.4 shipped L2-normalized corpus-pattern centroids
  instead, and re-fitting those after the 10 → 40 rule corpus growth is now
  option 1 of the live T3.5 contingency.)
- **ConvexTok-AST implementation** — Phase 13 is research-only. The implementation
  plan (Phase 15+) opens only if the encoding design + corpus experiment (T13.3)
  shows ≥2× compression.

## Risks

1. ~~**Span-preserving patch is harder than it looks.**~~ **CLOSED 2026-08-14
   (Phase 6 T6.1).** The span-preserving edit-splice emitter shipped directly;
   the `prettyplease` fallback was never needed and never built. (The original
   text pointed at "T1.3" — that referred to the *pre-revision* task numbering;
   in this plan T1.3 is `latent_matcher.rs`.)
2. **Visitor false positives.** A `vec-with-capacity` visitor might match a vec that's intentionally un-sized. Mitigation: visitors are conservative (only rewrite when the size is statically known); uncertain cases go to `--suggest`. **LIVE** — three real instances were caught by the T6.2 G5 run (use-before-decl capacity, non-usize bounds, fn-path coercion) and fixed at root cause; `--fix --verify` (Issue 015) is the standing net.
3. **CubeCL parse failure.** A real riir-gpu file with `#[cube]` macros won't parse with vanilla `syn`. Mitigation: the scanner skips files that fail to parse (with a warning); proposers run only on parseable files. Kernel proposers are tested on EXTRACTED snippets, not whole kernel files. **LIVE**, and it is the reason Phase 5 is deferred.
4. ~~**G5 might fail.**~~ **CLOSED 2026-08-14 (T6.2).** G5 passed in katgpt-rs
   (`katgpt-pruners`, 12 files / 20 edits, commit `8c331cd9`, pushed);
   1233/1233 tests green on the gated re-run.
5. **NEW — retrieval precision does not survive corpus growth.** G1 was 100% on
   a 24-item fixture and **58.3% on 130 real spans** once the corpus reached 40
   rules (Bench 022). The failure mode is nearest-neighbour crowding, which gets
   *worse* with every corpus batch — and this repo adds batches continuously.
   Mitigation: T3.5 (the live Phase 3 contingency) plus the committed
   regression gate, which now floors every future corpus batch at baseline − 1
   span. Treat corpus growth as a precision liability, not a free win.
6. **NEW — false confidence from cold UQ state.** Any calibrated-confidence
   layer (Phase 9) emits a **zero-width, 0%-coverage interval** on an unwarmed
   key rather than an honestly-wide one (measured, Issue 023). Mitigation:
   T9.2b's warm-up guard is a blocking prerequisite for T9.5, not a polish item.

## Numbering discipline

- Proposal 002: `riir-clippy/.proposals/002_real_file_span_healing.md`
- Plan 005: this file.
- Issue 023: the Phase 9 / Phase 11 design-question POC (`tests/poc_023_plan005_design_questions.rs`).
- Highwater at last edit (2026-08-16): `.plans` = `5`, `.proposals` = `3`,
  `.benchmarks` = `26`, `.issues` = `23`, `.research` = `54`.
- **Forward-declared benchmark filenames are banned in this plan.** Phases 9-14
  originally hard-coded `017`-`021`; every one of those numbers was taken by an
  unrelated bench before the phase started. Future tasks say "the next
  `.benchmarks/` slot (`.benchmarks/.highwater` + 1)" and resolve the number at
  landing time.
