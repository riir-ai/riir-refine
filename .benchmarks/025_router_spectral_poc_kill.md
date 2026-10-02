# Bench 025 — Issue 020: DomainRouter spectral-routing POC (KILLED at the gate bar)

**Date:** 2026-08-16
**Issue:** `020` (resolved-KILLED, removed per noise-reduction rule)
**Verdict:** **KILL for gating** — the best token-frequency variant reaches 70.3% routing accuracy, 15pp under the 85% kill-bar. **Salvage shipped:** the annotation-only router now uses the measured-best variant (TF-cosine, +20pp annotation accuracy at ~3.4× lower cost). Fan-out retrieval stays the shipped behavior; the "future tuning option" question is CLOSED for the lexical/spectral branch.

## The question

`DomainRouter`'s baseline embedding is `ModellessEmbedder`'s 8-D direction — 4 of whose dims are BLAKE3 hash buckets with **zero locality** (similar code → dissimilar embedding). Measured annotation accuracy (Bench 022): **53.4%** (2-domain) / **46.6%** (3-domain). The POC: a router-local **spectral block** — k DFT coefficient pairs over a **token-frequency vector** (top-N tokens across the enabled domains' corpora), where token distributions have locality by construction. Kill-bar (Bench 022's recommendation): **≥85%** routing accuracy on the same 88/118-span fixture set — below that, gating trades correctness for a fraction of the 349µs/chunk fan-out cost.

## Design (as specified by the issue, + one ablation)

