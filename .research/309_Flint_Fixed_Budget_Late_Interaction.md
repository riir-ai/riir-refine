# Research 309 — FLINT: Fixed-budget Late Interaction (arXiv:2610.27273)

**Status:** RECORD — distilled 2026-10-24; **T0 MEASURED 2026-10-24: KILL
(§7)** — the modelless multi-vector arm is closed by its own kill-test
(riir-refine Bench 120); the trained-pooler arm (riir-train Issue 647) is
closed with it. The GAIN verdict below was the PRE-measurement tier and is
superseded by §7 for the modelless track; the distillation record (mechanism
inventory, prior art, reframes) stands.
Primary modelless arm measured first, per its own pre-registration.
Source: Handa, Arabzadeh, Zaharia (UIUC/Berkeley), 2026-10-19,
<https://arxiv.org/abs/2610.27273>. Zero-GPU distill (panel + web searches only).

> **Related Research:** katgpt-rs Research 45 (MaxSim scoring — ADOPTED, shipped
> via Plan 080), katgpt-rs Research 213 (Still perceiver pooling — SHIPPED then
> DEMOTED LOSER in the KV-compaction slot), riir-refine Bench 115 (basis_cert —
> named "a token-level embedder swap" as the unclaimed remedy axis for the 8-D
> embedder).
> **Related Plans:** katgpt-rs Plan 080 (maxsim_score — GOAT PASS, in `katgpt-core`
> SIMD + `riir-gpu` WGSL + `katgpt-attn-match::rerank`), katgpt-rs Plan 245
> (still-perceiver, demoted).
> **Cross-ref:** riir-train Issue 647 (trained-pooler PoC), riir-neuron-db
> `riir-rag` (the `Embedder`/`RetrievalProvider` trait surface any pooled arm
> lands behind), katgpt-core `simd::maxsim_score` (the scoring substrate).
> **Classification:** Private (riir-refine).

---

## TL;DR

FLINT compresses BOTH sides of a ColBERT-style late-interaction retriever to a
FIXED number V of vectors per query/document via a Perceiver-Resampler-style
cross-attention pooling module (V learned latents attend over the backbone's
token outputs; MaxSim scores the V×V set). Storage and search cost become
independent of input length: 5.3–9.3× smaller indices than ColBERTv2, search
latency nearly constant in query length (0.004 vs 0.31 ms/token vs ConstBERT,
which pools documents only), and its largest quality wins are exactly on LONG,
complex, out-of-distribution queries (CRUMB, TREC ToT 2025, BrowseComp+) —
where token-level MaxSim is documented to collapse (SIGIR 2026 reproduction,
arXiv:2604.09982).

