# 434 — the mlx-serve ANE seed batch (Batch 399, Plan 399)

**Status:** LANDED 2026-11-23 (zed-m3-ane-b399@m3) — the ane domain 13 → 21 (M1–M8), rust_perf 387 → 389 (entries_94), +2 count-neutral faces (the free-width face on `ane-prediction-gap-fewer-bigger-calls`, the split-seam complement face on kernel `first-layer-ahead-submission`). Research 432 §Queue slice (2) is the controlling pool row; this note is the batch record (T0 + dedup + landed set + honest drops).

## T0 — source state

`ddalcu/mlx-serve` cloned full at **`96311b2638cf2f0ebdc7a2e39c85882a4fc4654a` — UNMOVED from Batch 398's T0 pin** (zero delta, every 398-verified byte still valid). Surface: `docs/reference.md` §"Neural Engine engine" (lines 440–456) + §"ANE prefill rules" (827–848) + §"Media offload" (853–862) + the w8a8 header (866–872) + §"Media offload share" (885–896) — read end to end — plus the gotchas incidents the rules cite: `docs/gotchas/engine-mlx.md` §"Neural Engine prefill offload" (the four ANE incidents, lines 680–730) and `docs/gotchas/models-media.md` (the H3-black incident, the tile-tail/share/calibration-dtype incident, the ACE-Step normalizer incident, lines 345–375). Every quote byte-verified at the clone this session.

## Dedup (T1) — the load-bearing adjudications

- **M1 fold-placement** vs `ane-fp16-norm-pow2-prescale-overflow` (norm-statistics overflow pole, scale-invariant output, no read-back), `constant-scale-folded-into-upload-copy` (gpu_04: fold-for-dispatch-elimination, perf motive), `masked-in-place-field-prescale-other-operand` (bit-offset cancellation), `fp16-subnormal-product-power-of-two-rescale` + `hi-lo-split-pow2-prescale` (B352, the underflow pole) — all adjudicated in-rule both directions; the operand-placement law (linear operand vs under-nonlinearity) is unowned.
- **M2 load bill** vs `batch-load-admission-sized-from-files-refuses-past-the-hard-budget` (B274 — the files-metadata sizing pole; THIS is the complementary pole: when the runtime TRANSFORMS the load, the file-sized bill under-admits), `gpu-residency-summed-beyond-process-and-cgroup-accounting` (B367), `one-program-many-functions-accelerator-service-memory` (B358), `budget-room-from-constructed-artifact-not-configured-cap` (B375) — all in-rule.
- **M3 compile cache** vs `reserve-then-commit-capacity-accounting` (entries_58), `vanished-entry-tolerant-eviction-panic-else` (B343), `derived-cache-throwaway-version-reset-recovery` (B359), `budget-room-from-constructed-artifact` (B375), `persistent-cache-teardown-needs-second-witness` — the volume-derived cap + lineage-ordered eviction + in-flight-set protection is unowned.
- **M4 pitch grid** vs `ane-int8-matmul-shape-silent-wrong-gate` (A4: silent-WRONG results, host-reference gate defense) — THIS the silent-FAIL class (compiles, fails every eval), plan-time grid-floor defense; both are "compile-success ≠ serve-correctness", signal + defense differ. `align-64-false-sharing` is the host cache-line grid, different cost.
- **M5 split share** vs `ane-split-calibration-fit-bandwidth-interference-term` (B376: the fit law T(s)) — THIS the share's lifetime (per-silicon table, solve-once-stick, override-vs-cached, never-interpolate); `measured-round-cost-spec-controller` (B398), `per-generation-plan-table-unmeasured-generation-fallback` (B376), `launch-heuristic-pinned-to-calibration-arch` cited.
- **M6 adapter decline** — clean, no corpus owner (the adapter-vs-split eligibility axis is new).
- **M7 deep-K chunk** — clean (the split-K GPU families split for occupancy, not an engine-rate cliff).
- **M8 I/O planes** vs `dual-bank-scratch-retention-per-round-class` (gpu_138: bank-per-class across flips) — THIS plane-shared-across-classes because evals serialize; the discriminator is class-transition interleaving. The serial premise cites `ane-concurrent-same-function-silent-corruption` (A10).
- **rust_perf probe-sweep** vs `benchmark-spread-above-threshold-reports-unstable` (B345 — the stability VERDICT after measuring), `aa-control-prices-the-fidelity-instrument` (B225), `measured-round-cost-spec-controller` (B398 — the contamination guards ON samples; THIS refuses the single-point INPUT shape; compose).
- **rust_perf normalizer metric** vs `unique-prompts-defeat-prefix-cache` (B367, the population-split family) — different axis (metric placement vs population split); clean.

