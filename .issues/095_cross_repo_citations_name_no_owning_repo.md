# Issue 095 — cross-repo citations that name no OWNING repo (36 rows)

**Status:** OPEN — record filed 2026-09-12 by the katgpt-rs `citation_drift_sweep.py` corpus run (katgpt-rs Issue 751 / Issue 752). No prose in this repo was edited: the repair is this repo's call, row by row.

## The finding class

`36` citations in this repo's contract documents name a document number that is **not allocated in this repo**, **is** allocated in one or more sibling repos, and whose surrounding prose names **no repo that OWNS the number**.

The defect is **rebinding, not dangling.** A bare `Issue 750` is not merely unresolvable — it is a *relative* address into whichever repo the reader happens to be standing in. It reads correctly today only because this repo has not yet allocated that number. The day it does, the sentence silently starts pointing at a **different, locally-real document** — no link rots, no grep fails, no gate reds. That is the founding example behind katgpt-rs Issue 749; it is why a dangling-link checker cannot find this class.

## The repair

Name the owning repo in the prose:

```
- (Issue 750 T3)                 ← rebinds the day this repo allocates 750
+ (riir-ai Issue 750 T3)         ← addresses one document, forever
```

The qualifier must be the **owning** repo's directory name, within the citation's 3-line window (katgpt-rs Issue 752: a repo name only qualifies a citation if that repo actually **owns** the number — naming a non-owner is the MISATTRIBUTED sub-class below, and it is worse than naming nothing).

A **crate** name in the window does NOT qualify the citation. The workspace's crate→repo map is unique but lives in no document the reader has; measured, 4 of the 45 crate-name-bearing CROSS rows workspace-wide resolve to a repo that does **not** own the cited number. Crate hints ORDER the repair; they never excuse it.

## ⛔ Read every row before editing it — 16% measured false-positive rate

The pre-Issue-752 corpus (254 rows) carries a **measured 7/43 = 16% false-positive rate** from a stratified manual sample (katgpt-rs Issue 751 T1). The 45 rows *recovered* by Issue 752 (the owner-must-own tightening, corpus 254 → 291) carry **0/45** from a full census — those are the high-confidence half. Either way:

- **Do NOT bulk-repair.** Read each row's actual sentence first. Roughly one row in six of the older half is a sweep artefact (e.g. the number is qualified by prose the 3-line window cannot see, or the citation is to an external/foreign numbering).
- A row being listed here is **evidence to look**, not a verdict that the prose is wrong.
- Line numbers are as of the 2026-09-12 sweep run; a concurrent edit moves them.

## Sub-classes, highest priority first

| priority | sub-class | rows | why |
|---|---|---|---|
| 1 | `⛔MISATTRIBUTED` | 0 | the prose names a repo that **does not own** the number — a plausible address that is wrong, strictly worse than no address |
| 2 | `⛔MISLEADING` | 3 | the only crate name in the window maps to a repo that does not own the number — same failure mode, one indirection out |
| 3 | `crate-hint` | 17 | a crate in the window maps to a real owner — the repair is mechanical, but the reader still cannot perform it |
| 4 | (plain) | 16 | a bare number with no locator at all |

### ⛔ Priority 2 — MISLEADING crate hint

