# 072 — MTPLX PR 391 distill batch (W87-led, kernel_opt D, 2–3 rules + riders)

**Status:** COMPLETE — Batch 103 landed 2026-09-04 (4 rules, corpus 459 → 463, D +4, + the B100 rider enrichment; T7 COMBINE executed; record: `.research/124_batch103_mtplx_nax_combined.md`; re-pin `a5e38bb`) + **two same-day addenda from the Issue-665 collision salvage** (the duplicate session yielded push-wins, then landed what the batch lacked): the silently-omitted MLX #4416 rule `dispatcher-eligibility-predicate-lags-kernel-capability` (corpus 463 → 464, D 102 → 103, `07fa0df`) + the mask-fuse cache facility rider on the shape-class rule (`8bbd59e`); R124 §addendum has the narrative

## Source + pin

- youssofal/MTPLX PR #391 ("perf(qwen4): restack Flash-Next optimizations on MTPLX 2.10.1"), fork **davidtai/MTPLX-1**, branch `port/qwen38-flash-next-210-restack`.
- Pin at verdict time: head `ef9631324766b3aa09de849a795bd6f3d40532ed` (Apache-2.0 both trees; OPEN / unmerged; mergeable_state dirty). **Re-pin at T0** — the fork is live (pushed 2026-09-03).
- Verdict record: [`.research/122_mtplx_pr391_distill_verdict.md`](../.research/122_mtplx_pr391_distill_verdict.md).

## Tasks

- [x] T0 — Re-pin + anchor re-verification: fetch the PR head sha; if it moved since `ef96313`, diff the touched files and re-verify every quoted anchor (the W87 W-number attribution, the `q_len × gqa_factor ≤ 32` dead-band numbers, the "hard-coded draft width" production note, the W54 19.2% ceiling arithmetic, the W68 two-outage corrections). Record the new pin here + in the research note. **DONE: pin ef96313 → a5e38bb (branch rewrite); all anchors re-verified — W87 NOT re-confirmable (carried honestly), hard-coded-draft-width re-anchored to the wiring test, dead-band + 19.2% + two-outage all verified byte-accurate at the new head.**
- [x] T1 — Batch-number allocation at batch start (next after B102 lands); do not pre-allocate in this file. Re-scan `.plans/` at write time; **071 is RESERVED for the flashquant renumber** per the snapshot collision flag — never reuse. **DONE: Batch 103.**
- [x] T2 — `shape-class-keyed-capability` (kernel_opt D, A−): a capability verdict must be keyed by the FULL shape tuple the native gate reads and re-asked per call; never persisted across shape classes. Decision content: the dead-band mechanism (probe sampled only the warmup shapes), the process-wide disarm, the warmup-ladder-before-first-wide-chunk ordering. Original Rust fixtures (host-side gating shape: a cached capability bool consulted across heterogeneous launch shapes).
- [x] T3 — `wavefront-overlap-pair-ceiling` (kernel_opt D/E, B): bound the overlap-pair count BEFORE building a multi-stream seam (2 streams × 2 stages ⇒ 1 overlappable pair ⇒ 19.2% ceiling; measured 0.4% e2e vs the 15% funding gate). Encode the ceiling arithmetic + the funding-gate refusal as decision content.
- [x] T4 — `request-shape-decline-mirror-completeness` (kernel_opt D, B): a routed fast path must mirror EVERY request-dependent decline branch of the native contract; named decline sites + counted reasons; install-time binding raise (a flag that cannot bind raises at install, never per request). Mirror-test fixture over a declared decline table.
- [x] T5 — Graph-build overlap (W67) evaluate-then-likely-drop: audit our residual exposed-host enqueue window (post Bench-761/771 caches). Expected drop with reason; rule only on a measured exposed window. **DONE: DROPPED — our caches consumed the seam (submits 577 → 1.98/token, decode at the ~31 ms GPU floor); no measured exposed window; reopen trigger recorded in Research 124.**
- [x] T6 — Riders (no corpus delta): Bench 805 record + external mask-fuse confirmation line; Issue 734 record + regime note (their −0.35 s is vs unblocked-eager stock, ours already batched); Bench 641/B77 + expert-major micro-vs-e2e external confirmation. **DONE — research-note records in Research 124 §T6.**
- [x] T7 — Merge decision vs the NAX lead: the queue snapshot's `dispatcher-eligibility-predicate-lags-kernel-capability` lead is ALSO gated behind B102. Decide at batch start: one combined batch (one corpus cycle, one GOAT run, one count-pin walk) or two. Default: **COMBINE** (corpus-cycle economy); the combined batch references both research notes. **DONE: COMBINE executed — the NAX backing-provenance rule + the B100 rider landed in the same corpus cycle; the dispatcher-predicate lead discharged as the engagement-canary complement inside the request-shape rule's fix-space (no separate rule — the mechanism is our own standing GOAT practice; see Research 124).**
- [x] T8 — GOAT: `batchNN_pseudocode_parses` (cfg(not(kernel_verify))) + `rules_present` + `concepts_covered` + kernel_opt lib green + retrieval floors HELD (no re-pins) + clippy 0 both states. **DONE: 3/3 gates + kernel_opt 1285/0/1i + default 1029/0 + kernel_opt+latent 1392/0/2i (floors HELD) + clippy 0 in three states.**
- [x] T9 — Count pins: tests.rs corpus-count + expected-array (D, +E if T3 lands there) + category sums ×2 + grouped + router.rs expert total + mod.rs doc line + category headers. Walk the FULL list — stale headers are the norm, not the exception (the 18th-count-pin-site lesson). **DONE: full walk + en-route rot fix (the 15 mod.rs category header comments were stale by up to 2 batches, pinned by no test — all 15 now match the pinned table).**
- [x] T10 — ONE commit: research note status → RECORD (batch landed) + AGENTS.md batch lines + corpus + gates + snapshot fold. Dedup pass documented: every near-neighbor in the D-category host-orchestration family read in full. **DONE (commit at push).**

