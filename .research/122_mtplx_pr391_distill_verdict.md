# 122 — MTPLX PR 391 distill verdict — youssofal/MTPLX Qwen3.8-Flash-Next restack (davidtai)

**Status:** RECORD — verdict CONSUMED by **Batch 103** (2026-09-04, Research 124 / Plan 072: 4 rules + the same-day addendum rule; mine pin `a5e38bb`). **SCOPE-CORRECTED + RESIDUE-INDEXED 2026-09-04** — the status line above read "queued behind B102 / no batch landed" for a day after the batch landed, and the title reads as a verdict over the SOURCE while the population was one PR diff; both fixed below (§Scope, §Post-batch disposition, §Residue).

## Source

- **PR:** [youssofal/MTPLX#391](https://github.com/youssofal/MTPLX/pull/391) — "perf(qwen4): restack Flash-Next optimizations on MTPLX 2.10.1" (davidtai; 999 commits; OPEN / **unmerged**, mergeable_state dirty).
- **Pin:** head `ef9631324766b3aa09de849a795bd6f3d40532ed` (fork **davidtai/MTPLX-1**, branch `port/qwen38-flash-next-210-restack`; fork pushed 2026-09-03, PR body updated 2026-09-03 — pin re-verified at filing via the GitHub API). Mine from the FORK, not upstream.
- **License:** Apache-2.0 on BOTH fork and upstream (verified at pin time).
- **Base:** youssofal/MTPLX main @ `8bc4d88e421d68a9b9e29edc75ff14ed3b81bfef`.
- **Provenance note:** a DIFFERENT source from oMLX (`jundot/omlx`, Batches 71/79/84/94) — youssofal is the model author; davidtai is a contributor restacking the Qwen3.8-Flash-Next model. Distinct measurement culture, distinct corpus surface.

## Scope — the population this verdict read

State the population or a narrow verdict reads as a whole-source verdict (the workspace's
own stated-population rule, `katgpt-rs` AGENTS.md §cfg-gated / §percentile classifiers).

**READ:** the PR #391 diff + PR body + the retained kernels/tests reachable from it, at the
fork head. **NOT READ:** upstream `main` itself (the base `8bc4d88` was named as a base, never
mined), the source's `mistakes/` post-mortem corpus, `docs/specs`, `docs/plans`,
`docs/concurrency`, `CHANGELOG.md`, `HISTORY.md`. Measured 2026-09-04: a grep of THIS note and
Research 124 for `mistakes/|MEASUREMENTS|docs/specs|docs/research|docs/concurrency|turbo-verify|
superpowers|runtime-contract|HISTORY.md` returns **zero** hits; the single `docs/perf` hit is
Research 124 recording those charts as DELETED by the branch rewrite. Everything in §Verdict
below — including "MARGINAL for our stated goal" — is a claim about the PR, not about MTPLX.

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

## Post-batch disposition (2026-09-04)

- **Batch 103 landed** — 4 rules, corpus 459 → 463 (D +4) + the B100 rider enrichment; T7 COMBINE
  executed with the MLX NAX lead. Same-day **addendum** (`07fa0df`): the silently-omitted
  `dispatcher-eligibility-predicate-lags-kernel-capability` rule, corpus 463 → 464.
- **Pin moved at T0:** `ef96313` → **`a5e38bb`** (branch rewrite). Two verdict-time anchors did
  NOT survive and are carried honestly in-rule: the W87 W-number attribution and the
  "hard-coded draft width" body note (re-anchored to the wiring test). The dead band, the 19.2%
  ceiling and the two-outage history all VERIFIED byte-accurate. Full ledger: Research 124 §T0.
- **W67 graph-build overlap: DROPPED** with reason — promoted default-on at their new head
  (−1.5% cycle), but our Bench-761/771 caches consumed the same seam (submits 577 → 1.98/token,
  M3 decode at the ~31 ms GPU floor). No exposed host-enqueue window remains.
- The three §Drops and both §Non-goals held; no prefill-throughput claim was filed.

## Residue — unmined surfaces of the SAME source (measured 2026-09-04)

Not a new verdict: this section is the **index** so the residue is findable. When executed it
takes its OWN verdict + pin (see §Next step) — do not widen this record's pin.

| Axis | Size (measured) | Grade | Note |
|---|---|---|---|
| **A. upstream `main` — never mined** | **196 commits / 300 files** ahead of the mined base `8bc4d88` (`behind_by` 0); releases v2.10.2 (09-01) + v2.11.1 (09-04, "MTPLX 2.11"); pushed 2026-09-04T11:05Z; head `dd4031a8f9ed9d0a532b6e16bcc147dc40399c1d` (10:25Z) | **B+** | The lead is below |
| **B. `mistakes/` post-mortem corpus** | **16 files upstream** at `dd4031a8`; the vendor clone at `a5e38bb` carries **9** (7 upstream-only, 0 fork-only) — the fork tree under-represents this surface by 78% | **B** (best confidence-per-token) | Filenames ARE the rules (pre-extracted); ~4 of 9 read as confirmations of faults we already hit, 2 novel |
| **C. design/spec docs, unread** | `docs/plans` 1217 · `docs/specs` 757 · `docs/concurrency` 259 · `CHANGELOG` 2613 lines | **C+/B−** | The two Qwen3.5-MTP batch-numerics design docs + `concurrency/qwen35b-mtp-batch.md` are plausible kernel_opt D leads; `docs/perf/pr391-battery-*` (1390 lines) is receipts → riders at most |

### The lead — and it has an in-stack analog, unlike every prefill lever above

`prefix-reuse-key-must-mirror-the-restore-predicate` (working name; kernel_opt D, sibling of the
two rules this batch landed). **TWO independent instances in one source**, which is what lifts it
above a single-instance C lead:

1. Upstream HEAD `dd4031a8` — "The 2.10.2 admission shed (#415) asked SessionBank.longest_prefix,
   exact containment only", while the restore path serves by *block* prefix; a tool-round
   follow-up sharing 41k tokens read as a full miss, so "the shed read a full miss, cleared the
   session, and the release gate's tool_result_forced turn re-prefilled 41,901 tokens cold (54 s)
   with a 41,391-token restore available." Fix: ask a second, block-aligned probe
   (`longest_shared_prefix_tokens`) and keep the larger figure.
2. `mistakes/per-request-steering-that-rides-a-trailing-sentinel-was-still-keyed-into-the-bank-fingerprint-...` —
   the mirror-image direction: a reuse key encoding something that does NOT change KV, so every
   transition round re-prefilled cold. Their rule: only things that change KV without changing
   tokens belong in the fingerprint.

Same shape as the landed `shape-class-keyed-capability` (a verdict valid only for the tuple the
native gate reads) one layer up. **Our analog exists**: Issue 742's GDN+KV prefix cache (441×
TTFT) — so unlike §"retained prefill levers", this residue is potentially actionable rather than
HAVE / refuted / no-analog. Verify at mine time; grade B+ pending.

