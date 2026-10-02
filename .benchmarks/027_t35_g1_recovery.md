# Bench 027 — Plan 005 T3.5: the G1 recovery contingency

**Date:** 2026-08-16
**Plan:** [005](../.plans/005_real_file_span_healing.md) Phase 3, T3.5
**Feature:** `latent_retrieval` (+ `bonsai_retrieval` for option 3)
**One command:** `cargo test --features latent_retrieval --lib retrieval_sweep_report -- --nocapture`

**Verdict: the G1 bar is MET.** Clippy top-1 54.2% → 70.8% (tokenizer) →
**87.5%** (re-rank) on the committed eval, and **96.0%** on the uncontaminated
composite set; perf 87.5% → 97.9%; blended 70.8% → 92.7%.

Route: option 1 refuted by architecture; option 2 (operator-aware BM25
tokenizer) delivered the retrieval half and is promoted; option 3 measured and
rejected on cost; T3.5.4 built an instantiation re-ranker whose apparent +25pp
was proven **circular**; T3.5.5 built an independent real-`cargo clippy` oracle,
found the labels were fine but the *sample* was degenerate (0/43 multi-lint),
authored the missing multi-lint spans, and re-measured: **+12.0pp on
uncontaminated evidence → GOAT PASS → `RerankMode::Structural` promoted.**

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

**All three domains, kernel-inclusive state** (`--features
latent_retrieval,kernel_opt`, ~52 s):

| domain | labeled | top-1 | top-3 | top-5 |
|---|---:|---:|---:|---:|
| clippy | 48 | 70.8% | 89.6% | 91.7% |
| perf | 48 | 97.9% | 97.9% | 97.9% |
| kernel | 30 | 100.0% | 100.0% | 100.0% |
| blended | 126 | **88.1%** | 95.2% | 96.0% |

Kernel self-variants hold at 100% — the standing crowding alarm did NOT fire,
confirming that adding operator tokens costs identifier-rich domains nothing.
3-domain blended rose 78.8% → 88.1% vs Bench 022; router annotation 69.8% →
70.6% (unchanged mechanism — the fixture and corpus moved, not the router).

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

**Measured 2026-08-16 — the run took 60.2 minutes wall (3613.87 s):** 170.0 s
model load, 22.3 min for the 40 pattern embeds, 35.1 min for the 48 span
embeds.

| dense weight | clippy top-1 | vs matched floor |
|---:|---:|---:|
| 0.0 (matched modelless floor) | **70.8%** | — |
| 0.5 | 70.8% | +0.0pp |
| 1.0 | 70.8% | +0.0pp |
| 5.0 | 70.8% | +0.0pp |
| 20.0 | 70.8% | +0.0pp |
| 100.0 | 68.8% | **−2.1pp** |

**Verdict: the 768-D dense rerank does NOT beat the matched modelless floor.**
Best across the whole sweep is 70.8% — *exactly* the floor, to the span.

The shape of the result is as informative as the number. The dense term changes
**nothing at all** for every weight from 0.5 to 20, then *hurts* once it is
large enough to dominate (−2.1pp at 100). A signal that never flips a ranking
until it overrides everything, and then makes things worse, is not a weak
signal — it is an **uncorrelated** one. Bonsai's mean-pooled semantic embedding
carries no information about *which clippy anti-pattern a span instantiates*,
which is unsurprising in hindsight: all 40 corpus patterns are minimal Rust
functions that mean roughly the same thing semantically, and they differ
syntactically.

**Cost/benefit is therefore unambiguous.** 6.7 GB of GGUF, a 170 s load, ~48 s
per embed, and 60 minutes for one scope-limited configuration, in exchange for
+0.0pp. The `bonsai_retrieval` feature stays opt-in and un-promoted; it exists
so this measurement is reproducible, not because it is usable.

This is a direct vindication of riir-rag's own rule that the modelless path is
the permanent floor a model must *beat* to justify its cost. It did not beat
it — it tied it, at roughly 10^5 times the latency.

**What actually closed the gap** was the modelless work: the operator-aware BM25
tokenizer (T3.5.2, +16.6pp) and proposer-instantiation re-ranking (T3.5.5,
+12.0pp uncontaminated), neither of which loads a model at all.

---

## T3.5.4 — instantiation re-ranker: MECHANISM SHIPPED, GAIN UNMEASURABLE, NOT PROMOTED

### The target

