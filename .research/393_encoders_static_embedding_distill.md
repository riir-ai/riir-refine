# Research 393 — flowercomputers/encoders: the static-embedding toolkit as a rust_perf distill source

**Status:** DISTILLED — pending owner decision (the mining-candidate filing; the batch is
plan-gated as [Plan 360](../.plans/360_encoders_rust_perf_batch.md), STAGED behind the live
queue claims). Claude verdict ping-pong: 2 rounds, AGREE round 2 (record: §7).

> **Source:** `flowercomputers/encoders` @ `d46c90e45f89a52feb8e41720f63c3fda2b976e1`
> (v3.0.0, tip 2026-11-08, Apache-2.0; model weights MIT/Apache-2.0 per the README table —
> re-verified at build time via `encoders::model::LICENSE` if ever adopted). Blog:
> "If you can believe it, even faster static embedding" (flowercomputer.com, 2026-11-04).
> Inspected 2026-11-10, `.raw/encoders` shallow clone, REMOVED at the landing commit.
> crates.io `encoders` v3, active (last commit days before inspection).
>
> **Related:** [Plan 360](../.plans/360_encoders_rust_perf_batch.md) (the batch, staged) ·
> riir-rethink [Research 023](../../riir-rethink/.research/023_encoders_l2_surface_rung_candidate.md)
> + [Issue 061](../../riir-rethink/.issues/061_static_l2_p0_admission.md) (the serving-lane
> half — this note is the corpus half only) · seal-game-editor Bench 010 (the honest negative
> prior for static-embed QUALITY on one workspace surface) · reflex `--bekko` lane (a DIFFERENT
> bekko — the bekko-system-one transformer teacher; lane-name collision only).

## 1. What the source is

A Rust static-embedding toolkit: compile-time perfect-hash weight tables (models compiled
into the binary via `ese-build`), a zero-alloc single-pass BERT tokenizer reimplementation
(`ese` + `ese-unicode`), per-row power-of-two exponent quantization with deferred
integer-lane pooling, MRL truncation, rayon batch parallelism. Claimed ~15M sentences/s
on an M4 Pro; the bekko-slim family is distilled from hotchpotch/bekko-static-embedding-v1
(the current SOTA static embedder, NanoBEIR-en 49.3–53.7 across the vocab ladder).
Workspace layout: `encoders` (the facade) + `ese` (the runtime core: pretokenizer 875 L,
wordpiece 307 L, accum 389 L, phf 215 L, elem, similarity) + `ese-build` (fetch/quantize/
PHF-emit) + `ese-unicode` (precomputed tables). Release profile carries `lto = "fat"` +
`codegen-units = 1` — they benchmark their code (criterion bench crate + a Python chart
companion). The perf claims are THEIR hardware; magnitude stories below are mechanism-level,
never our numbers.

## 2. Verdict

**YES B+ corpus — route `rust_perf`, NOT `kernel_opt`** (CPU SIMD/allocation/dispatch; the
tempting wrong domain is kernel_opt because the outputs are embeddings — there are zero
GPU kernels here). Lane-intel axis: **HIGH** (§5). Product-coverage axis: **seeds the
static-embedding/model2vec family, currently ZERO members** (§6).

## 3. Ranked candidates

All three rows are the source's OWN landed decisions (decision-content mining posture —
no fix-shaped leads, so no upstream-state tokens are owed; the P113 check is satisfied by
that classification itself, recorded per row).

