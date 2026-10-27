# Issue 185: `softmax-collapse` rule family — catch the f32 Softmax Collapse paste before it ships (arXiv:2610.31642)

**Status:** Open — 2026-10-27.
**Claimed:** zed-m3-i185, 1793094772, expires 2026-10-29T09:52:52Z

## Why

The `grad = softmax − onehot` paste computes the target gradient as `p_t − 1`, which rounds to **exactly 0.0** in f32 once the logit margin passes the cliff 24·ln2 ≈ 16.6. Every non-target gradient stays positive, so the row stops summing to zero; under AdamW this is the paper's Slingshot instability. riir-train Issue 645 fixed 7 sites, yet a census days later found ~20 more across 6 repos, written three different ways. clippy cannot see it because the code is valid. A rule is the only thing that stops the next hand-written training loop from bringing it back.

Real before/after pairs, which become the fixture corpus: riir-rethink `2850291`, katgpt-rs `4ba82db2f`, riir-ai `8981315af`, riir-reflex `6ba895d`, riir-refine `1b426b53`, plus riir-train Issue 651 (T1 commits; T2 `141900de`, T3 `80b104bc`).

## Tasks

- [ ] **T1 — `sigmoid-minus-one` (MECHANICAL fix).** `σ(z) - 1.0` and `σ(z) - if d == label { 1.0 } else { 0.0 }` (σ = any `*sigmoid*` call) → `-σ(-z)`, or the split `if d == label { -σ(-z) } else { σ(z) }`. An exact identity at full f32 range, safe in the same way as the `nan_safe_cmp` `total_cmp` rewrite. Comment guard and macro-arg refusal per house rules; `--verify` compile gate.
- [ ] **T2 — `softmax-onehot-collapse` (DETECT-only, suggest).** In one fn: a row normalized by `/ sum` / `*= inv` / `* inv_sum` (or a `softmax*` call result), then `x[target] -= 1.0`, `p - if v == t { 1.0 } else { 0.0 }`, `p - f32::from(i == t)`, or `(p - onehot)`. Suggest the excluded-LSE form (`grad[t] = −σ(m − z_t)`). Not mechanical: shapes vary (SIMD, temperature, weighted one-hot, per-repo primitive).
- [ ] **T3 — `neg-ln-softmax-prob` (DETECT-only, low severity).** `-p[t].ln()` / `-(p[t].max(eps)).ln()` where `p` is a softmax row: the loss reads 0 past the cliff; suggest `softplus(m − z_t)`.
- [ ] **T4 — fixtures + zero-hit sweep.** Fire on every "before" spelling, stay silent on every "after" and on the primitives themselves (`ce_collapse_free_into` and friends). A workspace sweep test/report must reach 0 hits once riir-train 651 T1 lands (Bench/report doc).
- [-] Bug 2 (two separately computed probability vectors subtracted) is OUT of scope: it is a cross-function dataflow question, too noisy for a syntactic rule. riir-train 651 T3 is the instrument for it.
