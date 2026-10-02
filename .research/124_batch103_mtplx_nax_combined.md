# 124 — Batch 103: the MTPLX PR #391 + MLX NAX combined kernel batch

**Status:** RECORD — batch landed (4 rules, corpus 459 → 463, D +4, + the B100 rider enrichment) + **ADDENDUM landed same day** (the silently-omitted MLX #4416 rule, corpus 463 → 464, D +1; see the addendum section at the end)

## Source 1 — youssofal/MTPLX PR #391 (the Plan 072 batch)

- **PR:** [youssofal/MTPLX#391](https://github.com/youssofal/MTPLX/pull/391) — "perf(qwen4): restack
  Flash-Next optimizations on MTPLX 2.10.1" (davidtai; OPEN / unmerged; mergeable_state `unstable`
  at mine time, was `dirty` at the verdict pin). Apache-2.0 both trees.
- **Verdict pin:** `ef9631324766b3aa09de849a795bd6f3d40532ed` (Research 122).
- **Mine pin (T0 re-pin):** **`a5e38bb7d6c099fe79b3a7d06fae91c64092cb9f`** — head moved (pushed
  2026-09-03 09:10 −0500, "perf(qwen4): Qwen3.8 Flash-Next stack, 16K decode 57.7 -> 80.9 tok/s";
  PR body rewritten 2026-09-03 18:24Z). The pin→head delta deleted the branch's experimental
  scaffolding (`fable_*` files, `scripts/fable/*`, the docs/perf charts + wavefront/ple/
  qsa-sparse-decode docs) and reworked `full_stack_env.py` (+1325), `generation.py`,
  `server/openai.py`; added `kernels/moe_glu_decode.py`. **Every anchor was re-verified at the NEW
  head; two verdict-time anchors did NOT survive and are recorded honestly in-rule** (below).
- Vendor clone: `/Users/katopz/git/vendor/mtplx` (shallow, pinned, read-only; `.raw` discipline).

### Anchor re-verification ledger (T0)

| Verdict-time anchor | State at `a5e38bb` | Disposition |
|---|---|---|
| W87 W-number attribution | The rewritten body no longer names W87 | Carried in-rule as "NOT re-confirmable"; mechanism verified independently |
| "hard-coded draft width ... always fell through" production note | Body rewritten; the mechanism now stated in `tests/test_fable_qsa_sparse_decode_wiring.py:10-14` ("the ONE call site that could reach the verify width asked about a width the flag never armed ... read a zero row count, returned False") | Re-anchored to the wiring test + the module docstring |
| `q_len × gqa_factor ≤ 32` dead band | `sdpa_2pass_paged_q8.py:193-194` — `if 32 * gqa_factor * int(q_len) > 1024: return None` | VERIFIED byte-accurate |
| W54 19.2% ceiling arithmetic | Body §3.4 "not in this PR" table: "Two-stream wavefront prefill \| 0.4 % against a 15 % gate; tile ceiling 19.2 % \| the ceiling is too low to fund" | VERIFIED |
| W68 two-outage corrections | `qsa_sparse_decode.py:340-349` REQUEST_SHAPE_DECLINES doc: "Two request-shape raises reached production before this list existed. The first was ``k_eff != TOP_K`` (2026-09-02, HTTP 500 on every HumanEval prompt). The second ... ``total_tokens // 4 > 512``, i.e. 2,052 tokens, so the call died inside ``attention()``" | VERIFIED (now first-class in-source) |
| Install-time binding raise | Body §3.1 note ("One key set to `0` on its own raises at model install") + `runtime_options.py:296-298` ("Anything else raises rather than falling back, so a typo in a sweep cannot quietly measure the default") | VERIFIED |
| Graph-build overlap (W67) | PROMOTED default-on at the new head (-1.5% cycle, decode 76.2→77.4 tok/s) — see T5 disposition | Record only |

### What landed (4 rules → the combined batch)

- `shape-class-keyed-capability` (D, the A− GOAT lead) — the wiring defect + the process-verdict
  keying discipline; anchors: the module-docstring GATE DISCIPLINE section, the wiring test, the
  imported-width routing predicate (`qwen4_exp.py:73-76` + `:2869`), the 32-cap native gate, the
  probe-geometry certificate (`qsa_sparse_decode.py:227-229`).
- `wavefront-overlap-pair-ceiling` (D, B) — the bound-before-build funding arithmetic; anchors:
  the body's not-in-this-PR rows (the wavefront row + the shared-expert stream row
  "cross-stream fences cost as much as the overlap gains").
- `request-shape-decline-mirror-completeness` (D, B) — THE MIRROR decline/raise taxonomy +
  counted named sites + the pinned mirror test; anchors: `qsa_sparse_decode.py:333-371`,
  `:264-291`, `:411-440`, `runtime_options.py:296-298`, the body's install-time raise note.
- `backing-provenance-causal-shadowed-extent-widen` (D) — from source 2 (below).

## Source 2 — apple/mlx NAX extraction (the 09-03 queued leads, T7 COMBINE)

