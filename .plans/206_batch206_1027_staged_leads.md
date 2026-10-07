# Plan 206 — Batch 206: the 10-07 staged leads (llama.cpp upstream #29442 + #28782)

**Status:** ACTIVE

Source: `ggml-org/llama.cpp` upstream, both commits already graded at the 10-07 watch
pass (riir-ai `.benchmarks/rematch/2026-10-07_watch.md`, subject-read first grades;
**full quote verification owed at THIS batch composition** — the standing law):
- `ab09ea4c148868aa4919b36e86962bf6ff2f32a7` — "cuda: BF16/FP16 conversion to f32
  chunking (#29442)" (B− class; conversion/narrowing batch-admission family) — 1 file,
  +34/−8, `ggml/src/ggml-cuda/ggml-cuda.cu`.
- `51ce9c11a6f2dfa895696c0048c4333e8953b728` — "ggml-cuda: use per-thread stream for
  buffer-init padding memset (#28782)" (C+ class; stream-assignment family) — 2 files,
  +2/−7, `ggml/src/ggml-cuda/ggml-cuda.cu` + `ci/run.sh`.

Method: Protocol A plan-first (Skill §Protocol A); ephemeral clone
`.raw/b206-llamacpp` (depth 150, master) already at the pins; removed at close.

- [x] T0 — locate both commits in the upstream window; confirm class grades hold at the
      diffs (B− / C+ vs the staged expectations); read both diffs IN FULL.
- [x] T1 — dedup grep BOTH vocabularies against the corpus (conversion/narrowing family:
      to_f32/dequant/narrow/chunk/batch-admission; stream family: stream/per-thread/
      memset/pad/stream-assignment) + kinship reads of the nearest neighbors.
- [x] T2 — author the rules (1–2 kernel_opt entries + riders where the mechanism is
      already owned); every quote VERBATIM at the pins via `git show`.
- [x] T3 — batch206 parse + contract gates (needles from the real quotes).
- [x] T4 — count pins re-pinned (kernel_opt 769 → N) with honest labels.
- [x] T5 — GOAT: G1 parse/syn (T3), G2 kernel_opt+latent_retrieval lib + default lib,
      G3 clippy `-D` at default/all-features/no-default.
- [x] T6 — retrieval-precision re-measure (21/21 floors must hold; decay → fix or
      re-pin + record the delta).
- [x] T7 — Research note 241 (`.research/.highwater` 240 → 241); §CONVERTED.
- [x] T8 — docs: AGENTS.md counts + `.docs/11_domains/kernel_opt.md` Batch-206 paragraph
      (README count row only if the total moves).
- [x] T9 — snapshot `.distill/001_mining_queue_snapshot.md` BATCH-206 row + queue-after
      line (same commit).
- [x] T10 — commit `feat: batch 206: ...` + push; `.raw/b206-llamacpp` removed at close.