| # | candidate | source | grade | notes |
|---|---|---|---|---|
| 1 | `deferred-integer-lane-pooling-per-row-exponent` | `ese/src/accum.rs` | **A−** | Per-row power-of-two exponent quantization (`2^-exp · (min + q·scale)`, `MAX_ROW_EXP = 8`) so small-norm rows (untrained `[UNK]`, SIF-down-weighted frequent tokens) keep most of their bits instead of collapsing; pooling collects row ADDRESSES in a bounded stack batch (`BATCH = 256` — also the integer-lane overflow bound), sums one 32-column block at a time (`BLOCK = 32`) with integer lanes held in registers, boosted rows (exp > 0) drain to an f32 side array scaled by `2^-exp`, single dequant at the end directly into the caller's output. The in-source negative: explicit cache-line touching at lookup time "made no difference; the out-of-order core already overlaps those misses" — carried as the anti-regime note. Two-regime (exponent-0 integer path vs boosted f32 path), bounded overflow, deterministic, benchable (pool throughput; the accumulation ORDER is part of the contract — compare like order, never vs a different fold). |
| 2 | `incremental-prefix-hash-wordpiece-scan` | `ese/src/wordpiece.rs` + `ese/src/phf.rs` (`Hasher`) | **B+** | Greedy longest-match-first where the hash of EVERY candidate piece starting at one position comes from ONE pass over the bytes (`Hasher` yields each prefix's hash incrementally), clamped by the longest vocabulary entry. Magnitude restated honestly (verdict round 2): naive greedy re-hashes each candidate from scratch — O(len·max_token²) per word; the one-pass prefix fold makes it O(len·max_token). The `max_word_len` cap (overlong → single UNK, `pretokenizer.rs`) bounds the worst case, so with the cap applied the win is a CONSTANT FACTOR per word — largest for long in-vocabulary words, smallest near the cap's short end. The fixture states both regimes; the O(len³) framing is dropped. Exact integer semantics (u64 fold, bijective per-chunk steps — "two different keys of the same length never fold to the same value"). |
| 3 | `normalizer-pretokenizer-fused-streaming-pass` | `ese/src/pretokenizer.rs` | **B** | `BertNormalizer` + `BertPreTokenizer` (the `tokenizers` crate's two stages) fused into ONE allocation-free streaming pass: ASCII classified + lowercased 64 bytes at a time with LLVM-vectorizable compares, words found from bitmasks and handed to the callback straight from the block, non-ASCII per-char via `ese-unicode`'s precomputed table, fixed stack word buffer, special tokens matched literally on RAW input before normalization. KIN RISK, adjudicate at authoring: `ascii-run-swar-block-skip` (Batch 243) owns the ASCII-run-skip law — the DELTA here is the pipeline FUSION + callback-from-block shape (the corpus has no pass-fusion-over-tokenizer-stages rule); if the fusion delta doesn't hold as its own rule, it folds as a rider face on the Batch-243 host. Expected to survive as distinct or as a rider — pre-named in Plan 360. |

**Drops (recorded with reasons):**
- **Compile-time perfect-hash weight table** — positive instance of
  `minimal-perfect-hash-static-lookup` (Batch 248, unicode-normalization's two-level salted
  MPH). The LAW is owned; encoders' build-time emit is the same law at a new scale. Dedup kill.
- **MRL / Matryoshka truncation** — a concept, not a bounded fix-space; the Matryoshka
  attribution primitive family already shipped katgpt-rs-side (Research 584 /
  `exact_mass_admit`, Bench 884).
- **rayon "couple-hundred-inputs" batch threshold** — the rayon family is owned (the
  Lattimore rules); the threshold is a tuning number for a specific workload shape, not a
  two-regime law. Expected drop at authoring.

## 4. Dedup gate evidence

Greps over `riir-refine/src/domains/**` under BOTH vocabularies (2026-11-10, corpus at
rust_perf 307 / kernel_opt 1076 per the Batch-343/345 landing rows — re-derive at claim
time): `perfect_hash|phf` → OWNED (Batch 248 — kill recorded above);
`tokenizer|wordpiece` → the Batch-243 Tokenmaxxing family (ASCII run skip + page
memoization) — candidate 3's fusion delta and candidate 2's prefix-hash scan have ZERO
owners; `integer lane|deferred sum|per-row exponent|drain` → the only drain owner is
kernel_opt GPU-side (`contraction-partials-sum-once-at-drain`, gpu_299 — different
surface); `pooling|mean pool` → zero rust_perf owners. Candidate rows carry one line of
evidence each in the table.

## 5. Lane-intel axis — HIGH

This is the axis that fired (the dial9/RIPL rule does NOT apply — this verdict is YES, so
it carries the full artifact trio, and the lane intel rides along):

- **The decision-serving lanes (the user's target):** riir-ai Proposal 051's tier-rung map
  pre-registered an L2 "Surface" rung for the rethink family — static-embed KNN + frozen OOF
  soft labels, µs-class, **unmeasured since 2026-10-03**. `encoders`/`ese` is the fastest
  substrate that rung could seat on (ns-class single-sentence, zero-init, wasm-able). The
  serving-lane filing lives in riir-rethink (Research 023 + Issue 061, the P0 admission
  protocol, owner-gated) — recorded here as the lane mapping, never a batch item.
- **riir-rag cheap-embedder tier (secondary):** neuron-db's retrieval facade
  (`Chunker`/`Embedder`/`RetrievalProvider`) and the AnyRAG escalation gateway name a
  dense-embed escalation lane; a static-embed tier is a candidate cheap first rung there.
  No neuron-db issue filed from this session — named for the owning lane.
- **NOT transferable / no reopen:** seal-game-editor Bench 010 ruled the static embedder
  NO-GO for quest semantic search on QUALITY (hit@5 17/20 vs FtsTrigram 20/20 on a
  lexical-friendly probe; misses were Thai paraphrases). `encoders` moves the COST axis
  ~100× but the Bench-010 kill was quality-vs-FTS on a probe where FTS has no headroom —
  the verdict stands; a reopen would need the paraphrase-heavy probe their plan already
  names, and is a seal-lane call, not this verdict's.

## 6. Product-coverage axis — seeds the static-embedding family

The corpus's text/unicode processing family begins at Batch 243 (the Tokenmaxxing rules);
the static-embedding/model2vec SERVING class has zero members. encoders is the founding
source. **Follow-up pool** (the accumulation clause applies — pool sources read before the
batch window fold into the same batch): `minishlab/model2vec-rs` (the Rust model2vec
runtime — the corpus's Potion-family counterpart), the hotchpotch `bekko-static-embedding-v1`
HF card (the SOTA static model's training/quant notes), the sentence-transformers
static-embed Matryoshka write-ups.

## 7. Honest caveats + verdict record

- Perf numbers are THEIR hardware (M4 Pro / M3 Pro); the crate is LLVM-autovectorization
  dependent ("you really, really need to compile this with optimizations on").
- Models compile INTO binaries (the 100k-vocab default is ~100 MiB at int8/mrl-512) and
  the first build downloads from HF into `target/ese-cache`. For OUR fixtures this is
  irrelevant (original Rust reproductions, the transcription law), but any ADOPTION would
  need the fetch_artifacts BLAKE3-pinned provisioning pattern, not a build-time download.
- `api-py` + `bench/` members unread (out of scope for the corpus verdict — the runtime
  core is fully read). Unverified-none.
- **Verdict record (the research-skill §5 gate):** `request_verdict` (claude backend),
  round 1 REVISE — four P0-protocol fixes on the rethink half (one-frozen-read law: k on
  train-CV only, never best-of-N over test; the 051 OOF-soft-label design stated as a
  deliberate stage-2 deviation; per-suite priors re-aligned to the cited evidence — sst5
  DEAD-expected per 051's own 0.3917 prior, the earlier ALIVE call withdrawn as unevidenced;
  the "016 T8" citation hazard softened to loose-not-baseless) + two minors adopted
  (candidate 2's magnitude restated per the word-length cap; the sst5 dataset licence made
  a checkbox). Round 2 AGREE with one wording change folded into Issue 061 (a stage-1
  BOARD-DEAD records "the 051 soft-label rung was NOT TRIGGERED", never "measured dead",
  plus a pre-declared 10.0-pt near-miss trigger). Track 1 (this note) landed as written
  in round 1. `dual_allocation_gate`: 0/0/0 both repos at landing. Counters re-read at
  write: refine .research 392→393, .plans 359→360; rethink .research 022→023, .issues 060→061.

**Bottom line:** mine as one batch — **Plan 360, nominal Batch 349, STAGED behind the live
claims (P113 exp 2026-11-12; b347/P120 exp 2026-11-12); lead with
`deferred-integer-lane-pooling-per-row-exponent`**. The serving half (the user's actual
question) is filed rethink-side: their bet is confirmed as substrate-for-a-pre-registered
rung, with BOARD-DEAD-on-all-8 stated as the most-likely P0 outcome and the surviving value
lanes named (ESC cost tier, riir-rag embedder tier, wasm semantic demo).