- **Pin:** `f99e916` (PR #4416, "Make head-dim-256 prefill with array mask run fused NAX
  kernel") + `bbf8a7a` (PR #4380, GQA 12/16 two-pass table). Verified in
  `/Users/katopz/git/vendor/mlx` via `git show <rev>:mlx/backend/metal/...` (the correct path is
  `mlx/backend/metal/kernels/steel/attn/kernels/steel_attention_nax.h` — the snapshot's
  path-slip lesson re-confirmed; `git show --stat=200` prints full paths).
- **`backing-provenance-causal-shadowed-extent-widen`** (D, new rule) — anchors re-verified
  byte-accurate this session at `scaled_dot_product_attention.cpp:52-77` (`has_backing_rows`:
  stride sanity / row-pitch containment "The rows must stay inside the head's row pitch (so they
  belong to the cache the slice was taken from)" / buffer end) + `:46-49` ("The causal offset
  describes the true diagonal even if kL is widened below." / `int qL_off = kL - qL;`) + the
  widening gate (`if (split_d && do_causal_ && !mask.has_value() && (kL % bk))`).
- **B100 rider enrichment** (`register-budget-head-split-gqa-reread-trade`, entries_gpu_14.rs):
  (1) the NAX hd-256 dsplit instance — head-dim split across WN=2 ("static_assert(WN == 2, ...)"
  + TD % WN == 0), the ONE-peer exchange (`s_xchg[WM][WN][2 * kEPF * 32]`), Q-half
  register-residency hoisted out of the KV loop; (2) the #4380 GQA 12/16 table (64/g8/hpt8,
  128/g8/hpt4, 128/g12/hpt4, 128/g16/hpt2 — hpt falls as GQA rises) with the generalized
  `gqa_dims` predicate and the unchanged KV ≥ 8192 eligibility threshold.

## T5 disposition — graph-build overlap (W67): DROP, recorded

