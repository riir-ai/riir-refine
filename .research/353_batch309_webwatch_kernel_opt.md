# Research 353 — Batch 309: the 10-30 staged web-watch intake (kernel_opt)

**Status:** RECORD — landed as Batch 309 (Plan 314); every quote verbatim at the pin.

## Source window

The 10-30 staged intake (zed-m3-i1117-collect + zed-m3-i1117-watch, snapshot rows P105–P110
— the web-watch wave that also produced the P106 double-correction). Six PRs fetched:

| # | Source | Grade | Disposition |
|---|---|---|---|
| P105 | llama.cpp #29869 @ `cc470b64` + #30065 @ `b0cb1513` (MIT) | RIDER B | rider face on gpu_13 `small-batch-matvec-gemm-bridge-rung` (the per-type crossover face's SECOND instance, table form) |
| P107 | mlx #4596 @ `e1ef8a9` (MIT) | STRONG B | OWN rule (positive form) |
| P110 | llama.cpp #29298 patch 2 @ `4d426d5` + #28770 @ `41a4ad0` (MIT) | STRONG B | OWN rule (repair form) — P107+P110 are ONE atom |
| P108 | llama.cpp #30553 (MIT) | CONSUMED PRE-EXISTING | `stream-k-grid-capped-at-useful-work-quanta` (gpu_108, Batch 253 / Plan 257) |
| P106 | (llama.cpp #29353) | CONSUMED PRE-EXISTING | Batch 234/235 (the snapshot's ⟲⟲ row) |
| P109 | watch-only (3 rows) | WATCH | no mining until a scored artifact surfaces them |

## T0 verification notes

All six `.patch` files fetched to `.raw/b309/`; every quote byte-verified at the pins
pre-claim. Pin notes: the mlx staged row cited clone state `0e3ff36` (Oct 3) — the PR #4596
bundle head is `e1ef8a9` ("metal/sdpa: increase vector decode memory-level [parallelism]");
the TK machinery appears in the bundle both as the ADDED hunk (sdpa_vector.h, the kernel at
:259 whose enclosing template gains nothing — the unroll is in-body) and as a moved/comment-
shortened hunk in the file's second kernel; the M3 lane's `0e3ff36:213` template citation is
provenance for the file state, the bundle pins are ours. The #30553 PR head `08276950`
differs from the B253 rule's cited merged sha `4c1004c4` — the PR-bundle-sha-drift class
again (the b308 METHOD NOTE); the merged state is the verification surface and the B253 rule
already quotes it.

## Dedup record (T1)

- Unroll family: 136 'unroll' hits across kernel_opt — owners are the gpu_06 array-footprint
  rule, the rms-norm vpt ladder (entries_gpu_104), the B276 inject-4 scheduling-hint rider,
  and per-loop pragma mentions. ZERO owners of the runtime-trip-count law ('trip count|
  runtime trip' zero; 'issue.*together|memory-level' zero). Own rule minted.
- Per-type crossover: gpu_13's B116 rider owns the OVERRIDE form (Q1_0's mv→mm override);
  P105's rows_min TABLE is the same atom's table form → rider face, not an own rule.
  gpu_119 owns the PER-SILICON axis (its kinship note explicitly scopes B98's crossover as
  per-type — the map held).
- P108: the staged row proposed a rider on gpu_135 (`mma-window-admission-output-width-
  occupancy-floor`), but the corpus ALREADY owns the atom as its own rule from Batch 253 —
  including the FINAL 3-commit state (the nwork ceil refinement, the fixup coupling, the
  FP32-grouping determinism caveat). The staged row (a web-watch row without a corpus grep)
  missed its own corpus. Recorded consumed-pre-existing; the gpu_135 rider was NOT needed.
- P106: already recorded in the snapshot's double-correction row (Batch 234 landed the
  chunked-GDN distill at the same merge pin).

## En-route findings

1. **The B-slice mod.rs comment sat stale at 114** vs the measured truth 115 (tests.rs
   `expected` array), and the A/C/D/E/K/M comments are stale the same way (196/66/239/90/27/
   22 vs measured 201/67/240/94/28/23 — the per-batch lanes prepend "+N rule" notes without
   bumping the leading count; B307's commit message claims "slices A 201 / D 240" yet its own
   landing reads 196/239). The tests.rs array is the authoritative pin (sums to the corpus
   total). B repaired to measured truth (116 post-batch) with the miss disclosed; the OTHER
   slices' prose counts are NOT repaired here (count-neutral prose drift, no gate reads
   them) — flagged for a future docs-hygiene unit.
2. **The per-category wiring is TWO sites, not one**: the `KERNEL_CATEGORIES` tuple carries
   an explicit `&[rule_id]` list per category — a new rule MUST join that list or the
   exhaustive-coverage + counts tests fail (measured: 37 → 3 → 1 failures across the three
   fixes). The B307-era plan's "mod.rs slices" task item understates this.
3. **37 historical count witnesses** (one `assert_eq!(corpus.entries.len(), N)` per batch
   test) all need the bump — the regex pass caught 34 + the 3 hand-fixed sites (the
   re-pin-history message, the category-sum, the grouped-total).
4. gpu_135's window rule was the WRONG host for P108's face but is the right KINSHIP cite —
   it already cites `streamk-starved-tile-grid-dispatch` and the occupancy family; no edit
   needed there.

## League intel carried (no mining)

- P105's upstream numbers are M3 Ultra kernel-only: 4.41× Q1_0@8r / 4.30× Q2_K@8r / 4.03×
  TQ2_0@16r; #30065's 16 more types at 3.06–4.14×; 512-row within 1%. The motivation note:
  DFlash2 on M3 Ultra was SLOWER than serial at few-row widths — the opponent builds
  speculation-over-GDN economics on these kernels.
- P106's operative residue (unchanged): the next scored opponent tag carries the chunked
  CUDA GDN DEFAULT-ON at SM89 — expected ≈ +3% (27B) on pp2048/pp4096 → our 1.19×/1.14×
  prefill leads compress to ≈1.15×/1.10× at re-pin (owner-gated); the RIIR_PREFILL_GDN_CHUNKED
  27B re-measure stays 4090-window-gated.
- P107's en-route: MLX #4734 sliding-window attention (360078a) + #4723 sdpa_split mid-row
  kernels for speculative decoding (065ec46) — the first MLX-side motion matching P109(c)'s
  spec-tree rumor.
- P110's class context: ggml sparse FA is opponent-mainline (read-skip substrate); no
  training-free selector upstream — the 1117 FlashMemory selector remains unmatched.

Sources: ml-explore/mlx @ e1ef8a9 (#4596, MIT), ggml-org/llama.cpp @ cc470b64 + b0cb1513 +
4d426d5 + 41a4ad0 (#29869/#30065/#29298/#28770, MIT). Session: shikuwa-4090-b309, 1793359500.
