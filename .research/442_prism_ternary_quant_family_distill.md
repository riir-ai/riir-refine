# 442 — Prism fork ternary-quant family residue distill (Batch 409, Plan 408)

**Status:** LANDED 2026-11-24 (Batch 409) — the batch record lives in the snapshot landing row + the tests.rs re-pin trail; this note carries the verdict-level provenance.

## Source

PrismML-Eng/llama.cpp (the league fork, MIT) — read at the standing vendor clone
`/Users/katopz/git/vendor/prismml-llama.cpp` (NO fresh `.raw` cloned; the clone is
a standing league asset, left untouched). Four pins:

- `origin/ptq1_0-cuda` @ `8f0f0ea8a` — a41dcc7ac "cuda: add PTQ1_0 support"
  (vecdotq.cuh widen/×3/byte_perm/vsub4/dp4a decode + mmq-load-tiles lane-alias
  loader + mmq-config-ampere CASE rows + mmvq multi-column + the ne11 ≤ 7
  crossover) → a417aeff4 "cuda : specify PTQ1_0 MMQ activation precision"
  (`GGML_PREC_Q8`) → 8f0f0ea8a "tests: compare PTQ mapping against production
  CPU codec" (the host-accessor replica check, "does not claim native CUDA
  execution").
- `origin/ptq1_0-base` @ `fd6d992c2` — a967c2a34 "ggml: add PTQ1_0, ternary at
  group 128" (the codec: TQ1_0 base-3 digit packing at group 128; 28 B/128 =
  1.75 bpw; the docs commit fd6d992c2 carries the CLI/GGUF compatibility table:
  TQ1_0 256/54 B/1.6875 bpw vs PTQ1_0 128/28 B/1.75 bpw; "Weights that are
  already exactly ternary with an fp16-representable scale per group of 128
  round-trip losslessly"; the `--pure` quantizer fallback Q4_0/F16 rows).
- `origin/feat/pq1_0-g64` @ `7fc75d9a6` — 76b97aa60 "PQ1_0, the Q1_0 binary
  codec at group 64" (ggml type 144 on that branch!) + 29cddbe02 the
  NEON/AVX2/AVX/SSSE3 vec_dot (table_b2b_0 bit-spread + 2s−1 sign expansion +
  vdotq_s32).
- `origin/prism` @ `d12ca13bb` — #428 "pq2_0m as 2-bit min/max KV-cache type
  heading toward ternary" (the codec: d = (max−min)/3, m = min, raw 2-bit
  codes, dequant y = m + q·d, worst-case d/2 = (max−min)/6; 24 B/128; measured
  Vulkan/RX 9070: KLD 0.05 mixed K=q4_0+bias / V=pq2_0m, 0.06 both pq2_0m, vs
  the PQ2_0 no-bias codec floor 0.258 — ~4×) + `802b53737` (#449 "Arm NEON
  PQ2_0 repack with Q8_K GEMV and GEMM" — the 4-row interleave, dup_half
  activation duplication, unsigned codes + per-128 block-sum offset removal,
  vdotq + vpaddq pairwise).

## The prior-coverage finding (the batch's controlling result)

