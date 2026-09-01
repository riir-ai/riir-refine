# Research 111 — mlx-lm gated_delta.py GDN prefill kernel distill (Batch 95)

**Status:** RECORD

- **Source:** `ml-explore/mlx-lm` `mlx_lm/models/gated_delta.py` @ main
  (Apache-2.0), fetched + read in full 2026-09-01 (615 lines; mine-time
  re-read per Protocol A — moving main). Snapshot lead from the Bench 831
  follow-up source read (same day). Plan: `.plans/055_batch95_mlxlm_gated_delta_prefill_distill.md`.
- **Verdict:** 3 kernel_opt rules + 2 riders + 1 B79 confirmation note.
  Corpus 411 → **414** (D 84→85, E 25→26, G 25→26). Two parallel subagent
  lanes INDEPENDENTLY CONVERGED on the same three decision contents (packing
  geometry, comparator oracle, whole-T single-dispatch) — the coordinator
  merged the best halves of each lane's texts and re-verified every quoted
  line against the fetched file (spot-checked at L8-13, L139-145, L234-238,
  L256-259, L266-268, L296-314, L334-336, L429-442, L490-494 — all
  byte-accurate).

## The source shape (why these rules)

mlx-lm's GDN prefill ships THREE Metal kernel variants of the same
whole-sequence gated-delta scan:

1. **Generic** — one 32-lane simd-group per value row; `n_per_t = Dk/32`
   state elements per lane; two `simd_sum` calls per row per token;
   `for (int t = 0; t < T; ++t)` inside ONE kernel; state in
   `float state[n_per_t]` registers, one load at entry, one store at exit;
   zero smem, zero barriers; ONE dispatch covers `[B, T, Hv, Dv]`.
2. **xtree comparator** — the unpacked layout with the two `simd_sum` calls
   replaced by the explicitly-written ascending butterfly
   (`simd_shuffle_xor 1,2,4,8,16`), existing PURELY as the packed kernel's
   bitwise oracle: "so the reduction order is a contract of this file rather
   than of the simd_sum lowering."
3. **Packed (Dk=128)** — `lanes_per_row = 4`, `rows_per_simdgroup = 8`,
   `values_per_lane = 32`; per-row reduction = 3-level in-lane pairwise tree
   over 8 partials + 2 shuffle levels inside the row group; grid y coarsens
   `Dv → Dv//8`; "bitwise-identical by construction" to the comparator.

Plus the shipping discipline: `MLX_GDN_PACKED=0` kill-switch (module-level
env read) + the `packed_eligible` shape/dtype predicate (mask is None, scalar
gates, Dk == 128, Dv % 8 == 0, f32 g/state) falling back to the original
kernels for every other shape.

## Rules landed

1. **`simdgroup-row-packing-reduction-amortization` (E)** — a register-
   resident scan whose per-lane state slice is thin (width/32) and whose
   rows pay full-plane reductions per token should pack
   `32/lanes_per_row` independent rows into ONE simd-group (lanes split each
   row's reduction; each lane carries `width/lanes_per_row` contiguous
   elements; grid coarsens in the row dimension). The bit-identity contract
   is the load-bearing half — four clauses: power-of-two lanes_per_row,
   contiguous per-lane slices, in-lane tree replicating the LOW butterfly
   levels' pairing shape, per-partial sequential order preserved (IEEE + is
   commutative, NOT associative — reassociation only at commutative nodes
   above the preserved chains). Distinct from B28's
   workgroup-equals-plane-native-reduction (one plane per ONE reduction vs
   MULTIPLE rows per plane), B22 warp-shuffle (substrate choice vs lane
   assignment geometry), B28 row-owned-register-blocked-recurrence
   (per-decode-step state-matrix rows vs many value rows amortized inside
   one plane), B81 cross-lane-column-packed-projections (inputs over one
   weight vs recurrence rows with order preservation), B25 tree-depth
   (accuracy-driven vs contract-driven tree shape).
2. **`whole-sequence-zero-barrier-single-dispatch-scan` (G)** — the FOURTH
   recurrence dispatch strategy beside the scan (parallel-t, needs
   associativity) and the chunked family (cross-workgroup state handoff):
   run the ENTIRE T loop inside ONE kernel, state register-resident across
   the whole sequence (one entry load, one exit store), every cross-lane
   reduction simd-local, cooperation radius pinned to one simdgroup
   (threadgroup.x == simd width) so a barrier is structurally never needed —
   exact by construction, no rel-error gate. Eligibility bound = register
   capacity + simdgroup shape, NOT a sequence-length crossover. Distinct
   from B79 scan (parallelizes t), B12 chunk family (chunk boundary +
   handoff), B28 row-owned (per-step residency vs per-SEQUENCE residency),
   B57 smem-slot-role-reversal (this shape structurally avoids that class —
   the zero-barrier elimination side), B33 smem-staging (only cross-thread
   reuse is the reduction scalar — nothing to stage).
