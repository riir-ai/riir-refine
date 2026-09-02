# Issue 063 — `latent_suggest_goat::g2_latency_under_1ms_per_chunk` is RED on `develop` (pre-existing)

Status: **OPEN (2026-09-02) — measured RED, cause NOT diagnosed, no fix attempted.
Filed as a record, not a work order: the bar is a Plan 005 Phase 3 GOAT gate and
the overrun is ~16–20%, so the verdict (raise the bar / optimize the path /
re-scope the gate) is a decision, not a mechanical fix.**
Branch: `develop` (per global rule — no feature branches)
Gate: `tests/latent_suggest_goat.rs::g2_latency_under_1ms_per_chunk`
(`#![cfg(feature = "latent_retrieval")]`, `#[cfg_attr(debug_assertions, ignore)]`
— it only runs in a **release** build with `latent_retrieval` on)
Recorded in: `.benchmarks/018_plan005_latent_suggest_goat.md` (the G2 row)

## The measurement

| axis | value |
|---|---|
| bar (asserted) | `per_chunk_us < 1000.0` |
| measured, this box, 2026-09-02 | **1155 µs/chunk** (480 chunks) |
| measured range across runs | **1155 – 1203 µs/chunk** |
| overrun | **+15.5% to +20.3%** over the bar |

Reproduce (isolated target dir — the workspace target is contended):

```bash
CARGO_TARGET_DIR=/tmp/riir_clippy_g2_confirm cargo test --release \
  --features latent_retrieval --test latent_suggest_goat \
  g2_latency_under_1ms_per_chunk -- --nocapture
```

Output of the 2026-09-02 run:

```
G2 latency: 1155µs per chunk (480 chunks)
G2 FAIL: 1155µs/chunk ≥ 1ms
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
```

**Instrument note (do not repeat this mistake):** piping the run into `tail`
made the shell report **exit code 0** for a FAILED test. Read the `test result:`
line, never the pipeline's status — the same class as the recorded
"PowerShell pipelines report failed builds as exit 0" lesson.

## It is PRE-EXISTING, not caused by recent work

Verified from a **worktree pinned at commit `ce9d395`** — a revision that
predates the 2026-09-02 Proposal 011 work entirely (Plan 062 / T1.1 / T1.3 /
T0.2b / T1.4 did not exist there). The gate measured **1193 µs/chunk** at that
revision, i.e. red by the same margin. Nothing in the KAT/payreq/account
surface is on this path: the gate exercises `LatentMatcher::suggest_span`
(`draft::LatentMatcher`, `latent_retrieval`), which shares no module, no
feature and no dep edge with `kat_service`/`kat_transport`.

## What is measured and what is NOT

- **Measured:** the gate is red, reproducibly, at two independent revisions.
- **NOT measured / NOT diagnosed:** which of route / embed / fan-out
  multi-scale retrieve owns the overrun; whether the bar was ever green on
  this box's hardware or only on the box it was authored against; whether
  corpus growth moved it (the corpus has grown substantially — `--corpus-stats`
  reported kernel_opt 422 / rust_perf 97 / clippy_lints 45 on 2026-09-01, and
  a fan-out over a larger index is the obvious suspect, **suspect, not
  finding**).
- **Load caveat:** the 2026-09-02 run happened with 3 other agents active on
  the box. Load can only make a wall-clock number WORSE, so it cannot explain
  the gate away — but it does mean 1155 µs is an upper bound on this
  revision's true cost, not a clean figure. Any fix attempt must re-measure on
  an idle box before claiming a delta (the load-flaky-timing-tests rule).

## Candidate directions (none evaluated — deliberately)

1. **Optimize the path.** Profile `suggest_span`; the fan-out over a grown
   corpus is the first suspect. If the win is real the gate goes green with no
   spec change — the preferred outcome.
2. **Re-derive the bar.** 1 ms/chunk was set in Plan 005 Phase 3 against a
   much smaller corpus. A bar that no longer describes the shipped index is
   miscalibrated, and the honest fix is to re-derive it **and say so in the
   benchmark doc** — never to nudge the constant until it passes (the
   "gate verdicts decided by orthogonal quantities" lesson).
3. **Re-scope the gate.** If per-chunk latency at this corpus size is not the
   product-relevant quantity (the `--suggest` path is interactive, not a hot
   loop), the gate may belong on a different metric. That is a spec decision.

**Do not** loosen the bar as a side effect of unrelated work. Whoever takes
this must pick one direction explicitly and record the evidence.

## Blast radius while it stays red

`scripts/full_gate.sh` / `cargo clippy --workspace --all-targets --all-features`
COMPILE this target but do not run it, so the lint gates stay green. Any
`cargo test --release --features latent_retrieval` sweep that includes this
binary is red until this is closed — which is why it went unnoticed: the
default and debug test paths both skip it (`debug_assertions` ignore + the
`latent_retrieval` cfg), the exact "a cfg-gated test compiles to nothing and
reports 0 passed" shape.
