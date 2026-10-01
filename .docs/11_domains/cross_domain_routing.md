# Cross-domain auto-routing — NOT in scope

> Each domain is selected by the caller enabling its feature flag. There is no
> "this code has both a lint AND a perf issue, which domain heals it?" router.
> That is over-engineering for now; revisit when a concrete need materializes.

## Why it's out of scope (Proposal 001 §"Cross-domain routing: NOT in scope")

The three shipped domains (clippy, perf, kernel) have **non-overlapping trigger
surfaces**:

- **clippy_lints** — triggered by `cargo clippy` output (lint codes)
- **rust_perf** — triggered by perf profiling (hot-path identification)
- **kernel_opt** — triggered by GPU kernel profiling (shape signatures)

A piece of code rarely triggers more than one domain simultaneously, and when
it does, the caller knows which domain they're targeting (you don't
accidentally profile a kernel while running clippy). Auto-detection would add
complexity for a non-problem.

## The `pick_domain` substrate IS used — within a domain

The `pick_domain<N, A>` primitive (Plan 558, katgpt-core) is used **within**
the `kernel_opt` domain to route between multiple kernel experts (coalesce /
tile / fuse / vectorize). It is NOT used across domains. The distinction:

- **Intra-domain routing** (kernel_opt experts): the domain has multiple
  experts with different intrinsic ranks; `pick_domain` picks the best one
  for the current kernel signature. This is the LatentMoE principle (Research
  453).
- **Cross-domain routing** (clippy vs perf vs kernel): the caller knows which
  domain they want. No router needed.

## When to revisit

A unified `cargo refine` CLI that auto-detects domain from code shape is a
future design sketch (Proposal 001 §Product vision), not shipped. It would
make sense when:

1. A concrete consumer needs to heal code without knowing which domain
   applies (e.g., a CI integration that runs all domains sequentially).
2. The domains have grown to the point where manual selection is a UX burden.

Neither condition holds today. The current model — caller picks domain via
feature flag — is simpler and sufficient.

## Source

- [Proposal 001 §"Cross-domain routing: NOT in scope"](../../.proposals/001_multi_domain_modelless_code_healer.md)
- [`src/domains/kernel_opt/router.rs`](../../src/domains/kernel_opt/router.rs) — intra-domain `pick_domain` use
