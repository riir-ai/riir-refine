# Research 289 — Batch 254: the #30795 moe_reduce dst-input-aliasing refusal

**Status:** RECORD

## Source

`ggml-org/llama.cpp` @ `098ce526b546b5a190327c1da82929cee051b9ac` (#30795, MIT,
merged 2026-10-19T08:55Z) — "metal : fix moe_reduce fusion race when dst
aliases its inputs" (Georgi Gerganov; assisted-by pi:llama.cpp GLM-5.3-Flash
on 2x DGX Spark RDMA TP=2). Found by the standing watch: the FIRST commit past
the B253 window (fetched at `F:/llama.cpp` 2026-10-19; exactly 1 new commit in
the window — a thin watch day converted as a solo batch, the B209/B210
precedent).

## The law (one sentence)

A single-dispatch fused kernel that READS its inputs while WRITING dst cannot
inherit allocator legality computed on the UNFUSED graph — the allocator sees
per-node liveness (inputs fully consumed by earlier nodes of the fused group,
so recycling input memory as dst is legal), the fused kernel sees one dispatch
whose read/write set spans the whole group, and the disagreement is a
threadgroup-vs-threadgroup race; the fusion admission is the only seam that
can see the conflict and must REJECT the fusion on dst/input overlap, falling
through to the always-correct unfused path.

## Quote ledger (all verbatim at the pin)

Commit message:
- 'The fused moe_reduce kernel computes dst = sum(experts[e] * weights[e]) in
  a single dispatch, reading experts and weights while writing dst.'
- 'The graph allocator computes tensor liveness on the unfused graph, where
  the inputs are fully consumed by earlier nodes of the fused group, so it can
  legally recycle the input memory as dst.'
- 'In the fused kernel this becomes a threadgroup-vs-threadgroup race that
  produces rare non-deterministic wrong results (observed with test-llama-archs
  -a kolibri on the meta backend).'
- 'Reject the fusion when dst overlaps either input, same guard as the
  mul_mat_add fusion. The unfused MUL + ADD path is always correct.'

Diff (ggml/src/ggml-metal/ggml-metal-fusion.cpp, +5 in
`ggml_metal_fusion_check_moe_reduce`, hunk @@ -636,6 +636,11 @@):
- :640 comment: '// the fused kernel reads experts and weights while writing
  dst, so dst must not overlap either input'
- :641-643: 'if (ggml_metal_fusion_overlap(match.dst, match.experts) ||
  ggml_metal_fusion_overlap(match.dst, match.weights)) { return false; }'

## Decision content (the four clauses in-rule)

1. **Liveness authority split** — allocator legality is per-node on the
   unfused graph; a fused kernel's read/write set spans the whole group; an
   input dead at its own node is live across the fused dispatch.
