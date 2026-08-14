# Proposal 002 — Real-file span-level healing for `rust_perf` + `kernel_opt` (the `quest_grammar` fusion)

**Status:** proposed (awaiting user go-ahead to execute via Plan 005)
**Date:** 2026-08-14
**Supersedes (partially):** Proposal 001 §"Product vision: `cargo heal` CLI (design sketch)" — this proposal makes the sketch concrete + scopes the first real milestone.
**Related:** [Plan 001](../.plans/001_multi_domain_framework_refactor.md) (framework DONE), [Plan 005](../.plans/005_real_file_span_healing.md) (execution).

## TL;DR

The three heal domains today have **asymmetric real-file capability**:

| Domain | Corpus | Drafter | Pruner | **Real-file rewrite (proposer)** | Matcher (what tells you the rule applies?) |
|---|---|---|---|---|---|
| `clippy_lints` | 10 lints | ternary matvec | `RustAnalyzerPruner` | ✅ **10 `syn` AST codemods** (Stage 2 proposers) | ✅ clippy emits the lint_name |
| `rust_perf` | 46 rules | ternary matvec | `RustAnalyzerPruner` | ❌ **pseudocode template match only** | ❌ none |
| `kernel_opt` | 123 rules | ternary matvec + MoE router | `RustAnalyzerPruner` | ❌ **pseudocode template match only** | ❌ none |

The asymmetry is the gap. `clippy_lints` already proves the pattern: a per-rule `syn` visitor (`fn rewrite(file: &mut syn::File) -> bool`) detects the buggy pattern in real code + rewrites it in place. The drafter picks the rule; the proposer applies it; the pruner validates. This is **`quest_grammar` in the Rust domain** — drafter + pruner + corpus, but the corpus entries are real `syn` rewrites, not string templates.

