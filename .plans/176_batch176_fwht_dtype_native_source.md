# Plan 176 — Batch 176: the FWHT dtype-native source read (the #29096 merge rider, FIRED)

**Status:** IN PROGRESS — claimed 2026-09-27 ~07:5x +07 (shikuwa/4090, riir-clippy idle
loop). Trigger FIRED: ggml-org/llama.cpp #29096 "cuda: add F16 input to the FWHT"
MERGED 2026-09-26 as `6f856c7` (`6f856c70990a1b5a159b0d4b2355160c83c3697b`) — the
rider's `merged_at` trigger, armed 09-21/09-23/09-24 (re-arm + adjudication rows in
`.research/206` + the snapshot). Composition was pre-graded (`8f2af1ae` pre-study +
the 09-25 addendum + the 09-24 03:5x adjudication); quotes re-verified VERBATIM at
the tree this session (shallow clone at `6f856c7`, T1 below). **LANDING CONSTRAINT:
B181 landed 05:20 +0700 today — the landing commit goes ≥14:20 +0700 (~9h, the
B175-fired-early precedent), nominal slot ~19:5x.** Same-day corpus work (authoring,
gates) runs now; the push waits for the window.

## The composition (fully pre-graded; this plan pins it)

1. **LEAD — new rule (N 50→51), `entries_gpu_51.rs`**:
   `dtype-native-source-read-with-shared-admission-predicate` — a transform kernel
   whose only legal input widening is EXACT (F16→F32) reads the source in its NATIVE
   dtype and converts AT LOAD (`ggml_cuda_cast<float>(src[i * warp_size + lane])` —
   the conversion rides the register fill), never via a convert-then-staging copy
   (the CPU #27779 contrast: `ggml_compute_forward_fwht` converts F16→F32 into wdata
   first). The dtype widening requires the ADMISSION machinery to widen WITH it, and
   the admission must be ONE PREDICATE shared by supports_op and the dispatch
   (`ggml_cuda_op_mul_mat_use_fwht`) — the gap class, from the PR's own text:
   "Without a shared predicate, supports_op could admit an op that ggml_cuda_op_fwht
   then rejects only after the unconditional same-shape assert has already fired;
   that gap predates this change (it applies to the existing F32 path too) but this
   PR is what touches supports_op, so it closes it here." The relaxation is NARROW:
   `b->type == GGML_TYPE_F16 && a->type != GGML_TYPE_F16 && !use_fwht(op)` refuses —
   the generic F32-src0 × F16-src1 path does not exist, so a too-wide admission falls
   through to a kernel that does not exist.
   **REFINEMENT RIDER (Metal #29094, one rider column on the same rule):** when the
   kernel family has a DEVICE-DEPENDENT envelope (threadgroup memory), the size term
   lives INSIDE the shared predicate — `ggml_metal_op_mul_mat_use_fwht(op, max_tg_mem)`
   folds `ggml_metal_fwht_supported_size` in, with the sync contract in-source:
   "must stay in sync with the kernel_fwht_<type>_<N> templates in misc.metal" +
   "supports_op and the dispatch must ask the same question: an F16 src1 that is
   admitted but then falls through reaches the generic path, which has no F32 src0
   by F16 src1 kernel."
   Category **N** — the exactness of the F16→F32 widening is the entire legality,
   kinship `exact-widening-enables-fp16-pv-accumulation` (B178, entries_gpu_48:
   the PV-accumulator dtype twin; pay explicit distinct-from), contrast arm
   convert-then-staging (#27779). Sources (all verbatim at `6f856c7`):
   ggml/src/ggml-cuda/fwht.cu L5 (`template <int N, typename T>`), L26
   (`reg[i] = ggml_cuda_cast<float>(src[i * warp_size + lane]) * scale;`), L101-110
   (`ggml_cuda_op_mul_mat_use_fwht` body: MUL_MAT + hint + a-F32 + dst-F32 +
   (b-F32 || b-F16) + contiguous ×2 + same_shape), L118-130 (the impl switch),
   ggml/src/ggml-cuda/ggml-cuda.cu L5214 (the narrow supports_op refusal);
   Metal: ggml/src/ggml-metal/kernels/misc.metal L380 (`device const src_t * src`),
   L405 (`reg[i] = float(src[i*NW + lane])*scale;`), L507-528 (the f32/f16 template
   sets, same host names), ggml/src/ggml-metal/ggml-metal-common.cpp L13-33
   (supported_size + the Metal shared predicate + both sync comments). Tests: 6 new
   F16 cases in tests/test-backend-ops.cpp (64/128/256/512 single-row, 128×32,
   256×512 many-rows). Merged 09-26, JohannesGaessler + am17an approved, 19 checks.
2. **RIDER — branchless butterfly select (on `fwht-shuffle-xor-butterfly-register-resident`,
   B96, entries_gpu_11)**: from #29094's body, now IN-TREE upstream: the exchange-form
   sign branch `if ((lane & i) == 0) { a + b } else { b - a }` becomes
   `reg[j] = val2 - val + 2*((lane & i) == 0)*val;` (misc.metal L405 region, the
   cross-lane stages) — the predicate multiplies instead of branching; +3.0% M5 Pro
   interleaved A/B (PR body, source-reported). Rider text on the B96 rule's concept
   column.
3. **RIDER — the #29095 upstream width instance (on
   `fwht-wide-block-row-per-threadgroup-exchange-ladder`, B116, entries_gpu_27)**:
   the fork-only mechanism is now UPSTREAM (`e351231c4f`, merged 09-25, Metal
   widths 1024–8192 in BOTH dtypes via `kernel_fwht_tg`, 8192 = 32 KB tgmem = the
   testable device limit, `ggml_metal_fwht_supported_size` stops there, wider falls
   back to the generic matmul). MUL_MAT_HADAMARD 26/26, MUL_MAT 1265/1265. The
   instance is the evidence; no new rule. (The B180 CUDA-twin rider already carries
   the arch-gated 512 boundary + signs-at-load on this rule.)
4. **ORBIT INTEL (research note only, no corpus content)**: #29100 (CUDA widths>512,
   Draft, stacked ON #29096 — `fwht_cuda_block` 256 threads for 1024–8192 both
   dtypes) + #29101 (Vulkan F16, Draft) + #29077 (PQ2_0/PTQ1_0 ternary types +
   Hadamard-folded weights — CLOSED not merged; league-relevant, report to the
   846/971 owner lane in the note only).

