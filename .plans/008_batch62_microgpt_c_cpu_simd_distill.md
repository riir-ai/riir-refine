# Plan 008 — Batch 62: microGPT-C CPU SIMD distill (transposed GEMV + Newton-loses annotation)

**Source:** `vixhal-baraiya/microgpt-c` (Research 070 — the distill verdict, corrected to YES B on user challenge).
**Route:** `rust_perf` (simd + arithmetic categories) — explicitly NOT kernel_opt (CPU SIMD trap; the GPU plane-per-row twins are cited distinct-by-substrate).
**Scope:** 2 candidate rules + 1 optional context-notes enrichment. Honest yield after coordinator falsification: 1–2.

## Phase 0 — dedup re-verification (mine-time)

- [x] Re-grep `riir-clippy/src/domains/rust_perf/` under both vocabularies (source terms + operator names) for: transposed/column-major/lane GEMV; Newton/reciprocal/rsqrt refinement. Re-verify the four GOAT-candidate axes for the lead rule (benchable / semantics strategy / magnitude story / deterministic) per Research 070. **DONE 2026-08-18 (queue idle cycle):** ZERO corpus hits for `transposed|column-major|lane.*gemv|register.*accum|broadcast` AND zero for `newton|rsqrt|reciprocal|refinement` (only `-C fast-math` context mentions in `simd-chunk`); closest transcendental rule is `powi-vs-powf` (constant-exponent libm — different axis). Both rules confirmed absent; Research 070's four GOAT axes stand unchallenged.
- [x] Re-read the lead consumer shapes still live: `katgpt-types/src/simd/binary.rs` (`y[r] = vaddvq_f32(acc0)` per-row reduce), `simd/dot.rs`, `math.rs` matmul family — confirm no sibling landed a transposed variant in the interim (sibling WIP check). **DONE 2026-08-18:** binary.rs:222 `y[r] = vaddvq_f32(acc0)` STILL LIVE (the exact target shape); `dot.rs` `simd_dot_f32` (NEON/AVX2/scalar dispatch) + `simd_fma_row` "Σ weight_row[i]·input[i]" intact; `math.rs` matmul family (matmul/{,_parallel,_relu,_f16*,_sparse}) intact. The sibling heal sweep's dirty set (`depth_invariance.rs`, `simd/tests.rs`, `temporal.rs`, `ternary.rs`) does NOT include the three core files — no transposed variant landed.

## Phase 1 — entry authoring (one-shot Python generator, the proven 32c method; `json.dumps(s, ensure_ascii=False)`; `assert count==1` anchors; awk orphan-`rule_id:` pre-compile scan)

- [ ] **Rule 1 — `transposed-weight-register-gemv`** (simd): buggy = row-major matvec, per-row load + dot + horizontal reduce (`hadd` tree / `vaddvq` per row — the in-tree `binary.rs` shape); fixed = pack weights transposed once at weight-freeze, per input element broadcast x[c] + FMA into output-vector accumulators, one store per tile, zero horizontal reduction; tail tiles 16/8/4. Fixtures ORIGINAL minimal reproductions (license hygiene; never paste C source).
- [ ] **Rule 2 — `newton-refinement-vs-hardware-transcendental`** (arithmetic, honest-annotation class à la `filter-count-to-any`): buggy = full-precision Newton refinement of recip/rsqrt on modern CPU; fixed = single-instruction hardware `fdiv`/`fsqrt`; context_notes carry the regime inversion (GPU multi-cycle div → approx+refine wins; CPU single-instruction → refine loses, measured 2–4% on Ryzen 5 5600H in source) + the separate relaxed-precision-estimate axis (`rsqrtss` 12-bit / `frecpe` — valid ONLY where accuracy can relax; never suggest replacing precision-required divs).
- [ ] (optional, defer-able) Enrich `precompute-table` context_notes with the macro-granularity regime (multi-stage deterministic pipeline over bounded input domain, e.g. embed+norms+layer-0-QKV over vocab×block; rebuild-at-weight-freeze coherence condition; cycles/entry × hit-rate vs bytes/entry).

## Phase 2 — standing gates

- [ ] Fixture tests per rule (buggy fires / fixed silent / clean-span no-lint).
- [ ] `batch62_pseudocode_parses` + `batch62_rules_present` + `batch62_concepts_covered` tests (the standing trio).
- [ ] Concepts text carries the distinct-from clauses (GPU plane-per-row family for Rule 1; `regularizer-above-noise-floor` (Newton-Schulz iteration robustness ≠ transcendental refinement) for Rule 2).

## Phase 3 — bench + GOAT

- [ ] Bench specs on the Pareto validator + exact alloc-count axis: Rule 1 synthetic matvec at small/mid/large nin (16/64/4096) — expect regime-conditional verdicts; if Mixed → ship WITH annotation (`soa-hot-field` precedent), never suppress. Float sum-order changes → rel-error tolerance gate (NOT bit-identity; `distance-sq-no-sqrt` 1-ulp caveat precedent). Rule 2 Newton-chain vs hardware op — deterministic, expect hardware-wins (the rule's purpose is stopping the plausible-looking port).
- [ ] GOAT: G1 (fixtures truthful), G2 (bench verdicts + alloc counts bit-identical ×3 runs), G3 (full lib suites default + `rust_perf_bench` + `latent_retrieval` green, clippy 0 — watch the `//!` doc-markdown backtick rules: bare `hadd`/`vaddvq`/`2^n`-style tokens need backticks), G4 (no new deps).

## Phase 4 — count pins + retrieval floors

- [ ] Update ALL count-pin sites (the recurring 17+ lesson: corpus.rs header, category tables, `kernel_categories_counts_match_plan_003_table` per-category expecteds where applicable, tests' expected counts). Rust_perf corpus 65 → 66/67.
- [ ] Re-run the Issue 017 retrieval-precision regression gate; re-pin floors if the corpus growth legitimately re-ranks (tie-order slack; floors = baseline − 1).

## Phase 5 — records + commit

- [x] Research note 070 (landed at verdict time, same commit as this plan + queue line).
- [ ] Update AGENTS.md Batch 62 section + `.docs/` per the repo's doc shape; update the mining queue snapshot at `.distill/001_mining_queue_snapshot.md` (QUEUED → DONE with the measured numbers; NEVER append to `.agents/skills/riir-clippy/SKILL.md` — instructions-only since 94655ac).
- [ ] `.plans/.highwater` 7 → 8 (landed with this commit); `.research/.highwater` 69 → 70 (landed with this commit).
- [ ] Commit `feat: batch 62: microgpt-c cpu simd distill (transposed-weight-register-gemv + newton-loses annotation)` + push, on `develop`.
