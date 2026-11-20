# Plan 392: Mining-Queue Epiplexity Allocation + Domain Saturation Gate

**Status:** OPEN — filed from katgpt-rs Research 685 + riir-refine Research 426; P0 census DONE 2026-11-20 (verdict SPARSE — univariate per-domain fit family, §Phase 0 below; spectrum CORRECTED same day, eff10 = 1); Phase 1 consumer LANDED 2026-11-20 (`examples/mining_gain_census.rs` + the gated `epiplexity_census` module + 11 gates; consumes katgpt-core `epiplexity_gain` — katgpt-rs Plan 641, landed `9151d140c` — consume, never fork; example output confirms the Sparse arm + a temporally stable kernel_opt-top queue; REPORT-ONLY until G2 passes). G2 (allocation value, the promotion bar) OPEN.
**Lease closed:** zed-m3-p392census@m3, epoch 1795182273, expiry 2026-11-22T13:44:18Z — Phase 0 census + Phase 1 consumer landed (867023c6); Phases 2–3 remain gated on the katgpt-rs Plan 641 G2 cell (open-unclaimed), nothing claimed-but-stopped.
**Priority:** fusion ladder #1 (self-evolve product stack).

## Premise

The mining queue (`.distill/001_mining_queue_snapshot.md`) orders rule-batch mining by hand/history. The epiplexity signal orders it by predicted structural gain: headroom (L̂−ε) × exposure × influence, fitted GD-free from score_history.jsonl's per-corpus-generation accuracy curves; ε-floor per domain = the saturation gate ("stop mining this domain"). The paper's evidence (beats ADO; epiplexity↔OOD r=0.88) is training-side; OUR value bar is the G2 cell below and is UNPROVEN.

## Phase 0 — census (BLOCKING, hours, CPU-only)

- [x] Mix-variation census over score_history.jsonl + batch landing logs — **DONE 2026-11-20, verdict SPARSE** (numbers below; the univariate per-domain fallback is the fit family; influence does not identify). **CORRECTED same day:** the first pass's Jacobi pairing was divergent (the +s sign with the pinned θ drives the target off-diagonal UP: 3 → 4.8 measured on a rank-1 Gram) and read rank 2 / PR 2.08; the converged spectrum reads **eff10 = 1, PR 1.392** — trace invariant rel err 2.3e-16 against the direct Gram trace. Verdict unchanged and strengthened.

## Phase 0 census results (2026-11-20, zed-m3-p392census@m3)

**Data + method.** Sources: `.benchmarks/score_history.jsonl` (254 rows, 2026-08-19 → 2026-11-13, shas in-row) + the per-domain count chain in `AGENTS.md`'s git history (the live counter per domain; parsed per sha by bullet regex — clippy `N corpus lints` (early: `N common lints`), rust_perf/kernel_opt/sec/docker/shader `N rules`, dist `N suggest-only rules`, ane `N suggestion-only seed rules`; earliest AGENTS.md uses a numbered list, handled). 196 distinct run shas → **138 unique corpus mix states** (2026-08-19 → 2026-11-13). Sanity: **zero monotonicity violations** per domain across the whole series; the last state cross-validates against `pins_manifest.json` (post-window deltas reconcile exactly: +44 rust_perf, +12 kernel_opt, +11 sec, +6 ane born after 11-13; clippy/docker/dist/shader unchanged).

**Exposure matrix** (138 unique mix states × K=8 domains from `pins_manifest.json`: clippy_lints, rust_perf, kernel_opt, sec, docker, dist, shader_perf, ane). Singular spectra (Jacobi, converged; values CORRECTED 2026-11-20 — see the checkbox note):

| variant | σ1..σ8 | eff rank (σ≥10%·σmax) | eff rank (σ≥1%·σmax) | participation ratio |
|---|---|---|---|---|
| per-column centered | 2845.2, 185.1, 153.7, 104.9, 50.8, 20.4, 11.6, 0.0 | **1** | 5 | 1.392 |

