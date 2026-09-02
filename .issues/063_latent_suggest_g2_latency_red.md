# Issue 063 — `latent_suggest_goat::g2_latency_under_1ms_per_chunk` is RED on `develop` (pre-existing)

Status: **OPEN (2026-09-02) — DIAGNOSED and ~19% faster (`e5ffe9c`), but the absolute verdict still needs an IDLE box. Originally filed as: measured RED, cause NOT diagnosed, no fix attempted.
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

## DIAGNOSED + PARTIALLY FIXED 2026-09-02 (`e5ffe9c`) — direction 1 taken

The issue said the overrun was "NOT diagnosed" and named the fan-out as a
*suspect, not finding*. It is now diagnosed, and the suspect was **wrong**.

### The profile (temporary in-source counters, since removed)

| stage | share of `suggest_span` |
|---|---:|
| `retrieve` | **92.1%** |
| `rank_into` | 2.3% |
| embedding (`query_for`) | 0.8% |
| rerank / other | ~5% |

Per chunk: 3 `suggest_deep_into` calls (one per enabled domain), 33 `retrieve`
calls (11 texts x 3 domains — the whole chunk plus every line over 8 trimmed
chars), 951 retrieved chunks.

### Two plausible culprits measured and REJECTED first

Both came from reading the code, and both were nearly irrelevant — recorded
because the reasoning was persuasive and wrong:

1. **Hoisting `query_for` out of the per-domain loop.** It *is*
   domain-independent (except the `bonsai_retrieval` dense arm), so every line
   really was being embedded once per enabled domain. Worth **0.6%**.
2. **Content/source loading.** `rank_into` reads only `zone_hash` and the
   scores — never `content` or `source` — while `TokenBudget(usize::MAX)`
   defeats riir-rag's Issue 602 lazy loading, so the packer loads content for
   every candidate and riir-clippy discards it. Worth only **~11% of
   `retrieve`**: a tiny-budget probe cut retrieved chunks to **0** and moved
   the total by that much. The cost is in `retrieve_candidates`, which runs
   *regardless of budget*.

### The fix: an upstream option this crate had never enabled

`NeuronDbRetrieverConfig::default()` ships `two_stage_oversample: None`, i.e.
`retrieve_diverse` at **O(N·k)**. Setting it selects
`retrieve_diverse_two_stage` at **O(N + M·k)** (riir-rag Issue 593 / Bench
583). No new code — shipped substrate, unconsumed.

- **Perf: median ratio 0.806** over 11 interleaved A/B pairs, 10 of 11
  favouring two-stage. Per-pair ratios rather than median-of-medians because
  the box ran at load 8–14 and absolute wall-clock drifted ~2x during the
  session.
- **Quality: all 14 floors IDENTICAL** in both feature states; G1 identical
  (21/24, same miss list); `uncontaminated_g1` 8/8, pool recall @12 100%.
- **One real, tiny cost — it is an approximation, not a free win:**
  `t48_gp_acquisition_ab` mrr **0.961 → 0.959** with @1, @3 and McNemar
  (b=74 c=1 z=8.429) unchanged. One item moved one rank **below top-3**.
  A/B'd in ONE worktree at ONE HEAD, so it is attributed to the flag and not
  to intervening commits. This is why no gated floor moved and why
  `CLIPPY_TOP3` (zero margin) is untouched.

### Why this issue stays OPEN

**The absolute verdict is still not established.** Every measurement this
session ran at load 8–14 (sibling agents building riir-mmorpg-examples, a
riir-games test, Unity Hub), and one A/B pair even inverted (ratio 1.176) on
noise alone. Extrapolating the recorded idle baseline of 1155 µs by the median
ratio gives **~931 µs against the 1000 µs bar — probably green, NOT
demonstrated.** The bar was not touched.

- [ ] **T1 — re-measure G2 on an idle box** (load < 1, no sibling cargo runs),
      3 runs, and record the absolute figure in
      `.benchmarks/018_plan005_latent_suggest_goat.md`. If green, close this
      issue. If still red, the remaining ~19% has to come from the next lever
      down, and directions 2/3 below become live again.
- [-] **T2 — the next lever if T1 is still red: cut the 33 retrieves/chunk.**
      One retrieve per line per domain is the structure; deduping identical
      lines and skipping lines that cannot match are both quality-affecting, so
      this needs the floors re-run and is **not** a mechanical change.
      **Unblock: T1 shows the gate still red.**

## Candidate directions (as originally filed; direction 1 has now been taken)

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
