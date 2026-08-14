# Plan 005 — Real-file span-level healing for `rust_perf` + `kernel_opt`

**Proposal:** [002](../.proposals/002_real_file_span_healing.md)
**Branch:** `develop` (per global rule — no feature branches)
**Status:** proposed (awaiting go-ahead)

## Goal

Lift the `clippy_lints` proposer pattern (real `syn` AST rewrites) into `rust_perf` + `kernel_opt`, add a shallow-reasoning matcher (so perf/kernel detect their own patterns), and ship a unified `cargo heal` CLI that scans real `.rs` files + applies all three domains. This is the `quest_grammar` fusion (Proposal 002): the corpus entries become real `syn` rewrites, not string templates.

**This is a product-capability plan, not a corpus-expansion plan.** The GOAT gate (G5) requires at least one real fix to land in a sibling repo.

## Substrate-first gate (pre-flight, MANDATORY)

Before implementing, verify the substrate exists:
- [x] `clippy_lints/proposers/` — 10 `syn` AST codemods (`fn rewrite(&mut syn::File) -> bool`). The pattern to generalize.
- [x] `propose_fix(rule_id, code) -> Option<String>` — the proposer dispatch shape. Already clippy-specific; perf/kernel get parallel free fns.
- [x] `TernaryDrafter` (generic over `E: HealEntry`) — the shallow-reasoning embedder. Already shipped.
- [x] `RustAnalyzerPruner` — `syn`-based parse validation. Already shipped.
- [x] `KernelExpertRouter::route()` (`pick_domain<15, 8>`) — MoE category narrowing. Already shipped.
- [x] `quote_file` (ToTokens → string) — the MVP emitter (formatting-non-preserving; T1.3 upgrades to span-preserving).
- [x] Corpus: 46 perf rules + 123 kernel rules — the knowledge base the matchers consult.

No new substrate invented. The proposer pattern + matcher compose existing pieces.

## Phase 1 — Proposer surface + span-preserving emitter

Generalize the proposer dispatch + fix the formatting-preservation gap (Proposal 002 caveat 3).

**Status: not started.**

### Tasks

- [ ] **T1.1** Extract the proposer dispatch shape into a domain-agnostic helper. Each domain ships `pub fn propose_fix(rule_id, code) -> Option<String>` (mirrors `clippy_lints`). No `HealDomain` trait change — the free fn is the contract.
- [ ] **T1.2** Add `src/draft/span_patch.rs` — a span-preserving patch emitter. Instead of re-quoting the whole file (which reformats), emit a `[Span { start_line, start_col, end_line, end_col } → replacement_text]` patch. The CLI applies only the matched spans; the rest of the file stays byte-identical. This is the production choice (Proposal 002 caveat 3).
- [ ] **T1.3** MVP fallback: if span extraction is too hard for a given proposer, fall back to `prettyplease` (whole-file reformat) behind a `--format` flag. Default is span-preserving; `--format` opts into reformat. Document the tradeoff.
- [ ] **T1.4** `src/draft/matcher.rs` — the `Matcher` trait: `fn detect(file: &syn::File) -> Vec<Match>` where `Match { rule_id, span, confidence }`. Hard-coded visitors implement this; the ternary-matvec suggester is a separate `Suggester` (advisory, not `Matcher`).

### Phase 1 exit criteria
- Proposer dispatch generalized; span-preserving emitter ships (or `prettyplease` fallback); `Matcher` trait defined. No proposers yet — just the surface.

## Phase 2 — Top-5 perf proposers (prove the pattern)

Pick the 5 highest-impact, most-frequent perf rules (from Bench 013 Pareto winners). Write real `syn` visitors + rewrites for each.

**Status: not started.**

### Candidate rules (from Bench 013 — all Pareto-dominant)

| Rule | Speedup | Why top-5 |
|---|---|---|
| `vec-with-capacity` | 1.1-4.3× | highest frequency (every `vec![]` in a loop) |
| `entry-vs-contains-insert` | 1.5× | high frequency (HashMap pattern) |
| `static-str` | 42× | easy to detect (`String::from("lit")` → `"lit"`) |
| `bool-to-bitpack` | 7-11× | easy to detect (`Vec<bool>` → bitflags) |
| `fxhashmap` | 3.3-5.8× | easy to detect (`HashMap` → `FxHashMap` in hot path) |

### Tasks

