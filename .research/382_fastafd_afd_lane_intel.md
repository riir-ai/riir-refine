# Research 382 — FastAFD distill verdict (arXiv 2611.07149 code release)

**Status:** RECORD — lane-intel HIGH for riir-infer plan 623; corpus delta FACE-ONLY (rides the next kernel_opt batch's rider slot); NO batch plan staged (Plan 350 NOT allocated — the claim's plan-first framing adjusts at landing per the LoopDuet precedent: the gate closes before allocation, the number stays free)

## Source

- Repo: `hao-ai-lab/FastAFD` @ `003770e2337d26ffc4db8825289cfc36c6df4c3d` (HEAD push 2026-11-04T19:16:55Z, "Update FastAFD to the latest version (#10)").
- Paper: arXiv 2611.07149 "FastAFD: When Does Attention-FFN Disaggregation Pay on Rack-Scale NVLink?" (walk-24 B lead; submitted 2026-11-04). The abstract names the repo directly.
- License: **MIT** (Copyright (c) 2026 **sgl-project** — the SGLang lineage; the package is `minisgl`, an SGLang-derived AFD engine). Outbound-PR posture OPEN.
- Size: 193 Python files (attention/ cad/ distributed/ engine/ kernel/{csrc,triton}/ kvcache/ layers/moe/ moe/ scheduler/ server/ spec/) — a full serving engine; 805 MB clone (34 MB assets).

## Verdict line

