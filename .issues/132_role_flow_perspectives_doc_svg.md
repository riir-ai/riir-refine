# Issue 132 — Role-perspective e2e flows + per-tab SVGs (the "Get started" roles)

**Status:** IN PROGRESS — Phase 1 (doc + 4 SVG renders) landed this commit; Phase 2 (web delivery per tab) deferred pending a riir-dapps worker window.

Follow-up to Issue 131 (the self-evolve flywheel hero): the front page's
role-tabbed "Get started" card (coder / miner / fixer / trainer) gets the
same treatment — each role's e2e flow as a diagram that says what it DOES,
what you GET, and what it COSTS (e.g. coder: healed code across
warn / error / perf / sec / docker / compile-error domains, paid by the
burn meter with TUNA trial credit first). Doc-first per the owner ask; the
web delivery (each front-page tab embeds its SVG) follows later.

## Tasks

- [x] Phase 1 — doc: `.docs/01_orientation/role_flows.md` — four role
      sections (DO / GET / COST each), the shared cost-plane section (two
      tanks, no debt, the 075 gate's two refuel doors, the 7-day settle),
      the role × tier grid the tabs + matrix share, README index row.
- [x] Phase 1 — SVG renders: `role_coder.svg` / `role_miner.svg` /
      `role_fixer.svg` / `role_trainer.svg` in `.docs/01_orientation/` —
      canonical mermaid.ink renders, KAT dark palette, CSS scoped to
      `#mermaid-svg`, aria-labeled (the Issue 131 post-processing
      conventions).
- [ ] Phase 2 — web delivery (riir-dapps): pinned SVG mirror in the
      worker + an `include_str!` hero module + a slot in each role pane
      of `front_page.rs` (the Issue 131 `self_evolve_hero.rs` shape).
      **Deferred:** the sibling session is actively editing
      `front_page.rs` in the same worktree (its Plan 037 WIP is red
      mid-edit — the raw-string defect disclosed in Issue 131's close);
      a concurrent edit there re-runs the Issue 131 incident. Deliver on
      the next worker window (Plan 037's T3.2 deploy or once its tree is
      green), then remove this issue per noise-reduction.

## Facts pinned (verified against the live copy — do not hand-wave these)

- **Burn meter:** 1 KAT-equivalent per million code-word tokens; the
  server draws TUNA trial credit FIRST, then KAT (the B3 order,
  `kat_billing.md`).
- **Login grant:** 100 TUNA trial credit, 30 days from claim
  (`FREE_GRANT_MICRO_KAT` = 100_000_000 µ; the front page renders it via
  `render_grant()` — never hardcode the number in code or copy).
- **Miner pool:** miners are paid from **70% of every KAT the network
  burns**, at each epoch settle (credited at the proof pass, paid ≤ 7
  days — mining = processing, minted = paid).
- **Fixer:** the API drain lane over the unproven queue runs TODAY as
  the **operator's** fixer (its accepted work settles through the miner
  rows on the board). Third-party replay verification is DESIGNED
  (Proposal 015) and does **not** settle — `replay_cut_bp` genesis 0;
  never advertise replay earnings until it can settle.
- **Trainer:** stake 1K KAT, host the daily trainer window — **ours only
  at launch** (vessel/stake machinery matures first); local models stay
  optional + UNARMED (the `l4_fallback.md` reopen bar).
- **Law:** capabilities-only per P013 — a diagram claims only what
  settles today; designed lanes render dotted and say so.

## Refs

- Issue 131 — the self-evolve flywheel hero (the shape this follows)
- `.docs/01_orientation/node_tiers.md` — the TIER axis of the same facts
- `.docs/01_orientation/kat_billing.md` — the billing plane (the cost
  edges all point here)
- `.docs/01_orientation/replay_lane.md` — the fixer replay lane's gates
