# Research 333 — Batch 295: the b10826 open leads (GDN conv-chain pair + FWHT rows→threads + replay-time host-scan skip)

**Status:** RECORD — landed with Batch 295 (Plan 300). Corpus kernel_opt 977 → 981 (D 233→235, C 63→64, B 109→110; entries_gpu_131) + one count-neutral rider annotation (the Batch-274 authority-law reconciliation on identity-keyed-graph-retention-stable-arena, entries_gpu_08).

## Lineage

The staged B10809→B10826 fork-distill row (snapshot, staged 2026-10-26 ~23:xx +07 by the 4090 lane) listed five leads. At claim time (zed-amo-b295-leads, claim `b277e98f`, epoch 1793057491) this lane adjudicated the staged row against the LIVE corpus and found two leads stale-by-landing:

- **The #395/#401/#402 quartet** — already Batch 288's riders P84/P85 (`.plans/293`, landed ~13:2x +07 Oct 26, hours before the staged row was written). Excluded.
- **#391 e75baeeb24 (norm+Hadamard fusion at both norms)** — already B283's OWN rule `look-ahead-fusion-precomputed-skip-overlap-walk` (entries_gpu_128), whose Source line cites exactly `e75baeeb24 (#391)`. The staged row's dedup pointer (vs `rms-norm-registers-carry`) named the wrong anchor. Excluded.
- **#393 face (a), the batched sched input uploads** — already corpus as the Batch-283 rider on `chain-dispatches-single-sync-point` (entries_gpu_01), citing #393 directly; the staged row's "#30358 rider" pointer was misaddressed. Excluded (no new rider needed).

The batch is therefore the genuinely unowned remainder — four own rules from three commits, every quote re-verified at the pins (the amo vendor clone `/Users/katopz/git/vendor/prismml-llama.cpp`; two read subagents over disjoint commits + coordinator re-checks).

## The four rules

