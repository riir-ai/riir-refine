# Issue 62: TPR × healer corpus fusion — structured retrieval axis + corpus-generalization bench (Research 527 F5)

**Status:** T4 **DECIDED 2026-09-03** — route (b) against `clippy_lints`
only, `rustc_errors` excluded, gated on the preregistered T4a withheld-pair
test (§ route (b) below). Original status follows.

**Status:** Phase 1 COMPLETE (T1+T2+T2a landed, Bench 062). Phase 2: **T3 + T5 LANDED
(Bench 063 + its coalescing addendum, 2026-09-02) and their verdict RE-SCOPES
T4** — the role scheme is
decided (role = shape-class, filler = rule identity), and the 8-D consumer
state is **INADMISSIBLE at every `d`**: `K = m·d ≥ D` fires at `d = 1`, so a
TPR fit in the reranker's scoring space reproduces its own training states.
T4 as written ("score against the TPR projection instead of the raw 8-D
embedding") is **not runnable**; the corpus IS structured (measured in a wider
control space: BoW ratio 1.1773, cross-state ratio 1.0812, two agreeing
probes), so the blocker is state width, not TPR. T4 is re-scoped to the bridge
route (§Phase 2 T4 below). **T5 is LANDED** with a per-domain verdict that
refutes Bench 062 §10's rustc prior as a category error, and adds T5b
(kernel_opt population, corpus plumbing). T6 unchanged and still actionable. The upstream
`tpr` promotion still waits on a landed consumer, i.e. still on T4.

**Research:** [katgpt-rs/.research/527_TPR_Emergent_Symbolic_Structure_Binding_Algebra.md](../katgpt-rs/.research/527_TPR_Emergent_Symbolic_Structure_Binding_Algebra.md) (arXiv:2608.29530, McCoy/Soulos/Linzen/Smolensky 2026)
**Consumer rationale:** the healer corpus is the workspace's richest (role, filler) dataset — every entry IS a binding: role = rule/lint identity, filler = code shape. The retrieval stack's own history is the paper's thesis in miniature: the Issue 017/T3.5 misses were spans sharing the filler (generic fn shape) while differing in the ROLE (operator/punctuation pattern) — the RustOpTokenizer fix injected role-ish signal into tokens; TPR makes the role axis structural. And the one axis nothing measures is generalization: **every retrieval floor (14/14 gate) and every fixture set is in-distribution by construction.** The paper predicts atomic memorization is the default failure mode on unseen (rule, shape) combinations; this issue converts that prediction into OUR measurement before building anything.

## Phase 1 — the unmeasured axis (no dependency, no GPU)

- [x] **T1** **Withheld-pair OOD generalization bench** for the healer corpus — LANDED, preregistered before running, results in [.benchmarks/062_withheld_pair_ood.md](../.benchmarks/062_withheld_pair_ood.md). Harness: `src/draft/withheld_pair_ood.rs` + `tests/t062_withheld_pair_ood.rs`. Population 113 distinct-text bindings (45 corpus + 48 fixtures + 20 oracle-labeled real-repo spans from `.benchmarks/data/049_t01/`), 45 roles x 8 context-derived shape-classes, 62 cells, **10 eligible**, n=25. **Measured: pure-retrieval ID 52.0% -> OOD 4.0% top-1 (paired Δ +48.0pp, 22/25 rows worse), against a 2.2% chance floor** — OOD is at chance. Shipped `RerankMode::Structural` arm ID 92.0% -> OOD 52.0% (Δ +40.0pp). Atomic null 0.0% on withheld pairs (asserted, by construction); NOROLE canary 0.0% on every arm. Bit-stable over 12 processes on the retrieval arm.
- [x] **T2** **rustc_errors extension** — LANDED, same protocol, fully deterministic (no cargo/oracle). Population 212 (200 generated + 12 anchors), role = error code, filler = borrow-carrier shape-class, 8 roles x 10 fillers, 14 cells, **4 eligible**, n=43. **Measured: ID 97.7% -> OOD 48.8% top-1 (paired Δ +48.8pp) vs a 12.5% chance floor; atomic null 79.1% ID / 0.0% OOD** — the informative null of the two tasks. Mechanism localized: OOD is a **step function in the role's filler count** — E0597 (6 fillers) generalizes at 100.0%, E0596 (2 fillers) collapses to 0.0% while keeping the true code at rank 2-7 (present-but-demoted = a re-ranking shape, not a retrieval shape).
- [x] **T2a (follow-up) — FIXED UPSTREAM, root cause found.** Substrate nondeterminism found by the 12-process determinism check: the shipped Structural OOD top-1 flipped 52.0% (9/12) / 56.0% (3/12) on exactly ONE row — `needless_return` sometimes absent from the `RERANK_POOL = 12` candidate pool. `corpus_index::rank_into` already applies a canonical tie-break, so the flip was upstream, in what `NeuronDbRetriever::retrieve` returns. Localized to **`riir-neuron-db` `Bm25Index::search`**, not riir-rag: the top-K comparator compared score only, `entries` was materialized from `scores.into_iter()` on a `HashMap<&[u8], f32>` whose `RandomState` is seeded once per PROCESS, and the `limit` cap is an unstable `select_nth_unstable_by` — so a tie cluster straddling `limit` was truncated in hasher-seed order and the returned **SET** varied per process (a membership flip, not an ordering flip; riir-rag could not recover a candidate BM25 never returned). Fixed by a total order (score desc, then key asc) applied BEFORE the truncation — riir-neuron-db `aeb24e0`, matching that repo's own Issue 591 T9 idiom. Pinned by riir-ai `crates/riir-rag/tests/retrieve_determinism.rs` (`5a08f5783`), which re-executes its own binary in 12 distinct processes because an in-process repeat loop is one `RandomState` sample: candidate COUNT varied 15/16/17/18 across 12 processes before, one digest across 144 processes after, revert-probe confirmed the pin fires. Full write-up: riir-ai `.issues/849`.