- **`HISTORY.md:756`** — `Issue 402` is owned by **riir-train**. ⛔MISLEADING: the only crate in the window is riir-ai, which does NOT own 402
  > Batch 39: 9 riir-gpu production-kernel rules (autotune-cache-extract-outlives-owner, append-cache-rewind-overwrite-in-place, slow-path-bootstraps-raw-buffer-hotswap-cache, entrypoint-variants-share-layer-body, elementwise-offload-needs-device-side-anchor, tiny-param-buffer-rebuild-over-cache, readback-destination-passing-free-fn, variant-owned-output-buffer-clone-escape, pipeline-layer-boundary-device-handle) from Research 043 — the twenty-eighth batch mining the stack's own kernels (kimi_k3_gpu_forward.rs 2,601 LOC — the KimiK3 full-model GPU decode forward: hybrid CPU/GPU stack, 7 forward entry-point variants, dual weight-update paths, MLA KV cache rewind for the Ouro loop, + the dense SiTU FFN helper family; the largest single unmined file): salvage the autotune cache across owner destruction via Arc inject/extract (re-bench ~5-10 min vs upload ~64 ms — the ownership-lifecycle complement to hot-swap), re-execution workloads OVERWRITE the append-only cache via rewind_to(pos) never re-append (O(max_seq) not O(iters × max_seq); recurrent-state layers are structural no-ops), the in-place weight-update two-tier bootstrap (first call runs the slow allocate path + harvests raw wgpu buffers into ORDER-keyed slots; subsequent calls queue.write_buffer in place — training weights change identity every step so pointer-keying is useless), versioned entry points multiply but the per-layer loop body stays shared per axis (forking the whole body guarantees divergence — observed TWICE in one file: a resident fork that initially called the OLD unbatched kernels + two saved forwards whose kernel-variant selection diverged under the same env vars), a GPU op sandwiched between upload+readback is compute-offload ONLY (the win requires a device-side anchor consumer — found INDEPENDENTLY by both mining agents), 3 f32 = 12 bytes rebuilds beat caching below a cache line, GPU readback helpers should be destination-passing free fns (dst: &mut [f32] not -> Vec<f32>; the free-fn form also splits the struct borrow — the API-level half of the Issue-689 churn fix), enum-variant-owned output buffers force per-layer-per-token clones out of the match arm (hoist to the parent + free fns over disjoint fields; merged from both agents — the free-fn extraction alone does NOT fix the clone), and the Handle IS the pipeline layer-boundary currency (host materialization becomes a layer-internal detail — later fusion composes without boundary changes). Corpus 294 → 303 (A 60, C 32, D 53, K 5). TWO rules merged from independent two-agent convergence (the anchor rule + the clone-escape rule). 13 dedup kills/falsifications (update_weights = B25 verbatim; OURO_* env reads = the B38 instance; inv_rms double-compute ×6 = B34 forward-side; the 4-buffer drain = B36+B26; the saved clones are checkpoint captures, NOT waste — B34 instance). Mining bug yield filed as riir-ai Issue 692 (14 hazards: the dead 640 MB embed_handle uploaded at construction + retained forever despite the Issue 402 comment proving it dead; the per-layer-per-token FFN clone/fresh-alloc storm (6+4 sites) across ALL variants; the two saved forwards' ~310-line divergence with LOST OURO env dispatch; contradictory sync-count docs 7→3 vs 3→1; gate+up duplicated GPU upload vs the doc's sharing claim; vocab logits double-copy ×4 sites; + 8 more). GOAT: 333 kernel_heal + 201 default tests, clippy 0 in all 4 states. The O() expressions joined the doc_markdown gotcha list (O(max_seq) needs backticks in //! prose).
