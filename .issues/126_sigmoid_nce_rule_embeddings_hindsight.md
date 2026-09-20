# Issue 126 — Sigmoid-NCE rule embeddings from fix-trajectory hindsight + Lemma-4.2 tier guard

**Status:** CLOSED — **Lane (a) LANDED + GATED PASS 2026-09-20** ([Bench 099](.benchmarks/099_rule_embed_sigmoid_nce.md): full-rig top-1 94 → 98, G1 zero regressions, withheld no-harm, clean FP unchanged; `rule_embed_poc` + the BLAKE3-sealed artifact + standing gate `tests/rule_embed_gate.rs`; dequant-convention defect caught by the self-pair probe, recorded). **Lane (b) LANDED + GATED PASS 2026-09-20** (`src/tier_guard.rs` + the store-level arms `tests/tier_guard_store_gates.rs`; mapping + gates recorded below; Issue-033 history read and carried — exclusion filter only, pinned knobs, cold-start bit-identical; drive-by: `RuleIndex.rule_ids`'s poc-only reader got the honest `cfg_attr` dead-code annotation). Training data was the fixtures+self-pairs proxy — NO organic fixseq rings existed at train time (measured). **Post-close same-day addition (the 4090 box): the organic-ring corpus substrate for the reopen-to-promote trigger landed** — `.benchmarks/data/126_t0/` (172 runs / 242 hindsight labels: 218 kept-positives / 24 reverted hard-negatives, span_embed stamped 242/242, BLAKE3-pinned manifest) + the `--dump-fixseq <root>` regeneration flag on `examples/rustc_errors_floor.rs`; the §T0 record below carries the vocabulary constraint (strategy-space, not clippy-rule-space) any retrain-on-rings pass must reconcile.
**Filed:** 2026-09-20 · Research: [riir-ai Research 386](../../riir-ai/.research/386_Contrastive_RL_Goal_Conditioned_Value.md) · arXiv:2206.07568 (CRL) — consumer-surface distillation (fusion priority #2)

## T0 record (2026-09-20, the 4090 box) — the label audit + first corpus

The filing's "the labels already exist" was **stale**: `.heal/fixseq/` rings
existed on NO repo on this box (Issue 107's 2026-09-15 "zero organic rings"
verdict still held; `trajectories.kv` exists in ~10 repos but carries per-rule
counters, not per-span labels). Generated the first corpus instead:

- **`--dump-fixseq <root>`** added to `examples/rustc_errors_floor.rs` (the
  driver section's label-gen posture; early-returns before the G1/G2
  measurement passes — the flag-less path unchanged).
- **Corpus committed** at `.benchmarks/data/126_t0/` (172 run files, 242
  attempt rows, BLAKE3-pinned manifest) — flattened to `rings/*.jsonl`
  because the live-store `.heal/fixseq` nesting collides with the repo-wide
  `.heal/` gitignore rule.
- **Measured inventory** (rustc 1.98.1): **218 positives / 24 hard
  negatives**, span_embed stamped 242/242, 5 error codes, **7 strategies**.
  Hard negatives live ONLY on the two E0502 strategies that compete with
  2 fixed alternatives (deref-removal 12, clone-out 12) — the E0502
  contested stratum is where the discriminative signal concentrates.
- **Vocabulary finding (the reopen trigger's design constraint):** the ring
  labels are `strategy_id`-space (the rustc_errors playbook), NOT clippy/perf/kernel
  rule-space — the fix-compile lane is the only lane that writes per-span
  labels. The shipped lane (a) trained on the fixtures+self-pairs proxy
  (clippy-rule-space) instead, which is why the two vocabularies never had to
  meet; any RETRAIN-ON-RINGS pass for the reopen-to-promote trigger must
  either bridge the vocabularies (strategy→code→rule mapping) or extend the
  ring schema to record the clippy-lane fixer per span — the composite rig
  stays the G3 no-regression control either way.
- **Class balance:** 90/10 pos/neg; smallest strategy n=12, largest n=72.
  At d=64 with 7 classes this is a small-data regime — the trainer should
  regularize (weight decay / early stop on a held-out code) rather than
  expect volume. The 4 codes with zero attempts (E0373, E0382, E0507,
  E0596) are playbook-coverage gaps, not label noise — they contribute
  nothing to this lane.

## Lane (a) — trained span↔rule embeddings, hindsight-labeled

**The reframe:** rule selection IS goal-conditioned value — `φ(span)ᵀψ(rule) ≈ P(rule fixes span)`, the CRL critic with rule-as-goal. The labels already exist and are **machine-checked**:

- **Positives (hindsight):** `(span_embed, rule_that_actually_fixed_it)` from the `.heal/fixseq/` rings (the `span_embed` column stamped per attempt, Issue 107 T2) + `traj_store` outcomes — the paper's "hindsight relabeling is a standalone RL algorithm" applied to repair: every kept fix is a reached goal.
- **Hard negatives:** rules that fired and **reverted** (compile-gate rejects) — stronger than the paper's sampled marginals.

Today `span_embed.rs` is a hashed bag (untrained); `retrieval_eval.rs` Issue 017 scoped out "no embedder re-training" — **this issue supersedes that scope for this lane only** (the eval fixtures + top-1/recall@k axes stay the authority).

- [x] Trainer (in-repo, `src/draft/rule_embed.rs` + `examples/rule_embed_poc.rs --train`; ≤0.5 GPU-h budget was the cap, not a mandate — the 270-pair dataset trains in ~8 s on the M3 CPU, deterministic; running it on the 4090 would add SSH orchestration for zero gain — recorded in Bench 099): project the hashed span bag + per-rule corpus snippets into d=64 (paper Table 2 pins 64); loss = **sigmoid-NCE (NCE-binary)** — `log σ(φᵀψ⁺) + log(1−σ(φᵀψ⁻))`, in-batch negatives. NOT softmax-InfoNCE: house rule (sigmoid never softmax) AND katgpt-rs Bench 048 (sigmoid reaches positive margin at d≈log n; InfoNCE needs Θ(n^⅓)).
- [x] Ship as **frozen i8 tables** — `assets/rule_embed_frozen_v1.bin`, versioned wire format + BLAKE3 over the full payload, `decode_verified` refuses hash/version/dims/truncation/duplicate-id mismatches; `embedded_tables()` include_bytes + verified lazy load; the runtime stays modelless (one hashed bag + matvec, one dot + σ per rule).
- [x] Eval: `run_split_eval` posture in `retrieval_eval.rs` (baseline = today's hashed + `RerankMode::Structural`, composite rig + the rule-level withheld-pair OOD set). **G2 gate: full-rig `hits_at_1` beat the baseline 94 → 98** (+4: 3 outranked misses + 1 not-retrieved rescue); G2-OOD withheld 31 = 31 (no-harm; zero headroom — recorded as the anti-memorization read, not a win denominator). G3: clean-span FP profile unchanged (26 = 26). G1: zero regressions. Standing count-pinned gate: `tests/rule_embed_gate.rs`.
- [x] Prior-art honesty: framed in Bench 099 — ContraCode / DPR / CodeContrast / ContrastRepair publish contrastive code embeddings; this is an APPLICATION on machine-checked repair outcomes (ground truth no external corpus has), not a new class.

## Lane (b) — Lemma-4.2 exclusion filter as the evidence-tier guard

`traj_store`'s Certified/Heuristic arbitration is hand-thresholded today. The paper's Lemma 4.2 prices the alternative: exclude trajectory credit when `|π(τ|s_g)/π(τ|s_j) − 1| > ε` (commanded-goal vs reached-goal likelihood ratio), with closed-form slack `V^{π'} ≥ V^π − 2γε/(1−γ)`.

**LANDED mapping (each quantity a field the store already logs):**
- τ = one fix trajectory (the store row `(rule, shape, fix)` + its keep/revert history).
- π(τ|`s_g`) — commanded goal "this rule fixes this lint on this shape" = [`elo::expected_score`]`(rule_elo, span_elo)` over the two rating fields every `TrajRowDoc` durably carries (Issue 039's batch replay: CROSS-ROW evidence — the rule's strength across all its rows, the span's difficulty across all rules).
- π(τ|`s_j`) — reached outcome = the row's own Laplace posterior mean `(S+1)/(S+F+2)` over its counters INCLUDING the arbitrated event (the event is part of the trajectory the reached dynamics produced, not a judge of its own trial).
- The certify transition (`success && verified` → Certified) IS the hindsight relabel — `tier_guard::certify_allowed` gates it. Counters ALWAYS record; only the promotion is gated. Withdrawn recovery passes the same evidence bar (one uniform FSM). Non-finite stored ratings fail CLOSED at the INPUT (`expected_score(+∞, base) = 1.0` is finite — an output-only check would silently admit; the Batch-54 NaN corollary, applied one step earlier).
- Knobs: `TIER_GUARD_EPSILON = 0.5` (agree within factor 1.5) + `TIER_GUARD_GAMMA = 0.9` (prices the documented slack `2γε/(1−γ) = 9`; report-only, never a behavior input). Pinned constants, not adaptive — the Issue-033 lesson.
- **Issue-033 history read before touching anything (the mandate):** Issue 033's TETHER-adaptive selection blend measured NEGATIVE (Bench 042; `5494dbec`, retired `159a0569`) — adaptive re-blending lost to the pinned `W_EVO`/`W_RATE` constants. Consequences carried: this guard is an EXCLUSION filter on one transition, never a re-weighting; knobs pinned; and the cold start is behavior-identical — a fresh row at base ratings reads ratio exactly 1.0 (at base ratings the guard refuses only rows with `F > 2S + 1`; the entire success-only population certifies exactly as before).
- **The two exclusion directions (both real defect classes):** ratio ≪ 1 = free-rider/wrong-goal credit (a disfavored rule certified by a lucky keep on a span the goal model says it loses); ratio ≫ 1 = overcertification (a mostly-failing row certified on a single keep).

- [x] Add the ratio test as the tier-arbitration guard (probabilities the store already logs; ε and γ become the ONLY knobs, with the bound as the documented cost). — `src/tier_guard.rs` (ungated pure half: `goal_probability`/`outcome_probability`/`lemma_4_2_excludes`/`priced_slack`/`certify_allowed` + the π→logged-fields mapping doc) + the `record_classified` integration (the one gated line) + the module/record doc updates.
- [x] G1 adversarial arm: a poisoned trajectory (wrong-goal credit) MUST be excluded; a clean one MUST pass. G3: selection monotone on replay. — `tests/tier_guard_store_gates.rs` (`[[test]]`, `required-features = ["latent_retrieval"]`, whole-file `#![cfg]` protecting the count / the row protecting the reader): `poisoned_wrong_goal_credit_is_excluded` (fixture premise measured: a rule rated ≈ 0.074 vs a span the Elo replay rates 1438 from 30+30 cross-row events — the keep's ratio ≈ 0.11, far outside ε → stays Heuristic, counters record), `clean_consistent_goal_credit_certifies` (p_goal 0.586 vs p_outcome 8/9 → ratio 0.66 → certifies), `guard_refusal_never_demotes_certified` (adverse-turning goal model excludes a later keep on an already-Certified row — tier holds, G3 monotone), `replay_is_deterministic` (identical event sequences → bit-identical rows incl. refusals), `fresh_verified_success_still_certifies` (cold start). Pure-half 9 tests at default features. Validation: default lib 2034/0; latent lib 2195/0/1i; clippy `--all-targets -D warnings` clean at default AND `latent_retrieval`; `--features rule_embed_poc` still compiles. Rating magnitudes in the fixtures were MEASURED via a scratch store + `recompute_elo`, not hand-modeled — the first 12/6-event guess landed p_goal 0.18 (replay damping: each match moves both sides by K·(S−E), E converging) and was scaled to 30/15 before landing.

## Order

Lane (a) first (cheapest, most instrumented); lane (b) is a small follow-on touching `self_evolve`/`traj_store` — read the Le-Critique/Issue-033 coefficient-shape history before touching the blend constants there.
