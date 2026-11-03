# Issue 199: sweep the deprecated "heal" vocabulary from doc surfaces — the rulewright/rulegram/refinery coins

**Status:** CLOSED 2026-11-03 — all sweeps landed: Tier A `771e7ad15`+`1862743`+`47e71ce78`; fleet-wide Lint-fixing act `c6d258115` (16 sibling AGENTS.md files, per-repo hashes in the row below, landed by the sibling idle lane); Tier B deep book + skills + derived flow artifacts `f9727c207` (41 files — the between-batches window came same-day: both miner batches discharged, contested paths clean). One open residue: the KAT node-tier rows, owner-call

## Sweep legend (what the sweep replaced vs intentionally kept)

- REPLACED: the product noun ("code healer"/"the healer" → **rulewright**), heal-verbs in craft prose (→ fix/draft phrasing), "X healing domain" → "X fixing domain", "heal-seat" → "rerank-seat".
- KEPT — code identifiers + file names: `HealDomain` / `HealPipeline` / `HealEntry` / `heal()` / `heal_with_pool` / `perf_heal_demo` / `heal_demo` / `watch_heal.sh` / `HEAL_*` envs / `heal_domain_trait.md` / `heal_pipeline.md` / the retired `clippy_healing/` git-history paths / Proposal-001/005 + Plan-005 + Proposal-034 record file names.
- KEPT — metrics + history: the `heal_rate` score-curve axis (schema), recorded bench run rows ("heal 84.6%…"), the mozilla evidence pack's "96.4% heal rate" (historical evidence record), batch-landing rule descriptions inside the README history table (e.g. Batch 157 "receivers heal `.get(n)`"), doc-sync historical run-log rows.
- KEPT — different "heal": MAPE-K self-healing (`MapeKLoop` self-heal cascade) is riir-neuron-db's autonomic-loop architecture term, not the refine product word.
- KEPT — owner-call: KAT plane vocabulary ("Healer client", "heal-proof hat", "KAT heal service network", "heal data/ingest").

## Landed (Tier A)

- [x] riir-refine AGENTS.md — 19 prose sites to the coins (`771e7ad15`)
- [x] riir-refine README.md — 25 identity/craft prose sites; identifiers/metrics/history intentionally left (`771e7ad15`)
- [x] riir-refine BOUNDARY.md — the domain-test line mirrors AGENTS L8 (`771e7ad15`)
- [x] riir-refine .docs/01_orientation identity docs — crate_role, boundary_rationale, sibling_layout, folder README (`771e7ad15`)
- [x] heal-seat → rerank-seat — instinct AGENTS.md L160 (`1862743`; refine AGENTS.md grep-verified to carry no heal-seat lines anymore; the example names `instinct_heal_train` / `heal_specialist_server` / `HEAL_HEAD_PATH` stay — code identifiers)
- [x] katgpt-rs skills — doc-sync (census row + Step-4 craft row), build-hygiene (description + perf law + 2 prose), research (10 sites incl. the MOAT table + routing rule) (`47e71ce78`)
- [x] riir-refine — this issue + highwater (199)

## Sweep (Tier B — LANDED `f9727c207`)

- [x] riir-refine `.docs/` deep book — ALL 12 folders swept (41 files total incl. the 11_domains batch surfaces + 02_corpus + 07_examples; tree was clean with both miner batches discharged — the between-batches window arrived same-day). Round-1 + round-2 phrase sets + hand-pass; residual grep-verified down to the keep-list only.
- [x] riir-refine skills (`cargo-refine`, `riir-refine` idle-loop, `distill`, `outbound-pr`, `refine-e2e-doctor`) — swept; the `fix: heal <repo>` commit convention → `fix: refine <repo>`
- [x] Derived flow artifacts kept in lockstep — `role_flows.md` / `role_miner_flow.walk.json` / `role_coder_flow.walk.json` say the same thing ("fix run", "fix burn", "fixing was always local"); kat_economy TUNA "heals only" untouched (KAT plane)
- [x] Fleet-wide "## Lint healing" headers — landed by the sibling idle lane (row below)
- [ ] KAT node-tier rows "Healer client" + "heal-proof hat" (riir-clippy Proposal 005; riir-ai `.docs/01_orientation/node_tiers.md`) — **OWNER-CALL** (the sweep's one open residue; sed patterns constructed to not touch it)

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

## Landed (coin landing — 2026-11-03 earlier pass)

- [x] riir-ai glossary — new "Developer tooling (riir-refine — the rulewright)" section, 3 rows (`4d185a863`)
- [x] riir-ai glossary — grammaturgy row + twinarm row now name the rulewright (`4d185a863`)
- [x] riir-ai twinarm_pipeline.md §10 — the healer row → the rulewright row (`4d185a863`)
- [x] riir-refine — this issue + highwater bump

## Explicitly NOT in scope

- Code-level `HealDomain` / `HealPipeline` / `heal()` / `heal_with_pool` — API
  stability; consumers re-export at historical paths. A code rename is a
  separate owner-gated act.
- Feature names (`fix_verify`, `clippy_verify`, …) — the "verify ladder"
  vocabulary already carries them.
