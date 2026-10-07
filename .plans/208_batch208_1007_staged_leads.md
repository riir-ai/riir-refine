# Plan 208 — Batch 208 conversion: the 10-07 staged leads (2 kernel_opt rules + 3 riders)

**Status:** IN FLIGHT — staged 2026-10-07 (`d11e9f0f`), conversion this session.

Sources (all MIT):
- `ggml-org/llama.cpp` @ `7e8324f5fe` (#30100 metal MUL_MAT+ADD fusion residual-by-identity) — RULE
- `ggml-org/llama.cpp` @ `d2a79e6046` (#29171 sycl GLM MLA prefill MKL FA admission + V-as-strided-view-of-K) — RULE
- `ggml-org/llama.cpp` @ `988190680d` (#30065 metal few-row MMA remaining src0 types, per-type row thresholds) — RIDER on `fewrow-tensor-tile-decode-into-rhs-operand` (B198; the B205 a3a1c47 precedent)
- `ggml-org/llama.cpp` @ `b9acf138a1` (#29928 GLM5Next MTP headless forward, prune-vs-crop in-commit measured both sides) — RULE-or-RIDER at conversion ( Lead 4)
- `tracel-ai/cubecl` @ `9865237018` (#1770 parallel compilation; the compile-queue drain-before-first-timed-sample member) — RIDER on `profile-window-drains-queued-work-before-open` (B207)

Pins at staging: llama.cpp master → `448147d42a`; cubecl main → `2f0d8e8df5`; bevy unchanged `5cd69c1357`; wgpu → `5c9660184d`.

## Tasks

- [ ] T0 Plan-first (this file; `.plans/.highwater` 207→208; dual_allocation_gate clean at allocation)
- [ ] T1 `.raw/b208-llamacpp` ephemeral clone (depth-150 master) + `git show` the four shas to diff files; verify every quote from the grading row at the pins (0 corrections or corrections folded in-rule)
- [ ] T2 Dedup re-greps at batch head (fusion/operand/residual; mla/strided-view/allowlist; fewrow; headless/draft/cache-write; drain) — the staging row's dedup re-confirmed at the working tree
- [ ] T3 Author rule `fusion-residual-selected-by-identity-not-negation` (kernel_opt D; #30100; the negated-predicate hazard, the Clef head collapse, the GGML_METAL_FUSION_DISABLE=1 diagnosis, the 27-of-28 test mode)
- [ ] T4 Author rule `fa-fast-path-validated-exact-shape-admission-v-as-strided-view` (kernel_opt D; #29171; the K/V-width gate refusal → TILE fallback, the allowlist widening, the V-view aliasing conditions + stride-exception restriction, pp8192 2.99×; the IN-COMMIT F16-scores revert recorded as the watch negative)
- [ ] T5 Riders: B198 fewrow += #30065 (per-type row-threshold table 5/4/3/2 + 16-weight-dequantizer generality + the measured table); B207 profile-window-drain += #1770 (compile-queue drains before the first timed sample; the work-stealing-over-heterogeneous-compile-costs note); Lead 4 disposition (own rule vs rider) decided at authoring time
- [ ] T6 Extend the standing gates: `batch208_pseudocode_parses` + `batch208_concepts_carry_their_load_bearing_contracts` (+ rider needles)
- [ ] T7 Count pins: kernel tests.rs totals + category array + router.rs + mod.rs B/D headers; AGENTS.md ×4; README rows; corpus 775 → 776/777 (rule count depends on Lead 4's disposition)
- [ ] T8 Gates: batch208 parse+contracts; kernel_opt+latent_retrieval lib (ONE full run); default lib; clippy `-D` ×3 postures; retrieval 21/21 floors; freeze/score-bench NOT owed (zero clippy-corpus delta, P30)
- [ ] T9 Research note (`.research/.highwater` 242→243; dual_allocation_gate clean) + AGENTS.md/README + snapshot landing row (same commit)
- [ ] T10 Commit `feat: batch 208: ...` + push; `.raw/b208-llamacpp` removed at close
