# Plan 410 — Batch 411: the four mlx residual slices (vendored mlx tree, slice 3 residue)

**Status:** CLAIMED — 2026-11-24, session zed-amo-i411@amo
**Claimed:** zed-amo-i411@amo, 1795491000, expires 2026-11-26T03:30:00Z

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

- [ ] T0 — re-clone ml-explore/mlx at pin `c69303b0` (blob:none, checkout the sha);
      LICENSE re-check; verify the four slice sites' quotes/lines at the pin
- [ ] T1 — dedup greps at 3+ vocabularies over `src/domains/kernel_opt/` +
      `src/domains/rust_perf/` (bitmask/bounds/mask-precompute; dilation/odometer/
      phase-base/tap; simd-reduce/width-dispatch/threadgroup/launch-cap;
      tier/2pass/longcolumn/partition) — record zero-owner or adjudicate
- [ ] T2 — two parallel reader subagents (conv-side: S1+S2; reduce-side: S3+S4);
      coordinator re-verifies every quoted line at the clone
- [ ] T3 — author corpus entries (`entries_gpu_167.rs`, ~4 rules) + count-neutral
      faces if any pole folds onto an owner; per-entry adjudications vs the named
      kin (S1 vs gpu_166 R4; S4 vs the 2pass rule), in-rule both directions;
      suggestion-only NotBenchable annotations with the domain reason
- [ ] T4 — fixtures `b411_fixtures` standalone-compiled AND executed pre-entry
      (debug AND -O; the four-arm discipline; the gate must catch planted defects)
- [ ] T5 — gates at the isolated CARGO_TARGET_DIR (/tmp/b411_tgt): default lib;
      kernel+latent lib; retrieval floors BOTH lanes (hold or re-pin with the
      delta recorded); clippy -D ×3 postures; fmt gate; numbering gate;
      pins_manifest (kernel_opt 1121 → expected +4); count web one-landing
      (router pin + category table + health_views + README + AGENTS +
      kernel_opt.md + re-pin trail); score-bench/freeze NOT owed if zero
      clippy_lints delta; release-set binary NOT owed if corpus-only
      suggestion-only delta (the standing disclosure)
- [ ] T6 — Research 444 batch record; snapshot claim → LANDED reshape; commit +
      push; teardown the clone + /tmp targets
