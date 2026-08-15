# Issue 020 — DomainRouter locality POC: spectral routing embedding

**Status:** OPEN (low priority — annotation-quality win, not load-bearing)
**Origin:** User wild-idea verdict session (2026-08-15). The idea: apply
Research 001 Fourier Spatial Search (`riir-ai/.research/001_Fourier_Spatial_Search.md`)
as the clustering/indexing substrate for riir-rag + this repo's domain
routing. Verdict landed: spatial hashing **NO** (see verdict notes), the
surviving 10% is this POC — a *spectral* (token-frequency DFT) routing
embedding to fix the router's locality deficit.

## Background (the empirical finding this targets)

Plan 005 / Bench 018: the `DomainRouter` (8-D per-domain centroids, argmax
dot) **mis-routed fixtures** — a `return expr;` fn routed to perf, a
linear-scan fn routed to clippy. Retrieval therefore **fans out to every
enabled domain**; the router verdict is annotation-only. The accepted
limitation is documented as "re-gating via tuned centroids or the 768-D
embedder is a future tuning option".

**Root cause:** the routing embedding is BLAKE3-derived
(`riir-ai/crates/riir-rag/src/embedder.rs::ModellessEmbedder` — 4 BLAKE3
scars + 1 DFT byte-histogram scalar + 3 structural scalars). BLAKE3 has
hash avalanche → **zero locality**: similar code spans get dissimilar
embeddings → coarse centroids → mis-routing. This is structural, not a
tuning problem; tuned centroids cannot fix embeddings without locality.

## The POC

Give the `DomainRouter` a **router-local spectral block**: k additional
DFT coefficients (k ≈ 4-8) computed over a *token-frequency* vector
(top-N tokens), appended to (or replacing) the routing embedding. Token
distributions have locality by construction (similar code → similar token
spectra), which is exactly what BLAKE3 lacks.

**Router-local by design — do NOT touch riir-rag's 8-D contract:**
- `ModellessEmbedder`'s 8-D shape is pinned by `ShardIndex::retrieve_diverse(&[f32; 8])`
  (riir-neuron-db, Clifford-wedge KNN, feature `diverse_retrieval`) —
  expanding it ripples into riir-neuron-db's algebra wiring + Bench 018's
  G2 numbers (123µs p50 KNN, 349µs/chunk total).
- The spectral block lives in `src/draft/domain_router.rs` (or a sibling),
  computed at corpus-index time from the pattern texts. riir-rag is not
  modified.

## Falsifiable gate (the kill criterion is the point)

Re-run the Plan 005 mis-routing fixtures:
- [ ] The two documented mis-routes (`return expr;` → perf, linear-scan → clippy) route correctly with the spectral block.
- [ ] The 12-item G1 fixture (Bench 018) top-1 routing accuracy ≥ current fan-out annotation baseline on the same fixtures.
- [ ] Determinism: same input → same routing verdict (no wall-clock, no RNG).
- [ ] Cost: routing stays < 10µs/span (it runs per-span in `--suggest`; the current argmax is ~free; k DFT coefficients over top-N tokens is O(N·k) once per span).

**KILL:** if token spectra fail to separate domains on those fixtures
(plausible — token-frequency similarity is lexical, domain membership is
semantic: a perf rule about HashMap and a clippy rule about HashMap share
a token spectrum), close this issue with the negative result recorded.
Fan-out retrieval stays the shipped behavior; the router stays
annotation-only. The negative result is valuable either way — it closes
the "future tuning option" question for the spectral branch.

## Non-goals (already verdict-NO, do not re-litigate here)

- **Fourier *spatial* hashing** for code indexing — Research 001's own
  dead-ideas table: "cross-domain Fourier merge = interference" +
  "Fourier cosine similarity ≠ semantic distance unless trained". The
  spatial machinery (translation/rotation equivariance, grid adjacency)
  presumes position structure code doesn't have. It lives in
  `riir-engine/src/fourier/` (26 files) where it belongs.
- **L1 (TernaryDrafter) → L3 (RuliologySearch) replacement** — L1
  generates/selects candidates, L3 ranks them post-hoc; complementary
  layers, not substitutes. WASM adds nothing to ~30 LOC native Rust
  running once per invocation.

## Value assessment (why low priority)

The router is annotation-only; fan-out works; retrieval runs once per
`--suggest` invocation (not per-tick). Even a perfect gate saves a
fraction of the 349µs/chunk fan-out cost. The real payoff is downstream:
IF spectral routing passes, a trusted router could later gate retrieval
(one domain instead of N) — but that's a separate decision with its own
gate.
