# Plan 170 — Batch 177: the EXL3/GB10 serving-recipe distill (Research 207)

**Status:** EXECUTED + LANDED 2026-09-25 as `3a1e96f0b` (shikuwa/4090 session) — fired AHEAD of the
event-blocked B176 FWHT rider (#29096 frozen through the 35th check;
cadence slot open post-B175, one-batch-per-day holds). **6 rules + 1 rider
landed** (T4 fork resolved: #2 = ONE rule — the T2 re-verification caught
`factor-affine-quant-scale-out-of-element-loop` as the loop-stage half of
#2's placement, which the verdict's phrase-grep could not see). All gates
green; close-out in `.distill/001` + Research 207 appendix. Post-landing
riders done: global binary rebuilt + refreshed (631/155/96 verified),
gate-calibration run (known standing state, unchanged).

**Source pin:** `vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe @
e0ebee8ddc4391b5e66d86fdce993b0756e00986` (MIT). Verdict, dedup evidence,
ranked table and caveats: [Research 207](../.research/207_qwen38_exl3_dgx_spark_distill_verdict.md).
Verdict line: **YES, B corpus / LANE INTEL HIGH (riir-infer) / product-coverage
does not fire.**

**Route:** `kernel_opt` for candidates 2–5 and 7; **`rust_perf`** for the two
measurement-honesty members (1, 6) — that is where B109
`dead-result-timing-loop-folded-under-lto` lives and where this family's
existing annotation precedent sits. The tempting wrong routing is putting #1 in
`kernel_opt` because the source is CUDA: the rule is about a COMPARATOR, is
runtime-agnostic, and belongs beside its family.

**Lead:** `aa-control-prices-the-fidelity-instrument` (A−, NotBenchable by
domain — the B109/Batch-136 precedent, an annotation and not a demotion).

## Tasks

- [x] **T1 — Re-clone at the pin and verify.** DONE — HEAD == `e0ebee8d`, MIT
  LICENSE verified; the repo moved +2 restructure commits past the pin
  (TabbyAPI-primary; no measurement content) — fetched the pin sha directly,
  lane intel recorded.
- [x] **T2 — Dedup RE-VERIFICATION.** DONE + ONE CATCH: (a) the 4 `page.cache`/
  `fadvise` hits READ — none carries the device-allocatable framing → #7
  lands as a rule; (b) `pinned` now 271 hits — the patch-0003 drop CONFIRMED;
  (c) `readout-trim…` READ → #5 = rider. **THE CATCH:** `factor-affine-quant-
  scale-out-of-element-loop` (entries_gpu_01) is #2's loop-stage half — the
  verdict's `scale.*factors out` grep (0 hits) could not see a rule whose ID
  IS the mechanism; recorded as the T4 input.
- [x] **T3 — Read the files the verdict did NOT grade.** DONE — patch 0001 in
  full (the in-code factors-out comment + its fp32 caveat; the 8-byte-load
  note; both scale granularities; the launcher `TORCH_CHECK` guards), patch
  0002 in full (the fail-closed `_pruned_head_cache=False` guards), patch
  0003 in full (the batched-verify eligibility predicate — no past-id
  penalties/filters/forced tokens), `docs/benchmark-data.json` (schema v2
  structure + context_curve), the `logs/` inventory + the i8 A/B pair
  re-read (78.95/72.93% on vs 73.80/73.12% off — the commit body's rounding
  exact), `tuning/README.md` header (the #6 quote + the sampler-pinning
  rider), `drop-model-cache.sh` (the #7 framing verbatim).
- [x] **T4 — Adjudicate candidate #2's split.** DECIDED: **ONE rule**. Reason:
  the T2 re-verification found the placement's loop-stage mechanism ALREADY
  SHIPPED (`factor-affine-quant-scale-out-of-element-loop`, Batch 18) — landing
  it separately would be a near-duplicate; the two halves also have different
  fix-spaces (run-an-audit-first vs apply-scale-outside-kloop) but the placement
  is the audit rule's natural FIX REGIME, so it rides as a clause + a kinship
  row (format-stage creates the constant, loop-stage exploits it — compose).
- [x] **T5 — Author entries with ORIGINAL fixtures.** DONE — 4 kernel_opt
  rules (`entries_gpu_47.rs`, Rust pseudocode fixtures, measured numbers
  quoted in `gpu_concept` with the pin) + 2 rust_perf rules (`entries_08.rs`,
  original harness-shape fixtures).
- [x] **T6 — Annotate the NotBenchable members.** DONE — #1 + #6 carry
  `bench_spec: None` + `context_notes` naming the domain reason (no latency
  quantity to ratio) + the machine-checkable surrogate (the A/A disagreement
  rate; the harness-class disclosure), citing B109/Batch-136.
- [x] **T7 — Standing gates.** ALL GREEN — batch177 gates ×4; lib default
  2286/0; kernel_opt 2645/0; kernel_opt+latent 2818/0; retrieval floors
  21/21; clippy `-D warnings` clean both postures (4 doc_markdown healed
  mechanically); score-bench #165 heal 0.96363634 FLAT (clippy corpus
  untouched — no freeze regen owed).
- [x] **T8 — Count pins walked.** DONE — kernel 627→631 (router + per-category
  table D 136→138 / N 44→46 + TWO 627s hiding inside legacy batch tests at
  5443/9550 — the B129 lesson again) + the batch-ledger comment row;
  rust_perf 153→155 (total pin + without_spec 57→59 + THREE 98→100
  annotated-count sites + corpus header + build doc + entries_08 header);
  AGENTS.md ×3 + kernel_opt.md + README.md + rust_perf.md ×3.
- [x] **T9 — Research 207 → `Status: RECORD`** with the mine-time verification
  appendix (landed/falsified/quote-re-verification + the dedup catch).
- [x] **T10 — Queue snapshot close-out row** in `.distill/001_mining_queue_snapshot.md`.
- [x] **T11 — Cleanup.** `rm -rf .raw/qwen38-flash-next-exl3-dgx-spark` (done at
  commit time). NOTE: `.raw/exllamav3` (cloned 2026-09-24 12:48 +07, turboderp
  master @ 6b84a21) is NOT this batch's and its owner is unverifiable from
  mtimes — left in place, flagged for its owner.
- [x] **T12 — Commit + push** on `develop`, `feat:` prefix, with the
  `Session:` marker.

## Deferred / not in this batch

- [-] **The SGLang sibling repo**
  (`vcruz305/Qwen3.8-Flash-Next-EXL3-SGLang-DGX-Spark-recipe`) — named as a
  lead in Research 207, **unread and therefore ungraded**. If read before this
  batch's window folds it accumulates into it (the Batch-145/146 staged-charter
  precedent); otherwise it is its own verdict.
- [-] **The riir-infer EXL3 issue** — the lane deliverable, and deliberately
  NOT a task here. A verdict does not edit sibling repos; Research 207 names the
  file and its content, and the issue number comes from that repo's
  `.issues/.highwater` at write time. It is independent of whether this batch
  ever mines.

## Standing notes

- **The engine under study is non-deterministic at temperature 0** — which is
  candidate #1's whole subject. Any re-measurement of this source's numbers must
  use the acceptance counters, never a text diff. The rule and the caveat are
  the same fact.
- **Do not join this source to Research 120/139 on the model name.** That arena
  is Qwen 3.8 **27B** on MLX/Metal (64 layers, hidden 5120, vocab 248320); this
  is Flash-Next on CUDA/GB10 (48 layers, hidden 2560, 512 experts top-10). Same
  family name, no shared measurement.
- **Provenance is three deep** and every entry citing the math must say so:
  this recipe → `vcruz305/exllamav3` PR #2 (fork `523ecd3`) →
  `turboderp-org/exllamav3` (MIT, the EXL3 format) → **QTIP**
  (arXiv:2406.11235, the trellis-coded VQ construction).
- **n=1 per cell, one box, one author.** The author's own default-off note on
  patch `0001` (*"pending a second box's numbers"*) is the right posture to
  inherit in the entry text.
