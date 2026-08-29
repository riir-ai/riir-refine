# 105 — arXiv pool refill walk #2 (fetch fallback; zero corpus delta, strategic routing)

**Status:** RECORD — pool walked 2026-08-29 (~13:00 +07 window), verdicts below, zero corpus deltas, zero new batches queued. All corpus-relevant leads are doctrine citations or sibling-lane notes; nothing has bounded fix-space.

**Method (the R355 network-unblock path, re-validated):** MCP search/webReader remain quota-gated, but the walk needs neither — `export.arxiv.org/api/query` via the raw fetch tool. Four queries: `cat:cs.PF` (15), `cat:cs.DC` (15), `cat:cs.AR` (12), targeted `all:"speculative decoding" AND cat:cs.LG` (8, the league chat/verify lane). Zero builds, zero GPU — a Plan-337 zero-contention unit (the sibling held the M3 perf lane all cycle: 809/810 quiet-box prep, anchor grep live).

## Already consumed (confirmations — do NOT re-queue)

| arXiv | Where it landed |
|---|---|
| 2608.26612 Launch-Bound/Substitutable MoE | Batch 82 (research 096) |
| 2608.23841 Pipeline-Native (cflow) | Batch 85 (research 098) |
| 2608.26368 SIMD bit-slicing | Batch 83 (research 097) |
| 2608.22110 ANE placement study | riir-ai Research 354 |
| 2608.20961 TreeWY | Batch 82 (research 096) |

## Strategic yields (routed, no fix-space)

