# Issue 138 — Owner-gate pickup tasks (workspace summary riir-ai 1016)

**Status:** OPEN — pickup tasks from the workspace owner-gated summary (riir-ai 1016); execution gates per item; prep/recording tasks are agent-pickupable.

Master: `../riir-ai/.issues/1016_workspace_owner_gated_decisions_summary.md` (riir-ai commit `e8cbd91ff`).
Owner direction 2026-09-28: riir-mmorpg-examples / seal-online-remaster / seal-game-editor / sealm-toolkit are DEFERRED; ALL mainnet actions ON HOLD.
Pair plan: `.plans/181_owner_gate_pickup.md` (same commit; renumbered 180→181 — a concurrent sibling landed Plan 180 batch185 on origin while this was in flight).

- [x] **E3 — drop the dead `gist-rs/riir-unity` checkout from `ops_dashboard.yml`.** Evidence `HISTORY.md:671` (repo retired to `git/obsolete/` 2026-09-04). Mirror set = the contract repos; one HISTORY row. Docs/cfg only — no code. **DONE 2026-09-28.**
- [x] **E4 — remove the 4 targets behind never-defined features `crowd_attention_bench` / `crowd_coherence_bench`.** Evidence `HISTORY.md:7045` (referenced by `[[test]]`/`[[example]]` required-features + lib.rs cfg gates, never defined in Cargo.toml — cargo silently skips). Delete the targets; note in the same HISTORY row. **STALE-NO-OP 2026-09-28 — nothing to delete (names exist in no commit; required-features all resolve today); verification in the HISTORY row.**
- [x] **E5 — `.research/083` numbering collision: add ONLY a one-line disambiguator to `.research/083_omlx_external_distill_batch71.md`.** Evidence `.research/084_dirty_encoding_write_elision_batch72.md:127` (historical dual allocation with `083_elo_selection_gate.md`). REC leave-as-is (owner); do NOT renumber — live references break. **DONE 2026-09-28.**
- [ ] **E6 — retire the `browser` feature** (zero consumers; wasm32 dep graph pre-broken). Evidence `HISTORY.md:10184`; upstream `riir-ai/.issues/958`. **CODE TASK — do in a dedicated follow-up commit AFTER this docs commit lands; keep `cargo check` green.**
- [ ] **E7 — standing CF sign-off (shared-account llama + CodeCureAgent loop): RECORD-ONLY.** Evidence `.benchmarks/057_glm53_flash_fixer_regate.md:42` + `.benchmarks/061_trained_drafter_phase0_stop.md:70`. **OWNER-GATED (allow/deny)** — record the verdict verbatim in HISTORY when given; no action before.
- [x] **E8 — boundary-guard C8 four-rev / C9 multi-rev interpretation: draft the one-paragraph HISTORY row.** Evidence `.distill/001_mining_queue_snapshot.md:107`. **OWNER-GATED (adjudication)** — commit the draft labeled DRAFT now; the owner verdict lands in the same row. **DRAFT LANDED 2026-09-28.**
- [x] **C7 — `structural_admission` stays inert until a priced first consumer (P1×frontier_miner fusion): RECORD-ONLY.** No activation work; re-open requires the priced consumer proposal. **RECORDED 2026-09-28 (HISTORY row).**
- [x] **A1-side — Proposal 015 replay: DATED HOLD at `replay_cut_bp = 0`** added to `.proposals/015_replay_endorsement_reward_class.md` status line (this commit; basis: riir-dapps Bench 013 — a cut <2,223 bp pays verifiers nothing).
