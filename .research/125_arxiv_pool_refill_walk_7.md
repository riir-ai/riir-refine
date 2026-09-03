# Research 125 — arxiv pool refill walk #7 (2026-09-05 ~04:15 +07 window check)

**Status:** RECORD — strict differential EMPTY (frontier not advanced); the walk's value
is the backfill triage of walk #6's window under the broader standing queries (11
unrecorded rows, 5 graded) + the window-mechanics correction. Zero corpus delta.

## Method + window mechanics (honest notes)

- Same five lanes as walk #6: spec-dec unrestricted (`all:"speculative decoding"`),
  KV (`all:"KV cache"` — BROADER than walk #6's KV+eviction), quant
  (`abs:quantization AND abs:inference`), kernel lane
  (`abs:"GPU kernel"` × cs.DC/cs.PF/cs.AR), cs.PF listing. Export API healthy —
  all five answered back-to-back, no 429/503.
- **Run through `curl` + stdlib XML parse this walk** (MCP search quota exhausted
  until 09-15; the `fetch` MCP tool strips Atom `<entry>` bodies — only the feed
  header survives its markdown conversion, and its `submittedDate:[...]` range
  filter returns totalResults=0 through both the `+` and `%20` encodings). The
  range-filter defect is why lanes were fetched unbounded (max 60–80, sorted
  submittedDate desc) and window-filtered locally by `<published>` — the robust
  form; recorded so future walks skip the range-filter dead end.
- **Strict window finding: EMPTY.** Walk #6's frontier was the batch with
  submissions 09-01T18:00Z → 09-02T18:00Z. Walk #7's strict differential =
  submissions 09-02T18:00Z → 09-03T18:00Z (the batch walk #6 predicted would
  land "Fri 09-04 ~07:00 +07"). Measured 2026-09-04T21:15Z: **zero rows
  in-window across all five lanes, and the ENTIRE API tops out at
  09-02T17:59:46Z** (cs.LG probe, newest = 2609.02887), with the cs.LG
  listing page's newest announcement group still "Thu, 3 Sep 2026" — no
  Fri 09-04 group. **Walk #6's announcement-timing prediction did not hold**;
  per the Sun–Thu 20:00 ET schedule the next observable announcement is
  **Sun 09-06 20:00 ET (Mon 09-07 ~07:00 +07)**, and the 09-02T18:00Z →
  09-03T18:00Z submissions roll into it. No re-walk before then can see
  anything — next window recorded in the queue snapshot.
- **Backfill triage (the walk's yield):** the broader lane queries surface
  **12 rows inside walk #6's own strict window** (09-01T18:00Z → 09-02T18:00Z)
  of which walk #6 recorded exactly ONE (AceSpec). This is the walk-#5
  miss-class again (SMELT/mzCache), root-caused differently: walk #6's
  per-lane queries were narrower than the standing set recorded in the queue
  snapshot (KV+eviction vs plain KV; quant+inference misses pretraining-side
  and capability-study rows that never say "inference"). The narrower queries
  were walk #6's execution choice; the standing set is the contract. Backfill
  rows graded below; nothing from the 09-03 window exists to grade.

## Backfill differential table (walk #6's window, unrecorded rows)

| # | ID | Title (short) | Submitted | Lane | Grade | Mechanism / relevance | Action |
|---|---|---|---|---|---|---|---|
| 1 | **2609.01962** | Post-Training Ternarization of Qwen3-4B | 09-02T00:42Z (walk-#6 win) | **ternary / quant — OUR LANE** | **B− — doctrine citation** | End-to-end PTQ study (KOTMS rotation + E2M-ATQ + GPTQ-style compensation, weight-only W1.58/A16): 1.641 **effective** bits/weight over 81.62% of params; aggregate accuracy 64.5% → 54.7% but **uneven per-task**: BoolQ retains 84.6% chance-corrected teacher performance vs ARC-Challenge 43.8%; PPL 13.639 → 18.748 (WT2); a packing run 8.29 → 3.96 GiB with essentially unchanged PPL (lossless packing preserving ternary planes + scales), while a third-party packing attempt was LOSSY and was excluded from the artifact claim. | Cite at the **lossy-surface promotion rule** (katgpt-rs AGENTS.md): the per-task spread (84.6% vs 43.8% retention under a modest aggregate drop) is a clean external confirmation of "aggregate perplexity can be flat-ish while family-conditional behavior flips" (Issue 750 T3) — third independent arrival (Research 502 / Bench 696 / this). The effective-bit accounting (1.641 vs nominal 1.58) is the honest-accounting analog for our ternary GGUF format claims; the lossless-vs-lossy packing split echoes the BLAKE3-committed packing discipline. Not a kernel rule — no bounded fix-space. |
| 2 | **2609.02029** | HeadWiseKV: Budgeted Per-Head Cache Residency for Hybrid LMs | 09-02T02:59Z (walk-#6 win) | **KV / GDN-hybrid lane** | **B− — lane record** | Training-free per-head STATIC multilevel history windows for hybrid models' residual global-attention layers; rate–distortion allocation (SeqCalib, depth-ordered, conditioned on the deployed lower-layer policy); the runtime **materializes per-head physical residency rather than masking a full cache**; near-FullKV quality on 4 hybrids + Qwen3.6-27B serving. | Lane record only, with the **boundary note: our KV caches are full-length append-only by pinned contract (Issue 746 — no evict/trim/rotate APIs)**, so per-head eviction/residency policies are structurally out-of-stack unless that contract changes. The "materialize residency, not a mask" mechanism is the kernel-shaped nugget worth re-reading if the KV contract ever opens (pairs with the B58/B100 family's predictability-before-serving instinct). |
| 3 | 2609.02027 | Multi-Turn LLM Conversations under LRU: Mean-Field Asymptotics | 09-02T02:57Z (walk-#6 win) | KV theory (cs.PF) | C+ — lane record | Mean-field limit of the multi-turn KV hit ratio under LRU + a practical estimator; capacity-provisioning guideline (validated on Qwen3-8B/Ascend). | Theory backing for serving-capacity math; same Issue-746 boundary as row 2 — no in-stack fix-space. |
| 4 | **2609.02846** | UE5M3 FP4 Block Scaling for Stable LM Pretraining | 09-02T17:32Z (walk-#6 win) | quant format (pretraining-side) | **C+/B− — doctrine citation** | Pairs E2M1 payloads with **unsigned E5M3 block scales** (wider range ⇒ periodic tensor scaling suffices), drops the randomized Hadamard transform AND the BF16 final-layer exemption; selective stochastic rounding on backward; −21.2% is wrong direction — +21.2% model-body token throughput from the joint removal ablation, with lower final loss than TE's NVFP4 recipe at ~190B tokens (Nemotron-H 8B). | **Rotation-tax echo, recipe-level**: removing the RHT entirely and paying with a wider scale FORMAT won on both loss and throughput — external confirmation of the rotation-tax doctrine alongside walk #6's hardware echoes (LightRot/GyRot, C+ residue) and our own Bench-716 Hadamard-sink refutation. Also a second data point that the scale-FORMAT axis (Issue 845 PoT = narrow-deterministic; UE5M3 = wide-range) is where outlier machinery gets bought or refunded. Pretraining-side, FP4 — no direct corpus rule. |
| 5 | 2609.02052 | SchedBlame: Culprit-Attributed CPU Contention for Containers | 09-02T03:29Z (walk-#6 win) | ops / measurement doctrine | C+ — lane record | eBPF tracer attributing CPU contention to the CAUSING cgroup on stock kernels (inverts victim-side PSI/wait counters: charges each run slice to a competitor×victim blame bitmap row, 16 B/record, no per-pair kernel state). | The principled version of our contaminator gates: Bench-649 GPU-exclusivity + the league's `watch_midrun` are victim-side heuristics (process-name/ancestry matching); SchedBlame is culprit-side accounting. eBPF/Linux — neither of our boxes runs it; cite in the perf-league doc at next touch as the named ideal our gate approximates. |
| 6 | 2609.02737 | Language Models Can Control Their Own Attention | 09-02T15:43Z (walk-#6 win) | attention/cognition | C — not-mineable | Self-control of attention (steering lane). | none |
| 7 | 2609.02780 | ShallowStream: Index Shallow then Answer Deep (streaming video) | 09-02T16:14Z (walk-#6 win) | KV query (off-lane hit) | C — not-mineable | Streaming video serving; matched the broad `all:"KV cache"` query only. | none (documents that the broadened KV lane pulls off-lane rows — acceptable recall/cost tradeoff, one row) |
| 8 | 2609.01834 | Hydration Proxy Pattern (stateless LLM APIs) | 09-01T20:08Z (walk-#6 win) | serving systems | C — not-mineable | Conversational data-systems architecture. | none |
| 9 | 2609.02264 | Codebook Agent: Amortized Topology Design for LLM MAS | 09-02T08:10Z (walk-#6 win) | multi-agent | C — not-mineable | Topology design amortization. | none |
| 10 | 2609.02219 | HW-Accelerated Instance Segmentation (space robotics) | 09-02T07:33Z (walk-#6 win) | quant query (edge, off-lane) | C — not ours | Edge CV acceleration. | none |
| 11 | 2609.02320 | Triggered Packet Streams (matrix-analytic) | 09-02T09:05Z (walk-#6 win) | cs.PF listing | C — not-mineable | Queueing theory. | none |

AceSpec (2609.02514) was walk #6's own strict-window row — unchanged, C.

## Shortlist (coordinator decisions)

- **No batch.** Rows 1/4 are citations, row 2 is a lane record behind a pinned
  contract boundary, rows 3/5 are C+ records. Nothing has a bounded fix-space
  in our kernels.
- **Citations landed this walk** (the queued "next doc-sync touch" from walk #6
  executed together with the new ones, one docs commit):
  - 2608.12700 (contract-grade verifier, walk #6 A− doctrine row) →
    katgpt-rs AGENTS.md **lossy-surface promotion rule** paragraph (the
    tolerance-budget external confirmation it was queued for).
  - 2609.01962 → same paragraph (per-task retention spread as the
    aggregate-vs-family external).
  - 2609.02846 → the same docs commit's rotation-tax note (queue snapshot is
    the canonical record; AGENTS.md stays rule-shaped, no paper list bloat).
- **Queue-snapshot updates:** walk #7 DONE entry (this file), next window
  **Mon 09-07 ~07:00 +07** (Sun 09-06 20:00 ET), and the standing-query
  broadening (KV lane = plain `all:"KV cache"`; quant lane stays
  quant+inference but pretraining-format rows are checked via a second
  `abs:"block scaling" OR abs:quantization` sweep when the window is small) —
  adopted because the narrow queries caused this backfill.

## Numbering note

`.research/.highwater` read **123** at walk time while `124_batch103_*` exists
on disk (the stale-highwater class — landed without the bump). This walk takes
**125** and writes highwater = 125.
