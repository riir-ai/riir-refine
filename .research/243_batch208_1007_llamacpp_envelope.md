# Research 243 — Batch 208: the 10-07 llama.cpp envelope (fusion residual identity, MLA FA exact-shape admission, headless draft crop)

**Status:** RECORD — the batch landing note (Plan 208; commit at landing).

## The pack

Three kernel_opt rules, all category D (dispatch/admission/graph-construction), all
from `ggml-org/llama.cpp` merged 10-07 (MIT), corpus 773 → 776:

1. **`fusion-residual-selected-by-identity-not-negation`** — @ `7e8324f5fe` (#30100).
   The fused MUL_MAT+ADD residual picker used a negated op-class predicate ("the ADD
   operand whose op is not MUL_MAT"); when both operands are matmul outputs
   (x = W1@u + W2@v) the negation matches both and the pick resolves to the fused
   matmul's own never-written output — the kernel adds uninitialized memory. Failure
   shape: silent, backend-local (Metal), deterministic per memory layout,
   kill-switch-flipping (`GGML_METAL_FUSION_DISABLE=1`), discovered in production
   (Clef head: probabilities 0.28 vs 0.977). Repair is ONE line: a pointer compare
   against the fused op (`dst->src[0] == op ? dst->src[1] : dst->src[0]`). The
   regression test CONSTRUCTS the violating shape (`MUL_MAT_ADD_MM_MM`, fails 27/28
   pre-fix). Law: a fusion admission that must pick one operand of a binary op
   selects by IDENTITY against the op just fused; a negated predicate is sound only
   under an at-most-one-match premise, and chains are ordinary graphs.

2. **`fa-fast-path-validated-exact-shape-admission-v-as-strided-view`** — @
   `d2a79e6046` (#29171). The MKL flash-attention gate required homogeneous K/V
   widths + head-dim ≤ 512, silently excluding MLA (576/576/512, GQA-20) to the slow
   TILE path; pp8192 432.80 → 1292.29 tok/s (2.99×, source-reported Arc Pro B70).
   The widening is an EXACT-SHAPE ALLOWLIST naming every dim + GQA + both dtypes,
   with the narrowness stated in-source ("Keep this narrow until other mismatched
   K/V shapes have independent correctness data"). Three view conjuncts: descriptor
   mode from the ACTUAL row stride (padded ⇒ strided, even at one KV head); the
   stride exception as a full triple (admitted shape AND identity-to-V AND real view
   AND K's row stride); dequant aliasing gated on LOGICAL width equality, never view
   identity alone. **In-commit negative recorded as watch**: the F16-scores sub-commit
   (pp +4.6/+12.2/+13.5%) was REVERTED in the same merged PR, reason unstated —
   measured-positive on record, never adopt without the revert reason.

3. **`headless-draft-forward-crops-to-output-rows-cache-writes-persist`** — @
   `b9acf138a1` (#29928). A zero-output-row NextN/MTP forward (the catch-up, the
   draft-context prefill) persists ONLY through its cache writes; everything
   consumer-side is dead compute. Both elimination depths measured in-commit: deep
   PRUNE 6.9 → 0.33 ms, CROP-to-output-rows 6.9 → 2.9 ms — the CROP landed for
   pattern consistency with the other MTP graphs despite being ~3.3× shallower (the
   prune also needed an extra no-unmasked-extraction-live guard conjunct). Correctness
   pins both arms: greedy output hashes + draft acceptance unchanged. The
   consistency-over-depth tradeoff is recorded honestly in-rule.

## Riders (2)

- **fewrow family** (`fewrow-tensor-tile-decode-into-rhs-operand`, B198) += @
  `988190680d` (#30065): the UPSTREAM generic few-row MMA generalizes to any type
  with a 16-weight dequantizer (BF16, Q1_0, Q2_0, MXFP4, Q2_K, Q3_K, TQ2_0, IQ); the
  new decision content is the per-type row-threshold table (5 TQ2_0 / 4 BF16 /
  3 MXFP4+Q2_0+Q2_K+IQ4_NL / 2 others) + the measured table (0.23–0.98×
  threshold-to-8-rows, 0.24–0.33× at 9–16, parity at 1 and 512; M3 Ultra,
  mean of two interleaved). The B205 `a3a1c47` precedent (upstream few-row instance
  = dedup-note only) holds; this commit is the same family EXTENDED → rider, not rule.
- **drain family** (`profile-window-drains-queued-work-before-open`, B207) += cubecl
  @ `9865237018` (#1770): the COMPILE-QUEUE member — queued compilation work drains
  before the first timed sample (`compile_queued()` at every measurement start, tune
  and throughput probe both); secondary clauses: the batch-compile work-stealing law
  ("kernels differ by orders of magnitude in how long they take to compile, so a
  fixed split would leave threads idle behind the slowest share") and
  one-source-finalizes-once (Waiting step + by-source store).

## Envelope notes

- bevy: ZERO new commits since the B207 pin `5cd69c1357`. wgpu: one dependabot bump.
- Watch (no leads): llama.cpp `448147d4` (nextn flags share — #29928 follow-up),
  `d0b490f2` (sampling greedy), `ad215653` (musa lightning-indexer port — B205-owned
  family), `78651c41` (sycl IQ3_S MMVQ), `c479922a` (hexagon DMA/HVX copy overhaul),
  `4d756bc7` (vulkan amd iGPU checkpoint read), `005a1e12` (sycl mixed-GPU FA fix).
- Provenance: all grades first-read at the commit bodies via the GitHub atom feeds +
  the cubecl full .patch; quotes re-verified verbatim at the pins from the
  `.raw/b208_*.diff` files (ephemeral clone, removed at close).

## Gates

batch208 parse+contracts 2/2 (two en-route marker fixes: 'a narrower strided view of
K rows' → the in-rule rendering '512-wide view of 576-wide K rows'; '5 rows for
TQ2_0, 4 for BF16' → the rider's '5 rows TQ2_0, 4 BF16') · kernel_opt+latent_retrieval
lib 3079/0 (3 ignored = the documented freeze-regen e2es; 1223.7 s, one full run per
command) · default lib 2472/0 · clippy `-D` ×3 postures clean (3 doc mechanicals in
the new module doc healed en route: `MUL_MAT+ADD` / `rewind_mla_caches_to` /
`fused_mla_key_concat` backticks + the list/paragraph separator) · retrieval 21/21
floors HELD (230.5 s, no decay, no re-pin) · freeze/score-bench NOT owed (zero
clippy-corpus delta, P30).

Batch 208 / Plan 208. NOTHING INVENTED — every quote read at the pin.