## Dedup (re-verified this session against the live corpus)

- B96 `fwht-shuffle-xor-butterfly-register-resident` (entries_gpu_11): owns the
  all-register exchange MECHANISM (N ≤ LANES × budget, zero smem). The dtype rule
  owns the SOURCE TYPE + ADMISSION axis. Distinct; compose (the new rule's kernels
  ARE B96-shaped below 512). Rider 2 lands here.
- B166 `fwht-butterfly-staging-ladder-warp-smem-register` (entries_gpu_45): stage
  PLACEMENT by span. Distinct.
- B116 `fwht-wide-block-row-per-threadgroup-exchange-ladder` (entries_gpu_27):
  width REGIME past the register wall (+ the B180 CUDA-twin rider). Distinct;
  rider 3 lands here.
- B105 `rht-epilogue-fusion-contract` (entries_gpu_18): WHERE the transform fuses
  into a GEMM epilogue. Distinct.
- B181 `fused-transform-quantize-writer` (entries_gpu_50): the writer layer; its
  own text cross-references the ladder family. Distinct.
- B178 `exact-widening-enables-fp16-pv-accumulation` (entries_gpu_48): the KINSHIP —
  exact widening licenses a dtype change; that rule's surface is the ACCUMULATOR
  dtype of an MMA, this one is the SOURCE READ dtype + the admission machinery.
  Pay distinct-from in the new rule.

## Tasks

- [x] T0 claim + plan (this commit); `.plans/.highwater` repaired 174→176 (the
      folder carried an unrecorded 175 — the known bump-miss class).
- [x] T1 verbatim quote re-verification at the tree — shallow clone at
      `6f856c70990a1b5a159b0d4b2355160c83c3697b` under `.raw/llamacpp-b176`
      (REMOVE at landing per the ephemeral-clone law). All quotes in the
      composition above verified this session: fwht.cu L5/L26/L101-110/L118-130;
      ggml-cuda.cu L5214; misc.metal L378-380/L405/L507-528; ggml-metal-common.cpp
      L13-33; ggml-cpu.c L1263 (the contrast route); 6 F16 test cases. The
      review-response commit `6587611` ("cuda: use ggml_cuda_cast in the FWHT load,
      drop the comment") is IN the merged form — the load reads ggml_cuda_cast,
      NOT the pre-studied head's manual cast; the pre-study's quotes were re-graded
      against the merge, no composition change.
- [x] T2 author `entries_gpu_51.rs` (the new N rule + module doc), register in
      `corpus/mod.rs`, rider 2 on entries_gpu_11 (B96), rider 3 on entries_gpu_27
      (B116). DONE — plus the DtypeAlignment registration in mod.rs's
      KERNEL_CATEGORIES (the category table lives there, not in the plan's
      original guess).
- [x] T3 count pins 648→649: router.rs:472; tests.rs:106 + :5437/:5448
      (category table: DtypeAlignment 50→51) + :9556; mod.rs:2016 doc;
      AGENTS.md (kernel_opt bullet + kernel_heal feature line);
      `.docs/11_domains/kernel_opt.md` (seed-corpus row + the batch paragraph);
      README kernel_opt row; corpus_history.jsonl 09-27 second row. DONE.
- [x] T4 standing gate: `batch176_rules_present` + `batch176_riders_present` in
      corpus/tests.rs. DONE (the first insertion nested inside
      batch181_pseudocode_parses by a fuzzy-edit mis-aim — caught by the
      unclosed-delimiter compile error, repaired to the original body + two
      top-level fns; the miss recorded here).
- [x] T5 GOAT: G1 syn-parse (kernel slice 374/0); G2 lib default 2340/0 +
      kernel_heal 2714/0; G3 clippy -D at default + kernel_heal postures (one
      doc_markdown mechanical healed); retrieval floors 21/21. DONE.
- [x] T6 research note `.research/214_fwht_dtype_native_b176.md` (highwater
      213→214) + snapshot row + this plan closed. DONE.
- [ ] T7 global binary rebuild + stamp verify (649/160); `.raw` clone removed;
      landing commit + push **at ≥14:20 +0700** (the ≥9h B175 precedent; B181
      landed 05:20 today). The gated work sits as a LOCAL commit; the next
      session pushes it after a clock check + origin twin-check if this one is
      gone by then.

## Landing checklist (the slot session's order)

origin twin-check (`git fetch` + log for any B176 rows — THIS plan's claim commit
is the only one expected) → verify the checkbox state → landing commit
`feat: batch 176: FWHT dtype-native source read (#29096 merge rider; ...)` → push →
remove `.raw/llamacpp-b176` → snapshot row in the SAME landing commit.
