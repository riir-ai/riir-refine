# Plan 170 — Batch 177: the EXL3/GB10 serving-recipe distill (Research 207)

**Status:** OPEN — plan-FIRST, created 2026-09-24 before any corpus edit.
Queued behind the B176 FWHT rider. No corpus edit has been made.

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

- [ ] **T1 — Re-clone at the pin and verify.** `git clone --depth 1` into
  `.raw/qwen38-flash-next-exl3-dgx-spark`; `rev-parse HEAD` must equal
  `e0ebee8d…`; confirm LICENSE is MIT. **Any stale `.raw/` entry from the
  verdict session is UNTRUSTED — delete and re-clone.** The verdict session's
  clone was removed at landing by design.
- [ ] **T2 — Dedup RE-VERIFICATION (not a re-read of Research 207's table).**
  Re-grep every candidate under both vocabularies at current corpus HEAD; the
  corpus moves between verdict and batch. Three rows need a READ, not a count:
  (a) the 4 `page.cache`/`fadvise` hits vs candidate #7 — if the existing
  `rust_perf` io entries already carry the device-allocatable framing, #7 is a
  rider or a drop; (b) the 68 `pinned` hits vs the patch-0003 drop row —
  confirm the drop; (c) `readout-trim-needs-committed-id-coverage`
  (`entries_gpu_26.rs`) vs candidate #5 — read the landed text and decide
  **rule vs rider** on its scope-correction claim.
- [ ] **T3 — Read the files the verdict did NOT grade.** `hc_mix.cu` full body
  (+256 ln; only the header and commit body were read), `docs/benchmark-data.json`,
  the `logs/` matrices backing the 4.05-bpw and PIECEWISE tables. Nothing
  ungraded enters the corpus.
- [ ] **T4 — Adjudicate candidate #2's split.** It may be TWO rules — the
  `(class, dtype)` residency audit discipline, and the scale-factors-out-of-the-
  k-loop exactness placement — or one. Decide with the reason written down.
- [ ] **T5 — Author entries with ORIGINAL fixtures.** No CUDA, no Python, no
  README prose pasted into `buggy_code`/`fixed_code`. Minimal reproductions in
  the corpus's own shape; measured numbers may be QUOTED in `gpu_concept` with
  the pin, as the B115/B130 entries do.
- [ ] **T6 — Annotate the NotBenchable members.** #1 and #6 take
  `bench_spec: None` + a `context_notes` annotation naming the domain reason
  (no latency quantity to ratio) and the machine-checkable surrogate (the A/A
  disagreement rate; the harness-class disclosure). Cite B109 as the family
  precedent, Batch 136 as the landing precedent.
- [ ] **T7 — Standing gates.** `batch177` gates + lib default + kernel_heal +
  kernel_heal+latent + retrieval floors + clippy 0 across default/kernel_heal
  + score-bench. Per the B121/B130 close-out shape.
- [ ] **T8 — Count pins walked.** Corpus totals, per-category counts, the
  category comment pins, member lists, and the annotated-rule count — both
  domains, since this batch lands in two. **The B129 lesson: the walk missed
  its own comment pins.**
- [ ] **T9 — Research 207 → `Status: RECORD`** with a mine-time verification
  appendix: which candidates landed, which were falsified and why, and any quote
  that failed re-verification at the pin.
- [ ] **T10 — Queue snapshot close-out row** in `.distill/001_mining_queue_snapshot.md`.
- [ ] **T11 — Cleanup.** `rm -rf .raw/qwen38-flash-next-exl3-dgx-spark`. *A
  landed batch with a live `.raw/` entry is an unfinished batch.*
- [ ] **T12 — Commit + push** on `develop`, `feat:` prefix, with the
  `Session:` marker (shared-worktree attribution discipline).

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