- **`HISTORY.md:799`** — `Plan 548` is owned by **riir-ai**. ⛔MISLEADING: the only crate in the window is katgpt-rs, which does NOT own 548
  > Batch 93 (2026-08-31, Research 107 residue -- the ktransformers DISPOSITION batch, Plan 054; zero corpus delta, the B50 zero-yield precedent): the snapshot's "REMAINING QUEUED (candidates 13-14)" line drove this batch and the mine-time verification COLLAPSED it -- candidate 14 was ALREADY INSIDE Batch 92 (`quant-group-scale-fetch-gating` carries the cu:465-471 stripe-alignment precondition verbatim: "Ensure that the number of tiles in each stripe is a multiple of the groupsize"; the queue line was stale the day it was written) and candidate 13 (`k-block-half-of-l2`) has NO source anchor (grep-verified: the 1/2-L2 formula is absent from the repo; the only L2 content already landed as `stripe-sms-l2-semaphore-reduce`). The 8 rust_perf/host candidates all dispositioned with grounded reasons (full table in `.plans/054`): `grow-only-arena-pointer-rebind` = the B32a split-scratch/arena family (verified site `shared_mem_buffer.h` MemoryRequest/append_pointer); `intensity-gated-mat-vec-twins` = B77 `batch-width-routed-dual-tile-twins` covered (external validation); `fuse-independent-gemms-one-task-space` = the B58/B63/B77 same-input family; `allow-n-pending-sync` KILLED ON THE BAR with its real anchor recorded (`task_queue.h:29` + `cpp:53-61` -- parameter-not-rewrite, workload-dependent win = the metric-mismatch class, zero evidence in source; every landed concurrency rule carried a measured verdict); `host-callback-on-gpu-stream` ROUTED to Plan 548 intel; `dual-seed-page-hash` = dead scaffolding in source (`hash_check`/`check_hash_seed` declared, never consumed -- and our prefix caches are BLAKE3-committed, 64-bit collision checks moot at 256-bit); `batched-evict-count` = commented-out config check + the B30 count family; `recompute-ratio-guard` = no anchor; `decode-prefill-kernel-split` superseded by B92 device-M (mode-FREE kernel, the better half) + B77 twins. Lane CLOSED; standing lesson: a "remaining queued" line that outlives its batch breeds stale re-mining -- disposition or delete at landing time. Also en route: `.plans/.highwater` repaired (said 52 while plan 053 existed -- the missed-bump collision class). Record: Research 107 (status updated) / Plan 054.
- **`HISTORY.md:4676`** — `Issue 513` is owned by **riir-ai / riir-train**. ⛔MISLEADING: the only crate in the window is katgpt-rs/riir-dapps, which does NOT own 513
  > Issue 513 class, caught here in-session). The real lanes re-ran with the

## The full row list (36 CROSS rows)

`owner(s)` = every contract repo that has the number allocated. Where more than one owns it, the correct qualifier is whichever one the sentence actually means — which is exactly why the sweep cannot repair this automatically.

### `AGENTS.md` — 1 rows

