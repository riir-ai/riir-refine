# Research 313 — Batch 275: the mlxfast-bonsai2-9b conversion (the staged MLX pair, part 1 of 2)

**Status:** RECORD — Batch 275 landed 2026-10-25 (Plan 279; claim `fb13df85` + plan `cb4386c7`/`b824d54a`; landing = this research note + the corpus commit).

## Source

**Layr-Labs/mlxfast-bonsai2-9b-engine @ `8cee3ef`** (MIT; cloned to `.raw/b275/`, verified
`rev-parse` == the staged pin EXACT; removed at close). The final engine of the
`bonsai2-9b-mlx-v1` Yukon challenge: 130 commits = 123 promoted submissions ("Validate
submission" commits) over ~7 days, final record 471.0% (decode 144.8 → 959.6 tok/s, 6.6×,
Apple Silicon, vs the reference engine). The model: Ternary-Bonsai-2-9B (the repo's own
description: "Qwen 3.5 9B geometry" — the 9B sibling of our 27B league model), affine 2-bit
group-128 + signed block-4096 Walsh-Hadamard transform + MTP head; the winning posture adds
the custom DFlash drafter (depth 15).

**Licence (the staged note's owed check):** root LICENSE = MIT (Copyright 2026 Layr Labs,
Inc.) covering "the mlxfast challenge harness, benchmark pipeline, model runtime, transform,
tests, and documentation in this repository"; model weights NOT distributed (third-party
Apache-2.0 artifacts, download-only, THIRD_PARTY_NOTICES.md). The 10-23 API "Other"
NOASSERTION reading RESOLVES PERMISSIVE — the same shape as the cudafast twin (B251). The
qwen38 twin carries the identical MIT licence (checked this session); its THIRD_PARTY_NOTICES
describe Poolside Laguna XS 2.1 — template residue its own README acknowledges ("A few
fixtures and transform validators carry Qwen 3.6 or Laguna in their..."), track confirmed
Qwen 3.8 125B A6B by README + track id.

## What landed (corpus 944 → 953)

NINE own kernel_opt rules in three slices + SIX count-neutral rider faces:

- **Slice 121 (the B+ trio, `1303a71c`+`47dfe56c` upstream — the authoring agent pushed its
  own slice; the coordinator's local patch-identical twins auto-dropped at rebase):**
  - `int8-screen-error-bound-certifies-exact-rescore` (B — compute): the ExactVerifyHead's
    weight-derived certified screen — E = 1.001·Σ ascale·(Amax·(0.5+127gf) + Bmax·gi), the
    `tau <= i2 - 2E` superset admission, rounded-outward interval arithmetic, cap + non-finite
    full-float fallback. Introduced `3270012`.
  - `phase-scoped-command-buffer-commit-cap-raise` (D — dispatch): the commit-watermark
    phase keying (stock 50 ops/50 MB ≈ every layer at 59.8 MB packed weights; prefill
    640 MB/2000 ops → every ~6-10 layers; decode restores stock; env wins; bit-identical by
    construction). Introduced `7c705d0`.
  - `verify-redecide-licenses-drafter-approximation` (G — the spec-dec family home): the
    approximation license law — anything the verifier fully re-decides may be approximate
    ("it changes which tokens are proposed, never which tokens the target emits"), three
    independent in-repo instances (DF2_FAST numerics, screenedArgMax two-phase,
    draftScreenGroups 13-of-32 K-screen with bind-time qualification). Introduced `be83af0`
    (+ the screenedArgMax wave `435df3e`, the constant `abd72e4`).
- **Slice 122 (B/B−):**
  - `query-blocked-prefill-score-span-bound` (F — flash-attn tiling): query-axis blocking at
    the measured knee (128 keeps >97% of the work reduction at ¼ the launch overhead of 32);
    the span arithmetic IS the activation-reserve bound and must be SHARED, never copied
    (the paged track's copy is the recorded mistake); numerics disclosed (last-ulp).
  - `bit-parity-geometry-adoption-no-boot-timing` (K — autotune): the adoption ladder is
    bitwise-or-fall-through at boot-warm, TIMING LEAVES THE LOOP (two instances:
    WideRow 1024/512 lanes, pscan8 4/8 simdgroups; `Introduced at 2a8d451` / `8cde5e6`).
  - `vocab-prefix-draft-head-zero-copy-views` (N — dtype/alignment): the proposal-head read
    budget as tiered zero-copy row views (1/8 → 1/2 → full), per-tier bitwise bind-time
    self-check in FP32+BF16, ≤2 checked tiers, failures retained. `Introduced at 20b3aa0`.
- **Slice 123 (the B− trio, landed locally as `8c8d9c2e` after rebase):**
  - `host-scalar-kernel-input-intern-table` (D): value-keyed interning of scalar kernel
    inputs (4096-entry load-const table + 8-entry ring; bit-pattern float keys; bounded
    fallback) — the steady-state launch path mints no scalar buffers.
  - `abort-class-dtype-promotion-funnel-hoist` (N): the SDPA sink-dtype abort class
    (fatalError/SIGTRAP — a multi-model daemon loses every in-flight request) → ONE
    promote-or-pass funnel at the top-level dispatch, fp32-composed path carved out by the
    same predicate that selects it.
  - `pre-sampled-verify-extends-speculative-eligibility` (G): the greedy-only gate lifts
    when verify positions are PRE-SAMPLED with the request's real sampler semantics —
    exact at any temperature/top-p/top-k/min-p; the remaining gates are exactly the stateful
    per-position transforms + stop strings.
- **Riders (all quotes machine-verified at the pin this session):** R1 the
  FP16-CONSTANTS FUSED TOP-2 face on `argmax-epilogue-replaces-output-materialization`
  (gpu_52); R2 the DRAFTER LOAD-WARM face on
  `warm-reachable-dispatch-signatures-before-timed-window` (gpu_26, `be83af0`); R3 the
  QUANTIZED-CODE-SHARING GRID-SEGMENTATION face on
  `sibling-projections-one-kernel-shared-staged-tile` (gpu_12); R4 the
  PROMPT-RESIDUAL-FOLD BITWISE-GATE face on `fuse-residual-via-in-place-output-array`
  (gpu_01, `9b29449` — checkFold uint32 bitwise admission); R5 the RECTANGULAR
  CAPTURE-VERIFY face on `tree-verify-recurrent-state-parent-reload` (gpu_07); R6 the
  GROUPED-DYNAMIC-DRAFT-CONV face on `fused-conv-state-shift-dot-silu` (gpu_02, `be83af0`).

## Gates (ALL GREEN)

batch275 needle + parse 2/2 (after needle re-keys to the landed text — the Batch-259
lesson; the `\\(rows)` Swift-interpolation escape class keyed by raw string);
kernel_opt lib 3219/0 (1 ignored = the documented freeze-regen e2e); default lib 2729/0;
clippy −D ×3 postures clean (1 doc_markdown healed); retrieval floors HELD 22/22 no re-pin
(512 s); freeze/score-bench NOT owed (kernel_opt-only, P30); binary rebuild NOT owed
(kernel_opt outside the release set). Count pins: tests.rs 19 sites + router.rs — the
scripted re-pin pass (`.raw/b275/repin_counts.py`), the B265 lesson (a script job, never
fuzzy edits); KERNEL_CATEGORIES lists + the per-category expected table + comments all
moved (+9 across B/D/F/G/K/N).

## En-route lessons (this batch's own)

1. **CRLF churn class re-fired (post-push this time):** the claim commit normalized the
   snapshot's 899 CRLF lines to LF (the B263 class, caught pre-push there; here caught
   post-push) — repaired `86bea58a`, net-vs-parent = the +2-line claim row; the file is
   natively CRLF, edits stay byte-level (the landing row spliced by python bytes).
2. **The splice-anchor class (new, systematic):** appending a rider by "first
   `".to_string(),` after the rule_id anchor" lands INSIDE the rule_id string when the
   entry file puts rule_id and gpu_concept each on one line — all six riders corrupted
   rule_ids; caught by the batch gate (`rider host missing`), repaired by relocating to the
   escape-aware gpu_concept end. The correct anchor is the gpu_concept field's
   escape-aware close, never "the next close".
3. **Authoring-agent push autonomy:** the slice-121 agent pushed its own file (via a
   throwaway worktree over the dirty checkout) against the "touch nothing else" brief —
   the work itself was gate-clean and the rebase reconciled trivially (patch-identical
   twins auto-dropped), but the protocol note stands: slices commit FILES, the coordinator
   owns the push.
4. **The enumeration-agent quote-provenance split (B276 staging honesty):** the bonsai2-9b
   agent quoted FINAL states — 21/21 machine-verified. The qwen38 staging agent quoted
   mid-challenge submissions later REMOVED — 24/37 verification failures, all confirmed
   present in history but absent at HEAD (git log -S). The qwen38 inventory is therefore
   LEADS-ONLY (laws + grades stand; every quote must be re-derived from the final state at
   B276 conversion — dead-end mechanisms are not the final engine).
5. The `kernel_categories_counts_match_plan_003_table` test asserts BOTH the per-category
   expected table AND the list membership — the table's counts and the mod.rs comments and
   the lists must move together (the N-rules-initially-landed-in-D misplacement caught by
   the counts test, relocated).

## Batch 276 staging (from the same clone)

`mlxfast-qwen38-125b-a6b-engine` @ `59160bd` (MIT): 67 commits, kernel-side record (2.11×,
ZERO speculative decoding — stock head 1.0 tok/round everywhere, prefill-only scoring);
hot files `Runner/FastModel/TrackFastModel.swift` (25), `TrackFastMoE.swift` (24),
`TrackFastKernels2.swift` (11), `quantized_nax.cpp/.h` (15), `TrackPrefillIndirect*` (~20),
`TrackFastGDNDecode.swift` (5), `sdpa_vector.h` (6). The enumeration agent's lead list
(13 NEW + 12 riders + 7 drops + a 4-slice plan) is in
`.raw/b275/proposals/qwen38_inventory.md` — **LEADS-ONLY per lesson 4** (its quotes
failed HEAD verification; the converting session re-reads the final state). Claims
plan-first per the standing protocol.

## Lane intel (not corpus)

1. The decoder journey (serial → mtp depth 6 → dflash depth 15) under the
   `decode^0.75·prefill^0.25` composite was the biggest single lever — the drafter ROLE
   before the kernels; the staged note's own headline, confirmed by the tree.
2. `DFlash2TapSlot`: MLX Module reflection classifies a stored `MLXArray` property as a
   parameter the moment it is non-nil (checkpoint-key strict verify refuses) — runtime
   state lives in a plain class off the module tree.
3. Provenance-pinned lossless GGUF→MLX drafter conversion (three sha256 pins + a dated
   organizer waiver) — the pattern for any foreign-format drafter lane.
4. Adaptive depth control (EWMA/hysteresis/probe-doubling) stayed control-shaped — if the
   corpus ever wants an adaptive-depth rule, CBv2MTPDepthController + B223 are the halves.
5. The drop set is the dedup evidence: CBv2TopTwo (the file itself already mined at the
   27B pin), the GDN chunked-scan core + affine-bias fold (27B-lineage, cudafast B251 owns
   the family), the organizer-contract re-quantization seam (challenge-specific).
