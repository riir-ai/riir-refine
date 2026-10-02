# Research 057 — riir-gpu Batch 51: the FINAL file-mining batch (kimi_k3_gpu_backward_batched + attention_q8kv_cubecl, corpus 344→347, riir-gpu corpus COMPLETE)

**Date:** 2026-08-16
**Batch:** 51 (the FINAL kernel file-mining batch per the Batch 50 queue
recommendation)
**Lanes:** A = `kimi_k3_gpu_backward_batched.rs` (1,398 — the batched twin of
the B40/B41 kimi backward pair); B = `attention_q8kv_cubecl.rs` (967 — the
Q8_0 quantized-KV sibling of the B23 attention cluster).
**Corpus:** 344 → **347** (A 81, D 62, N 19).
**Rules:** `batched-read-slot-membership-decided-at-launch` (D),
`unpack-loop-indexed-by-container-word` (A),
`dequant-placement-by-component-traffic-share` (N).
**Bug yield:** riir-ai Issue 710 (15 hazards; headline: the sync2 push/extract
guard drift with a guaranteed OOB panic path + the 9-site CPU weight-grad
backlog + the production-wired q8kv kernel's untested multi-tile path).

## 1. The rules

### batched-read-slot-membership-decided-at-launch (D)
The sync2 readback in `kimi_k3_gpu_backward_batched.rs` pushes handles under a
local Option-pair guard `(Some(d_gate_pre), Some(w_g_t))` (L1041) but extracts
under `config.use_output_gate` (L1097) — a flag that implies the pair but is
not implied by it. The builder (kimi_k3_gpu_backward.rs L2345-2351) propagates
`w_g = weights.w_g.as_ref().map(...)`, so flag=true + weights.w_g=None yields
push-skips-while-extract-consumes: every slot shifts by one + guaranteed OOB
at the tail. The drift condition is LIVE (constructible through the real
builder), the in-file dead `sync2_layout` vec is the unwired version of the
fix, and the same file's `t6_meta` (L524-641) is the wired positive contrast.
**Distinct from B40's write-plan-derived-from-cache-slots** on the load-bearing
axis: a READ list is consumed by the read — there is no persistent state to
derive the plan from, so the fix shape is genuinely different
(record-at-launch / decide-once binding, not derive-from-cache).

### unpack-loop-indexed-by-container-word (A)
`attention_q8kv_cubecl.rs` Phase 1 (L212-224) indexes the Q8_0 unpack loop by
the 32 LOGICAL bytes: `e/4`, `e%4` runtime div/mod, the same u32 re-loaded 4×,
runtime-variable shift per element. Fix: index by the CONTAINER word — one
load serves 4 bytes, shifts become immediates. Honest scope encoded in the
rule: the redundant loads are L1-hit loads (win = L1 bandwidth + issue slots,
not DRAM). Distinct from B44's bit-cursor (width DISPATCH vs loop
GRANULARITY), B44's hoist-bit-address (invariant hoist vs load-batching —
nothing is loop-invariant here), and popcorn vectorized-pack-load (the
unpack twin).

### dequant-placement-by-component-traffic-share (N)
The q8kv file deliberately splits its Q8_0 KV cache representation: VALUES
stay packed i8 (inline dequant per read) while SCALES are CPU pre-decoded
f16→f32 at upload (L398-400 + L514-517). The rule encodes the per-component
decision (traffic share × conversion cost × read count) and its tension with
B31 dtype-normalize-once-at-upload: **B31 is CORRECT for the sidecar and
WRONG for the payload** — normalizing the whole operand at upload is exactly
naive shape (a) (materialize-to-f32-forever, 4× KV bandwidth). Also encodes
why B14 dequant-once-per-k-tile does NOT apply (a K row's within-workgroup
reuse is 1 — the B35 gate applied to dequant placement). The lane subagent
honestly flagged this rule as possibly-ambient; the coordinator ACCEPTED it
on the grounds that the naive shape (a) is a real, matchable anti-pattern
that B31 would actively encourage if applied blindly.

## 2. Coordinator verification — one CRITICAL refutation

The Lane B subagent's headline hazard ("the module is ORPHANED — never
compiled, no `mod` declaration, tests never run") was **REFUTED by direct
grep**: lib.rs L895-898 wires `#[cfg(feature = "cubecl_runtime")] mod
attention_q8kv_cubecl;` + re-exports; `kv_cache.rs::get_combined_q8kv`
produces its buffers; `AttentionQ8KVCubeCL::launch::<ActiveRuntime>` runs at
3 production sites in `gemma2_cubecl/dispatch.rs` (L702/759/1444). The
subagent's repo-wide grep silently failed (the multi-root-workspace grep
trap — the same class as this box's known terminal flakiness). Recorded in
Issue 710 so future audits don't re-chase it.

**Consequence of the refutation:** the file's hazards are
LIVE-production-path, so the "multi-tile path untested" finding was
UPGRADED (HIGH, not academic): production decodes with context > 256 tokens
run the never-tested cross-tile online-softmax rescale.

## 3. Issue 693 closeout verified (good news)

- H1 (MLA d_attn_w spurious aw factor): **fix present** in both twins
  (batched L1156-1165 + unbatched L2605-2609, corrective comments).