## Phase 1 verdict (Bench 062, 2026-09-02)

**Phase 2's TPR axis is JUSTIFIED by our own measurement, not by the paper's
claim** — T1 Δ +48.0pp and T2 Δ +48.8pp on the pure-retrieval arm, both above
the preregistered 20.0pp bar; the demote branch (Δ ≤ 5pp, "OOD at ceiling")
did NOT fire. Three caveats that bound what Phase 2 may claim:

1. **T1's atomic null is VACUOUS** (0.0% on the ID arm too — no two healer-corpus
   bindings share a normalized token structure, so the memorizer cannot fit its
   own training set). T1's OOD failure is therefore *filler-dominated
   similarity*, not conjunction memorization. T2 is where the memorization
   reading is direct (79.1% ID / 0.0% OOD).
2. **The corpus is nearly role/filler-COLLINEAR**: 32/45 clippy roles and 6/8
   error codes occur under exactly ONE shape-class, so no unseen combination of
   them can exist. More fillers per role is a CORPUS task; the 20 real-repo
   spans supplied all of T1's shape diversity.
3. **Part of Δ is structurally unclosable.** OOD tracks the role's remaining
   filler count as a step function; a role with one remaining filler has
   nothing to compose from. T4's claim must be scoped to the
   retrieved-but-demoted rows (all 22 E0596 rows; the `match_bool` /
   `map_unwrap_or` T1 cells), and any gain on single-filler roles read as
   leakage.

## Phase 2 — structured retrieval axis (katgpt-core `tpr` LANDED, katgpt-rs `1f7a96d4`)

