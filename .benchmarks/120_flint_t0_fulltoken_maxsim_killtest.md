# Bench 120 — Issue 169 T0: the FLINT full-token MaxSim kill-test (KILL — negative)

**Status:** RECORD — measured 2026-10-24. **VERDICT: KILL.** Full-token MaxSim
over hashed op-token embeddings does NOT beat the shipped single-vector matcher
at any width — best (e) arm 85/141 (60.3%) blended top-1 at the widest D=64 vs
arm (a) 124/141 (87.9%); paired +3 wins / −44 losses / 94 ties. Per Issue 169's
pre-registered kill rule, T1 (pooling constructors) is never built and
riir-train Issue 647 (the trained-pooler control) stays closed — its premise
required BOTH (b)>(a) AND (e)>(a).

> **Source:** Research 309 (arXiv:2610.27273, FLINT) → Issue 169 T0. Harness:
> `examples/multivector_poc.rs` + `src/draft/multivector.rs` behind
> `multivector_poc` (implies `latent_retrieval`, forwards
> `katgpt-core/maxsim` — never default). Issue file removed per the
> noise-reduction rule; `HISTORY.md` carries the durable row.

## Box state / provenance

M3 Max (16-core, macOS 27.0.1), `--release`, AC power. ⚠ Load-disclosed: FIVE
sibling agent sessions were active during the runs (cargo builds among them) —
wall-clock times are context, not a gate; the quality axes are deterministic
computations and load cannot move them (two full runs byte-agreed on every
count and digest). G2's formal bench is T5, never reached (killed at T0).

## Setup (fair population, one binary, one feature state)

Default features (`clippy_lints, rust_perf, self_evolve, docker,
rustc_errors, dist` + the PoC set): 4 retrieval domains, **416 rules**
(clippy 112, perf 266, docker 31, dist 7), **141 labeled spans**, 21,275
scored (span, rule) pairs per (e) run.

- **Arm (a)** — the shipped `LatentMatcher` (8-D latent + BM25 + two-stage KNN
  + instantiation rerank), measured by `retrieval_eval::run_eval` AND by the
  harness's paired loop; the two agree EXACTLY (blended + per-domain assert).
- **Arm (e)** — every rule of the span's domain scored by the shipped
  `maxsim_score` (katgpt-core `simd`, Plan 080) over per-op-token hashed
  embeddings: BLAKE3 XOF → D u32 → f32 ∈ [-1,1] → L2 (scheme digest pin
  `3466d6ed36f96a0c`, unit-test-pinned). Tokenizer = the shipped
  `RustOpTokenizer`; doc side = the rule's `buggy_code` op-tokens (dedup is
  EXACT under MaxSim — duplicate rows are identical), query side per
  `QueryMode` (`All` = every token, count-weighted — the literal pinned arm;
  `Unique` = one vote per type). `Integration::Full` = the shipped pipeline
  shape with ONLY the score channel swapped (pool 37 → same instantiation
  marking + rerank + truncate); `Integration::Raw` = pure MaxSim ranking.
  Both sides share ONE vector space per the issue's T1 rule-side spec.
- Width sweep D ∈ {8, 32, 64} per G4 (an 8-D negative alone would be
  confounded by hashing collisions; the negative counts only at the widest D).

## Results

Arm (a) blended: **top-1 124/141 (87.9%) · top-3 131/141 (92.9%) · top-5
133/141 (94.3%)** — per-domain: clippy 42/48, perf 52/56, docker 26/31,
dist 4/6.

Arm (e), blended top-1 (top-3 / top-5 in parens):

| D | Posture | Query | top-1 | paired vs (a) |
|---|---|---|---|---|
| 8 | Full | All | 63/141 44.7% (49.6/53.2) | +1 / −63 / 77 |
| 8 | Full | Unique | 63/141 44.7% (49.6/53.2) | — |
| 8 | Raw | All | 36/141 25.5% (37.6/44.0) | — |
| 8 | Raw | Unique | 37/141 26.2% (38.3/44.0) | — |
| 32 | Full | All | 75/141 53.2% (68.1/73.0) | +2 / −52 / 87 |
| 32 | Full | Unique | 79/141 56.0% (68.1/75.2) | — |
| 32 | Raw | All | 49/141 34.8% (58.2/66.7) | — |
| 32 | Raw | Unique | 56/141 39.7% (59.6/68.8) | — |
| **64** | **Full** | **All** | **84/141 59.6% (72.3/76.6)** | **+3 / −44 / 94** |
| 64 | Full | Unique | 85/141 60.3% (73.0/76.6) | — |
| 64 | Raw | All | 60/141 42.6% (63.8/70.2) | — |
| 64 | Raw | Unique | 65/141 46.1% (65.2/72.3) | — |

