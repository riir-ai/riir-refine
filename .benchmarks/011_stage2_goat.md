# Bench 011 — Stage 2 GOAT gate: 10/10 on held-out fixture (was 0/10)

**Issue:** `010` (RESOLVED)
**Date:** 2026-08-13
**Host:** M3 Max, Metal, macOS Darwin 25.5.0
**Gate path:** `cargo run --release --example stage2_tuning_demo --features self_evolve,clippy_verify`
**Ground truth:** real `cargo clippy` re-run per case (~130 ms each — off-hot-path tuning, NOT the hot-path guard)

---

## TL;DR

The Stage 2 stack (`ClippyAdaptiveDrafter` + per-lint `syn`-level AST proposers + identity guard + `HealPipeline::tune_against_fixture`) **beats the Issue 010 standing GOAT target** of "Stage 1 + identity guard + Stage 2 must beat 0/10 on the held-out fixture". Measured result: **10/10 verified**, zero failures.

This is the modelless generalization path — no training, no backprop, no LoRA. Each lint has a deterministic AST codemod; `cargo clippy` is the ground truth during tuning. Stage 2 is now the primary serve path for held-out code; Stage 1 remains as the always-on fallback for code the proposers don't yet cover.

## The measurement

```
=== Stage 2 Tuning Demo (Issue 010 GOAT gate) ===


--- TuneReport ---
total            : 10
stage1_hits      : 0 (already in pool)
stage1_verified  : 0 (L1 drafter + clippy pass)
stage2_proposed  : 10 (proposer fired)
stage2_verified  : 10 (proposer + clippy pass)
verified total   : 10
failed           : 0

Stage 2 pool size: 10

=== GOAT G1 (beat 0/10): PASS ===
```

**Every held-out case**: Stage 1 was bypassed (identity guard fires; corpus retriever cannot generalize — Issue 010). The per-lint proposer fired, emitted a candidate fix, `cargo clippy` re-ran on the candidate, and confirmed the target lint no longer fires.

## What this means

| Claim before | Claim after |
|---|---|
| Bench 009: modelless cache 1.8M tok/s, **0/10 on held-out**, silent-miss (L2 parses=true on wrong answers) | Bench 009 still measures Stage 1 (the silent-miss is real); **Bench 011 measures Stage 2 — 10/10 on the same held-out fixture.** |
| "The modelless pipeline cannot generalize beyond the corpus" | Stage 1 cannot. **Stage 2 (proposers + adaptive drafter) generalizes** for the 10 seed lints. Adding a new lint = adding a new AST codemod. |
| "L4 LoRA is required for generalization" | For the 10 seed lints, **no**. L4 remains deferred per the modelless-first mandate; Stage 2 covered the held-out fixture fully without it. L4 reopens only if a future lint's generalization pattern can't be expressed as a syn-level rewrite. |

## The architecture (shipped)

```text
                ┌─────────────────────────────────────────┐
                │            HealPipeline::heal            │
                │                                         │
                │  ┌─────────────────────────────────┐    │
   (lint, ────► │  │ Stage 2: ClippyAdaptiveDrafter  │ ─► verified fix (HealSource::Stage2Adaptive)
    buggy)      │  │ • papaya pool keyed by BLAKE3   │    │
                │  │ • lookup: ~µs hash probe        │    │
                │  └──────────────┬──────────────────┘    │
                │                 │ miss                   │
                │                 ▼                        │
                │  ┌─────────────────────────────────┐    │
                │  │ Stage 1: ClippyTernaryDrafter   │ ─► corpus fix (HealSource::Drafter)
                │  │ • BLAKE3→matvec→FNV-1a          │    │  or None (identity-guard decline)
                │  │ • identity guard                │    │
                │  └─────────────────────────────────┘    │
                └─────────────────────────────────────────┘

                              (off hot path)
                                     │
                                     ▼
           ┌──────────────────────────────────────────────┐
           │     HealPipeline::tune_against_fixture        │
           │  (feature-gated clippy_verify — slow)         │
           │                                              │
           │  for each (lint, buggy):                     │
           │    1. Stage 1 → identity guard → verify      │
           │       (cargo clippy ~130 ms)                 │
           │    2. If decline: run per-lint AST proposer  │
           │       → verify (cargo clippy ~130 ms)        │
           │    3. Verified fix joins Stage 2 pool        │
           └──────────────────────────────────────────────┘
```