> **UNBLOCKED 2026-09-02** — katgpt-rs `1f7a96d4` landed `katgpt_core::tpr`
> (Issue 707 T1–T7 + GOAT G1/G2/G3/G4/G8 ALL PASS,
> katgpt-rs `.benchmarks/698_tpr_binding_algebra_goat.md`). The rows below are
> now `[ ]` actionable. What to consume, so nobody re-derives it:
>
> | need | API |
> |---|---|
> | T3 fit | `tpr::als_fit(AlsInput { dim, n_fillers, states, bindings }, &AlsConfig::new(d, scheme))` → frozen `TprArtifact` + `AlsReport` |
> | T4 denoise | `tpr::project_into(&art, state, &mut TprScratch::new(&art), &mut out)` → residual; `unbind_state_into` for role-conditioned match |
> | T4 OOD eval | `tpr::withheld_pair_top1(...)` vs `tpr::AtomicNull` (the null this issue's Phase 1 already measured) |
> | T5 router | `tpr::bow_router(input, &cfg, eps)` → `r_bow / r_full`, plus `tpr::shuffled_role_control` as the dual |
> | T6 surgery | `tpr::surgery_delta_into` / `tpr::surgery_move_into` (exactly additive, one GEMV) |
>
> Four things measured upstream that change how these rows should be run:
>
> 1. **Feature is `tpr` on katgpt-core, opt-in, and stays opt-in** — its
>    T-Promote is blocked by the no-default-consumer rule, i.e. by THIS issue's
>    T4. Landing T4 is what promotes it, not another upstream gate.
> 2. **`n_slots ≤ arity` (the square unbind case).** A role-per-rule scheme with
>    426 kernel_opt rules cannot be a role-vector scheme at `m = 426`; either use
>    `TprScheme::Orthogonal` with a small block count and put rule identity in the
>    filler, or fold rules to categories. The fit REFUSES the over-complete case
>    rather than silently mis-unbinding — decide the scheme before wiring T3.
> 3. **The atomic-null discipline this issue already learned holds upstream too**
>    — G8 checks the null's ID coverage FIRST, and a healer-corpus null is
>    vacuous (0.0% ID). Phase 1's finding is now enforced in the shared harness.
> 4. **`project_into` needs the artifact's cached inverse** (built when
>    `K = m·d ≤ 1024`). A large filler dimension silently costs the denoise
>    readout — size `d` with that in mind, not by embedding convenience.
>
> Phase 1 already discharged the half that needed nothing (T1/T2/T2a), and its
> measurement is what justifies Phase 2 at all.

- [x] **T3** **ALS fit of the healer corpus** into role-filler form — LANDED,
  [.benchmarks/063_tpr_structure_admissibility.md](../.benchmarks/063_tpr_structure_admissibility.md),
  harness `src/draft/tpr_fit.rs` + `tests/t063_tpr_structure.rs`, feature
  `tpr_structure`, preregistered at `c6d0452` before the harness existed.
  Four things it settled, two of which change the plan:
  1. **Scheme decided (the question the Phase 2 preamble left open):** role =
     **shape-class** (`Orthogonal { arity: m = 8 }`), filler = **rule
     identity**. Inverted from Phase 1's naming and forced to be:
     `n_bind_slots == arity` means role = rule identity would need `m = 45`
     (`K ≥ 45` against `D = 8`), and `kernel_opt`'s 426 rules are only
     expressible on the *filler* axis, whose count is unconstrained. This is
     the upstream note's recommendation, now priced.
  2. **The 8-D consumer state is INADMISSIBLE at every `d ∈ {1,2,4,8}`** on
     both domains — `K = m·d ≥ D = 8` at `d = 1` already. A fit there
     reproduces every training state, so any OOD number read off it is
     content-free. This is what re-scopes T4.
  3. **The corpus IS structured** — measured in the 256-D token-bag control at
     the only admissible geometry (`d = 1`): `bow_router` ratio 1.1773
     (structured) AND `cross_state_role_control` ratio 1.0812 (degraded). Two
     probes that can disagree, agreeing.
  4. **`rustc_errors` is unmeasurable, not unstructured** — inadmissible at
     every `d` because `plain-binding` has exactly 1 state. Bench 062 §10's
     rustc BoW prior is still a prior; T5 must not read this as its answer.
  Two of the run's own predictions were wrong and are recorded as stated (P2
  refuted by the per-role bound it omitted; P1 confirmed by a different bound
  than it argued) — Bench 063 §7.3.
  **Substrate tie deliberately NOT taken yet:** `self_evolve`'s EMA domain
  direction vectors are rank-1 role embeddings on the *domain* axis, and the
  fitted role axis here is the shape-class. Wiring the freeze/thaw cycle to a
  role table is T4 work and pointless before T4's space is settled.
  **Found upstream, filed, and RESOLVED:** `shuffled_role_control` could not
  fail on a single-binding corpus (`for i in (1..1).rev()` is an empty loop) —
  it contradicted `bow_router` on the same artifact. katgpt-rs `.issues/710`,
  fixed + closed + removed in katgpt-rs `af3bca25`: vacuity is now a reported
  quantity on all four validation instruments and the cross-state arm resolves
  automatically. The consumer-local workaround is deleted; `tpr_fit.rs` calls
  upstream on both arms and reproduces every Bench 063 number bit-identically
  (Bench 063 §7.4a).
