# Issue 199: sweep the deprecated "heal" vocabulary from doc surfaces — the rulewright/rulegram/refinery coins

**Status:** OPEN — coins landed riir-ai-side `4d185a863` (2026-11-03); this issue owns the cross-repo doc sweep

## Premise

Owner deprecated "heal/healer" (2026-11-03): the metaphor carries no mechanism —
nobody heals code, and the word encodes none of mine/rate/draft/seal. Three
coins owner-approved the same day, built from the glossary's own coin families:

- **rulewright** — the PRODUCT (the `-wright` maker family, questwright lineage):
  the maker that works rules — mines/distills them (L0), drafts fixes from the
  corpus-as-model (L1), parse/compile-verifies (L2), ruliology-searches (L3),
  Elo-rated (score-bench, `elo_k_eff`); L4 LoRA stays the deliberately inert
  escape hatch.
- **rulegram** — the ARTIFACT (the `-gram` family, topologam/todogram lineage;
  the act/artifact split is load-bearing the way topologammar/topologam is):
  one sealed epoch of the rated rulebook — the unit the corpus lease/update
  machinery ships (epoch + per-segment BLAKE3s, atomic ADD-ONLY swap).
- **the refinery** — the PLANE (bin-faithful, descends from `cargo refine`):
  the whole plant — crude rules in (mining/distill), refined stock out (rated
  rulebook, corpus leases, drafted fixes).

Collision checks run before the offer: `rulelora` TAKEN (Plan 244
`RuleLoraParser` — rules parsed INTO LoRA weights, a different shipped
mechanism), `rule_graph` TAKEN (the decisions vocabulary), `codex` rejected
(external resonance), `refinery`/`rulewright`/`rulegram` grep-clean
workspace-wide at coin time.

## Landed

- [x] riir-ai glossary — new "Developer tooling (riir-refine — the rulewright)" section, 3 rows (`4d185a863`)
- [x] riir-ai glossary — grammaturgy row + twinarm row now name the rulewright (`4d185a863`)
- [x] riir-ai twinarm_pipeline.md §10 — the healer row → the rulewright row (`4d185a863`)
- [x] riir-refine — this issue + highwater bump

## Sweep (doc surfaces only; each task names its repo)

- [ ] riir-refine AGENTS.md — replace prose "healer/heal" mentions with the coins (grep first, per-section; code identifiers stay)
- [ ] riir-refine README.md + the `.docs/` book — same treatment
- [ ] riir-refine "heal-seat" informal name (the Plan 225 rerank lane) → "the rerank seat" in refine docs; instinct-side `ext_seat_rerank` prose likewise
- [ ] katgpt-rs skills prose mentioning "the healer" (doc-sync / build-hygiene / research SKILL.md) — cosmetic; rides those files' next edits
- [ ] KAT node-tier rows "Healer client" + "heal-proof hat" (riir-clippy Proposal 005; riir-ai `.docs/01_orientation/node_tiers.md`) — **OWNER-CALL**: service-plane vocabulary from a proposal, not freeform prose

## Explicitly NOT in scope

- Code-level `HealDomain` / `HealPipeline` / `heal()` / `heal_with_pool` — API
  stability; consumers re-export at historical paths. A code rename is a
  separate owner-gated act.
- Feature names (`fix_verify`, `clippy_verify`, …) — the "verify ladder"
  vocabulary already carries them.
