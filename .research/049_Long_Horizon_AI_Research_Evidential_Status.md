# Research 049: Long-Horizon AI Research Harness — Evidential Status & State-Representation Findings

> **Source:** "Long-Horizon AI Research for Grothendieck Constant: A Case Study in Human–AI Mathematical Collaboration" — [arXiv:2608.11195](https://arxiv.org/abs/2608.11195) — Li, Saha, Xue, Chaudhuri, Klivans, Kothari, Meka (UT Austin / Princeton / UCLA), 2026-08-14
**Date:** 2026-08-15
**Status:** Implemented — Issue 021 shipped 2026-08-16 (Bench 026); the §2.3 game-side provenance note remains a research lead only
> **Related Research:** katgpt-rs R461 (PRO-LONG — the lossless-log thesis), riir-ai R169 (AgentMemBench F6/F7/F9), riir-ai R333 (Prime Agent continual-harness verdict), katgpt-rs R473 (GAMER dual-stream Q+/Q−), katgpt-rs R104/R088 (AlphaProof Nexus), katgpt-rs R368 (AutoMem)
> **Related Plans:** riir-clippy Plan 005 (latent retrieval + `cargo heal --suggest`)
> **Cross-ref:** companion math paper [arXiv:2608.11158] "New Lower and Upper Bounds for the Grothendieck Constant" — **PASS** (no SDP/rounding surface in the stack; certification methodology already covered by Lean + spec-match; interval arithmetic explicitly rejected in katgpt-rs R220). Redirects landed in katgpt-rs R087 (ConvexTok) + riir-chain R004 (LatCal Lean bridge).
> **Classification:** Private

---

## TL;DR

A 6-week human-steered AI research program (reasoning model + coding agent + bulletin + file-based session reports; ~240 sessions, 2,091 reasoning calls, 152M tokens, ~$5,400) tightened the Grothendieck-constant bounds. The load-bearing findings for THIS repo are about the harness, not the math: (1) a clean decomposition of long-horizon research into **technical execution / research judgement / research-state representation**, with measured asymmetry — execution strong, the two global functionalities weak; (2) the **retelling-bias failure mode**: iterated compression of the curated state loses caveats and evidential status while headline values survive (an uncertified numerical score got promoted to a record; a proved lemma was lost and reproved 25 days later); (3) humans supplied ~40 program-level directives, the decisive one being "stop varying the failing search — synthesize the failures into an obstruction theorem."

**Distilled for riir-clippy (self-adaptive agent loop):** our `LatentFixMemory` (`FixTrajectory`) has exactly the schema the paper's session-44 incident predicts will fail: `success_count`/`fail_count` u32s + boolean `record_outcome`, **no evidential tier, no withdrawn transition, no trust axis in retrieval**. A fix that passed only the syn-parse gate is indistinguishable in memory from one that passed the full validator — and iterated retrieval+resummarization launders the former into the latter. Fix: an `EvidenceTier` axis (Certified / Heuristic / Withdrawn) fused from GAMER's Q+/Q− split + `adaptive_drafter`'s verified-fix pool. Filed as [Issue 021](../.issues/021_evidential_status_memory_axis.md).

---

## 1. Paper Core Findings

### 1.1 The harness

Four components, deliberately boring:

1. **Reasoning model** (GPT-5.5-Pro → GPT-5.6-Sol, max effort) — does mathematics in natural language, chooses directions, proposes experiments.
2. **Coding agent** (Claude Code, Opus → Fable 5) — writes/runs/verifies the programs, incl. the Arb interval-arithmetic certificates.
3. **Bulletin** — a file carrying human directives; agents re-read it as they work (asynchronous steering without halting research).
4. **Session reports** — file-based long-term memory (code, proofs, prose); later sessions read them to continue. Progress accumulates without clogging per-call context.

Loop shape: session starts from reports+bulletin → reasoning model picks a direction → a few theory-only calls (added specifically to prevent myopia — empirically observed before) → up to 12 calls executing with the coding agent. Up to 5 parallel sessions for different proof strategies.

### 1.2 The research-cycle decomposition

Research **state** (objectives, claims, evidence, failures, open obligations) → **research judgement** (which high-level action next: prove lemma / test construction / compare failures / search counterexample / reformulate) → **technical execution** (plan + carry out the steps) → results distilled back into the updated state. Precedents: Newell–Simon problem spaces, Pólya, Schoenfeld's *control*, Schön's reflective practice.

### 1.3 The asymmetry (the paper's headline analysis)

- **Technical execution: strong.** Given a fixed high-level action, the system developed the entire lower-bound proof chain (Gaussian harmonic analysis, variational arguments, finite-dim optimization) with little further intervention.
- **Research judgement: limited autonomy.** Six mechanically distinct upper-bound escapes failed against the SAME obstruction; the run's response was to open a seventh variation rather than ask whether the obstruction was universal. Once prompted to synthesize/reframe, it did so effectively — the failure was *knowing when to activate the mode*, not the mode itself (Mason & Spence's "knowing-to act").
- **Research-state representation: fragile.** Two named incidents:
  - **Session 44 (withdrawn record):** a fast numerical evaluator built in session 4 carried an explicit caveat ("score safe only for exploration"). The caveat did not survive later compressions of the curated state; by session 44 the uncertified score held the record. Withdrawn 25 days later.
  - **The lost lemma:** a feasibility criterion was *proved* in session 8, disappeared in a later handoff, and was reproved from scratch by the test that withdrew the record.
  - In both, **the archive retained the facts; the compressed state governing decisions failed.** Iterated rewriting regresses toward the genre of finished papers — results without caveats, scope, or doubt. ~200 rewrites over the run.

