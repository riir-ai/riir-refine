# Plan 229 — Batch 227: the lithos-metal wave-2 pack (2 rules + 2 riders)

**Status:** COMPLETE — landed 2026-10-13 ~10:1x +07 (m3-max idle loop; corpus 801→803, 2 rules + 2 riders; gates ALL GREEN — see Research 257)

## Goal

Convert the 08:2x envelope's staged lithos wave-2 pool (4 PRs @ repo pin `5485ad6bac`,
Apache-2.0, all diffs read in full this session from `.raw/lithos-b227`) into:

- **NEW A** — `coop-tensor-layout-measured-per-chip-family` (#35, `7781e93`): a SIMD-group
  cooperative tensor's register layout is a PER-CHIP-FAMILY contract (Apple9 lays 8×8
  matrices: a thread holds reduction slots kr + 8·jj of row pairs nc + e + 8·s, measured with
  coop_layout) — a tile written for Apple10's layout is WRONG on Apple9, silently, and the
  autotuner keeping the tile off is the SYMPTOM not the fix (every T>1 projection stuck on the
  shader GEMV at 8.80×). The repair ports the LAYOUT (COOP_A9: permuted slots over twice the
  rows / half the columns, epilogue reads row pairs, the row GEMVs read four slots 8 apart,
  TK≥128 for 4-bit, compile-time #error guards on the Apple10-only paths) — not a disable.
  Measured (60-core M3 Ultra): NVFP4 8 rows 8.80×→2.50× a T=1 pass (32 rows 5.81×); 27B prefill
  of 43 tokens 11.8→0.65 s. → `entries_gpu_90.rs`.
- **NEW D** — `exact-pass-rows-take-their-sub-pass-chunking` (#36, `087db24` + `182d016` +
  `4effa4a`): a shape-dependent scheduling parameter (chunk width from the pass's context,
  pick_chunk) silently breaks an N-exact reproduction promise — a 512-row pass summed a row's
  keys in a different order than the 128-row pass promised. The repair: EXACT_SUB — every row
  takes the chunking of the sub-pass it belongs to (core: per-sub-pass blocks + the
  `key >= qpos0 + s0` append guard; merge: per-row ctx_t), the boundary law (an exact chunk is a
  MULTIPLE of EXACT_SUB — a 192-row chunk puts rows 128-191 in a sub-pass of the wrong context;
  CLI + session refuse loud), the fold-chain admission (clone the variant only when the whole
  decode+merge chain is chunked: STEP_STATE=1, no DRAFT/LM_MODE/STEAL/FIXED_CHUNK/ADAPTIVE_CHUNK),
  the serving posture (M3 Ultra 512-row exact passes, 2846-token prompt first token 9.2→8.6 s;
  1024 rows measured no better). → `entries_gpu_90.rs`.
- **RIDER on B223** `attention-partition-two-knob-shared-partial-workspace`
  (`entries_gpu_86.rs`) — #37 (`5842639`): the ARENA-ADMISSION face. The cross-layer sharing
  law is defeated by the arena's free-slot policy: a small value parked in a freed
  attention-partials slot made every later layer open another slot of that size — Qwen3.8-27B
  at 32K context held FIVE copies of the partial workspace (18.7 GB at 128 rows, 34.9 GB at 256,
  past the working set = GPU page fault). The bound: a value takes a free slot at most twice its
  size (`s['size'] <= 2 * size`) — 5.8 and 9.1 GB. Sharing policy (the host rule) and slot
  admission (the rider) are one law's two halves.
- **RIDER on B207** `minifloat-normal-code-shift-bias-diff-decode` (`entries_gpu_77.rs`) —
  #38 (`258ef6e`): the decode family's FIFTH member, the LANE-PAIRING face. The FP8 decode GEMV
  is ALU-bound on Apple9: pairing two codes per half2 (even bytes → one half, odd bytes → the
  other, `((even & 0x007F007F) << 7) | ((even & 0x00800080) << 8)`) decodes every finite code to
  the same value at half the decode instructions — M3 Ultra T=1 FP8 projections 514-562 →
  672-675 GB/s, M4 Pro +2-4%, M5 Max +3-6%; the default flips FP8_DECODE 0 → 2 (0 and 1 remain);
  the exhaustive finite-codes test extends to mode 2.

## Tasks

- [x] Claim row pushed (`ef344d96`).
- [x] Plan-first (this file; `.plans/.highwater` 228→229; dual_allocation_gate run at allocation).
- [x] Fresh `.raw` clone (`.raw/lithos-b227`, HEAD == `5485ad6`) — walk complete, all 4 PR diffs
      read in full this session.
- [x] Dedup grep BEFORE proposing: #37 slot-size-class (ZERO owners); #36 exact-chunking (kin:
      `batch-composition-invariant-grid-early-return` gpu_14 — order-independence BY
      CONSTRUCTION, the inverse face; `wide-verify-warp-specialized-scores-in-registers-pv`
      gpu_48 — order-keeping in one kernel; #36 repairs a DERIVED-PARAMETER break, NEW);
      #35 coop-layout (ZERO owners — "cooperative tensor" appears only as consumer prose);
      #38 half2-pairing → RIDER on B207 (the family's fifth member).
- [x] Rule authoring: `entries_gpu_90.rs` ×2 (A + D) + the two riders (entries_gpu_86.rs B223,
      entries_gpu_77.rs B207).
- [x] Wiring: mod.rs `include!` + KERNEL_CATEGORIES (A 163→164, D 187→188) + count pins
      (×5 sites: build corpus, Plan-003 sum, round-trip, batch116 running pin, router expert
      total — 801→803) + `batch227_rules_parse_and_needles_landed` (2 rules, parse both halves +
      verbatim needles) + rider needles in the same test.
- [x] GOAT: G1 parse/needles · G2 kernel_opt lib 2948/0 + default lib 2505/0 · G3 clippy -D
      ×4 postures + retrieval floors HELD (kernel 100.0%, blended 91.2/94.7/95.3) ·
      freeze/score-bench NOT owed (zero clippy-corpus delta, P30).
- [x] Research note `.research/257_batch227_lithos_wave2.md` (`.research/.highwater` 256→257) +
      AGENTS.md kernel_opt line (Batch 227 entry, corpus 803) + `.docs/11_domains/kernel_opt.md`
      table cell + snapshot (BATCH-LANDED head row; the claim discharged).
- [x] Commit `feat: batch 227: ...` + push; rm `.raw/lithos-b227`.

## Non-goals

- #39 (kernel-choices sidecars) + #48 (mlx-4bit M5) stay WATCH per the 08:2x grading.
- The ollama-serve series (#43) + the CI/runner infra (#44-46) stay NULL.
- No riir-train contact; no league edits; no owner-gated lanes.
