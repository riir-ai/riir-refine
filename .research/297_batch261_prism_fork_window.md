# Research 297 — Batch 261: the PrismML fork prism-b10787 window

**Status:** DISTILLED — landed as Batch 261 (Plan 265), three own rules A/D/B, corpus 920 → 923.

## Source

PrismML-Eng/llama.cpp (MIT), the fork's prism branch: window `1180918489..73bfe6cc4`, +4 commits, the first fork movement since the B252-era pin. **The tip carries a NEW SCORED TAG `prism-b10787-73bfe6c`** — discovered at clone time (the claim row's ls-remote grep `prism-b[0-9]+$` missed it because the tagging convention now appends `-<sha>`; corrected in plan 265 + the snapshot landing row). Per the 10-19 intake row's rule, the scored tag carrying #369 FIRES the decode re-pin re-cut trigger — the bout (fork-vs-ours decode/pp at the new pin tuple) is the riir-ai league lane, gated on a verified-idle 4090 (StartLux resident to 13:30Z 10-20); it is NOT discharged by this batch.

## The three converted laws

1. **`prefill-batch-pool-f16-conversion-sized-by-actual-length`** (A, `25f68a4` #330): `GGML_CUDA_FA_PREFILL_F16=N` — prefill-sized batches (Q->ne[1] > 8) over a quantized K/V cache of ≤ N cells convert K/V to f16 in POOL memory sized by the ACTUAL KV length and run the f16 MMA kernel; decode keeps the in-place quantized read. The route economics: the boundary predicate is the AMORTIZATION economics (>8 queries pays one dequant pass across the batch); the copies live in the graph pool, NEVER a load-time reserve ("the reserved f16 copy would be n_ctx cells" — steady-state zero cost is what makes the lever free when the shape never fires); the accounting consequence (get_alloc_size treats pool-backed as "nothing to reserve beyond dst"); and the in-place kernel's supported-predicate REFUSES when the route armed (disjoint by the same query-count conjunct — the two routes can never both hold). Off by default. Our relevance: the PREFILL lane is our highest-priority axis; the lever is the conversion-economics mirror of our own prefill toggles.
2. **`dtype-branch-dispatch-parity-in-place-precondition`** (D, `ec1ee3a` #307): the quantized branch of the Ada kernel-choice ladder lacked the F16 branch's GQA>4 ratio rule — "the vector kernel was picked for 1 and 2 queries at every context length" — 1.21–2.95× at long context on THE LEAGUE MODEL (Ternary Bonsai 2 27B PTQ1_0, GQA 6, q4_0 K/V, head 256, RTX 4070: 44.8→54.2 @32k, 25.3→41.2 @120k, mtp 53.3→84.0 / 23.2→68.5; range computed from the commit's own rows — the slice agent's honest correction of the task text's 2.71×). The law has TWO halves: sibling branches of one dispatcher carry PARITY on measured admission rules (the burden is a measured reason for divergence), and the widening carries the IN-PLACE PRECONDITION (`ggml_cuda_fattn_mma_kv_native_supported` — q4_0/q8_0, head 128/256, K-type==V-type) — "Otherwise it converts the whole cache to f16 on every call, so keep the vector kernel." The measure-only switches were removed in-PR (measure-then-land-clean). Our relevance: OUR BOX IS ADA; the league model's GQA is in the widened band.
3. **`one-warp-ordered-reduce-row-batched-no-smem-barrier`** (B, `483bdab` #368): the Pascal PQ2_0 mat-vec re-shapes 4 warps + shared memory + a block barrier to ONE warp doing whole rows (R=4 batched), "bit-identical to the generic mmvq there" — licensed by REDUCTION-ORDER PRESERVATION (the chunk assignment lane + 32*w + 128*i and the ((v0+v1)+v2)+v3 fold + butterfly are the generic's own). Secondary decision contents: the algebraic decode identity sum(q*(c-1)) = sum(q*c) − sum(q); the clamped-compute-discard tail ("clamped rows are computed but not stored"); 2-byte-aligned block loads as composed u16 pairs; the arch-BAND admission (≥PASCAL <VOLTA, ne11 ≤ 4, exclusions). **The B180/B217 consumption disclosure**: `deferred-bias-ternary-dp4a-exact-isum` (entries_gpu_49) owns the exact-integer bias-cancel algebra, and its B217 Ada-twin rider ALREADY quotes the same identity + tail clamp + u16 composition (the fork's own #306, converted at B217) — the identity/tail/alignment atoms are CONSUMED COMPONENTS here; the novel atom is the warp-topology deletion under the order-preservation license. Pascal is not our lane; the law is the license.

Watch rows (no corpus): `73bfe6c` #321 server `--reasoning-effort-allow/-fallback` + `--reasoning-max-tokens-floor` — Bonsai-2 template ops intel (their template raises on "high"; thinking-model token floors), NULL for the corpus.

## Method

2 parallel authoring subagents over disjoint files (entries_gpu_113 ×2 rules, entries_gpu_114 ×1 rule), coordinator wiring + gates. All quotes verbatim at the pin (Slice B re-verified 25 fragments programmatically against the pin blob; Slice A verified 20 by fixed-string grep). Fixtures validated standalone: rustc --test --edition 2021, 6 fixtures / 22 tests, 0 failures, before landing.

## Gates (all run this session)

- batch261 needle gate FIRST-RUN PASS (within kernel_opt lib)
- kernel_opt lib **3190/0** (was 3187 + 3 new + the 10 transitory count-pin failures repaired mid-flight — the first run caught 6 unregistered count pins my multiline regex missed, the inline-assert form; repaired, second run caught the KERNEL_CATEGORIES slice registration missing, repaired, third run green — the B246 "pin-set-wider" lesson repeating)
- default lib 2715/0; clippy default + no-default postures 0 code warnings
- retrieval floors HELD 21/21 no re-pin (283.4 s; blended 90.6/94.2)
- freeze/score-bench NOT owed (kernel_opt-only, P30/B250); binary rebuild NOT owed (kernel_opt outside the release set)
- ZERO-GPU throughout (StartLux resident to 13:30Z)

## En-route notes

- The CRLF hazard did NOT fire (both new files LF; the corpus files' line endings checked before splicing — the B259/B260 discipline held).
- The slice A commit subject carries mojibake (`โ€”`, the em-dash mangled through the subagent's git invocation) — cosmetic on a pushed commit, left as-is; the record lives here + in the snapshot row.
- The count-pin edit went through a python multiline-regex pass that initially produced 11 adjacent-string-literal defects (the note inserted as a second literal instead of a prepend) — caught by reading the file, repaired in the same session before any gate ran on it. Lesson: a mechanical multi-site pin edit gets ONE immediate `cargo test` on the touched file before the next edit batch.