- [ ] **T4 (RE-SCOPED by T3 — read Bench 063 §8 before starting)** **TPR
  structural evidence channel.** The original "score against the TPR
  projection, not the raw 8-D embedding" is **not runnable**: the projection
  lives in the state space, and the state space is 8-D, where no admissible
  fit exists (T3 finding 2). Two routes, and the second is recommended:
  - **(a) widen the consumer state.** `riir_rag::query::LATENT_DIM = 8` is a
    cross-repo contract (riir-rag + riir-neuron-db indexes + every committed
    floor). File in riir-ai; do NOT work around it here.
  **AMENDED by Bench 063 §11.6 — the justification got stronger, the
  recommendation did not change.** §8 *inferred* that the 8-D state cannot host
  the structure, from that space being unmeasurable. §11 **measures** it there
  (clippy, coalesced to m'=6, `d = 1`, admissible) and finds it **flat**: both
  probes below 1.0, `r_bow` beating `r_full`, and permuting roles across states
  *improving* the fit. So route (a)'s premise is now measured rather than
  argued — the structure exists in the code text (control ratios 1.14 clippy /
  2.21 rustc) and does not survive projection to 8 dimensions.
  **QUALIFIED 2026-09-02 by the composition MEAN (`f4f5df3`, Bench 063 §12.6) —
  read before starting either route.** The two control ratios this paragraph
  leans on come from arms that are **near-degenerate**: `clippy_lints` max 4 but
  **mean 1.378** over 45 fillers, `rustc_errors` max 4 **mean 1.500** over 8.
  The typical filler contributes no `(role, filler)` variation at all and the
  max is carried by a handful of them.

  *(Arm-matched 2026-09-02. The `rustc_errors` covariate was first written here
  as max 6 / mean 1.750 — that is the **uncoalesced** arm, whose probes were
  `SkippedInadmissible` and which therefore produced no ratio at all. 2.2096
  comes from `rustc_errors / tokenbag256 [coalesced T=8]`, whose covariate is
  max 4 / mean 1.500; coalescing merges roles and can only lower it. Same for
  1.1417, which is `clippy_lints / tokenbag256 [coalesced T=8]` — that one was
  already right, because coalescing leaves clippy's covariate unchanged. The
  conclusion is unaffected: both are near-degenerate either way. Recorded
  because quoting a covariate from a different arm than the ratio it qualifies
  is the same one-step-wider-than-measured shape this whole thread is about.)*

  **The two arms are thin in DIFFERENT ways, and the remedies differ.** From the
  arm-matched numbers, distinct `(role, filler)` pairs = `fillers x mean`:

  | arm (coalesced, 256-D) | fillers | m' | distinct pairs | pair space | covered | states/pair |
  |---|---|---|---|---|---|---|
  | `clippy_lints` | 45 | 6 | 62 | 270 | 23.0% | **1.8** |
  | `rustc_errors` | 8 | 8 | 12 | 64 | 18.8% | **17.7** |

  `clippy_lints` is thinly **observed** — 1.8 states per distinct pair, so each
  pair's contribution is essentially a single sample and more corpus would
  sharpen it. `rustc_errors` is thinly **covered** but well observed — 17.7
  states per pair, so its 18.8% is not an undersampling artifact and more data
  cannot fix it; only pairs that do not occur occurring would. Route (b)'s
  precondition below therefore has two different failure modes to test for, not
  one.

  **Is rustc's coverage a structural bound on route (b)? No — an EVIDENCE bound,
  and that distinction is what makes it actionable.** A scalar channel fitted on
  12 of 64 possible pairs does have to act on pairs it has never seen, and it is
  tempting to call that circular. It is not: acting on unseen `(role, filler)`
  pairs is what TPR is FOR, and a mechanism being used as intended is not a
  defect. What IS true is narrower and worse for the current record — the 2.2096
  ratio was measured on the 12 pairs that occur, so it certifies nothing about
  the 52 that do not, and route (b) would be fused on the strength of a number
  that never looked at the regime it will run in.

  **MEASURED 2026-09-02 (Bench 063 §12.7): `multi_role_fillers` on the rustc arm
  is 2 of 8.** The consumer did not surface the field; it does now, alongside
  the pair-coverage arithmetic, printed per arm. The answer is the bottom of the
  2-3 range this section anticipated, so route (b)'s generalization question
  **cannot be answered on this corpus** — the OOD test that would validate the
  2.2096 signal can draw on two fillers. The uncoalesced arm is no better (max 6
  / mean 1.750 reads healthier on the threshold, still 2 of 8 testable), which
  is exactly the gap `multi_role_fillers` was added to expose. `clippy_lints` is
  the tractable arm instead: 13 of 45 testable, but 1.8 states per distinct pair,
  so it is thinly observed rather than thinly covered and more corpus fixes it.
  **OWNER CALL MADE 2026-09-03 — route (b) proceeds against `clippy_lints`
  ONLY, gated on a withheld-pair pre-test. The `rustc_errors` arm is excluded
  from the fuse. Option (c), fusing on unvalidated generalization, is
  rejected.** Rationale below; T4a is the cheap gate that can still kill it.

  *Why not widen rustc (option a).* Its 18.8% coverage is **structural, not a
  sampling artifact** — 17.7 states per distinct pair means the pairs that
  occur are saturated, so more corpus buys more observations of the same 12
  pairs. New pairs would have to come from an error code appearing in a shape
  class it never appears in, and the vocabulary is 8 x 8 and fixed. High cost,
  low expected yield, and the diagnostic that would tell us it worked
  (`multi_role_fillers`) is the one already reading 2.

  *Why clippy (option b).* 13 of 45 testable fillers is a real population, and
  at **1.8 states per pair it is thinly OBSERVED rather than thinly covered** —
  the opposite failure, and the one more data actually fixes. 62 of 270
  possible pairs seen means the corpus is nowhere near saturated, so widening
  clippy adds pairs *and* observations. It is also the cheaper corpus to widen:
  lints are abundant, `rustc` error codes are not.

  *Why not (c).* Choosing the unmeasured option while a tractable measurement
  sits on the other arm is the exact move this issue's whole §12 thread exists
  to correct. If the fuse is worth doing it is worth gating, and the gate is
  affordable.

- [ ] **T4a (gates T4, cheap, kill-switch)** Run
  `withheld_pair_top1_report` on the `clippy_lints / tokenbag256 [coalesced
  T=8]` arm over its 13 multi-role fillers: withhold one `(role, filler)` pair
  at a time, score against the shared pool, read `verdict()` and `coverage` —
  never the bare `top1`. Compare against the `AtomicNull` on the same withheld
  pairs, which must fail OOD by construction.

  **Preregistered, before the harness exists**, because a threshold chosen
  after seeing the number is not a threshold:

  - **PASS** — `verdict()` is `Some` on every arm that runs (no withheld pair
    may make the corpus degenerate, which `ObservedPairs` now makes checkable
    up front), `coverage == 1.0`, and TPR beats the null by **≥ 20 pp**, the
    same bar Issue 707 G8 registered. Route (b) proceeds.
  - **FAIL** — TPR does not clear the bar. Route (b) is dead on this corpus and
    T4 closes REFUTED; do not widen and retry, that is the fishing expedition
    this bar exists to prevent.
  - **INCONCLUSIVE** — clears the bar but on so few withheld pairs that a
    single flip would cross it. That is the 1.8-states-per-pair risk
    materialising, and it is the ONE branch where widening the clippy corpus
    is the right response rather than a retry.

  Report the three populations apart — validated, unvalidated-but-fitted, and
  outside-the-fit — rather than pooling them into one accuracy. Pooled, a
  prediction reads as a measurement.

  Unlike a structural bound, an evidence gap is measurable, and the instrument
  already ships: `katgpt_core::tpr::withheld_pair_top1_report` (katgpt-rs
  `8c7ca74b`) — withhold a pair, score against the shared pool, and read
  `verdict()` / `coverage` rather than the bare `top1`. It is the same test G8
  runs on a planted corpus, pointed at this one.

  **The one thing that was missing is the population**, now shipped as
  `FillerRoleSpread::multi_role_fillers` (katgpt-rs `149a0c02`): withholding a
  pair from a single-role filler withholds the whole FILLER, so only fillers
  seen with 2+ roles can host the test at all. `max` cannot say how many those
  are — measured upstream, a corpus at `max 2 / mean 1.1250` had **1 of 8**
  testable, while G8's planted corpus has 7 of 8. Read `multi_role_fillers` on
  the rustc arm FIRST: with 8 fillers, `mean 1.500` and `max 4`, the testable
  population is small by arithmetic and may be 2-3 fillers. If it is, route
  (b)'s generalization question cannot be answered on this corpus and the honest
  options are a wider corpus or an explicit decision to fuse on unvalidated
  generalization — an owner call, made before the fuse and not after. `structured()` is still `Some(true)` and
  the guard is still two-sided, so nothing here is overturned — but "the
  structure exists in the code text" is a thinner claim than it read as when
  only `max` was reported, and it is the claim BOTH routes are built on.
  **Precondition on (b), not a blocker:** before building the scalar channel,
  check that the role-conditioned match carries signal on the fillers that
  actually vary rather than on the handful holding the max up. A scalar fused
  into the rank from a near-degenerate role axis is a channel that fires on a
  few rules and is noise on the rest — which is testable up front and much
  cheaper than discovering it after the fuse. This is the same lesson as
  katgpt-rs Issue 711 one level out: the covariate an interpretation depends on
  has to be checked before the interpretation is spent.
  - **(b) bridge instead of widen — RECOMMENDED.** Keep the 8-D scoring
    state. Fit TPR in the wider token-bag space, emit a **scalar**
    role-conditioned match (`unbind_state_into` against the candidate rule's
    filler row), and fuse that scalar into the existing rank. Latent
    structural evidence → one scalar → the raw score: the documented
    latent→raw bridge, no `LATENT_DIM` change, no index rebuild. It also
    matches where Bench 062 §10 said the headroom is — *retrieved-but-demoted*
    rows are a re-ranking shape, not a retrieval shape.
  The claim discipline below is unchanged and still binding. Original text
  retained for the record:
  **TPR-structured retrieval axis** *(OOD baseline to beat, from Bench 062: T1 retrieval 4.0% / Structural 52.0% top-1; T2 retrieval 48.8% top-1, 51.2% top-3)* behind a `tpr_retrieval` feature (off by default): query-side structural denoise (score against the TPR projection, not the raw 8-D embedding — paper finding 7: decoders read better from the idealization) + role-conditioned match in the reranker. **Honest claim discipline: in-distribution gains are NOT the claim** (top-1 already 89.6–100 on the floors); the claim axes are (a) the OOD delta over T1's baseline by a preregistered margin, and (b) kernel_opt long-tail crowding (426 rules, the biggest corpus). Floors MUST hold (the 14/14 retrieval gate + score-bench flat unless OOD wins land).
- [x] **T5** **BoW-null router per domain — LANDED** (Bench 063 §10-11, the
  coalescing addendum; preregistered at `d64523a` before running). Needed one
  thing first: `rustc_errors` was unmeasurable because `plain-binding` — the
  classifier's final `else` branch — holds 1 of 212 spans, so the per-role
  bound killed every `d`. Fixed by coalescing roles with `≤ T` states into one
  `other` block at a **fixed** `T = 8` (per-`d` coalescing changes `n_states`
  and makes BIC scores incomparable — §10.2a). Zero states dropped on either
  domain. **Verdict, over the coalesced vocabulary, at BIC's `d = 1`:**

  | domain | space | m' | verdict |
  |---|---|---|---|
  | clippy_lints | consumer 8-D | 6 | **NOT structured** (bow 0.9961 / cross-state 0.9897) |
  | clippy_lints | control 256-D | 6 | **structured** (1.1417 / 1.0693) |
  | rustc_errors | consumer 8-D | 8 | unmeasurable (`K == D` at `d = 1`) |
  | rustc_errors | control 256-D | 8 | **structured** (2.2096 / 2.6661) |

  Three results that change what the rest of Phase 2 may assume:
  1. **Bench 062 §10's rustc prior is REFUTED, and was a category error.** It
     predicted little role gain from "6/8 error codes are single-filler";
     rustc is the **most** structured arm measured. The prior was about the
     *error-code* axis, which this scheme puts on the **filler** side — a
     prior about one axis applied to the other. Do not re-import it.
  2. **BIC prefers `d = 1` even with all four `d` admissible**, scores rising
     monotonically in `d` on both control arms. Wide filler rows never pay for
     themselves on this corpus, which retires the "K too shallow" class of
     hypothesis for this fit.
  3. **The 8-D consumer space is measurably FLAT** — both probes below 1.0,
     `r_bow` beating `r_full`, and a cross-state role permutation *improving*
     the fit. Which is what amends T4 (below).

  Preregistered guard §10.4 was itself refuted (it claimed coalescing could not
  make the consumer space admissible "because it only shrinks `m`" — shrinking
  `m` is exactly what takes `K = m'·d` below `D`); replaced by the
  bound-vs-verdict invariant, asserted on every arm. §11.5.
  **Confound stated, not fixed** (§11.7): the shape-class role is computed from
  the same span text the state is hashed from, so a role is partly readable off
  the state. Unavoidable under the modelless mandate — any modelless role is a
  function of the input — so it bounds what "structured" claims, and it
  explains rustc's high ratios (8 fillers vs 8 role blocks) without appeal to
  deeper compositionality.
  Original text retained:
  **BoW-null router per domain:** *(prior measured by Bench 062: `rustc_errors` roles are 6/8 single-filler, so m=1 shared-role ALS should show little residual gain there; `clippy_lints` has 13 multi-filler roles)* fit the m=1 shared-role ALS per corpus; residual ratio `r_bow/r_full < 1+ε` ⇒ that domain carries no binding structure ⇒ structured machinery skipped there. Records a per-domain structure verdict (prediction to test: clippy_lints strongly structured; kernel_opt categories possibly weaker than rule-level).
- [x] **T5b** **`kernel_opt` population — LANDED** (Bench 063 §12, preregistered
  at `a6bea77` before any output was read). `kernel_opt_population()` ships
  beside the other two builders; `tpr_structure` now pulls `kernel_heal`
  (which gates `domains::kernel_opt`) rather than cfg'ing the arm away, because
  an arm that silently disappears on a feature is the vacuous-instrument
  failure this bench exists to avoid.
  **The row exists and T5's original prediction is still untested — for a
  reason that is itself the finding.** `kernel_opt` binds 445 rule ids over 445
  states, each rule in exactly one declared `KernelCategory`, so **the role is
  a deterministic function of the filler**: there is no unseen `(role, filler)`
  pair to generalize to, and systematicity — TPR's whole claim — is not posed,
  let alone answered. P15 was refuted (0.9307 / 1.0147, both below the bar) but
  §12.3's planned confound-discount is **retracted rather than triggered**: the
  inference needed this arm to be a valid test of role structure and it is not.
  Testing T5's prediction needs a corpus where one rule id appears under more
  than one category, or a *derived* shape class over `ref_shape` — which would
  re-import §11.7's confound the declared category was chosen to escape.
  **New instrument, shipped:** `FitCorpus::role_is_a_function_of_filler()` +
  the `filler_roles` covariate; `SpaceReport::structured()` returns `None` on
  it, joining the two Issue 710 vacuity flags, and the printed table carries
  `max roles/filler` on every arm so a *near*-degenerate corpus is visible too.
  The test pins the guard **two-sided** — it must fire on some arm and not on
  all, so a dead guard and an always-on guard both red the suite.
  Filed upstream as katgpt-rs `.issues/711` (`08310b4c`).
  **New caution on §11.3, recorded not resolved:** both probes are monotone in
  states-per-filler across all three arms (1.00 → 0.93, 2.51 → 1.14, 26.50 →
  2.21), which is the shape of a capacity effect. It does not overturn §11.3
  (the cross-state control is params-neutral and rustc degrades hard at 2.6661)
  but "most structured arm" and "arm with the most filler reuse" are not
  distinguished by anything measured here.

- [ ] **T6** **Counterfactual surgery on LatentFixMemory trajectories:** for score-bench persist-miss spans, surgery-swap the rule/domain filler and ask closed-form "would domain X / rule Y have healed this span?" — complements Bench 032's re-run-based switch-cost measurement with a zero-recompute counterfactual; feeds the post-mining intake (`.distill/001_mining_queue_snapshot.md`) with attributed misses.
  **T6 INHERITS T4's evidence bound, and the mechanism does NOT warn you.**
  Noted 2026-09-02 so it is not discovered after the build. `surgery_delta_into`
  is exactly additive (Issue 707 G1, max |Δ| 9.5e-7) and stays exact on any
  corpus — including one where the structure verdict is unreadable. So T6 will
  produce clean, confident, zero-recompute numbers whether or not the artifact
  has ever seen the counterfactual it is being asked about. That is the same
  shape as Issues 710/711 a third time: the operation can succeed perfectly and
  still answer a question the corpus cannot support.
  Concretely: a counterfactual swaps the span's filler to rule `Y`, producing
  the pair `(shape_of_span, Y)`. That pair is **observed** iff some other span
  of the same shape class carried `Y`. Coverage is 23.0% on clippy and 18.8% on
  rustc (§12.7), so a substantial fraction of counterfactuals will land on pairs
  the fit never saw — an extrapolation, not a readout. **NOT a blocker and NOT
  a claim that most counterfactuals are unobserved:** the counterfactual
  distribution is over rules that plausibly compete for a span, not uniform over
  the pair space, so the real fraction is unknown and is worth measuring rather
  than assuming.
  **First task, cheap, before any attribution is published:** tag every emitted
  counterfactual with whether `(role, filler_new)` occurs in the fit corpus, and
  report the two populations separately. `filler_role_spread` already carries
  the machinery; the per-pair lookup is one sorted-set membership test. An
  attributed miss on an observed pair is a readout; on an unobserved pair it is
  a prediction, and the intake at `.distill/001_mining_queue_snapshot.md` should
  not receive them mixed.

## GOAT gate

> Also all `[-]`: these gates measure the Phase 2 machinery, so they inherit its
> unblock (707 T1–T5). G8's OOD baseline is the ONLY part already in hand — Phase 1
> produced it (Bench 062), and the caveats recorded above bound what G8 may claim.

> **G1 is now PASSED by T3** (Bench 063 §7.5) — kept in the list because the
> remaining gates still inherit T4's unblock.

- [x] **G1** ALS determinism — PASS. Double fit bit-identical on all 16 grid
  points, and the whole report bit-identical over **5 processes** (not an
  in-process repeat loop — the T2a lesson). The monotone assertion is the
  **rollback certificate** (`final_ssr ≤ min(ssr_per_sweep)`), not
  `monotone_violations == 0`: a violation is the descent guard *working*, and
  one does occur (`clippy_lints/latent8 d=1`, where `K == D` makes the fit
  exactly determined and f32 noise exceeds the `1e-9·ssr_0` band). Asserting
  the counter was a false red on a correct fit — Bench 063 §7.5.
- [-] **G3** Floors hold with the feature off AND on (14/14 retrieval gate; score-bench run recorded before/after).
      **Selector resolved by measurement 2026-09-02 - do not re-derive:** the "14/14
      retrieval gate" is the LIB test module `src/draft/retrieval_eval.rs` (floors in its
      private `mod floors`, assertion `retrieval_precision_regression_gate`), NOT a
      `tests/*.rs` file. `--features latent_retrieval` alone selects **12** of the 14 -
      the other two need `sec` + `kernel_heal`. Both states measured GREEN:
      `latent_retrieval` = 12/0 (1103 filtered), `latent_retrieval,sec,kernel_heal` =
      14/0 (1409 filtered, ~343 s). A "12, not 14" reading is the wrong selector, not a
      regression. **Feature-off state measured HELD** by the T2a consumer A/B: 3
      processes per arm, bit-identical (clippy 89.6/93.8/95.8, perf 96.2/98.1/100.0,
      blended 93.1/96.0/98.0, router 74/101). Two standing fragilities recorded there,
      neither caused by this work: `CLIPPY_TOP3` sits at **exactly zero margin**
      (45/48 measured vs 45/48 asserted - one casualty reds the gate), and the `mod
      floors` P7 re-pin comment understates measured blended top-3/top-5 by 1-2 spans
      (floors were not weakened; the corpus moved).