**Verdict (pre-registered bar: effective rank ≥ K−1 = 7):** under floor by EVERY standard definition (1 / 5 / 1.39) → **SPARSE — the cross-domain γ_m,k influence is structurally unidentifiable from logged score-bench history; the fit family is univariate per-domain (headroom × exposure survive, influence does not), carried as a typed `Sparse` record (exactly katgpt-rs Plan 641's pre-registered fallback shape).** Mechanism: sequential one-domain-at-a-time accumulation makes every domain's count a monotone time staircase — after de-meaning, the columns collapse onto the kernel_opt variance ramp (σ1 = 2845; the second direction σ2 = 185.1 sits at 6.5% of σmax, under the 10% share) — one accuracy scalar per state along a single temporal path cannot separate which domain's spans moved accuracy. The all-zero ane column contributes the exact 0 singular value. Nuance, for the record: the landing DELTA matrix does touch every domain independently (8 landing directions), but cumulative-sum mixing along one path destroys the separability the scalar regression needs — mix variation exists in the landing record, not in the observation structure.

**Per-domain landing census** (count-bump events over the window; batch-size min/median/max):

| domain | first → last | growth | landings | med batch |
|---|---|---|---|---|
| rust_perf | 72 → 334 | ×4.6 | 69 | 2 |
| kernel_opt | 344 → 1081 | ×3.1 | 61 | 6 |
| clippy_lints | 10 → 120 | ×12.0 | 32 | 2 |
| sec | 0 → 69 | (born 09-06) | 21 | 3 |
| dist | 0 → 8 | (born 09-13) | 4 | 1 |
| docker | 0 → 31 | (born 08-28, one seed) | 1 | 31 |
| shader_perf | 0 → 8 | (born 09-2x, one seed) | 1 | 8 |
| ane | 0 → 0 | born 2026-11-14, AFTER the last run sha | 0 | — |

Total corpus 426 → 1651 rules (+177%) over 137 landing events; docker/shader are single-seed flat domains (allocation-saturated by construction — nothing landed since birth), ane is in-window-empty.

**Accuracy points per generation** (`generation` = the M5 fixture-set family, NOT a corpus epoch: 0 = pre-tag rows, 1 = the v1 frozen atom set = the standing series, 3–8 = the ladder families — composite v3, rustc-errors, rust_perf vocabulary, real-repo, overlap-chain, shaped):

| gen | n pts | heal first → last | mean | span | ood pts |
|---|---|---|---|---|---|
| 0 | 45 | 0.8462 → 0.9623 | 0.9592 | 08-19 → 09-06 | 0 |
| 1 | 172 | 0.9623 → 0.9643 | 0.9633 | 09-06 → 11-13 | 152 |
| 3 | 9 | 0.8750 → 0.9618 | 0.9384 | 09-06 → 11-07 | 8 |
| 4 | 3 | 0.5429 → 0.5429 | 0.5429 | 09-06 → 09-13 | 1 |
| 5 | 8 | 0.6923 → 0.6923 | 0.6923 | 09-06 → 11-01 | 5 |
| 6 | 6 | 0.9524 → 0.9524 | 0.9524 | 09-06 → 09-13 | 2 |
| 7 | 9 | 0.7826 → 0.8750 | 0.8442 | 09-06 → 09-13 | 1 |
| 8 | 2 | 1.0000 → 1.0000 | 1.0000 | 09-11 → 09-13 | 2 |

Saturation reading: the standing gen-1 series is SATURATED on its own instrument (+0.002 heal over ~130 batches / +177% corpus) — the v1 ε-floor sits at ≈ 0.964 with ~3.6pp headroom; the live headroom lives in the ladder families (gen 3/4/5/7 means 0.94 / 0.54 / 0.69 / 0.84), which are themselves domain-flavored instruments (gen 4 = rustc-errors side, gen 5 = the rust_perf vocabulary ladder) — Phase 1's univariate per-domain curves ride the ladder series, not just v1.

**Consequences carried forward:** (a) Phase 1 fits univariate per-domain (α_m, ε_m only) and MUST carry the typed `Sparse` record — never a silent γ (matches Plan 641's `Saturation::Sparse`); (b) the saturation gate (ε-floor per domain) is the load-bearing half on our data — the paper's γ-shaped allocation adds nothing here; (c) future re-census: if the queue ever interleaves domains NON-sequentially against a per-domain accuracy instrument (e.g. per-domain held-out-fix accuracy rather than one aggregate), the centered rank can rise and the cross-domain fit re-opens — re-run this census before ever arming a γ fit; (d) ane joins the mix matrix from 2026-11-14 forward.

## Phase 1 — consumer wiring (behind a demo/measurement binary, not the production fixer)

- [x] `examples/mining_gain_census.rs` (or extend the existing census lane): fit + rank domains by sigmoid-tempered marginal gain each corpus generation; print the ordered queue + per-domain ε-floor status; REPORT-ONLY (exit 0, never changes the queue) until G2 passes. (LANDED 2026-11-20: the gated `epiplexity_census` src module carries `load_panel` + the two decision fns + `run_census` as data; the example is a thin printer; `--history` shows the queue temporally stable — kernel_opt top / rust_perf second at all 9 snapshots; 11 gate tests bit-identity-pin the pipeline over the committed 138-row fixture `.benchmarks/mining_gain_panel.jsonl`.)
- [x] Saturation gate: a domain whose predicted remaining gain Ŝ(∞)−Ŝ(t) sits under the bar is printed DEMOTED — the Pile-saturation law as a queue gate; if one domain dominates the area integral, print SKIP-SELECTION (the allocation machinery refuses to run). (Bars: `SATURATION_DEMOTE_HEADROOM` 0.005, `DOMINANCE_SKIP_WEIGHT` 0.85; on this fixture: none demoted, none dominant, `ane` typed `NoCurve` — not allocable.)

## Phase 2 — GOAT cell

- [ ] **G2 allocation value:** resolutions per mining-hour, gain-ordered queue vs incumbent queue order, over ≥2 corpus generations of REAL mining (paired, one frozen read, LB95 > 0; state box state per the bench_preflight law). **FAIL ⇒ demote in-landing**: the lane records a DONE-NEGATIVE and the queue stays incumbent (Research 591 precedent quoted verbatim in the verdict).
- [ ] **Admission canaries (collapse taxonomy):** for any mined-batch admission gate touched by this lane — a repetitive batch (n-gram rep ≥ ~0.4-class) and an off-manifold batch are planted REFUSAL arms; PPL-only admission is a documented negative import (the paper's rep-0.47 / frozen-scorer-ppl-169.7 fixtures).
- [ ] **G3/G4:** report-only lane (feature-off byte-identity by construction); fit cost ≤ the ~12 s-class budget; zero steady-state alloc.

## Phase 3 — promotion (only after G2)

- [ ] Wire the ordered queue into `.distill/001_mining_queue_snapshot.md` ordering as the DEFAULT lane header (the queue file gains a `gain:` column); demote switch stays documented. Warm-start gate: the gain column arms only on a corpus with a live Elo floor.

## Boundary + laws

- Consume `katgpt-core::epiplexity_gain` (katgpt-rs Plan 641) — no local re-implementation (goat-audit direction).
- Allocation surfaces only: this signal never touches scan/fix/serving paths (stationary-surface ban).
- The guide of record is `riir-refine/.research/426_Epiplexity_Guided_Mining_Allocation_Guide.md`.