1. **2608.26912 TOPIQ (SC'26) — lossy-compression QoI error propagation.** Per-element error bounds do NOT translate into bounds on downstream quantities of interest; predicts QoI bias/uncertainty from <0.1% compression metadata, 93.1% calibration across 552 evals. **Route: doctrine citation for the Issue 750 T3 lossy-surface promotion rule** ("aggregate perplexity flat while family-conditional behavior flips") — third independent academic arrival alongside Research 502 ("Behavior Before Perplexity") + MoEXBench (R102 P3, 2608.21693). Cite as external validation at the next lossy-surface gate touch; no batch.
2. **2608.26225 Agent Mesh — reliability primitives for non-idempotent agent delegation.** 147 production incidents: retry/timeout/error-rate breakers all violate their assumptions on agent workloads (54-call loops no breaker sees; constant progress signals false-tripping; enforcement blocking correct work 107 turns). The doctrine half is load-bearing: **"a reliability decision may be taken only on evidence capable of moving, attributable to what it measures, and deterministic under identical conditions"** (evidence adequacy) + identity adequacy. **Route: riir-dao / KAT service lane** (ban composer's evidence pinning, the KAT ledger anomaly screen, kat_drain runaway guards — the 169-request class is their §retry-inflation). Owner-visible note; no batch.
3. **2608.23986 The Shadow Price of Intelligence.** Degraded-answer retry inflation makes "cheaper model" consume MORE capacity per satisfied answer exactly when capacity binds; reactive throttles can cross an ignition threshold that manufactures traffic. **Route: riir-dao KAT economy + riir-clippy CF ops** (the kat_drain metered lanes' runaway class, formalized; the ops budget-alert design in Issue 044's lane). Strategic reading for the KAT pricing/cap levers; no batch.
4. **2608.26402 SILK — TOCTOU integrity for weight streams.** Load-time RoT verification leaves a check-to-use gap (DRAM/DMA/prefetch tamper after auth); SILK verifies at the final pre-compute boundary using keyed integrity bits riding the quantized weight LSBs. **Route: riir-neuron-db strategic note** — the freeze/thaw commitment → mmap-consume path has the same TOCTOU shape (BLAKE3 check at thaw, raw bytes consumed later); our mitigation today is the freeze envelope's checked read, not a consume-boundary re-verify. Not actionable at current threat model (single-tenant local), recorded for the vessel/attestation lane's threat-model revisit.
5. **2608.27091 Sintr — Byzantine clients in BFT transactional systems.** Client-centric BFT DBs leave integrity exposed to malicious clients; redundant execution + heterogeneous integrity policies cost 3–16% throughput. **Route: riir-chain strategic note** (validator-sdk / client-claim integrity lane; the avatar anti-cheat "validate-then-mirror" pattern is the same family — external validation, no delta).

## League notes (chat/verify lane context — Issue 755/742 reopen conditions, owner-gated)

- **2608.22788 TailSieve (v2):** tail routing + route-specialized speculative decoding; cites **MTP or DFlash** per-route with 2.59× over uniform routing. External confirmation that drafter-per-route specialization is where the chat axis moves — relevant when Issue 755's chat lane reopens (regime-router territory).
- **2608.03447 ASD (Approximate Speculative Decoding, v2):** accept draft mismatches under a target-logit regret budget + reuse the target-greedy suffix; +3.05–15.26% throughput over strict verify, +10–16% acceptance on DeepSeek-V4-Flash+DSpark. **Lossy by contract** — our verify seam is lossless (G1 bit-identity) and stays so for code/doc lanes; the chat lane's tolerance question is the owner's. Record only.
- **2608.13524 DARTree:** diffusion-drafter AR-correction extended chains→trees, training-free, +98.6% acceptance over DFlash same-setting. Chat-lane material if the DFlash2 drafter family reopens (tree verify = the Issue 721 armed-but-structural-negative seam; their acceptance gain is the demand side our 721 economics said can't pay under sequential verify). Record only.
- **2608.26523 VPP (chunked-prefill pipeline bubbles):** equal-size chunks have imbalanced cost because later chunks attend longer KV (bubble 6.4%→0.1% via V-shaped virtual stages, multi-NPU). Single-GPU transferable bit is only the cost-asymmetry observation — our Bench 710 chunked prefill already measured the width axis (8192 loses); pipeline topology is not ours. Record only.

## Drops (with reasons)

- 2608.27046 RLM training parallelism survey (Besta et al.) — training-systems taxonomy; riir-train adjacent at survey grade, zero fix-space.
- 2608.27029 FoldPipe — self-described integration of established prefetch techniques; 1.059× inconclusive by its own CI.
- 2608.26235 Reasoning Tax (TES metric) — benchmarking economics; strategic at most, no consumer lane.
- 2608.25053 Hydra edge-LLM characterization — measurement-methodology (phase-aware timing schemas); our Bench 761/763 decomposition work is the in-stack equivalent; no delta.
- 2608.26418 Redwood (AI-designed accelerator) — system claim paper, no mechanism transferable to our software stack.
- 2608.24070 Compression Trinity (PhD thesis) — training-side compression methods (riir-train weak pointer, modelless-first excludes); post-training members (OPTIMA/SLiM) are quantization-recovery training, not inference primitives.
- 2608.27278 WSI I/O pipeline / 2608.27099 quantum partitioning / 2608.26453 WAN training / 2608.26575 Blackwell TEE / 2608.26537 IBLT reconciliation / 2608.26573 gateway MILP / 2608.27336 stochastic broadcast / 2608.26040 RLNC / 2608.26021 Slasher / 2608.25890 RNN load-balancing — no lane.
- cs.AR block (27199/27184/26758/26342/26100/26072/26049/26223/26220/25536) — EDA/NoC/Rowhammer/Ising/DNA-storage; no lane.
- 2608.13505 Intern-S2 / 2608.05303 EdgeXpert / 2608.04962 SpecRoll / 2608.02989 AcceptMoE — model release / HW accelerator / RL-rollout engine / MoE verifier routing (we are not MoE); no lane.

## Queue state after this walk

- Corpus: UNCHANGED (400 kernel_opt + 94 rust_perf + 41 clippy_lints + 11 sec + 31 docker). No batch queued from this walk — the intake lanes (opponent watch, measured-GOAT distillations, sec/docker phase tails) remain the live feeders.
- Next executable units unchanged: the sibling-held M3 lane (809/810 quiet-box promotion decision) on a quiet box; 4090 league units per Issue 780; pool refill re-walk only after new submissions accumulate (this walk swept 08-24→08-29; re-walk window 09-01+ is the honest cadence).
- MCP search/webReader quota: still gated until 09-15; the fetch fallback (this note) is the standing refill path per R355.
