# Research 123 — Batch 102: FlashQuant sparse-dense fusion distill (arXiv 2608.15531)

**Status:** RECORD — batch landed (6 rules, corpus 453 → 459); Plan 071 T1–T5 complete.

## Provenance

- **Source:** "FlashQuant: Sparse-Dense Fusion for Memory-Efficient Outlier-Aware
  LLM Inference" — Junqing Lin, Jingwei Sun, Zhengding Hu, Guangzhong Sun (USTC).
  arXiv 2608.15531, HTML v1 rendering (https://arxiv.org/html/2608.15531).
- **Armed by arxiv walk #6** (`.research/121`, commits `7a99912` + `d5e8461`):
  headline REFUTED for direct adoption (INT4+BF16 weight outliers unreachable
  from ternary/Q4_K; batch ≥ 8 regime pads to 8 — "its advantage diminishes for
  very small batches, such as 1 and 2" — not our batch-1 decode). The FUSION
  MECHANISM was kept queued as the distillable core: heterogeneous-execution-unit
  warp specialization = the additive-heterogeneous-paths member of the B58/B98
  same-input family.
- **Plan:** `../.plans/071_flashquant_sparse_dense_fusion_distill.md` (renumbered
  070 → 071 at takeover; see below). **Session constraint:** CPU-only unit on the
  4090 box while the 504 T3 NoRA confirm matrix holds the GPU (launched 01:10
  09-04, decisive ~09:00–10:30); targeted GOAT validation (the B95 honest-skip
  shape — affected feature states individually, full CI guard not run this cycle).
- **Takeover note:** the batch was filed 09-03 by a sibling session that left the
  plan file all-unchecked and untracked; no cargo processes were live and no
  corpus edits existed at takeover (~02:00 09-04). Renumbered 070 → 071 per the
  reserved-071 collision flag in the queue snapshot (kat062 keeps 070, committed);
  `.plans/.highwater` stays 72 (write-back = max(72, 71) — no regression).
- **Paper-fetch lesson:** the jina PDF route (`r.jina.ai`) 500'd twice at mine
  time; the arXiv HTML rendering (webReader) was the fallback and worked. The
  lanes' quotes are byte-accurate to the HTML v1 prose (the PDF's math markup
  differs cosmetically). ONE quote correction at coordinator verification: the
  Stream-K reduction quote is "When required by the Stream-K strategy, a
  subsequent global reduction is performed to produce the final outputs."
- **Discipline:** two parallel distillation lanes with mandatory dedup verdicts
  (Lane A: fusion + ownership; Lane B: format + bank scheduling); zero prior
  `FlashQuant`/`Tile-COO`/`Stream-K` corpus hits grepped across all 15 corpus
  files before proposing.

## The 6 rules (all LAND; corpus 453 → 459)

| rule_id | cat | one-line content |
|---|---|---|
| `heterogeneous-unit-paths-shared-tile-fusion` | C | Content-sharing fusion: Tensor-Core dense + CUDA-core sparse paths co-scheduled over ONE tiled execution space; activation staged once, partials accumulated on chip; offline tile correspondence + EU warp specialization preconditions; EW rider folded (low-intensity path gets more warps, +2–4%) |
| `lpt-min-heap-disjoint-column-ownership` | H | Assignment-time LPT over a descending-nnz sort; one owner per column forever → conflict-free accumulation, zero atomics; balance purchased OFFLINE vs B97's arrival-order atomics and B92's runtime conflict ordering |
| `tile-coo-sparse-tile-aligned-format` | N | Tile-local sparse format designed backward from the consumer's four constraints; u8 indices derived from the staged-tile extent (≤256, assert at encode); padding slots repurposed as free high-precision storage |
| `bucketed-sparse-reader-column-partition` | A | Access-granularity mismatch: sparse per-element readers conflict when > bank_size/4 threads hit a column of the swizzled staged tile; fix = partition columns into N_bucket = warp_size/N_thread buckets — the consumer-side arm of "layout is contested" |
| `bank-group-circular-element-reorder` | A | Offline DATA-side bank staggering: reorder elements per bucket by gid = floor(k/2) % 4 in the bucket-dependent circular order ([0,1,2,3]+bucket_id) % 4; keyed to the consumer's walk schedule; stable reorder |
| `stream-k-persistent-cta-mac-segments` | C | Work-DECOMPOSITION rule: contiguous MAC-space segments across persistent CTAs, spanning tiles, with a follow-up global reduction; mandatory control arm = B58 `occupancy-grid-cap-refuted-uncapped-default` (uniform work measured 1–2% LOSS capped) |

Category deltas: A 103→105, C 39→41, H 16→17, N 27→28.

## Coordinator adjudications

1. **Stream-K: LAND, not a B92 rider.** Lane A deferred to the plan's conditional
   disposition rule ("if Stream-K ≡ B92 stripe → rider"); Lane B ran the full
   kill-check and found the two DISTINCT: B92's decision content is the REDUCTION
   mechanism (in-output-buffer counting semaphore over whole-tile striping — no
   tile is ever split); Stream-K's is the DECOMPOSITION unit (contiguous
   MAC-space segments that deliberately split tiles, with a separate global
   reduction). Complementary, composable. Adjudicated LAND with B58 as the
   mandatory control arm — the regime split (irregular per-tile cost adopts;
   uniform work keeps the uncapped default) is the decision content.