- [-] **G4** Zero-alloc hot path for the projection/unbind inside retrieval (tracking allocator, exact counts).
- [-] **G8** OOD delta vs the T1 baseline at the preregistered margin, OR an honest no-gain record + demote-loser (flat cosine + Structural rerank keeps the slot at OOD parity). Per-domain BoW verdict recorded.
- [-] **Promote** (katgpt-core `tpr` → default-on) Only after gates pass AND T1 shows the OOD gap the axis claims to close; opt-in until then.

## Cross-refs

- katgpt-rs: R527 (§3 Fusion F5 = this issue), `.issues/707` (the `tpr` primitive; its G8 "real downstream consumer" battery is satisfied by T1+T4 here — the healer corpus is a better validation dataset than a synthetic micro-model).
- Existing substrate consumed, not duplicated: riir-rag `RuleIndex` (already consumes the R299 Clifford wedge KNN — the wedge is pairwise interaction; TPR adds role composition), `RerankMode::Structural`, `clippy_oracle.rs` ground truth.

**Consumer note for katgpt-rs 707:** the primitive's first real-world validation consumer is THIS issue's T1/T4 — the healer corpus (580+ labeled (rule, shape) bindings, oracle ground truth, measurable downstream accuracy) replaces a synthetic micro-model for the G8 intervention battery. T1 runs without the primitive and produces the OOD baseline that gates Phase 2.
