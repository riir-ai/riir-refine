# Plan 401 — Batch 401: the mlx-serve staged pair (procedure-bank wrong-API-twin + lane-split bar) + the mlxfast-bonsai2 delta check

**Status:** OPEN — claimed zed-m3-b401 (epoch 1795388007, expires 2026-11-24T22:53Z); claim row in `.distill/001` the same commit.

Pool: Research 434's two UNMINED staged candidates (both from ddalcu/mlx-serve @ `96311b26` — the pin B399 verified UNMOVED; re-verify at T0 regardless) + slice (1)'s mlxfast-bonsai2 return-visit delta check (Research 417 named it). 9b engine tip UNMOVED @ `8cee3ef` (record-only, no verdict owed); 27b engine tip MOVED `cfb35b0d` → `0c7c3923` — the delta is graded at T0 and joins the batch ONLY under the 4–6 rule size discipline; overflow stages for the next batch, never bloats this one.

- [ ] T0a: clone mlx-serve @ `96311b26` (`.raw/b401/mlx-serve`); re-verify the two staged candidates' quote sites at the pin (the procedure-bank wrong-API-twin `inputSymbolIndicesForProcedureIndex:` answering 0 + the lane-split bar's convert-zeroed-first-rows-at-a-16-multiple-boundary).
- [ ] T0b: mlxfast-bonsai2-27b delta check — clone @ tip `0c7c3923`, rev-list `cfb35b0d..tip` count + subject triage; grade the delta (yield estimate, unmined-surface check); zero-mineable delta → recorded honestly, no forced rules.
- [ ] T1: dedup greps at 3+ vocabularies per candidate over the rust_perf (+ kernel_opt where touched) corpus + the staged pools; adjudications vs owned families named in-rule.
- [ ] T2: author the corpus entries (rust_perf measurement-honesty / testing-shape classes; kernel_opt only if the 27b delta yields); drop-or-face decisions recorded in Research 436.
- [ ] T3: fixtures `b401_fixtures` standalone-compiled AND executed BEFORE entry (four-arm discipline; debug AND -O).
- [ ] T4: gates — clippy `-D warnings` at default + all-features postures, lib tests incl. the batch needles, retrieval floors (DEFAULT lane; the kernel,latent lane too if kernel_opt moved), fmt gate, `pins_manifest --write` only on real moves.
- [ ] T5: count web one-landing (AGENTS §Role + README + `.docs/11_domains` + health_views + narrative heads) — only the counts that moved; freeze/score-bench NOT owed unless clippy_lints moves; bin-age check after landing (`cargo refine --version` build hash vs HEAD; rebuild at the release set if stale).
- [ ] T6: Research 436 batch record; `.distill/001` snapshot update (same commit); `feat: batch 401` commit + push; teardown `.raw/b401`.
