# Plan 244 — Batch 242: the mlx-serve GDN/QSA pack (kernel_opt)

**Status:** COMPLETE (T0-T7 done; T8 = this batch's commit) — Batch 242 LANDED: corpus 816→847 (+31 own rules: slice 97 ×10, slice 98 ×9, slice 99 ×12; A +7, B +3, D +9, E +2, K +1, L +1, N +8) + 6 riders landed + 1 rider skipped honestly (the predicted face absent from source) + 1 candidate killed (C+ thin) + 4 folded. Landed ABOVE the 12-16 estimate, disclosed per the lithos precedent. Numbering note: plan 244 / research 274 / Batch 242 per the committed claim — the tokenmaxxing lane renumbered to 245/275/243 around it (Bench-881 first-committed-on-origin rule; their note discloses the yield); the `.highwater` files stand at 245/275 (already above — monotone, no future reuse).

Source: `ddalcu/mlx-serve` @ pin `1015f7854fd8ee5c22e34993aaddee2443b9cfbf` (MIT, 2026-10-14). Verdict: YES B+ kernel_opt / LANE INTEL HIGH (Research 274 — the full ranked table, dedup evidence, and anti-leads live THERE; this plan is the execution charter).

## Tasks

- [x] **T0 — re-clone at the pin + freshness check.** Cloned at `1015f7854fd8ee5c22e34993aaddee2443b9cfbf` (verified HEAD == pin; tip had NOT moved past the pin as of landing). Coordinator spot-verified the load-bearing quotes (gdn_decode prework header, the decline-latch pair, the GOAT prescale) + each slice agent byte-verified its own set.
- [x] **T1 — per-candidate dedup re-run** — done at authoring per slice (kinship greps recorded in-rule; the slice-A agent corrected two verdict kinship names — the real B182 id is verify-window-prework-loads-before-stores; tree-verify-recurrent-state-parent-reload (entries_gpu_07) found as a closer kin the verdict missed).
- [x] **T2 — author slice A (GDN/QSA/RHT core)**: 10 rules landed (entries_gpu_97.rs; fold-depth + declined-shape + the tree-protocol sub-faces folded in-rule).
- [x] **T3 — author slice B (quant matmul + packs)**: 9 rules landed (entries_gpu_98.rs; per-expert-density folded into admission-clones; the 6 count-neutral riders landed on hosts by the rider agent — the 7th skipped honestly, the predicted face absent from source).
- [x] **T4 — author slice C (attention + oracles)**: 12 rules landed (entries_gpu_99.rs; dequant-dtype killed C+ thin; the int8 memo merged into the identity-memo rule).
- [x] **T5 — wiring**: mod.rs registration (97/98/99) + KERNEL_CATEGORIES (31 ids filed; the stale K comment 14→16 repaired) + router expert-total 816→847 + tests (batch242_rules_parse_and_needles_landed ×31 + batch242_riders_landed_on_hosts ×6; all historical count pins re-pinned 847).
- [x] **T6 — GOAT/G1**: corpus 404/404; kernel_opt --lib 3126/0/1i; clippy ×3 postures clean (6 doc_markdown healed via cargo refine, the B192 class); retrieval floors HELD 21/21 (297 s, no re-pin); freeze/score-bench NOT owed (zero clippy-corpus delta, P30 — write set kernel_opt-only).
- [x] **T7 — docs**: AGENTS.md ×2 bullets (Role + Feature list, 847); .docs/11_domains/kernel_opt.md Batch 242 ledger row; Research 274 status → LANDED; this plan ticked; .distill/001 landing row (below); .raw/mlx-serve removed (ephemeral-clone law).
- [x] **T8 — commit + push** (`feat: batch 242 — the mlx-serve GDN/QSA pack`).

## Yield ledger (vs Research 274)

Target: 12-16 own rules + 4-7 riders. If conversion falsification kills below 8 own rules, record the kills honestly in the landing row (the lithos disclosure precedent — landed ABOVE estimate was disclosed; landing BELOW is equally disclosed).

## Anti-goals

- No edits to `.raw/` (read-only; copy out for experiments).
- No sibling-repo edits (lane-intel addenda are recommendations recorded in Research 274, not edits).
- No batch242 corpus edits before this plan is committed (plan-first, Protocol A step 0 — satisfied by this file's commit).