The staged lead (the 11-24 watch leads row (2): "distill value = their CUDA
PTQ1_0 mat-vec kernel shapes vs our ternary_gemv family") is **ALREADY OWNED**:

- `swar-lane-widened-base3-dp4a-dot` (Batch 116 addendum, Research 140
  addendum 2) owns the byte_perm 0x4140/0x4342 widen + ×3 + 0x00FF00FF +
  0x7531 gather + vsub4 + dp4a decode — from the SAME fork lineage @
  7113f6462 — and its riders through B251 cover: the isum deferred-bias
  sibling (B180), the NEON measured-negative + the three-arm ISA decision
  (B204), the i8mm halving-add validation + SMMLA 2×2 tiling WITH a Ternary
  Bonsai 2 27B device A/B (B211), the yukon direct-difference ladder decode
  face (B251). a41dcc7ac (Sept 29) predates the riders and lands INSIDE
  owned territory.
- The verdict lesson (recorded for the watch lane): a staged-lead row's
  "distill value" claim expires against the corpus AS THE RIDERS LAND —
  re-grep the corpus at claim time, never mine from the staging text.

What survived (zero-owner at claim-time greps over both corpora + the staged
rows): PQ2_0M #428 (Nov 21, AFTER every rider) and the NEON PQ2_0 repack
#449 (Nov 20).

## Landed (Batch 409)

- R1 `min-max-range-scale-asymmetric-block-codec` (DtypeAlignment) — the
  codec-choice law + the KV-only split-surface clause + the lossy-surface
  adoption gate + the three-defect adoption tail folded.
- R2 `offset-block-dot-block-sum-dp4a-correction` (Compute) — the two-dp4a
  decomposition, transport-free by the codec's own m-word; B180 compose.
- 3 count-neutral faces: deferred-bias-ternary-dp4a-exact-isum (gpu_49) +=
  the NEON unsigned-codes block-sum + dup_half interleave face; ane
  silent-wrong += the reference-census-catches-dequant face; fewrow-mma
  (gpu_119) += the decode-once-multi-column mmvq face + the ne11 ≤ 7
  crossover.
- Fixtures b409_fixtures 8/8 standalone-compiled AND executed BEFORE entry
  (debug AND -O) — the gate caught THREE authoring defects (the as-i8
  overflow; the sym codec's shift/round order; the fold-order mismatch).
- Gates: default lib 3978/0; kernel_opt 581 passed; retrieval floors BOTH
  lanes 30/0 HELD no re-pin; clippy -D ×3 postures clean (14 mechanicals in
  the new file healed via clippy --fix — the maintained binary is STALE
  (build e4b2dd31 vs landing HEAD), the fix fell back to cargo clippy --fix
  per the stale-binary convention, disclosed); fmt bare rustfmt on the
  touched files.

## Drops / honest dispositions

- The planned R1 (byte-lane-widen digit extraction) DROPPED at T1 dedup —
  owned (above), never re-staged.
- The divergence-free lane-alias tile loader (mmq-load-tiles) — face-grade
  thin at authoring (the 8-lane-same-loop + lane-6-alias atoms sit beside
  the B251 decode family), dropped from the face list, recorded here.
- The mixed-KV fatnn instance table (15 template pairs) — infrastructure,
  never a rule.
- The type-ID conflict — lane intel (below), never a rule (upstream-project
  process, no Rust-shaped fix space).

## Lane intel (recorded, never landed)

1. **T-R3 adjacency**: PQ2_0M #428 sits on `origin/prism` tip — the next
   scored tag carries pq2_0m KV. The league bout lane (riir-ai perf-rematch)
   owns the scored-tag response; the KV type changes the opponent's cache
   footprint (22→24 B/128, +9%) AND its KLD posture — a scored-tag cut with
   `-ctk/-ctv pq2_0m` would be a DIFFERENT opponent posture, the fairness
   manifest should pin the cache types explicitly.
2. **Type-ID conflict**: `feat/pq1_0-g64` took GGML type 144 for PQ1_0;
   #428 takes 144 for GGML_TYPE_PQ2_0M on prism (and its own glsl says
   145u — an intra-commit enum/glsl inconsistency). A merge of both
   branches must re-allocate one; until then the branches are
   mutually-incompatible GGUF producers.
3. **The "heading toward ternary" signal**: the fork is prepping 1-bit
   ternary class support (pq1_0-g64 + ptq1_0 branches) — the league model's
   own weight class becoming a first-class opponent serving path. Watch for
   a scored tag carrying PQ1_0/PTQ1_0 weight loading.
4. The MMQ activation precision knob (`GGML_PREC_Q8`) and the
   "ne11 <= 7 → mmvq" crossover are the fork's own dispatch tuning —
   consumed as the fewrow face, nothing further owed.

## Provenance

Every quote byte-verified at the vendor clone this session via `git show` at
the four pins. NOTHING INVENTED. Session: zed-amo-i408@amo, epoch 1795484168.
