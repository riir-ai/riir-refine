# Bench 134 — PRE-REGISTRATION: the per-span line-query cap (issue 208 T6) — the G5-cost tail lever, Bench-129 lever (a)

**Status:** PRE-REGISTRATION LANDED 2026-11-05 (zed-m3-i208pregate, lease on issue 208) — **NO TIMING RUN HAS HAPPENED.** Implementation-first lane: the cap lands as a code commit BEFORE any timed chunk. Nothing in this file may be amended after the first timed chunk (the T1 law). The run record is a SEPARATE later bench document (fresh number).

## The mechanism under test (from the code, not a guess)

`RuleIndex::suggest_deep_into` (src/draft/corpus_index.rs) issues **1 whole-span query + one query per significant line** (`trimmed.len() > 8`, not the whole-span duplicate). Each query pays `query_for` + the O(N=1056) two-stage corpus scan + BM25 + rank merge (Bench 129's attribution, confirmed by read this session). Kernel-fixture spans (test fns) carry tens of significant lines — the p95 tail population (31 files, Bench 132: GATE 122.5 / SCOPE 92.4 ms added vs the ≤ 5.0 ceiling). The cap bounds the QUERY COUNT per span; the whole-span query always runs.

## The pinned design

1. **Selection rule (stride, deterministic):** when a span's significant lines exceed the cap, query the lines at `floor(i * len / cap)` for `i in 0..cap` — distinct ascending indices, uniform coverage of the span, first-index-safe. Below the cap: every line (unchanged). The merge is per-rule best over all retrieved chunks, so query ORDER never affects results — selection affects only WHICH lines are queried.
2. **Scope of the cap: the kernel index ONLY.** The cap lives in the `KernelIndex` dispatch (flat arm); clippy/rust_perf/sec/docker/dist keep the uncapped path byte-identically (their retrieval floors are load-bearing elsewhere — Bench records pin them). The route-narrow lane (Issue 196's measured-negative) is NOT in this registration.
3. **Arming:** `RIIR_KERNEL_PREGATE_LINE_CAP` — promote-only env, the exact literals `4` and `2` arm (any other value = unset-equivalent, never armed). Composable with `RIIR_KERNEL_PREGATE_SCOPE` (the candidate posture is GATE+SCOPE+CAP together).
4. **Feature:** rides `kernel_pregate` (implies `kernel_opt`), default-off as before. The default and GATE/SCOSE-without-cap postures are byte-identical to Bench 132's binaries' behavior.

## The arms (one build family + the OFF reference)

| arm | posture | env |
|---|---|---|
| OFF | `latent_retrieval` | — |
| GATE | `latent_retrieval,kernel_opt,kernel_pregate` | — (same-session drift arm vs Bench 132) |
| SCOPE | same binary | `RIIR_KERNEL_PREGATE_SCOPE=1` |
| **CAP4** | same binary | `SCOPE=1` + `LINE_CAP=4` |
| **CAP2** | same binary | `SCOPE=1` + `LINE_CAP=2` |

**The promotion candidate is the LARGEST cap whose BOTH cost legs clear with the utility floor held** (larger cap = better expected utility, higher cost). A cap that clears cost but drops the utility floor FAILS (the Bench-131 kill criterion, verbatim).

## The pinned contract (unchanged)

- Corpus: this repo's `src/**/*.rs` at the implementation commit, fresh same-session walk (all five arms one session, minutes apart — the Bench-129 posture).
- Ceilings: added (ON−OFF) p50 ≤ 1.0 ms / p95 ≤ 5.0 ms per file, nearest-rank, tail support disclosed — never re-registered.
- Utility floor: 15/15 distinct rank-1 experts per ON arm; rule census disclosed per arm, never auto-demoted.
- Paired recall delta: SCOPE↔CAP arms share the span population (the paired rank-1 compare APPLIES here, unlike Bench 132's gate↔scope refuse); flips/lost/cited-set shrinkage disclosed.
- Projection arithmetic (DISCLOSED EXPECTATION, not a claim): fixture spans carry ~30–100 significant lines → CAP4 ≈ 8–26× per-span query-mass cut, CAP2 ≈ 16–51×; against SCOPE's measured 92.4 ms tail, CAP4 projects ~3.5–11 ms (marginal), CAP2 ~1.8–5.8 ms (marginal-green). The whole-span query's fixed cost + per-query text-length variance mean the projection is wide — the run decides.

## Verdict arithmetic (pre-pinned)

- The fence CLEARS iff the candidate arm's added p50 ≤ 1.0 AND p95 ≤ 5.0 with the utility floor held in that arm.
- Both cap arms FAIL p95 → the fence verdict stays FAIL and the substrate-side lever (score-only retrieval, riir-neuron-db's lane) is the named next pre-registration; no ceiling re-registration is requested or permitted (D2).
- Marginal pass (< 2× headroom over either ceiling) → re-run alone before any green (the box-state law); a confirmed marginal pass is recorded marginal.

## Kill criteria

- Any ON arm's utility floor drops below 15/15 → that arm FAILS; the cap value is the finding (a deeper cap needs its own pre-registration, never an in-place amendment).
- Build red → LOUD, the leg reads BUILD-BLOCKED, never a silent skip.

## PROVENANCE law

Bench 132's disclosure block applies verbatim (m3 box state at start/end, toolchain, commit hash, isolated target dirs, wall clock, the sequential-process caveat + the alone-rerun rule).

## Refs

- Issue 208 T6 (the lane), Bench 132 (the run that promoted this lever: p50 green, p95 the named residual), Bench 129 (lever (a)'s naming + the query-mass attribution), Bench 131 (the pregate registration — its contract carries), Bench 125 (the instrument + floors), Issue 191 (the re-campaign sequence).
