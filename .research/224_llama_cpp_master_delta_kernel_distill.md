# Research 224 — llama.cpp master delta (post-B186 pin) kernel distill — Batch 192

**Status:** RECORD (batch landed 2026-09-30; Plan 189; the item-11 staging refill's candidates converted)

Source: `ggml-org/llama.cpp` master delta since the B186 pin `d834d44e` (09-28), ~40
commits, read from the STANDING local clone (branch `dflash2`, tree untouched; all
commits accessed via `git show <sha>`). Every load-bearing quote re-verified VERBATIM
at the pins by TWO independent authoring subagents + the staging pass before them.

## Verdict table (staged grade → landed)

| # | Commit | Staged | Landed | Disposition |
|---|---|---|---|---|
| L1 | `94a0ae3` vulkan MOE tile selection (#29182) | A− | **NEW RULE** `tile-picker-input-from-dispatch-grid-partition` (D) | The picker-INPUT axis is genuinely distinct from B186's admission-floor axis: B186's floor is a binary THRESHOLD on per-expert rows (path admission), this is a multi-way SELECTOR input (which compiled variant within the admitted path); failure modes disjoint, they compose. |
| L2 | `0bc845d` vulkan descriptor-set reuse (#29280) | B+ | **NEW RULE** `descriptor-set-binding-memo-destroy-epoch` (D) | Novel axis: DESTROY-EPOCH invalidation for handle-identity caches (zero corpus hits for descriptor vocabulary). Distinct from B26 (static/eager), B98 (arg-keyed probe), B178/B189 (graph caches), B100 (rebind no-op, no epoch). |
| L3 | `284153e` AVX512-FP16 f16-dot f32-accumulate (#29545) | B | **NEW RULE** `dot-macro-family-widens-at-load-f32-accumulate` (N) | The CPU REFUSAL pole of the accumulator-dtype family (B178 GPU licensing pole; B142 bounded-promote middle pole). Key refinement: the naive f16 accumulation was macro-family INHERITANCE (an ISA extension changed a shared storage-keyed family's meaning under unchanged call sites), not a chosen trade. |
| L4 | `5c200e0` vulkan GDN tune + Intel fix (#29476) | B− + LANE HIGH | **NEW RULE** `vendor-keyed-reduction-lane-partition-subgroup-request` (D) | Two novel levers: the vendor-keyed partition TABLE (identical resource reports — both report sg 32 — want different partitions: the measured residue a resource-ratio key cannot reach) + the subgroup-width REQUEST under `subgroup_size_control` (fail-open). GDN is OUR league arch; no lane-partition table exists in riir-gpu today (the adoption offer rides the rule). |
| L5 | `83dd71f` vulkan 2-aligned F32 paired loads (#29254) | B− | **RIDER** on `dequant-byte-pairs-as-aligned-u16-loads` (B98) | The full-width-element (32-bit) rung of the existing load-width-twin ladder + the family's first CORRECTNESS delta (the original F16 patch shipped MISSING the `a_offset` gate term — widening exposed it). |
| R1 | `18bbc46` metal FWHT TG-threshold 1024→512 (#29602) | rider | **RIDER** on `fwht-wide-block-row-per-threadgroup-exchange-ladder` (B116) + 1-line amend on the B176 host | TWO staged-claim corrections: (a) does NOT "match the CUDA twin" — the Batch-180 CUDA 512 boundary is `rows==1 && Blackwell`-gated, upstream Metal takes 512 UNCONDITIONALLY (same crossover datum, different gate shape — the arch gate did NOT reproduce cross-vendor); (b) no numbers ship. EN-ROUTE FINDING: `ggml_metal_fwht_supported_size`'s sync-comment drifted in the same commit (benign, but the "must stay in sync" prose contract broke exactly when the boundary moved). |
| R2 | `748d422` HIP `__vsubss4`→`__vsub4` (#29478) | rider | **RIDER** on `maddubs-overflow-two-exact-mac-selector` (B186) | The family's INVERSE-polarity member: DELETE the safety semantic the bound proves dead. The proof is STRUCTURAL not documented (fresh mask immediately before every sub — the commit title's "non-saturating" is the whole assertion). VGPR cost disclosed in-commit (sched_group_barrier + 1 allowlisted spill). |
| R3 | `6a2743f` CUDA bitonic argsort wide rows (#28957) | MAYBE | **RIDER** on `wide-row-copy-tile-rows-across-workgroups` (B96) — the CONTRAST arm | The staged "cross-block merge" framing does NOT reproduce — the commit keeps one block per row and widens columns-per-thread. The real law: the row op's EXCHANGE STRUCTURE decides the widening axis (streaming → across workgroups; barrier-synchronized exchange network → in-thread column ownership). Capability extension (bit-exact pin vs CUB at 2048), not a perf win. |

**Drops (staged, honored):** `c13e04e` model-loading speedup (cold-path, off-lane) ·
`19e28a2` hexagon f16 activation ops (vendor-specific annotation, B189 LUT-adjacent) ·
`5c200e0`'s Intel tile constants (instance, not law).

## Corpus movement

- kernel_opt **685 → 689** (D 149→152, N 52→53); 4 riders + 1 amend land on their hosts (no count change).
- New slice file `entries_gpu_59.rs`; registrations in `KERNEL_CATEGORIES` (D +3, N +1).
- Pins re-pinned with the honest label: tests.rs corpus-count ×2 + per-category table (D 152, N 53) + sums 689 ×2 + router.rs expert-coverage 689.
- rust_perf unchanged (165) — L3's CPU-ISA numerics law routed kernel_opt (N: dtype/alignment specialization) not rust_perf, following B178's GPU-twin placement.

## Gates (all green, 2026-09-30, M3, isolated `CARGO_TARGET_DIR=/tmp/riir-clippy-b192`)

- G1 parse/syn: `batch192_pseudocode_parses` (4 rules) + `batch192_concepts_carry_their_load_bearing_contracts` (the batch190_unsigned_only precedent — each rule's load-bearing caveat pinned in-text) + `batch192_riders_landed_on_their_hosts` (5 hosts) — 3/3.
- Kernel slice: 386/0 (`--features kernel_opt,latent_retrieval --lib kernel_opt`).
- G2 lib default: 2398/0.
- G3 clippy `-D warnings`: default 0 / no-default 0 / all-features 0 (4 doc-lint mechanicals in the new file's module header fixed en-route — the MoE backtick + a `+ `-leading doc line clippy's parser reads as a list bullet, the doc_lazy_continuation class).
- G4 retrieval floors: 21/21 pass (`retrieval -- --nocapture`, 252s) — kernel self-variants 100% (blend@3 90.6-94.7%, op-bigram@1 96.4% — no decay from +4 entries).
- Freeze/score-bench NOT owed (zero clippy-corpus delta — the B189/B190 P30 precedent).

## Standing lessons (this batch's additions)

- **A `+ `-leading continuation line in a module doc comment is a latent `doc_lazy_continuation`** — clippy's markdown parser reads it as a list bullet and every following prose line becomes an "unindented list item". Spell conjunctions ("and X and Y"), never `+ Y`, in `//!` blocks.
- **Quote re-verification at the pins keeps paying**: TWO staged claims corrected (the CUDA-twin "match" — the arch gate did not reproduce cross-vendor; the R3 "cross-block merge" framing — the commit is columns-per-thread), one staging-refinement (L3's inheritance-not-trade framing), one en-route upstream finding (the fwht_supported_size sync-comment drift). The staging pass is honest but staging-grade; the batch session's re-verify is where the corrections surface.
- **Subagent-count claims need source-count verification at landing**: one authoring subagent reported "N 75→76" — the actual N count was 52 (it counted grep hits, not slice entries). Every count in this batch was re-derived from `KERNEL_CATEGORIES` at landing time.

## Lane notes (adoption offers, not landed work)

- **GDN vendor-keyed lane partition** (L4): GDN is OUR league architecture; grep finds NO host-side lane-partition table in riir-gpu (no `lanes_per_column`/`cols_per_wg` anywhere). Before any cross-vendor tuning of the GDN lanes, encode the vendor-keyed table + the walk-down and re-measure per vendor. Intel Xe class: full-width reduction ~10× over COLS_PER_WG=8 (in-source claim, no bench table).
- **MoE picker shape-truth** (L1): any CubeCL/CUDA MoE grouped-GEMM tile selector re-measures before adoption — the derived per-workgroup N (CEIL_DIV of assignments over experts) vs the batch total is exactly the input our dispatch would reach for lazily.
- **Descriptor destroy-epoch** (L2): directly relevant to our wgpu/CubeCL bind-group caching lanes — the handle-recycling invalidation epoch is the sound-by-construction half any binding memo needs.
