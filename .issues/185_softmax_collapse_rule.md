# Issue 185: `softmax-collapse` rule family — catch the f32 Softmax Collapse paste before it ships (arXiv:2610.31642)

**Status:** In progress — 2026-10-27. T1–T4 landed (`17965377`); the zero-hit re-sweep waits on riir-train 651 T1 + the three follow-up sites below.
**Claimed:** zed-m3-i185, 1793094772, expires 2026-10-29T09:52:52Z

## Why

The `grad = softmax − onehot` paste computes the target gradient as `p_t − 1`, which rounds to **exactly 0.0** in f32 once the logit margin passes the cliff 24·ln2 ≈ 16.6. Every non-target gradient stays positive, so the row stops summing to zero; under AdamW this is the paper's Slingshot instability. riir-train Issue 645 fixed 7 sites, yet a census days later found ~20 more across 6 repos, written three different ways. clippy cannot see it because the code is valid. A rule is the only thing that stops the next hand-written training loop from bringing it back.

Real before/after pairs, which become the fixture corpus: riir-rethink `2850291`, katgpt-rs `4ba82db2f`, riir-ai `8981315af`, riir-reflex `6ba895d`, riir-refine `1b426b53`, plus riir-train Issue 651 (T1 commits; T2 `141900de`, T3 `80b104bc`).

## Tasks

- [x] **T1 — `sigmoid-minus-one` (MECHANICAL fix).** `σ(z) - 1.0` and `σ(z) - if d == label { 1.0 } else { 0.0 }` (σ = any `*sigmoid*` call) → `-σ(-z)`, or the split `if d == label { -σ(-z) } else { σ(z) }`. An exact identity at full f32 range, safe in the same way as the `nan_safe_cmp` `total_cmp` rewrite. Comment guard and macro-arg refusal per house rules; `--verify` compile gate. — `17965377`: `src/domains/sec/proposers/sigmoid_minus_one.rs`, in `FIXABLE_SEC`. Also takes `σ(z) - f32::from(d == label)` and the swapped-arm `if`. Gates TIGHTER than the task text: σ is a name ALLOWLIST (`sigmoid` + only `exact/stable/f32/f64/scalar/safe/precise/logistic/std/ref` segments), not "any `*sigmoid*`", because `fast_`/`hard_`/LUT/repo-prefixed sigmoids may be approximations that break `σ(-z) = 1 - σ(z)`. The rule refuses inside comparisons or `.abs()` (those are saturation checks), allows the split form only in a bare context (let init, assignment RHS, parens), skips tests (premise tests must keep the naive form), and needs a single-arg free call. 16 unit tests, including the exact riir-reflex `6ba895d` repair and idempotence.
- [x] **T2 — `softmax-onehot-collapse` (DETECT-only, suggest).** In one fn: a row normalized by `/ sum` / `*= inv` / `* inv_sum` (or a `softmax*` call result), then `x[target] -= 1.0`, `p - if v == t { 1.0 } else { 0.0 }`, `p - f32::from(i == t)`, or `(p - onehot)`. Suggest the excluded-LSE form (`grad[t] = −σ(m − z_t)`). Not mechanical: shapes vary (SIMD, temperature, weighted one-hot, per-repo primitive). — `17965377`: `src/domains/sec/proposers/softmax_collapse.rs::detect`. Evidence is in walk order and per innermost fn. Rows come from `softmax*` producers (not `log_softmax` and not loss/entropy/grad producers), `/= sum`-class divisors, `&mut X` scale-by-sum calls, aliases (`let d = probs`), closure/`for` elements, and row copies (`dl[..] = p[v] / l`). The sweep added one shape: the MEAN-scaled one-hot `dl[t] -= 1.0 / l` (riir-ai `binary_kernel_poc`). Recall limits: weighted one-hot `-= w`, rows normalized in a helper, and un-sum-named divisors.
- [x] **T3 — `neg-ln-softmax-prob` (DETECT-only, low severity).** `-p[t].ln()` / `-(p[t].max(eps)).ln()` where `p` is a softmax row: the loss reads 0 past the cliff; suggest `softplus(m − z_t)`. — `17965377`: same detector; it also covers `loss -= p[t].ln()` and the `+ eps` clamp.
- [x] **T4 — fixtures + zero-hit sweep.** Fire on every "before" spelling, stay silent on every "after" and on the primitives themselves (`ce_collapse_free_into` and friends). A workspace sweep test/report must reach 0 hits once riir-train 651 T1 lands (Bench/report doc). — `17965377`: synthetic before/after fixtures for all six source commits (rethink, katgpt-rs SIMD, riir-ai ×2, refine choice_scorer, reflex). The primitive `ce_collapse_free_into`, the fixed callers and `ce_one_minus_p_target` stay silent, even when renamed. Sweep: `examples/softmax_collapse_sweep.rs` (read-only, tracked `*.rs` only, prints `EMPTY-WALK`/`unparsed` rather than a clean zero). Results below.
- [ ] **T4b — zero-hit re-sweep.** Re-run `cargo run --release --no-default-features --features sec --example softmax_collapse_sweep` once riir-train 651 T1 lands and the follow-ups below are fixed. Target: 0/0/0 outside deliberate incumbent arms.
- [-] Bug 2 (two separately computed probability vectors subtracted) is OUT of scope: it is a cross-function dataflow question, too noisy for a syntactic rule. riir-train 651 T3 is the instrument for it.