2. **Race granularity** — elementwise same-thread read-then-write tolerates
   the alias (the deliberate in-place family); a REDUCING fusion (dst = sum
   over threadgroups' contributions) does not: group g writes while group h
   reads the aliased input, unordered.
3. **Refusal = safe fall-through** — the unfused path computes the same graph
   correctly (the B176 superset-admission law's cousin).
4. **The guard set is per-fusion, never family-inherited** — the commit's own
   'same guard as the mul_mat_add fusion' proves the older fusion carried the
   guard and this later one omitted it. Every new fusion re-derives the full
   admission conjunct set from the fused kernel's actual read/write set.

## Class signature

Rare NON-DETERMINISTIC wrong results (never a crash, never
deterministic-per-layout like the B208 stale-buffer mispick — the race window
depends on threadgroup scheduling), surfacing only when the allocator happens
to recycle (shape-dependent) — which is why it reached production as a flaky
model-output bug rather than a kernel bug.

## Dedup + kinship (greps run at claim, zero owners)

- `moe_reduce` — only a rider mention inside
  `shared-activation-quantize-memo-per-eval` (entries_gpu_07); no owner.
- `unfused|allocator.{0,20}liveness|recycle.{0,20}(input|dst)|dst.{0,15}overlap`
  — zero owners; hits are the gate|up fall-through rider and pass-merging
  prose.
- Distinct-from set (in-rule): B208 fusion-residual-selected-by-identity
  (operand SELECTION vs aliasing REFUSAL) · fuse-residual-via-in-place-
  output-array (deliberate elementwise in-place alias, safe vs reducing-
  fusion unsafe) · B208 fa-fast-path V-as-strided-view (aliasing as an
  admission CONJUNCT when intentional vs refusal when accidental) · B253
  loop-back-edge-scratch-war-trailing-warp-barrier (intra-kernel WAR across a
  loop back-edge vs allocator-created overlap across the fusion boundary —
  both 'the optimized path's own memory ordering vs the naive path's implicit
  correctness') · captured-graph-bakes-buffer-addresses (capture-time address
  baking, not admission-time refusal).

## Grading

D (dispatch/admission correctness), NotBenchable (the evidence class is a
reproduced race + a 5-line guard; the guard's cost is one overlap check per
admission). Solo batch (1 rule, corpus 905→906, D 213→214) — the thin-watch
day converted whole; B209/B210 precedent.

## League / lane intel

- Upstream is OUR absorbed fusion family: the fork's #107 MoE-reduce fusion
  was absorbed at Bench 982 (ours +3.6% in-band). Our corpus rule B220
  (`moe-router-weight-fold-into-reduce-never-materialize`) already records
  OUR fusion's aliasing pair: 'dst over the dying weights STAGES them into
  pool scratch, dst over x declines' — the alias-aware admission shape.
  The upstream defect (allocator-liveness blindness at a LATER fusion) is
  recorded as watch for any future fusion lane. No riir-infer/riir-ai edit
  from this session (routing restriction + frozen league pins); a targeted
  grep of riir-infer for a moe_reduce-shaped fusion surface found none
  (league model dense; MoE second-tier per the standing note).
- Metal-side fix — no CUDA cell moves; the decode re-pin trigger stays the
  fork-tag rule (checked this session: prism tip `1180918489` UNCHANGED,
  newest tag `prism-b10777` — NOT fired).

## Gates (all run this session)

- batch254 needle gate 1/1 FIRST-RUN PASS.
- kernel_opt+latent_retrieval lib 3351/0 (3 ignored = the documented
  freeze-regen e2es; 1670 s). En-route pin catch: the per-category expected
  table still asserted Dispatch==213 — repaired to 214 (the B249-class
  pins-doing-their-job catch).
- default lib 2696/0.
- retrieval floors HELD: blended 90.6% == floor, no decay, no re-pin; 21/21
  retrieval tests.
- clippy −D clean: default / all-features / no-default / kernel_opt, all at
  --all-targets. En-route: (a) doc_markdown on my module header (`moe_reduce`
  backticked); (b) an unused `HealDomain` import in the batch254 test;
  (c) PRE-EXISTING at HEAD: the B250-T6 `batch250_pareto_benches` doc comment
  (rust_perf/corpus/tests.rs:7405, landed `de8e167b` this morning) carried a
  doc_markdown on `#[cfg(target_arch = "x86_64")]` — repaired with the
  attribute backticked (mechanical, the B205 en-route-repair precedent).
- Freeze/score-bench NOT owed (kernel_opt-only corpus delta, P30). Binary
  rebuild NOT owed (kernel_opt not in the release set).

## Queue after

- B255 needs a FRESH watch pass (the window is consumed again — exactly 1
  commit converted).
- L8 league-posture A/B: owner-gated behind the StartLux stop (claim to
  2026-10-20T13:30Z; the verdict channel claude_code auth-failed ×3).
- Dated leads: walk #15 ~10-20 (M3 claim-first), ai-1017 ~10-21, mlxfast
  ~10-24, nnethercote ~11-01.

NOTHING INVENTED — every quote read at the pin this session.