Their graph-build-overlap lane survived the branch rewrite and is default-on at the new head
(-1.5% cycle; the retained-stack census attributes 1.934 ms/cycle of GPU idle to the verify
body's host-side replay). Our stack: the Bench-761/771 caches consumed the same seam — the
uniform-content cache cut uniform writes 845.3 → 1.0/token and submits 14.86 → 1.98, and M3
decode now sits AT the ~31 ms/token GPU floor. No measured exposed host-enqueue window remains
in our decode path, so the rule has no firing population here. Re-open trigger: any future
host-side decode decomposition (e.g. a new graph runner) re-measuring >1 ms/cycle of GPU idle
attributable to host graph construction.

## T6 riders (no corpus delta, recorded here)

- **Bench 805 mask-fuse external confirmation:** their `MTPLX_FABLE_PREFILL_MASK_FUSE`
  (causal fused attention instead of a materialized score tensor) measured prefill 1,483 →
  1,535 tok/s (+3.5%), TTFT −0.38 s, peak 92.1 → 89.4 GB — but diverges from stock on 8/8 long
  greedy prompts (median position 7.5, rounding class, "relative divergence gate only").
  Confirms the lever's value shape AND the class difference: our Bench 805 promotion was gated
  on 0 argmax flips across 7 lengths (bit-identical), theirs ships a lossy fuse with a
  documented opt-out. Their +3.5% is engine-comparable context for our default-on tiled flash;
  the divergence posture is the record's point.
- **Issue 734 GDN-blocked regime note:** their GDN blocked prefill measured −0.35 s on their
  UNBLOCKED-eager stock; our chunked-recurrence measurement was 0.97-0.98× on an ALREADY-batched
  path (Issue 734). Regime difference — do not cite as contradiction. Also retained at the new
  head (`kernels/gdn_blocked_prefill.py`, default-on) with a matching divergence note
  ("The divergence profile matches the GDN blocked prefill kernel").
- **Bench 641/B77 micro-vs-e2e external confirmation:** their expert-major MoE prefill schedule
  measured micro 1.94× @R=320 but e2e −3.6% + peak +1.44 GB ("the schedule-bound matmul gain
  does not survive the whole prefill") — a third independent instance of the micro-vs-e2e class
  (Bench 641's expert-major micro gain; B77's batch-width routing caveat).
- **League rider (from Research 122, already filed):** their cross-engine-table withdrawal
  (venv editable-install cwd contamination) independently confirms the FairnessManifest's
  "opponent repo + commit" field — recorded as rule 11 in the riir-ai league doc at verdict time.

## Drop ledger (per Plan 072, all held)

mincore read-skip (their own negative), route kernel / draft K20 pre-scatter / FR-Spec
(spec-decode closed-negative in-stack: Issues 717/721/737), sparse split-K decode attention
(Bench 763: ≈0% share), GDN keep-mask fold (their own measured LOSS: cycle 33.93 → 35.36 ms,
+4.2%), oMLX keepwarm port (they falsified it themselves). En-route add: the branch rewrite
DELETED the wavefront/ple/fable files, so several verdict-time lead FILES no longer exist to
mine — the surviving mechanism anchors are the body tables + the retained kernel files.

## Non-goals held

No prefill-throughput claim (the verdict is explicit: none of the retained levers transfer —
all HAVE / refuted / no-analog in-stack). No league action (their 1,332-1,638 tok/s @16K is
M4-class silicon + a finer-quant MoE pack — a mining source, not an opponent row).

## GOAT

- `batch103_rules_present` + `batch103_concepts_covered` + `batch103_pseudocode_parses`
  (cfg(not(kernel_verify)) per the B70 convention): 3/3 PASS (one gate-catch: the
  wavefront concepts needle "stages - 1 pairs" did not match the concept text — the concept
  carries "exactly S-1 overlappable producer/consumer pairs"; needle re-pointed, no text change).
- kernel_opt lib **1285/0/1i** (was 1282/0/1i + 3 new gates); default lib **1029/0**;
  kernel_opt+latent **1392/0/2i** with `retrieval_precision_regression_gate` PASS (floors HELD,
  no re-pins).
- Clippy 0 in all three states (kernel_opt / default / kernel_opt+latent), `--lib`.
- Count pins walked: tests.rs corpus total 459→463 (+ message) + batch-list comment + expected-
  array D 98→102 + comment "updated for Batch 103" + sums ×2 + cover-all comment 459→463 +
  router.rs expert total + mod.rs 463-rule doc + D category header comment 98→102 + 4 list
  appends. En-route count-pin rot FIXED: the 15 category header comments in mod.rs were stale by
  up to 2 batches (A said 103 vs true 105, C 39 vs 41, H 16 vs 17, N 27 vs 28) — none were
  pinned by any test (the pinned table lives in `kernel_categories_counts_match_plan_003_table`),
  so the headers drifted silently; all 15 headers now match the pinned table (A 105, C 41,
  H 17, N 28, others unchanged).
- Category deltas: D 98 → 102. Total 459 → 463.

## Discipline notes

- ONE commit per the plan; the generator script + vendor clones are not committed
  (target/ + vendor are ignored); the `.raw`/vendor discipline: clones stay read-only, pins
  recorded here.
- The B77 edit-tool escape-strip lesson struck AGAIN: the first in-place edit of the B100 rider
  landed bare quotes inside the Rust string literal (22 compile errors); repaired via a
  byte-precise Python pass over the rider span with a scan-verified string terminator. Corpus
  entry GENERATION went through the Python heredoc path (json.dumps ensure_ascii=False) and was
  clean first-shot — the standing rule (generator, never in-place edits for string-heavy corpus
  content) re-proven.
- Plan 072 checkboxes flipped at landing; the batch number was allocated at batch start (103)
  per T1; `.plans/.highwater` stays 72 (072 was the plan, not the batch).

## Bottom line

The MTPLX PR is a **measurement-culture and gate-discipline mine, not a prefill lever** — its
3 retained rules encode host-orchestration discipline our own GOAT gates already practice
(shape-keyed verdicts, bound-first funding, decline-mirror completeness), now corpus-backed with
production-outage evidence. The MLX extraction adds the one kernel-grade rule (backing-provenance
extent widening) that has a direct analog in our chunked-KV-cache decode surfaces. The real
prefill frontier for our stack stays Issue 771 (GEMM TF gap) and Issue 844 (flash-vs-GEMM long-P
prize) — untouched by this batch, as the verdict predicted.

## Addendum (2026-09-04, second session — the Issue-665 collision salvage)

Both sessions executed the same Plan-072 NEXT-UP row concurrently (the 4090
handoff thread and the M3 sibling). Push-wins adjudication: this landing
(98b3b8f, 4 rules, corpus 459 → 463) kept the batch number and all baked
references; the duplicate was yielded without merge.

**The salvage — one rule the landing silently omitted:**
`dispatcher-eligibility-predicate-lags-kernel-capability` (D) — the MLX #4416
lead-2 from the 09-03 extraction unit. It is absent from the corpus above, from
the drop ledger, and from this note (an oversight, not a decision — the landed
`request-shape-decline-mirror-completeness` rule even cites its fix-space, "the
engagement-canary defense", without the rule existing). Anchors re-verified
byte-accurate by the salvage session at `714a7ef` (#3842: the kernel's
`has_mask` function_constant(300) + buffer(5)/buffer(6) mask bindings landed
WITH the `!has_arr_mask` dispatcher conjunct) and `f99e916` (#4416: the 2-line
predicate fix, zero kernel changes); `origin/main` moved `90846ad` → `b636898`
(allocator-query API, no kernel) — anchors unaffected.

Landed as the Batch 103 addendum: corpus 463 → 464, D 102 → 103, gates
extended in place (rules_present + concepts + parses), GOAT green (kernel_opt
lib 1291/0/1i, default 1035/0, retrieval floors HELD kernel 100/100/100,
clippy 0). AGENTS.md carries the addendum rider on the Batch 103 line.
