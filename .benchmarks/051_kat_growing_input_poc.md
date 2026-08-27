# Bench 051 — Issue 049 growing-input conditioning POC (NEGATIVE-no-gain)

**Status:** CLOSED 2026-08-27 — verdict `NEGATIVE-no-gain` per T3's own rule
**Issue:** [.issues/049_kat_growing_input_conditioning_poc.md](../.issues/049_kat_growing_input_conditioning_poc.md) (removed at close)
**Source:** katgpt-rs [R512](../../katgpt-rs/.research/512_Meta_n_Fixed_Omega_Growing_Input_PASS.md) (Meta^n, arXiv:2608.24735 — "the gain comes from giving the improver more to read")

## What shipped (T1 — permanent, arms A/B untouched)

- `cf_workers_ai::prompt_growing(module, modelless, fewshots, trajectories,
  failure_patterns)` — arm B's pre-pass prompt + the two Meta^n growing
  channels spliced before the closing line:
  - **Channel 1** — prior trajectory outcomes for shape-similar inputs
    (`TrajStore::retrieve(module, "", 3)`, Certified first, Withdrawn
    excluded by retrieve itself), rendered as `[tier: N ok / M failed]`
    before/after pairs.
  - **Channel 3** — the per-rule failure-pattern summary over the whole
    durable store (attempts, kept-rate, up to 2 representative REJECTED
    pairs) — the depth≥3 structured-summary analog.
  - Channel 2 (the modelless candidate) already ships in arm B.
- `RegateHarness { store: Option<TrajStore> }` + `trajectory_context()` —
  best-effort store open at `<repo>/.heal/trajectories.kv`; both channels
  degrade to empty (arm G ≡ arm B byte-identically, pinned by unit test).
- Runner arm `--arm G` wired (dispatch, prompt select, JSONL
  `growing_channels_empty` per row, DRY golden-hash line). Unit test:
  `prompt_growing_carries_both_channels_and_keeps_baselines_intact`.
- DRY validation: channels **live** on all 9 reachable spans (the repo
  store feeds real cross-rule content), planted controls ALL PASS.

## The measurement (T2 — contemporaneous A/B/G triple, llama-3.3-70b-fp8-fast)

9 REACHABLE spans, same model, same day, same token, run back-to-back
(2026-08-27 ~17:35Z):

| arm | strict-keep | keep_oracle | regressions | vetoed | divergent | prompt tok | $ |
|---|---|---|---|---|---|---|---|
| A (raw) | 5/9 | 6/9 | 3 | 1 | 0 | 820 | 0.0014 |
| B (pre-pass) | 5/9 | 6/9 | **1** | 1 | 0 | 3,128 | 0.0024 |
| G (growing) | 5/9 | 6/9 | 2 | 1 | 0 | **8,800** | 0.0049 |

Per-case (strict): A/B/G keep different-but-equal-sized sets — G shares
B's `large_enum_variant` win over A, recovers A's `vec_init_then_push`
keep that B lost, and LOSES `manual_strip` (a keep both A and B had; the
merged fix simply did not resolve the lint — findings 1→1, not a veto).

Never-wrong held everywhere: **0 divergent, 0 behavioral violations, the
contract veto fired correctly on `or_fun_call` in every arm** (the same
destructive generation class across arms).

## Verdict (T3): `NEGATIVE-no-gain`

- Strict-keep ties across all three arms (5/9). G costs **2.8×** arm B's
  prompt tokens for zero strict-keep gain. Per the issue's own rule: no
  gain → record the negative + close; the conditioning finding is
  regime-bound to Meta^n's cross-task scale, honestly noted.
- T4 (propagate a positive to R512) is moot.

## The load-bearing secondary finding (measurement discipline)

**Bench 049's arm-A-vs-B reachable-set difference (5 vs 6) was within
serving variance.** Today B scores 5/9 under identical conditions — at
n=9, ±1 case is noise (temp-0 llama serving is not run-to-run stable;
yesterday-vs-today B flipped exactly one case). Consequences:

1. Single-run A/B comparisons on the 9-span reachable set CANNOT resolve
   <2-case differences — the v2edit partial-credit axes at n=408 remain
   the robust basis for arm-level claims (Bench 049's own B-dominance
   evidence lives THERE, and stands).
2. Any future re-gate on this set needs either paired multi-run medians
   or a bigger span population.

## Scope + cost

- The POC's channel content is cross-rule (the reachable spans' lints are
  off-corpus by construction — no same-rule history exists anywhere); the
  channels still grew the prompt with real store content (the Meta^n
  mechanism was given something real to read).
- Total spend: $0.0058 of $1.00 daily cap (27 calls incl. baselines;
  ~446+220+164 client-side neurons ≈ well under 1k — the Proposal 005
  headroom rule respected; account day started EMPTY).
- Zero GPU involvement (no Bonsai arms — they stay blocked-optional per
  the issue's own status).

## Disposition

- Arm G + the composer ship as the reproducible artifact (the `gemma2_directions`
  / Issue-026 negative-result precedent): re-runnable when the store grows
  adoption-era rows (channel 1 then carries SAME-rule history — the
  mechanism's honest best case, unmeasurable today with n=8 store rows).
- Reopen triggers: the store reaching same-rule coverage on reachable-class
  lints, or a second consumer asking for the growing prompt by default.
