# Latent suggest — Plan 005 span-level retrieval (the `--suggest` MVP)

**Source:** [`src/draft/`](../../src/draft/) — `corpus_index.rs` + `domain_router.rs` + `latent_matcher.rs`
**Feature:** `latent_retrieval` (opt-in — pulls `riir-rag`, transitively `riir-neuron-db`)
**Plan:** [005](../../.plans/005_real_file_span_healing.md) Phases 1–3 (DONE 2026-08-14)
**GOAT:** [Bench 018](../../.benchmarks/018_plan005_latent_suggest_goat.md) — **G1–G4 ALL PASS**

## What it is

The retrieval layer that matches **real code spans** to corpus rules. Real
`.rs` files are chunked at fn/impl/struct boundaries, embedded modellessly,
routed across domains, and matched against a per-domain latent index of the
corpus. **ZERO proposer code** — the suggestion IS the corpus pattern text
(`buggy_code`/`ref_shape` + the head of `fixed_code`/`opt_pattern`). The
proposers (Phase 4 rust_perf + Phase 6 T6.4 clippy emitter migration) are
wired into the `cargo refine --fix` CLI — they consume the same rule book
this layer builds; the emitter substrate lives in `src/edit_emitter.rs`.

Zero retrieval code reimplemented — the layer composes riir-rag's substrate
as-is: `AstChunker` (spans) + `ModellessEmbedder` (BLAKE3+DFT+sigmoid, 8-D,
no weights; opt-in 768-D `BonsaiEmbedder` for higher accuracy) +
`NeuronDbRetriever` (`ShardIndex` Clifford-wedge KNN + BM25).

## Pipeline

```text
.rs file
  → AstChunker.chunk()            (fn/impl/struct spans — riir-rag)
  → ModellessEmbedder.embed()     (8-D, no weights — riir-rag)
  → DomainRouter.route()          (annotation only — see deviations)
  → RuleIndex.suggest()           (fan-out across ALL enabled domains)
  → ranked (rule_id, score) + RuleInfo {domain, buggy, fixed}
```

## Components

