# 433 — omarchy-mlx Vulkan barrier-scope article (joshuawarren.com, 2026-11-22)

**Status:** LANDED — Batch 430 (2026-11-28, Plan 421-sibling slot; the verdict taken to execution by the idle-loop batch lane: entries_gpu_174 `compute-to-compute-narrow-barrier-transfer-guarded` + 1 count-neutral census face on merge-dispatches gpu_02; fixtures b430 5/5 standalone debug AND -O; lib 4021/0 both postures + clippy -D ×3 + pins_manifest 1154 AGREE + G1 census 299 green / 0 FAIL / 1154 rows; T0 re-clone at tip cccad655 — the mined surfaces untouched by the 83 commits past the article pin). Prior status: DISTILLED — pending owner decision

## Source

- Article: <https://joshuawarren.com/blog/mlx-vulkan-beats-llama-cpp-cuda-on-rtx-5060-ti-egpu/> — "124 tok/s: MLX over Vulkan beats llama.cpp CUDA on an RTX 5060 Ti eGPU" (Joshua Warren, 2026-11-22). Full-article fetch this session; **NOT cloned** (the blog-verdict precedent — kapteko, Research 432-era snapshot row; a future batch T0 clones `joshuaswarren/omarchy-mlx` at the PR-604 merge sha and pins the license).
- Provenance chain: `joshuaswarren/omarchy-mlx` (a fork adding a Linux Vulkan backend to Apple's MLX; **MLX upstream has NO Vulkan backend — the fix is fork-local**, so there is no upstream fix space at ml-explore/mlx). The mechanism shipped as their **PR 604** (merged, default-on for discrete GPUs; `MLX_OMARCHY_COMPUTE_BARRIER=0/1` env override). **License UNVERIFIED** — MLX upstream is MIT-family but the fork's tree was not read; any batch T0 pins it.
- Upstream state (the P113 check): the fix is **THEIR OWN landed decision** (PRESENT at the fork pin, default-on) — mineable as decision content ONLY, never an outbound PR.

## The mechanism (article verbatim facts)

Autoregressive decode = a long chain of short dependent compute kernels. Their Vulkan backend's dependency tracker issues 90,003 dispatches across 1,525 submissions over a 256-token run; it already elides non-overlapping hazards (10,678 barriers skipped) and still emits **82,238 barriers = 321 per token**. Before PR 604 every emitted barrier was a full `VK_PIPELINE_STAGE_ALL_COMMANDS_BIT` synchronization; PR 604 adds `CommandEncoder::record_dispatch_barrier()` emitting a narrow `COMPUTE_SHADER → COMPUTE_SHADER` memory barrier (`SHADER_WRITE → SHADER_READ | SHADER_WRITE`) when FOUR conditions hold together:

1. the node is a compute dispatch;
2. the encoder is past the batch head (work arriving after host writes or an earlier batch still takes the full barrier);
3. no transfer ran since the last full barrier — every direct `vkCmdCopyBuffer` / `vkCmdFillBuffer` / inline `vkCmdUpdateBuffer` sets a `transfer_since_barrier_` flag, and only the full `record_dependency_barrier()` clears it;
4. wave scheduling is off (condition unexplained in the article — the PR is the authority; T0 reads it).

Barrier COUNT unchanged (82,238 emitted, 10,678 skipped, 1,525 submissions — the synchronization schedule is identical); only SCOPE narrowed. Measured (source-reported, foreign box — provenance, never bars): Qwen3-4B-Instruct-2507 4-bit greedy on RTX 5060 Ti 16 GB (driver 615.78.08), five interleaved A/B pairs, token IDs bit-identical across all ten runs (`f0684b4aa9b35796`): decode 99 → 124 tok/s (**+25.8%**, 5/5 pairs), token-6-onward 103 → 131 (+27.4%), per-token latency −2.0 ms, **GPU busy time per token −1.87 ms**. Same card: llama.cpp CUDA ~101 tok/s, llama.cpp Vulkan 85 tok/s.

## Verdict

**YES B− corpus / LANE INTEL HIGH / product-coverage additive (wgpu host-orchestration family).**

One deep, measured mechanism — barrier-SCOPE selection — which the corpus does not own. The measured story is exceptional for the corpus's league metric (decode tok/s on NVIDIA silicon, beating llama.cpp CUDA on the same card, bit-identical tokens, interleaved pairs — exactly our league's A/B discipline).

## Domain routing

