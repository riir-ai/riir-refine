# Plan 193 — Batch 197: MLX VJP kernel distill (PRs #4563 + #4565)

**Status:** OPEN — staged behind B195 + B196 (one-batch-per-day; the earliest slot is after B196 converts). Source verdict: Research 227.

## Tasks

- [ ] T0: re-clone `ml-explore/mlx` at the resolved full merge shas (PR #4563 `83b976e…`, PR #4565 `9295197…`) into `.raw/mlx-b197`; verify MIT LICENSE; record the full shas in this plan (the pin is load-bearing — merged 09-29, tree moves)
- [ ] T1: re-verify every Research-227 quote VERBATIM at the pin (the diff-fetched text is the provisional source; the tree is canonical) — incl. the 34 GB comment, the "tenth of a full chunk's work" replay argument, the entry-state clause, the zero-stride comment, the operand-typed-mma comment, the closed-#3241 claim
- [ ] T2: dedup RE-RUN at the live corpus (both vocabularies per candidate; the B177 lesson — phrase-greps miss rule-id mechanics): `checkpoint|replay`, `materializ|dematerializ`, `stride.*0|broadcast`, `lse|logsumexp`, `row-fold|repeat`, + read the 5 named kinships in full (`reverse-running-decay-weight`, B182 `verify-fold-gqa-repeats…`, B192 `dot-macro-family…`, staged-B196 `row-independent-pass-block-bounded-scratch`, B178 `gdn-transition-replay…`)
- [ ] T3: author entries (target `entries_gpu_63.rs` or next free slice): leads 1–7 per the Research-227 table — decide #6 fold-into-#2 vs standalone at authoring (coin-flip reason written in-batch); riders → B192 (operand-typed mma) + lead-1 (save-lse, dead-row guard, diag_off, tail tag, dummy-binding)
- [ ] T4: disambiguate `gated_delta_vjp_fused_nax1`/`nax0` vs the dispatched Ckpt variant at the pin; the Ckpt variant is the rule source, the others are the evolution record (annotation only)
- [ ] T5: ORIGINAL minimal fixtures (no verbatim source); tolerance strategy for lead 1 stated in-rule (reduction-order class, source atol 5e-3); bit-exactness argument for lead 2's replay in-rule
- [ ] T6: standing gates — batch197 parse gate (rules count) + contracts gate (distinct-from markers present) + riders gate; kernel_heal slice; lib default; clippy `-D` ×3 postures (healer first for mechanicals)
- [ ] T7: count pins walked (router + per-category table + tests.rs ×4 + mod.rs doc + AGENTS.md kernel bullet + kernel_opt.md seed row + README)
- [ ] T8: score-bench + freeze regen — NOT owed (kernel_opt-only, zero clippy-corpus delta, P30) unless the clippy corpus moved
- [ ] T9: `.raw/mlx-b197` removed at close (ephemeral-clone law); Research 227 → Status RECORD with the mine-time appendix (staging-claim corrections recorded there per the B192/B193 precedent)
- [ ] T10: queue row updated in `.distill/001` at landing + close-out

## Notes

- The standing MLX routing call holds: opponent-watch, ONE-OFF family distill (3rd one-off after Batch 103 NAX + B188 gather_qmm). No standing-source commitment.
- Lane intel is detection-only (Research 227 §Lane intel): no scored league cell moves; no issue filed (no current consumer on the riir-ai Metal-backward / GDN-training lanes — re-read at conversion, file then if a consumer appeared).
- All numbers provenance-labeled (source-reported M5 Ultra; never adoption bars).
