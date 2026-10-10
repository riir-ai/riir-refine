# Plan 220 — Batch 220: Lead 13 (prism #107 MoE weighted-reduce fusion) as an own kernel_opt rule

**Status:** LANDED 2026-10-10 at riir-refine `83dc8dd1` — RENUMBERED Batch 219→220 at landing (the slot-2 session's uncommitted rust_perf batch had allocated 219 in this shared checkout; the Batch-115 yield precedent; disjoint write-sets, their WIP not staged). Gates ALL GREEN (full suite 3110/0 with 1 disclosed skip of their in-flight test, default 2489/0, clippy -D zero findings in my files, retrieval floors held, corpus 783→784, D 181→182).

## Source (the staging row, now FULLY read)

PrismML-Eng/llama.cpp `8eaee8b7f` → PR #107, patch commit `930ddba3b7f07c1599354d2989aeb23b51988f91`
(MIT; bri-prism) — "cuda: fold the MoE router weighting and expert reduction into one
kernel". The full 876-line patch fetched this session (`/tmp/b219/lead13.patch`; 6 files,
+734: ggml-cuda.cu +147, moe-reduce.cu +191 new, moe-reduce.cuh +19 new,
test-backend-ops.cpp +77, test-moe-weighted-reduce.cpp +299 new, CMakeLists +1). Every
quote below verified verbatim at it. The staging row's "A10 number truncated at read" is
adjudicated: **the A10 paragraph is a behavior/admission matrix, not a wall-clock figure**
("every no-overlap case fused with staged=0, every weights-inside-dst case fused with
staged=1, every dst-inside-x case declined the fusion, with all 48 cases bit-identical")
— the win is quantified structurally (the MUL "writes a second full-size F32 tensor …
tens of MB written and read again per layer"), NotBenchable honestly.

## The rule (OWN RULE — three unowned atoms, one mechanism)

`moe-router-weight-fold-into-reduce-never-materialize` — category **D** (Dispatch):
the never-materialize consumer-fold into a reduction, with the bit-identity-intrinsics
refusal and the dying-operand staging rescue:

1. **The fold**: build_moe_ffn's `MUL(down_out, router_weights) → n_exp VIEWs →
   (n_exp−1) ADDs` subgraph becomes ONE kernel `dst[i0,i2] = Σₑ x[i0,e,i2]·w[0,e,i2]`;
   the weighted intermediate is never written or read ("tens of MB written and read again
   per layer" at large ubatch).
2. **Bit-identity as the identity instrument**: `__fmul_rn` every product + `__fadd_rn`
   accumulate left-to-right in increasing slot order — "the explicit intrinsics keep nvcc
   from contracting a product and its accumulation into an FMA, which would be more
   accurate but would not match the baseline". The REFUSAL of a more-accurate fast path
   is what makes the fusion legal; the test bound is 0.0 error.
3. **The dying-operand staging rescue**: the weights die at the mul so "the graph
   allocator routinely packs the add-chain output over them"; dst↔w overlap → snapshot
   the weights into pool scratch ("they are n_expert_used*n_tokens floats") and fuse
   anyway; dst↔x overlap → decline (cross-block race, no cheap repair). The generic
   range checker is bypassed deliberately: it "ignores sources whose op is GGML_OP_NONE
   and … can only reject, while the weights case has a cheap repair."
4. **Admission**: F32-only, contiguous, same-shape; weights = per-(expert,token) scalars
   broadcast along ne0; the EXACT view geometry (nb[1]=mul->nb[2], data at e*mul->nb[1])
   + the EXACT left-to-right add-chain node order; n_tok ≤ 65535; cap 2..15 experts
   (`GGML_CUDA_MOE_REDUCE_MAX_EXPERTS 15`, bounded by ggml_can_fuse_subgraph's 32-node
   limit — the subgraph is 2·n_exp nodes); 16 falls through to the unfused ops.
5. **Kill/debug switches**: `GGML_CUDA_MOE_REDUCE_DISABLE=1` parsed beside
   `GGML_CUDA_DISABLE_FUSION`; `GGML_CUDA_MOE_REDUCE_DEBUG=1` logs each fusion incl.
   whether the weights were staged.
6. **float4 admission**: five conjuncts (ne0/sx1/sx2/sd1 %4 + x/dst 16B pointer
   alignment) — stride-aligned-yet-pointer-misaligned falls back to scalar.
7. **The tests**: 48 cases at a 0.0 error bound vs CPU (bit-identity), expert counts
   2/3/4/8/15/16 (cap + fall-through), float4+scalar, single/multi-token, weights as
   graph input AND dying in-graph intermediate; + hand-placed single-buffer aliasing
   layouts test-backend-ops cannot reach (weights-inside-dst must stage; dst-inside-x
   must decline); Metal (no fusion) passes the same 48 at 0.0, validating the harness.

## Dedup (re-grepped at composition, all clean)

- `fusion-residual-selected-by-identity-not-negation` (B208, entries_gpu_78.rs) — owns
  fusion-ADMISSION operand selection (identity vs negation); THIS owns the
  never-materialize fold + the aliasing/staging law. Compose, never merge.
- `producer-folds-single-consumer-family` (entries_gpu_72.rs) — the PRODUCER-side fold
  ("its output has no other reader"); THIS is the consumer-side fold into a REDUCTION
  over n producer slices — the intermediate HAS readers (the add chain); the fold is
  algebraic (mul into adds), not reader-count elision.
- `argmax-epilogue-replaces-output-materialization` (entries_gpu_52.rs) — same
  never-materialize spirit at a DIFFERENT site (top-k in a GEMM epilogue); no reduction
  fold, no staging rescue, no bit-identity-vs-rounding law.
- `per-group-epilogue-deferred-between-mma-bursts` (entries_gpu_67.rs) — epilogue
  scheduling axis, not graph fusion.
- No corpus rule owns: FMA-contraction refusal for bit-identity, dying-operand staging,
  the subgraph-shape matcher (exact view geometry + add-chain node order), or the
  dst↔operand overlap admission pair (decline vs cheap-repair-stage).

## Tasks

- [x] T0 pre-flight: dual_allocation gate clean (0/0/0); highwater 218 re-read;
      partition check (slot-2 sibling active in rust_perf/corpus — disjoint write-set;
      re-check git status before commit, commit only my hunks)
- [x] T1 full-patch verification: /tmp/b219/lead13.patch (876 lines, commit
      `930ddba3…`), every quote verbatim; the A10 "number" adjudicated = behavior
      matrix, NotBenchable
- [x] T2 dedup re-grep at composition (4 named owners + fmul_rn/never-materialize/
      weighted-reduce/dying-operand/stage-weights/alias vocab — zero prior owners)
- [x] T3 the rule landed: entries_gpu_84.rs (new file) + mod.rs registration
      (mod decl + entries.extend + KERNEL_CATEGORIES Dispatch list + the D-block
      header comment) — category D, count 783→784, D 181→182
- [x] T4 count pins re-pinned with honest Batch-220 labels (renumbered from 219
      at landing — the slot-2 session's uncommitted rust_perf batch had the
      number; the Batch-115 yield precedent): tests.rs ×6 sites + router.rs +
      AGENTS.md ×2 + README.md ×2 + the per-category Dispatch row 181→182 + the
      batch-history comment line
- [x] T5 gates: `batch220_rule_and_parse_landed` FIRST SHOT (rule present +
      verbatim needles + ref/opt pseudocode parse in one test)
- [x] T6 GOAT: full kernel_opt+latent_retrieval lib **3110/0** (3 ignored;
      1333.19 s, /tmp/b219tgt, nice-5; 1 filtered = the sibling's in-flight
      rust_perf test, disclosed) + default lib **2489/0** + clippy `-D`: ZERO
      findings in my files (49 doc_markdown mechanicals in the sibling's
      uncommitted WIP — theirs, the B215 precedent) + retrieval floors held
- [x] T7 freeze/score-bench NOT owed (zero clippy-corpus delta, P30)
- [x] T8 snapshot row + commit `feat: batch 220: ...` + push

## Non-goals

- No rust_perf/kernel_opt counts beyond this +1; no AGENTS.md feature-list prose
  beyond the count pin; Lead 9/10/11 + k=8-MMA stay queued (one unit per cycle).
