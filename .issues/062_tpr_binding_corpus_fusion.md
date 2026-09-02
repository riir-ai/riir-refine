# Issue 62: TPR × healer corpus fusion — structured retrieval axis + corpus-generalization bench (Research 527 F5)

**Status:** Phase 1 COMPLETE (T1+T2+T2a landed, Bench 062; T2a's substrate nondeterminism fixed upstream — riir-neuron-db `aeb24e0` + riir-ai `.issues/849`) — Phase 2 (T3-T6) JUSTIFIED by our own measurement and still gated on katgpt-core `tpr` primitives (katgpt-rs `.issues/707`).

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

## Phase 2 — structured retrieval axis (gated on katgpt-core `tpr`, issue 707 T1–T5)

> **All rows below are `[-]` (deferred), not `[ ]` (actionable): every one needs the
> `tpr` primitive that does not exist yet. **Unblock: katgpt-rs `.issues/707` T1–T5
> ship the algebra** (bind/unbind/surgery/project + the ridge-ALS fit). As of
> 2026-09-02 that work is in flight in a sibling session — `katgpt-core/src/tpr/`
> holds an UNTRACKED `types.rs` — so treat 707 as unlanded until it is committed.
> Phase 1 already discharged the half that needed nothing (T1/T2/T2a), and its
> measurement is what justifies Phase 2 at all.

- [-] **T3** **ALS fit of the healer corpus** into role-filler form: filler = code-shape embedding (existing AstChunker/ModellessEmbedder path), role = rule identity (and separately domain); ridge-ALS closed form, monotone certificate, frozen BLAKE3 artifact. Substrate tie: `self_evolve`'s EMA domain direction vectors are already runtime-updated rank-1 role embeddings — the TPR role table should thaw from the same freeze/thaw cycle, not fork it.
- [-] **T4** **TPR-structured retrieval axis** *(OOD baseline to beat, from Bench 062: T1 retrieval 4.0% / Structural 52.0% top-1; T2 retrieval 48.8% top-1, 51.2% top-3)* behind a `tpr_retrieval` feature (off by default): query-side structural denoise (score against the TPR projection, not the raw 8-D embedding — paper finding 7: decoders read better from the idealization) + role-conditioned match in the reranker. **Honest claim discipline: in-distribution gains are NOT the claim** (top-1 already 89.6–100 on the floors); the claim axes are (a) the OOD delta over T1's baseline by a preregistered margin, and (b) kernel_opt long-tail crowding (426 rules, the biggest corpus). Floors MUST hold (the 14/14 retrieval gate + score-bench flat unless OOD wins land).
- [-] **T5** **BoW-null router per domain:** *(prior measured by Bench 062: `rustc_errors` roles are 6/8 single-filler, so m=1 shared-role ALS should show little residual gain there; `clippy_lints` has 13 multi-filler roles)* fit the m=1 shared-role ALS per corpus; residual ratio `r_bow/r_full < 1+ε` ⇒ that domain carries no binding structure ⇒ structured machinery skipped there. Records a per-domain structure verdict (prediction to test: clippy_lints strongly structured; kernel_opt categories possibly weaker than rule-level).
- [-] **T6** **Counterfactual surgery on LatentFixMemory trajectories:** for score-bench persist-miss spans, surgery-swap the rule/domain filler and ask closed-form "would domain X / rule Y have healed this span?" — complements Bench 032's re-run-based switch-cost measurement with a zero-recompute counterfactual; feeds the post-mining intake (`.distill/001_mining_queue_snapshot.md`) with attributed misses.

## GOAT gate

> Also all `[-]`: these gates measure the Phase 2 machinery, so they inherit its
> unblock (707 T1–T5). G8's OOD baseline is the ONLY part already in hand — Phase 1
> produced it (Bench 062), and the caveats recorded above bound what G8 may claim.

- [-] **G1** ALS determinism: monotone-certificate assert + double-run bit-identical artifact.
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
- [-] **Promote** Only after gates pass AND T1 shows the OOD gap the axis claims to close; opt-in until then.

## Cross-refs

- katgpt-rs: R527 (§3 Fusion F5 = this issue), `.issues/707` (the `tpr` primitive; its G8 "real downstream consumer" battery is satisfied by T1+T4 here — the healer corpus is a better validation dataset than a synthetic micro-model).
- Existing substrate consumed, not duplicated: riir-rag `RuleIndex` (already consumes the R299 Clifford wedge KNN — the wedge is pairwise interaction; TPR adds role composition), `RerankMode::Structural`, `clippy_oracle.rs` ground truth.

**Consumer note for katgpt-rs 707:** the primitive's first real-world validation consumer is THIS issue's T1/T4 — the healer corpus (580+ labeled (rule, shape) bindings, oracle ground truth, measurable downstream accuracy) replaces a synthetic micro-model for the G8 intervention battery. T1 runs without the primitive and produces the OOD baseline that gates Phase 2.