## Landed (8 ane + 2 rust_perf + 2 count-neutral faces)

M1 `ane-fp16-range-scale-folds-into-linear-operand` (Numerics) · M2 `ane-load-bills-runtime-copy-shape-not-shard-bytes` (Lifecycle) · M3 `ane-compile-cache-cap-volume-derived-lineage-pruned` (Lifecycle) · M4 `ane-plane-row-pitch-grid-refused-at-plan-time` (GraphShape) · M5 `ane-split-share-per-silicon-never-interpolated` (Placement) · M6 `ane-full-activation-adapter-declines-split` (GraphShape) · M7 `ane-deep-k-reduction-chunked-in-graph` (GraphShape) · M8 `ane-io-planes-shared-per-shape-class` (Lifecycle) · P1 `calibration-probe-swept-band-single-point-outlier` (measurement-honesty, entries_94) · P2 `metric-after-normalizer-measures-the-normalizer` (measurement-honesty, entries_94). Faces: the free-width face (64 rows cost what 8 do → batch to the free width) rides `ane-prediction-gap-fewer-bigger-calls`; the split-seam complement face (submit the complement before the blocking accelerator wait; launch-latency-not-rows) rides kernel `first-layer-ahead-submission`.

## Positive instances recorded, never landed (the w8a8 section is mostly owned law at home)

- The sibling-projection act-quant memo = the TileQuant face (B338's SIBLING-PROJECTION ACT-QUANT MEMO arm) — same law verbatim.
- Stacked weights + row views (`w8a8.fuse(.stack)`) = `co-lifecycle-rows-pack-one-entry-strided-views` (B355).
- The tied block-256 Hadamard rotation before int8 = `tied-rotation-bilinear-invariance-quant-isotropy` (B339 TriAttention).
- The per-tile softmax merge by maxima (`attnCore`) = the standard streaming-softmax (textbook prior art, not mined).
- fp16 planes not bit-lossless; the bar is per-program cos/rms parity + perceived-content equivalence = the house lossy-surface law (per-family retention) + the G5-parity shape — the HF companion-pack amendment recorded it independently (2026-11-22b).
- ReqKey bind-identity request cache = `pipeline-cache-key-covers-every-function-constant` (TileQuant B338) kinship.
- The int8 copy REPLACES the packed weights + `dropBlock` (loading peaks at converted size) = folded into M2's notes.
- The procedure-bank wrong-API-twin (`inputSymbolIndicesForProcedureIndex:` answers 0 and fails every procedure > 0 as a swallowed slowdown; `eval_failures` the tell) — a real swallowed-slowdown law, left UNMINED at the size discipline (4-6 target exceeded already); staged for a return visit alongside the mlxfast re-visit.
- The lane-split bar story (self-consistency passing while `convert` zeroed first rows at a 16-multiple boundary) — a testing-shape law, left unmined (same size call).

## Yield vs estimate

Plan estimated 4–8 after falsification; landed 10 rules + 2 faces — above estimate, disclosed (the ANE sections were denser than the verdict's unread-residue framing suggested; the source self-documents its own corrections, which transcribe directly as decision content).

## Gates

lib 3957/0 · clippy `-D warnings` ×2 postures (lib, all-features) clean · retrieval regression gate at DEFAULT posture 30/0 + the kernel,latent posture 30/0 + the synth floor re-run (1366 spans, blended 98.2% top-1 — floors HELD, no re-pin) · fmt gate PASSED · pins_manifest --write in-landing · the count web one-landing (AGENTS, README, .docs/11_domains ×2 + README table, health_views ×4 sites, the kernel-side rust_perf witness ×2 sites, annotated pins 335→337 ×3 sites) · **the maintained-binary rebuilt at the release feature set (the exact release.yml line) and re-linked; `build:` == HEAD** (the owed-at-next-maintained-binary-lane residue from B398 discharged with this batch). Score-bench/freeze NOT owed (zero clippy_lints delta asserted).

Teardown: `.raw/mlx-serve` removed at close (this note's quotes all read at the pin before removal).

Session: zed-m3-ane-b399@m3, epoch 1795378145.