T3.5.2 left a specific failure *shape*, not just a number: 9 of the 14 residual
clippy top-1 misses had the correct rule at **rank 2-3**, with top-3 at 89.6%.
Right answer present, wrong answer first — a re-ranking problem. A better
first-stage retriever cannot fix that; different *evidence* can.

Every score in the stack is text similarity, and Bench 018 already established
its ceiling: the *correct* `Vec::with_capacity` function scored higher on
`vec-with-capacity` than the buggy `Vec::new` one. The proposers' `syn`
visitors answer the question similarity cannot — *does this rule actually
apply?* `src/draft/instantiation_rerank.rs` runs the candidate rules' visitors
over the span (one parse, K visitor walks) and lets a firing visitor outrank a
non-firing one. Retrieve pool of 8 → mark instantiation → re-rank → truncate.

### Measured — and why the numbers are NOT an effect size

| mode | clippy@1 | clippy@3 | perf@1 | blend@1 | blend@3 |
|---|---:|---:|---:|---:|---:|
| off (shipped) | 70.8% | 89.6% | 97.9% | 84.4% | 93.8% |
| structural | 87.5% | 93.8% | 97.9% | 92.7% | 95.8% |
| all | 95.8% | 95.8% | 97.9% | 96.9% | 96.9% |

**This measurement is contaminated and the deltas must not be quoted as a
gain.** `clippy_fixture_labels_verify_via_proposers` — a committed, passing
test — asserts that for all 48 clippy fixtures the OWN proposer fires and
**zero** of the other 40 cross-fire. That is exactly the property the
re-ranker exploits: the oracle that defined the labels is the oracle doing the
ranking.

`rerank_circularity_diagnostic` states the falsifiable version and it **holds
exactly**:

```
pool recall @8        : 95.8% (46/48)
RerankMode::All top-1 : 95.8%
verdict: CONTAMINATED — All-mode top-1 IS pool recall
```

`All`-mode top-1 equals pool recall to the digit. The table is measuring *"was
the correct rule retrieved?"*, not *"did re-ranking pick it?"*. Under the
repo's GOAT rule — a gain measured on a biased oracle is not a modelless gain —
**the gate has not passed, so this is NOT promoted.** `RerankMode::default()`
is `Off`, asserted by a test so nobody flips it by reading the table.

### What IS established, non-circularly

- **Safety.** The structural proposers are verified *not* to fire on real
  production code by `clean_spans_fire_no_lint` (12 real excerpts from this
  crate) — an oracle with no connection to the labeled fixtures. `Structural`
  therefore cannot promote a spurious rule on clean input; worst case it does
  nothing.
- **Coherence.** `--fix` already treats a firing visitor as the decision
  (Phase 6 T6.1). Enabling this makes `--suggest` rank by the same evidence
  `--fix` acts on, rather than by a score that cannot tell a bug from its fix.
- **Cost.** One parse + K visitor walks per span; no allocation beyond the
  candidates already held. `Off` short-circuits before the parse, so the
  shipped path pays nothing.

### What a valid gate needs

1. Clippy fixtures labeled by an oracle **independent of our proposers** —
   real `cargo clippy` output. The harness already exists for the `self_evolve`
   G7 gate (Bench 024), so this is wiring, not invention.
2. Spans where **several lints genuinely co-apply**. Zero-cross-fire is the
   artificial part; real code trips multiple lints, and that is precisely
   where a re-ranker either earns its place or does not.

Until then the honest claim is "mechanism sound, effect size unknown", and
`Off` is the correct default.

---

## T3.5.5 — the uncontaminated oracle: G1 BAR MET, re-ranker PROMOTED

### What the independent oracle actually found

`src/draft/clippy_oracle.rs` asks real `cargo clippy`
(`--message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery`)
what fires on a span, intersects with the corpus vocabulary, and reports
compile failures separately (a span rustc rejects yields no trustworthy
labels). Clippy knows nothing about our corpus, proposers, or scores.

Run over the 48 existing clippy fixtures:

| | result |
|---|---:|
| compile under rustc | 43/48 (89.6%) |
| **our label confirmed by real clippy** | **43/43 (100.0%)** |
| real clippy reports no corpus lint | 0/43 |
| **spans tripping >1 corpus lint** | **0/43 (0.0%)** |

