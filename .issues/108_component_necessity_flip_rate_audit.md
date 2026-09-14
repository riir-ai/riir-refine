# Issue 108: Component-necessity flip-rate audit (CVRR Research 555 follow-up)

**Status:** OPEN — T1 seam audit DONE 2026-09-15 (the `heal_fixpoint_with`
iteration loops; see the T1 audit section). T2 harness DONE 2026-09-15
(`--necessity-bench` + `ComponentOverride`/`heal_fixpoint_cfg` + the
before-verdict cache; smoke e2e validated, both canaries green). T3–T4
pending (the full measurement pass + the table). Filed 2026-09-15 (deferred
from the 2026-09-14 CVRR session — blocked then on a sibling mid-batch
checkout, filed now that the repo pulls clean).

**Lineage:** [katgpt-rs Research 555](../../katgpt-rs/.research/555_CVRR_Latent_Necessity_Strict_Interface.md)
(CVRR, arXiv:2609.06746 — "informativeness ≠ use") · katgpt-rs Issue 776
(contrastive matched-swap + norm-matched noise interventions) · riir-ai Issue
953 (the think-brain necessity audit — the game-runtime consumer of the same
substrate).

## Problem

The healer's per-rule heal rate reports how often a rule's fix LANDS, not
whether the rule's contribution is **load-bearing** in the pipeline's
composition — the CVRR distinction: a component can be *informative*
(correlated with resolved diagnostics) yet *decorative* (removing or
swapping it changes nothing downstream). Nothing measures the flip-rate
consequence of a component's absence/swaps at the composed-pipeline level.

