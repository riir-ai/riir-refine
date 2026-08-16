# Bench 027 — Plan 005 T3.5: the G1 recovery contingency

**Date:** 2026-08-16
**Plan:** [005](../.plans/005_real_file_span_healing.md) Phase 3, T3.5
**Feature:** `latent_retrieval` (+ `bonsai_retrieval` for option 3)
**One command:** `cargo test --features latent_retrieval --lib retrieval_sweep_report -- --nocapture`

**Verdict: clippy top-1 54.2% → 70.8% (+16.6pp), perf 87.5% → 97.9% (+10.4pp),
blended 70.8% → 84.4% (+13.6pp). The G1 bar of 80% is NOT met — Phase 3 stays
open.** Option 1 is refuted by architecture, option 2 delivered the whole gain
and is promoted to default, option 3 is measured and rejected on cost/benefit.

## Baseline (the state T3.5 was opened against)

| domain | labeled | top-1 | top-3 | top-5 |
|---|---:|---:|---:|---:|
| clippy | 48 | **54.2%** | 70.8% | 83.3% |
| perf | 48 | 87.5% | 95.8% | 97.9% |
| blended | 96 | 70.8% | 83.3% | 90.6% |

Router annotation 68.8%. Note this is *worse* than Bench 022's 58.3% clippy —
the fixture grew from 88 to 96 labeled spans in between, so the two numbers are
not directly comparable. Everything below uses the 96-span fixture throughout.

---

## Option 1 — re-fit corpus centroids: REFUTED BY ARCHITECTURE

**The option cannot move G1, for a structural reason that predates it.**