**This proposal lifts that proposer pattern into `rust_perf` + `kernel_opt`**, adds a **shallow-reasoning matcher** (so perf/kernel can detect their own patterns — clippy isn't there to hand them a rule_id), and ships a **unified `cargo heal` CLI** that scans real `.rs` files + applies all three domains.

The result: `cargo heal --fix` gives you clippy lints + perf wins + kernel-launch/dispatch fixes on **your actual code**, modellessly. Optional model-based (L4 LoRA) stays deferred.

## The problem this solves

### Problem 1: `kernel_opt`/`rust_perf` heal pseudocode, not your code

Right now, `kernel_heal_demo` + `perf_heal_demo` draft a fix from a corpus *template* (`ref_shape` → `opt_pattern`) and validate it *parses*. They do **not** read `riir-gpu/src/gemv_q4k_batched_rmsnorm_cubecl.rs` and say "this launch has 8 redundant planes — here's the 1-plane rewrite." The 123 kernel rules + 46 perf rules are a knowledge base that nothing applies to real files. That's the gap.

### Problem 2: clippy is already ruliology — our value-add must be "clippy + shallow reasoning"

`cargo clippy --fix` already handles the common case (it IS a rule-based healer). Our `clippy_lints` domain adds value in the tail: the Stage 2 proposers do AST rewrites clippy can't (they learn from fixtures + emit caller-idiomatic fixes). The same asymmetry should hold for perf + kernel: **there's no `cargo perf --fix` and no `cargo kernel --fix`** — that entire space is ours. The shallow-reasoning matcher (ternary matvec + `syn` visitors) IS the product for those two domains.

### Problem 3: the unified CLI is a sketch, not a plan

Proposal 001 §"Product vision" describes `cargo heal` as a design sketch + flags span-level patching as the "Key technical challenge." It's actually not that hard — `syn` parses the file, the proposer rewrites in place, you write back. The real work is the **proposer coverage** (writing the visitors + rewrites) + the **matcher** (detecting which rule applies without an external diagnostic). This proposal scopes both.

## The proposed design

### Layer 0 — the proposer pattern (generalize from `clippy_lints`)

Each domain gets a `proposers/` module mirroring `clippy_lints/proposers/`:

```rust
// src/domains/rust_perf/proposers/mod.rs
pub fn propose_fix(rule_id: &str, code: &str) -> Option<String> {
    let mut file: syn::File = syn::parse_file(code).ok()?;
    let changed = match rule_id {
        "vec-with-capacity" => vec_with_capacity::rewrite(&mut file),
        "entry-vs-contains-insert" => entry_vs_contains::rewrite(&mut file),
        // ... one arm per Pareto-dominant rule (Bench 013: 34/38)
        _ => false,
    };
    if !changed { return None; }
    Some(quote_file(&file))
}
```

Each proposer is ~50-100 LOC: a `syn::visit::Visit` detector + an in-place rewrite. Same shape as `needless_return.rs`. **No trait change needed** — the `propose_fix` free function mirrors `clippy_lints` exactly; the `HealDomain` trait stays provisional (Proposal 001 caveat 1).

### Layer 1 — the shallow-reasoning matcher (the NEW piece)

For `clippy_lints`, clippy hands you the `lint_name`. For perf/kernel, **nobody hands you the rule_id** — you have to find the pattern yourself. Two complementary mechanisms:

**1a. Hard-coded `syn` visitors (precise, high-frequency rules).**
Each proposer's detector IS a visitor. For the top-N Pareto-dominant rules (Bench 013 winners), the visitor walks the AST looking for the specific anti-pattern. Examples:
- `vec-with-capacity`: find `Vec::push` in a `for`/`while` loop where the vec was built with `vec![]` / `Vec::new()` (no `with_capacity`).
- `entry-vs-contains-insert`: find `if map.contains_key(k) { map.insert(k, v) } else { ... }` → rewrite to `map.entry(k)`.
- `single-plane-sufficient-reduction` (kernel): find `CubeDim::new_1d(N)` where N > PLANE_DIM and the body is a single `plane_sum` → flag the redundant-plane pattern.

These are precise (no false positives) but require one visitor per rule.

**1b. Ternary-matvec shallow-reasoning suggester (the "clippy + shallow reasoning" tail).**
For rules WITHOUT a dedicated visitor, the `TernaryDrafter` (already shipped, generic over `E: HealEntry`) embeds the candidate code span + matches it against the corpus `ref_shape`s. The closest match (above a confidence threshold) SUGGESTS the rule. If a proposer exists for that rule, it applies; if not, the suggestion is surfaced for manual review (the CLI's `--suggest` mode). This is the **shallow reasoning**: latent-space nearest-neighbor over a BLAKE3-embedded code span, not deep semantic analysis. It's exactly `quest_grammar`'s drafter mechanism, applied to Rust spans.

The two compose: visitors handle the confident cases; the ternary matvec handles the long tail. This is the "clippy + shallow reasoning" positioning from Proposal 001 §"Shallow reasoning is the honest positioning".

### Layer 2 — the MoE router (kernel_opt only, already shipped)

`KernelExpertRouter::route(activity) -> expert_index` (Plan 003/004, `pick_domain<15, 8>`) narrows 123 rules → ~8 category candidates before the proposer runs. This composes with the matcher: the router picks the CATEGORY (memory/fusion/dispatch/...), then the visitor/ternary matvec picks the RULE within the category. 15× narrowing already measured.

### Layer 3 — the unified `cargo heal` CLI

```
cargo heal [--fix] [--domain clippy|perf|kernel|all] [--suggest] [path...]
```

- Scans `.rs` files under `path` (default: current crate `src/`).
- For each file, runs the enabled domains' matchers (Layer 1).
- For each match, applies the proposer rewrite (Layer 0).
- Validates the result parses (`RustAnalyzerPruner`).
- `--fix`: writes the changes in place. Without `--fix`: prints a diff (review mode).
- `--suggest`: for rules without a proposer, surfaces the ternary-matvec suggestion + the corpus `opt_pattern` for manual review.
- `--domain`: selects which domain(s) to run. Default `all` (all enabled feature-gated domains). **Cross-domain auto-routing stays out of scope** — the user picks the domain(s), the CLI doesn't infer "this is a perf issue not a clippy issue." (That inference is a future enhancement, not this proposal.)

This is the `cargo riir-clippy --fix` end state the user asked about — scoped, honest, modelless.

## Fusion lineage (the novelty)

This proposal is a **composition of four existing substrates**, not a new invention:

1. **`quest_grammar`** (riir-games-quest) — drafter + pruner + corpus pattern. The `TernaryDraftModel` + `SealQuestCorpus` + `ConstraintPruner` shape. Borrowed into clippy_lints already; this extends it to perf + kernel.
2. **`clippy_lints` Stage 2 proposers** (riir-clippy) — real `syn` AST rewrites (`fn rewrite(&mut syn::File) -> bool`). The proof that the pattern works on real Rust code.
3. **`katgpt-core` ternary matvec** — the shallow-reasoning embedder. `TernaryDrafter` is already generic over `E: HealEntry`; the matcher reuses it on code spans.
4. **`kernel_opt` MoE router** — `pick_domain` (Plan 558) category narrowing. Already shipped; composes with the matcher.

No new substrate invented. The novelty is the **composition** (real-file healing for perf + kernel, grounded in latent-space matching) + the **honest scope** (CubeCL macro bodies are out of scope; launch/dispatch/layout Rust code is in scope).

## Honest caveats — READ BEFORE IMPLEMENTING

1. **CubeCL `#[cube]` macro bodies are OUT OF SCOPE for kernel proposers.** `syn` cannot parse inside the `#[cube]` proc macro. Kernel proposers operate on the Rust code OUTSIDE the macro: launch params (`CubeDim::new_1d(...)`), dispatch (`CubeCount::Static(...)`), buffer setup, the `launch` function. Many of the 123 rules ARE about this outer Rust (dispatch, layout, autotune, reduction-split, single-plane, CPU-offload) — but the kernel-body rules (flash-attention tiling, linear-attn recurrence) are not proposable on real files until we have a CubeCL-aware parser. **This is a hard scope limit, not a deferral.** Documented per-proposer.

2. **Proposer coverage is voluminous.** 34 Pareto-dominant perf rules (Bench 013) + ~30 kernel-launch/dispatch rules = ~64 proposers. Each is ~50-100 LOC. Plan 005 phases this: top-5 per domain first (prove the pattern), then expand. Full coverage is multi-week.

3. **`quote_file` is NOT formatting-preserving.** The existing `quote_file` uses `ToTokens` (token-stream → string), which reformats. A `--fix` CLI that reformats the user's whole file is hostile. Plan 005 T1.3 adds `prettyplease` OR a span-preserving patch emitter (only rewrite the matched span, leave the rest byte-identical). The latter is the honest production choice; the former is the fast MVP.

4. **The ternary-matvec suggester can false-match.** Latent-space nearest-neighbor over code embeddings will sometimes suggest the wrong rule. The `--suggest` mode makes this explicit (human reviews); `--fix` only applies rules with a CONFIRMED visitor match (not ternary suggestions). This is the safety contract: visitors are precise; the matvec is advisory.

5. **Cross-domain auto-routing stays out of scope.** The CLI takes `--domain`. It does not infer domain from code shape. Revisit only with a concrete consumer + a trained classifier (that's L4 territory).

## What ships (Plan 005) vs deferred

### Ships (Plan 005 scope)
- Proposer trait surface (free fn, mirrors clippy_lints — no `HealDomain` trait change).
- Top-5 perf proposers + top-5 kernel proposers (the Pareto-dominant, high-frequency ones).
- Shallow-reasoning matcher: hard-coded visitors for the 10 proposers + ternary-matvec `--suggest` fallback for the rest.
- `cargo heal` CLI MVP: scan, match, propose, validate, `--fix` (span-preserving) + `--suggest`.
- GOAT gate: the 10 proposers round-trip on real fixture files (detect → rewrite → parse → semantics-preserving via `cargo test`).

### Deferred (separate plans)
- Full proposer coverage (all 34 perf + ~30 kernel rules) — Plan 005 proves the pattern with 10; expansion is mechanical follow-up.
- CubeCL macro-body proposers — blocked on a CubeCL-aware parser (research, not impl).
- Cross-domain auto-routing — out of scope per caveat 5.
- L4 LoRA — deferred across all domains (Plan 001 Phase 4).
- `--watch` mode (Proposal 001 sketch) — deferred.

## GOAT gate (Plan 005 §Phase 6)

- **G1 (correctness):** each proposer round-trips on a fixture file — detects the pattern, rewrites, output parses, `cargo test` passes (semantics preserved).
- **G2 (perf):** matcher + proposer latency < 10ms per file (syn parse + visitor walk + rewrite). Measured on a 1000-LOC fixture.
- **G3 (no-regression):** all existing tests pass (the framework + clippy + corpus tests); the new CLI adds tests, breaks none.
- **G4 (alloc-free hot path):** the visitor walk reuses a scratch buffer; no per-file allocation in steady state (mirror the drafter's alloc-free contract).
- **G5 (real-file value):** at least ONE real fix lands in a sibling repo (riir-ai or riir-gpu) — a perf or kernel rule detects a real anti-pattern + the proposer rewrites it + the change compiles + tests pass. This is the "does it actually help" gate.

## References

- [Proposal 001](001_multi_domain_modelless_code_healer.md) — the multi-domain meta-architecture + the `cargo heal` sketch this concretizes.
- [Plan 001](../.plans/001_multi_domain_framework_refactor.md) — the framework refactor (DONE).
- [Plan 005](../.plans/005_real_file_span_healing.md) — the execution plan for this proposal.
- `src/domains/clippy_lints/proposers/` — the 10 proposer codemods this generalizes.
- `katgpt-core` `TernaryDrafter` — the shallow-reasoning embedder.
- [Bench 013](../.benchmarks/013_*.md) — the perf-rule Pareto verdict (34/38 dominant) that prioritizes proposer coverage.
- `quest_grammar` (riir-games-quest) — the drafter+pruner+corpus precedent.
