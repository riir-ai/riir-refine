# Research 271 — Batch 240: the P67–P70 conversion (kernel_opt +3)

**Status:** RECORD — the staged set converted same-day; P70 adjudicated
watch-only. Batch 240 / Plan 242 / Research 271 (this note).

## Source

The 2026-10-15 watch poll's staged set (`.distill/001` top row at commit
`22fe7777c`) — the llama.cpp envelope window `e467e5b5e5ba → e36df604`.
All three patches fetched at the pins and EVERY quote verified verbatim
this session (`.scratch/b240_p6{7,8,9}.patch` + `p70.patch`; removed at
close):

| lead | pin | PR | grade | disposition |
|---|---|---|---|---|
| P67 | `9166ceaf16599406366cd2d01dc6a2126bbada4d` | #30421 | A− | OWN rule, E |
| P68 | `23f83e056d88ddae0d56b01ccc65fa42ab6625f7` | #30526 | B+ | OWN rule, A |
| P69 | `5a638aedb7c4352376d1e565a9a420b85f604628` | #29200 | B | OWN rule, B |
| P70 | `733f57a9994f7a237176aa9f4cdc31bc466d056d` | #27703 | B/watch | WATCH-ONLY |

## The rules (corpus 813 → 816; A 167, B 88, E 77)

1. **`subgroup-tail-lane-clamp-live-loads-gate-stores`** (E) — the
   subgroup TAIL-LANE law, verbatim in-source: "Keep tail lanes active
   for subgroup ops, but read a valid weight row." Every lane of a
   collective stays LIVE; its loads CLAMP (`gid_s = min(gid,
   LINE_STRIDE_A - 1)` at all 225 read sites across 13 adreno noshuffle
   gemv kernels); only its stores gate on the real bound — the
   pair/splitk store SPLITS into per-element guards ("// Keep tail
   stores inside this K-slice." + the two `gid * 2 + n < M` scalar
   stores). The asymmetry is the law: loads clamp, stores gate; a gated
   load is impossible (the collective needs a value). Kin:
   tail-clamped-row-reuse (B205 — the admission-widening face; THIS is
   the participation-safety face), masked-boundary-load (per-element
   skip vs redirect), role-scoped-store-must-match-compute-guard
   (aligns guards vs this DECOUPLES them).
2. **`pad-row-classification-interior-memcpy`** (A) — the pad op
   executes as ROW classes under row-major threading: interior+contig
   rows are `memset + memcpy + memset` (three libc calls, zero
   per-element arithmetic); boundary rows are one whole-row memset; the
   non-contig/negative-inner-pad fallback stays scalar; the circular
   variant keeps its full-row wrap. The old code: the 4-deep i0-i3 nest
   with `// TODO: optimize`. Kin adjudication (rider-vs-rule): the pad
   family owns WHETHER/WHAT/WHERE to pad (ragged-chunk-pad the
   STRATEGY, zero-pad-ragged-tail the additive economics, pad-tail-
   to-fixed-tile the tile fit, B226 the placement, the P45 ladder the
   zero source) — nobody owns the pad OP's execution; OWN RULE.
3. **`f16-upcast-at-foreign-graph-translation`** (B) — at a
   foreign-graph boundary the CONSTANTS' dtype pins the subgraph
   ("Swish inputs must have the same type but they are: f16 and f32");
   the fix upcasts every non-f32 INPUT, builds in f32, converts back to
   the caller's output type at the end — and the commit cites its own
   in-tree precedent ("translate_glu_swiglu_clamp already upcasts to
   f32 and converts back at the end; apply the same pattern here.") —
   the pattern-law class. Riders on the law: the supports_op
   honest-refusal arm (CUDA declines F16 SWIGLU_OAI until the kernel
   exists) + the CPU reference's tolerance-that-names-its-reason ("//
   F16 epsilon ~ 9.7e-4: relax tolerance vs the F32 default ...").

## P70 — the watch-only adjudication

#27703 "vulkan : copy strided f16 KV once on AMD so reads spread across
memory channels". The law, verbatim: "The copy is described in source
memory order so the strided access lands on the writes, which stream,
instead of the reads, which would concentrate on a few memory
channels." Gated AMD-not-RDNA4, neq1 ≥ 256, HSK%8==0; reuses the
generic copy shader. NOT converted: (a) the mechanism is
vendor-channel-aliasing-specific (AMD interconnect; our lanes are CUDA
4090 + Metal M3 + CubeCL/WGSL); (b) the grep for a Rust-side host —
strided-KV reads our attention kernels make — comes up empty by
design: our FA/GDN lanes read KV TILED (the A-category coalescing
family owns that axis), so there is no strided-KV surface to repair.
Re-read trigger: a future delta landing more of the class, or an
AMD/strided-KV host appearing in our trees.

## Gates (all green, isolated CARGO_TARGET_DIR=/tmp/riir-refine-b240)

- **G1** `batch240_rules_parse_and_needles_landed` — 22 needles across
  the three rules + both pseudocode halves parse ×3. First run caught
  one needle drift (em-dash vs `--` in the asymmetry sentence — the
  corpus is the authority); repaired, green.
- **G2** kernel_opt lib **3122/0/1i**; default lib **2670/0** (standing).
- **G3** clippy `-D warnings`: default lib ✓, kernel_opt --all-targets ✓
  (3 doc_markdown in the fresh `//!` header healed en route — the B192
  class), no-default --all-targets ✓, all-features lib ✓. DISCLOSED
  PRE-EXISTING (verified at HEAD with the batch stashed): one
  `drop(keys)`/TtyKeys warning in `src/bin/heal.rs:4053` at
  all-features --all-targets on Windows — the B234 tty_pick stub era,
  NOT this batch's surface (the batch's all-features LIB is clean);
  left for its owning lane.
- **T11 retrieval floors HELD** — 21/21 pass, 252.93 s; blended
  90.6/94.2/93.6, 94.7 vs floors 89.4/93.6/93.6; kernel self-variants
  100%. No re-pin.
- **Freeze/score-bench NOT owed** — zero clippy-corpus delta (P30).

## Count-pin walk (the B234 lesson held again)

The first pin pass covered 10 sites; the suite surfaced FIVE more
historical batch asserts (batch-227/231/232/235/237 gates) still
reading 813 — re-pinned 816 with honest labels. Full set: tests.rs ×15
(master, plan-003 sum, grouped, batch116, 5 per-category rows moved by
A/B/E, 8 historical asserts) + router.rs + mod.rs (exhaustive doc,
A/B/E comments, A/B/E slices, entries chain) + AGENTS ×2 + README +
kernel_opt.md (head + batch row). Also repaired en route: the B-category
comment read 86 through B225's +1 (87) — disclosed in the new 88
comment.

## En-route (no action)

- Envelope `e36df604 → 47f5a64ae` = ONE commit: #29766 vulkan concurrent
  first-device-init race — NULL (backend-lifecycle correctness; no
  kernel/perf law). New envelope pin: `47f5a64ae`.
- Watch legs NO-FIRE fresh (15:43 +07): prism tip `e3de715222` = pin;
  prism-b ladder 10777; cubecl ≤ v0.11.0 (the pre.N sort trap avoided).
- Intake hygiene: P58/P59/P60/P61 rows ticked `[x]` CONSUMED Batch 237
  (the M3's landing — verified in-tree at entries_25/26 +
  entries_gpu_87 before ticking; the rows had been left open by that
  lane).

NOTHING INVENTED — every quote verbatim at the fetched .patch files
this session; every number from this session's gated runs.
