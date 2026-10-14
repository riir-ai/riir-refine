# Plan 237 — Batch 235: the B234 deferred rider pair (count-neutral)

**Status:** COMPLETE 2026-10-14 — landed `c7461f0a` (claimed `d5233464`, M3 idle loop; the claim discharged by the .distill landing row); T12 hygiene-closed 2026-10-15 (4090 idle loop — a duplicated unticked T12 row removed; the landing row, commit and push were already verifiably in). Plan 236 T5's `- [-]` discharged by this plan.

Batch 235 = the two count-neutral riders Plan 236 deferred (recorded Research 266 §"The integration shape (riders)"):

- **Rider (a)** — the mma.cuh ldmatrix I==8 `.x2.b16` variants + the `((threadIdx.x / t.I) * (t.J / 2)) % t.J` wrap fix (ggml-org/llama.cpp #29353 @ merge `62df1815` / head `dde6a4d93`, MIT) → rider on `ldmatrix-pair-x2-to-x4-adjacency` (entries_gpu_37.rs) — the width-matched loader law; the pair rule merges x2→x4 in the OPPOSITE direction (composition, not duplicate).
- **Rider (b)** — the ggml.h `when n_tokens > K only slot 0 is guaranteed; backends may leave slots 1..K-1 unwritten` API-contract line + the two overflow test-row REMOVALS in test-backend-ops.cpp → second UB-contract rider on B224 `moe-duplicate-id-compact-prepass-static-bound-tile-dispatch` (entries_gpu_88.rs) — the remove-vs-disable disposition split (B233's face disabled-behind-literal; this face removes because the contract narrows the guarantee itself).

## Tasks

- [x] T0: quotes re-verified at the merged pin — the #29353 .diff fetched fresh THIS session (`62df1815`/`dde6a4d93`); all needle lines verbatim (the `static_assert(I == 8 || I == 16, "bad tile width")`, both lane-map changes, both PTX x2 spellings, the ggml.h slot-0 line + its preceding slot-semantics comment, the removed `/*K=*/3` + `/*K=*/4` rows + their `// overflow:` framing comment).
- [x] T1: homes + dedup adjudication — Research 266's own staging followed: rider (a) → the ldmatrix pair rule (kinship note names it; the width choice composes with pair fusion); rider (b) → B224 (Research 266: "B224's UB-contract rider family"). Dedup greps: no corpus rule owns width-matched ldmatrix issue counts or the state-slot guarantee (the mma-fragment-layout rule owns the fragment identity; `ldmatrix-mma-layout-loads` owns layout-at-load; neither owns loader WIDTH selection; the GDN B234 rules own the kernel internals, not the API contract).
- [x] T2: author rider (a) into entries_gpu_37.rs `ldmatrix-pair-x2-to-x4-adjacency` gpu_concept (RIDER block appended inside the string literal, escaped quotes).
- [x] T3: author rider (b) into entries_gpu_88.rs B224 gpu_concept (second RIDER block, same discipline).
- [x] T4: `batch235_riders_parse_and_needles_landed` in tests.rs — rule lookup + verbatim needles for both riders; count-neutral assertion (corpus len pins untouched). FIRST-RUN PASS.
- [x] T5: G2 — kernel_opt lib 3108/0/1i + default lib 2659/0 (isolated target /tmp/riir-refine-b235).
- [x] T6: G3 — clippy `-D warnings` clean at default AND all-features (the block-v0.1.6 future-incompat note is the pre-existing third-party notice, not a warning).
- [x] T7: retrieval floors — 21 passed / 0 failed (307.15 s) — HELD, no re-pin.
- [x] T8: freeze/score-bench — NOT owed (zero clippy-corpus delta, P30; B229/231/233 rider precedent).
- [x] T9: counts — count-neutral: corpus stands 812; NO pin edits anywhere.
- [x] T10: research note 267 (`.research/.highwater` 266→267, dual_allocation_gate clean at allocation) + plan 236 T14/T15 hygiene tick (the c83c323f landing row IS in .distill, the commit IS pushed).
- [x] T11: docs — AGENTS.md kernel_opt bullet gains the Batch 235 discharge row (the B234 "DEFERRED" clause resolved in-place); `.docs/11_domains/kernel_opt.md` batch list row appended.
- [x] T12: snapshot landing row (.distill — the claim discharged) + commit `feat: batch 235: ...` + push.

## Notes

- Count discipline: riders add NO rules — counts stand at 812; per-category tables untouched.
- The B233 escape lesson applies to both corpus edits: any python-side append needs the escape pass in ONE pass; here the edits go through the file editor with pre-escaped quotes in the replacement text.
- Sources stay the merged pin; no re-cloning needed (the .diff is the full surface for both faces — mma.cuh + ggml.h + test-backend-ops.cpp hunks read verbatim this session).