3. **`reduction-restructure-needs-source-owned-comparator` (D)** — when an
   optimization restructures a reduction across lanes, pin its bit-identity
   against a dedicated comparator that WRITES the butterfly explicitly in
   source; a library intrinsic (simd_sum/subgroupAdd) is toolchain-owned
   lowering, and asserting packed == generic when BOTH call the intrinsic
   pins only a toolchain coincidence (a driver update moves both kernels'
   bits together while the assert stays green). Sequencing: materialize the
   intrinsic's tree as the comparator FIRST (device-scoped agreement
   recorded, never promoted to the contract), build the variant against the
   contract SECOND, prove the level mapping at the rewrite site. Two levers:
   commutativity (pairwise tree ≡ low butterfly levels given per-partial
   order) + trajectory propagation (bit-identity of the per-step reduction
   propagates through the deterministic recurrence to every y and the final
   state — pinning ONE reduction pins the whole trajectory). Eligibility
   predicate + kill-switch + comparator-as-A/B-arm are part of the
   discipline. Distinct from B57 tile-loop gate rule (GATE-fixture half vs
   ORACLE half — they compose), B35 exact-op-parity (fwd/bwd op consistency
   vs variant-vs-variant identity), B79 scan (rel-error gate when no
   source-owned oracle exists vs bit-exact-by-construction when one does),
   B51 fast-math flag (across builds vs across restructurings), B23
   hardcoded-fast-path (which kernel runs vs what its numbers are pinned to).

## Riders (positive instances — external confirmation, no new rules)

- `MLX_GDN_PACKED` module-level env read → **B38
  env-flag-resolved-once-not-per-dispatch**.
- `packed_eligible` shape/dtype dispatch predicate → **B23
  hardcoded-fast-path-jit-specialization**.
- `hv_idx / (Hv / Hk)` GQA head-sharing arithmetic → **B23
  gqa-kv-head-sharing-bandwidth-reduction**.
- **B79 confirmation:** B79's concept text notes "mlx-lm at HEAD still runs
  the sequential per-token loop" (mamba.py); gated_delta.py confirms the
  same house shape for GDN — and B79's regime note (dxd state recurrences
  chunk/minimize rather than scan) is exactly what rule 2 names as the
  fourth strategy's niche. No text change needed to B79.

## Rejected candidates (dedup ledger)

1. Explicit-butterfly-over-simd_sum as a PERF rule — B22 substrate choice;
   the source itself states the trees lower identically on current Apple
   GPUs; the value is the contract (rule 3).
2. Associative-scan the T loop — the delta correction intervenes between
   updates; B87's boundary rules govern; all three kernels keep t
   sequential by design.
3. Grid coarsening standalone — the launch-side half of rule 1; folded in.
4. Contiguous-per-lane mapping as a coalescing rule — REQUIRED by rule 1's
   bit-contract clause (b); not independently actionable.
5. Eligibility predicate + kill-switch as a rule — house GOAT convention,
   not kernel decision content; encoded in rule 3's discipline.
6. Redundant-total += butterfly (no broadcast needed) — micro-property of
   the butterfly; folded into rule 1's mechanism.
7. Ops-reference-kept-alongside-kernel — generic testing practice; the
   comparator supersedes it as the oracle.
8. Per-t pointer-increment walk vs recompute — plain affine induction;
   compilers strength-reduce it (unlike B44's packed bit-address).

## GOAT

- G1: `batch95_pseudocode_parses` (cfg(not(kernel_verify)) per B70) — all
  3 ref_shapes + opt_patterns parse; PASS.
- G2: kernel_heal lib **1235/0** (+1 = the new gate; B92 baseline 1234);
  default lib **1001/0**; PASS.
- G3: clippy `--all-targets` 0 warnings in BOTH states (default +
  kernel_heal); PASS.
- Retrieval floors: `--features kernel_heal,latent_retrieval --lib
  retrieval` **14/14 HELD** (blended 94.7/96.9/97.7 band unchanged — no
  re-pins); PASS.
- Count pins walked: tests.rs L69-70 (414 "through Batch 95"), L5362/5364
  (category sum), L5375-76 (grouped total), expected-array D 85 / E 26 /
  G 26, the stale "// Total: 209 ... current 400" comment (was 2 batches
  stale → corrected to 414), router.rs L461 (414); mod.rs KERNEL_CATEGORIES
  comments D/E/G + 3 array insertions.
- rustfmt: entries_gpu_09.rs was fmt-clean at HEAD and remains fmt-clean
  post-edit (file-mode check, both editions); mod.rs / tests.rs / router.rs
  carried pre-existing drift at HEAD → left unfmt'd per house discipline.
- Full CI guard: NOT run this cycle (background guard takes >60 min after a
  corpus edit; the three affected states + retrieval floors + count pins are
  all validated above — the corpus is cfg'd under kernel_heal only and
  those are the states that compile it). Recorded honestly.

## Snapshot queue state after this batch

- **Batch 95: LANDED (this note).**
- **Batch 94 (oMLX PR #3260 expert-streaming distill, ~8 rules + 2 riders,
  Research 110 grade A-): still QUEUED** — the next mining unit; mine-time
  re-read required (head `3343e44` force-push-prone; re-clone
  `refs/pull/3260/head`).
- Two-lane convergence note for future batches: both lanes independently
  derived the same 3-rule decomposition and split it differently (lane A:
  packing + comparator; lane B: scan + comparator + packing-construction) —
  convergence on decision content is the strongest dedup signal observed in
  the program.
- Observed stale surface (NOT fixed this batch): `.docs/11_domains/
  kernel_opt.md`'s per-category sections (e.g. "Category N ... 13 rules")
  predate several batches while the Total line is current — a doc-sync
  unit owns the per-category re-derivation, filed here as the record.