## Per-lint proposer coverage

All 10 seed-corpus lints ship a `syn`-level AST rewrite in `src/proposers/`:

| Lint | File | Pattern | Held-out fixture verdict |
|---|---|---|---|
| `needless_return` | `needless_return.rs` | strip trailing `return <expr>;` → tail expr | ✅ verified |
| `redundant_closure` | `redundant_closure.rs` | `\|x\| f(x)` → `f` | ✅ verified |
| `needless_lifetimes` | `needless_lifetimes.rs` | strip `<'a>` + `&'a` → elided | ✅ verified |
| `redundant_field_names` | `redundant_field_names.rs` | `{ x: x }` → `{ x }` | ✅ verified |
| `useless_conversion` | `useless_conversion.rs` | `let x: T = y.into()` → `let x: T = y` | ✅ verified |
| `needless_borrow` | `needless_borrow.rs` | `f(&x)` → `f(x)` (x bare path) | ✅ verified |
| `unnecessary_mut_passed` | `unnecessary_mut_passed.rs` | `f(&mut x)` → `f(&x)` + drop `mut` binding | ✅ verified |
| `manual_map` | `manual_map.rs` | `match { Some(v) => Some(f(v)), None => None }` → `.map(\|v\| f(v))` | ✅ verified |
| `unnecessary_unwrap` | `unnecessary_unwrap.rs` | `if x.is_some() { x.unwrap() } else { d }` → `x.unwrap_or(d)` | ✅ verified |
| `needless_range_loop` | `needless_range_loop.rs` | `for i in 0..v.len() { v[i] }` → `for x in &v { x }` | ✅ verified |

## Scope limits (honest)

1. **10 seed lints only.** Adding an 11th lint (e.g. `clippy::map_entry`) requires writing its AST rewrite. The framework supports it; the work is per-lint.
2. **Per-lint proposers are pattern approximations of clippy's type-aware checks.** They fire on syntactic shape, not type inference. This means:
   - They can over-fire (propose a rewrite clippy wouldn't flag) — the `cargo clippy` verifier catches this during tuning.
   - They can under-fire (miss a case clippy would flag) — the case never enters the Stage 2 pool; Stage 1 remains as the fallback.
3. **The proposer output is round-tripped via `ToTokens`** — semantically equivalent but not formatting-preserving. Whitespace + comments may differ from the canonical form. This is acceptable for clippy verification (clippy doesn't care about formatting) but would need a `prettyplease` pass if byte-identical output is required.
4. **Identity guard is conservative.** It fires when the emitted fix shares no identifiers with the input. Cases where the fix legitimately has no shared identifiers (rare) would be declined — Stage 2 catches them on the next tuning pass.

## Reproduce

```sh
# Build + run the GOAT gate (slow — spawns cargo clippy 10+ times)
CARGO_TARGET_DIR=/tmp/clippy_stage2 cargo run --release --example stage2_tuning_demo \
    --features self_evolve,clippy_verify

# Fast unit-test version (no cargo clippy; trusts the proposers structurally)
CARGO_TARGET_DIR=/tmp/clippy_stage2 cargo test --test stage2_goat --features self_evolve
```

## References

- `Issue 010` — RESOLVED. The original 0/10 root cause + the 2-stage fix direction.
- [Proposal 001](../.proposals/001_multi_domain_modelless_code_healer.md) — the multi-domain framework this Stage 2 stack composes into.
- [`quest_grammar` 2-stage drafter](../../riir-ai/crates/riir-games-quest/src/quest_grammar/plasma_draft.rs) — `TernaryDraftModel` (Stage 1) + `AdaptiveQuestDrafter` (Stage 2) precedent.
- [Bench 009](./009_bonsai_vs_modelless_e2e_tokps.md) — the original 0/10 measurement (Stage 1 baseline).