### 1.4 Why the asymmetry (their hypothesis)

Training data contains vastly more local technical execution than global program-management: finished papers record product, not process (Lakatos; Thurston). Rewards are automatic for execution, sparse and open-ended for judgement. Curation is a *prediction about the future* — the record that would teach it (what was kept, what was dropped, which omission later mattered) is exactly what finished exposition omits.

### 1.5 The human contribution (3 areas)

Initial research state (framework + certification machinery + problem statement); ~40 program-level directives (the pivot: failures → obstruction theorem; priorities: generality over numerical sharpness; "certify one rung before climbing a better one"); continuous restructuring of the state representation itself (what to store, how). Technical execution was left almost entirely to the system.

---

## 2. Distillation

### 2.1 What already ships here (the paper is external validation)

| Paper finding | Shipped cousin |
|---|---|
| Lossless archive + read-time search beats curated summaries | katgpt-rs R461 (PRO-LONG) + `event_log_query` — the same diagnosis, resolved at **read time**. The paper's curated-summary failures are the write-time-resolution failure mode R461 predicted. |
| "Raw evidence > compressed summaries" / consolidation destroys episodic detail | riir-ai R169 (AgentMemBench) F6/F7/F9 → raw Pod shards, DeltaMemory write-then-gate, sleep-cycle cadence + `FreezeGateReport`. |
| Harness shape (execution loop + memory + outcome feedback) | riir-clippy `self_evolve` (G1–G7 PASS, incl. G6 corpus truthfulness + G7 vs real clippy feedback). |
| Versioned-CRUD state surface, evidence records, rollback | riir-ai R333 (Prime Agent verdict): "every distillable sub-mechanism already ships" — SkillCatalog + LocalKvStore + MerkleFrozenEnvelope. |
| Failures as first-class records (no silently-retried dead ends) | `katgpt-rs/.docs/09_feature_catalog/negative_results.md` (28+ entries) + `.benchmarks/` caveat blocks + the retracted-verdict precedent in riir-agents. |
| The whole human-steered loop itself | The user's AGENTS.md + `.agents/skills/riir-clippy` idle-loop skill + `## Summary` handoff contract + `.highwater` write-time re-scan discipline. We ARE the paper's architecture, at repo scale. |

**Fusion insight (validation direction):** the riir-clippy proposer-adaptation rule — "wrong suggestion → fix the proposer + fixture, never suppress" — is the code-healing instance of the paper's decisive human directive: *convert an accumulation of failures into the general mechanism that explains them*. The paper's pivot produced an obstruction theorem; ours produces a corrected proposer + regression fixture. Same judgement, different codification target.

### 2.2 The gap the paper exposes (actionable)

`riir-clippy/src/memory.rs` — `FixTrajectory` (L35–51): `lint_name`, `lint_name_hash`, `buggy_code`, `fixed_code`, `direction: [f32; 8]`, `success_count: u32`, `fail_count: u32`. Verified line-level:

- `record_outcome(success: bool)` — a bare boolean increment. Nothing distinguishes "passed `clippy_verify`/`rust_perf_verify` re-run" (a certification) from "syn-parse + looked right" (heuristic).
- Retrieval (`retrieve_into`, L251–302) ranks by `latent·sigmoid + bm25·hash-match` — **no trust axis**. `success_rate()` exists as a separate lookup (0.5-neutral when untried) but is not blended into ranking.
- **No withdrawn/retired transition** — a wrong entry can only be outvoted by `fail_count`, never retracted. The session-44 analog: a heuristic success recorded early can keep being retrieved as if certified.
- No scope-of-validity field — the paper's "restricted closures recorded as global progress" failure (session 55's recurring error: sub-lemmas proved in restricted settings accumulating as apparent global progress) maps to a fix validated on one fixture family being retrieved for any span.

The only certified/uncertified split in the crate is one level up (`adaptive_drafter.rs` verified-fix pool keyed by `(rule_id, buggy_hash)`) — a binary pipeline gate, not a per-memory evidential taxonomy, and it doesn't propagate into `LatentFixMemory`.