Concretely: with rule A disabled (or swapped with a contrastive donor
rule's parameters), does the pipeline's resolved-rate actually move? A rule
whose disable/swap moves nothing is decorative in composition — its solo
heal rate is measuring a lane the composition routes around.

## Proposal (report-first, no gate until measured)

Consume the shipped intervention substrate — do NOT re-implement:

- `katgpt_core::faithfulness::perturb::{perturb_matched_swap, perturb_empty}`
  (Issue 776) applied at the per-rule / per-proposer component seam;
- `katgpt_core::interpolation_geometry::InterventionReport::{latent_is_causal,
  flips_to_donor}` for the verdict shapes;
- the score-bench fixture corpus as the deterministic replay ground
  (`.benchmarks/score_history.jsonl` generations pin reproducibility).

Deliverable: a **component-necessity table** over the clippy_lints +
rust_perf corpora — per rule: solo heal rate (known) vs composed flip-rate
under (a) empty (disabled), (b) matched-swap with the highest-co-occurrence
donor rule. Report decoratives by name. Two-sided canaries mandatory (the
audit-instrument law): a known-load-bearing rule (a top-5 heal-rate rule
with downstream dependents) MUST flip; a planted no-op control MUST be
flagged.

## T1 seam audit (2026-09-15, read-only — the answer)

**The seam: the two rule-iteration loops inside `heal_fixpoint_with`
(`src/score_bench/mod.rs`, the `for rule in FIXABLE_RULES` + `for lint in
FIXABLE_LINTS` loops).** Every bench composition path routes through this one
function — v1/v2/v3/v6/v7/v8 clippy generations via `run_bench_with` →
`heal_fixpoint_with`; v5 perf via `run_score_bench_perf` → `heal_fixpoint`.
One override parameter there covers both corpora.

Why this is the right seam (verified against source):

1. **The dispatch functions are already total.** `collect_rule_edits`
   (clippy, `proposers/mod.rs`) and `collect_edits_opts` (perf,
   `rust_perf/proposers/mod.rs`) are flat matches ending `_ => Vec::new()` —
   a rule absent from the iteration list behaves EXACTLY like an unknown
   rule: zero edits, no error, no panic. "Disable" = skip in the loop;
   "swap" = dispatch A's slot to donor B's collect fn.
2. **In-loop single-rule skip precedent exists**: the `fxhashmap` dep-gate
   skip (`if *rule == "fxhashmap" { continue; }`) — production-loop rule-level
   disabling driven by an environment property, already shipped.
3. **Parameterization precedent exists**: `candidates: &[(&str, &str)]`
   threads `run_bench_with` → `heal_fixpoint_with`, empty = byte-identical
   (`heal_fixpoint(source) == heal_fixpoint_with(source, &[])` is the
   shipped contract). A `ComponentOverride { disabled, swap }` parameter
   threads identically; the empty override MUST stay byte-identical (the
   T2 non-vacuity pin).
4. **The corpus build is untouched by construction**: `build_seed_corpus()`
   is a static literal table compiled in; `FIXABLE_LINTS`/`FIXABLE_RULES`
   are const arrays READ by the loop, never rebuilt. The retrieval/suggest
   lane (`rule_book`) and self-evolve trajectories are separate surfaces —
   the harness must not feed the trajectory store (the `nonergodic_poc`
   measurement-only precedent).
5. **The composed outcome is what the bench already measures**: resolved-rate
   via the clippy oracle (v1 family) / the mechanical shape oracle
   (`v5_persists`, gen-5) — the downstream the CVRR flip-rate needs.
   Chain-truth gates (v7/v8 expected passes) run the UNOVERRIDDEN fixpoint
   and stay green — the harness never feeds them.

**Production-loop note**: the `--fix` loop in `src/bin/heal.rs` (main fix
pass + `reheat_fix_pass` + `scored_cycle`) has the same shape (`lint_set`
Vec + the `fxhashmap` skip), but the audit harness must NOT run against
production paths — T2 stays in `score_bench` behind the `score_bench`
feature, measurement-only.

**Substrate mapping (verified to exist in katgpt-rs @ `be4ff672`, Issue 776
DONE — the handoff's "776 open" was stale):**

| CVRR arm | katgpt-core substrate | healer-seam analog |
|---|---|---|
| empty (disable) | `faithfulness::perturb::perturb_empty` (buffer → Default) | rule skipped in the iteration loop; contribution = ∅ |
| matched-swap | `faithfulness::perturb::perturb_matched_swap` (coherent whole-state swap, same-shape donor) | rule A's dispatch slot redirected to donor B's `collect_edits` on the same source state |
| verdict: load-bearing | `interpolation_geometry::InterventionReport::latent_is_causal` | Δ(resolved-rate) under intervention ≠ 0 |
| verdict: flips-to-donor | `InterventionReport::flips_to_donor` | outcome vector moves toward the donor arm's outcome |

Honest caveat for T2: the katgpt-core operators are buffer-level (`&mut [T]`);
the healer's fix lane is dispatch-level. What T2 consumes is the intervention
PROTOCOL (the arm semantics above) + the verdict-predicate SHAPES — not the
literal buffer ops. The literal ops apply only if the retrieval lane's ternary
embedding matrix is ever audited (a different axis: retrieval routing, not fix
composition — out of 108's scope).

**Canary candidates for T2's two-sided gate** (empirical pick happens in T3):
- Load-bearing: `uninlined_format_args` (the highest-frequency sweep class —
  top solo contributor on the v1 set), or `manual_map` (the gen-8
  closure-birth mechanism: disabling it also kills the born-closure
  `redundant_closure` shaped verdicts — a two-axis downstream dependent).
- No-op: any rule with zero fixture presence on the frozen v1 set (pick from
  the outcome feed's persist/rule table — a rule the composition never
  routes to; disabling it MUST measure Δ=0, else the instrument is broken).

## Tasks

- [x] T1 Component seam audit — where can a single rule be disabled/swapped
  without touching the corpus build? (the proposer dispatch loop).
  **Answer: the `heal_fixpoint_with` iteration loops — see the T1 audit
  section above; dispatch fns are total (`_ => Vec::new()`), the fxhashmap
  skip + `candidates` lane are the shipped precedents, corpus/const arrays
  are read-only inputs.**
- [x] T2 Flip-rate harness behind a dev feature or bench — reuses the
  score-bench runner with per-run component overrides; JSONL out.
  **Landed 2026-09-15: `ComponentOverride` + `heal_fixpoint_cfg` (the seam,
  `src/score_bench/mod.rs` — empty override byte-identical, pinned) +
  `run_bench_overridden` with the cross-arm before-verdict cache +
  `src/score_bench/necessity.rs` (target schedule, co-occurrence donors,
  both arms, two-sided canaries, JSONL to
  `.benchmarks/necessity_history.jsonl`) + the `--necessity-bench
  [--necessity-top N --necessity-random K --necessity-smoke]` CLI.
  Smoke e2e (20 fixtures): canaries green; first real finding —
  `needless_return` solo=1 but Δdisabled=−17 (solo rate massively
  understates composed importance — the exact CVRR phenomenon).**
- [ ] T3 Measurement pass on the current corpus (one score-bench generation
  per arm × top-20 rules by heal rate + 5 random + 1 planted no-op).
- [ ] T4 The table + verdicts in `.benchmarks/`; decorative findings get
  their own follow-up issues (demote/repair decisions stay owner-side).

## Non-goals

No gate wiring from day one (report-first — a ceiling over an unmeasured
population is a pin nobody has watched fail). No changes to rule emission —
this is an instrument, not a healer behavior change.
