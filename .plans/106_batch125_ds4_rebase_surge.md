# Plan 106 — Batches 125/126: the ds4 PR rebase-surge re-mine

**Status:** PLANNED 2026-09-07 — trigger verified by API (Plan 102 T7 discharge); `.raw` clone NOT staged yet; mine not started. Batch 125 = Part A (Metal perf lanes, kernel_opt). Batch 126 = Part B (tool-text + QA methodology, rust_perf). Part C = league intel, named-not-edited. **→ COMPLETE 2026-09-07: Batch 125 LANDED (Research 148); Batch 126 CONFIRMED N/A at the trigger re-check (no ds4 delta since the B125 pin); T7 clone removal verified. No open tasks.**

## Trigger (verified 2026-09-07 09:15 +07, Plan 102 T7 re-scan)

- **PR `qwen3.8-flash-next` head moved: `69c38d23` → `ffd85d42`** (updated 2026-09-06T23:18:03Z, AFTER the Batch-122 mine read the older head). The PR was **RE-BASED onto upstream main `c0a6119f`** (upstream's head is in the new PR ancestry — the fork and upstream cross-pollinated; upstream main itself is UNCHANGED at `c0a6119f`, the Batch-123 pin — no upstream trigger).
- **The compare-vs-pin trap (recorded):** `compare/69c38d23...ffd85d42` lists the WHOLE diverged history (62 commits, ~19K ln incl. files B118/B121 already mined: `metal/qwen4.metal`, `QWEN38_PERF_HANDOFF.md`, `qwen4_iq2.py`). The TRUE new delta = commits dated **2026-09-05 21:48Z .. 2026-09-06 23:11Z** (~47 commits). Never quote counts from the compare; re-derive at the pin.
- Size class: BIG (ds4.c +4432/−146, ds4_server.c +1621/−149, ds4_metal.m +1420/−134, ds4_agent.c +1373/−473, 5 new speed-bench files ~3.6K ln, 4 new gguf-tools python files ~4.7K ln). Same class as B123's surge — split lanes, keep batches reviewable (the Plan-102 lesson).

## CORRECTED SURVEY (T0, 2026-09-07 09:25 +07 — clone staged at `ffd85d42`, HEAD == pin, MIT verified)

- **The trigger is SMALL, not a surge.** The `69c38d23...ffd85d42` compare lists 62 commits / ~19K ln because the PR line was REWRITTEN (re-based onto upstream main) — old mined content re-appears under new SHAs. Two already-banked populations subtract out:
  1. **The upstream-side window** (`9ab7053..c0a6119f`: DGX Spark, ROCm gfx1151, Metal SSD expert-cache, tool-text preservation) = **Batch 123's entire 41-file delta** (Research 146). The original Part A lane A1 and Part B tool-text lane below are WITHDRAWN as mine targets — already banked; ride only as dedup-grep needles.
  2. **The Qwen3.8 stack as of `69c38d23`** = B118/B121/B122's mines (old SHAs rewritten — e.g. old `9f4313d`→new `32724a2`, old `69c38d2`→new `948edcd`). `c0a6119f..ffd85d42` = 14 commits = the whole Qwen line, NOT the delta.
- **The genuine new delta (message-level mapping vs the B118–B123 records; the old pin tree `69c38d23` is UNFETCHABLE — GitHub refuses force-pushed-away shas — so content-level verdicts happen at entry time via dedup grep + quote re-verify):**
  - `658b2fc` Preserve Qwen tool-turn KV reuse and assistant boundaries — NEW (no counterpart recorded)
  - `681b9ce` Fix JPEG decoding parity + separate vision quantization checks (`third_party/iris/jpeg.h`, `tests/test_jpeg_decode.py`) — NEW
  - `4010cc6` Allow Qwen PLE sidecars in the native agent — B118 PLE-sidecar rider territory
  - `b98e009` Record Qwen IQ2 and Q4K vision smoke results — methodology (B121-adjacent; Q4K is the new arm)
  - `ffd85d4` Preserve Qwen tool handling and validate the main rebase — methodology/rebase-harness
  - **Drift-check population:** `metal/qwen4.metal` + `ds4_qwen4.c` re-committed content vs B118's mined shapes (dedup grep decides rule vs already-banked per shape).
- **Revised yield estimate: SMALL batch — 3–5 rules + 2–3 riders.**

## Staged clone (the `.raw` discipline)

`.raw/ds4-metal` (gitignored; MIT — verify LICENSE at stage time), **fresh clone at mine time pinned to `ffd85d42`** — the PR has force-pushed twice already; NEVER reuse an old clone, re-verify HEAD == ls-remote at stage time. Quotes re-verified against the clone by the read lanes; clone removed at landing of the last batch.

## Part A — Batch 125: Metal perf lanes (kernel_opt; M3 box, CPU-only)

Commit-message survey (re-verify every file/line at the pin — messages are NOT evidence):

**A1 — SSD streaming + expert-cache axis** (the B123 family continuation on the fork side; dedup grep `ssd|expert|stream|readahead|resident|working.set|preload` FIRST — expect riders/confirmations of `batch-pool-read-makes-serial-readahead-redundant`, `cold-large-prefill-whole-layer-lookahead-over-partial-cache`, `static-seed-priority-bounded-to-observed-scale` where the content cross-pollinated, NEW rules where the fork side is the original):
- `22e41728` static Metal streaming weights resident within the runtime budget
- `6ccc1e7e` Metal cache reuse + overlap the first GLM streaming prefill
- `edcafe4d` reuse GLM router readback during IQ2 expert streaming
- `85837d02` more of the safe Metal working set for DeepSeek expert caching
- `91dda0bc` Metal expert caching adapts beyond the built-in preload ranking
- `715ed7a8` age Metal expert caches from the first routed layer
- `660e1d4b` bound Metal streaming model views without losing auxiliary mappings
- `437dbcba` read full IQ2 GLM layers ahead during large continued prefills
- `d7ade9fd` reuse cached IQ2 experts for small Metal GLM prefills
- `fb2abb29` align Metal GLM attention scratch for arbitrary context lengths
- `4814f44f` remove managed-storage notifications from shared Metal buffers
- `e9653529` preserve RoPE in full GLM compact prefill

**A2 — speed-bench protos (new unmined files, ~3.6K ln):** `speed-bench/gqa_mma_proto.metal` (1640) + `gqa_mma_proto.m` (890) + `qsa_prefill_bench.c` (590) + `hc_chain_proto_bench.c` (251) + `hc_chain_proto_kernels.h` (276). QSA = the B118 attention family; GQA MMA proto may carry new tile/pipeline shapes.

## Part B — Batch 126: tool-text + QA methodology (rust_perf io/methodology)

- **Tool-text preservation family** (NEW `ds4_tool_text.h` 59 ln + the delimiter/marker commits): `d077fa67` split tool delimiters in the server sampling tracker · `5b3cc8bf` GLM tool syntax + speculative server boundaries · `fc6414cd` literal tool/thinking markers inside argument data · `d108ae4e` literal tool data across agent/server rendering · `e1711c21` report tool observation errors without compacting the conversation · `930ab732` valid opportunistic drafts across tool sampling boundaries · `6eac69e0` bound tool recovery by request budget + explicit stops · `658b2fc1` Qwen tool-turn KV reuse + assistant boundaries · `c0a6119f` preserve the final byte of in-memory session snapshots (upstream head — its un-banked residue, B123 did not mine it).
- **QA/measurement methodology** (the B121 `write-verify-readback` family neighbors; dedup first): `validate_scores.py` (236) + its test (219) · `compare_frontier_logits.py` (228) + its test (200) · `42da390e` frontend regressions without loading a model · `aa8fef4a` real server coding sessions + prefix-reuse checks in QA.

## Part C — LEAGUE INTEL (named-not-edited; no corpus rules from hardware we don't own unless the rule is hardware-independent)

- **DSpark EOS-early-stop** (`7a8c1b27` "stop verification at the first EOS") — spec-decode verify early-stop; READ-ONLY intel for the riir-ai 721/717 lanes (both closed-negative; do not reopen without measured parity evidence).
- **ROCm gfx1151 lane** (`1ef9bba6` version-matched hipBLASLt prefill plans · `64e490bc` IQ2 prefill input staging specialization · `e83b1063` DSpark verifier attention projection · `394865c5` aligned target features + rolling window) — DGX Spark hardware we do not own; B110 precedent says the corpus serves user code on foreign hardware, so VALID decision-content rules may route here, but each must be hardware-independent in its fix shape (plan-selection/staging predicates), not a gfx1151 tuning constant.

## Tasks

- [x] T0 — stage `.raw/ds4-metal` at `ffd85d42` (fresh clone 2026-09-07 09:24 +07, HEAD == pin, LICENSE MIT verified); true delta re-derived — trigger is SMALL (~5 new commits, see Corrected Survey); old-pin fetch refused (force-pushed sha) — content verdicts at entry time
- [x] T1 — Batch 125 (Part A, REVISED per Corrected Survey): dedup grep the ~5 new-commit concepts (`tool.turn|kv.reuse|jpeg|iris|ple|sidecar|vision.quant|q4k`) + the qwen4.metal drift-check shapes against the corpus; 2 parallel read lanes (lane 1: tool-turn KV reuse + PLE sidecars + tool handling in ds4_agent.c/ds4_server.c/qwen4 host files; lane 2: jpeg.h parity + vision quant checks + qwen4.metal/ds4_qwen4.c drift) proposing rules (pattern, bounded fix-space, category, rationale, source quote); re-verify every quoted line. **DONE 2026-09-07: lanes returned 6 candidates; coordinator verified all quotes at the pin; landed 4 kernel_opt rules + 1 rust_perf io rule + B57 rider + config-surface rider; declined the escape-round-trip rule (single-source)**
- [x] T2 — Batch 125: corpus entries + `batch125_pseudocode_parses` gate + count pins (kernel_opt total/category; walk every manifest/doc line). **DONE: entries_gpu_32.rs (4 rules) + rust_perf corpus.rs (1 rule) + gates 4/4 + pins walked (totals ×3, categories A/D/E/N, annotated ×2, mod.rs lists, doc counts)**
- [x] T3 — Batch 125 GOAT: batch gates first-shot + lib default/kernel_opt/kernel_opt+latent + retrieval re-run (kernel 100/100/100, blended ≥94.7/96.9/97.7) + clippy 3 states + score-bench FLAT. **DONE: 1167/0 default, 1483/0/1i kernel_opt, 1600/0/2i latent, retrieval byte-identical, clippy 0 ×3, score-bench #75 FLAT 96.2/1.9**
- [x] T4 — Batch 125 landing: Research note (next number per `.research/.highwater`, currently 148) + AGENTS.md corpus count + `.docs/11_domains/` + snapshot row; commit `feat: batch 125: ...` + push. **DONE: Research 148, AGENTS.md 559/112, .docs/11_domains both files, snapshot row**
- [x] T5 — Batch 126 (Part B, REVISED — the original tool-text lane was B123-banked; the only Part-B remainder is the escape-round-trip observation, deferred single-source). **CONFIRMED N/A 2026-09-07 (~10:30 +07): trigger re-checked by API (B120 protocol) — PR #991 head still exactly `ffd85d42` (the Batch-125 pin, updated_at 23:18:03Z BEFORE the B125 mine read) and upstream main still `c0a6119f` (the B123 pin); no second instance of the escape-round-trip shape surfaced; the deferred single-source observation stays deferred, reopen on the next ds4 trigger.**
- [x] T6 — Part C league intel: REVISED — the DSpark EOS-early-stop + ROCm gfx1151 rows are UPSTREAM-side (B123 window), already surfaced there; no new league row expected. **CONFIRMED N/A at entry time — no new content condition**
- [x] T7 — remove `.raw/ds4-metal` at the last landing; final snapshot update (pins current: PR `ffd85d42`, upstream `c0a6119f`). **DONE 2026-09-07: clone verified ABSENT at the `.raw` root (removed at the Batch-125 landing session; only the tiny historical batch scripts `batch118/` 52K + `ldg_probe/` 16K remain — not clones, left in place); final snapshot update = the Batch-126-N/A row (same commit as this plan close).**
