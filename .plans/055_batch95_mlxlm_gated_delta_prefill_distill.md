# Plan 055 — Batch 95: mlx-lm gated_delta.py GDN prefill kernel distill

**Status:** COMPLETE (2026-09-01) — 3 rules landed, corpus 411 → 414; record Research 111.

Queue lead from the Bench 831 follow-up source read (2026-09-01): distill the
mlx-lm `gated_delta.py` GDN prefill kernel family (the 190-tok/s shape's GDN
half) into the riir-clippy `kernel_opt` corpus. Estimated yield ≈ 2-3 rules +
1 rider:
1. simd-group row-packing geometry for narrow-row register scans (Dk=128 packs
   8 value rows per SIMD-group, 4 lanes/row, 32 state elems/lane; 2-level
   in-lane + 2-level cross-lane reduction split)
2. whole-T zero-barrier single-kernel scan shape (sequential-in-T inside one
   kernel, state register-resident per lane, zero smem/barriers — the
   barrier-elimination complement of B35's barrier-count rules)
3. RIDER: bitwise-contract-by-comparator-kernel test discipline (pin an
   optimized kernel bit-identical to an explicitly-written comparator
   independent of library lowering — the Issue-715 class, external
   confirmation)

Source: `raw.githubusercontent.com/ml-explore/mlx-lm` main
`mlx_lm/models/gated_delta.py` (Apache-2.0; read in full 09-01; mine-time
re-read mandatory — moving main).

## Tasks

- [x] T1 — Mine-time source re-read: fetch gated_delta.py, verify the
      snapshot's structural claims (fused whole-T kernel, register-resident
      state, simd_sum / shuffle_xor butterflies, packed Dk specialization,
      2-level reduction split) against the CURRENT file (moving main —
      snapshot claims may have drifted). — VERIFIED; one correction: the
      split is 3-level in-lane + 2-level cross-lane.
- [x] T2 — Dedup grep BEFORE proposing: B22 warp-shuffle, B28 plane-WG +
      row-owned, B25 tree-depth, B79 scan, B12 chunk family, B57 barrier +
      bitexact-gate, B23 fast-path, B38 env-flag — all checked; 3 genuinely
      new decision contents confirmed; 8 candidates rejected with reasons.
- [x] T3 — Two parallel subagent lanes read the source + propose rules;
      both lanes INDEPENDENTLY CONVERGED on the same three decision
      contents; coordinator re-verified every quoted line byte-accurate.
- [x] T4 — Author corpus entries (entries_gpu_09.rs, E+G+D) +
      `batch95_pseudocode_parses` gate (cfg(not(kernel_verify)) per B70).
- [x] T5 — GOAT: batch95 gate green; kernel_opt lib 1235/0; default lib
      1001/0; clippy 0 --all-targets both states; retrieval floors 14/14
      HELD (no re-pins); count pins walked (tests.rs 69-70/5362/5364/5375 +
      expected-array D85/E26/G26 + stale "current 400" comment + router.rs
      461 + mod.rs D/E/G). Full CI guard not run — the 3 affected states +
      floors + pins all validated; recorded honestly in Research 111.
- [x] T6 — Research note `.research/111_*.md` (born-complete, Status:
      RECORD), AGENTS.md batch entry + corpus-count bump + research
      highwater 111, `.docs/11_domains/kernel_opt.md` Total + batch row,
      queue snapshot one-line update, commit + push.
