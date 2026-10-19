# Plan 258 — Batch 254: the #30795 moe_reduce dst-input-aliasing refusal rule

**Status:** ACTIVE (claimed 2026-10-19, shikuwa/4090 idle loop; claim row pushed before authoring)

Source: `ggml-org/llama.cpp` @ `098ce526b546b5a190327c1da82929cee051b9ac` (#30795,
MIT, merged 2026-10-19T08:55Z) — "metal : fix moe_reduce fusion race when dst
aliases its inputs". Found by the standing watch (first commit past the B253
window; fetched at `F:/llama.cpp` this session, diff read at the pin).

The law: a single-dispatch fused kernel that READS its inputs while WRITING
dst cannot trust allocator legality derived from the UNFUSED graph — the graph
allocator computes tensor liveness per-node, where the inputs are fully
consumed by earlier nodes of the fused group, so it can legally recycle input
memory as dst; in the fused kernel that recycling becomes a
threadgroup-vs-threadgroup race (rare non-deterministic wrong results).
Admission must REJECT the fusion when dst overlaps either input; the unfused
path is always correct (safe fall-through). Upstream's own note: "same guard
as the mul_mat_add fusion" — the guard set is PER-FUSION, never
family-inherited (the later fusion omitted it).

## Tasks

- [x] T0 Watch poll: upstream fetched past B253; exactly 1 new commit
      (#30795); prism tip `1180918489` UNCHANGED + newest tag still
      `prism-b10777` → decode re-pin trigger NOT fired (ls-remote this
      session).
- [x] T1 Claim row in `.distill/001_mining_queue_snapshot.md`, commit+push
      BEFORE authoring (the push is the arbiter).
- [ ] T2 Dedup greps both vocabularies (DONE at staging: zero owners of the
      allocator-liveness aliasing law; distinct-from set = B208
      fusion-residual-selected-by-identity-not-negation [operand SELECTION vs
      aliasing REFUSAL] + fuse-residual-via-in-place-output-array [the SAFE
      elementwise in-place alias vs the reducing fusion's cross-threadgroup
      UNSAFE alias] + B208 fa-fast-path V-as-strided-view [aliasing as an
      ADMISSION conjunct when intentional vs refusal when accidental] +
      loop-back-edge-scratch-war-trailing-warp-barrier [B253 — the intra-kernel
      WAR ordering law, adjacent race class, different surface]).
- [ ] T3 Author `entries_gpu_109.rs` — 1 D rule
      `fused-read-write-dst-input-overlap-refusal`, quotes verbatim at the
      pin (inner double-quotes rendered as single quotes per house
      string-literal safety, disclosed in the module header).
- [ ] T4 Wire: `mod.rs` (mod decl + entries() call + KERNEL_CATEGORIES D
      entry + D header comment 213→214); `router.rs` experts pin 905→906;
      count pins in `corpus/tests.rs` (the 905 sites → 906 with the B254
      annotation).
- [ ] T5 `batch254_rules_landed` gate: parse both sides + needle asserts
      (rule present exactly once + the verbatim guard/comment needles).
- [ ] T6 Gates: batch254 gate; kernel_opt lib full; default lib full; clippy
      −D ×3 postures (default / all-features / no-default); retrieval floors
      re-measure (kernel self-variants 100%, blended ≥ 90.6 floor HELD, no
      re-pin expected). Freeze/score-bench NOT owed (kernel_opt-only corpus,
      the P30 precedent).
- [ ] T7 Research note `.research/289_batch254_moe_reduce_alias_refusal.md`
      (highwater 288→289, dual_allocation_gate clean).
- [ ] T8 Docs: AGENTS.md kernel_opt bullet count; `.docs/11_domains/kernel_opt.md`
      batch row; queue-snapshot LANDING row (claim discharged).
- [ ] T9 Commit `feat: batch 254: ...` + push; verify `git status` clean
      after.

## Lane intel (recorded, no tree edit from this lane)

Our absorbed fork-#107 MoE-reduce fusion (B220's
`moe-router-weight-fold-into-reduce-never-materialize`) carries its OWN
aliasing pair per its in-rule text: "dst over the dying weights STAGES them
into pool scratch, dst over x declines" — i.e. the corpus rule already
records the alias-aware admission shape for OUR fusion. The upstream defect
class (allocator-liveness blindness at a LATER fusion) is recorded as watch
for any future fusion lane; no riir-infer/riir-ai edit from this session
(routing restriction + frozen league pins).