2. **EW (extra warps to the low-intensity path): rider inside the fusion rule**
   (Lane A's disposition accepted) — same kernel, same trigger; the warp-budget
   split of the specialization decision.
3. **Warp-level Sliced-K: folded into the fusion rule** — intra-CTA K-split
   reduce, adjacent to B44/B92 sub-mechanisms, thin standalone.
4. **Categories:** Lane B's "B (layout)" guesses for tile-coo were remapped to N
   (DtypeAlignment — the u8-index/uint4-packing/bf16-value decisions are the
   N-bucket's content, beside trit-packing + the dequant recipes); B3 landed in A
   beside xor-swizzle (the bank-conflict family), not B.

## Measured evidence carried (paper's numbers, honestly scoped)

- Fused vs Sputnik+Marlin: 1.03–1.16× (3090), 1.12–1.37× (4090), 1.23–1.53× (5090).
- Ablation: sparse-dense tiling alone 6–16%; extra warps +2–4%; reordering +4–9%.
- Memory traffic −45.4% vs Sputnik. cuBLAS BF16 reference speedup 2.74–4.18×.
- End-to-end (vLLM, LLaMA3-8B): 114.4 vs 90.7 (cuSPARSE) vs 138.5 (Marlin) tok/s
  at batch 8 — slower than Marlin, higher accuracy; the accuracy–speed trade is
  the outlier-aware premise, not a fusion property.
- Preprocessing offline one-time < 100 s. Tile-COO storage (N_tile+1)·4 +
  N_B·4 + nnz·4 vs CSR (K+1)·4 + nnz·12.
- Regime caveats pinned in-rule: batch ≥ 8 (pads below; batch 1–2 diminished),
  INT4+BF16 instantiation evidence-only, TC+CUDA-core hardware mix required.

## Not landed here (queued elsewhere)

- NAX hd-256 dsplit (`attention_nax_dsplit`) + the #4380 GQA-12/16 table: stay
  QUEUED as B100 rider material per the 09-03 extraction unit (B100 touch).
- `backing-provenance-causal-shadowed-extent-widen` + `dispatcher-eligibility-
  predicate-lags-kernel-capability`: still queued for the post-B102 batch
  (Plan 072 MTPLX + NAX leads, default COMBINE) — now UNBLOCKED by this landing.

## GOAT (targeted, B95 honest-skip shape)

- `batch102_rules_present` + `batch102_concepts_covered` +
  `batch102_pseudocode_parses` (cfg(not(kernel_verify)) per B70): all 3 first-shot PASS.
- kernel_opt lib: **1282/0/1i**; default lib: **1029/0**; kernel_opt+latent:
  **1389/0/2i** with `retrieval_precision_regression_gate` PASS — kernel 30
  labeled, top-1/3/5 **100/100/100** (floors HELD, zero re-pins, 13th
  consecutive batch).
- clippy 0 both states (`--lib --features kernel_opt` + default) — one
  doc_markdown round on the new //! header (`ternary/Q4_K` needed backticks;
  the caret's column pointed at the slashed token, not the kebab-case chain
  suspected first — the B37 lesson's continuing strikes).
- Full CI guard NOT run this cycle (the B95 honest-skip shape; the 504 matrix
  holds the box). Each affected state validated individually.
- Count pins walked: tests.rs corpus total 453→459 + the new Batch 102 comment
  line + expected-array A/C/H/N + sums ×2 + grouped + the cover-all comment
  398→459 + router.rs expert total + mod.rs 459-rule doc + 4 category list
  appends + AGENTS.md domain line + Batch 102 ledger entry + queue snapshot.

## Landing files

- `src/domains/kernel_opt/corpus/entries_gpu_15.rs` (new, 6 entries)
- `src/domains/kernel_opt/corpus/mod.rs` (mod + extend + KERNEL_CATEGORIES ×4 + doc)
- `src/domains/kernel_opt/corpus/tests.rs` (count pins + 3 gates)
- `src/domains/kernel_opt/router.rs` (expert-total pin)
- `AGENTS.md` (domain count + Batch 102 ledger entry)
- `.plans/071_flashquant_sparse_dense_fusion_distill.md` (T1–T5 complete)
- `.distill/001_mining_queue_snapshot.md` (B102 landed; Plan 072 unblocked)