| line | kind | № | owner(s) | sub-class | source line |
|---|---|---|---|---|---|
| 80 | Plan | 558 | katgpt-rs | crate-hint | Borrows the **drafter+pruner+corpus pattern** from `riir-games-quest/quest_grammar` but has **zero game-domain coupling**: no dep on `riir-games*` / ` … |

### `HISTORY.md` — 35 rows

| line | kind | № | owner(s) | sub-class | source line |
|---|---|---|---|---|---|
| 730 | Issue | 615 | riir-ai / riir-neuron-db | crate-hint | Batch 22: 5 riir-gpu production-kernel rules (dynamic-shared-memory-runtime-sized, ternary-bit-plane-u64-as-2x-u32, warp-shuffle-intrinsics-for-sub-wa … |
| 734 | Issue | 641 | riir-ai | crate-hint | Batch 24: 4 riir-gpu production-kernel rules (widen-index-multiply-to-64bit, dual-accumulator-single-reduction-sweep, dual-grad-same-pass, broadcast-u … |
| 736 | Issue | 424 | riir-ai / riir-train | crate-hint | Batch 25: 5 riir-gpu production-kernel rules (device-side-argmax-minimal-download, monotone-transform-elision-under-argmax, fan-out-shared-input-singl … |
| 756 | Issue | 402 | riir-train | MISLEADING | Batch 39: 9 riir-gpu production-kernel rules (autotune-cache-extract-outlives-owner, append-cache-rewind-overwrite-in-place, slow-path-bootstraps-raw- … |
| 760 | Plan | 436 | katgpt-rs / riir-ai | crate-hint | Batch 43: 5 riir-gpu production-kernel rules (mutable-input-scratch-residency-write-per-call, topk-select-nth-over-full-vocab-sort, post-scatter-renor … |
| 760 | Issue | 468 | riir-ai / riir-train | crate-hint | Batch 43: 5 riir-gpu production-kernel rules (mutable-input-scratch-residency-write-per-call, topk-select-nth-over-full-vocab-sort, post-scatter-renor … |
| 760 | Issue | 697 | katgpt-rs / riir-ai | crate-hint | Batch 43: 5 riir-gpu production-kernel rules (mutable-input-scratch-residency-write-per-call, topk-select-nth-over-full-vocab-sort, post-scatter-renor … |
| 761 | Issue | 468 | riir-ai / riir-train | crate-hint | Batch 44: 9 riir-gpu production-kernel rules (row-dot-split-inner-axis-block-reduce, fuse-indirect-gather-into-dot-consumer, share-cube-size-const-bet … |
| 761 | Issue | 698 | katgpt-rs / riir-ai | crate-hint | Batch 44: 9 riir-gpu production-kernel rules (row-dot-split-inner-axis-block-reduce, fuse-indirect-gather-into-dot-consumer, share-cube-size-const-bet … |
| 762 | Issue | 699 | katgpt-rs / riir-ai | crate-hint | Batch 45: 3 riir-gpu production-kernel rules (scratch-depth-matches-shape-invariance, softmax-denominator-hoist-out-of-weighted-v, frozen-cooperand-ho … |
| 763 | Issue | 700 | katgpt-rs / riir-ai | crate-hint | Batch 46: 3 riir-gpu production-kernel rules (size-gate-covers-every-cost-factor, declared-max-array-taxed-per-thread-under-runtime-bound, shared-oper … |
| 764 | Issue | 701 | katgpt-rs / riir-ai | crate-hint | Batch 47: 3 riir-gpu production-kernel rules (per-key-projection-hoist-out-of-query-sweep, multi-head-out-proj-consumes-head-concat, shared-producer-p … |
| 799 | Plan | 548 | riir-ai | MISLEADING | Batch 93 (2026-08-31, Research 107 residue -- the ktransformers DISPOSITION batch, Plan 054; zero corpus delta, the B50 zero-yield precedent): the sna … |
| 810 | Bench | 805 | riir-ai | plain | Batch 103 (2026-09-04, Research 124 / Plan 072 -- the MTPLX PR #391 + MLX NAX combined host-orchestration kernel batch; the Plan-072 NEXT-UP unit unbl … |
| 810 | Issue | 734 | katgpt-rs / riir-ai | plain | Batch 103 (2026-09-04, Research 124 / Plan 072 -- the MTPLX PR #391 + MLX NAX combined host-orchestration kernel batch; the Plan-072 NEXT-UP unit unbl … |
| 810 | Bench | 641 | riir-ai | plain | Batch 103 (2026-09-04, Research 124 / Plan 072 -- the MTPLX PR #391 + MLX NAX combined host-orchestration kernel batch; the Plan-072 NEXT-UP unit unbl … |
| 928 | Research | 484 | katgpt-rs | plain | cross-domain switch-cost measurement + `RERANK_POOL` 8→12.** Research 484's |
| 966 | Issue | 701 | katgpt-rs / riir-ai | plain | unadjudicable). Pair inventory 1 → **3 validated** (B47×Issue 701 H-C, |
| 967 | Issue | 686 | katgpt-rs / riir-ai | plain | B35×Issue 686 H7, B44-body×Issue 698 A-H1); Phase 5 reopen trigger honestly |
| 967 | Issue | 698 | katgpt-rs / riir-ai | plain | B35×Issue 686 H7, B44-body×Issue 698 A-H1); Phase 5 reopen trigger honestly |
| 1277 | Issue | 669 | katgpt-rs / riir-ai | plain | **Bug found at baseline (Issue 669, fixed same day):** |
| 1297 | Issue | 669 | katgpt-rs / riir-ai | plain | failure is pre-existing (or fix it, as Issue 669 did). **fmt note:** several |
| 1313 | Plan | 339 | katgpt-rs / riir-ai / riir-train | plain | Sibling Plan 339/340 files excluded (kimi_k3_train.rs 162 edits, |
| 1627 | Plan | 339 | katgpt-rs / riir-ai / riir-train | crate-hint | AVOID (real-model deps), bench_401 sibling-caution (Plan 339/340 territory). |
| 1791 | Issue | 701 | katgpt-rs / riir-ai | plain | test gate was 7/17-red at HEAD** (Issue 701 H-K hard error on the toy |
| 1907 | Issue | 603 | riir-ai / riir-neuron-db | plain | reliable detector. A sibling commit (`7d19917`, Issue 603) landed on the |
| 1950 | Issue | 665 | katgpt-rs / riir-ai | plain | **Parallel-session collision on the SAME unit (Issue 665 class, yielded |
| 2848 | Issue | 665 | katgpt-rs / riir-ai | plain | **Parallel-session collision on the SAME unit (Issue 665 class, third |
| 3712 | Plan | 336 | katgpt-rs / riir-ai / riir-train | plain | Plan 336's 0/60 — structurally capped by the 96.6%-unmergeable v2edit |
| 3840 | Research | 527 | katgpt-rs | plain | **Issue 062 (TPR × healer corpus fusion, Research 527 F5) CLOSED-NEGATIVE + |
| 4315 | Plan | 558 | katgpt-rs | crate-hint | `katgpt_core::variable_rank_domain_expert::pick_domain<15, 8>` (Plan 558). |
| 4344 | Plan | 558 | katgpt-rs | crate-hint | - `kernel_heal` (opt-in, Plan 001 Phase 3): GPU kernel optimization healing domain. Absorbs Proposal 012. Enables `katgpt-core/variable_rank_domain_ex … |
| 4346 | Bench | 572 | katgpt-rs / riir-ai / riir-train | crate-hint | - `ruliology_search` (opt-in): L3 Pareto-front candidate search. Reuses the algorithm pattern from `katgpt-ruliology` (Bench 572) without a code-level … |
| 4418 | Plan | 558 | katgpt-rs | crate-hint | Plan 558). Two opt-in feature-gated arms consume non-game sibling |
| 4676 | Issue | 513 | riir-ai / riir-train | MISLEADING | Issue 513 class, caught here in-session). The real lanes re-ran with the |