- [ ] **T2.1** `src/domains/rust_perf/proposers/vec_with_capacity.rs` — visitor: find `Vec::new()`/`vec![]` followed by `push` in a loop; rewrite: insert `with_capacity(N)` if N is statically known, else `with_capacity_hint`.
- [ ] **T2.2** `entry_vs_contains_insert.rs` — visitor: `if m.contains_key(k) { m.insert(..) } else { m.insert(..) }` → `m.entry(k)`.
- [ ] **T2.3** `static_str.rs` — visitor: `String::from("literal")` / `"lit".to_string()` in non-generic context → `"literal"` (borrow).
- [ ] **T2.4** `bool_to_bitpack.rs` — visitor: `Vec<bool>` / `[bool; N]` in hot path → flag for bitpack (advisory — semantics change, so `--suggest` only).
- [ ] **T2.5** `fxhashmap.rs` — visitor: `HashMap<K,V>` in a hot-path fn (annotated or inferred) → `FxHashMap`.
- [ ] **T2.6** `proposers/mod.rs` — `propose_fix(rule_id, code)` dispatch (mirror clippy_lints).
- [ ] **T2.7** Fixture tests: each proposer detects + rewrites a real `.rs` fixture; output parses; `cargo test` passes.

### Phase 2 exit criteria
- 5 perf proposers ship, each round-trips on a fixture. The pattern is proven. `rust_perf` now heals real files (for these 5 rules).

## Phase 3 — Top-5 kernel proposers (launch/dispatch/layout only)

Pick 5 kernel rules that operate on Rust OUTSIDE the `#[cube]` macro (Proposal 002 caveat 1). These are the rules about launch params, dispatch, buffer setup.

**Status: not started.**

### Candidate rules (all operate on outer Rust, not macro body)

| Rule | Category | Why proposable on real files |
|---|---|---|
| `single-plane-sufficient-reduction` | E | detect `CubeDim::new_1d(N)` where N > 32 + body is single `plane_sum` |
| `offload-cpu-bottleneck-to-gpu-kernel` | A | detect CPU transpose/permute of large buffers → suggest GPU kernel |
| `block-count-clamp` | E | detect unbounded `CubeCount` without `.min(65535)` |
| `cube-count-bounded-batch-pack` | L | detect per-batch launch without `batches_per_run` packing |
| `anchor-shape-bucketing` | L | detect autotune key without `anchor(shape, ..)` |

### Tasks

- [ ] **T3.1** `src/domains/kernel_opt/proposers/single_plane_reduction.rs` — visitor for `CubeDim::new_1d(N)` + `plane_sum` body.
- [ ] **T3.2** `cpu_offload.rs` — visitor for CPU transpose/permute patterns (`for r { for c { dst[c*R+r] = src[r*C+c] } }`).
- [ ] **T3.3** `block_count_clamp.rs` — visitor for `CubeCount::Static(..)` without clamp.
- [ ] **T3.4** `batch_pack.rs` + `anchor_bucket.rs` — autotune-key visitors.
- [ ] **T3.5** `proposers/mod.rs` — `propose_fix(rule_id, code)` dispatch.
- [ ] **T3.6** Fixture tests on real riir-gpu snippets (extracted, not the whole file — avoid the `#[cube]` parse failure).

### Phase 3 exit criteria
- 5 kernel proposers ship, each round-trips on a fixture. Kernel-domain real-file healing works for launch/dispatch/layout rules. Kernel-body rules (flash-attn, linear-attn recurrence) are honestly out of scope (caveat 1).

## Phase 4 — Shallow-reasoning matcher (ternary matvec `--suggest`)

For rules WITHOUT a dedicated proposer, the `TernaryDrafter` embeds the code span + matches against corpus `ref_shape`s. Closest match above threshold → suggestion.

**Status: not started.**

### Tasks

- [ ] **T4.1** `src/draft/suggester.rs` — `Suggester` struct wrapping `TernaryDrafter`. Input: a code span. Output: `Vec<(rule_id, confidence)>` ranked.
- [ ] **T4.2** Embedding: BLAKE3 the span + ternary-encode (reuse the existing `embedder`). Match against the corpus `ref_shape` embeddings (precomputed at boot).
- [ ] **T4.3** Threshold tuning: what confidence counts as "suggestable"? Default conservative (0.6); the `--suggest` mode surfaces anything above it with the corpus `opt_pattern` for manual review.
- [ ] **T4.4** The compose contract: visitors (Phase 2/3) are PRECISE (auto-apply in `--fix`); suggester is ADVISORY (only in `--suggest`). Never auto-apply a suggestion.

