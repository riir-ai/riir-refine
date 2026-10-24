# Issue 169 — PoC: fixed-budget deterministic multi-vector span matching (FLINT-style, modelless)

**Status:** OPEN — filed 2026-10-24 from Research 309 (arXiv:2610.27273, FLINT).
Primary modelless arm; the trained-pooler control arm is riir-train Issue 647
(contingent on this issue's read). Paired-read discipline REQUIRED: `set_rerank`
(Issue 121 / Bench 096) is this repo's measured-negative retrieval-layer
precedent — an unpaired delta does not adjudicate this class.

## Claim under test (pinned, one sentence)

Deterministic fixed-V pooling of op-token embeddings (V ∈ {4, 8, 16} pooled
vectors per span, no training) scored by the shipped `maxsim_score` improves
oracle-labeled rule-retrieval quality over the shipped single-vector matcher —
specifically on long spans and the withheld-pair OOD axis — at index cost
independent of span length.

## Why this is expected to work here (and where it can fail)

- The retrieval query IS the long side (incoming code span vs short corpus
  rule) — FLINT's winning regime; token-level MaxSim is documented to collapse
  on long queries (SIGIR 2026 repro, arXiv:2604.09982).
- Coverage argument for selection-beats-truncation (NOT a monotonicity claim —
  adding anchors to the query side adds terms to the MaxSim SUM, and signed
  dots make neither the score nor the ranking monotone in the anchor set):
  FPS/k-means cover the token cloud where prefix truncation provably drops
  every non-prefix match. The truncation arm (c) carries this evidence
  empirically; nothing is claimed structurally.
- Honest failure modes: at V ≥ #op-tokens the pooled set degenerates toward
  the full token set (no compression win); the 8-D modelless signals may be
  too coarse for per-token views to add anything (the signals are span-level
  statistics — per-token views need per-op-token hashed embeddings from
  `op_tokenizer`, which changes what "token embedding" means — see T0); 8-D
  hashing collides heavily (the paper used 128-d projections), so a negative
  at 8-D alone is NOT evidence against the representation law — hence the
  width sweep in G4; and pooled vectors are MORE directionally concentrated
  (lower effective rank), which the paper found helps ANN cost but may hurt
  discrimination on a LINEAR-SCAN matcher — our matcher is not an ANN, so the
  paper's latency story does not transfer, only the quality story does.

## Gates (all four, or the negative is the answer)

- **G1 (quality, paired ABBA):** top-1/top-3/top-5 on `retrieval_eval` fixtures
  (all domains + blended) AND the `withheld_pair_ood` axis. Arms: (a) shipped
  single-vector incumbent, (b) fixed-V pooled + MaxSim (V ∈ {4,8,16}),
  (c) prefix-truncation floor (same V budget as tokens), (d) salience-weighted
  pooling variant (op-rank/norm/entropy weights from existing signals),
  (e) **full-token MaxSim ceiling — every op-token vector, NO pooling**.
  ORDERING: (e) is the kill-test and runs FIRST (T0) — if unpooled per-token
  MaxSim cannot beat (a) at the widest D, no deterministic pooler can, and T1
  is never built. Promotion bar: (b) beats (a) AND (c) with paired LB95 > 0 on
  the blended axis WITHOUT regressing any single-domain floor; the OOD axis
  must not regress; (e) > (a) is the PREREQUISITE for building (b)/(d) at all.
  A (b) ≈ (a) read with (b) > (c) is still a REPORT (the truncation law is
  confirmed, the upgrade is declined) — record it either way.
- **G2 (perf):** pool + score cost per (span, corpus) pair vs the incumbent
  matcher at current corpus scale (~1.4k rules × 8-D) — `--release`, preflight
  PROVENANCE line quoted. The bar is parity-class (µs), not a speedup claim:
  this is an index-representation change, not a latency play.
- **G3 (no regression):** default-off feature; the shipped matcher path
  byte-identical when off; `EmbedProvenance.model_id` discipline — a pooled
  arm gets its OWN model_id string; two vector spaces never mix in one index.
- **G4 (alloc + width sweep):** pooled set is const-generic `[[f32; D]; V]`
  with **D swept ∈ {8, 32, 64}** (hashed op-token width — an 8-D negative is
  confounded by hashing collisions, so **a negative counts only if it holds at
  the widest D**); index cost stays span-length-independent at every D
  (D×V×4 B/span: 128–1024 B at V=8). Scratch-buffer scoring through
  `maxsim_score`, zero alloc in the hot path; deterministic pooling (same
  input bytes → same pooled bytes, BLAKE3-committable); fixed first-index max
  tie-break per the Issue-940 determinism law.

## Tasks

- [ ] **T0 (kill-test, runs FIRST):** define the per-token object — hashed
  `op_tokenizer` op-token embeddings at D ∈ {8, 32, 64} (deterministic,
  BLAKE3-committable) — and run arm (e): full-token MaxSim over ALL of a
  span's op-token vectors vs arm (a), both sides in the SAME vector space
  (see T1's rule-side spec). If (e) ≤ (a) at the widest D: record the
  negative in `.benchmarks/`, update note 309, STOP — multi-vector does not
  work on this task and riir-train 647 stays closed.
- [ ] T1 (only on T0 pass): pooling constructors behind one feature (e.g.
  `multivector_poc`): frozen-direction sigmoid assignment (Hadamard rows /
  committed FPS anchors), farthest-point sampling, fixed-seed k-means (≤16
  iterations), salience-weighted strided selection — all deterministic, all
  `[[f32; D]; V]`. **Rule-side spec (both sides must share ONE vector
  space):** the corpus rule's vectors are the op-token embeddings of the
  rule's `buggy_code`/example-code shape (the same tokenizer, same D) —
  NEVER its prose description and never the single 8-D span vector repeated
  (prose-vs-code would be MaxSim across two unrelated spaces). The rule's
  existing single 8-D vector remains arm (a) unchanged. Each arm carries its
  own `EmbedProvenance.model_id` string (G3).
- [ ] T2: wire `maxsim_score` into the PoC matcher path (katgpt-core dep needs
  the `maxsim` feature on this arm only — check the dep already carries it or
  add it to the PoC feature set, never default).
- [ ] T3: paired harness run over `retrieval_eval` fixtures (5 arms × V-sweep ×
  D-sweep × ABBA order), committed as a gated test — NOT wired into the
  standing regression gate until promotion.
- [ ] T4: `withheld_pair_ood` paired read, both postures.
- [ ] T5: G2 bench + G4 alloc assert (`CARGO_TARGET_DIR=/tmp/...` per house
  rule when the workspace is hot).
- [ ] T6: verdict record in `.benchmarks/` (number from `.highwater`) + note
  309 status update; the record quotes (e) − (a) at EVERY D, not just the
  widest — if (e) wins only at D=64, the shipped 8-D embedder being too
  narrow is its own finding line, separate from the pooling verdict; trained
  arm (riir-train 647) unblocked only on BOTH (b) > (a) AND (e) > (a) reads.
- [-] T7 (deferred): erank-of-query adaptive rerank-pool k for the dense
  two-stage channel (`TWO_STAGE_LATENT_K` is a fixed 32; the file itself names
  the 48–64 recall-vs-latency trade) — only if T1's pooling survives, and
  measured on the dense lane, not this one.
- [-] T8 (deferred): game-side episodic-ring pooled Pods — no documented pain
  on that surface (Research 309 §5); needs a consumer first.

## Non-goals

No embedder re-training (that is 647's lane, owner-gated, dense channel only);
no score-threshold work (standing retrieval-eval non-goal); no ANN/candidate
index change (the matcher is a linear scan; the paper's cell-activation
economics do not apply at this scale).