1. **D `gdn-conv-chain-fusion-gather-relocation-done-set`** (#383 `97ea5aeee1` + #398 `bf8c069fde`) — the GDN decode conv-chain graph fusion: GET_ROWS skipped at its position, its read RELOCATED into the CONCAT's launch, which computes gather+concat+CPY-write-back+ssm_conv+silu+L2 in one kernel behind an eight-conjunct pure-function matcher, with a two-phase registry (gathers + done-set) on the evaluation loop. The novel scheduling atoms vs the B283 look-ahead: BACKWARD gather relocation (a read moved into a later launch) + the FOLDED CONSUMER STORE (a downstream CPY into a whole row of the read's own cache), licensed by the relocation-window write-exclusion walk + row containment/alignment + the in-place read-before-write argument. Bit identity by per-element ordering replay (same `ssm_conv_f32` form incl. the absent-bias `0.0f`, same `l2_norm_f32` warp butterfly). Opt-in `GGML_CUDA_GDN_FUSION=1` → #398 flips demote-only (`=0` restores); H200 +5–10% 2B–27B, 9B +7.9% at DEFAULT; 90–240 fewer kernels/token; bit-exact incl. MTP. Scope disclosure: #399 (`820701ce3`) had already extended the fusion to verify widths (1–16 tokens) by #398's tree — disclosed, not owned by these pins.

2. **C `same-activation-mmvf-pair-channel-stride-broadcast`** (#383/#398, mmvf.cu) — two independent single-column mat-vecs over ONE activation as ONE launch of the SAME kernel: the second weight addressed as first + byte offset expressed as a CHANNEL STRIDE, the activation broadcast at stride 0. The admission: pointer identity + same shape/strides/dtype + single-column F32 outputs + the offset gate (nonzero, element-aligned, int-range) + precision agreement; the yield laws (hinted matmuls / COMPUTE-buffer weights → the deeper quantizing fusion; a pair feeding ADD/GLU stands aside). Exactly two products (no N-way evidence). Bit-identical by template identity. The same-input family's LAYOUT-FREE member — no stacking, no concat, launch arguments only.

3. **B `in-flight-transform-count-admits-block-thread-width`** (#396 `ef119d136e`) — the block's THREAD axis admitted by the grid's ROWS axis: ≤64 transforms in flight widen the one-block-per-transform FWHT kernels to min(n/2, 1024) filtered through the compiled template set {512, 1024}; the admission never invents a variant; the pair floor (≥2 elements/thread); bit-identical by stage-order invariance; ONE shared helper feeds both the transform kernel and the fused quantize writer; `GGML_CUDA_FWHT_NT` overrides, 256 disables. H200 9B +2.6–2.9% / MTP3 +2.5–3.0% / DFlash7 +4.3–4.9% / 27B MTP2 +4.0–4.4%. NOT in the scored tag `prism-b10819-909f0ca` — no our-box datum; league re-cell pending. Staged-row drift flagged: the pooled "+2.6–4.9% 9B" range merges DFlash7's endpoint into plain decode — quote per-row.

4. **D `graph-replay-skips-host-node-scans`** (#393 `63f16c408f` face b) — replay visits no nodes so its host node-scans are dead: the compat proof MEMOIZED on the uid token minted only inside `ggml_cuda_graph_update_required` after the scan passed (the minting-site audit IS the safety argument; sentinel 0 and mismatch route to the full scan — the memo's miss paths are the scan paths, never a refusal), plus the consumer-count map built only under the visiting postures (eager ∪ capture) behind a reader-audited gate. ~55–65 µs/decode-step (~130 on 27B) prices the WHOLE commit incl. face a — never per-gate.

## The authority-law reconciliation (count-neutral rider)

The Batch-274 rider on `identity-keyed-graph-retention-stable-arena` recorded "the hash is an index, the node-walk is the proof" — true of the pre-#21764 tree. The current fork tree mints `graph->uid` after a validated cycle and skips both walks on match (this batch's rule 4 extends the skip to the compatibility scan itself). The rider now carries the reconciled law: the proof is MEMOIZED, not skipped — a token substitutes for a walk only where the MINTING SITE is audited to run after the check it vouches for, on inputs the token's identity covers. The safety posture is unchanged in kind (a mismatch still re-runs the walk; update-in-place degrades, never wrong-replays).

## Gates (all run this batch, amo, /tmp/b295-gate target)

- batch295 needle 4/4 + parse 4/4 GREEN (two needle-string fixes en route: the quote's `/`-wrap artifacts and a possessive prefix — the needles now quote contiguous in-rule text).
- kernel_opt lib **3258/0** (3256 + 2); default lib **2749/0**; clippy `-D warnings` ×3 postures + `--all-targets` clean (one doc_markdown heal in-batch: the slice-header `entries_gpu_01` backtick).
- Retrieval floors HELD both postures: kernel+latent 22/0 (~207 s), default `retrieval_` 21/0 (~51 s) — no re-pin.
- Freeze NOT owed (zero clippy-corpus delta, P30); score-bench NOT owed (no rust_perf delta); release-set binary rebuild NOT owed (kernel_opt outside the release set — B286/B288 precedent).

## Count pins

The two total asserts + 33 bare value sites (perl `^        977,$` → `981,` — the B286 VALUE-tracks-live convention) + the router Plan-003 lineage prepend + the B/C/D KERNEL_CATEGORIES slices + per-category counts (B 110, C 64, D 235) + the Plan-003 expected table (the same three categories) + the two "Sum must equal" asserts + AGENTS.md ×2 + the `.docs/11_domains/kernel_opt.md` batch row.

## En-route defect (fixed pre-push)

The first `mod` registration edit hit the WRONG anchor — the file declares `mod entries_gpu_13;` (slice 13, no zero) between 129 and 130, and the fuzzy edit corrupted that line into a duplicate `entries_gpu_130`, leaving `entries_gpu_13` unresolved (E0433 + E0428 on the first gate run). Repaired by restoring the 13/130/131 sequence. Lesson: the corpus mod declares BOTH `entries_gpu_13` and `entries_gpu_130` — any future slice-13x registration must anchor on the unique `entries_gpu_131`-era lines, never on `entries_gpu_130` alone.

## Queue after

plan-294 resin (M3, claim to 08:28Z 10-28) — untouched; walk #18 (amo, due Tue 2026-10-28 00:00Z — another lane's claim); nnethercote ~11-01; the mining queue's next sec pool EMPTY (B294 discharged rustsec-T1; the b10826 kernel_opt pool now DISCHARGED by this batch — the next pool is the nnethercote window or new staged leads).