### Phase 4 exit criteria
- Suggester ships; `--suggest` mode surfaces ternary-matvec matches for rules without proposers. The two-layer matcher (precise visitors + advisory suggester) is the "clippy + shallow reasoning" product.

## Phase 5 — Unified `cargo heal` CLI

**Status: not started.**

### Tasks

- [ ] **T5.1** `src/bin/heal.rs` (or `cargo-heal` — decide naming). Args: `--fix`, `--domain <clippy|perf|kernel|all>`, `--suggest`, `[path...]`.
- [ ] **T5.2** File scanner: walk `path` (default `src/`), parse each `.rs` with `syn`, run enabled domains' `Matcher::detect(file)`.
- [ ] **T5.3** Apply loop: for each `Match`, call `propose_fix(rule_id, code)`. If span-preserving patch (T1.2), apply patch; else `prettyplease` reformat (T1.3, `--format`).
- [ ] **T5.4** Validate: `RustAnalyzerPruner` on the result. If invalid, skip + warn.
- [ ] **T5.5** Output: `--fix` writes in place; no `--fix` prints a unified diff (review mode). `--suggest` prints the suggestions (rule_id + confidence + opt_pattern snippet).
- [ ] **T5.6** Domain gating: `--domain perf` requires `--features rust_perf`; `--domain kernel` requires `--features kernel_heal`. `--domain all` runs all enabled feature-gated domains.

### Phase 5 exit criteria
- `cargo heal --fix --domain perf src/` applies the 5 perf proposers to real files, span-preserving, validated. `cargo heal --suggest` surfaces ternary matches. The CLI exists.

## Phase 6 — GOAT gate

**Status: not started.**

### Tasks

- [ ] **T6.1 (G1 correctness)** Each of the 10 proposers (5 perf + 5 kernel) round-trips on a fixture: detect → rewrite → parse → `cargo test` passes (semantics preserved).
- [ ] **T6.2 (G2 perf)** Matcher + proposer latency < 10ms/file on a 1000-LOC fixture. Record in `.benchmarks/`.
- [ ] **T6.3 (G3 no-regression)** All existing tests pass (framework + clippy + corpus). New CLI tests added.
- [ ] **T6.4 (G4 alloc-free hot path)** Visitor walk reuses scratch; no per-file alloc in steady state.
- [ ] **T6.5 (G5 real-file value)** **At least ONE real fix lands in a sibling repo** (riir-ai or riir-gpu): a perf or kernel rule detects a real anti-pattern in real code, the proposer rewrites it, the change compiles + tests pass. This is the load-bearing gate — "does it actually help."

### Phase 6 exit criteria
- G1-G5 PASS. `.benchmarks/016_real_file_heal_goat.md` records the measurement. The CLI is production-usable for the 10 shipped rules.

## Out of scope (per Proposal 002)

- Full proposer coverage (all 34 perf + ~30 kernel) — Plan 005 proves the pattern with 10; expansion is mechanical follow-up plans.
- CubeCL `#[cube]` macro-body proposers — blocked on a CubeCL-aware parser.
- Cross-domain auto-routing — user picks `--domain`.
- L4 LoRA — deferred across all domains.
- `--watch` mode — deferred.

## Risks

1. **Span-preserving patch is harder than it looks.** `syn` spans are byte-offsets; re-emitting only the changed span while keeping surrounding bytes intact needs care. Mitigation: `prettyplease` fallback (T1.3) ships first; span-preserving is the upgrade.
2. **Visitor false positives.** A `vec-with-capacity` visitor might match a vec that's intentionally un-sized. Mitigation: visitors are conservative (only rewrite when the size is statically known); uncertain cases go to `--suggest`.
3. **CubeCL parse failure.** A real riir-gpu file with `#[cube]` macros won't parse with vanilla `syn`. Mitigation: the scanner skips files that fail to parse (with a warning); proposers run only on parseable files. Kernel proposers are tested on EXTRACTED snippets, not whole kernel files.
4. **G5 might fail.** If no real anti-pattern exists in the sibling repos for the 10 shipped rules, G5 can't pass. Mitigation: the rule selection (Phase 2/3 candidates) is chosen BECAUSE the patterns are common; if G5 fails, expand the rule set or target a different repo.

## Numbering discipline

- Proposal 002: `riir-clippy/.proposals/002_real_file_span_healing.md`
- Plan 005: this file.
- Highwater: `.plans/.highwater` = `5`, `.proposals/.highwater` = `2` (written back after this plan lands).
