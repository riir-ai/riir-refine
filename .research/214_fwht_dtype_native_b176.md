# Research 214 — the FWHT dtype-native family (#29096 merge rider) distill verdict

**Status:** RECORD — Batch 176 LANDED 2026-09-27 (~07:5x claim, gates green in-session; the
landing push held to ≥14:20 +0700 per the B175 fired-early precedent — B181 landed 05:20
the same day; +1 rule / +2 riders, corpus 648→649; gates: lib default 2340/0 · kernel slice
374/0 · retrieval 21/21 · clippy -D ×2 clean)
**Date:** 2026-09-27 (~07:2x–08:3x +07, shikuwa/4090 — the riir-clippy idle loop)
**Source:** `ggml-org/llama.cpp` @ `6f856c70990a1b5a159b0d4b2355160c83c3697b` (MIT; the
#29096 merge, 2026-09-26, JohannesGaessler merged after am17an + JohannesGaessler approval,
19 checks; shallow clone `.raw/llamacpp-b176`, **removed at landing**). The rider the idle
loop armed 09-21 and polled **72 times** before `merged_at` fired — the longest-armed rider
in the program so far.

## Verdict

**YES, B+ corpus / route `kernel_opt` N (DtypeAlignment)** — one rule (`dtype-native-
source-read-with-shared-admission-predicate`) + two riders (B96 branchless select; B116
upstream width instance). The rule is the ADMISSION-machinery twin of B178's
exact-widening legality: the same lossless-widening argument, applied to the SOURCE READ
dtype instead of an MMA accumulator, with the new content being the shared-predicate gap
class and the narrow-admission discipline.

## The family map (the rider's real value — five PRs, one axis)

| PR | Backend | Merged | Role in the rule |
|---|---|---|---|
| #27779 | CPU | 09-18 | the CONTRAST arm (convert-then-staging — the avoid-shape) |
| #29094 | Metal | 09-20 | the dtype-native INSTANCE + the supported-size refinement |
| #29096 | CUDA | 09-26 | THE PIN — the shared-predicate gap closure |
| #29095 | Metal wide | 09-25 | the B116 width-ladder instance (fork mechanism → upstream) |
| #29100/#29101 | CUDA wide / Vulkan | Draft | orbit intel — the family continues |

The 09-24 03:5x adjudication (in the snapshot) is the reason the trigger stayed clean: a
body-level read counted #27779 as a satisfied instance; the definition owner's re-arm row
read it commit-level as convert-then-staging — the staging shape the rule exists to
ELIMINATE. Instance counting and contrast-classification are different questions; the
trigger keyed on #29096's own merge either way.

## The rule's three decision contents (all verbatim at the tree)

1. **Cast-at-load.** `template <int N, typename T>` over the source; the load line does
   the conversion while filling registers: `reg[i] = ggml_cuda_cast<float>(src[i * warp_size
   + lane]) * scale;` (fwht.cu L26). Metal twin: `device const src_t * src` +
   `reg[i] = float(src[i*NW + lane])*scale;` (misc.metal L380/L405). Legality = f16→f32
   is EXACT; a lossy conversion would make this a numerics change (the lossy-surface
   walk), never "looks fine".
2. **The shared predicate.** `ggml_cuda_op_mul_mat_use_fwht(op)` carries the whole
   envelope (MUL_MAT + `GGML_HINT_SRC0_IS_HADAMARD` + a-F32 + dst-F32 + b∈{F32,F16} +
   contiguity ×2 + same-shape) and is called by BOTH supports_op and the dispatch.
   From the PR's own text: "Without a shared predicate, supports_op could admit an op
   that ggml_cuda_op_fwht then rejects only after the unconditional same-shape assert has
   already fired; that gap predates this change (it applies to the existing F32 path too)
   but this PR is what touches supports_op, so it closes it here." The predicate SPLIT is
   the defect class; one function, both callers.
3. **The narrow admission.** `if (b->type == GGML_TYPE_F16 && a->type != GGML_TYPE_F16 &&
   !ggml_cuda_op_mul_mat_use_fwht(op)) return false;` (ggml-cuda.cu L5214) — the generic
   F32×F16 kernel does not exist, "so admitting a case the FWHT dispatch then declines
   would fall through to a kernel that does not exist". A too-wide admission converts a
   clean refusal into a wrong-path dispatch.