**Distilled for this stack (modelless, inference-time):** the transferable
mechanism is NOT the trained pooling module — it is the representation law:
"a fixed V-vector set per item, scored by MaxSim, beats a single mean-pooled
vector on long/multi-constraint queries at length-independent index cost".
That law is testable with ZERO training on this repo's oracle-labeled retrieval
fixtures, using the already-shipped `maxsim_score` (Plan 080) and deterministic
poolers (frozen-direction sigmoid assignment, farthest-point sampling, k-means).
Our long side is the QUERY side: the incoming code span is long; the corpus
rule is short — precisely the paper's winning regime. This is the concrete,
cheap resolution of the Bench-115 remedy axis ("a token-level embedder swap,
owner-gated, unclaimed") — and it is cheaper than the swap the bench named:
token-level information ACCESS at fixed-V budget, no new model.

---

## 1. Paper core findings (measured, their box: single A100, MS MARCO training)

| Finding | Number |
|---|---|
| Symmetric fixed-budget pooling preserves effectiveness vs doc-only pooling (ConstBERT) and beats single-vector | BEIR avg NDCG@10: FLINT(BERT) 0.452 ≈ ConstBERT 0.469, > single-vector 0.449; on ModernBERT FLINT 0.530 vs ColBERTv2 0.534 (near-tie) |
| Long/complex/OOD queries are where fixed-budget multi-vector wins | BERT best-in-class on CRUMB (0.145), ToT (0.065), BrowseComp+ (0.063); ModernBERT beats ConstBERT on EVERY suite, e.g. ToT 0.147 vs 0.070 |
| Index size decoupled from length | 5.3–9.3× smaller than ColBERTv2 (V=32, 128-d projected) |
| Search latency ~constant in query length | FLINT +0.004 ms/tok vs ConstBERT +0.31 ms/tok (TREC-ToT); 2.5–4.8× faster search |
| Budget V is the effectiveness–efficiency knob | V=16 still beats single-vector everywhere at ~half ConstBERT's index; V=64 helps complex queries, not BEIR |
| Query-vector SPREAD predicts search cost | participation ratio erank(Q) = (Σλ)²/Σλ² correlates with PLAID cells activated (ρ=0.87), candidates scored (r=0.99), search latency (ρ=0.84); FLINT's pooled vectors are MORE CONCENTRATED than token vectors (part of the speedup) |
| Learned pooling ≫ truncation | truncating ConstBERT queries to 32 tokens drops NDCG 30–60% on every complex suite — learned compression is not "just keep the prefix" |
| Failure mode they document | deep/iterative pooling variants (self-attention stacks) collapse document vectors to erank ≈ 1 and LOSE — shallow single cross-attention + FFN is the working point |

## 2. Distillation (fusion, not direct-map)

### 2.1 The fusion

**FLINT × Plan 080 (`maxsim_score`) × Research 213 pooling precedent × Bench 115
remedy axis =** length-independent multi-vector span matching with zero
training, on an oracle-labeled corpus that can adjudicate it the same day.

- `maxsim_score(queries, docs, lq, ld, dim)` SHIPS (katgpt-core SIMD, 7.38× vs
  naive, GOAT PASS; WGSL twin 41–74×; `RerankMethod::MaxSim` with NDCG eval).
  Plan 080 proved the SCORING half of late interaction. Nothing here proved the
  REPRESENTATION half — every embedding surface in the stack is single-vector
  per item (`riir-rag::Embedder::embed -> [f32; LATENT_DIM]`, 8-D modelless;
  `DenseEmbedIndex` 768-D single; `ShardIndex` KNN over `hla_moments`).
- Research 213/Plan 245 (Still) supplies the pooling CODE precedent (learned
  latent queries cross-attending over a token/state set) — but in the
  KV-compaction slot, where it was DEMOTED LOSER. Caution inherited: a trained
  pooler is not automatically a win on a new slot.
- Bench 115's recorded verdict names the open axis this paper addresses: the
  8-D modelless embedder has "no transferable formatting subspace" and the
  honest remedy is token-level representation. FLINT says the remedy does not
  need a new model — it needs V pooled views per span instead of 1 mean-pooled
  view.

### 2.2 Modelless extraction — Path 0 inventory (components, not vibes)

| Paper component | Coverage in stack | Modelless extraction | Row verdict |
|---|---|---|---|
| MaxSim scoring `Σ_i max_j dot(z_qi,z_dj)` | **SHIPS** — Plan 080, GOAT | n/a | covered |
| Cross-attention pooling w/ learned latents | PARTIAL — Research 213 `lora_still` (KV slot, demoted); `AttentionFnFamily::Perceiver` in katgpt-core canvas | Frozen latent set L (Hadamard rows / committed FPS anchors) + sigmoid soft-assignment `w = fast_sigmoid(dot(L_i, x_j)/τ)` row-normalized; or FPS; or fixed-seed k-means; or corpus-salience-weighted strided selection. No W_Q/W_K/W_V training, FFN dropped | open (modelless-validable) |
| Fixed-V multi-vector INDEX (length-independent) | NONE — all indexes single-vector per item | `embed_multi(text) -> [[f32; D]; V]` behind the existing `Embedder`/`DualEmbedder` trait family; fixed-size pooled records; PoC sweeps D ∈ {8,32,64} hashed op-token width (8-D negative is confounded by collisions — counts only at widest D) and V=4–16 | open |
| Symmetric budget + V knob | NONE | config const; V-sweep bench artifact | trivial |
| erank / participation-ratio spread metric | PARTIAL — jacobi eigen + spectral tools ship; no set-spread primitive; no spread→cost gate | closed-form `erank(Q)`; adaptive rerank-pool k for the dense two-stage channel (TWO_STAGE_LATENT_K is a fixed 32 today; the file itself says "raise to 48–64 if recall suffers" — an erank-of-query gate is the deterministic version of that hand-tuning) | open (katgpt-core primitive candidate) |
| Supervised pooling salience (the trained core) | dense channel exists (riir-rag-bonsai, frozen 27B mean-pool) | training-only | Path 0.5 → riir-train Issue 647 (secondary) |

Selection-beats-truncation is a coverage ARGUMENT, not a monotonicity claim
(the feasibility core, stated carefully): adding anchors to the query side adds
terms to the MaxSim SUM, and signed dots make neither the score nor the ranking
monotone in the anchor set — so nothing is claimed structurally. What holds is
coverage: FPS/k-means represent the token cloud where a prefix provably drops
every non-prefix match, and the paper's 30–60% truncation loss is the floor our
deterministic pooler must beat — arm (c) of Issue 169 carries that evidence
empirically. The supervised edge on top is all we decline to claim modellessly.

### 2.3 Signal-diffs (§3.6 — every "covered" claim adjudicated by formula read)

- **Plan 080 `maxsim_score`**: consumes pre-built vector sets; scores them. It
  is the scoring OPERATOR only — no pooling, no index representation, no
  per-span multi-vector emission anywhere upstream of it. Not coverage.
- **Research 213/Plan 245 `lora_still`**: trained perceiver compacting a
  GENERATIVE model's KV cache for decoding (un-rotate/compress/re-rotate;
  KL+CE training; demoted in its slot by AM/Plan 271). Consumes K/V attention
  state; produces a compact cache. FLINT pools ENCODER outputs into a
  retrieval representation scored cross-set by MaxSim. Different signal,
  different consumer, different slot. Not coverage — but its demotion is
  inherited evidence AGAINST assuming a trained pooler wins a new slot, which
  is why the modelless arm is primary.
