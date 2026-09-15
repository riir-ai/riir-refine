# Corpus authoring discipline — the meta-rule (how rules get authored)

> **What this is:** the explicit meta-rule guide consulted at mining time —
> Library Drift's "meta-skill" ([Issue 102](../../.issues/102_corpus_lifecycle_library_drift.md)
> T5; source arXiv:2609.11873 §3.5/§6 adopting arXiv:2605.19576). It
> formalizes practice that already exists in the batch protocol, the
> `distill` skill, and the corpus records — nothing here is new policy.
> When this file and a skill disagree, the skill wins and this file gets
> fixed in the same commit.

Every quality property downstream — L1 retrieval precision, the L2 verify
gate's revert rate, the GOAT verdicts, the binary cost of the corpus
tables — is decided at authoring time (L0). A rule that enters the corpus
wrong costs every future heal run; the authoring filters below are
ordered cheapest-first, so most candidates die before any code is written.

## 1. Route before you propose

A candidate rule belongs to exactly one `HealDomain` (Plan 001). Decide
the domain FIRST: a rule authored into the wrong domain dilutes that
domain's retrieval fan-out and its validator's oracle is wrong for it.
Domain tests live in the `distill` skill (§domain routing); the standing
example is the shader split — compute/dispatch/buffer concerns are
`kernel_opt`, fragment/vertex-stage concerns would be a separate
`shader_perf` domain with a different scanner and oracle, never a
`kernel_opt` entry.

## 2. Dedup grep BEFORE proposing (vocabulary translation)

Grep the existing corpus for each candidate concept BEFORE writing any
entry — and grep NAME VARIANTS, not one name: concepts ship under
operator names. A single-vocabulary grep returns zero hits for a concept
that is fully covered. `grep -ri "<concept>" src/domains/*/corpus/` plus
2–3 alternates is the floor; the read of adjacent hits decides
DUPLICATE (drop), ADJACENT (note the neighbor, author the delta), or
NEW (proceed). Batch 30 dropped 13 of its candidates at this filter —
it is the load-bearing step, not ceremony.

If the candidate is a consolidation of two existing rules rather than a
new rule, stop: that is a corpus REFACTOR (the cross-OOO axis, Issue 098
T4 / Bench 086 — e.g. clippy's own `single_match`/`single_match_else`
split is the recorded consolidation candidate), not a mining batch.

## 3. Probe against real clippy FIRST

No rule lands without probe evidence — a temp crate + real
`cargo clippy --message-format=json` run establishes: that the lint fires
on the shape TODAY (the pinned toolchain version), which shape variants
fire and which are SILENT (the probe envelope), and what clippy's own
suggestion does (applicability, and whether it is CORRECT — clippy's
suggestion can be wrong; the `useless_format` escape class, HISTORY
§P23, is the canonical specimen: the healer must refuse consuming it
where the literal carries brace escapes).

The probe corrects the intake's premise routinely — `filter_map_bool_then`
was intake-described as a "filter+map split" lint and the probe showed the
reverse (it fires on `bool::then` INSIDE `filter_map`; the split is the
FIX). Batch 33's rejected seed is the precedent: probe evidence or no
landing.

## 4. Test decision content, not facts

