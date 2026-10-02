# Plan 193 — Batch 197: MLX VJP kernel distill (PRs #4563 + #4565)

**Status:** COMPLETE (2026-10-02, shikuwa/4090) — landed as Batch 197 (6 rules + 1 rider, corpus 728→734, E +2 / G +3 / A +1; B196 converted same day, disclosed). Source verdict: Research 227; quote record: Research 230 (22/22).

## Tasks

- [x] T0 (full shas: 83b976ea438466418dea20c89709601201851fe1 / 9295197533d586dc26d6814b92899fa088320ab3; recorded here + Research 230): re-clone `ml-explore/mlx` at the resolved full merge shas (PR #4563 `83b976e…`, PR #4565 `9295197…`) into `.raw/mlx-b197`; verify MIT LICENSE; record the full shas in this plan (the pin is load-bearing — merged 09-29, tree moves)
- [x] T1: re-verify every Research-227 quote VERBATIM at the pin (the diff-fetched text is the provisional source; the tree is canonical) — incl. the 34 GB comment, the "tenth of a full chunk's work" replay argument, the entry-state clause, the zero-stride comment, the operand-typed-mma comment, the closed-#3241 claim
- [x] T2: dedup RE-RUN at the live corpus (both vocabularies per candidate; the B177 lesson — phrase-greps miss rule-id mechanics): `checkpoint|replay`, `materializ|dematerializ`, `stride.*0|broadcast`, `lse|logsumexp`, `row-fold|repeat`, + read the 5 named kinships in full (`reverse-running-decay-weight`, B182 `verify-fold-gqa-repeats…`, B192 `dot-macro-family…`, staged-B196 `row-independent-pass-block-bounded-scratch`, B178 `gdn-transition-replay…`)
- [x] T3 (entries_gpu_65.rs, ONE slice file — parts merged at authoring; lead 6 FOLDED into rule 2 as the register posture, coin-flip reason in-rule): author entries (target `entries_gpu_63.rs` or next free slice): leads 1–7 per the Research-227 table — decide #6 fold-into-#2 vs standalone at authoring (coin-flip reason written in-batch); riders → B192 (operand-typed mma) + lead-1 (save-lse, dead-row guard, diag_off, tail tag, dummy-binding)
- [x] T4: disambiguate `gated_delta_vjp_fused_nax1`/`nax0` vs the dispatched Ckpt variant at the pin; the Ckpt variant is the rule source, the others are the evolution record (annotation only)
- [x] T5: ORIGINAL minimal fixtures (no verbatim source); tolerance strategy for lead 1 stated in-rule (reduction-order class, source atol 5e-3); bit-exactness argument for lead 2's replay in-rule
- [x] T6 (parse/contracts/riders 3/3 · kernel slice 403/0 · lib 2424/0 · clippy -D clean default/kernel_opt/no-default + lib all-features; the --all-targets all-features lane carries a PRE-EXISTING Windows-only red in tests/corruption_corpus_freeze.rs Verdict::Signal, untouched, disclosed): standing gates — batch197 parse gate (rules count) + contracts gate (distinct-from markers present) + riders gate; kernel_opt slice; lib default; clippy `-D` ×3 postures (healer first for mechanicals)
- [x] T7 (tests ×6 + router 734 + mod doc + category comments A/E/G + AGENTS ×2 + kernel_opt.md seed + README ×2; the AGENTS kernel feature bullet's stale kernel_heal NAME repaired en route + the global SKILL.md gate command): count pins walked (router + per-category table + tests.rs ×4 + mod.rs doc + AGENTS.md kernel bullet + kernel_opt.md seed row + README)
- [x] T8: score-bench + freeze regen — NOT owed (kernel_opt-only, zero clippy-corpus delta, P30) unless the clippy corpus moved
- [x] T9: `.raw/mlx-b197` removed at close (ephemeral-clone law); Research 227 → Status RECORD with the mine-time appendix (staging-claim corrections recorded there per the B192/B193 precedent)
- [x] T10: queue row updated in `.distill/001` at landing + close-out

## Notes

- The standing MLX routing call holds: opponent-watch, ONE-OFF family distill (3rd one-off after Batch 103 NAX + B188 gather_qmm). No standing-source commitment.
- Lane intel is detection-only (Research 227 §Lane intel): no scored league cell moves; no issue filed (no current consumer on the riir-ai Metal-backward / GDN-training lanes — re-read at conversion, file then if a consumer appeared).
- All numbers provenance-labeled (source-reported M5 Ultra; never adoption bars).