### 2.3 The fix (fused from three precedents)

**`EvidenceTier` axis on `FixTrajectory`** — see [Issue 021](../.issues/021_evidential_status_memory_axis.md):

- **GAMER Q+/Q− (katgpt-rs R473):** decouple success-rate from violation-rate so a 50%-wins-50%-catastrophes fix isn't averaged into neutrality. Direct analog: decouple *how often a fix fires* from *how it was verified*.
- **`adaptive_drafter` verified pool:** the Certified tier's admission rule already exists (validator-pass).
- **The paper's taxonomy:** Certified / Heuristic / Withdrawn, with transitions that make status **monotone under retelling** — the property whose absence caused both named incidents. Withdrawn should be absorbing (like riir-chain's slashing monotonicity, proven in Lean: once slashed, never un-slashed) — a retracted fix is never re-emitted as a suggestion without an explicit re-certification event.

Game-side fusion idea (**novelty TBD, not claimed**): the retelling-bias mechanism applies to NPC memory consolidation too — Raven/δ-Mem sleep-cycles compress raw experience; a provenance axis (witnessed vs heard vs inferred) would keep hearsay from consolidating into ground-truth beliefs. R169 already mapped the compression half; the provenance half is unexplored. Filing as a note here only — needs its own substrate-first pass before any claim (the two-brain confidence decay + coverage-mask work in `coverage_curiosity` are the nearest shipped substrate).

**Close-out (2026-08-16):** Issue 021 shipped — see [Bench 026](../.benchmarks/026_evidential_status_axis_goat.md). `EvidenceTier` (Certified/Heuristic/Withdrawn) + absorbing-Withdrawn transitions + trust-weighted retrieval (Withdrawn never suggested; Certified beats a small-similarity-edge Heuristic at default weights) landed with the full-validator admission wired through the Stage 2 tuning loop. The honest A/B finding: exactly-equal similarity between two distinct-tier entries is NOT constructible through the real API (`record_outcome` matches by `(lint, fixed)` — one stats bucket), so the falsifiable A/B is the within-subject certification delta (exactly the 0.1 trust gap) + the cross-entry small-edge flip. Scope-of-validity (the stretch goal) deferred. The game-side provenance note above remains unclaimed/unshipped.

---

## 3. Verdict

**Gain.** One-line: the harness/state-compression findings validate shipped design (R461/R169/R333) — no new files warranted for that half — but the paper exposes a concrete, line-verified failure mode in our own shipped agent memory (`FixTrajectory` has no evidential axis, no withdrawn transition, no trust-weighted retrieval) with a fused fix from two in-tree precedents. Issue 021 filed; no plan until the issue's POC shows retrieval-ranking benefit.

**Super-GOAT gate: fails Q2/Q3** (not a new capability class; internal tooling, no MMORPG product selling point). Correctly routed as Gain.

**MOAT gate (riir-clippy):** in scope — strengthens the self-adaptive track (`self_evolve` memory semantics) which is this repo's active moat surface. Not katgpt-rs material (agent tooling, not inference math); not riir-ai material (the actionable substrate lives here).

**Defend-wrong scope:** verdicts in this note are architectural (grep + line-level reads via subagent, sufficient per §3.6); no quality-parity claim made. The issue's G-gates define the empirical burden.

---

## 4. Fusion

- **Paper × GAMER R473 × adaptive_drafter:** `EvidenceTier` = Q+/Q−-style decoupling of verification evidence from outcome counts + the drafter's verified-pool admission rule + the paper's Withdrawn taxonomy, with riir-chain's slashing-monotonicity pattern as the transition discipline. None of the three alone gives per-entry evidential status that survives retrieval+resummarization.
- **Paper × PRO-LONG R461:** the paper's failures are the missing empirical arm for R461's thesis — R461 showed lossless-log+search wins on benchmarks; the paper documents the curated-summary alternative failing in the wild with named incidents. Cross-cite both directions.
- **Paper × the batch-mining loop:** "synthesize repeated failures into the general mechanism" is the shared judgement move; worth one line in the skill's mining-unit description so future sessions recognize pivot-from-failure as a first-class move, not a human-only intervention.

---

## References

- Li, Saha, Xue, Chaudhuri, Klivans, Kothari, Meka (2026). Long-Horizon AI Research for Grothendieck Constant. arXiv:2608.11195
- Saha, Li, Xue et al. (2026). New Lower and Upper Bounds for the Grothendieck Constant. arXiv:2608.11158 (companion; PASS)
- Naor & Regev (2014). Krivine schemes are optimal. Proc. AMS 142(12).
- katgpt-rs R461 (PRO-LONG), R473 (GAMER), R104/R088 (AlphaProof Nexus); riir-ai R169, R333; riir-chain slashing monotonicity (`.proofs/RiirChainProof/Economics/SlashingMonotone.lean`)