## Drop ledger (pre-declared — do not re-litigate at batch time without new evidence)

mincore read-skip (their own W62 negative), route kernel, draft K20 pre-scatter, FR-Spec (spec-decode closed-negative in-stack: Issues 717/721/737), sparse split-K decode (Bench 763: attention ≈ 0% share), GDN keep-mask fold (their measured LOSS), oMLX keepwarm (they falsified it themselves).

## Residue (added 2026-09-04, after the batch closed)

This plan's population was the PR #391 diff + body only. The SAME source has three unmined
surfaces — upstream `main` (196 commits / 300 files ahead of the base `8bc4d88`; v2.10.2 + v2.11.1
released since), the 16-file `mistakes/` post-mortem corpus (the vendor clone carries only 9), and
`docs/{specs,plans,concurrency}` — indexed with grades, anchors and the B+ lead
(`prefix-reuse-key-must-mirror-the-restore-predicate`, TWO instances, and the first MTPLX residue
row WITH an in-stack analog: Issue 742's prefix cache) in
[`.research/122`](../.research/122_mtplx_pr391_distill_verdict.md) §Residue. Execution takes its
own verdict + pin (Research 129 / Plan 076) — do NOT widen this plan's pin.

## Non-goals

- **No prefill-throughput claim from this batch.** The verdict is explicit: none of the retained levers transfers (all HAVE / refuted / no-analog). This batch buys corpus rules, not tok/s.
- **No league action.** Their 1,332–1,638 tok/s @16K is M4-class silicon + a finer-quant MoE pack — a mining source, not an opponent row.

## Standing discipline

- Plan-first (Protocol A step 0); ONE commit per batch; vendor tree (if cloned) is read-only — `git show`/shallow fetch or `../vendor/` clone, never modified.
- Original fixtures only; no source bytes copied into the corpus.
- kernel_opt corpus contention: all corpus edits land AFTER B102's commit lands on develop.
