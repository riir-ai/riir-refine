# 01 — Orientation

> **Purpose:** crate role, sibling-repo layout, boundary rationale, build
> commands, and numbering discipline. The entry point for understanding
> why this repo exists and where it fits in the 7-repo stack.

## Files

| File | Scope |
|---|---|
| [`crate_role.md`](crate_role.md) | What riir-clippy is + what it is NOT (the domain test) |
| [`sibling_layout.md`](sibling_layout.md) | The 7-repo sibling layout + path-dep assumptions |
| [`boundary_rationale.md`](boundary_rationale.md) | Why this is a separate repo, not a module in riir-ai |
| [`build_commands.md`](build_commands.md) | CI guard + GOAT benches + feature combos |
| [`numbering_discipline.md`](numbering_discipline.md) | Monotonic numbers — never reused |
| [`foo_end_to_end.md`](foo_end_to_end.md) | Proposal 011 §8 onboarding walkthrough — install → login → heal → `pay --request` / `faucet --request` / `wetrun`, with each step marked runnable-now or blocked-and-why |
| [`node_tiers.md`](node_tiers.md) | The install tier spec (Ultra/Max/Pro/Lite — Proposal 013; the Max replay earn class — Proposal 015): one place to compare what a rig can run, per-tier cost/earn, and honest shipped/designed/blocked status |

## See also

- [`../08_benchmarks/`](../08_benchmarks/README.md) — GOAT gate methodology
- [`../09_lessons/`](../09_lessons/README.md) — canonical patterns + caveats
