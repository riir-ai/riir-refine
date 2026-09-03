# 072 — MTPLX PR 391 distill batch (W87-led, kernel_opt D, 2–3 rules + riders)

**Status:** QUEUED — execution gated behind B102 FlashQuant (the sibling lane owns the kernel_opt corpus until it lands); see `.distill/001_mining_queue_snapshot.md`

## Source + pin

- youssofal/MTPLX PR #391 ("perf(qwen4): restack Flash-Next optimizations on MTPLX 2.10.1"), fork **davidtai/MTPLX-1**, branch `port/qwen38-flash-next-210-restack`.
- Pin at verdict time: head `ef9631324766b3aa09de849a795bd6f3d40532ed` (Apache-2.0 both trees; OPEN / unmerged; mergeable_state dirty). **Re-pin at T0** — the fork is live (pushed 2026-09-03).
- Verdict record: [`.research/122_mtplx_pr391_distill_verdict.md`](../.research/122_mtplx_pr391_distill_verdict.md).

## Tasks

- [ ] T0 — Re-pin + anchor re-verification: fetch the PR head sha; if it moved since `ef96313`, diff the touched files and re-verify every quoted anchor (the W87 W-number attribution, the `q_len × gqa_factor ≤ 32` dead-band numbers, the "hard-coded draft width" production note, the W54 19.2% ceiling arithmetic, the W68 two-outage corrections). Record the new pin here + in the research note.
- [ ] T1 — Batch-number allocation at batch start (next after B102 lands); do not pre-allocate in this file. Re-scan `.plans/` at write time; **071 is RESERVED for the flashquant renumber** per the snapshot collision flag — never reuse.
- [ ] T2 — `shape-class-keyed-capability` (kernel_opt D, A−): a capability verdict must be keyed by the FULL shape tuple the native gate reads and re-asked per call; never persisted across shape classes. Decision content: the dead-band mechanism (probe sampled only the warmup shapes), the process-wide disarm, the warmup-ladder-before-first-wide-chunk ordering. Original Rust fixtures (host-side gating shape: a cached capability bool consulted across heterogeneous launch shapes).
- [ ] T3 — `wavefront-overlap-pair-ceiling` (kernel_opt D/E, B): bound the overlap-pair count BEFORE building a multi-stream seam (2 streams × 2 stages ⇒ 1 overlappable pair ⇒ 19.2% ceiling; measured 0.4% e2e vs the 15% funding gate). Encode the ceiling arithmetic + the funding-gate refusal as decision content.
- [ ] T4 — `request-shape-decline-mirror-completeness` (kernel_opt D, B): a routed fast path must mirror EVERY request-dependent decline branch of the native contract; named decline sites + counted reasons; install-time binding raise (a flag that cannot bind raises at install, never per request). Mirror-test fixture over a declared decline table.
- [ ] T5 — Graph-build overlap (W67) evaluate-then-likely-drop: audit our residual exposed-host enqueue window (post Bench-761/771 caches). Expected drop with reason; rule only on a measured exposed window.
- [ ] T6 — Riders (no corpus delta): Bench 805 record + external mask-fuse confirmation line; Issue 734 record + regime note (their −0.35 s is vs unblocked-eager stock, ours already batched); Bench 641/B77 + expert-major micro-vs-e2e external confirmation.
- [ ] T7 — Merge decision vs the NAX lead: the queue snapshot's `dispatcher-eligibility-predicate-lags-kernel-capability` lead is ALSO gated behind B102. Decide at batch start: one combined batch (one corpus cycle, one GOAT run, one count-pin walk) or two. Default: **COMBINE** (corpus-cycle economy); the combined batch references both research notes.
- [ ] T8 — GOAT: `batchNN_pseudocode_parses` (cfg(not(kernel_verify))) + `rules_present` + `concepts_covered` + kernel_heal lib green + retrieval floors HELD (no re-pins) + clippy 0 both states.
- [ ] T9 — Count pins: tests.rs corpus-count + expected-array (D, +E if T3 lands there) + category sums ×2 + grouped + router.rs expert total + mod.rs doc line + category headers. Walk the FULL list — stale headers are the norm, not the exception (the 18th-count-pin-site lesson).
- [ ] T10 — ONE commit: research note status → RECORD (batch landed) + AGENTS.md batch lines + corpus + gates + snapshot fold. Dedup pass documented: every near-neighbor in the D-category host-orchestration family read in full.

## Drop ledger (pre-declared — do not re-litigate at batch time without new evidence)

mincore read-skip (their own W62 negative), route kernel, draft K20 pre-scatter, FR-Spec (spec-decode closed-negative in-stack: Issues 717/721/737), sparse split-K decode (Bench 763: attention ≈ 0% share), GDN keep-mask fold (their measured LOSS), oMLX keepwarm (they falsified it themselves).

## Non-goals

- **No prefill-throughput claim from this batch.** The verdict is explicit: none of the retained levers transfers (all HAVE / refuted / no-analog). This batch buys corpus rules, not tok/s.
- **No league action.** Their 1,332–1,638 tok/s @16K is M4-class silicon + a finer-quant MoE pack — a mining source, not an opponent row.

## Standing discipline

- Plan-first (Protocol A step 0); ONE commit per batch; vendor tree (if cloned) is read-only — `git show`/shallow fetch or `../vendor/` clone, never modified.
- Original fixtures only; no source bytes copied into the corpus.
- kernel_opt corpus contention: all corpus edits land AFTER B102's commit lands on develop.