1. **Retrieval is fan-out; the router is annotation-only.** `run_eval` selects
   `by_domain` for the span's *own* domain and ranks within it
   (`retrieval_eval.rs:441`); the router verdict feeds only `router_correct`.
   Centroid quality therefore cannot change any top-1/3/5 number by
   construction. Bench 022 said the same from the miss side ("wrong-domain
   (router): 0 misses caused").
2. **The centroids were never stale.** `DomainRouter::with_embedding` computes
   each centroid as the L2-normalized mean of the *live* corpus pattern
   embeddings at every construction, and `LatentMatcher::from_enabled_domains`
   feeds it straight from `build_corpus()`. There is no cached or frozen
   centroid file, so "re-fit after the 10 → 40 rule growth" was already in
   effect on every run. The premise of the option was wrong.
3. **The fit space was already swept** by Issue 020 (Bench 025), and that sweep
   is committed and re-runnable (`cargo test --features latent_retrieval --lib
   router -- --nocapture`):

| routing embedding | annotation accuracy | mis-routes fixed |
|---|---:|:--|
| baseline-8d | 53.1% | yes |
| **dft8-global** | **75.0%** | no |
| dft8-global+struct | 74.0% | no |
| dft8-disc | 61.5% | yes |
| dft8-disc+struct | 62.5% | yes |
| tf-cosine-global (**shipped**) | 68.8% | yes |
| tf-cosine-disc | 68.8% | yes |

The best available fit is 75.0%, still far under Issue 020's 85% gating bar, so
gating stays killed and the router stays an annotation. **No change shipped.**

---

## Option 2 — tiny-token crowding: ROOT-CAUSED, FIXED, PROMOTED

### Root cause

The substrate default `riir_neuron_db::bm25::CodeTokenizer` splits on
`!c.is_ascii_alphanumeric() && c != '_'` — it keeps identifiers and **discards
every punctuation character**. The clippy corpus is 40 minimal generic
functions whose entire discriminative signal is punctuation:

| lint | pattern | signal | after `CodeTokenizer` |
|---|---|---|---|
| `identity_op` | `fn f(x: u32) -> u32 { x * 1 }` | `* 1` | `fn f x u32 u32 x 1` |
| `unused_unit` | `fn f() -> () {}` | `-> ()` | `fn f` |
| `deref_addrof` | `&*x` | `&*` | `x` |
| `double_parens` | `((x))` | `((` | `x` |

Stripped of punctuation every rule collapses toward the same `fn f x` bag, and
the rule with the most *common* residual tokens — `identity_op` — becomes the
nearest neighbour of nearly everything. It topped **7 of the 22** baseline
clippy misses. This also explains why the other domains never had the problem:
`rust_perf` (`with_capacity`, `HashMap`, `Arc`) and kernel (`CubeDim`, `launch`)
are identifier-rich, and scored 87.5% / 100%.

### The fix

`RustOpTokenizer` (`src/draft/op_tokenizer.rs`) emits identifier tokens exactly
as `CodeTokenizer` does — so identifier-rich domains cannot regress from the
addition alone, pinned by a test — **plus one token per maximal punctuation
run**, prefixed `op:`. `RustOpBigramTokenizer` adds operator-adjacency bigrams.

Punctuation *runs* rather than a hand-picked operator table is deliberate: it
is fully generic (no Rust-grammar knowledge to keep in sync) and it lets
**BM25's own IDF decide what discriminates**. Ubiquitous runs (`(`, `;`, `{`)
earn near-zero IDF; rare ones (`()`, `&*`, `((`) do the separating. No tuned
weights anywhere.

Bigrams were needed because several signals are sequences whose parts are
individually ubiquitous: `op:()` appears in nearly every pattern (`fn f()`), so
unigram IDF cannot lift `unused_unit` — but the bigram `op:->|op:()` is rare.

Injected via `NeuronDbRetriever::with_bm25_tokenizer`, a designed substrate
hook. **Zero substrate code reimplemented.**

### Measurement (grid: tokenizer × latent fusion weight)

| tokenizer | latent_w | clippy@1 | clippy@3 | perf@1 | blend@1 | blend@3 |
|---|---:|---:|---:|---:|---:|---:|
| code-default | 1.00 | 54.2% | 70.8% | 87.5% | 70.8% | 83.3% |
| code-default | 0.00 | 54.2% | 72.9% | 87.5% | 70.8% | 84.4% |
| op-aware | 1.00 | 66.7% | 87.5% | 97.9% | 82.3% | 92.7% |
| op-aware | 0.00 | 66.7% | 87.5% | 97.9% | 82.3% | 92.7% |
| **op-bigram** | **1.00** | **70.8%** | **89.6%** | **97.9%** | **84.4%** | **93.8%** |
| op-bigram | 0.00 | 70.8% | 89.6% | 97.9% | 84.4% | 93.8% |

**Determinism:** bit-identical across 3 separate processes.

**Secondary result — the latent fusion weight is an inert lever, REFUTED.**
clippy top-1 is identical at weight 1.0, 0.5, 0.25 and 0.0: the 8-D
`ModellessEmbedder` term never changes a ranking, because BM25 dominates and
latent-only hits (`exact_score: None`) never win. `DEFAULT_LATENT_WEIGHT` is
therefore kept at **1.0** — the pre-T3.5 value — so the promotion carries no
gratuitous behaviour change. The knob survives only to keep this refutation
reproducible.

### Promoted

`RetrievalConfig::default()` = `{ tokenizer: OpAwareBigram, latent_weight: 1.0,
dense_weight: 0.0 }`. Regression floors rebased to measured − 1 span:
blended 80/89/90 of 96, clippy 33/42 of 48, perf top-3 46/48.

### Residual misses (7 at rank ≥ 3, down from 16)

`redundant_closure` and `needless_borrow` (both type-blind — their fixtures are
ordinary-looking code with no distinctive tokens, by design), `unused_unit`,
`semicolon_if_nothing_returned`, `needless_return`, `collapsible_if`, and perf's
`shrink-to-fit-after-build`. Of the 14 top-1 misses, 9 have the correct rule at
rank 2–3 — near-miss crowding, not absent signal.

---

## Option 3 — BonsaiEmbedder 768-D: MEASURED, REJECTED ON COST

### The plan's wording was imprecise

T3.5 said "switch to `BonsaiEmbedder` (768-D, opt-in)". `BonsaiEmbedder::embed`
— the `Embedder` trait — returns **8-D** (`project_chunk_mean_8d`). The 768-D
vector lives on the separate `DenseEmbedder` trait, consumed through
`index_chunks_dense` + `with_dense_index` + `rerank_dense`. A plain `Embedder`
swap would only have changed the 8-D term that option 2 just proved **inert** —
i.e. the option as written was a no-op. Wired the real dense-rerank path instead
(`bonsai_retrieval` feature, `RetrievalConfig::dense_weight`).

### Cost probe (the finding that decides this option)

| | measured |
|---|---:|
| GGUF load (6.7 GB Ternary-Bonsai-27B-Q2_0) | 166.3 s |
| `embed_dense`, short pattern | **16 575 ms** |
| `embed_dense`, long span | **48 074 ms** |

The full eval issues ~1018 embeds per config (index chunks + multi-scale line
queries × 96 spans) → **~13.6 hours per config**, ~41 hours for a 3-weight
sweep, on CPU. That is not a runnable benchmark.

This is a CPU ternary forward pass of a 27B model per *text*. The substrate has
no Metal path for `BonsaiEmbedder`; adding one is a riir-ai project, not a
riir-clippy one.

### What was measured instead

`tests/bonsai_retrieval_probe.rs::dense_rerank_precision` — the largest matched
subset that is feasible, with the scope stated rather than quietly shrunk:
clippy only (the domain that fails), whole-span scale on **both** arms so the
comparison is apples-to-apples, dense vectors cached once so the weight sweep
(0.5 / 1 / 5 / 20 / 100) is free.

<!-- T3.5-OPTION3-RESULT -->

---

## Reproduce

```bash
# options 1-2 (seconds)
cargo test --features latent_retrieval --lib retrieval_sweep_report -- --nocapture
cargo test --features latent_retrieval --lib retrieval_precision_regression_gate -- --nocapture
cargo test --features latent_retrieval --lib router -- --nocapture

# option 3 (~1 h, 6.7 GB GGUF; run the cost probe first)
cargo test --release --features bonsai_retrieval --test bonsai_retrieval_probe probe_cost -- --nocapture
cargo test --release --features bonsai_retrieval --test bonsai_retrieval_probe dense_rerank_precision -- --nocapture --test-threads=1
```
