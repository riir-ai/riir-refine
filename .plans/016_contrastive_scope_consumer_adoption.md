# Plan 016 — Contrastive-scope consumer adoption (Bench 669 T5 reopen)

**Date:** 2026-08-20
**Source:** katgpt-rs [Bench 669](../../katgpt-rs/.benchmarks/669_contrastive_scope_poc_goat.md) T5 verdict — "a shipped opt-in primitive awaiting its consumer". The issue's own promotion rule: *"if G-gates pass AND a consumer adopts (riir-clippy L4 2D gate or riir-ai engram), promote per GOAT discipline; else record negative result and close."*
**Primitive:** `katgpt_core::contrastive_scope` (Issue 674 / Research 493 / arXiv:2608.13545) — `ContrastiveScoreTable`, `scope_score D(x)`, `ScopeGate` (κ haircut / θ decline), `oos_probe_battery`.

## The gap the adoption closes

The Issue 030 relevance gate on the heal path is **rule-coverage only** (1D
provenance): `rule_in_corpus || source != Memory`. It cannot catch the
complementary failure — an **out-of-scope INPUT** (non-Rust code, garbage, a
mis-routed span from another tool) paired with an **in-corpus rule id**. In
that cell the drafter serves the corpus template, it parses, the rule is in
corpus → **served confidently**. Feeding Python to the clippy healer returns a
Rust fix with a straight face; on the L4 path the same garbage burns a ~48 s
GPU call.

The 2D gate: (rule coverage) × (input scope). Rule coverage stays as shipped
(Issue 030). Input scope is the new axis — `D(x)` under a table built from the
domain corpus (in) vs a canonical non-Rust corpus (out):

| rule coverage | input scope | behavior |
|---|---|---|
| any | **out** (`D > θ`) | **decline immediately** — never serve, never consult L4 (NEW) |
| any | in | exactly today's path, `evolution_score × sigmoid(−κ·D)` (== score bit-identically for strongly in-scope docs) |
| in corpus | in | serve (unchanged) |
| not in corpus | in | Issue 030 path (decline-after-L4) |

**Honest scope claim (the Issue 020 boundary):** this gates **Rust-vs-not-Rust
+ garbage**, NOT cross-domain Rust. Lexical domain routing between the healer's
own Rust domains was measured at a ~70% ceiling and killed for gating (Issue
020); the out-corpus here is deliberately non-Rust so the discrimination is
near-trivial and the claim is small.

## Tasks

- [x] T1 `src/scope_model.rs` — `ScopeModel` (vocab + `ContrastiveScoreTable` + `ScopeGate`), zero-alloc tokenizer (`&str` slices → ids, unseen skipped ≈ symmetric smoothing), `verdict_str`, `OUT_OF_SCOPE_DOCS` (12 non-Rust docs: Python/JS/WGSL/GLSL/JSON/YAML/TOML/SQL/Go/HTML/Markdown/prose), corpus-generic `build_for_corpus<E: HealEntry>`.
- [x] T2 κ/θ re-pin (the Bench 669 caveat: "consumers re-pin per their benches") — measure D over the clippy seed corpus (in) + held-out non-Rust probes (out); pin θ inside the measured gap; pin κ so `sigmoid(−κ·|D_worst_in|)` saturates to EXACTLY `1.0f32` (bit-identity precondition, argument ≥ ~16.6).
- [x] T3 `HealPipeline` wiring behind `contrastive_scope` — `with_scope_model`, scratch reuse (`mem::take` dance, G4), gate at heal() top (decline → `None`, L4 never consulted), haircut applied to `evolution_score` on every serve path (Stage2/drafter/memory/L4).
- [x] T4 `tests/scope_gate_adoption.rs` — G1 falsifiable A/B (without model: OOS served = the demonstrated bug; with model: declined); G3 bit-identity over the FULL corpus (`to_bits` on evolution_score, same fixed_code/source); L4-savings (CountingL4 probe, 0 calls on garbage); held-out probe battery (Report-the-Floor discipline); κ/θ separation pins.
- [x] T5 G-gates + record — G1 defend-wrong delta, G2 overhead (< 10 µs/heal), G3 no-regression (default + all-features lib suites + bit-identity), G4 alloc (scope check 0 allocs; heal-with-model − heal-without = 0). Bench 040 + AGENTS.md.
- [x] T6 katgpt-rs promotion per the T5 rule — consumer adopted + gates pass ⇒ add `contrastive_scope` to katgpt-core `default`, update Bench 669 verdict + the feature-def comment.

## Non-goals

- No CLI `--fix`/`--suggest` gating (those paths operate on real files already known Rust).
- No cross-domain Rust discrimination (Issue 020 killed it; claim stays Rust-vs-not).
- No papaya/live-update table (immutable `Arc`-share is the POC design; live update is a future consumer concern).
- No new deps (katgpt-core already non-optional; the primitive is pure math + BLAKE3).
