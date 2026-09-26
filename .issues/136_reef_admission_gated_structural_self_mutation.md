# Issue 136 — admission-gated structural self-mutation lane (reef distill)

**Status:** Open — filed from riir-clippy `.research/213_reef_continual_self_improving_agent_infra.md` (reef @ ecec4e20b29e7dcbd96150d4b4f1a80b146e120d, Apache-2.0).
**Class:** Gain (architectural). No benchmark, no feature flag yet — this is the triage issue for the lane.

## Problem

The healer already has **outcome gates** on numbers (`self_evolve` Lemma-4.2 tier certification, Elo/BetaPosterior selection, lexicographic keep rule) and **oracle gates** on content (`frontier_miner` verifies against real cargo clippy), but **structural** changes to its own surface are human-batch-only: corpus entries land via mining batches, `--active-set-demote` is a human act by law, config values evolve only through GEPA-D's numeric bandit. There is no vocabulary for a *proposed structural edit* to be admitted, reviewed, committed, and rolled back without a human hand-writing every batch.

## Proposal (consume reef's safety vocabulary, P1+P3+P5)

1. **Per-kind shape gates** on structural proposals (reef P1): the healer's mutable surface is typed (corpus entry / config field / active-set marker / fixture / playbook); each kind gets an admission plugin that validates SHAPE at proposal time (e.g., a corpus entry must carry a bench spec or be explicitly suggest-only; a fixture must not embed credentials — reef's `_SECRET_TEXT` class; a config edit must name an existing knob). A proposal that fails its kind's gate is a FAILED fiber: logged, never applied.
2. **Person-review classes** (reef P3): review is keyed on the KIND's privilege, not on confidence — anything that can execute (a playbook change, a fixture that ships code, a `-D` list edit) is ALWAYS_REVIEWED regardless of how good its oracle score is; data-shaped kinds (counts, centroids, frontier rows) can auto-serve through the commit chain.
3. **Two-head commit chain** (reef P5): immutable content-addressed artifacts + `current`/`checkpoint` heads with CAS advance, so a bad auto-served batch is a repoint, not a revert.

## Non-goals / constraints

- Quality gates stay exactly where they are: oracle (clippy) for content, score-bench/freeze gates for corpora, GEPA-D for numeric config. The shape gate is ADDITIVE and pre-oracle.
- This does not promote any auto-mutation today. Landing = the gate + chain vocabulary behind an opt-in feature; activation of any auto-serve kind is a separate owner call.
- The flat-tree invariant maps to: no proposal may group sub-edits out of sight of the root walk (one proposal = one visible diff).

## Provenance

- reef `harness/tree/nodes.py` + `artifact/release_chain.py` @ ecec4e20b29e7dcbd96150d4b4f1a80b146e120d.
- Signal-diff vs GEPA-D / MUSE / frontier_miner / MotifAdmitter: `.research/213` §Dedup-grep evidence.
