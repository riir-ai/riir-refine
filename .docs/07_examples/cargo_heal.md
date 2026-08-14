# cargo-heal — the unified healing CLI (Plan 005 Phase 2)

**Source:** [`src/bin/heal.rs`](../../src/bin/heal.rs)
**Feature:** `latent_retrieval` (required — the matcher + indexes live behind it)
**Run (in-repo):**

```sh
cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/
```

Installed via `cargo install --path . --features latent_retrieval`, cargo's
subcommand convention makes it `cargo heal --suggest src/`.

## What it does

`--suggest` mode (the MVP): walks real `.rs` files (std-only recursive
walker — no new dep; skips hidden/`target`/`vendor`), chunks them at
fn/impl/struct boundaries, retrieves matching corpus rules via the latent
substrate, and prints top-K rules per span with scores + the
fixed/optimized pattern snippet head for manual review. The Phase 4
rust_perf `propose_fix` proposers landed as library code; a `--fix` CLI
mode is Phase 6.

```text
cargo heal --suggest [--domain <auto|clippy|perf|kernel>] [--top-k <N>] <path>...

--suggest    Suggest mode: print matching rules per code span (no edits).
--domain     Filter output to one domain (default: auto — all enabled).
--top-k <N>  Rules per span per domain (default 3).
<path>...    Files or directories to scan (.rs files only).
```

## Domain gating (T2.4)

Domains are feature-gated: clippy + perf are default-on; kernel needs
`--features kernel_heal`. Requesting a domain whose feature is off is a
**hard error** listing the enabled set (verified on `--domain kernel`
without `kernel_heal`).

## Smoke + GOAT

- Smoke-verified on this repo's own `src/` — 19 files, ~0.4 s total
  including index build: real spans + scores + suggestions (Plan 005
  Phase 2 exit criterion).
- The retrieval layer behind the CLI passed its GOAT gate — G1 100%
  (12/12), G2 349 µs/chunk, G3 104/104, G4 scratch stable — see
  [Bench 018](../../.benchmarks/018_plan005_latent_suggest_goat.md) and
  [`../03_drafter/latent_suggest.md`](../03_drafter/latent_suggest.md).

## Caveat

No minimum-score threshold yet — every span prints suggestions (scores are
relative). Treat output as a ranked review queue, not a verdict.

## See also

- [`../03_drafter/latent_suggest.md`](../03_drafter/latent_suggest.md) — the retrieval layer this CLI wraps
- [Plan 005](../../.plans/005_real_file_span_healing.md) Phase 2 — the CLI tasks