**This overturns half of T3.5.4's diagnosis.** The labels were never wrong —
an oracle with no connection to our proposers confirms every one. The defect is
**sample selection**: every fixture is a *minimal single-lint example*, so
exactly one rule can possibly be right and a ranker has nothing to decide.
Re-running the G1 against real labels reproduced the degeneracy exactly
(`all` top-1 97.7% == pool recall 97.7%), confirming the fixture *shape*, not
the labels, was the blocker.

Side finding: 5 fixtures (`needless_return`, `collapsible_if`, `single_match`,
`comparison_to_empty`, `semicolon_if_nothing_returned`) do not compile, so no
independent oracle can validate them. Recorded, not fixed here.

### The missing sample, authored

`src/draft/eval_fixtures_composite.rs` — 28 spans that trip several corpus
anti-patterns at once, the way real code does. **Deliberately not hand-labeled:**
the labels come from the oracle at measurement time, because hand-labeling
would reintroduce the coupling T3.5.5 exists to remove. 27 compile, **25 are
genuinely multi-lint** per real clippy. The second batch pairs non-`return`
rules, because the first leaned on `needless_return` as the partner (16 of 19
sets) and a ranker biased toward one popular rule would have scored well.

### Uncontaminated G1 (25 multi-lint spans, real-clippy labels)

| mode | top-1 | top-3 |
|---|---:|---:|
| `Off` | 84.0% | 96.0% |
| **`Structural`** | **96.0%** | 96.0% |
| `All` | 96.0% | 96.0% |

**+12.0pp top-1**, and the degeneracy check now *fails to fire*: `all` top-1
(96.0%) **≠** pool recall (100.0%). Retrieval already has a true rule in the
pool for every span, so top-1 is purely an ordering decision — exactly the
thing the old fixture could not test. The gain is the re-ranker's.

`Structural` ties `All` while excluding the known over-matchers, so the safer
arm is also an equal-best one.

### GOAT gate

| gate | result |
|---|---|
| **G1 quality** | **PASS** — +12.0pp top-1 on uncontaminated multi-lint evidence |
| **G2 latency** | **PASS** — +52-64 µs/span (+5.6-7.2%) for one parse + K visitor walks |
| **G3 no-regression** | **PASS** — 415 / 461 / 481 / 471 lib tests green (default / `latent_retrieval` / +`clippy_verify` / `fix_verify`), clippy 0 warnings |
| **G4 bounded alloc** | **PASS** — one `syn::File` per span; no growth in the candidate path |
| **modelless** | **YES** — `syn` visitors, deterministic, no weights or training |

**→ `RerankMode::Structural` PROMOTED TO DEFAULT.** Pinned by an assertion so a
silent revert is caught.

### Phase 3 G1 bar: MET

| measurement | clippy top-1 | ≥80% bar |
|---|---:|:--|
| committed eval (single-lint fixture, inflated) | **87.5%** | PASS |
| uncontaminated composites (real-clippy labels) | **96.0%** | PASS |

Both clear it, and the honest one clears it by more. Committed eval after
promotion: clippy 87.5/93.8/93.8, perf 97.9/97.9/97.9, blended 92.7/95.8/95.8.
Floors rebased to measured − 1 (blended 88/91/91 of 96, clippy 41/44 of 48).

The committed clippy rows remain inflated by the fixture shape and are a
**regression floor, not an effect size** — the promotion rests on the composite
measurement.

---

## Reproduce

```bash
# T3.5.5 uncontaminated oracle (~10 s warm, spawns cargo clippy per span)
cargo test --release --features latent_retrieval,clippy_verify \
  --test uncontaminated_g1 -- --nocapture --test-threads=1

# options 1-2 + T3.5.4 (seconds)
cargo test --features latent_retrieval --lib rerank_sweep_report -- --nocapture
cargo test --features latent_retrieval --lib rerank_circularity_diagnostic -- --nocapture
cargo test --features latent_retrieval --lib retrieval_sweep_report -- --nocapture
cargo test --features latent_retrieval --lib retrieval_precision_regression_gate -- --nocapture
cargo test --features latent_retrieval --lib router -- --nocapture

# option 3 (~1 h, 6.7 GB GGUF; run the cost probe first)
cargo test --release --features bonsai_retrieval --test bonsai_retrieval_probe probe_cost -- --nocapture
cargo test --release --features bonsai_retrieval --test bonsai_retrieval_probe dense_rerank_precision -- --nocapture --test-threads=1
```