## Domain decision (2026-10-27)

**`sec`, new APPEND-ONLY family `SecFamily::Numerics`.** This is a float-correctness defect in valid code, which is the same shape as `nan-unsafe-sort-comparator`: a float defect, conservative gates, one mechanical rewrite plus suggest-only siblings. That rule is already a `sec` rule (`PanicSafety`), so the `sec` proposer framework is the existing substrate (`FIXABLE_SEC`, `collect_edits`, the `--domain sec` fix loop, the boundary gate and the eval fixtures). The alternatives do not fit. `rust_perf` is about cycles, which is the wrong axis. `clippy_lints` heals clippy's own findings, and clippy cannot see this defect. A new domain would need a `DomainId`, a router lane and a matcher overlay for three rules. BOUNDARY.md is unaffected: no new dependency and no new crate.

## T4 sweep — 2026-10-27 (worktree state; riir-train includes the in-flight 651 lane's uncommitted edits)

`cargo run --release --no-default-features --features sec --example softmax_collapse_sweep` scanned 29 repos (`riir-*`, `katgpt-rs`, `seal-*`). Per-repo counts below; every other repo is 0/0/0.

| repo | files | unparsed | sigmoid-minus-one | softmax-onehot-collapse | neg-ln-softmax-prob | verdict |
|---|---|---|---|---|---|---|
| riir-ai | 3076 | 0 | 0 | 1 | 2 | **TP ×3**: real remaining bugs, follow-up below |
| riir-rethink | 63 | 14 | 0 | 0 | 1 | **TP ×1** (low severity). The 14 unparsed files are `bin_hunks/*.rs` include-fragments, not files; a grep confirms they contain no softmax/`.ln()`/sigmoid |
| riir-train | 1322 | 0 | 0 | 20 | 1 | recorded only (in-flight lane, not edited) |
| seal-online-site | 0 | — | — | — | — | EMPTY-WALK (no tracked `.rs`) |

There are **0 false positives outside riir-train**, so no gate needed tightening for FPs. The only gate change from the sweep was a recall addition: the mean-scaled one-hot. Already-repaired sites (rethink `2850291`, katgpt-rs `4ba82db2f`, riir-ai `8981315af`, reflex `6ba895d`, refine `1b426b53`) read 0.

**Follow-up — real remaining bugs (not fixed here, per scope):**