- **`riir-rag::Embedder` / `DenseEmbedIndex` / `ShardIndex`**: one vector per
  item, by signature (`embed -> [f32; 8]`). Not coverage.
- **`CorpusDistanceGate`** (katgpt-core): `sigmoid(scale·(max_sim − mid))` over
  a registered exemplar set — a max-sim READ for abstention, not an index
  representation or a ranking score. Not coverage.
- **`set_rerank`** (Issue 121 / Bench 096, REFUTED for promotion): the pool
  MEMBERSHIP of a rerank stage, measured negative. Cautionary cousin — this
  repo has one measured-negative retrieval-layer precedent, so Issue 169's
  gate is a paired ABBA read with the incumbent, never an unpaired delta.
- **External wall (the §4 sweep — see §3)**: FLINT / ConstBERT / omni-col-press
  / Clavié token pooling / MUVERA / MetaEmbed own every architecture-novelty
  reading of this mechanism. What none of them carries: the training-free
  deterministic constraint at an exact fixed V, frozen-direction pooling
  keeping MaxSim scoring, and any code-rule / developer-tooling corpus at all.

### 2.4 Boundary check (latent-vs-raw, sync)

Pooled vectors are latent, local, and committed as BLAKE3-hashable fixed-size
records — the same posture as `EmbedProvenance`/`model_id` today. Wherever
pooled Pods cross a sync/quorum boundary (the episodic-ring arm below), the
pooling fold pins its reduction order (sequential ascending, `dot_f32_ordered`
precedent, Issue-940 determinism law) and stays zero-alloc. Raw-domain data
(positions, HP) is untouched — this is a retrieval representation, not sync
state.

## 3. Published prior art (§4 sweep — web agent, 10 searches, 2026-10-24)