## What is NOT in this list

- **AMBIGUOUS (314 rows)** — the number exists BOTH locally and in a sibling. Undecidable from the number alone; deliberately not gated (a ceiling would red on every perfectly correct citation to a new local number a sibling happens to share).
- **IN-LOCAL-RANGE (9 rows)** — the number is below this repo's own highwater for that kind, so the citation is probably local and correct (a removed-file number, per the noise-reduction rule). This bucket is **UNDECIDED** and is never folded into either neighbour.
- **ORPHAN (0 rows)** — no repo in the workspace owns the number.

Corpus for this repo: **855 citations** across **2 contract documents** (`AGENTS.md`, `HISTORY.md`).

## Reproduce

```bash
../katgpt-rs/scripts/citation_drift_sweep.py        # all 19 contract repos
../katgpt-rs/scripts/citation_drift_sweep.py .      # this repo only
```

Workspace standing at filing (2026-09-12, a dated SNAPSHOT and not a checksum — five-plus concurrent sessions edit these documents and the walk moved by 3 between the repair's first and last run): **291 CROSS · 54 IN-LOCAL-RANGE · 0 ORPHAN · 908 AMBIGUOUS** over 2,948 citations in 35 documents across 19 contract repos; of the 291, 7 are MISATTRIBUTED and 22 MISLEADING. Upstream records: katgpt-rs `.issues/749` (the rebinding class), `.issues/751` (the sweep + its error rate), `.issues/752` (the owner-must-own tightening, 254 -> 291, commit `60bc76aa`).

## Tasks

- [ ] T1 — read and repair the MISATTRIBUTED rows (priority 1)
- [ ] T2 — read and repair the MISLEADING rows (priority 2)
- [ ] T3 — read and repair the crate-hint + plain rows; record any row judged a sweep false positive HERE, with the reason (that is the data that lowers the 16%)
- [ ] T4 — re-run the sweep; this repo's CROSS count should fall to the FP residue, and the residue must be enumerated, not assumed