- Vocab N=128 tokens (runs of `[A-Za-z0-9_]`, len ≥ 2, case-sensitive), built at router-construction from ALL enabled domains' pattern texts. Deterministic ranking: (score desc, token asc) — BTreeMap counting, lookup-only HashMap.
- Two orderings (the DFT axis): `GlobalFreq` (global count) and `Discriminative` (cross-domain TF range `max_d p_d − min_d p_d` — domain-separating tokens at low indices where low-k coefficients see them).
- DFT block: k=1..=8 coefficient pairs (re+im, 16 dims), twiddle tables precomputed at construction (route-time is pure FMAs), L2-normalized. Optional structural-4 suffix (the 8-D embedding's dims 4-7, BLAKE3 dims dropped).
- **Ablation reference:** `TfCosine` — full-128 TF cosine, no DFT compression. Upper-bounds any projection of the lexical signal.

## Measurement (the one-command harness: `cargo test --features latent_retrieval[,kernel_opt] --lib router_spectral_poc -- --nocapture`)

### 2-domain state (clippy + perf, 88 labeled spans)

| variant | accuracy | mis-routes |
|---|---|---|
| baseline-8d | 47/88 = **53.4%** | fixed |
| dft8-global | 62/88 = 70.5% | NOT fixed |
| dft8-global+struct | 66/88 = **75.0%** | NOT fixed |
| dft8-disc | 59/88 = 67.0% | fixed |
| dft8-disc+struct | 59/88 = 67.0% | fixed |
| tf-cosine-global | 61/88 = 69.3% | fixed |
| tf-cosine-disc | 60/88 = 68.2% | fixed |

Baseline reproduces Bench 022's 53.4% **exactly** — the harness is valid.

### 3-domain state (clippy + perf + kernel, 118 labeled spans)

| variant | accuracy | mis-routes |
|---|---|---|
| baseline-8d | 57/118 = 48.3% | fixed |
| dft8-global | 57/118 = 48.3% | NOT fixed |
| dft8-global+struct | 55/118 = 46.6% | NOT fixed |
| dft8-disc | 82/118 = 69.5% | fixed |
| dft8-disc+struct | 79/118 = 66.9% | fixed |
| tf-cosine-global | 81/118 = 68.6% | fixed |
| tf-cosine-disc | 83/118 = **70.3%** | fixed |

(3-domain baseline 48.3% vs Bench 022's 46.6% — small drift from Issue 022's 5-entry corpus correction moving the centroids; within reproduction tolerance.)

### Cost (release, per span; gate < 10µs) — PASS

| mode | cost |
|---|---|
| Baseline | 3.31 µs |
| SpectralDft disc+struct | 4.06 µs |
| TfCosine disc | 0.96 µs |

### Gate items

- [x] The two documented mis-routes route correctly under the spectral block (disc/tf variants; note: the current-corpus baseline ALSO routes them correctly — those two fixtures are no longer the live failure mode; overall accuracy is)
- [x] Accuracy ≥ the current baseline (70.3% > 53.4%/48.3%) — the issue's original gate PASSES
- [x] Determinism (BTreeMap vocab + lookup-only HashMap + precomputed tables; asserted in tests)
- [x] Cost < 10µs (0.96–4.06µs release; the gate is release-only — debug pays ~10× on the transcendental paths, the `g2_*` pattern)
- [ ] **Accuracy ≥ 85% (the kill-bar) — FAIL. KILL.**

## The finding (why it failed — the precise negative result)

**The k-coefficient DFT compression is NOT the bottleneck; the lexical signal itself is the ceiling.** The ablation splits it cleanly:

- Full-128 TF cosine (no compression): 70.3% / 69.3%
- DFT-k8 over the discriminative vocab: 69.5% / 67.0%

They are within noise of each other — with domain-separating tokens ordered into the low indices, the low-k DFT coefficients carry essentially all of the lexical signal. Compressing further changes nothing because there is nothing more to carry: **domain membership is semantic, not lexical**. A perf rule about `HashMap` and a clippy rule about `HashMap` share a token spectrum — the issue's predicted failure mode, now measured. The only plausible future re-gate path is a semantic-signal router (the 768-D embedder or similar); the lexical branch is closed.

Secondary observations:
- The structural-4 suffix helps the global ordering (+4.5pp) but not the discriminative one (−2.6pp) — orthogonal signal already captured by token choice.
- `dft8-global` WITHOUT struct (70.5% 2-domain) still mis-routes both documented fixtures — raw frequency-ranked vocab is the weakest axis ordering.

## What shipped

1. **The measurement harness** (kept, standing): `run_router_eval` + the `router_spectral_poc_report` test re-measure every variant per corpus batch; the `RoutingEmbedding`/`SpectralVocab` machinery in `domain_router.rs` is the reproducible negative-result artifact (the `gemma2_directions` precedent — Bench 571). Baseline stays available as a mode.
2. **The annotation salvage** (production change, annotation-only): `LatentMatcher`'s router now builds with `TfCosine { order: GlobalFreq }` — annotation accuracy **53.4%→69.3%** (2d) / **48.3%→68.6%** (3d) at **0.96µs vs 3.31µs** per span. `tf-cosine-global` over `tf-cosine-disc`: equal accuracy across states (69.3/68.6 vs 68.2/70.3) but the global-frequency ordering is insensitive to the enabled-domain set — more predictable under feature/corpus churn. Gating is NOT enabled — retrieval stays fan-out.
3. **A standing floor** on the annotation accuracy in the Issue 017 regression gate: ≥63.6% both states (measured − 5 spans of corpus-growth headroom). Decay re-measures + re-pins per the established contract; never silently weakens. This is the per-batch telemetry any future re-gate decision consumes.

## GOAT

- **G1 determinism:** PASS — vocab (BTreeMap + deterministic tie-breaks), embed (integer counts + precomputed tables), routing (argmax with pick_domain tie semantics); `router_spectral_poc_report` asserts table-identical repeat runs; `eval_report_is_deterministic` covers the wired router.
- **G2 cost:** PASS — 0.96–4.06µs/span release (bar: <10µs). Debug-mode exclusion documented (`g2_*` pattern).
- **G3 no-regression:** PASS — 408 default / 435 latent_retrieval / 592 combined lib tests; clippy 0 warnings in all states; retrieval floors unchanged and passing.
- **G4 alloc:** PASS — route-time is stack-resident (fixed `[u32; 128]` counts + `[f32; …]` vectors; no heap); the vocab HashMap is lookup-only.
- **The POC's own gate:** KILLED at the ≥85% bar (best 70.3%) — recorded above as the deliverable.

## Follow-ups (not filed — below the value bar)

- Sub-70% residual: dominated by clippy↔perf confusion on shared-vocabulary shapes (`return` inside scans, `unwrap`/`clone` in both corpora). A semantic embedder is the only measured-plausible path; that's a riir-rag/C13-scoped decision, not a router patch.

## Environment

- M3 Max, macOS, Rust 1.97 toolchain (matching the Issue 022 corpus-drift toolchain).
- GPU-exclusivity: N/A (CPU-only measurement; no other cargo processes running).
- Build: `CARGO_TARGET_DIR=/tmp/riir-clippy-i020` (isolated, cleaned post-commit).