**RECORD — lane-intel HIGH for riir-infer plan 623 (the multi-host lane) / corpus B−/C+ THIN: one face + one suggestion-only, no dedicated batch / product-coverage: the AFD-serving family (first member; sgl-project lineage — second source from that org after Batch 333's sgl-nvidia-mega).**

## Mechanism map (what the repo does)

Attention servers (KV-cache attention, routing, sampling) and MLP servers (expert execution) split an MoE decode: attention ranks send routed activations N2M, MLP ranks run experts and return combined outputs M2N, mapped onto an **asymmetric DeepEP-style dispatch/combine path** run as a **role-specialized MoE megakernel** ("fewer launches and synchronization points"). Each decode batch splits into **micro-batches so N2M dispatch, expert execution, M2N return, and attention-side work overlap across layers** instead of serializing at every AFD boundary (micro-batch ping-pong). A **zero-overhead coordinator prepares the next decode plan while GPUs execute the current one** — "publishes micro-batch order, buffer slots, request ownership, and peer metadata one step ahead, so scheduling stays off the decode critical path." Measured: 1.35-1.45× per-GPU decode over colocated on GB200 NVL72 (README), up to 1.17× vs the best vLLM wide-EP (paper); the analytical model predicts AFD throughput **within 3% from colocated-vLLM profiles alone** via the decomposition **batch gain (attention GPUs hold no expert weights → larger batches) × overlap gain (MoE runs behind attention → shorter steps) vs disaggregation cost (the GPUs given to experts)** — AFD pays when the two gains outweigh the cost; speculative decoding helps light-expert models most (+8%); experts on SRAM-resident accelerators would remove most of the cost (ideal 2.48×).

## Candidate adjudication (all graded in-row; the corpus yield is the point)

| # | Candidate | Verdict | Reason |
|---|---|---|---|
| 1 | One-step-ahead plan publication (the zero-overhead coordinator) | **FACE** on `pipelined-token-slot-ping-pong-submit-ahead` (gpu_18) | gpu_18 owns the submit-ahead GENUS (keep the loop-carried dependency off the critical path; ping-pong slot pair + submit-ahead, single-GPU decode-loop scope). FastAFD's coordinator is the DISTRIBUTED face: publishes order + slots + ownership + peer metadata one step ahead across servers. Same law, wider scope — cite, never re-own. The scheduler's order-cache invalidation pattern (`scheduler/decode.py:13-45` — `_ordered_uids`/`_ordered_reqs` cached, `changed`-flag invalidated on membership mutation) is a memoization face (kin: the B334 single-entry last-key cache lead). |
| 2 | Micro-batch ping-pong overlap across the AFD boundary | KILL (dense kin) | The corpus's overlap/dispatch/batch families: `expert-loop-two-phase-split-batch-across-iterations`, `expert-prompt-row-batch-amortizes-weight-pool-stream`, `batch-blocking-pool-segment`, `barrier-on-reader-open-group-conflict-swap-overlap` — the boundary-overlap axis is owned class-wise; the AFD-specific wiring is serving policy, not a Rust fix-space. |
| 3 | N2M/M2N role-specialized MoE megakernel | KILL (same-org dedup) | Batch 333 distilled `sgl-project/sgl-nvidia-mega` @ `c396f2ab` (the SGLang NVIDIA megakernel plugin) — the megakernel dispatch/fail-stop/mailbox families are owned from the SAME ORG's codebase; FastAFD is the minisgl fork applying them to AFD topology. The DeepEP-style asymmetric dispatch/combine path is the same family the vllm-rlt/B336 + B333 entries cover. |
| 4 | The analytical model (batch gain × overlap gain vs disaggregation cost, predicted within 3% from colocated profiles) | **SUGGESTION-ONLY / lane-intel** | Offline planning policy — NotBenchable-by-domain (no runtime kernel fix-space; we don't run multi-host MoE serving). The DECOMPOSITION is the decision content: it is the payoff-probe frame for ANY disaggregation decision, including plan 623's (below). |
| 5 | Speculative decoding helps light-expert models most (+8%) | lane-intel row | The overlap-gain/spec-dec interaction — recorded for the serving/spec-dec tier watch (the S-g6-class row from walk-23's ledger). |

## Lane-intel (workspace) — the VALUE of this lead

- **riir-infer plan 623 (multi-host pipeline) — lane-intel HIGH.** The AFD decomposition maps DIRECTLY onto plan 623's attention/FFN multi-host split decisions: (a) the batch-gain × overlap-gain vs disaggregation-cost frame is the pre-deployment payoff probe — plan 623's dist_stage cells (the B928 wire-law lineage: recv rings ≤ 3 MB or runtime-probed, UC wire shape, foreground-chunked cells) can adopt the colocated-profile-only prediction shape (profile once colocated, predict the disaggregated payoff, deploy only when the gains outweigh the cost — never a blind split); (b) the N2M/M2N asymmetric dispatch/combine path is the same dispatch-plane shape as our RDMA verbs lane (SEND/RECV-only, poll-timeout peer failure — the B928 laws); (c) the "predict within 3%" bar is the calibration bar for any plan-623 cost model. **The OWNING LANE (riir-infer) files any issue; never this verdict's act.**
- **4090 league axis:** none — GB200 NVL72-only machinery (aarch64 + CUDA 13 + NCCL 2.30 + DeepEP); nothing on sm_89. The league prefill/decode lanes are untouched.
- **Author/org watch:** hao-ai-lab (vLLM ecosystem; DistCA authors) + sgl-project lineage — a THIRD org-family source after B333 (sgl-nvidia-mega) and the vLLM distills. The sgl-project org is now twice-mined; a third source from it would justify a family census row.

## Product-coverage

Seeds the AFD-serving family (first member). Follow-up pool: AFD-Ledger (arXiv 2608.04502, deployment provisioning), vLLM AFD plugin (vllm.ai blog 2026-07-23), DistCA (core-attention disaggregation, same lab — the cad/ module lineage).

## Dedup evidence (greps this session, at corpus HEAD 1056)

- Submit-ahead: `pipelined-token-slot-ping-pong-submit-ahead` (gpu_18) present — candidate 1 is its distributed face.
- Megakernel/dispatch: the B333 sgl-nvidia-mega entries + `splitk-*` cluster (6 rules) + the dispatch families — candidates 2-3 densely kinned; killed.
- Overlap: `barrier-on-reader-open-group-conflict-swap-overlap` + the batch-split families — candidate 2 killed.
- Offline policy: no allocator/calibrator owner needed — candidate 4 suggestion-only.

## GOAT note

No benchable rule survives dedup — the batch slot is NOT justified (yield: 1 face + 1 suggestion-only). The face rides the next kernel_opt batch (338/339/340 conversions) as a count-neutral rider; the lane-intel row graduates to a plan-623 issue ONLY when the owning lane arms a multi-host AFD-shaped cell.

## Re-open triggers

- plan 623 arms a second-host AFD-shaped split (attention/FFN or prefill/decode across hosts) → the payoff-probe shape + the N2M/M2N dispatch-path kin graduate to owned issues in riir-infer.
- A third sgl-project/hao-ai-lab source lands → the family census row opens (sgl family currently: B333 mega + this record).