- riir-ai `crates/riir-poc/src/binary_kernel_poc.rs:628`: `dlogits[i * vocab + clean[i]] -= 1.0 / l as f32` after `dlogits[..] = probs[v] / l`. This is the mean-scaled `p − onehot`, so the target slot collapses. Lines `:605` and `:624` have `-= probs[clean[i]].max(1e-12).ln()`, the clamped loss.
- riir-rethink `examples/instinct_teacher_ft.rs:395`: `loss += -(p[gold].ln()) as f64` over `softmax32`. This only affects the loss report: the gradient on the next line goes through the fixed `ce_dlogits`.
- riir-rethink `examples/instinct_typed_head_trainer.rs:365-367`: a gold-only BCE where `fwd_opt` is a closure that inlines `1.0 / (1.0 + (-s).exp())`, then `dy = p - f32::from(opt == gi)`. The positive slot collapses at s > 16.6. The manual census found this one; **no rule can reach it**, because σ is inlined behind a closure call (a cross-call recall limit, the same class as the out-of-scope Bug 2).

**riir-train hits (recorded, not edited):** `riir-bench-algo/src/train.rs:219,224` (Brier-on-softmax `p - onehot`; the shape is TP, but the repair advice differs for a squared loss); `riir-train-engine/examples/bonsai_clippy_l4_rlvr_train.rs:1976,2149`, `bonsai_clippy_l4_sft_train.rs:2249`, `bonsai_lora_accuracy_parity_arm_c.rs:3130`, `plan338_delta_signature.rs:323`, `weak_probe_train.rs:458`; `riir-train-engine/src/deltanet/lm_head_lora_train.rs:299`, `elbo_boundary_trainers.rs:240` (neg-ln), `kimi_k3_gpu_backward.rs:527`, `kimi_k3_ouro.rs:990`; `riir-train-engine/tests/bench_asft_lora.rs:275,304`, `bench_nitp_layer_sweep.rs:314`, `bench_peira_convergence.rs:359,384`, `tests/common/mod.rs:172`; `riir-train-gpu/src/backward/mod.rs:1967`; `riir-train-gpu/tests/plan389_phase0_decoupling_ablation.rs:546`, `plan433_phase1_warmup_battle.rs:867`. Some of these may be 645's deliberate incumbent arms behind `RIIR_CE_COLLAPSE_FREE=0`; 651 owns that adjudication.

## Verification (2026-10-27, `CARGO_TARGET_DIR=/tmp/r185`)

- `cargo test --release --lib --features sec`: 2998/0. The `domains::sec` + `draft::retrieval_eval` slice under `--features sec,latent_retrieval`: 271/0.
- `cargo test --release --features sec --test sec_boundary_gate`: 2/2. It first went RED because the new fixtures called undefined `sigmoid`/`softmax`, so the probe crate stopped compiling and `unwrap_used` vanished. The fixtures are now self-contained. `indexing_slicing` fires on the T2/T3 fixtures with no machine fix, which is additive.
- `cargo clippy --no-default-features --features sec --lib --tests --example softmax_collapse_sweep`, and `--features sec,latent_retrieval --lib --tests`: zero warnings.
- Router annotation (sec arm, `sec,latent_retrieval`): measured 69/172 (40.1%), all 3 new spans routed Sec. Re-pinned 62/169 → 64/172.
- End-to-end CLI on a /tmp fixture crate (`cargo-refine` built with `sec,latent_retrieval,kat_service,fix_verify`). The dry run listed the three rules among the suggestions. `--fix --write --verify --domain sec` then kept 2 edits on lines 14 and 20 and reverted 0. The `#[cfg(test)]` premise line was untouched, and the crate's collapse test went FAILED (pre-fix) → ok (post-fix).
- Score-bench and the gen2/gen3 freeze are not owed: `score_bench` / `generations.rs` read the clippy/perf corpora only, and the sec corpus is not in them.
