# Plan 189 — Batch 192: llama.cpp master delta (post-B186 pin) kernel pack

**Status:** COMPLETE — landed 2026-09-30 ~07:0x +07 (M3, cargo-quiet: canon_p4 exited;
the Plan-337 deferral discharged). All gates green; corpus 685→689.

Source: the item-11 staging refill (`.distill/001` 2026-09-30 ~06:0x row) — llama.cpp master
delta since the B186 pin `d834d44e`, read from the STANDING local clone (`/Users/katopz/git/llama.cpp`,
tree untouched on `dflash2`). Route: kernel_opt (kernel-only → zero clippy-corpus delta →
**no freeze regen, no score-bench owed** — the B189/B190 P30 precedent). Record: Research 224.

## Leads (from the staging row)

- [x] **L1** `94a0ae3` vulkan MOE-aware mat_mul_id tile selection (#29182, A−) — NEW RULE
  `tile-picker-input-from-dispatch-grid-partition` (D). Dedup held vs B186's per-expert FLOOR
  (admission vs variant-selection axes; disjoint failure modes, they compose) +
  expert-remainder/batch-width-twins/indirect-dispatch read in full.
- [x] **L2** `0bc845d` vulkan descriptor-set reuse (#29280, B+) — NEW RULE
  `descriptor-set-binding-memo-destroy-epoch` (D). Zero corpus vocabulary hits — clean-new axis.
- [x] **L3** `284153e` AVX512-FP16 f16-dot f32-accumulate (#29545, B) — NEW RULE
  `dot-macro-family-widens-at-load-f32-accumulate` (N). Framing REFINED at re-verification:
  the naive f16 accumulation was macro-family INHERITANCE (an ISA extension changed a shared
  storage-keyed family's meaning under unchanged call sites), not a chosen trade.
- [x] **L4** `5c200e0` vulkan GDN tune + Intel fix (#29476, B− + LANE INTEL HIGH) — NEW RULE
  `vendor-keyed-reduction-lane-partition-subgroup-request` (D). GDN league lane note in-rule.
- [x] **L5** `83dd71f` vulkan 2-aligned F32 paired loads (#29254, B−) — RIDER on
  `dequant-byte-pairs-as-aligned-u16-loads` (B98; the full-width-element rung + the family's
  first CORRECTNESS delta — the original patch's missing a_offset gate term).

## Riders

- [x] **R1** `18bbc46` metal FWHT TG-threshold 1024→512 (#29602) — RIDER on B116
  (`fwht-wide-block-row-per-threadgroup-exchange-ladder`) + one-line amend on the B176 host.
  TWO staged-claim corrections recorded (the CUDA-twin gate-shape divergence; no numbers ship)
  + the en-route finding: `ggml_metal_fwht_supported_size`'s sync-comment drifted in-commit.
- [x] **R2** `748d422` HIP `__vsubss4`→`__vsub4` (#29478) — RIDER on
  `maddubs-overflow-two-exact-mac-selector` (B186; inverse-polarity member; the proof is
  STRUCTURAL — fresh mask before every sub — not documented).
- [x] **R3** `6a2743f` CUDA bitonic argsort wide rows (#28957) — RIDER (not DROP) on
  `wide-row-copy-tile-rows-across-workgroups` (B96) as the widening-axis CONTRAST arm; the
  staged "cross-block merge" framing corrected (one block per row, columns-per-thread).

## Batch gates (the B189/B190 kernel-pack gate set) — ALL GREEN

- [x] G0: entries authored (`entries_gpu_59.rs`, 4 rules) + `batch192_pseudocode_parses`
- [x] G1: parse/syn check — 4/4 + `batch192_concepts_carry_their_load_bearing_contracts`
- [x] G2: `cargo test --lib` (default) — 2398/0
- [x] G3: kernel slice 386/0 + clippy `-D warnings` clean ×3 postures (4 doc-lint
  mechanicals in the new file fixed en-route)
- [x] G4: retrieval-precision re-measure — 21/21 floors HELD (252s; blend@3 90.6–94.7%,
  op-bigram@1 96.4%)
- [x] Corpus count pins: tests.rs ×4 (685→689, D 149→152, N 52→53) + router.rs expert-coverage 689
- [x] Research note `.research/224_llama_cpp_master_delta_kernel_distill.md` (Status: RECORD)
- [x] Docs: AGENTS.md kernel_opt row 689 + B192 record; `.docs/11_domains/` batch row
- [x] Commit `feat: batch 192: ...` + push; `.distill/001` snapshot row updated (same commit)

## En-route notes

- [x] Two staged claims CORRECTED at pin re-verification (R1's CUDA-twin "match"; R3's
  "cross-block merge" framing) — the quote-re-verification discipline paying for itself;
  recorded in Research 224.
- [x] One authoring-subagent count claim corrected at landing (N was 52, not 75 — grep hits
  vs slice entries); every count re-derived from KERNEL_CATEGORIES.
- [ ] Healer global binary STALE stamp found this session (missing kat_transport,fix_verify)
  — the rebuild remains owed AFTER this batch (standing hazard, separate unit; the batch gates
  used cargo test/clippy directly, not the global binary). DEFERRED to the next unit with
  reason: the rebuild is a release build contending with this batch's gates on the same
  target dir; next session takes it as its first unit.
