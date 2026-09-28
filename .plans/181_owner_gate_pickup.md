# Plan 181 — Owner-gate pickup execution (issue 138, refs riir-ai 1016)

**Status:** OPEN — agent-pickupable prep tasks below; E6 is a code task deferred to its own follow-up commit. (Renumbered 180→181 at rebase: a concurrent sibling landed Plan 180 batch185 on origin; per numbering discipline the landed document keeps the number.)

Refs: `../riir-ai/.issues/1016_workspace_owner_gated_decisions_summary.md` (riir-ai `e8cbd91ff`) · pair issue `.issues/138_owner_gate_pickup.md` · owner direction 2026-09-28 (4 repos deferred; no mainnet).

- [ ] E3: remove the `gist-rs/riir-unity` checkout step from `ops_dashboard.yml`; mirror set = contract repos; one HISTORY row (evidence `HISTORY.md:671`).
- [ ] E4: delete the 4 targets gated on never-defined `crowd_attention_bench` / `crowd_coherence_bench`; note in the same HISTORY row (evidence `HISTORY.md:7045`).
- [ ] E5: one-line disambiguator in `.research/083_omlx_external_distill_batch71.md`; no renumber — REC leave-as-is (evidence `.research/084_dirty_encoding_write_elision_batch72.md:127`).
- [-] E6: retire the `browser` feature — **CODE TASK — dedicated follow-up commit, this docs commit first**; bounded removal, zero consumers, keep `cargo check` green (upstream `riir-ai/.issues/958`).
- [ ] E7: record the standing CF sign-off (shared-account llama + CodeCureAgent loop) — **OWNER-GATED allow/deny**; record-only until the verdict (evidence `.benchmarks/057_glm53_flash_fixer_regate.md:42` + `.benchmarks/061_trained_drafter_phase0_stop.md:70`).
- [ ] E8: draft the one-paragraph HISTORY row for the C8 four-rev / C9 multi-rev interpretation — **OWNER-GATED adjudication**; commit draft labeled DRAFT (evidence `.distill/001_mining_queue_snapshot.md:107`).
- [ ] C7: record `structural_admission` inert-until-priced-consumer posture — record-only (P1×frontier_miner fusion is the re-open trigger).
- [x] A1-side: P015 status line carries the DATED HOLD at `replay_cut_bp = 0` (this commit).

Verification: this is a docs-only commit — no cargo; the E4/E6 follow-ups re-run `cargo check` (E6 keeps it green).