| Component | File | Role |
|---|---|---|
| `RuleIndex` | `corpus_index.rs` | Per-domain index over `NeuronDbRetriever`. Multi-scale: whole pattern + significant pattern lines, keyed `BLAKE3 → rule_id`. `suggest()` (whole-query) + `suggest_deep()` (whole + per-line, max-merge per rule). |
| `DomainRouter` | `domain_router.rs` | Corpus-derived L2-normalized centroid directions + runtime-sized argmax mirroring `pick_domain` tie semantics (the enabled-domain set is feature-dependent). Since the Issue 020 POC (Bench 025): the annotation embedding is the token-frequency cosine block (`RoutingEmbedding::TfCosine`, 69%/69% 2-/3-domain vs the BLAKE3-baseline's 53%/48%); the spectral DFT variants + the baseline live on as the reproducible measurement matrix (`run_router_eval`). |
| `LatentMatcher` | `latent_matcher.rs` | The composed file/span pipeline + `rule_book` (`rule_id → RuleInfo`, built once at index time — the `--fix` Phase 4/5 proposers consume the same book) + `SuggestScratch` (reused buffers, G4). |

## Two empirical design deviations (Plan 005 Phase 1)

1. **Fan-out, not gated routing.** The plan's gated route→retrieve starved
   the correct domain on fixtures — the 8-D structural centroids mis-routed
   a `return expr;` fn to perf + a linear-scan fn to clippy (swapped). The
   router verdict survives as an **annotation** on each span. **The re-gate
   question is CLOSED for the lexical branch** (Issue 020, Bench 025): the
   spectral POC (token-frequency DFT block + full-TF ablation, 7 variants)
   topped out at **70.3%** routing accuracy — 15pp under the 85% kill-bar,
   with the ablation proving the k-coefficient DFT compression loses
   nothing (full TF ≈ DFT-k8: the LEXICAL signal is the ~70% ceiling;
   domain membership is semantic). The annotation itself was salvaged to
   the measured-best variant (TF-cosine-global: +20pp accuracy at 0.96µs
   vs 3.31µs). Only a semantic-signal embedder (768-D class) could re-open
   gating.
2. **Multi-scale indexing + queries.** Corpus patterns are 1–5 line excerpts
   while real spans are whole items; single-scale retrieval missed the
   needle on the file fixture. Both index and queries carry the whole +
   significant-lines scales.

## GOAT (Bench 018, 2026-08-14 — ALL PASS)

| Gate | Target | Measured |
|---|---|---|
| G1 retrieval precision (top-1, per domain) | ≥ 80% | **96% (23/24)** — 10 clippy + 14 perf renamed-identifier variants (expanded from 12 → 24, 2026-08-14; all 10 clippy rules now covered; single miss = `retain-inplace` → `collect-avoidance`, the near-miss sibling pair) |
| G2 latency per chunk (end-to-end) | < 1 ms | **349 µs** (multi-scale + 2-domain fan-out, release) — 2.9× headroom |
| G3 no-regression | lib suite green | **104/104** |
| G4 alloc-free steady state | scratch capacity stable | **stable across 50 repeat scans** (`SuggestScratch`) |

The first G1 run FAILED at 75% (9/12) — fixture audit showed 3 variants
mis-instantiated their rules (method-closure vs fn-closure, value-borrow vs
ref-borrow, iterator-chain vs canonical `.into()`); faithful rewritten
variants scored 12/12. Lesson (Issue 669 class): **audit fixture
faithfulness BEFORE tuning the retriever** — a retrieval gate is only as
honest as its fixture.

## Retrieval-precision eval at scale (Issue 017, Bench 022)

The Plan 005 G1 gate's small fixture (12 → 24 items, 96% top-1) is a
point estimate that corpus growth silently invalidates. The scaled eval —
[`src/draft/retrieval_eval.rs`](../../src/draft/retrieval_eval.rs) — measures
ranking precision on **130 spans** (48 clippy + 40 perf labeled variants,
30 kernel self-variant probes, 12 real production clean excerpts) at the
current corpus scale, and re-measures on every batch via the committed
gate test:

```bash
cargo test --features latent_retrieval --lib retrieval_precision_regression_gate -- --nocapture
# kernel-inclusive (on-demand, ~54 s at 343 rules):
cargo test --features latent_retrieval,kernel_opt --lib retrieval -- --nocapture
```

**Baseline (Bench 022, 2026-08-16; RE-PINNED same day by Issue 022 — Bench
024):** clippy **54.2/70.8/83.3** (top-1/3/5), perf 87.5/95.0/97.5, kernel
self-variants 100/100/100. The 12-fixture 100%
top-1 claim is dead at scale — crowding is real, concentrated in tiny-token
clippy patterns (`identity_op` alone tops 5 misses). Router annotation
accuracy: **69.3% (2 domains) / 68.6% (3)** since the Issue 020 wiring
(TF-cosine; was 53.4%/46.6% on the BLAKE3 baseline) — fan-out stays correct,
and the annotation accuracy carries its own floor (63.6% both states) in the
gate. Floors are
pinned in the eval module (baseline − 1 span); a batch that decays them
fails the suite and must re-pin with a recorded delta. **The Issue 022
re-pin's −2 clippy spans are the corpus-truthfulness trade**: 5 corpus
entries' buggy shapes were corrected to forms real clippy 1.97 actually
fires (the `comparison_to_empty` text moved to the real `s == ""` shape);
the 2 len-shaped fixtures retrieve at lower rank by design now. Ranking is
canonicalized (`score` desc, `rule_id` tie-break) — the measurement is
reproducible across backing-map iteration orders.

## Known limitations (honest scope, Bench 018 + addendum)

- Routing is fan-out + annotation, not gating (deviation 1 above).
- **No minimum-score threshold — and none is possible** (measured, Bench 018
  addendum 2026-08-14): adversarial clean snippets (the correct version of
  each corpus anti-pattern) score as high as or higher than the true
  positives, in both whole-span and multi-scale modes. The fused score is
  text similarity, not rule instantiation. Every span printing suggestions is
  inherent; treat `--suggest` output as a ranked review queue. The
  calibration instrument lives in `tests/latent_suggest_goat.rs`
  (`NEGATIVE_FIXTURE` + the two calibration tests).
- G1's fixture was expanded 12 → 24 (2026-08-14): 96% (23/24) — the one
  miss is the `retain-inplace`/`collect-avoidance` near-miss pair (same
  root cause as the threshold refutation: text similarity cannot see the
  named-binding-re-consume distinction). Wilson 95% lower bound ≈ 0.80 =
  the gate floor. The expansion's real gain is coverage breadth (all 10
  clippy rules), not statistical tightness.

## CLI

The `cargo-refine` bin wraps this layer: see
[`../07_examples/cargo_refine.md`](../07_examples/cargo_refine.md).

## See also

- [`../02_corpus/`](../02_corpus/README.md) — the indexed knowledge base (L0)
- [`../01_orientation/sibling_layout.md`](../01_orientation/sibling_layout.md) — the `latent_retrieval` dep posture (opt-in riir-rag arm)
- [Bench 018](../../.benchmarks/018_plan005_latent_suggest_goat.md) — the full GOAT record
