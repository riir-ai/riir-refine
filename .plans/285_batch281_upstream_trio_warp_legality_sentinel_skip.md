# Plan 285 — Batch 281: the upstream llama.cpp trio (warp-legality + sentinel-skip + barrier-uniformity)

**Status:** IN PROGRESS — claim `31530c184` (pushed 2026-10-25 ~08:1x +07, expiry 2026-10-27 ~08:1x +07)

## Sources (the claimed lead pack, diff source = upstream PR patch URLs, fetched this session)

- **#30837** `60341eb74d482a2c516634ba9b6ec9c53a935f3e` — "ggm-cuda: Enforce DP4A MMQ
  tile height multiple of warp size" (merged 2026-10-23): one line in
  `ggml/src/ggml-cuda/mmq.cuh` — `static_assert(I % warp_size == 0, "DP4A MMQ tile
  height must be a multiple of the physical warp size");` where
  `warp_size = ggml_cuda_get_physical_warp_size()`.
- **#30836** `c295a3d9dcbb41ae4599b4b000bcd5235fec6511` — "HIP/CUDA: Fix reduction in
  mmf getting tile widths not devisiable by 64 in mul_mat_id on wave64 devices"
  (merged 2026-10-20): `ggml_cuda_mul_mat_id` + `_needs_sync` pass the DEVICE's
  `warp_size` into `ggml_cuda_should_use_mmf` where they passed the compile-time
  `WARP_SIZE`. VERIFY-OUT-OF-LANE adjudicated: behavior-neutral on NVIDIA (32==32),
  the fix is wave64 (RDNA/HIP); as a LAW it is portable — dispatch geometry keys on
  the physical warp size. **Faces of ONE rule with #30837** (static contract +
  dispatch-predicate consumer).
- **#26631** 8-patch series, head `3332eff1`, merged 2026-10-24 — "allow MUL_MAT_ID
  −1 index to skip the computation". TWO rules:
  - The op-level skip contract (id == −1 ⇒ slot skipped + result row zeroed;
    compact-set consumers re-bounded; dedicated zero pass for expert-unowned rows;
    whole-block/warp uniform exit). Category H.
  - Patch 4/8's barrier face ("ensure all threads reach workgroup barriers in
    mul_mat_id_vec", cherry-pick of d8a4b9ef): the pre-barrier early return
    (`if (!found) { return; }`) replaced by found-gated stores. Category E.
  - **Patch 7/8 EXCLUDES CUDA/ROCm/MUSA** (test-side `TODO: [TAG_MUL_MAT_ID_SKIP]`
    NOT_SUPPORTED marking) — the fail-loud adoption-boundary face rides the H rule.
- Riders (count-neutral, only if conversion supports them): the Vulkan
  clamp-then-mask face (`expert_id = expert_skip ? 0 : uint(expert); // clamp, so
  that the offsets below stay in range`) as a rider on
  `clamp-tile-indices-to-sentinel-discard-at-write` (B17, dedup-verified kin).

## Dedup verdicts (grep pass this session)

- physical warp / tile height / multiple-of-warp: ZERO owners. `wave64` appears only
  in passing (gpu_15 partition rule's constants-recompute note). CLEAN.
- `gather-signed-index-wrap-semantics` owns the WRAP semantic (−1 → last row, still
  computes) — the SKIP-and-ZERO semantic is a distinct sibling; name as kin.
- `moe-duplicate-id-compact-prepass-static-bound-tile-dispatch` owns the compact
  prepass STRUCTURE; the op-level skip CONTRACT is adjacent, not duplicate; name.
- `broadcast-uniform-skip-reduce` (post-reduction uniformity) and
  `batch-composition-invariant-grid-early-return` (per-row work extents) — different
  classes; name as kin.
- workgroup-barrier participation / pre-barrier early return: ZERO owners.

## Tasks

- [x] Plan first (this file), `.plans/.highwater` 284→285, push before authoring.
- [ ] `entries_gpu_127.rs`: 3 own rules (L `physical-warp-size-keys-tile-legality-and-dispatch`,
      H `sentinel-id-skip-shrinks-work-set-zero-unowned-rows`,
      E `all-threads-reach-workgroup-barrier-no-pre-barrier-return`), fixtures
      original Rust reproductions (no upstream string literals in fixtures), quotes
      verbatim from the fetched patch text, GREP EVIDENCE + KINSHIP in-rule.
- [ ] Rider: the Vulkan clamp-then-mask face on `clamp-tile-indices-to-sentinel-discard-at-write`.
- [ ] `mod.rs`: mod decl + extend + 3 category-row prepends + 3 header comments
      (E 87→88, H 40→41, L 28→29).
- [ ] `tests.rs`: count pins 966→969 (14 sites incl. the category-sum asserts) +
      `batch281_rules_and_riders_landed` + `batch281_pseudocode_parses` gates.
- [ ] `router.rs` Plan-003 assert 966→969 (line 472).
- [ ] AGENTS.md ×2 + README.md row + `.docs/11_domains/kernel_opt.md` ledger paragraph.
- [ ] `.research/318_batch281_upstream_trio.md` + `.research/.highwater` 317→318.
- [ ] `.distill/001_mining_queue_snapshot.md` landing row (claim discharged).
- [ ] Gates: batch281 gate first-run; kernel_opt lib (floor 3226 + new);
      default lib (floor 2729); clippy −D ×3 postures; retrieval floors re-measured
      (kernel_opt+latent_retrieval ~1 min + default `retrieval_` ~2 min, the
      Issue-017/170 batch law). freeze/score-bench NOT owed (kernel_opt-only, P30);
      binary rebuild NOT owed (kernel_opt outside the release set).
- [ ] Commit `feat: batch 281: ...` + push.

## Notes

- Quotes verify against the machine-fetched `.patch` text (verbatim by construction);
  no local clone needed (upstream ggml-org/llama.cpp, MIT).
- Fixture discipline: original minimal Rust reproductions; compile standalone; NO
  upstream string literals; each fixture carries #[test]s that RUN.
- Expected count: 966 → 969 (+3).
