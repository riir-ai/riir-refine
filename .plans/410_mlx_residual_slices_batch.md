# Plan 410 — Batch 411: the four mlx residual slices (vendored mlx tree, slice 3 residue)

**Status:** LANDED — 2026-11-24, session zed-amo-i411@amo (Research 444; claim closed by the landing; the four rules in entries_gpu_167, gates green, fixtures 16/16 pre-entry at debug AND -O; the router annotation floor re-pinned 97/195 with measured pre-batch attribution).

Queue head: plan 409's own staged residue (Research 443 §"Staged for the next
batch") — the four slices Batch 410 recorded but did not author, all from the
vendored upstream MLX tree at submodule pin `c69303b0` (the mlx-serve tip
`2201ec9` pin). Research 437's 8 mlxfast staged candidates stay queued behind.

Scope: FOUR corpus rules, one slice each:

- **S1 — small-filter per-row bounds bitmask** (`loader_channel_l.h:273-297`):
  `mask_h[i] |= in_bounds << kh` precomputed per row + the bit-test load path —
  the mask-precompute pole vs gpu_166 R4's hoisted branch (`whole-tile-inbounds-hoisted-loader-fast-path`).
- **S2 — dilation base tables + kernel odometer** (`conv.cpp:514-554` `base_h`/`base_w`
  + `loader_general.h:361-377` `next()` wrap-to-phase-base) — the tap-subsequence
  precompute for dilated/strided consumers.
- **S3 — simd-reduce width dispatch + 8-byte launch cap** (`ops.h:8-20` enable_if
  split + `reduce.cpp:494-496` threadgroup 512) — the kernel fallback + host
  tightening pairing.
- **S4 — col-reduce tier dispatch** (`reduce.cpp:947-976` — small <32 / longcolumn
  stride<32 && size≥1024 / 2pass >256 && out/32<1024 / looped; longcolumn's 32→128
  at total≥32768) — the tier-selection pole vs the landed 2pass-partition rule (compose).

License: ml-explore/mlx MIT (re-verify at clone). Transcription law: Metal/C++
source → ORIGINAL Rust fixtures, suggestion-only `NotBenchable-by-domain` (no
Metal runner in the validator) unless a rule is host-benchable.

- [x] T0 — re-clone ml-explore/mlx at pin `c69303b0` (blob:none, checkout the sha);
      LICENSE re-check; verify the four slice sites' quotes/lines at the pin
- [x] T1 — dedup greps at 3+ vocabularies over `src/domains/kernel_opt/` +
      `src/domains/rust_perf/` (bitmask/bounds/mask-precompute; dilation/odometer/
      phase-base/tap; simd-reduce/width-dispatch/threadgroup/launch-cap;
      tier/2pass/longcolumn/partition) — S1's only kin is gpu_166 R4 (the named
      pole, compose); S2/S3/S4 zero owners; the loose-word grep hits
      (gpu_117 constexpr-capacity, gpu_04 mask-sentinel, gpu_92/164) adjudicated
      distinct in-rule
- [x] T2 — two parallel reader subagents (conv-side: S1+S2; reduce-side: S3+S4);
      coordinator re-verified every quoted line at the clone; reader corrections
      carried (mask_t short + n-check asymmetry; clamp bites at row ≥ 2049;
      widening ×4 longcolumn-only; per-thread-rounds constant/aggregate-scaling;
      the trip-count clause load-bearing)
- [x] T3 — author corpus entries (`entries_gpu_167.rs`, 4 rules; D/D/N/D joins);
      per-entry adjudications vs the named kin, in-rule both directions;
      suggestion-only NotBenchable annotations with the domain reason
- [x] T4 — fixtures `b411_fixtures` standalone-compiled AND executed pre-entry
      (debug AND -O, 16/16, zero warnings; the gate caught THREE fixture
      defects: the masked path's missing weight multiply, the cost model's
      double-scaled narrow weight, the tamper expectation's wrong
      surviving-row value)
- [x] T5 — gates at the isolated CARGO_TARGET_DIR (/tmp/b411_tgt): default lib
      3981/0; kernel+latent 4128/0; retrieval floors kernel-variants 30/0 HELD,
      router annotation RE-PINNED 97/195 (measured 102/195 at landing; the
      pre-batch worktree at ccbcb76fb measured 108/195 — the batch cost 6
      annotations, the B223 mechanism amplified; delta recorded in the arm);
      clippy -D ×3 postures 0 (16 mechanicals healed via cargo clippy --fix —
      the stale-binary fallback, disclosed); fmt PASSED; numbering PASSED;
      pins_manifest agrees (1125); count web one-landing (router 1125 + the
      Plan-003 category table D274/N105 + health_views 1125/1795 + README +
      AGENTS + kernel_opt.md + the re-pin trail + the census count 168);
      score-bench/freeze NOT owed (zero clippy_lints delta); release-set binary
      NOT owed (corpus-only suggestion-only delta — the standing disclosure)
- [x] T6 — Research 444 batch record; snapshot claim → LANDED reshape; commit +
      push; teardown the clone + /tmp targets + the attribution worktree