| Work | What it owns | Delta vs our claim |
|---|---|---|
| **FLINT** (2610.27273) | fixed-V + symmetric pooling + MaxSim, trained | trains the pooler; IR domain |
| **ConstBERT** (MacAvaney et al., ECIR 2025) | constant-space doc-side pooling, trained | doc-only, queries token-level |
| **Clavié et al. token pooling** (2409.14683) | **training-free** hierarchical clustering pooling, drop-in at index time | RATE-based budget (grows with length), doc-side only — not fixed-V, not symmetric |
| **omni-col-press** (2602.21202, JHU) | constant-budget query-agnostic multi-vector compression incl. a **non-parametric pooling arm** (spatial/grid hierarchical; learned AGC is their winner) | not cluster-representatives/frozen-directions; no code domain; no reproducibility motivation |
| **MUVERA** (2406.19628, verify-flag: ID from memory + FLINT's references) | deterministic fixed-size encoding approximating MaxSim — but scoring becomes DOT PRODUCT over FDEs | we keep MaxSim scoring |
| **ConstBERT reproduction** (2604.09982, SIGIR 2026) | documents MaxSim's long-query collapse (86–97% drop on TREC ToT; uniform token weighting fails beyond ~20 words) | our MOTIVATION citation — pooled MaxSim is a candidate fix for exactly this |
| **MetaEmbed** (ICLR 2026) | fixed-V learned Meta Tokens + Matryoshka test-time scaling | trained; multimodal |
| **ColGREP** (2610.05099) / **LateOn** (2607.27178) / **jina-embeddings-v4** (2506.18902) | late interaction over CODE exists — token-level, trained | no pooled/budget variant in the code domain anywhere |
| **ArtSeek** (2507.21917, WACV 2027) | fixed-budget token pooling DEPLOYED in a production RAG (multimodal art) | evidence the technique class is production-viable; not code, not deterministic-fixed-V |
| SCV (Park et al., COLING 2025) | doc-side span pooling + single-vector prefilter | cited VIA FLINT's related work only — standalone ID unverified this session, do not cite independently |

**Kill/narrow verdict:** the claim "fixed-budget multi-vector pooling is novel"
is DEAD — do not write it. What survives, and only as an application + ablation
claim: (a) deterministic/training-free pooling at an EXACT fixed V with
reproducibility (BLAKE3-committable) as the motivation — between Clavié's
rate-based pooling and omni-col-press's learned winner, both of which are
mandatory baselines in any write-up; (b) frozen-direction pooling that KEEPS
MaxSim; (c) the domain: no paper applies any pooling-budget variant to
code-rule or developer-tooling retrieval; (d) if pooled variants measurably
improve exactly where token-level MaxSim collapses (the 2604.09982 axis), that
is a defensible evidence-backed sliver. FLINT is 5 days old — its citation
graph is invisible; re-check follow-ups before any external write-up.

## 4. Verdict

**Tier: GAIN** (modelless track, primary) — with the training track filed
separately as secondary. Reasoning per gate:

- **§1.5 novelty gate:** Q1 prior art — the architecture class is owned
  externally (§3); the surviving sliver is the deterministic constraint + the
  code-rule domain, which is an application claim, not a mechanism claim. Q2
  new behavior class for THIS stack — YES (no multi-vector retrieval surface
  exists anywhere in the workspace; single-vector is load-bearing in every
  index). Q3 selling point — internal-tooling quality (healer retrieval
  floors/OOD), not a product moat sentence. Q4 force multiplier — YES
  (riir-rag traits + refine eval + shipped MaxSim + katgpt-core spectral).
  All-4-YES is NOT claimable while the quality axis is unmeasured — and §3.6
  forbids asserting it from architecture alone. No "candidate" wording: the
  two issues below ARE the commitment.
- **§1.55 PASS vs Gain:** actionable — fills the Bench-115 named remedy axis,
  ships against a documented OOD gap (Issue 62's withheld-pair axis measures
  exactly the regime the paper wins), and the harness exists. Gain.
- **Per-track (no cascade):** modelless arm = Issue 169 (primary). Training
  arm = riir-train Issue 647 (secondary; its premise DEPENDS on 169's read —
  if pooled multi-vector does not beat single-vector deterministically on our
  fixtures, training a pooler is pointless, because the deterministic pooler
  is 647's mandatory control arm anyway).
- **MOAT gate (§1.6):** riir-refine consumer-first moat — "corpus/retrieval
  structure, healer-consumed primitives; notes live here" — IN SCOPE, this is
  the note's home; the filing priority follows the ladder (#2 surface;
  #1 game-runtime surface has NO documented retrieval pain — see §5, recorded,
  unmeasured, not filed). Landing spots: `embed_multi` behind riir-rag's
  existing trait family (riir-neuron-db, the index home) if 169 passes;
  pooling/erank primitives in katgpt-core (default-off feature) only at
  promotion, to avoid a substrate stall. Game-side arm only after a consumer
  exists.

**Filed:**

| File | What |
|---|---|
| `riir-refine/.issues/169_flint_fixed_budget_multivector_poc.md` | the modelless PoC: T0 kill-test (full-token MaxSim, no pooling, vs single-vector — poolers are built ONLY if it wins at the widest D), then V-pooled deterministic span sets + `maxsim_score` vs single-vector vs prefix-truncation on the oracle fixtures + withheld-pair OOD axis, paired; D ∈ {8,32,64} width sweep |
| `riir-train/.issues/647_flint_trained_pooler_poc.md` | the trained-pooler control arm on cached frozen-backbone states (4090, 3–5 GPU-h), contingent on 169's read |

## 5. Reframes (mandatory before verdict)

- **Consumer (healer — primary):** the retrieval query IS the long side. The
  paper's winning regime (long, multi-constraint, low-lexical-overlap queries)
  maps to: long code spans whose rule-relevant shape is one locus among
  boilerplate; the OOD withheld-(role, filler) axis (Issue 62) is the exact
  "unseen combination" instrument; Bench 115 named the token-level axis
  unclaimed. Three independent documented anchors — this is not a solution
  looking for a problem.
- **Game (priority-#1 surface, honestly weaker):** pooled fixed-V Pods per
  episodic capture cycle → needle retrieval by MaxSim at fixed per-cycle
  footprint (ndb RawWakeCycle ring; `capture_into_cycle`); AnyRAG/dense
  consumers for content retrieval. No documented limitation found on these
  surfaces (needle-retrieval smokes pass at full fidelity; game queries are
  short). Recorded as a fusion idea only; nothing filed until a consumer or a
  measured pain exists.
- **Inference-perf league (priority #3):** the erank→candidate-pool finding
  maps to the dense channel's rerank-pool sizing (TWO_STAGE_LATENT_K), not to
  the serving league. One optional deferred task in Issue 169; not a league row.

## 6. Honest ledger — what this distillation does NOT claim

- FLINT's absolute numbers (NDCG, ρ=0.87, 5.3–9.3×, 1.5–2.8×) are
  instrument-and-corpus-specific. Only mechanisms transfer; Issue 169 measures
  OUR numbers or none.
- Supervised salience in pooling (learned which positions matter) is NOT
  claimed modellessly. Fallback under test in 169: corpus-stat salience
  (op-rank/IDF/norm/entropy signals the ModellessEmbedder already computes)
  folded into pooling weights; residual gap disclosed, not hidden.
- The learned FFN post-pooling, end-to-end trained rerank, and learned
  adaptive-V are out of the modelless track entirely (adaptive-V's stand-in is
  the erank heuristic — arguably the more interesting primitive, per the
  panel).
- The retrieval-eval harness's standing non-goal ("no embedder re-training")
  is respected by 169 (deterministic pooling, no training) and does NOT cover
  647 — the trained arm belongs to the dense-embed lane, owner-gated, and
  must not silently join `retrieval_eval`'s committed gates.
- One unverified implementation premise for 647: whether riir-rag-bonsai's
  reader exposes per-token hidden states (mean-pooling implies they are
  computed internally; exposure is T1 of that issue).
- FLINT's negative result is inherited as a design constraint: deep/iterative
  pooling variants COLLAPSE (erank→1, quality loss). The PoC pools with ONE
  shallow deterministic assignment — no self-attention stacks, per their
  Appendix D.

---

## 7. T0 outcome (2026-10-24) — KILL

Issue 169's pre-registered kill-test ran FIRST and the answer is NO:
full-token MaxSim over hashed `op_tokenizer` op-token embeddings (D ∈ {8, 32,
64}, BLAKE3-XOF scheme, document-dedup exact) loses to the shipped single-
vector matcher at EVERY width — best (e) arm 85/141 (60.3%) blended top-1 at
D=64 vs arm (a) 124/141 (87.9%); paired +3 wins / −44 losses / 94 ties; with
the shipped instantiation rerank already riding the (e) arm. Width sweep:
44.7 → 56.0 → 60.3 — monotone but mechanism-bound: hashed random directions
carry no token similarity and no IDF, so MaxSim degenerates to unweighted
token-overlap counting (the tiny-token-crowding defect re-created inside the
scorer). The paper's per-token vectors are trained-encoder outputs — the
representation law does not transfer to untrained hashed tokens on this task.
Mechanism diagnosis, full tables, cost disclosure, and the honest residuals
(MaxSim-as-ensemble-channel untested; trained token embedder is a different
lane): **`riir-refine/.benchmarks/120_flint_t0_fulltoken_maxsim_killtest.md`**.
Per the issue's own STOP rule: T1 is never built, Issue 169 closed (file
removed, `HISTORY.md` carries the row), and riir-train Issue 647 stays closed
— its gate needed BOTH (b)>(a) AND (e)>(a). The §2.2 Path-0 component
inventory remains accurate; its quality premise is measured false HERE.