### The 7 upstream-only `mistakes/` files (absent from the vendor clone)

- `a-benchmark-arm-assertion-shipped-in-a-default-on-lane-and-returned-finish-reason-error-to-a-user-after-a-30s-prefill-...` — a benchmark assertion in a default-on serving lane; "verdicts are receipts in serving, and raises only behind a strict flag". Direct read on our feature-flag + GOAT-gate discipline.
- `per-request-steering-...-bank-fingerprint-...` — instance 2 of the lead above.
- `a-request-arrival-optimisation-that-predicted-from-the-prompt-alone-fired-on-every-warm-bank-turn-and-blocked-the-owner-thread-on-work-no-prefill-consumed-...` — speculative-prefetch anti-pattern: ask the bank first, never await unconsumed work (bound-first doctrine, Bench 801/803).
- `git-stash-is-shared-by-every-worktree-so-two-lanes-stashing-at-once-swapped-each-others-work-swap-files-in-with-git-show-copies-instead` — **independent external confirmation of our own standing rule** (AGENTS.md: never stash files you did not modify this session) *and* of our fix pattern (commit YOUR blob via `git show` / `hash-object` + `update-index`, per `staged_set_audit.py`). Discipline rider, not kernel_opt.
- `pytest-tests-rewrote-the-founders-real-opencode-config-mid-run-with-a-fixture-model-id-every-external-config-path-the-code-can-write-gets-an-autouse-scratch-override-in-conftest` — tests writing real external config paths (our TMPDIR/libsql class).
- `an-isolated-onboarding-qa-run-started-a-real-homebrew-upgrade-use-injected-no-op-installers-or-stop-before-the-setup-step`.
- `when-the-user-forbids-app-launches-stop-ui-qa-immediately-and-use-static-build-and-test-checks-only`.

### The one novel measurement rule in the 9 already-cloned files

`mistakes/mean-tps-and-sliding-averages-hid-thirty-sub-second-emit-silences-per-turn-...` — felt
speed is the **inter-emit gap distribution**, not the mean: "An earlier wire probe had already
measured 0.51 s read gaps and was WRONGLY read as exoneration because mean throughput held",
while the app-side census exonerated the app (apply p95 0.17 ms, 3 stalls in 4 min) and "22-30
gaps ≥200 ms per turn arrived with only 3-11 BYTES waiting behind them: the GENERATOR itself went
silent". Fix: every request record carries `producer_gap_ms_p95/max` +
`producer_gaps_over_200ms`, and the QA gate stopped failing only on gaps >2 s. This is the
pooled-vs-split lesson (`katgpt-rs/.docs/10_audits/percentile_index_tail_support.md`) on an axis
we do not census at all — we record mean and p99 *latency*, never delivered-cadence silence.
Second novel one: `silently-stripping-dead-tool-call-markup-...` — suppression must always leave
a visible truthful notice (the 3-state-wrapper class). The other ~4 are confirmations, not rules:
stale-binary A/B via a duplicate bundle ID, piped build output masking a failed build then
A/B-testing the stale product, a hand-rebuilt state key drifting from the save side, and
hands-off automation not seeing interaction bugs.

### Next step (not taken here)

File a NEW verdict pinned at upstream `main` (Research **129**) + Plan **076**, mining axes A+B in
one corpus cycle per the B103/B105 COMBINE precedent, axis C read for riders only.
**Blockers/notes:** the vendor clone is **depth-1 shallow** (`.git/shallow` = `a5e38bb` +
`ef96313`) so it cannot serve axis A — a fresh pinned upstream clone is required;
`kernel_opt` is cadence-gated behind Batch 105's 8 deferred uzu/lily candidates; and
`.research/.highwater` reads **127** while `128` exists (the stale-highwater class, same as the
Plan-073 catch in Batch 104) — repair at allocation.

## Bottom line

The real prefill levers for us remain exactly where the league already points: **Issue 771 (GEMM 2.89 TF vs the ≥5.4 gate; llama.cpp-proven 6.8 TF on this silicon)** and **Issue 844 (flash-vs-GEMM long-P prize)** — this PR touches neither. Distill as a small W87-led batch queued behind B102 for the capability-keying rule + the decline-mirror rule + riders.

**Refined 2026-09-04:** the prefill conclusion above STANDS unchanged — Issue 771 and Issue 844
are still the only real prefill levers, and this PR touched neither. What changed is the scope
claim: this record covered one PR diff, and the source's larger surface (§Residue) is unmined,
with one lead that — unlike every row in the levers table — has an in-stack analog (Issue 742).
