# Distill verdict — ss16118/low-latency-nccl (arXiv:2607.16100, near-SoL GPU collectives)

**Status:** RECORD — batch landed 2026-09-06 (Batch 119: 2 kernel_opt rules + 1 rider, corpus kernel_opt 550 → 552, category E 43 → 45; `entries_gpu_30.rs`; GOAT green — batch119 gates 5/5 FIRST-SHOT incl. the sentinel ABA/reset numeric gate + the epoch-band/rotation exactness gate, kernel_heal lib 1455/0/1i debug + 1456/0 release, default 1142/0, combined 1574/0/2i floors HELD zero re-pins [retrieval re-run kernel 100/100/100, blended 94.7/96.9/97.7 of 131 — byte-identical to Batch 118], clippy 0 in default / kernel_heal / kernel_heal+latent lib + all-targets, score-bench #66 FLAT 96.2/1.9/100/100). PROVENANCE corrections at mine time (repo pin 5357eff3, verified): the repo names the sentinel protocol POISON — `ncclLLPoisonBuffer(Async)` fills, `ncclLLBufferIsPoisoned` polls, the receive path's Reset arm re-poisons (no `ncclLLBufferInitSentinel` exists); the host sizing helper is `ncclCalcMinReductionBufferSize` (no `ncclCalcScratchBufferSize`); LL128 is ENUMERATED as "NOT YET IMPLEMENTED" with a bare TODO in the receive path — the LL128-atomic determinism caveat cites the PAPER, never shipped code. Honest finds carried in-rule: the poison table maps by dtype SIZE; the "Floating-Point Type Overrides" section header is followed by NO specializations; a usage comment claims 0x7FC00000/0xDEADBEEF where the table carries 0xFFFAFFFA-class NaN patterns; the 4-byte poison IS a representable i32 (-327686) — the int-dtype collision hazard is documented as constraint (a)'s boundary. `raw` clone removed at landing.

- **Verdict date:** 2026-09-06 (M3 box, paper-surface lane)
- **Source (paper):** arXiv:2607.16100 — "Every Microsecond Matters: Achieving Near Speed-of-Light Latency in GPU Collectives", Shen, Korzh, Bachan, Chen, Goel, Schneider, Kousha, He, Jeaugey, Iskra, Chandawala, Hammond, Hoefler (ETH Zürich + NVIDIA). Fetched full PDF via jina reader; 14 pp.
- **Source (code):** https://github.com/ss16118/low-latency-nccl (paper footnote 1) — **NOT yet cloned at verdict time**; sha + license to be pinned at mine time (Plan 098 T1) BEFORE any entry cites file:line. Provenance chain for the mechanisms: NCCL LL/LL128 (Hu et al., HOTI 2025 "Demystifying NCCL"), NVSHMEM, MSCCL++ — the paper cites its own lineage honestly; the corpus entries mine the PAPER's composed taxonomy (LL vs sentinel vs barrier as a three-way sync-mode decision), not any single upstream.

## What it is

Barrier-free low-latency AllReduce family within one NVLink domain, built on NCCL's device-side API. Four mechanisms, composed:

1. **LL flag-in-data** — pack an 8-byte flag with 8-byte data per slot into one 16-byte atomic store; the reader polls flag==epoch. Kills the signaling step entirely; costs 2× slot width (halved effective bandwidth) + 2× scratch.
2. **Sentinel sync** — initialize the peer-visible scratch with a dtype sentinel (e.g. `-NaN`); writers store data directly; readers poll `value != sentinel`. Full effective bandwidth, no flag traffic, no data→flag ordering fence; costs a reset-before-reuse protocol + the constraint that no valid value may equal the sentinel.
3. **Bidirectional double-buffering** — chunked exchanges rotate two scratch sub-buffers; a rank cannot overwrite a peer's next-iteration buffer until it has received that peer's current-iteration data ("each receive is implicit permission for the next send" — credit-based flow control), eliminating iteration-boundary barriers.
4. **Two-shot LL128-atomic** — 8 threads own one 128B cache line; element 0 is the flag carrier (displaced to smem, restored by 16 extra threads); contribution via atomic ADD into the target scratch; flag==N means all ranks contributed. ~3% bandwidth overhead at FP32 — but non-deterministic FP accumulation (γ_{N−1} ≈ 3.8e−6 worst case at N=64 FP32), addition-only, and bound to NVLink cache-line atomicity.

Plus a **SoL floor methodology**: `L_SoL = 2·L_L2RTT + L_remote_store`, with `__threadfence()` latency as the L2-RTT probe and a ping-pong RTT decomposition for the remote-store term — measured 1.404 µs on GB200; their one-shot lands ~7% over it; NCCL ring reads 11.0 µs → their kernel 2.37 µs on 4×GB200; two barriers ≈ 40% of a small-message AllReduce.

## Verdict

**YES, grade B corpus / NO lane mapping.** The league and the entire workspace are single-GPU (zero collective/multi-GPU surface — league doc, `.issues`, `.docs/09_performance` all grepped), so the lane-intel axis does not fire. The corpus case stands on the B110 precedent: that batch landed Hopper/Blackwell-only PDL machinery that cannot execute on any box we own, at grade A−, because **corpus rules serve user code on hardware the workspace does not own** — multi-GPU peer-access CUDA (NCCL/RCCL consumers, inference clusters) is a real segment of healed traffic, and the LL/sentinel pair needs strictly LESS hardware than PDL (peer store visibility only; the paper states the protocols port to any platform with GPU-initiated peer access + device-side fences).

Yield estimate: **1–2 rules + 1–2 riders** after ~50% coordinator falsification — the recent-batch norm (B113: 4 rules + 18 riders; B117: 5 + 8).

## Domain routing

`kernel_opt` (sync category). No tempting wrong domain — the host-orchestration half of barrier elimination already has its rule (`barrier-free-per-dependency-wait-from-loop-counters`, entries_gpu_07); this batch is the DEVICE-side peer-poll half.

## Ranked candidate table

| # | rule_id (proposed) | Grade | Mechanism + regime | Disposition |
|---|---|---|---|---|
| 1 | `sentinel-readiness-over-separate-flag` | **B — batch lead** | Peer-written data polled by device readers: init scratch with dtype sentinel (`-NaN`), write data directly, poll `val != sentinel` — deletes the flag array, the flag stores, and the data→flag ordering fence; full bandwidth preserved. Regime: moderate messages (LL wins the tiny regime). Constraints as decision text: (a) no valid value may equal the sentinel (FP32: sign-bit-set NaN payload), (b) reset-before-reuse protocol, (c) requires peer store visibility, (d) single-writer/single-reader slots ⇒ result determinism PRESERVED (unlike #4). | KEEP — T3 entry 1 |
| 2 | `ll-flag-in-data-packed-poll` | **B−** | Tiny-message regime twin: 16B atomic `{flag, data}` per slot, poll flag==epoch (monotone round counter). Halves effective bandwidth + doubles scratch, removes the signaling RTT — wins where payload is latency-bound small. Two-regime pair with #1 selected by message size. | KEEP — T3 entry 2 (or folded into #1 as the regime split) |
| 3 | — | rider | Bidirectional double-buffer credit: "each receive from a peer is implicit permission for the next send" — the receive-grants-reuse credit logic B58 `output-buffer-rotation-pool-multi-forward` already encodes for dispatch pools, phrased for peer exchange. | RIDER → B58 concept text + citation |
| 4 | two-shot LL128-atomic | **C− — DROP as a rule** | Displaced-element flag carrier + atomic-add accumulation. Hardware-bound (NVLink cache-line atomicity; CUDA vectorized atomics f32/f16 only), addition-only, NON-DETERMINISTIC FP (γ_{N−1} bound). Disqualifying for bit-identity/replay-gated consumers — recorded as the determinism caveat INSIDE entries 1–2, and the displaced-element trick recorded here for a future hardware-specific rule. | DROP (caveat text + note) |
| 5 | SoL floor methodology | not-a-rule | `2·L_L2RTT + L_remote_store`, threadfence-as-probe, ping-pong decomposition. Measurement instrument; consumable only if a collective lane ever opens (none exists). | Recorded in this note only |

## Dedup-grep evidence

- `sentinel|LL128|AllReduce|NCCL|collective|multi.?gpu|peer.?poll` over `src/domains/**/corpus.rs` **and** the full `kernel_opt/**` tree: zero relevant hits (2 unrelated: a division-test "sentinel boundary" comment in rust_perf; a sec-domain FFI value-return note).
- Third-vocabulary pass `semaphore|spin.?wait|acquire|poll` over `kernel_opt/**`: every hit is **barrier-class** — `stripe-sms-l2-semaphore-reduce` (B92: flag AFTER data behind `ld.global.acquire.gpu` spin + `fence.acq_rel`; the baseline this paper replaces), `barrier-free-per-dependency-wait-from-loop-counters` (host-orchestration, separately-scheduled workers), `shared-counter-decode-producer-progress`, `first-readback-pays-queue-drain` (host readback timing). **No flag-IN-data, no sentinel-init entry exists.** B58 owns buffer-rotation credit for dispatch pools — rider target, not a collision.

## Honest caveats

1. **Verdict surface is the PAPER, not the repo** — `ss16118/low-latency-nccl` uncloned; sha + license unpinned; Plan 098 T1 pins both and locates the file:line anchors (ncclLLBuffer ctor/`advanceEpoch`, `send`/`recv`/`recvUnrolled`/`recvReduce`/`bcast`/`reset`, `ncclLLBufferInitSentinel`, the LL128-atomic kernel) before any entry carries a source citation.
2. **No local re-measurement possible** — M3 and 4090 are single-GPU; the magnitude story is the paper's own (barrier ≥ 1 µs each; 11.0 → 2.37 µs small-message AR on 4×GB200; SoL 1.404 µs, one-shot within 7%) under the absolute-numbers caveat (their silicon, not our cells). GOAT takes the measured-GOAT-distillation form (source measurements back the rule), as with the B49/B52-class precedents.
3. **Suggest-only fix class** — replacing a barrier/flag protocol with sentinel/LL sync is a restructure, not a mechanical proposer edit; entries land suggest-only like the rest of the sync family.
4. **Validation = parse gate only** (kernel_verify is a compile STUB) — as for all kernel_opt entries; no numeric oracle exists for peer-poll code on this hardware.
5. Queue slot: **behind arxiv walk #8 (time-gated Mon 09-07 ~07:00) + Batch 118**; 4090-activity checked before mining per the standing rule.
