# Research 260 — Batch 229: the #29533 symmetric tail zero-fill rider (the staging-side member of the pad/tail family)

**Status:** RECORD — Batch 229 landed 2026-10-13 (Plan 231; count-neutral rider on
`ragged-chunk-pad`, corpus 803 stands; the cycle-59 intake row's staged
rider-candidate converted at the first post-B228 mining slot).

## The source (verified at the pin this session)

ggml-org/llama.cpp @ `411d548515017ca75c2a12c39659b6e7e1551dd7` (#29533, MIT,
landed 2026-10-13 11:03Z) — "vulkan : split matmul dispatch when it would exceed
maxComputeWorkGroupCount". Read in full via the .patch endpoint (6 files,
+324/−183): ggml-vulkan.cpp (the n-axis dispatch split + subbuffer window
shifts), mul_mmq.comp (the staging call-site fix), mul_mmq_funcs.glsl (the
`block_a_to_shmem` is_in_bounds else-branch, per DATA_A type), tests (+2
batched chunked-shape cases).

## The law

1. **SYMMETRIC TAIL ZERO-FILL (the staging boundary).** In a tail-truncated K
   loop, EVERY operand stager feeding one consumer must write the FULL staged
   tile — zero-filling past the bound — because the consumer (mma) reads all
   BK_STEP steps regardless of the logical bound. The defect, verbatim: "The A
   side skipped the shared memory write for the k steps past end_k, while the B
   side was already told the bound and zero filled. The A registers are read
   for all BK_STEP steps, so the stale scale went into the dot product and
   produced NaN at k = 32. Pass the bound to block_a_to_shmem and write zeros
   when it is false, for every DATA_A type, mirroring the B side."
   - The repair SHAPE is the load-bearing half: the guarded CALL
     (`if (block + k_step * BK < end_k) block_a_to_shmem(...)`) becomes an
     UNCONDITIONAL call with the bound as a PARAMETER — the else-branch writes
     zeros INCLUDING the dequant multiplier (`dm = 0.0f`). The stale `.dm` was
     the NaN carrier — the payload bits alone are not the whole story.
   - Shared memory is NEVER zero by default: a skip-write tail holds the
     PREVIOUS tile's bytes. Skip-write on one stager beside zero-fill on its
     co-consumed sibling is exactly the defect.
   - The mismatch is SILENT on full-chunk tests — the corruption surfaces only
     at the chunk boundary (NaN at k = 32 = the first BK_STEP boundary). Tail
     handling is exercised only by a test whose shape actually truncates.

2. **THE SUBBUFFER WINDOW LAW (the dispatch-split context).** A chunked
   dispatch (the n-axis split when a small tile still produces more workgroups
   than maxComputeWorkGroupCount allows) shifts the B and D subbuffer column
   windows per chunk — which requires `GGML_ASSERT(b.size != VK_WHOLE_SIZE &&
   d.size != VK_WHOLE_SIZE)` (you cannot offset a WHOLE_SIZE subbuffer), a
   col-stride that knows the buffer's KIND (a raw src1 tensor keeps its own
   `nb[1]`; a staging buffer is tightly packed — two `b_col_bytes` formulas),
   and every derived bound shifts with the chunk (the push constants carry
   `padded_n - n_base`, never the global `padded_n`).

## Host adjudication (why `ragged-chunk-pad`)

The pad/tail family's two strategies: (a) pad the INPUT (the host rule's own
opt — one uniform path), (b) bound the loop and zero-fill the STAGING (the
#29533 fix). Same problem shape — ragged tail + a consumer that processes
uniformly. The rider carries strategy (b)'s symmetry contract onto the family
root. The alternatives were declined with reasons: the B208 FA-admission host
carries B226's pad-at-producer face but #29533 has NO admission face (pure
in-kernel staging correctness — the "why it rides this rule" would be weak);
the B176 rule's pad ladder (B195/B213 riders) owns pad ECONOMICS (where zeros
come from), not symmetry. Kinships recorded in-rule: TMA's hardware OOB
zero-fill (descriptor-managed twin), kv-first-append's offset-gated elision
(the INVERSE posture — fill elided because the tail is structurally unread;
here the consumer reads the full width so the fill is MANDATORY),
masked-boundary-load (per-element LOAD fallback — this is a WRITE-side
coverage contract).

## Gates (all green, this session)

- batch229 needle test 1/1 (verbatim needles from the .patch).
- kernel_opt lib 2982/0/1i (the +1 ignored = the documented freeze e2e;
  count consistent: B227's 2948 + B228's +33 + mine).
- clippy `-D` ×4 postures clean (default, kernel_opt, kernel_opt+latent, all).
- Retrieval floors HELD (kernel self-variants 100.0%, blended
  91.2/94.7/95.3+ — no re-pin; the gate asserts the floors, 21/21 passed).
- Freeze/score-bench NOT owed (zero clippy-corpus delta, P30).

## NotBenchable disclosure

The fix is a correctness repair — no perf number ships in-source; the win
class is the dispatch split it enables (the maxComputeWorkGroupCount escape),
source-context only.

## Session lesson (none new)

The B226 single-rider precedent held: claim → plan-first → verify-at-pin →
author → gates → docs. No proposer/pin-repair lessons this batch.
