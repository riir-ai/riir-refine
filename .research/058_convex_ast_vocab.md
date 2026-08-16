# Research 058 — ConvexTok over AST node-kind sequences (Plan 005 T13.1)

**Date:** 2026-08-16
**Plan:** [005](../.plans/005_real_file_span_healing.md) Phase 13 (research-gated)
**Status:** DISTILLED — pending owner decision (design filed; the T13.3 corpus
experiment NOT run — reopen conditions §5).

Phase 13 is explicitly research-only ("this is a research question, not a
plan"), and T13.1 — *file the design of the AST-node colouring scheme* — is its
deliverable. This is that note.

## 1. The idea

ConvexTok solves tokenisation as a linear program over a graph of candidate
merges, then rounds to an integer vocabulary with a certified optimality gap.
katgpt-tokenizer ships the whole pipeline — `GraphBuilder` (`convex_graph.rs`),
`ConvexSolver` (`convex_solver.rs`), `Rounder` (`convex_rounding.rs`),
`Certifier` (`convex_certify.rs`) — all verified present 2026-08-16.

The proposal: run it not over *bytes* but over **AST node-kind sequences**, to
learn a globally-optimal multi-node vocabulary for code structure. If recurring
AST motifs compress well, that vocabulary is a candidate representation for
retrieval, for corpus indexing, and for the proposer dispatch.

## 2. The open question the plan names: what is a "colour"?

ConvexTok's graph needs a notion of token identity. Three candidate encodings,
in increasing fidelity and cost:

| scheme | colour | vocabulary size | loses |
|---|---|---:|---|
| **A. kind-only** | `syn` node discriminant (`ExprCall`, `ExprIf`, `Stmt`) | ~120 | arity, shape — `f(a)` ≡ `f(a,b,c)` |
| **B. kind + arity** | `(discriminant, child_count)` | ~600 | child *kinds* — `f(x)` ≡ `f(1)` |
| **C. kind + child-kind-shape** | `(discriminant, [child discriminants])` | ~10⁴ (long tail) | little; risks a vocabulary larger than the corpus |

**Recommendation: B.** A is too lossy to distinguish the very patterns this
repo cares about — `identity_op` (`x * 1`) and `double_parens` (`((x))`) are
both "an expression inside an expression" under A. C's tail is a real hazard:
with only ~40 corpus rules the LP would find mostly singleton colours, and a
vocabulary that never repeats compresses nothing. B keeps arity, which is what
separates the operator shapes T3.5.2 found to be load-bearing.

## 3. What the encoding would look like

Walk each corpus `buggy_code` with a `syn::visit::Visit` impl, emitting one
colour per visited node in pre-order. A rule becomes a sequence; the corpus
becomes the LP's input multiset. `max_len=8` (the plan's suggestion) bounds
motif length, which is generous — the corpus patterns are 1–5 lines.

## 4. What it would be measured against

The honest gate is **not** compression ratio in isolation. Plan 005's own
history is emphatic on this point: T3.5.4 produced a spectacular +25pp that was
an artifact, and Bench 030 catalogues seven tasks whose gates measured
something the mechanism could not move. T13.4's ≥2× compression bar has the
same weakness — compression is a property of the vocabulary, not of anything
the healer does.

If this is ever run, the gate should be **downstream**: does an AST-motif
vocabulary improve retrieval top-1 over the operator-aware BM25 tokenizer
(T3.5.2)? That is the metric the product actually has.

## 5. Why the experiment is NOT run here

The downstream gate is currently **unreachable by arithmetic**, for the same
reason that closed T10.6 and T14.5: retrieval top-1 on the held-out multi-lint
set is **96.0%** (24/25 correct), leaving **4.0pp** of total headroom. A new
tokenisation scheme would be competing for 1 span.

The pre-condition for running T13.2–T13.4 is therefore **not** effort — it is
headroom. Reopen when either:

1. the corpus grows enough that retrieval precision drops materially (restoring
   room a better representation could win), or
2. a consumer appears that needs AST motifs for something *other* than
   retrieval ranking — proposer dispatch and corpus dedup are the plausible
   candidates, and both would need their own gate.

## 6. Honest assessment of novelty

The plan calls this "THE SUPER-GOAT" and "genuinely novel — no code-domain
analog exists". The novelty claim survives scrutiny: grep confirms the LP
pipeline is only wired to byte-level tokenisation today. But novelty is not
value. The measured situation is that this repo's retrieval was fixed by
**restoring discarded punctuation to BM25** (+16.6pp) and by **running the
proposers' own visitors as evidence** (+12.0pp) — two changes that used signals
already present. A globally-optimal AST vocabulary is a more sophisticated
answer to a question that is currently 96% answered.
