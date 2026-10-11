# Bench 114 — the Issue-161 pool widening + the T1.6 UNBLOCK: deciding 3 → 1416

**Status: PASS (the T1.6 case-supply gate unblocks)** — 2026-10-11, m3 (M3 Max, macOS), AC.
The direct re-run of Bench 113's census after the Issue-161 substrate landed; same tool, same
population, same pinned power inputs (δ₀ = 0.05, α = 0.05, power 0.80 — never lowered).

## The landing (three changes; one widening, two instrument repairs)

| change | where | what |
|---|---|---|
| **Pool birth point 3** | `heal_inner` (self_evolve) | `propose_fix(rule, module)`'s output joins the POOL ONLY — never modelless selection (`heal_with_pool.result` stays bit-identical to `heal`; plain `heal` pays one branch; G4 untouched). Singleton-on-miss pools expose the true pool. Deduped. |
| **`rewrite` merge strategy** | `apply_edit` (cf_workers_ai) | A response that parses as a file and preserves every top-level named item (`use` exempt) merges WHOLE. The snippet strategies mangle real module rewrites: `substitute_fn_spans` splits on BLANK LINES (a body-internal blank line bails the `?`), then LCS line-merge drops mid-item lines into parseable-but-broken modules. |
| **behav driver real-module fixes** | behavioural_equivalence | (a) domain extension: floats (signed-zero/NaN/inf corpora), non-u8 int slices, `Vec<int>`, `Option<float>`; (b) private-fn visibility — real modules declare private `fn`s and every driver call read E0603 (the t51 fixtures were all `pub fn`, so this never surfaced); the driver publicizes both sides' top-level fns (name-anchored, col-0, identical transform, compile-fail = honest Unsupported). Plus the pilot tally repair: `deciding_full` was structurally unreachable (it required ALL candidates behav-scored, but behav only ever runs on gold_a∧c candidates) — now confirmed-based, and the census carries `p_behav_ok_given_gold_ac`. |

En route: `tests/t40_fixer_regate_harness.rs` repaired — broken on develop since the 2026-09-18
REACHABLE re-pick (`ptr_arg` → `unused_self`) left its planted controls referencing a removed
fixture id; controls re-based on `manual_strip` (probe-checked to FIRE; the `clear_with_drain`
and `unused_self` re-picks do NOT fire on their fixture shapes — decline-only fixtures).

## How the instrument bug was found (the honest chain)

1. Widened census #1: deciding 455 (< 621) — proposer members merged but read `not_compiled`
   (91/91 on single_match).
2. Direct probe (`propose_fix` + `verify` on the same modules, no merge): fires + compiles +
   resolves ≈ 25/25 on 11/12 lints — including the "no-proposer" needless_borrow and
   unnecessary_mut_passed (suggest-only-tier proposers exist; FIXABLE_LINTS is --fix
   eligibility, not proposer existence; useless_conversion is the one lint with none).
3. Conclusion: the MERGE was destroying faithful module rewrites → the `rewrite` strategy.
   Census #2: deciding **1416**.

## The census (committed: `114_issue161_pool_widened_harvest/`)

- 1500 modules (1494 with a firing target), pool≥2 **1446** (was 643)
- strata: **deciding 1416** (was 3) · none-gold 29 (28 useless_conversion — the only lint with
  no proposer; 1 redundant_field_names) · all-gold(unscored) 1
- split (lint-disjoint): test 666 / 1445 scored
- clause-(b) pilot (40 cases, 40 gold_a∧c candidates): **22 ok / 0 divergent / 18 unresolved**
  → **P(behav-ok | a∧c-ok) = 0.55**, deciding_full = 22. Zero divergences — the proposer
  member is never behaviourally broken on the settled cases; the unresolved 18 are the
  driver's remaining domain gaps (Result params, custom types — disclosed, never gold).
- T1.6 N(p_d): 0.20 → 621 · 0.35 → 1092 · 0.50 → 1562. **1416 clears the p_d=0.20 bar at
  2.3× AND the p_d=0.35 bar; not 0.50.** Full-oracle projection: 1416 × 0.55 ≈ 779 ≥ 621 —
  the p_d=0.20 bar clears even under the three-clause oracle. Verdict: **T1.6 PASSES** —
  Plan 225 T2/T3 re-enter scope (owner-gated as before).

## Laws held

- The rerank wire is unchanged (options-in/pick-or-abstain); pool composition transparent to
  the seat; A0 never demoted; every widened member passes the same three-clause oracle.
- No second harness: the HARVEST arm is the census instrument (re-run deterministic, ~25 min).
- `heal_with_pool.result` ≡ `heal` — pinned by the agrees-test; G4 (alloc-free `heal`) green.

## Artifacts

- `fixer_regate --arm HARVEST --source v2edit_all --harvest-behavioural 40 --out <dir>`
- [`114_issue161_pool_widened_harvest/census.json`](114_issue161_pool_widened_harvest/census.json)
  · `split.json` · `cases.jsonl` (local; sha256 `5a7a9c280975ee78…` — deterministic re-run
  reproduces it: same population, same replay order, same memory-warming shape)
- Follow-ups (non-blocking): a `useless_conversion` proposer (76 cases at 0 deciding); the
  oracle-feed pool half stays UNLANDED by design (the with-feed posture's MachineApplicable
  spans are P21's, already applied verbatim — feed-candidates-as-pool-members would model a
  posture that does not exist).
