# Research 302 — astral-sh/uv PR #22651: marker DNF covers extracted from decision diagrams (the distill verdict)

**Status:** DISTILLED — pending owner decision

## Source + pin

- **Source:** <https://github.com/astral-sh/uv/pull/22651> — "Extract marker DNF covers from decision diagrams", author charliermarsh (Astral), merged 2026-10-18 into `astral-sh/uv` `main`.
- **Pin:** head `551c17c` (the branch tip at merge; short sha — resolve to full at clone time), merge commit `2978e76`, branch `charlie/marker-isop`, 5 commits (`0335d73`, `40312c6`, `d462a17`, `73761cc`, `551c17c`).
- **License:** MIT OR Apache-2.0 (uv's standing dual license) — verify the LICENSE files at clone (plan T0b).
- **Inspection date:** 2026-10-21. **Inspection depth:** the full PR `.diff` fetched and read end to end (every hunk); NOT a repo-wide clone — this is a single-PR verdict, and the quotes below are from the diff text at the pinned head. At mine time, clone into `.raw/uv` at the merge and re-verify every quoted line (the standing `.patch`-read discipline; B231 precedent).
- **Provenance chain:** the PR ← the **Minato–Morreale ISOP recurrence** (irreducible sum-of-products over a decision diagram), cited IN-SOURCE (isop.rs module doc): "See <https://people.eecs.berkeley.edu/~alanmi/publications/2003/iwls03_sop.pdf>, section 4." (Mishchenko, IWLS 2003). If the pattern is mined, cite BOTH: the PR for the Rust shape and the IWLS03 notes + Minato's BDD literature for the math.

## What the PR does

`uv-pep508` represents PEP 508 environment markers as a hash-consed, reduced, variable-ordered decision diagram (an interned BDD over marker variables — `NodeId`, a global `Interner`). Converting a marker to user-facing DNF previously enumerated every root-to-TRUE path and then ran a quadratic clause-simplification pass. The PR replaces both with a memoized ISOP extraction over the diagram's nodes, producing a **shared** cover that is expanded to clauses only at render time.

The replaced code's own doc comments are the cleanest statement of what died (both deleted in this PR, `marker/simplify.rs`):

> "Completely simplifying a DNF expression is NP-hard and amounts to the set cover problem."

> "Note: This function has quadratic time complexity. However, it is not applied on every marker operation, only to user facing output, which are typically very simple."

— "typically very simple" was the load-bearing bet, and conflict-heavy markers (many mutually-exclusive extras) broke it. The PR body:

> "We now extract marker DNF directly from the decision diagram with a memoized ISOP algorithm, instead of enumerating paths and simplifying the resulting clauses. This avoids expensive clause comparisons for markers with many conflicting extras (and replaces #22640, #22641, and #22642)."

> "Marker strings _can_ change (i.e., you can end up with a slightly different DNF), but should remain logically equivalent."

> "We also fix Python-version complexification to preserve the variable ordering required by the traversal."

### Measured (PR body, microseconds, `main`-before-stack `b8993e0` vs PR `f9df0b8`)

| Operation | Main | This PR | Speedup |
|---|---:|---:|---:|
| Two-clause DNF | 0.670 | 0.433 | 1.5× |
| Mixed Python/platform DNF | 1.159 | 0.658 | 1.8× |
| Sparse DNF, 512 clauses | 13,239.5 | 65.0 | 204× |
| Conflict DNF, 32 extras | 483,748.7 | 46.0 | 10,500× |
| Conflict-dependent formatting, 32 extras | 1,312,396.2 | 109.9 | 11,900× |

CodSpeed on the PR: `marker_to_dnf_conflicting_extras[32]` 227,071 µs → 37.9 µs (×6,000); `imbibe_and_format_marker_conflict_dependent[32]` ×9,800; one honest small-cell regression disclosed: `imbibe_marker_conflict_dependent[32]` 137 ns → 145 ns (−5.5%, the `Cover` indirection on a tiny marker — the regime trade, visible at the anti-regime end).

## The mechanism — four load-bearing decisions

All quotes verbatim from `crates/uv-pep508/src/marker/algebra/isop.rs` (new file, +354) at the pinned head.

1. **Extract the cover from the canonical structure; never materialize the path set.** Module doc, verbatim: "Extract a sum-of-products cover directly from a decision diagram." The old `to_dnf` doc admitted the shape it replaced, verbatim: "Instead, you must traverse and collect all possible solutions to the diagram, which can be used to create a DNF expression, or all non-solutions to the diagram, which can be used to create a CNF expression." Path count through a diagram with k independent conflicting decisions is exponential in k; node count in a reduced hash-consed diagram is not.

2. **The output is shared like the input.** Verbatim: "A shared cover whose paths are expanded only when producing the final DNF." `enum Cover { False, True, Guard(NodeId), Branches(Rc<[(Guard, Self)]>) }` — the intermediate is a tree with `Rc` sharing, and equivalent suffixes merge their guards (the `IndexMap` entry-occupied arm ORs the two guards; comment, verbatim: "Equivalent suffixes can share a union of ranges, including inequalities."). Expansion to `Vec<Vec<MarkerExpression>>` happens only in `collect_cover`, at the leaf renderer. The exponential blowup is confined to the last linear pass over an already-irreducible cover.

3. **The don't-care interval memo.** Verbatim: "Return a cover and its function `I`, where `lower => I => upper`. The interval allows recursive calls to use already covered cases as don't-cares." The cache key is the PAIR: `cache: &mut FxHashMap<(NodeId, NodeId), (NodeId, Cover)>` — the Minato–Morreale trick. An already-covered sub-case is a don't-care for the residue, which is what makes the extracted cover irreducible without any post-pass; memoizing on the interval (not the bare node) lets differently-contexted subproblems that agree on both bounds share one extraction.

4. **Correctness strategy is roundtrip + primality, not string identity.** The tests (`marker/tree.rs`, all new in this PR) walk every non-trivial 3-variable Boolean function — truth tables 1..255 — and assert (a) the OR of the extracted clauses equals the original diagram (roundtrip), (b) deleting any single literal or any single clause changes the meaning (prime + irredundant), verbatim: "Every Boolean function of three extras must roundtrip to the same diagram. The cover must also be prime and irredundant: deleting any literal or clause changes its meaning." Plus range/string/membership/darwin-release variants and the variable-ordering guard, verbatim: "Adding Python bounds must keep version decisions before extras in the diagram." The old comment "requires an exponential algorithm such as Quine-McCluskey, which is not currently implemented" is replaced by the honest non-uniqueness note, verbatim: "This function has multiple covers. Extraction chooses one in diagram variable order."

The `complexify_python_versions` fix (4th commit) is part of the contract: the old hand-rolled edge-clipping walk could insert the new version decision at a different rank than the traversal expects; the fix composes through the diagram's own `and()` op so the ordering invariant ("version decisions before extras") holds by construction. Without it the memo is unsound — a stale `(lower, upper)` entry keyed under one ordering would answer for another.

## Dedup gate — evidence

Greps run this session over `riir-refine/src/**` (corpus + everything), both vocabularies per pattern:

- `isop|shannon|quine|espresso|decision.?diagram|\bbdd\b|robdd` → **zero corpus owners**. The only "shannon" hits are our own entropy/dispersion metrics (`collapse_monitor.rs`, `dispersion.rs`, a fixture in `generations.rs`) — the information-theoretic Shannon, a different concept entirely. `entries_popcorn.rs` hits "fused Jensen-Shannon" — same class, not this.
- `dnf|sum.?of.?products|truth.?table|path.?enumerat|enumerate.{0,20}path` → zero owners. Hits: truth-table test fixtures in `score_bench/corruption*.rs` + `rescue_mine.rs` (our own G1 tables), and `lop3-int4-to-half2-dequant`'s "(0xf0 & 0xcc) | 0xaa is exactly the truth table of (a & b) | c" — a kernel bit-op, unrelated.
- Memoization kin (from the `memoiz|hash.?cons|interner|canonical.{0,30}(form|representation)|exponential` sweep): the corpus's memo family (`range-table-page-memoization` B243, `batch-local-memoization-negative-cache-tombstone` B06x, `mask-memo-hoist-to-owner-lifetime` B181, `tokenizer-segment-memo` B249, `autotuner-key-memoized-per-launch-not-rebuilt` B265) is ALL keyed on input values/geometry/text/launch keys — **no rule owns memoized structural recursion over a hash-consed DAG's node pairs**.

Nearest-kin adjudications (both in `complexity`):

- `decision-projection-before-construction` (B91): decide-from-the-raw-projection, skip canonicalizing for a predicate. THIS source is the dual: the canonical form already exists, and the rule is about how its user-facing OUTPUT is produced. Distinct axis.
- `canonical-set-guarded-rebuild` (B245): guard an idempotent normalize behind its invariant. No sharing structure, no covers, no memo-over-nodes. Distinct.

PASS for every candidate below.

## Decision-content adjudications (ranked)

| # | Pattern | Source | Grade | Adjudication |
|---|---|---|---|---|
| 1 | `isop-cover-from-decision-diagram-not-path-simplification` — produce DNF/clause output from a hash-consed decision diagram by memoized Shannon-style cover extraction (with the don't-care interval); never enumerate paths then post-simplify clauses | `isop.rs` (new) + `simplify.rs` (deleted) | **A− — GOAT** | Two-regime (below); bounded fix-space; deterministic; measured 204×–11,900× source-side with the regime trade visible at 1.5×/−5.5% on the tiny end |
| 2 | shared-cover intermediate — the OUTPUT of the exponential conversion is itself hash-consed (`Rc` branches, guard-merge on equivalent suffixes), expanded only at the leaf renderer | `isop.rs` `Cover`/`Guard` | **B — rider on 1** | Distinct decision content (share-the-output, not share-the-input) but lives or dies with 1; lands as a context_notes face |
| 3 | interval don't-care memo — cache keyed on `(lower, upper)` node pairs so covered sub-cases become don't-cares | `isop_cached` | **B− — rider on 1** | The engine of 1; a standalone rule would be thin without the cover context; context_notes face |
| 4 | variable-order preserved by composing through diagram ops | `algebra.rs` `complexify_python_versions` | **C — KILL as rule** | A representation-correctness invariant of this codebase's diagram (positive-instance class). Recorded as rule 1's regime condition: the memo is only sound under a consistent variable ordering |
| 5 | (provenance intel, not a rule) the superseded trio #22640/#22641/#22642 — bitset clause representation, cached clause diffs, shared simplification — all replaced by the algorithmic fix | PR body | — | The lesson rides rule 1's notes: **simplify-after-enumeration is the wrong layer**; three representation-level patches lost to one complexity-level fix |

## GOAT candidate — the four-criteria test (rule 1)

1. **Benchable** — YES, on the existing rust_perf Pareto validator. Fixture: an original minimal hash-consed BDD interner (~40 lines), arm A = path enumeration + quadratic clause-subset simplification (the deleted `simplify`'s SHAPE, re-authored from scratch, never pasted), arm B = memoized interval-ISOP. Sizes: conflict-dense / sparse / tiny × n {1K, 16K, 262K}. The rule class is suggestion-only (the fix-space is an algorithm replacement, not a mechanical rewrite — same class the tokio family ships).
2. **Exact semantics** — achievable with the strategy STATED UP FRONT: covers are NOT unique, so G1 is NOT byte-identity; the oracle is logical equivalence over all 2^n assignments (n ≤ 12 in-fixture, exact and cheap) plus roundtrip-through-the-representation and prime/irredundant spot asserts — the PR's own test SHAPE, re-authored as an original fixture. This is the same stated-oracle discipline as a float tolerance strategy, decided before any timing.
3. **Magnitude story** — mechanical, not "probably faster": path count through k independent shared decisions is exponential in k, and the deleted simplifier was quadratic in the (potentially exponential) clause count with per-term linear searches (its own TODO: "if we intern variables we could reduce this from a linear search to an integer `HashSet` lookup"); ISOP visits each memo slot once — work linear in the reduced diagram's node count. 32 extras: 483 ms → 46 µs.
4. **Deterministic** — YES: pure structural recursion over interned nodes, no timing, no RNG, no platform variance.

All four pass. No domain-unbenchable annotation owed.

**Regime conditions (the rule's content):**
- APPLIES: the function is already (or cheaply becomes) a reduced, ordered, hash-consed decision diagram; the consumer wants a DNF/clause rendering; the function has meaningful sharing (conflicting/overlapping conditions). Win scales with the conflict count — that IS the two-regime shape (1.5× at two clauses, 10,500× at thirty-two extras).
- ANTI-REGIMES: tiny variable counts / low sharing (path enumeration is simpler and near-equal — the 1.5×/−5.5% end); no hash-consing available (an ad-hoc AST has no shared nodes to memoize — build the interner first or the rule does not apply); consumers needing the unique canonical DNF or byte-stable output (an ISOP cover is one choice among many — "Extraction chooses one in diagram variable order"; uv accepted marker-string changes, a consumer pinning exact strings cannot); memo soundness requires a consistent variable ordering (the `complexify` contract).

## Lane-intel axis

**NO mapping.** Protocol run: league scoreboard state known (decode hold won, prefill the open lane — GPU/kernel territory); grep over `../riir-ai/.issues/` for `isop|bdd|decision.?diagram|astral|pep508|sum.?of.?products` returned exactly ONE hit — an incidental "`uv venv`" tooling mention inside the uzu install notes of ai-851 (uv as a Python-env installer, not a code surface). No workspace lane owns boolean-cover minimization, decision diagrams, or marker algebra. Nothing to file, no league/lane-intel line owed beyond this record.

## Product-coverage axis

Moderate. uv is a famous Rust repo (top-tier tooling reach), and `uv-pep508`/`uv-pep440` are published crates with real downstreams — the rule's firing surface is condition-graph-to-flat-condition-list code: feature-flag matrices, permission engines, config evaluators, version/marker resolvers (the pubgrub-class family). This is NOT tokio-class ubiquity, but the decision-diagram family would be a **corpus first** — zero owners today — and the generalized law ("derive the output from the shared canonical structure; never materialize an exponential intermediate and clean it up") travels beyond BDDs. Follow-up source pool, if the family is seeded and continued: the Mishchenko IWLS03 notes (already the in-source citation, §4), Minato's BDD papers (the variable-ordering + memo soundness contract), and the `pubgrub` crate's resolver condition handling as the nearest ecosystem consumer. Accumulation clause per the skill: pool sources read before the batch window fold into the same batch.

## Yield estimate

1 own rule + 2 count-neutral rider faces (shared-cover, interval-memo) staged; after the ~50% coordinator falsification rate: **1–2 rules land**. Nominal Batch 266, plan 270.

## Honest caveats

- Single-PR source: the verdict's scope is the `isop.rs`/`simplify.rs`/`tree.rs` surface of this one PR, not a uv repo-wide mine. uv is huge and perf-active (CodSpeed-wired CI) and could seed future verdicts, but that is a separate decision.
- Quotes are from the fetched PR `.diff` at head `551c17c` (short shas); full-sha resolution + LICENSE verification + quote re-verification at a `.raw/uv` clone are plan T0b tasks before any corpus edit.
- WIP follow-ups #22665 ("Scope intermediate marker cover nodes to conversion") and #22666 ("Fuse marker cover subtraction") are DRAFT PRs on the same stack — not inspected; if merged before mining, they may refine the rider faces.
- The fixture's BDD interner is new fixture machinery (the corpus has no diagram fixture precedent) — budget for it in the plan; it is also what makes the equivalence oracle cheap.
- The −5.5% tiny-marker regression is the rule's own anti-regime made visible; carry it in-rule as the honest cost line.
- The rule is suggestion-only: a parse-only proposer can flag the enumerate-then-simplify shape but cannot author the ISOP replacement.

## Queue slot

Plan 270 (`270_distill_batch266_uv_marker_isop.md`) / this note / the snapshot verdict row — one commit. NOT claimed; the next idle-cycle claimant opens Batch 266 from the plan.