**`kernel_opt`, host-orchestration-around-GPU / synchronization class** (the corpus's launch/sync family span). The tempting WRONG domains: `rust_perf` concurrency (the deferred-command-flush family is CPU-level command-barrier batching — different medium, see adjudication below) and the CPU-SIMD trap (n/a — this is GPU). Note the canonical shape: the rule's Rust fixture is a host-side encoder policy (state machine + barrier-scope decision), not a kernel body.

## Ranked candidates

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `compute-to-compute-narrow-barrier-transfer-guarded` — emit the NARROW compute→compute barrier (SHADER_WRITE → SHADER_READ\|SHADER_WRITE) for pure-compute dependent chains instead of the full all-stage barrier; the four-condition predicate + the `transfer_since_barrier_` flag are the decision content; barrier COUNT unchanged, SCOPE narrowed | article §"Keeping the narrow barrier safe" + PR 604 | **B+ LEAD** | UPSTREAM: present at their fork (PR 604 merged, default-on discrete GPUs; Apple iGPU + software Vulkan keep full; env override) — decision content, never a PR. Two-regime shape: pure-compute chain (narrow) vs transfer/host-containing chain (full) — the corpus's favorite. Fixture = deterministic state machine over dispatch/transfer/full-barrier events asserting scope choice + stale-read refusal arms. NotBenchable-by-domain (criterion 1 fails for a DOMAIN reason — the win metric is decode tok/s on a real GPU runtime no validator hosts; Batch 136 / Lattimore precedent: suggestion-only annotation, corpus can still grade YES). The four conditions are separately fixture-assertable; condition 4 needs the PR read. |
| 2 | barrier-census-per-token face (count-neutral) — extend the launch-census rider on `merge-dispatches-into-single-compute-pass` (Batch 142: "count LAUNCHES per step, not just kernel time") with the BARRIER half: count barriers per token AND the hazard-tracker's elided count — the article's 90,003 dispatches / 82,238 emitted / 10,678 skipped / 321-per-token census is what PRICED the lever before the fix | article §"Why one barrier moved the needle" | face, B | Folds onto rule 1's notes or the existing rider; count-neutral. The census-before-arm discipline matches our engagement-proof law. |
| 3 | Drop — "MLX-Vulkan beats CUDA" hardware line (eGPU/USB4 config, 5060 Ti) | article framing | drop | League intel only, not corpus: a config datum, not a pattern. |
| 4 | Drop — llama.cpp-Vulkan over-barrier hypothesis | inference from their 85 tok/s row | drop (UNVERIFIED) | The article does not diagnose llama.cpp's barrier behavior; no code read. If ever re-opened, it is an upstream-PR lane (outbound-pr gates), not a corpus row. |

## Dedup gate (greps this session, `src/domains/`, both vocabularies)

`barrier|ALL_COMMANDS|compute-to-compute|dispatch barrier|pass boundary|stage_mask|hazard` over kernel_opt: zero owners of barrier SCOPE / stage-mask narrowing; every hit is an adjacent-but-different axis. Four adjacencies adjudicated for the future rule's in-rule text:

1. **`merge-dispatches-into-single-compute-pass`** (entries_gpu_02, + Batch 111/142/274 riders) — wgpu pass-level: dependent dispatches in ONE pass get in-order memory ordering with NO per-dispatch barrier; the axis is "how many pass boundaries/dispatches". THIS source's axis is "which scope does an already-emitted barrier carry" — a raw-Vulkan-level decision that exists precisely where a backend (or an as_hal custom lane) emits explicit per-dependency barriers instead of hiding them behind a pass. Compose, never merge.
2. **`two-pass-partial-reduction-inter-dispatch-barrier`** (entries_gpu_116, B269) — the CORRECTNESS face: stale partials between dispatches fixed by ADDING an inter-dispatch memory barrier. Opposite-polarity sibling: that rule adds synchronization for correctness, this rule keeps the schedule identical and narrows scope for speed; both are barrier-policy decisions, neither subsumes the other.
3. **`chain-dispatches-single-sync-point`** — command-buffer-level sync amortization (how many sync points); scope-of-each-point is unowned.
4. **`deferred-command-flush-hoist-out-of-batch-loop`** (rust_perf B174) — CPU-level deferred-command barrier amortization; the vulkan `transfer_since_barrier_` flag is a hazard-tracking predicate, not a flush hoist.
5. The in-kernel smem barrier rules (`dual-accumulator-single-reduction-sweep`, the MLX staged-loader one-barrier-per-K-step faces) are intra-kernel threadgroup barriers — different layer entirely.

## GOAT test (the four criteria, rule 1)

1. **Benchable** — FAILS for a DOMAIN reason: the win is decode throughput on a real serving runtime; no Pareto/alloc validator hosts GPU decode. Per the dial9 two-class split this is the annotation class (`NotBenchable`, `bench_spec: None`), NOT a demotion.
2. **Exact semantics** — barrier scope is exact; their own G1 anchor is token-ID identity across all ten runs — the fixture's correctness gate mirrors it (identical schedule, identical output, scope-only delta).
3. **Magnitude story** — mechanism-level: 321 barriers/token × per-barrier pipeline-drain cost = 1.87 ms/token of GPU busy time on a ~9.7 ms token (−19% busy; −25.8% wall). Not "probably faster".
4. **Deterministic** — greedy decode, bit-identical tokens, 5/5 interleaved pairs.

## Lane-intel axis (HIGH — this is our league's metric)

Mapped against the league record (`.docs/09_performance/perf_rematch_league.md`; standing 4090 rows: decode tg128 ~0.970×, pp2048 ~1.27×, pp4096 ~1.21× vs llama.cpp — we sit at decode parity with the opponent):

- **Opponent-class headroom, named mechanism:** a barrier-SCOPE fix alone moved decode +25.8% on NVIDIA silicon and crossed past llama.cpp CUDA on the same card. Read: the sync-structure class (not kernel math) holds ≥20% decode headroom for every chained-dispatch runtime, llama.cpp included. Our own league cells compare against llama.cpp CUDA — if the opponent takes this class of fix, our tg128 row moves without us touching a kernel; if WE take the equivalent in our own runtime, the row moves the other way. Escalation-ladder-relevant (the `perf-rematch` skill's opponent-diff lane: this is exactly the "opponent merged PR" class the weekly matrix watches — except it landed in a fork, not upstream).
- **Our runtime:** our wgpu/Metal decode chains already apply the corpus's pass-merging rule (riir-gpu forward-orchestration cluster — dependent dispatches merged into one compute pass), which is the wgpu-level answer to the same physics. The residual lever this article names lives BELOW wgpu's abstraction: raw stage-mask scope on explicitly emitted barriers, reachable only through an `as_hal` raw-backend lane (the corpus's `native-handle-zero-copy` precedent is the integration path). No current riir-infer/riir-ai lane emits raw Vulkan barriers; the lever is LATENT, not open. **Concrete offered follow-up: a barriers/dispatches-per-token census on our wgpu postures** (4090 Windows wgpu = Vulkan/DX12; M3 Metal) — if any posture still pays per-dispatch pass boundaries the census prices it before any code moves (engagement-proof law).
- **Not transferable:** the CUDA serving lane (4090 cudarc/qwen38_dense_cudarc) — CUDA's stream model has no pipeline-stage barrier to narrow; the eGPU/USB4 5060 Ti posture (16 GB, small-VRAM class) is not our league silicon; the 4B-model magnitude is unproven at our 27B scale (decode is shorter-chain-per-token at larger batch, but the barrier/token count scales with LAYERS, so the class plausibly grows — unmeasured).
- **Follow-up offered:** file nothing automatically (no open lane owns raw-Vulkan barriers); the census probe folds into the next wgpu-posture lane touch; the maya/omarchy-class opponent-watch row rides the existing league weekly.

## Product-coverage axis (additive)

The patterns live in wgpu/Vulkan host-orchestration — wgpu is top-tier Rust infrastructure and the corpus's wgpu host-orchestration family (pass-merging, uniform slots, pool aliasing, as_hal native dispatch) already seeds it. This adds the barrier-scope axis the family lacks; not a GAP-class seeding (the family is not thin), so it does not inflate the grade.

## Honest caveats

- Source-reported numbers on a foreign box (5060 Ti eGPU, Omarchy Linux) — provenance, never bars.
- Single model (Qwen3-4B), single card, small-VRAM eGPU posture; magnitude at 27B/dedicated-class unproven.
- Fork-local fix; MLX upstream has no Vulkan backend, so the provenance chain ends at the fork — license UNVERIFIED until a batch T0.
- Condition 4 ("wave scheduling off") unexplained in the article — the PR diff is the authority; treat the predicate as 3-verified + 1-T0.
- Article not cloned; byte-level quotes exist only for the fetched prose (the numbers table + the four conditions). A batch T0 re-derives everything from the PR diff at the merge sha.
- The +25.8% headline is median-of-5-pairs on a 256-token window — honest, but one posture; their token-6-onward (+27.4%) row suggests steady-state slightly higher.

## Yield estimate

1 rule (`compute-to-compute-narrow-barrier-transfer-guarded`) + 1 count-neutral census face after coordinator falsification. Nominal slot: **Batch 380**, staged pending owner decision; the batch plan opens before any corpus edit per the plan-first law.