- H2 (MoE latent-norm gamma over-application): **fix present** (batched
  L407-409, gamma scoped to the dy term).
- The broken LCG test generator: **fixed** where it lives (mantissa-masked,
  Issue 693 H3 comments at all three sites).
- The B40 corpus note claiming the H2 slip "ALSO lives in the batched twin
  L407" was STALE (true at mining time, fixed since) — refreshed in this
  batch's commit (entries_gpu_05.rs).

## 4. Drop tables (condensed)

**Lane A (15 dropped):** rank-l weight-grad GEMMs → popcorn
weight-grad-batched-outer-product (the file is a showcase of it); shared H^T
operand → B34/B33; k_c_j_h recompute inside the t sweep → B47 backward twin
(hazard, not rule — the corpus rule exists); MLA 9 CPU outer-product sites →
non-application backlog (hazard); per-expert latent re-upload → B31; d_gate
dual upload → interface limitation (thin); dead _d_h_g_batch + clones → B41;
read_handle value-return → B39's own naive example; naive transpose → B37
**but measured benign at l≤16 shapes (honest drop)**; per-(t,head,j) allocs →
B43/rust_perf; phase-1 GEMM riding pipeline → B36 positive instance;
CPU-added weight grads → B27 architectural alternative; T6 expert grouping →
mechanism not decision; seq contract → ambient (unchecked part filed);
per-head strided gather → thin layout churn.

**Lane B (14 dropped):** keys||values split-half → B23 instance; GQA formula
→ B23 verbatim; softcap tanh → B23; hardcoded constants + launch_unchecked →
B23/B29; online softmax + smem reductions → ambient B23 orchestration;
sign_extend two-branch → documented CubeCL workaround (trade-off, not rule);
query re-read → B35 instance; scale per-32-threads → B35 instance; f16→f32
pre-decode → B31 instance (folded into R3 as the contrasting half);
per-element scale mul → B18 non-application (hazard); two-buffer layout →
documented Q4_K-matching trade-off; n_positions from payload → B32b positive;
CPU alloc churn → rust_perf; 8-workgroup dispatch → ambient decode shape.

## 5. GOAT

- **G1**: `batch51_pseudocode_parses` PASS (3 rules × ref+opt syn-parse).
- **G2**: **577 kernel_opt + 415 default** lib tests, 0 failures (+3 gates
  vs B49's 574).
- **G3**: clippy **0** in default + kernel_opt states (2 doc_markdown
  backtick fixes pre-commit — the `.rs`-filenames-in-`//!`-prose gotcha, 5th
  recurrence); all-but-ternary combo check clean (9.46s); CI guard layers
  1/2 green (3/4/5 remain Windows-blocked by the pre-existing Issue 707).
- **Retrieval floors (Issue 017 gate, kernel-inclusive 45.16s): HELD —
  kernel self-variants 30/30 = 100/100/100 (crowding alarm floor), blended
  3-domain 126 spans 77.8/87.3/92.9 — all above the Issue 022 re-pinned
  minimums (77.1/86.4/92.4).** No decay from the 3 new rules.

## 6. Corpus COMPLETE — the program's natural end

With Batch 51, the riir-gpu file-mining program is **COMPLETE** (~115 of
~200 files mined by line-weight; the unmined remainder is: the
sibling-blocked `ternary_deltanet_gpu_forward.rs` cluster [re-verify clean
before any future batch], test files, B26-covered `forward/` dir internals,
and thin config/stub leaves judged out-of-scope at Batch 50). Batch 50's
zero-rule result on fresher files predicted this; Batch 51's 3-rule yield on
the two highest-value remaining files confirms the tail was thin but not
empty — the right place to stop.

**Queue shift (per the Batch 50 recommendation, now in effect):** the idle
loop's mining item moves to rust_perf + issue-driven work; kernel_opt
remains open for measured-GOAT distillations (the B49 path) and sibling-WIP
unblocks (ternary_deltanet).

## 7. Lessons

1. **Subagent greps on multi-root workspaces can fail SILENTLY and
   produce confident CRITICAL claims** — the Lane B orphan hypothesis was
   fully wired at 3 production sites. Coordinator grep-verification of
   every CRITICAL reachability claim is now a standing rule (this batch's
   instance; Batch 33's was comments-vs-kernel, this is grep-vs-wiring).
2. **Category membership must be verified by ROW INSPECTION, not rule-text
   memory** — the generator's first pass appended `dequant-placement` to
   the B row because "rademacher-signs" (remembered as N) actually lives in
   B. The count-pin test table caught nothing at write time (it only runs
   at test time); the fix was a row-span inspection. Add to the generator
   checklist: verify the anchor id's row BEFORE appending.
3. **The concepts-covered case-sensitivity gotcha struck a 4th time**
   ("NO persistent state" vs "no persistent state") — always assert the
   EXACT case of the prose, or lowercase both sides.
4. **Stale corpus notes are themselves findings** — the B40 note asserting
   the H2 slip "ALSO lives in the batched twin" had been falsified by a
   later fix landing at exactly the cited line. Corpus commentary that
   names live-bug locations should be re-verified when the bugs are fixed
   (this batch refreshed it).
