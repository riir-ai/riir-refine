# 122 — MTPLX PR 391 distill verdict — youssofal/MTPLX Qwen3.8-Flash-Next restack (davidtai)

**Status:** RECORD — verdict closed; execution queued behind B102 (Plan 072; no batch landed at filing)

## Source

- **PR:** [youssofal/MTPLX#391](https://github.com/youssofal/MTPLX/pull/391) — "perf(qwen4): restack Flash-Next optimizations on MTPLX 2.10.1" (davidtai; 999 commits; OPEN / **unmerged**, mergeable_state dirty).
- **Pin:** head `ef9631324766b3aa09de849a795bd6f3d40532ed` (fork **davidtai/MTPLX-1**, branch `port/qwen38-flash-next-210-restack`; fork pushed 2026-09-03, PR body updated 2026-09-03 — pin re-verified at filing via the GitHub API). Mine from the FORK, not upstream.
- **License:** Apache-2.0 on BOTH fork and upstream (verified at pin time).
- **Base:** youssofal/MTPLX main @ `8bc4d88e421d68a9b9e29edc75ff14ed3b81bfef`.
- **Provenance note:** a DIFFERENT source from oMLX (`jundot/omlx`, Batches 71/79/84/94) — youssofal is the model author; davidtai is a contributor restacking the Qwen3.8-Flash-Next model. Distinct measurement culture, distinct corpus surface.

## Verdict

**YES as a mining source (grade B) — MARGINAL for our stated goal.** Distilling this PR will not make our prefill faster: every retained prefill lever is already shipped or already measured-refuted in-stack. The honest yield is **2–3 host-orchestration kernel_opt rules** (W87-led) + confirmation riders. No prefill-throughput claim may be filed from this batch.

Not league-comparable: their 1,332–1,638 tok/s @16K prefill is M4-class silicon + a finer-quant MoE hybrid pack (137 GB vs mlx-serve's 98 GB uniform 4-bit) — a mining source, not an opponent row.

## The retained prefill levers, mapped against our stack

| Their lever | Their gain | Our status |
|---|---|---|
| Causal fused SDPA (`mask="causal"`, W48) | +3.5% @16K / +6.2% @32K; peak 92.1→89.4 / 97.1→92.0 GB | **HAVE** — Bench 805 tiled flash prefill, default-on, same win shape |
| Prefill chunk 4096 + compile-rows lockstep (W30) | −0.69 s (−5.1%) @16K; peak 92.7 GB | Wider-chunk **refuted** in-stack (Bench 710: 8192 = 0.926×); the `MTPLX_QSA_PREFILL_COMPILE_ROWS` lockstep is MLX compiled-graph coupling, not ours |
| QSA prefill query tile 2048 (W32) | −0.29 s stacked | **HAVE equivalent** — our flash tiling owns the query axis |
| GDN blocked prefill (W31/37) | −0.37 s alone / −0.35 s stacked | **Measured negative** in-stack (Issue 734: chunked recurrence 0.97–0.98×; state-dependent work dominates 6:1). Regime note: their stock runs unblocked-eager GDN prefill, ours is already batched — do not cite as contradiction |
| PLE lookahead + early first-chunk gather (W22/W46) | −2.0 s of 14.7 s @2K-chunks; gather 0.62→0.013 s | **No analog** — model-specific n-gram sidecar + per-request disk gather; we have neither |
| Sparse prefill lane forced @16K (W19) | +7..+36% SLOWER (their own data) | n/a — confirms staying dense below ~32K, which we are |
| SessionBank 8 GiB prefix restore (W20) | TTFT 15.9→0.53 s (3/3) | **HAVE** — GDN+KV prefix cache, 441× TTFT (Issue 742) |
| Expert-major MoE prefill (W53) | micro 1.94× @R=320, e2e **−3.6%**, peak +1.44 GB | No MoE in Bonsai; external confirmation of the micro-vs-e2e class (Bench 641 record class) |

## Mineable candidates (what the batch takes)

| Candidate | Route | Grade | Mechanism |
|---|---|---|---|
| `shape-class-keyed-capability` | kernel_opt D | **A−** (GOAT lead) | A probe refusal at ONE shape disarmed the flag process-wide: MLX's vector kernel caps `q_len × gqa_factor` at 32; their GQA-12/256-dim geometry makes a dead band at q_len 3..8; the warmup ladder (verify steps) ran before the first wide chunk, so the capability probe sampled only the dead band. Fix: key the capability by the FULL shape tuple the native gate reads and re-ask per call — never persist a capability verdict across shape classes. The PR's own production notes name the mechanism: "the fixed-capacity verify path asked the route with a hard-coded draft width, so it always fell through to the gather path." (W-number attribution re-verify at mine time — the updated PR body no longer names W87 explicitly.) |
| `wavefront-overlap-pair-ceiling` | kernel_opt D/E | B | Anti-pattern WITH arithmetic: a 2-stream × 2-stage wavefront (W54) has exactly ONE overlappable pair = 19.2% tile ceiling; measured micro 0.4% e2e vs the 15% funding gate → bound the pair count BEFORE building the seam. Encodes the ceiling formula + the funding-gate refusal as decision content — the Bench-801/803 bound-first doctrine in kernel-shaped form. |
| `request-shape-decline-mirror-completeness` (W68) | kernel_opt D | B | A routed fast path must mirror EVERY request-dependent decline branch of the native contract — two HTTP-500 outages root-caused (sparse lane raised on short contexts <2,052 tokens; pre-scatter raised on request-time ineligibility). Fix: routing separated from failure, named decline sites with counted reasons, install-time binding raise. Complements the NAX lead's engagement-canary defense: canaries prove the fast path RAN; the mirror test proves it declines exactly where the native path would. |
| Graph-build overlap (W67) | kernel_opt D | C+ | −1.5% decode cycle (host enqueues layers 0–2 of the verify graph while the GPU replays the rest). Evaluate-then-likely-drop: our residual host cost post Bench-761/771 caches is small; rule only if the batch's host-cost audit finds a real exposed enqueue window. |

**Yield estimate: 2–3 rules** after coordinator falsification — a normal batch. Plus riders (no corpus delta): mask-fuse → Bench 805 external confirmation; GDN blocked → Issue 734 regime note; expert-major → Bench 641/B77 micro-vs-e2e external confirmation.

## Drops (with reasons)

- **mincore read-skip** — their own W62 negative (resident fraction 0.11–0.13, +0.16 ms/cycle): the read pass is real work; nothing for us to skip (we have no per-request disk gather at all).
- **Route kernel / draft K20 pre-scatter / FR-Spec** — no router, no draft head in Bonsai; spec-decode closed-negative in-stack (Issues 717/721/737).
- **Sparse split-K decode attention** — our decode attention ≈ 0% share (Bench 763); below any funding bar.
- **GDN keep-mask fold** — their own measured LOSS (+4.2% cycle, W66): an anti-instance recorded in the source, not a candidate.
- **oMLX keepwarm port** — they falsified it themselves (W16 audit: MTPLX already has stronger equivalents).

## Measurement culture (league rider)

The PR **withdrew its preliminary cross-engine table** after forensics showed the upstream venv's editable install resolved `mtplx` to whichever tree the process STARTED in — receipts carried `server_cwd: null`, health payloads were indistinguishable (146 identical keys), and the measured delta was consistent with either tree. The battery re-ran with per-engine working directories + per-receipt engine versions; the harness now refuses to launch without an explicit cwd. **Independent external confirmation of the FairnessManifest's "opponent repo + commit" field** — recorded as rule 11 in the league doc (riir-ai, filed with this note). Their ABBA arm order / 40 °C thermal gate / receipts discipline also independently validates our protocol; noted, not actionable.

## Caveats

- **Unmerged PR** — mine at the pinned head sha of the FORK branch; re-pin at batch T0 (the fork is live: pushed 2026-09-03).
- `kernel_opt` corpus is HOT: **B102 FlashQuant in flight in a sibling lane** — corpus edits queue behind it (Plan 072's gate).
- Mostly Python/MLX host orchestration; rules re-expressed with ORIGINAL Rust fixtures per the standing discipline; all anchors re-verified at mine time (T0).
- W87's W-number is verdict-session attribution; the mechanism itself is confirmed by the PR body's production notes ("hard-coded draft width ... always fell through") — re-verify the anchor at mine time.

## Bottom line

The real prefill levers for us remain exactly where the league already points: **Issue 771 (GEMM 2.89 TF vs the ≥5.4 gate; llama.cpp-proven 6.8 TF on this silicon)** and **Issue 844 (flash-vs-GEMM long-P prize)** — this PR touches neither. Distill as a small W87-led batch queued behind B102 for the capability-keying rule + the decline-mirror rule + riders.
