# 308 — Distill: dphnAI/sonar PR #1788 commit `927f326` (paired PCIe all-reduce, SM120 TP4)

**Status:** DISTILLED — pending owner decision (nominal Batch 272 / plan 276; corpus edits deferred to the next mining slot)

## Source

- **Repo/PR:** dphnAI/sonar (the Aphrodite Engine — vLLM-family inference server, formerly Aphrodite) PR #1788 "perf(sm120): optimize DeepSeek-V4.1 inference", OPEN, 4 commits, branch `perf/dsv41-4x-6000pro`.
- **Pointed commit:** `927f3262f87936e518b602538f2fe24a25359675` — "perf(sm120): add paired PCIe all-reduce" (this verdict's unit; the sibling commits `7c1bd39`/`9829c99`/`f653ba4` are out of scope).
- **License:** Apache-2.0 (SPDX headers on every new file).
- **Inspection date:** 2026-10-23 (zed-m3, user-directed @distill).
- **Method:** the full commit `.diff` fetched and read in full — the 879-line CUDA kernel `csrc/pcie_pull_all_reduce.cu` + the 549-line Python communicator `pcie_pull_all_reduce.py` + the communicator/runner wiring (`cuda_communicator.py`, `parallel_state.py`, `model_runner.py`, `async_utils.py`, `gpu_worker.py`, docs). **No `.raw` clone** — the diff carried the whole unit. **PR-stage caveat:** the commit is unmerged; a mining batch must re-pin against `git fetch origin pull/1788/head` and byte-re-verify every quote at the head sha (the B193 freshness law; the tokenmaxxing branch precedent).

## What the commit is

A from-scratch **single-lane, fixed-order TP4 paired all-reduce** over CUDA IPC P2P slabs, opt-in (`APHRODITE_ALLREDUCE_USE_PCIE_PULL=1`), SM120-only, for small decode-side tensors (BF16, contiguous 2D, 1–8 rows × {5120, 6144} cols, ≤ 49,152 elements). Source-measured (PR table, 4× RTX PRO 6000 Blackwell, DSpark K7 speculative decode, seven-run medians): **290.4 → 327.2 tok/s (+12.7%) vs NCCL** at identical acceptance (~59%) — the two DSpark rows differ ONLY in the all-reduce backend, so the delta is attributable to this commit's mechanism. Kernel anatomy: two XOR-partner exchange rounds (rank^1 then rank^2) + local rank-order fp32 combine rounded once to BF16; round-2 transport packs own+peer BF16 pairs into one 16-byte `cp.async.bulk` store (leader-issued TMA, double-buffered shared tiles, `fence.proxy.async.shared::cta` between generic writers and the async-proxy read); epoch-flag `Ready` records in parity banks (128-B aligned, `(epoch&1)*kRanks+sender`); explicit PTX ordering (`ld.relaxed.sys` spin → `ld.acquire.sys` confirm; `st.release.sys` publish; `fence.acq_rel.gpu/sys`; `__threadfence_system`); `%globaltimer` deadlines + `__nanosleep` backoff + fault-poll every 64 misses; single-CAS-winner fault record broadcast to every peer's fault slot; per-step `close_step` output gate (snapshot tensor copied on the output stream, checked before tokens release). Python anatomy: staged collective init (Gloo `monitored_barrier` + `all_gather_object` policy manifests, bit-exact cross-rank agreement), NVML topology qualification (P2P read/write caps + common-ancestor ≤ NODE), **fail-stop policy — a faulted workspace is pinned until process exit and NEVER falls back to NCCL**, IPC teardown as a collective (importer-close → barrier → dispose → free), graph-capture lifecycle (quiesce → bind capture stream → capture → restore), and a sticky-backend selector agreement gathered across ranks at communicator construction.

## Verdict

**YES B− corpus** (bottom of the band — sized as 2–3 rules + 3–4 riders, every rule NotBenchable-annotated) / **lane-intel N/A** / **product-coverage MODERATE — extends the distributed-collective family** (B119's 2 NCCL-device-API sync rules + B249 slice-3's 4 TP rules; this would add the corpus's first from-scratch custom-collective protocol rules).

## Domain routing

`kernel_opt` — CUDA kernel + host-orchestration-around-GPU collectives (the gpu_30 header's own framing: "the league and the workspace are single-GPU, so these rules serve USER code on multi-GPU"). Categories: D (serving correctness/admission), E (concurrency). **The tempting WRONG domain named and refused:** `rust_perf` (the Python orchestration half tempts host-side routing — but its decision content is GPU-collective policy, the B119/B249 precedent's home), and `clippy_lints` (nothing here is parse-provable).

## Lane mapping — N/A (stated, not skipped)

The league is single-GPU by design (4090 SM89 + M3 Metal; decode/prefill cells). There is no TP4 lane, no SM120 box, no multi-GPU serving lane anywhere in the workspace (the riir-infer disaggregated-handoff lane is prefill/decode state transfer, not TP collectives). The distributed family exists purely as product coverage — this verdict adds to it with **zero lane-intel output**, per the skill's own rule that a lane-intel zero must not suppress the corpus axis and vice versa.

## Product coverage

The corpus's distributed-collective family is small and this commit is its highest-quality custom-collective sighting to date (the fail-stop + agreement + gate design is production serving policy, not kernel trivia). **Follow-up source pool** (accumulation clause applies — pool reads before the batch window fold in): vllm-project/vllm `custom_all_reduce` (the canonical one-shot/two-shot custom AR, the most-deployed custom collective in inference — the family's primary unmined upstream), MSCCL++, torch `distributed._symmetric_memory`, TensorRT-LLM `custom_all_reduce`, ss16118/low-latency-nccl (mined B119).

## Ranked candidate table

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `faulted-collective-fail-stop-no-numerics-fallback` | `pcie_pull_all_reduce.py` (whole design), `cuda_communicator.py`, docs §"Experimental paired PCIe all-reduce" | **B−** | THE lead. Three decision contents: (a) two collectives with DIFFERENT rounding contracts must never hot-swap at runtime — the docs disclose "adds operands in rank order in FP32, then rounds once to BF16. This uses a different rounding contract from BF16 NCCL. Do not assume that the modes are numerically interchangeable" — a mid-flight fallback would convert a transport fault into silent numerical divergence; (b) selectors agreed BIT-EXACT across ranks at init (`all_gather_object` + refuse mismatch + refuse both-set) — "a mismatched environment can strand only some ranks inside the backend's collective initialization"; (c) failure is terminal and surfaced through the output gate ("the engine must be restarted before serving more requests") — never a silent retry. Category D. |
| 2 | `ipc-peer-mapping-collective-teardown-pin-on-failure` | `pcie_pull_all_reduce.py` `_release_resources`/`destroy`/`__del__` + `.cu` `close_ipc_handle` | **B−** | The teardown is a COLLECTIVE with a pinned order: every importer confirms `cudaIpcCloseMemHandle` BEFORE the owner disposes/frees ("Every owner retains its raw allocation until every importer has confirmed"); the barrier between close and free is load-bearing; ANY failure (rollback included) pins the whole session to process exit (`_UNRETIRED_WORKSPACES` — "A failed or uncoordinated shutdown must not return live IPC/state storage to an allocator. Only process exit may reclaim"); **destructors never issue collectives, frees, or peer-mapping closes** (the `__del__` doc). Category E. Broad Rust surface: any shared resource handed across process boundaries. |
| 3 | `relaxed-spin-acquire-confirm-metadata-flag-wait` | `.cu` `wait_ready` | **B−/C+** | The wait protocol: spin `ld.relaxed.sys` on the epoch; on apparent match re-read the SAME word with `ld.acquire.sys` (closing the torn-read race) then verify count/kind with relaxed loads (the acquire orders them); mismatch = `kMetadataMismatch` fault; future epoch = `kFutureEpoch` (protocol desync, diagnosed not spun on); `%globaltimer` deadline + `__nanosleep(64)` backoff; fault-poll interleaved every 64 misses. Distinct from B92's single-tier acquire spin and B119's epoch-band flag formats — THIS owns the wait ORDERING. Category E. The weakest of the three (the double-check discipline is textbook); grade settles at authoring. |
| R1 | Parity-bank double-buffered flag slots (`(epoch&1)*kRanks+sender`) | `.cu` `ready()`/`paired_ready()` | rider | Positive instance of B119's epoch+multi-buffering ABA defense → face on `ll-flag-in-data-packed-poll` (count-neutral). |
| R2 | Rank-order fp32 + round-once disclosure | `.cu` `paired_finish`/`paired_result8` + docs | rider | Positive instance of B249 slice-3's `cross-rank-reduce-pins-rank-order-not-ring-order` + `rank-partial-fp32-travel-round-once-at-epilogue` — provenance line at most. |
| R3 | `fence.proxy.async.shared::cta` — generic WRITES before async-proxy READ (leader TMA `cp.async.bulk.global.shared::cta` reads the staging tile) | `.cu` `paired_bulk_payload` | rider | Mirror face of B137's fence (which owns generic READS before async WRITE). Home adjudicated at claim: B137 `setmaxnreg-fit-gate-elected-arrive` vs `tma-single-issuer-bulk-tile`. Count-neutral. |
| R4 | Selector/policy manifest bit-agreement at init | `cuda_communicator.py` + `_manifest()` | rider | Face on `tp-checkpoint-byte-agreement-hash-while-loading` (the cross-rank agreement family): agreement extends from checkpoint BYTES to backend POLICY. Count-neutral. |
| D1 | Paired two-round XOR topology + BF16-pair wire packing | `.cu` `paired_pack8` | DROP | Recursive-halving variant (textbook); the packing trick is a wire-layout detail with no bounded rule shape. |
| D2 | Output-gate placement (gate enqueued before `speculator.propose(...)` queues draft collectives) | `model_runner.py` | WATCH | The Codex review's own finding — a LIVE upstream bug, not a landed fix; mining a rule from an unfixed defect is weaker provenance. Watch row only; re-open if upstream lands the fix. |
| D3 | `--ftz=false;--fmad=false` + per-file CMake `COMPILE_OPTIONS` | CMakeLists.txt | DROP (thin) | Positive instance of B03's fast-math law (emmy rider already splits fmad/ftz); the per-file scoping face is a build-system footnote. |
| D4 | pynvml `NVML_P2P_CAPS_INDEX_READ = (0,)` → `0` | `pynvml.py` | DROP | Trivial tuple-vs-int bug. |

## Dedup evidence (grep pairs, run this session)

- `allreduce|all_reduce|pcie|p2p|peer` over `src/domains/kernel_opt/corpus/` → hits are B119's sync modes, B92's semaphore, gpu_30's multi-GPU header, B249's TP rules — **none own fail-stop policy, IPC teardown, or the wait protocol**.
- `relaxed|two-tier|acquire.*confirm` → B92 spins `ld.global.acquire.gpu` single-tier; **the two-tier form is unowned**.
- `proxy\.async|cp\.async\.bulk` → B137 carries `fence.proxy.async` (one direction) + the tests assert it; **the generic-writes-before-async-read mirror face is unowned** (rider).
- Fail-stop/fallback: the nearest corpus law is B208's superset-admission SAFE-FALL-THROUGH (kernel dispatch, not collectives) — **unowned**.

## GOAT adjudication

Criterion 1 fails for a **DOMAIN** reason — multi-GPU sys-scope collective latency has no validator on any single-GPU box we host — so all three rules carry **NotBenchable annotations** (`bench_spec: None`), never a demotion (the dial9 two-class split; the B119/B249/B263 precedent). The magnitude story is carried source-reported (+12.7% DSpark decode, 4× RTX PRO 6000, PR table; foreign silicon, never our bench claim). Criteria 2–4 are satisfied at the decision-content level: integer/flag semantics are exact, the mechanisms are named, the fault paths are deterministic by construction.

## Yield estimate

**2–3 rules + 3–4 riders claimed → 1–2 rules + riders after the ~50% coordinator falsification.** Sized like B119 (2 rules) or B263 (0 rules, 1 rider); never above 3.

## Honest caveats

- **PR-stage, unmerged** — the pin is to a PR head commit; upstream may rebase. Mine-time re-verification against `pull/1788/head` is mandatory and the plan carries it.
- **Python + CUDA, no Rust surface** — pure corpus distillation (the B119 precedent); fixtures will be original Rust reproductions of the protocol shapes, never transcriptions.
- **Silicon-bound** — the kernel is SM120-only with a sticky `kUnsupportedDevice` fault; the RULES survive (the protocols are silicon-neutral), the code does not port.
- **Known-unfixed upstream defect** — the output-gate placement bug (D2) means the source's own "terminal-before-output guarantee" is not yet true of its code; quotes must be taken from the code and docs, never the claims.
- **The +12.7% is e2e and source-reported** — confounded by nothing within the DSpark rows (backend-only delta), but on foreign silicon with no per-phase breakdown; never quotable as ours.
- Inspection depth: full commit diff read twice; the surrounding repo (the other three commits, the B12x backend) NOT read — out of scope by the user's pointer.

## Bottom line

Mine as **nominal Batch 272 (plan 276)**, lead with `faulted-collective-fail-stop-no-numerics-fallback`; authoring at the next mining slot (the queue is post-splash-window; the batch claims plan-first per protocol). Queue line landed in the snapshot; this note + plan 276 landed in the same commit (highwaters 307→308, 275→276). NOTHING INVENTED — every quote read at the fetched commit diff this session.

## Corroboration addendum (2026-10-24, second independent pass)

A second `@distill` pass over the same PR re-derived the verdict independently before finding this note — agreement on every load-bearing call (YES B− / lane N/A / the three-rule + four-rider shape / the D1+D2 drops). Its additions, recorded so the batch lane inherits them (all facts from the fetched PR page + a full shallow clone at the pins below):

- **Upstream absence (novelty provenance):** `https://raw.githubusercontent.com/vllm-project/vllm/main/csrc/pcie_pull_all_reduce.cu` → **404** at vLLM main (checked 2026-10-24) — the mechanism is sonar-original, not a vLLM port; the sonar repo itself is a vLLM sync fork (HEAD `chore(sync): port vLLM through 255f14599`), so the absence is meaningful.
- **The NCCL baseline was TUNED, not a strawman:** the PR body's launch recipe sets `NCCL_ALGO='allreduce:Tree' NCCL_PROTO=Simple` — the low-latency algo/proto pair — and still lost the DSpark cell by 12.7%. This strengthens the note's "confounded by nothing within the DSpark rows" claim with direct evidence.
- **The +12.7% is spec-path-specific:** the two non-spec rows are BOTH NCCL (100.0/100.5 tok/s — no paired cell), so the headline never claims a non-spec win; and draft acceptance dipped 59.43% → 59.05%, consistent with the disclosed rounding-contract difference (a real, disclosed trade, not noise).
- **Pin shas for the T0 re-fetch:** sonar main `11c991b7d7ad1b58bd9dfbfe8f803dd16a29db50`, PR head `f653ba4e156a585531608bb59e51bd1fb63e866e` (commit under verdict `927f3262` = its parent), fetched via `git fetch --depth 3 origin pull/1788/head`. Clone `.raw/sonar` created for this pass and REMOVED after the addendum (unreferenced by the artifacts; T0 re-clones per plan).
- **Dedup independently reproduced** (4 grep families, same four owners) plus one addition: `tma-single-issuer-bulk-tile` (B130) owns the INBOUND tensor-tile mbarrier path — further evidence the R3 rider's OUTBOUND `wait_group.read` staging face is unowned (home adjudication at T2 stands).
- **D2 confirmed live:** the Codex review finding (output gate enqueued before `speculator.propose(...)` queues draft collectives → fault observed one step late) is visible on the fetched PR page, dated 2026-10-23 — the watch row stands.
