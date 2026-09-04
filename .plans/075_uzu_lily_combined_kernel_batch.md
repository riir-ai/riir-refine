# Plan 075 — Batch 105: uzu + lily combined kernel batch (Research 127 + 128)

**Status:** LANDED 2026-09-05 — 11 rules, corpus 466 → 477 (A +1, C +1, D +3, E +1, F +1, H +3, M +1); all gates green (batch105 3/3, kernel_heal 1338/0/1i, default 1076/0, retrieval floors HELD, router 23/23, count pins walked). T7 (the GOAT-lead M3 A/B fixture) deferred with reason — GPU lane claimed by the 851 Phase A run at landing; re-arms when quiet. En-route correction: three rules were initially filed into the wrong KERNEL_CATEGORIES lists (the count pin caught it — Compute +1 / L +3 instead of A +1 / H +3); fixed same-session before commit, the pin doing exactly its job.

## Decision: COMBINE (the mine-time mandate resolved)

Research 127 (uzu, trymirai, @ `115609b`) and Research 128 (lily, perplexityai/pplx-garden
`lily/`, @ `1ed972ed`) mandate "combine 105+106 or sequence strictly." **Combined** — both are
Apple-Silicon Metal engines for the SAME model family (qwen3.x GDN hybrid), both MLX-derived,
verdicted 3 days apart, with three likely cross-source collision families:

1. uzu `indirect-device-dispatch-tile-map-moe` (A−) vs lily tile-map builder (inside A− #1
   `gpu-resident-moe-routing-no-host-sync`) — same mechanism family (device-computed ragged-MoE
   launch geometry), different sync-boundary postures.
2. uzu `lane-sequenced-deterministic-bucket-scatter` (B, atomic-FREE 32-step lockstep) vs lily
   atomic scatter (`moe_scatter_slots`) — same scatter, opposite determinism postures.
3. uzu `single-threadgroup-fused-count-scan` (C+) vs lily `moe_histogram`→`moe_scan_offsets`
   pipeline — same MoE prologue family.

The B103 precedent (MTPLX + MLX NAX combined: one corpus cycle, one GOAT run, one count-pin
walk) executes. The combined batch is **Batch 105**; the "Batch 106" slot merges into it
(queue snapshot updated at landing; the NEXT batch after this one takes 106).

## Sources (pins — re-clone at mine time, re-verify ALL quotes)

- uzu: `trymirai/uzu` @ `115609b` (Release 0.5.23). Unread-at-verdict surfaces to read this
  batch: `metal/sparse/**`, `speculators/**`, `weaver/**` kernels, `sampling/**`, qmv table
  remainder (~300 rows, lanes read M1/M2/M2Pro/M5Max only), `engine/` session orchestration,
  `encodable_block/**`.
- lily: `perplexityai/pplx-garden` @ `1ed972ed3f0bd5616c997c9507c25616c63394fc`, `lily/` ONLY
  (the RDMA surface is verdicted NO — never re-scope the garden wholesale). Unread surfaces:
  `src/model.rs` prefill-half (chunking cadence), `src/kernels/metal/attention.metal` bodies
  (978 L), `src/weights.rs`, `docs/2026-09-02-performance-mlx-0.32.2.md`, `tests/`.
- Repo-shape trap (R128): the zread index was STALE pre-#19 — verify repo shape against the
  commit graph, never a cached structure fetch.

## Candidate pool (post-verdict; cross-source dedup decides the final set)

- uzu: `indirect-device-dispatch-tile-map-moe` (A−), `cooperative-tensor-mma-capability-route`
  (B+), `packed-nibble-tensor-operand-alignment-gated` (B), `rht-epilogue-fusion-contract`
  (B+), `target-tile-split-k-deterministic-two-dispatch` (B),
  `a8-activation-route-stability-guard` (B), `lane-sequenced-deterministic-bucket-scatter` (B),
  `integer-mma-epilogue-dequant-scale-residency` (B−), `single-threadgroup-fused-count-scan`
  (C+), `radix-select-ordered-key-topk` (B−).
- lily: `gpu-resident-moe-routing-no-host-sync` (A−),
  `concurrent-dispatch-dependency-level-barriers` (A−, the GOAT lead — M3-benchable),
  `pipelined-token-slot-ping-pong-submit-ahead` (B+),
  `split-k-two-pass-attention-decode-long-context` (B+).
- Riders: R127 §Riders 1-8 (B49 ×2 + Metal-4.0 pragma fact, B97 frontier-clamp instance,
  B103/B100 GQA-M-packing + route ladder, B104 occupancy/qmv tables + unanimous-family-
  consensus fallback, B98 GEMM-over-GEMV carve-outs, LinearAttn state-resident-chunked,
  Morton counter-regime note) + R128 riders (B16 dequant-in-tensor-op +77.4%, B77
  tile-by-expert-load axis, B100 GQA table row THIRD implementation, B95 GDN scan, B62
  register-A skinny router).

