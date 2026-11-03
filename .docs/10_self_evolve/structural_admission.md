# Structural admission — the self-mutation safety vocabulary (Issue 136 / reef distill)

> **Purpose:** the PRE-oracle layer for structural changes to the rulewright's
> own surface. The loop already owns outcome gates on numbers
> (`self_evolve`'s Lemma-4.2 tier certification, Elo/BetaPosterior
> selection, the lexicographic keep rule) and an oracle gate on content
> (`frontier_miner` verifies against real cargo clippy) — but structural
> edits (corpus entries, config knobs, active-set markers, fixtures,
> playbooks) were human-batch-only. This module ships the missing
> vocabulary, taken from reef @ `ecec4e2`
> (Research 213 P1+P3+P5):
>
> 1. **Per-kind shape gates** (reef P1) — a [`StructuralProposal`] is
>    admitted or refused by its kind's SHAPE gate at proposal time; a
>    refusal is typed ([`Refusal`]) so a future admission log can bucket
>    FAILED proposals without re-parsing prose. Shape only — the oracle,
>    the score-bench/freeze gates and GEPA-D stay exactly where they were;
>    this layer is ADDITIVE and pre-oracle.
> 2. **Person-review classes keyed on KIND** (reef P3) — review is keyed
>    on the kind's privilege, never on confidence. 4 of the 5 kinds are
>    `AlwaysReviewed` (fixture, config knob, active-set marker, playbook —
>    each executes or changes executed behavior); `CorpusRule` is the one
>    data-shaped `AutoServable` class. The honest finding is recorded, not
>    smoothed over: the rulewright's surface is mostly privileged.
> 3. **Two-head commit chain** (reef P5) — `ChainState` carries
>    `current` + `checkpoint` heads over immutable content-addressed
>    artifacts (full 64-hex BLAKE3 of the canonical proposal JSON) with CAS
>    advance, plus a `live` child built from the checkpoint WITHOUT moving
>    a head. A bad auto-served batch is a CAS repoint, not a revert.
>
> **Flat-tree invariant** (reef): enforced by the TYPE — one
> `ProposalPayload` = exactly one edit; no list-of-edits variant exists to
> construct. One proposal IS one visible diff.
>
> **P2 fold:** every text-bearing payload is scanned for credential shapes
> at admission (`sk-`/`ghp_`/`github_pat_`/`AKIA`/`xox*`/`AIza` prefixes at
> word boundaries with full-length continuations, plus PEM private-key
> blocks) — no regex dep. The directive-shaped half (prompt-injection
> phrasings) stays folded OFF: no prompt-bearing store persists today
> (Research 213).

## Files

| File | Scope |
|---|---|
| [`src/structural_admission.rs`](../../src/structural_admission.rs) | The UNGATED pure vocabulary: kinds, review classes, shape gates, secret scan, chain state machine |
| [`src/structural_admission_store.rs`](../../src/structural_admission_store.rs) | The `structural_admission`-gated store: `.refine/structural/artifacts/<blake3>` + `.refine/structural/heads.json` CAS |

## Standing posture

- **Nothing mutates anything.** The gates are pure functions; the chain is
  a pure state machine; the store has no production caller. Activation of
  any auto-serve kind is a separate owner call — the feature gates the
  persistence surface only, and no auto-serve is wired.
- **Review classes are kind-keyed, not policy-keyed** — no oracle score,
  however high, moves a privileged kind out of person review.
- **Rollback is a commit, not a revert:** `rollback_to_checkpoint` is a CAS
  advance of `current` to the durable checkpoint's artifact; history stays
  intact for audit.
- **The store refuses corruption loudly:** an artifact whose bytes no
  longer hash to its address is refused on every touch; a heads file with
  an unknown version is refused (CAS semantics could differ across
  versions — a silent accept would be a repoint past an unread state
  machine).
- **Word-boundary guard lesson:** the secret scan reads `task-` and
  `risk-assessment` WITHOUT firing — a prefix hit requires a non-word byte
  (or string start) before it plus a full-length continuation run.
  Prefix matching without the boundary + run guards fires on prose (the
  test pins seven shapes and six near-misses). Since Issue 151 T9 the
  scan CONSUMES the one canonical scanner
  ([`kat_redact::scan_secrets`](../../../riir-kat/src/kat_redact.rs) — the
  consolidated v4/v5 set), post-filtered to the persistence boundary's
  posture: prefix-kind ∪ private-key findings only (entropy/env/bearer/
  conn rules would block every payload carrying hashes) + the word
  boundary; the private prefix table is gone (its `AIza`-vs-gate drift
  was the fragmentation evidence). The legacy shape names are pinned by
  the tests and preserved via a kind→name map.

## Provenance

- reef `harness/tree/nodes.py` + `artifact/release_chain.py` @
  `ecec4e20b29e7dcbd96150d4b4f1a80b146e120d` (Apache-2.0), quotes
  verbatim-verified at distill time — Research 213.
- Landed by Plan 175 (`riir-refine/.plans/175_issue136_structural_admission_lane.md`).

## See also

- [`../09_lessons/modelless_first.md`](../09_lessons/modelless_first.md) — the lane stays modelless (gates are pure, not learned)
- [`latent_fix_memory.md`](latent_fix_memory.md) — the outcome-gate layer this composes ABOVE
- Research 213 (`.research/213_reef_continual_self_improving_agent_infra.md`) — the full distill + dedup evidence
