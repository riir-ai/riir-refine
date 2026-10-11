# Plan 208 — Batch 208 conversion: the 10-07 staged leads (2 kernel_opt rules + 3 riders)

**Status:** COMPLETE — Batch 208 landed this session; gates all green (see the landing row in `.distill/001_mining_queue_snapshot.md` + Research 243).

Sources (all MIT):
- `ggml-org/llama.cpp` @ `7e8324f5fe` (#30100 metal MUL_MAT+ADD fusion residual-by-identity) — RULE
- `ggml-org/llama.cpp` @ `d2a79e6046` (#29171 sycl GLM MLA prefill MKL FA admission + V-as-strided-view-of-K) — RULE
- `ggml-org/llama.cpp` @ `988190680d` (#30065 metal few-row MMA remaining src0 types, per-type row thresholds) — RIDER on `fewrow-tensor-tile-decode-into-rhs-operand` (B198; the B205 a3a1c47 precedent)
- `ggml-org/llama.cpp` @ `b9acf138a1` (#29928 GLM5Next MTP headless forward, prune-vs-crop in-commit measured both sides) — RULE-or-RIDER at conversion ( Lead 4)
- `tracel-ai/cubecl` @ `9865237018` (#1770 parallel compilation; the compile-queue drain-before-first-timed-sample member) — RIDER on `profile-window-drains-queued-work-before-open` (B207)

Pins at staging: llama.cpp master → `448147d42a`; cubecl main → `2f0d8e8df5`; bevy unchanged `5cd69c1357`; wgpu → `5c9660184d`.

## Tasks

- [x] T0 Plan-first (this file; `.plans/.highwater` 207→208; dual_allocation_gate clean at allocation)
- [x] T1 `.raw/b208-llamacpp` ephemeral clone (depth-150 master) + `git show` the four shas to diff files; verify every quote from the grading row at the pins (0 corrections — leads 1+2 read in full at the diff level; the authors re-verified 3+4)
- [x] T2 Dedup re-greps at batch head — the staging row's dedup re-confirmed by both authoring subagents (zero owners, findings recorded in the gpu_78 module doc)
- [x] T3 Author rule `fusion-residual-selected-by-identity-not-negation` (kernel_opt D; #30100)
- [x] T4 Author rule `fa-fast-path-validated-exact-shape-admission-v-as-strided-view` (kernel_opt D; #29171; the in-commit F16-scores revert recorded as watch)
- [x] T5 Riders: B198 fewrow += #30065 (per-type row-threshold table); B207 profile-window-drain += #1770 (compile-queue drain + work-stealing + one-source-finalizes-once); Lead 4 DISPOSITION = own RULE `headless-draft-forward-crops-to-output-rows-cache-writes-persist` (zero owners, distinct mechanism)
- [x] T6 Extend the standing gates: `batch208_pseudocode_parses` + `batch208_concepts_carry_their_load_bearing_contracts` (2 en-route marker fixes: the in-rule quote renderings)
- [x] T7 Count pins: kernel tests.rs ×5 (112/5370/5470/5495/5508/9616) + category array (D +3) + router.rs:472 + mod.rs D header (174→177) + AGENTS.md ×2 + README ×2 (row + tree)
- [x] T8 Gates: batch208 parse+contracts 2/2; kernel_opt+latent_retrieval lib 3079/0 (1223.7 s, one full run); default lib 2472/0; clippy `-D` ×3 postures clean (3 doc mechanicals healed in the new module doc); retrieval 21/21 floors HELD (230.5 s); freeze/score-bench NOT owed (zero clippy-corpus delta, P30)
- [x] T9 Research note (`.research/.highwater` 242→243; dual_allocation_gate clean) + AGENTS.md/README + snapshot landing row (same commit)
- [x] T10 Commit `feat: batch 208: ...` + push; `.raw/b208-llamacpp` removed at close
