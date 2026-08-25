# Issue 042 — Self-collapse monitor for the fix-trajectory retrieval loop (RAG-collapse measured instance)

**Research:** riir-ai [R346](../riir-ai/.research/346_rag_collapse_self_authored_retrieval.md) (arXiv:2608.22118, Druck & Smith — "RAG Collapse"). Sibling of our own [Research 049](../.research/049_Long_Horizon_AI_Research_Evidential_Status.md) (retelling bias, arXiv:2608.11195): 049 fixed the *consolidation* laundering (EvidenceTier); this is the *retrieval* half the new paper measures.
**Composes with:** [Issue 037](037_e2e_self_evolve_adoption_proof.md) (the audit script gains the columns), [Issue 021](../../.benchmarks/026_evidential_status_axis_goat.md)'s EvidenceTier (the quality axis the paper measured does NOT remove self-bias), [Issue 038](../.docs/10_self_evolve/latent_fix_memory.md)'s TrajStore (the corpus being measured).

## Problem

`HealPipeline::heal` Step 1 retrieves past trajectories for `(rule, buggy_code)` from `LatentFixMemory` / the Warm-tier `TrajStore`; every applied fix is written back via `record_applied`. This is a **closed self-referential retrieval loop** — the exact structural shape arXiv:2608.22118 measured collapsing in LLM RAG (79.6% of 1,528 simulations; a single self-authored reference suffices; self-authored content is cited at 38.9% vs 7.4% for human-written, and the gap **survives an 8-dimension quality regression at +26pp**). Mechanism transfer is mechanical, no LLM needed: a memory trajectory IS the answer to a previous similar query, hence maximally similar to future queries — the similarity score that drives our latent+BM25 retrieval is the same property that drives collapse.

Our existing defenses are the axes the paper measured as insufficient: EvidenceTier trust is a *quality* control (the +26pp survived quality regression); Issue 030's rule-coverage gate declines *off-corpus* memory, not similarity-advantaged *related* memory. Nothing measures the collapse signature: the **Memory-fraction of top-k candidates over time** and the **rule-entropy of the suggestion pool** as the store ages.

The store to measure already exists (the 8,235-trajectory production store from the sweep campaign; the sweep validated manually because "the verdict loop never closed" until P5) and the audit infrastructure exists (`self_evolve_audit.sh`).

## Falsifiable outcome

The paper's prediction: older/larger stores show elevated Memory-source fraction and reduced suggestion rule-entropy vs fresh stores at matched fixture inputs. If the production store shows NO elevation vs fresh baselines, the transfer does not hold at our scale/params → record the negative in R346, close with the monitor still shipped (cheap tripwire, standing value).

## Tasks

- [x] T1 measured baseline: run the dry `--suggest` path over a fixed fixture set against (a) the 8,235-trajectory production store, (b) a fresh empty store, (c) optionally a mid-age store; record per-span top-k composition — `HealSource` counts (Memory vs Drafter/Corpus) + distinct-rule entropy of the top-k. `HealSource` already exists (`self_evolve.rs`) — this is instrumentation of an existing enum, no new axis needed for the measurement. *(Superseded by the verified mechanical finding: the suggest path never consults memory — measured the `heal()` fix-path retrieval instead, per the issue's own redirect.)*
- [x] T2 audit column: `self_evolve_audit.sh` gains memory-fraction + mean top-k rule-entropy (feeds Issue 037's adoption observability; a collapse trend is a degradation signal for the A-axes).
- [x] T3 owner-gated mitigation design note (numbers first, design after T1): (a) self-fraction cap on any top-k (a diversity quota composing with the existing diverse-retrieval philosophy), vs (b) origin discount composing with trust (`origin(self) ∧ !certified → discount`; certified-self neutral — Stage 2 verified fixes should not be suppressed), vs (c) monitor-only. The paper's quality-control-immunity evidence argues for (a) or (c) over pure per-item discounts.
- [x] T4 record the verdict in R346 §"PoC Addendum" (elevated / not-elevated) + this issue resolved with the commit hash.

## Non-goals

- No change to retrieval weights before T3's owner call.
- No LLM-based self-detection (the paper §15.1 shows detection is fragile and false negatives are catastrophic at single-reference sensitivity; our origin axis is structural — `HealSource` — not classifier-based).
- No TrajStore schema change (origin is derivable from source tags at query time).

## Resolution (2026-08-25)

**Verdict: NOT ELEVATED** — the paper's collapse signature does not transfer
at our scale/params. Full record: [Bench 048](../.benchmarks/048_self_collapse_monitor.md);
R346 carries the §"PoC Addendum". All tasks closed:

- [x] **T1 measured baseline** — `src/collapse_monitor.rs` instruments the
  REAL loop (Step 1 `retrieve_into` + Step 2 `draft_fix` + Step-3 dedup +
  `heal()` selection) over the 48-span fixture set. Production envelope
  (8,241 rows) vs fresh vs dose-response prefixes (100/1k/4k/8k) + 6
  per-repo stores + 2 secondary regimes (dominant-rule, exact-corpus).
  Determinism: double-run bit-identical, asserted. Numbers in the bench.
  Two structural findings en route: `--suggest` cannot collapse (corpus-only
  retrieval); the CLI fix path never reads memory back — risk is live at the
  API level only.
- [x] **T2 audit columns** — `self_evolve_audit.sh` §1c (pool memory-fraction
  + top-k rule entropy + mem-win-rate per store, via the read-only
  `--collapse-report` bin mode; 139 ms on the 8k store; stale-binary probe
  degrades gracefully).
- [x] **T3 design note** — in Bench 048 §T3, numbers-first. Recommendation:
  **(c) monitor-only**; re-arm trigger for the origin discount (option (b),
  certified-neutral) = a consumer reading retrieval order OR head-to-head
  wins detaching from the verdict-bucket share. Option (a) rejected with
  numbers (the quota is inapplicable in the vacuum regime).
- [x] **T4 verdict recorded** — R346 §"PoC Addendum" (NOT ELEVATED + the three
  mechanisms) + Status line updated.

Commits: riir-clippy `feat(042): self-collapse monitor…` (instrumentation +
script) + `docs(042): bench 048 + issue resolved` (this file's content moves
to Bench 048 + R346 per the noise-reduction rule); riir-ai
`docs(346): PoC Addendum — healer-side NOT ELEVATED (Issue 042 / Bench 048)`.

Gates: latent_retrieval lib 760/0/1i · default lib 677/0 · clippy
`--all-targets -D warnings` 0 in both states · determinism bit-identical ·
production stores measured on /tmp copies only.