**REFINE rider (in-rule, Metal #29094):** when the kernel family has a DEVICE-DEPENDENT
envelope, the size term lives INSIDE the shared predicate —
`ggml_metal_op_mul_mat_use_fwht(op, max_tg_mem)` folds `ggml_metal_fwht_supported_size`
(64–512 always; 1024–8192 iff `n * sizeof(float) <= max_tg_mem`; wider refuses), with the
sync contract in-source: "must stay in sync with the kernel_fwht_<type>_<N> templates in
misc.metal" and the fall-through consequence spelled where the envelope lives: "supports_op
and the dispatch must ask the same question: an F16 src1 that is admitted but then falls
through reaches the generic path, which has no F32 src0 by F16 src1 kernel."

## Riders

- **B96 branchless select** (from #29094's body, now in-tree at misc.metal): the
  exchange-form sign branch becomes `reg[j] = val2 - val + 2*((lane & i) == 0)*val;` —
  the boolean predicate multiplies into the sum; +3.0% M5 Pro interleaved A/B
  (source-reported).
- **B116 upstream width instance**: `e351231c4f` (#29095, merged 09-25) ships the
  fork-only row-per-threadgroup ladder upstream at Metal widths 1024–8192 in BOTH dtypes;
  8192 = 32 KB tgmem = the testable device limit, encoded in the supported-size fn as the
  regime gate; MUL_MAT_HADAMARD 26/26, MUL_MAT 1265/1265.

## Orbit intel (routed here, NOT composed)

- **#29077 "ggml : add PQ2_0 and PTQ1_0 ternary types; llama : apply Hadamard-folded
  weights" — CLOSED, not merged** (observed 09-23, closed by 09-26). League-relevant: it
  would have given stock llama.cpp native load support for our scored model class
  (`Ternary-Bonsai-2-27B-PQ2_0.gguf`). Routed to the Issue-846/971 owner lane as a dated
  note only — no league trigger fired (the scored fork carries the format support
  independently; upstream NOT carrying it keeps the decode-axis comparison fork-pinned).
- #29100 (CUDA widths>512, Draft, stacked ON #29096 — `fwht_cuda_block` 256 threads for
  1024–8192 both dtypes) + #29101 (Vulkan F16, Draft): if they merge, they are the CUDA
  instance of B116's ladder + the Vulkan member of the dtype family — future riders, no
  action now.

## Consumer context (why this family is league-adjacent)

Hadamard-folded weights are the Bonsai prefill lane (`bonsai2_hadamard` DEFAULT-ON in
riir-gpu since 2026-09-19; the B166 in-tree trigger). The upstream family's dtype-native
reads mean the scored opponent can load F16 activations for the fold without the staging
pass — the same activation-side traffic axis our kernels own. No league trigger (the pin
is unchanged); recorded for the next re-pin cycle's context.

## Gates (all run this session)

- batch176 ×2 (`batch176_rules_present` + `batch176_riders_present`): kernel slice 374/0.
- lib default 2340/0; count pins 648→649 at router.rs:472, tests.rs ×4 (incl. the
  DtypeAlignment 50→51 category row), mod.rs doc.
- Retrieval floors 21/21 (kernel_heal,latent_retrieval).
- clippy -D clean ×2 postures (one doc_markdown mechanical healed in-passing — the
  module doc's `ggml_cuda_cast`).
- score-bench NOT owed (zero clippy-corpus delta — the P30 precedent; freeze tables
  unmoved).
- Global binary rebuild owed at landing (649/160 stamp verify).

## Pointers

- Plan: `.plans/176_batch176_fwht_dtype_native_source.md`
- Queue line: `.distill/001_mining_queue_snapshot.md` (the landing commit)
- Clone `.raw/llamacpp-b176` REMOVED at landing (the ephemeral-clone law; T0 re-clones at
  the pin).
- Pre-study lineage: `8f2af1ae` (the 09-24 diff read + dedup), the 09-24 03:5x
  adjudication, the 09-25 addendum (`5f82db85`), the 09-23 re-arm row (`.research/206`).