(e) − (a) at EVERY D (best arm per width, blended top-1): **D=8 −61 spans,
D=32 −45, D=64 −39.** Per-domain at D=64 Full/All: clippy −7, perf −27,
docker −6, dist +0. The kill rule holds at the widest D (G4's condition) and
at every width.

## Why it loses (mechanism, not tuning)

The hashed per-token embedding is a RANDOM direction per token string: identical
tokens match exactly (dot = 1) and DIFFERENT tokens are near-orthogonal — so
MaxSim over hashed tokens degenerates to an UNWEIGHTED token-overlap count,
with no IDF (ubiquitous `fn`/`(`/`)` count exactly like a discriminative
`op:&*` run) and no semantic similarity between related-but-distinct tokens.
That re-creates the tiny-token-crowding defect the `op_tokenizer` module docs
describe for BM25, now inside the MaxSim scorer. FLINT's per-token vectors are
TRAINED-encoder outputs where related tokens have related directions; this
repo's 8-D embedder carries span-level statistics only and cannot produce
per-token views at all — exactly the failure mode Issue 169's "where it can
fail" list pre-registered ("per-token views need per-op-token hashed
embeddings … which changes what 'token embedding' means"). The width sweep
confirms the collision confound is bounded but NOT binding: the trend is
monotone (44.7 → 56.0 → 60.3) yet the gap to (a) stays 27.6 pp at D=64 and the
binding constraint is the representation class, which width cannot fix.

Secondary reads, all consistent:

- The instantiation rerank channel carries a large share of the Full posture
  (D=64: Full/All 84 vs Raw/All 60) — the symbolic channel rescues a third of
  the slate on top of a weak similarity ranking; arm (a)'s rank channel is
  simply far stronger at the job.
- `Unique` ≥ `All` at every width (count-weighting by ubiquitous tokens
  hurts, mildly) — T1's salience-weighting axis had at most a ~1-span lever
  here, nowhere near the 39-span gap.
- Clean-excerpt profile (D=64 Full/All): the MaxSim slate names probeable
  fixable-lint rules on clean code (`len_without_is_empty`,
  `manual_let_else`, `chunks_exact_to_as_chunks`, …) — the unweighted-overlap
  false-positive surface, the same shape arm (a)'s floors guard against.

## Cost disclosure (G2-class, parity bar — load-disclosed, NOT a gate)

Release, loaded box (five sibling agents): (e) D=64 scoring 238 ms / 21,275
pairs ≈ **11.2 µs/pair** (All) / **7.9 µs/pair** (Unique); arm (a) 521 ms
total ≈ 3.7 ms/span. The linear-scan MaxSim is parity-class at this corpus
scale — but a parity cost on a −39-span quality loss is not an index-candidate,
which is the whole reason the kill rule runs first.

## Determinism

Two independent runs byte-agree on every count, paired row, and corpus digest:
D=8 `c22b6cee284cc1f3`, D=32 `4783e8af18b3591f`, D=64 `453020710ff90287`
(BLAKE3 over scheme tag + D + every rule id + matrix bytes, canonical order).
Ranking is `total_cmp` + `rule_id` — the house canonical order. Corpus digests
are corpus-state-bound (they move with the next corpus batch by design; the
SCHEME digest pin `3466d6ed36f96a0c` is corpus-independent and unit-tested).

## Standing context observed en route (not this record's lane)

Arm (a) measures **124/141 = exactly AT `BLENDED_TOP1`** (the floor's own
convention is measured − ONE span; the standing note last measured 125 at
Batch 244). Corpus batches 245–271 have grown the perf pool since; the
zero-margin state means the next corpus landing owes the Issue-063 one-span
repair per the floor's own text. Recorded here as the paired baseline's
context; the re-pin belongs to the next corpus-batch lane. **Filed as
`.issues/170_retrieval_gate_red_router_annotation_below_floor.md`** — which
also carries the WORSE leg found right after this record landed: the gate's
router-annotation axis is ALREADY below floor (60.3% < 72.3%, verified at
plain `latent_retrieval`, pre-existing), and the gate only compiles under
`latent_retrieval`, so the standing default-features test lane cannot see
either leg.

## Honest residuals (what this negative does NOT claim)

- **MaxSim-as-ensemble-channel is untested** — adding a MaxSim score beside
  latent+BM25 (a fusion weight, not a replacement) is a DIFFERENT claim from
  Issue 169's ("the multi-vector representation improves retrieval OVER the
  single-vector matcher") and is not measured here. Nothing observed suggests
  headroom: the strongest integration loses +3/−44 paired.
- **A trained per-token code embedder** would change the geometry this kill
  measured (random directions → learned directions), but that is the training
  lane, and Issue 647's own gate (the deterministic pooler must beat (a)
  first) is now permanently failed — no trained-pooler path re-opens on these
  fixtures without new evidence.
- The paper's 128-d projections rode a trained encoder; the pinned sweep ends
  at D=64 per Issue 169 G4. Extrapolating the monotone trend to (a)'s 87.9%
  would need roughly D≈256+ on trend AND still carry the no-IDF/no-similarity
  mechanism — recorded, not chased.

## Repro

```sh
cargo run --release --features multivector_poc --example multivector_poc
cargo test --features multivector_poc --lib draft::multivector   # 4 gates incl. the scheme pin
```