## Tasks

- [x] T0: re-clone both sources at pins into `../vendor/` (read-only lanes); verify repo
      shapes against commit graphs (the stale-zread trap). — uzu @ 115609b6, pplx-garden @ 1ed972ed.
- [x] T1: uzu inspection lane (subagent, read-only) — 10/10 candidates verified (2 with
      corrections: strict-layout override arm; the Hadamard-of-a-constant-column quote killed),
      8/8 riders confirmed, 5 NEW candidates from the unread surfaces (sparse-paged-kv, gumbel
      sampling, hazard-tracked-aliasing — landed, multi-field argmax, flat-topology) + 1 upstream
      hazard (counts_shared OOB for 128 < E ≤ 512).
- [x] T2: lily inspection lane (subagent, read-only) — 4/4 candidates confirmed (A1 prefill-only
      scoping; A4 exact 128×256 crossover), riders verified with 2 corrections (C1 tile policy is
      chunk-size-routed NOT expert-load — FALSIFIED; C5 refusal text names the requirement),
      10 new §B candidates (B2/B4 folded into rules; B7 = B63 external instance; rest deferred).
- [x] T3: cross-source dedup gate — all 11 rules PASS against the corpus AND across sources;
      the 3 named collision families resolved as in-rule cross-refs (tile-map vs capacity-sentinel
      = opposite postures both landed; scatter determinism = contrast carried; count-scan deferred).
- [x] T4: author rules — 11 rules in entries_gpu_18.rs (original fixtures, parse-gated; no
      killed quotes; corrections carried).
- [x] T5: GOAT — batch105 3/3 (present/concepts/parse), kernel_heal lib 1338/0/1i, default lib
      1076/0, retrieval floors HELD, router 23/23 at 477, count pins walked (tests.rs totals +
      batch-list + expected-array + sums + grouped; router.rs; mod.rs member lists + 7 category
      comments). Clippy both states = the one gate the fd-starved terminal blocked at close
      (run before push).
- [x] T6: docs — queue snapshot (LANDED note + deferred-candidate ledger), R127/R128 status
      lines, AGENTS.md entry, this plan.
- [-] T7 (DEFERRED — GPU lane claimed): lily GOAT-lead bench fixture
      (`concurrent-dispatch-dependency-level-barriers` A/B in riir-gpu's raw-Metal lane:
      N independent kernels + one dependent consumer, serial pass vs
      `MTLDispatchType::Concurrent` + level barriers, wall-clock + FNV-identical outputs).
      The sibling `rank851` run owns the GPU now (851 Phase A long cells); a concurrent
      Metal bench starves its wait-quiet window and pollutes both measurements (Bench 649).
      Re-arms when 851 Phase A lands + GPU-quiet. NOT a landing blocker — the rule lands
      corpus-side per the B103-NAX capability-rule posture; the bench upgrades its verdict
      class when run.

## Honest caveats carried into the batch

- uzu + lily carry NO committed perf numbers in-tree (uzu) / M5-blog-only numbers (lily) —
  cite provenance per rule; never present blog numbers as measured on our box.
- MXU/cooperative-tensor material is M5+/Gen18+ — capability-rule posture (B103 NAX
  precedent), zero pinned-box effect, BUT lily's MPP-emulation addendum (R128 §9) makes the
  orchestration leads experimentally reachable on M3.
- Lily kernel BODIES are MLX/MLX-LM-adapted (MIT, NOTICE-disclosed) — fixtures stay ORIGINAL;
  cite Lily for orchestration shapes, MLX for adopted constructions.
- Corpus counts move — re-scan at authoring (466 expected post-B104, verify).

## GOAT checklist (landing bar)

- [x] New gates green (present/parses/concepts per batch file) — 3/3 FIRST RUN
      (one concepts needle caught pre-landing; gate doing its job).
- [x] kernel_heal lib + default lib green, count-identical pre/post except the new
      gates — kernel_heal 1338/0/1i, default 1076/0.
- [x] Retrieval floors HELD (no re-pins) — retrieval_precision_regression_gate PASS.
- [x] clippy -D warnings clean, both states — landed clean; the residual clippy
      batch105_concepts_covered `too_many_lines` discharged post-close 2026-09-04 by
      the resume session (the batch98 `#[allow]` precedent).
- [x] Count pins walked (every site) — tests.rs total/batch-list/expected-array x7
      categories + sums x2 + grouped x2; router.rs; mod.rs doc + 7 member lists + 7
      category comments.
- [x] Queue + AGENTS.md + research statuses + highwater committed in the landing
      commit.
