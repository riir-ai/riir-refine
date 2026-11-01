# 01 — Orientation

> **Purpose:** crate role, sibling-repo layout, boundary rationale, build
> commands, and numbering discipline. The entry point for understanding
> why this repo exists and where it fits in the 7-repo stack.

## Files

| File | Scope |
|---|---|
| [`crate_role.md`](crate_role.md) | What riir-refine is + what it is NOT (the domain test) |
| [`sibling_layout.md`](sibling_layout.md) | The 7-repo sibling layout + path-dep assumptions |
| [`boundary_rationale.md`](boundary_rationale.md) | Why this is a separate repo, not a module in riir-ai |
| [`build_commands.md`](build_commands.md) | CI guard + GOAT benches + feature combos |
| [`numbering_discipline.md`](numbering_discipline.md) | Monotonic numbers — never reused |
| [`foo_end_to_end.md`](foo_end_to_end.md) | Proposal 011 §8 onboarding walkthrough — install → login → heal → `pay --request` / `faucet --request` / `wetrun`, with each step marked runnable-now or blocked-and-why |
| [`node_tiers.md`](node_tiers.md) | The install tier spec (Ultra/Max/Pro/Lite — Proposal 013; the Max replay earn class — Proposal 015): one place to compare what a rig can run, per-tier cost/earn, and honest shipped/designed/blocked status |
| [`replay_lane.md`](replay_lane.md) | The `cargo refine replay` lane (P015 Phase 2 / Plan 127): lease → local clippy-oracle replay → signed verdict — gates, the pinned rate rule, canaries, and the honest no-settlement-yet status |
| [`corpus_pr_lane.md`](corpus_pr_lane.md) | The `cargo refine corpus-pr` lane (riir-dapps Plan 050 S1.11): the devnet→mainnet corpus PR client half — the S1.0 auto-rebase, the three-way golden-pinned wire mirror, the T10/T2b gates, the per-pair bookkeeping, and the read-only gauge for the dark era |
| [`v035_publish_attempt.md`](v035_publish_attempt.md) | v0.3.5 — PUBLISHED 2026-10-30 (sealed-token `gh-release-pat` lane) + the build-from-TAG lesson: the first upload shipped HEAD+dirty binaries (caught by the post-publish smoke test); rebuilt in a worktree at `0542da6c`, `--clobber`ed, verified |
| [`kat_billing.md`](kat_billing.md) | The KAT billing + TUNA trial-credit surfaces: the two tanks and the fixed-rate draw order, the run footer spec, the offline arc (stacking, report-time plane decision), why no debt exists, and the depleted posture (the 075 gate, the two refuel doors, the count-only survey, the 7-day payout clock) |
| [`mozilla_grant_readiness.md`](mozilla_grant_readiness.md) | Mozilla Human-Scale AI grant playbook (PRIVATE strategy — deadline 2026-11-20): program facts, per-repo eligibility matrices + verdicts, frictions, evidence inventory, consult-email draft, timeline |
| [`mozilla_evidence_pack.md`](mozilla_evidence_pack.md) | Issue 190 T3 evidence pack (2026-10-31): public-surface counters (13 releases, 201 downloads, stars/forks) + internal-usage ledger (209 score-bench runs, 96.4% heal rate, corpus counts) + the honest T10 reading |
| [`mozilla_site_audit_2026_11_01.md`](mozilla_site_audit_2026_11_01.md) | Live audit of all four fronts vs the Mozilla principles/criteria (2026-11-01, four read-only subagents): refine.riir.ai sourced from riir-dapps (not riir-refine), the economy layer is structural, riir.ai IS the network home, "private moat" public leak, /pay devnet-mint wording — input to the Plan 324 re-scope |
| [`role_flows.md`](role_flows.md) | The four Get-started roles (coder / miner / fixer / trainer) as DO → GET → COST flow diagrams + the shared cost plane + the role × tier grid; the per-tab SVG renders the front page embeds (Issue 132) |

## See also

- [`../08_benchmarks/`](../08_benchmarks/README.md) — GOAT gate methodology
- [`../09_lessons/`](../09_lessons/README.md) — canonical patterns + caveats