A rule must encode a DECISION: pattern → bounded fix + regime + rationale.
Two-regime structure is required — state WHEN the fix wins and WHEN it
does not (the pufferlib bf16-store candidate was rejected as "a 5-line
intrinsic wrapper with no two-regime structure"). A fact about the world
("crate X is slow") or a bare style preference is not a rule. The
`distill` skill's §decision-content test is the formality.

## 5. Bound the fix-space; one anti-pattern per proposer

Every entry names: pattern, bounded fix, category, rationale, source
quote — and the quote is re-verified against the real file before landing
(Batch 31). The proposer fires on ONE shape. A broad proposer that fires
on virtually every trivial fn cross-fires every eval fixture and
pollutes the instantiation re-ranker's structural signal —
`missing_const_for_fn` (the fired-histogram's #1, 723 instances) was
implemented and KILLED at the gates for exactly this; the record is
`seed_corpus.md` + `.plans/050`.

Refusals are part of the fix-space: document what the proposer will not
touch (attributed/cfg'd elements, macro-argument token streams — syn's
visitor does not descend into them, the P13 finding — arg-position
roots, non-const sizes). A refusal with a recorded reason is knowledge;
a silent one is a future wrong edit.

## 6. Tier honestly (fix / verify / suggest)

- FIXABLE = the rewrite is parse-provable and behavior-preserving by
  construction; the shape is narrow enough that wrong fires are
  structurally impossible.
- VERIFY = parse-only cannot see the type; wrong fires must be
  check-visible (E0599/E0594/E0507 class) so the compile gate reverts
  them. Type-blind shapes whose detection is still worth attempting land
  here, never in FIXABLE.
- SUGGEST-ONLY = the fix is architectural (a parse-only proposer cannot
  see the direct-access path) or the domain is advice-shaped (`dist`).

Measured SAFER-than-clippy divergences are allowed and good — the
proposer takes the form real clippy cannot correctly suggest (the
`chunks_exact` `.0.iter_mut()` chain fix, the `map_or` fallback for
type-unknown receivers) — but each one is pinned by a test naming the
divergence and its probe evidence.

## 7. Check fixpoint safety (ping-pong)

The rewrite must not re-enter another proposer's territory or its own.
`option_if_let_else` was REFUTED at review: its output re-enters
`single_match_else`'s territory → fixpoint ping-pong with our own
proposer. No corpus entry. When a fix LEGITIMATELY births a new lint
(the gen-8 closure-birth mechanism — `manual_map`'s arm→closure splice
produces `redundant_closure`-shaped code outside FIXABLE), price it:
the shaped-verdict axis (half-credit) exists for exactly this and the
born-lint cluster is documented, not merged away.

## 8. Gate the batch (the GOAT is not optional)

A batch is NOT done at corpus-add time. G1: every new rule's fixed code
parses (the standing `batchNN_pseudocode_parses` gate extends per batch).
G2: lib tests pass. G3: no-regression — clippy 0 in BOTH feature states +
the CI guard. Kernel batches re-measure retrieval precision against the
pinned floors (kernel self-variants, blended 3-domain) — a decay FAILS
the batch; fix retrieval or re-pin with the delta recorded, never
weakened silently. rust_perf rules run the Pareto bench with alloc-count
and carry `context_notes` on regime-fragile/Mixed verdicts (the
MEASURED + REGIME form, `entries_05.rs:141` is the model). Never promote
a perf gain on a wrong result.

Count pins walk per batch (the Batch-143 walk — corpus count, gate
counts, bench-spec count, drafter-propagation count) and any stale
prose count in the docs is repaired in the SAME commit.

## 9. Provenance and honesty

Organic-feature-churn provenance = 1.0; synthetic/injected = ε
(`frontier_pairs`, Issue 089 T5) — and NEVER inject failures on purpose.
Corpus mining from external sources uses the `.raw` discipline: clone
into `<repo>/.raw/<name>` (gitignored), pin `@ <sha>` + license in the
note BEFORE the verdict, `rm -rf` when the batch lands. WIP/provenance
caveats are written into the research note, not remembered.

## 10. The corpus is append-only (the lifecycle law)

Deletion is never a lifecycle op (Issue 089 Caution 2, adopted by Issue
102): entries are kept forever; demotion = exit the retrieval fan-out,
and the demotion/cap machinery is itself gated on the fade report
proving predictive (report-first law, R135/Bench 047 — the same law that
keeps admission unwired). An authoring mistake is fixed by editing the
entry + a fixture, never by removing it quietly; retirement candidates
go through the saturation/fade reports first.

## 11. Distinct-from-neighbor rows

Every kernel rule (and every rust_perf rule with close neighbors) carries
an explicit "distinct from `<neighbor-rule>`" note — what this rule is
NOT, at retrieval granularity. This is the dedup decision (step 2)
preserved IN the entry so future dedup greps and the cross-OOO axis can
read it without re-deriving it. Omit it and the next author either
re-authors a duplicate or merges two distinct rules.

## 12. The meta-recursion

This file is maintained by the discipline it describes: when a batch or
intake lands a NEW authoring lesson (a divergence class, a killed lead, a
probe correction, a generator lesson), append it here — or extend the
entry it belongs to — in the SAME commit. Generator lessons
(edit_file escape-stripping, `ensure_ascii=False`, line-initial
`+ ` doc-markdown, CRLF-before-fmt) live in AGENTS.md batch entries and
the `.distill/001` standing rules; this file holds the craft, not the
mechanics.

## Worked examples (killed and refuted leads are knowledge)

| Lead | Verdict | Lesson |
|---|---|---|
| `missing_const_for_fn` (723 instances) | KILLED at gates | broad proposer = cross-fire; single-anti-pattern invariant (§5) |
| `option_if_let_else` (38 instances) | REFUTED at review | fixpoint ping-pong with own proposer (§7) |
| `filter_map_bool_then` intake premise | CORRECTED by probe | probe-first (§3) — the intake described the lint backwards |
| `useless_format` oracle consumption | GUARDED (P23) | clippy's own suggestion can be wrong; escape-free gate (§3) |
| pufferlib bf16 AVX2 store | corpus-NO | no two-regime structure → not a rule (§4) |
| `chunks_exact` chain shape (P13) | EXTENSION, measured safer | SAFER-than-clippy divergence, pinned + documented (§6) |

## See also

- [`seed_corpus.md`](seed_corpus.md) — the entries these rules produced, with their killed/refuted leads
- [`../../.distill/001_mining_queue_snapshot.md`](../../.distill/001_mining_queue_snapshot.md) — live queue + standing rules (read at session start)
- [`../../.agents/skills/distill/SKILL.md`](../../.agents/skills/distill/SKILL.md) — source verdicts, domain routing, decision-content test
- [`../11_domains/`](../11_domains/README.md) — per-domain corpora + validators
- Issue 102 (this file's origin), Issue 089 (frontier/admission + the never-delete law), Issue 098 T4 / Bench 086 (cross-OOO axis)
