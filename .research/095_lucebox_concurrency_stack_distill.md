# Research 095: lucebox PRs #642/#651/#659/#668 — concurrency stack distill verdict (YES A−; Plan 548 P1-2 T1)

**Status:** RECORD — Batch 81 LANDED 2026-08-27 (Plan 039; 6 rules + the B77 rider, corpus 387→393). The T2 port list below remains the Plan 548 concurrency-lane decision (ports are GPU-gated, NOT landed).

**Source:** [Luce-Org/lucebox](https://github.com/Luce-Org/lucebox) PRs —
[#642](https://github.com/Luce-Org/lucebox/pull/642) (concurrent fixed-chain GDN transition journal; **superseded by #651**, do not port separately) ·
[#651](https://github.com/Luce-Org/lucebox/pull/651) "batch draft projections across lanes" (full diff read at verdict time) ·
[#659](https://github.com/Luce-Org/lucebox/pull/659) "various DFlash2 optimizations concurrency C1-C5" (page + full measured tables read; 6 mechanisms) ·
[#668](https://github.com/Luce-Org/lucebox/pull/668) "trim legacy device pools between requests" (page read). C++/HIP on vendored ggml, R9700/gfx1201, Apache-2.0 lineage. Companion Plan: [Plan 548 P1-2 T1](../../riir-ai/.plans/548_beat_llamacpp_vllm_perf_closure.md).

## Verdict

**YES, A− — dual-purpose:** (1) the T2 port list for Plan 548's concurrency lane;
(2) a queued external kernel_opt batch (the Batch 77/79 live intake path — the
riir-gpu file-mining frontier is CLOSED). Route `kernel_opt`, NOT rust_perf
(GPU kernels + host orchestration around them).

## Ranked candidates (mechanism → corpus grade)

| # | Mechanism (source) | Grade | Notes |
|---|---|---|---|
| A | **Packed-tree verify over paged KV via virtual-tail partitioning** (#651 `paged-attn.cu`): tree candidates = a virtual tail of `tree_width` tokens; prefix rows resolve via block table, candidate rows via `tree_scratch_base + slot*stride + node` gated by an ancestor-visibility walk (`paged_attn_tree_visible`); siblings/padding resolve to no physical row. ONE stable softmax over prefix+tree. | **A** (GOAT) | The enabling mechanism for concurrent spec-decode. Novel to corpus (zero hits virtual-tail/replay-log). Port T2-1. |
| B | **GDN transition replay-log + commit-many** (#651 `gdn-replay-log.cu`, evolved from #642): the forward kernel journals `[g \| k \| delta]` per (token,head) into a compact row appended to the result tensor (`capture_replay_log` — the journal view OWNS storage so gallocr can free the big GDN result); after posterior selection one kernel replays accepted prefixes onto per-slot state in token order (`fmaf(key, delta, gate*current)`), + conv-window commit, no re-forward. | **A** | Kills the second target forward for accepted tokens. Our Issue-721 `commit_tree_verify_fast` is the rank-1 special case from tree rows; this is the generalized journal form. Port T2-2. |
| C | smem tiling of hot replay transition loads (#659 mech 3; state 128 × width 257 × ≤8 tokens) — **−37.6/−39.1/−37.7/−34.6% state-promotion p50 @C1-C4, zero launch-count change**. | B → **rider on B35** | Positive instance of `smem-staging-gated-by-cross-thread-reuse` (transitions re-read once per state element → reuse ≫ 1). Not a new rule; fold the measured regime into the port of B. |
| D | Cross-lane projection batching/packing (#651/#659 mech 1-2): concat lane columns before shared wq/wk/wv/gate/wo/w_gate/w_up/w_down + append-projection matmuls — draft-compute p50 **−26.9/−34.6/−45.9% @C2/C3** (per Plan 548's #651 record), append-p50 −8.3/−12.6/−14.4%, −22 projection dispatches per added lane. | **B+** | The batching-axis decision (pack columns, keep conv/RoPE/masks/attention lane-local — they are not weight-sharing-shaped). Adjacent to the B58/B63/B77 same-input-merge family but distinct (lane-packing vs same-input). Port T2-3. |
| E | Exact-bucket graph specialization (#659 mech 5): width-5 draft graph at C5 while target keeps bucket-6 "to preserve MMQ numerics" — goodput +1.33% bracket-mean, −125 dispatches/round. | B− | Graph-level analog of `batch-width-routed-dual-tile-twins` (sweep, never interpolate) + the dual-shape split (draft exact, target padded for numerics). Candidate at mine time; SKIP as standalone port unless we bucket identically. |
| F | **Stable target-graph retention keyed by owner/backend/topology identity** (#659 mech 6): `target_graph_build` p50 **−99.98%**, goodput +0.27..+0.99% @C1-C5; metadata arena kept at stable addresses so the ggml-cuda CUDA-graph topology cache key (nodes[0]) stays valid across rounds. | **A−** | Host-side; complements B25 `hot-swap-weights-preserve-autotune-caches` (that preserves caches across weight swaps; this preserves GRAPHS across serving rounds via identity keys + arena stability). Port T2-4. |
| G | Staged append/rollback with LIFO retry-blocks (#651 `paged_kv_pool`): tokens stage into `staged_tokens`; rollback pushes freed blocks onto a per-sequence `retry_blocks` LIFO so the retry append reproduces the SAME physical rows (captured graphs + block-table deltas stay valid). | **B+** | Correctness companion REQUIRED with A under CUDA graphs (retry-stability is the decision content). |
| H | Fail-closed multi-resource commit transaction (#651 `tree_commit_transaction`): VALIDATE every destination (incl. device-pointer + duplicate-slot checks) → COMMIT_PREVALIDATED; after the first mutation no recoverable error may escape (`GGML_ABORT`) — fallback from a mixed pre/post-commit state is impossible. | B | Transactional two-phase shape; the rollback-write-vs-destroy split decision. |
| I | Per-eval memo release at request-scratch release + inter-request pool trim (#668): Q8 memo freed before trimming legacy pools; **2.2-2.4 GiB returned**, OOM-class fix on non-VMM ROCm (pool retained operator temporaries indefinitely). | B | The RELEASE half of `shared-activation-quantize-memo-per-eval` (B77) — memo lifetime must pair with the REQUEST lifecycle, not the process. Confirms Plan 548 P3-2 as a rider; separate small port candidate for our serving path (our vendored sliced_pool ALLOC_AFTER_FREE hysteresis is the intra-run analog). |

**Drops:** async metadata upload (#659 rejected candidate — metadata_upload p50 −70..85% but the wait moved into target compute, goodput mixed −0.07..+0.46%; THEIR OWN rejection holds → rule annotation, never a port) · W16 widening (−8.3% @C6; we ship block-16, W8-stays confirmed) · #642 as a unit (fully superseded by #651's rename + packed drafter).

## Port verdict (the T2 decision Plan 548 P1-2 T1 owed)

- **PORT [GPU-GATE]**: A (packed-tree paged verify), B incl. C's tiling (replay-log
  commit — fold the smem staging in), D (cross-lane projection packing), F
  (stable graph retention), G+H ride along as A's correctness contract.
- **SKIP**: E standalone, W16, async-upload, #642.
- **SEPARATE SMALL CANDIDATE**: I (pool trim + memo release at request boundary).
- Port hazards from the review record (carry into the port plan): the P1
  stream-sync hazard (commit kernels must launch on the graph/backend stream or
  sync it first — they read replay logs written by the graph stream); SYCL-style
  dispatch predicates must reject replay-log nodes (backends without capture
  consume garbage); F16 codebook `get_rows` needs an F32 cast before readback;
  `chain_decode_bucket_width` >64-lane OOB.

## Yield estimate

2-4 corpus rules after ~50% coordinator falsification (A, B, F strongest; D/H
plausible; C/E/I riders on existing rules). GOAT candidate for the batch: **A**
(virtual-tail tree verify — benchable as a measured-GOAT distillation only if
we land the port; otherwise concept-grade with the source's C1-C5 tables as
evidence, the Batch 77 precedent).

## Honest caveats

- #651's description TABLES (draft-compute −26.9/−34.6/−45.9%, MMQ −41.7%) are
  taken from Plan 548's record; the #651 DIFF was fully read at verdict time.
  Re-verify the exact table cells at port/batch time.
- Source is C++/HIP on vendored ggml — patterns port, code does not (fixtures
  must be original minimal reproductions per the standing constraint).
- #659's Strix-Halo dynamic-conv timing was inconclusive (C1 control moved
  8.65%); no Strix claim for GDN/mech-5/6.
- All numbers are R9700/gfx1201 (their box); our 4090 re-gate is mandatory
  (G2 + verdict-stable ×2 per Plan 548's gates).
## Unread: the 28-commit #659 per-commit evolution (page-level verification
  only); the LuceForge PR #73 observation record (linked, not fetched).

---

## Batch 81 record (landed 2026-08-27, Plan 039)

6 rules — the full A/B/D/F/G/H set, ZERO dedup kills (Lane C adjudication:
the ~50% falsification load Research 095 estimated was pre-absorbed by the
C/E/I rider rulings at research time; every near-neighbor was read in full
before the verdict). Corpus 387 → 393 (A 91→93, D 79→82, G 21→22):
`virtual-tail-packed-tree-verify-over-paged-kv` (A),
`gdn-transition-replay-log-commit-many` (G),
`cross-lane-column-packed-projections` (D),
`identity-keyed-graph-retention-stable-arena` (D),
`staged-append-rollback-lifo-retry-blocks` (A),
`fail-closed-multi-resource-commit-transaction` (D).
Plus the P3-2 rider on B77 `shared-activation-quantize-memo-per-eval` (the
#668 REQUEST-boundary release half: memos popped LIFO before pool trim;
legacy non-VMM pools only).

**Provenance corrections folded at batch time (Lane B, diff-verified):**
(1) NO measured tables exist in the local diffs — every p50/goodput/GiB
figure is a PR-body number via Plan 548's record (the diffs contain no
commit messages; the "-22 dispatches" figure is not reproducible from the
op inventory). Rules cite mechanisms, never the unverifiable numbers.
(2) The F rule gained TWO halves the verdict summary missed: the kv axis of
the key is BUCKET-quantized to 256 (`((requested + 255) / 256) * 256` — the
mechanism that makes retention hit despite per-round kv growth) and the
arena SALT loop (bucket-proportional dummy tensors so two shapes sharing
the retained arena never alias in the address-keyed backend cache —
"retention and salting are two halves of one contract").
(3) The generation-counter invalidation belongs to the SELECTOR graph
cache, not the target graph (the target invalidates via identity-key
mismatch + park/unpark teardown).

**GOAT:** kernel_heal lib 929/0/1i (926 + 3 gates) · default lib 703/0 ·
all-features 1348/4/13i (the 4 = the KNOWN environmental l4_daemon
Windows-spawn failures, identical B79/B80 signature; count-consistent —
the parses gate is cfg-compiled-out under all-features) · clippy
`--all-targets` 0 lint warnings both default + kernel_heal states (the
exFAT hardlink cargo noise excepted; 1 doc_markdown backtick fix
pre-commit) · retrieval floors 14/14 HELD zero decay (kernel
self-variants 100/100/100, blended 93.9/96.9/97.7 — 10th consecutive
batch) · CI guard layers 1-4 + 5a + 5b-default + 5c PASS.

**Generator:** `target/gen_b81.py` (rustlit()-escaped, python-side needle
+ anchor assertions — the B51/B75 classes; one anchor bug caught pre-apply:
the slice-insert searched for the OLD marker after replacing it).
