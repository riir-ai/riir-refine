# Bench 064 — Issue 64 Phase 2: the modelability ↔ downstream top-1 Spearman sweep (GenFirst fusion)

**Status:** RECORD — pre-registered protocol executed 2026-09-02; **verdict: NEGATIVE (kill branch, ρ = 0.2256 < 0.5)** — the scaffold-value oracle does not predict downstream retrieval top-1 on the healer's real retrieval populations. The oracle primitive (Phase 1) and this harness are kept as the standing artifacts per the pre-registration's falsification clause.

**Harness:** `tests/modelability_sweep.rs` (feature `latent_retrieval` + `clippy_lints` + `rust_perf`; add `kernel_heal` for the full 12-point surface).
Re-run: `cargo test --features latent_retrieval,kernel_heal --test modelability_sweep -- --nocapture` (~42 s debug).

## Protocol (pre-registered in Issue 64 — executed, not restated)

Sweep ≥ 4 corpus/embedder variants on the retrieval surface; metric = tie-corrected Spearman rank correlation between `scaffold_value` (the Phase 1 null-calibrated k-means/Lloyd scaffold marginal value) and measured downstream top-1 across variants. **Gate ≥ 0.8** ⇒ the oracle predicts where embedder investment pays. **< 0.5** ⇒ the oracle is noise on our populations — record negative, keep the harness as the artifact.

## Wiring-map correction (recorded before execution)

The issue's Phase 2 recon map proposed the `IndexTokenizer` ladder × domain surfaces as the variant axis. That axis is **incapable of testing the oracle**: the BM25 tokenizer touches postings and queries only — never the `ModellessEmbedder` embeddings the oracle measures. Three tokenizer variants over one domain are three points with an identical x (tie-slotted by construction). The protocol's own wording ("≥ 4 **corpus/embedder** variants") names the correct axis; the map's variant identification was the error, caught at harness-design time and executed as the secondary below.

## Sweep points (the corpus/chunking axis — the levers the issue names)

12 population-distinct points with `kernel_heal` (8 without): per domain (clippy / perf / kernel),
`full-ms` (multi-scale index, shipped shape) × `slice-a` / `slice-b` (deterministic even/odd halves of the rule-id-sorted rule set; top-1 over the slice's own labeled spans) × `full-ss` (single-scale index — whole patterns only, the pre-multi-scale shape).

Oracle population = the index chunk set replicated exactly (whole text + significant trimmed > 8-char lines, content-deduped in first-occurrence order — the substrate upserts duplicate zone hashes), embedded by the same `ModellessEmbedder`. Defaults: k=8, holdout 1/5, 32 Lloyd iters.

Measurement note: top-1 here is the **raw ranker** (`RuleIndex::suggest` + the canonical score/rule-id ordering) — deliberately WITHOUT the instantiation re-ranker, uniform across all 12 points (the correlation needs a consistent ranker, not the production stack). Cross-check against the recorded floors via the standard eval (tokenizer table below): clippy 89.6% / perf 96.2% / kernel 100.0% at op-bigram — matches `retrieval_precision_regression_gate` exactly, so the eval side of the harness measures the known surface.

## Results

```
point             chunks  spans     v_raw      null     value    top-1
clippy/full-ms        79     48     0.879     0.654     0.225    64.6%
clippy/slice-a        42     26     0.608     0.573     0.036    73.1%
clippy/slice-b        38     22     0.526     0.683    -0.157    72.7%
clippy/full-ss        45     48     0.552     0.754    -0.202    64.6%
perf/full-ms         622     53     0.850     0.769     0.081    94.3%
perf/slice-a         325     25     0.835     0.659     0.176    88.0%
perf/slice-b         307     28     0.831     0.750     0.080   100.0%
perf/full-ss          98     53     0.793     0.807    -0.014    94.3%
kernel/full-ms      4766     30     0.887     0.829     0.058   100.0%
kernel/slice-a      2355     15     0.880     0.781     0.099   100.0%
kernel/slice-b      2490     15     0.859     0.814     0.045   100.0%
kernel/full-ss       453     30     0.934     0.788     0.146   100.0%

Spearman rho(oracle value, top-1) = 0.2256 over 12 points — NEGATIVE (kill branch)
domain-axis-only rho = -1.0000 over 3 full-corpus points (indicative, tiny n)
```

Secondary — the map's literal (domain × tokenizer) axis, executed for the record:

```
clippy   code-default  v=0.225  77.1%   op-aware 85.4%   op-bigram 89.6%
perf     code-default  v=0.081  84.9%   op-aware 98.1%   op-bigram 96.2%
kernel   code-default  v=0.058 100.0%   op-aware 100.0%  op-bigram 100.0%
tie-corrected rho = -0.8577 — x tie-slotted by construction (3 tokenizers share one
population per domain); the axis measures the tokenizer, not the space
```

## Verdict + mechanism analysis (why the oracle is blind here)

1. **The real populations are near-structureless by this measurement.** Every point's excess structure sits in −0.20…+0.23 nats; Phase 1's synthetic clustered populations score > 1.5 excess nats. The 8-D modelless BLAKE3+DFT space carries little exploitable k-means structure on any corpus variant — the pre-flight diagnostic delivering exactly its intended message: **do not invest embedder/density-scaffold effort on this space.**
2. **Residual value conflates k/n quantization headroom with structure.** k is fixed at 8 while n spans 38→4766 chunks: clippy's 79 chunks under 8 centroids quantize finely (large raw AND large null → the residual 0.225 is a small-population artifact, not richer structure). The diagonal-matched null corrects the shapeless part, not the k/n scaling part.
3. **Downstream top-1 on this surface is not latent-shape driven at all.** It is BM25 token discriminability (clippy's short near-identical patterns are hard for ANY ranker; the tokenizer ladder moves it 77→90%) plus span construction (kernel spans are identifier-renamed corpus excerpts — top-1 ceiling-pinned at 100% regardless of shape). Both consistent with the already-measured refutations: T3.5's latent-weight sweep (the 8-D latent term never flips a ranking at any weight) and Issue 020's ~70% lexical routing ceiling. On the domain axis the oracle is ANTI-correlated (ρ = −1.0): the "most structured" population is the worst retriever.
4. **What would change the verdict:** a population axis where the latent channel actually drives top-1 — e.g. the 768-D Bonsai dense space (`dense_weight > 0`, requires the GGUF) or a future trained embedder (the Plan 524 Phase 7 gate). The harness is parameterized for re-measurement when such a surface exists; the [[test]] row + harness stay.

## Disposition

- Issue 64 resolved + removed per the noise-reduction rule (Phase 1 landed `067b725`; Phase 2 recorded here).
- `src/modelability.rs` keeps `scaffold_value` (synthetic-validated) + `rank_correlation` (new, tie-corrected Spearman, unit-gated) with the real-population verdict pinned in the module doc.
- `tests/modelability_sweep.rs` + the `[[test]]` row stay as the standing re-measurement instrument (the pre-registration's keep-the-harness clause).
