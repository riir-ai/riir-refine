# Scope model — the input-scope gate at the heal seam

> The 2D complement of the Issue 030 relevance gate: rule coverage × **input
> scope**. An out-of-scope input (`D > θ` under a two-corpus contrastive
> table) declines the whole heal — never served, never escalated to L4.
> Source: katgpt-core's `contrastive_scope` primitive (Issue 674 / Research
> 493 / arXiv:2608.13545); consumer adoption = Bench 040 / Plan 016.

The Issue 030 gate asks "does the corpus cover this RULE?" — a provenance
question. It cannot catch the complementary failure: an out-of-scope INPUT
(non-Rust code, config files, prose) paired with an in-corpus rule id. With
memory seeded (the production state), memory retrieval returns the nearest
trajectory for any query; the rule is covered so the relevance gate passes;
the fix parses → the pipeline serves a Rust fix for a C++ input with a
straight face, and on the L4 path the same garbage burns a ~48 s GPU call.

## The model

`ScopeModel` (`src/scope_model.rs`) is built once (cold path) from:

- **in-corpus**: every entry's `buggy_code` — the input shape the pipeline is
  FOR (`ScopeModel::build_for_corpus`).
- **out-corpus**: `OUT_OF_SCOPE_DOCS` — 18 canonical non-Rust documents
  (Python/JS/WGSL/GLSL/JSON/YAML/TOML/SQL/Go/HTML/Markdown/prose/CSS/C++/
  Java/Ruby/shell/XML) covering the garbage space a daemon client or API
  caller could realistically send.

Reads are zero-alloc: a `&str`-slice lexical tokenizer (maximal identifier /
punctuation runs) → vocab-id lookup (unseen tokens skipped — symmetric
smoothing makes them ≈ 0) → the primitive's `scope_score` (Naive-Bayes
log-odds document sum).

## The gate

| rule coverage | input scope | behavior |
|---|---|---|
| any | **out** (`D > θ`) | decline immediately — never serve, never consult L4 |
| any | in | today's path; `evolution_score × sigmoid(−κ·D)` |

The haircut applies at the two confident serve sites (Stage 2 + the final
modelless serve). L4's neutral `0.5` and the non-parsing handback are
deliberately untouched — there is no confidence there to discount.

## κ / θ pinning (consumer-specific)

Pinned from the measured D landscape over the 40-fixture clippy seed corpus
vs 8 held-out non-Rust probes: corpus ∈ [−183.2, −7.6], probes ∈ [+24.4,
+131.1]. **θ = 0** (decline any out-lean; neutral all-unseen docs pass —
absence of evidence is not decline). **κ = 8.5** (re-pinned from 4.5 on
2026-10-26, riir-refine Issue 172: the worst fixture `redundant_static_lifetimes`
drifted to D = −2.08, and the pin's min/max accumulators were swapped so it
never saw it) so the worst (least-negative) fixture saturates f32 `sigmoid` to **exactly `1.0`** — making in-distribution
healing bit-identical with the model installed (`to_bits`-pinned by test).
The separation-pinning test fails loudly if the corpus grows a shorter
fixture or a probe family drifts into the gap.

## Honest limitations

1. **Rust-vs-not-Rust only.** Cross-domain Rust discrimination is out of
   scope — Issue 020 / Bench 025 measured the lexical ceiling at ~70% and
   killed gating.
2. **Uncovered language families pass** (e.g. Lua scores neutral — its tokens
   are unseen on both sides). Fix: extend `OUT_OF_SCOPE_DOCS`.
3. **Short mixed snippets are the gray zone** — a 9-token Python snippet
   sharing `return`/`x`/parens with Rust accumulates sub-threshold evidence
   and passes (pinned as the documented gray-zone test).

## See also

- [Bench 040](../../.benchmarks/040_contrastive_scope_adoption.md) — the GOAT record
- [katgpt-rs Bench 669](../../../katgpt-rs/.benchmarks/669_contrastive_scope_poc_goat.md) — the primitive
- [heal_pipeline.md](heal_pipeline.md) — the seam this gates
