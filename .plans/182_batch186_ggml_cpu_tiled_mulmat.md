# Plan 182 — Batch 186: ggml-cpu tiled mul_mat k-quant distill (staged lead `d834d44e6`)

**Status:** COMPLETE (2026-09-28 — renumbered 181→182 pre-push: the sibling pickup session allocated 181 first; citations rewritten in-commit — Batch 186 landed; see .research/218; all gates green — the B185 handoff's designated next-pool kernel slot; lead staged 09-26 13:0x in `.distill/001`, caveats re-adjudicated at the pin)

## TL;DR

Convert the staged 09-26 lead: `ggml-org/llama.cpp` commit `d834d44e643681f7b046a22d357335f6f4ff6107`
"ggml-cpu: tiled mul_mat for k-quants (#27851)" (jbooth; 11 files ALL ggml-cpu + tests: iqp.cpp/.h
REMOVED ~1292L, a NEW `ggml-cpu/tiled/` family ~2175L, ggml-cpu.c +48, test-tiled-mulmat.cpp 852L).
Commit's own numbers: 3-6x for large matmul, break-even 4096x64·64x4096, 80% net loss GEMV.
Source found IN the local prismml-llama.cpp clone (`E:/git/vendor/prismml-llama.cpp` carries the
commit) — no new clone needed; every quote re-verified verbatim at the pin (Batch 31 discipline).
Staging caveats discharged: (a) k-quants ≠ our ternary class — still mined, the corpus takes the
REGIME/admission/staging/measurement substrates, not the architecture; (b) dedup grep clean
(3+ vocabulary variants × 0-13 hits, all non-colliding: the 13 "repack" file hits are GPU
register/smem repacks, different layer); (c) iqp.cpp removal — corpus has ZERO iqp-derived rows
(grep clean), nothing staled; its surviving lessons (floor 8, late-consult constraint) are folded
into the landing rules.

## Slot adjudication

Batch 185's queue: "coverage-report re-arm at the next batch slot (head's remainder = standing
skip list) + the ggml-cpu tiled mul_mat lead (kernel slot) + walk #12 Mon 10-05". The coverage
re-arm's head is the recorded skip list — a re-derivation now yields nothing new. The tiled lead
is the designated kernel-slot content. TAKEN. No twin rows on origin (checked 10:41 UTC).

## Tasks

- [x] T0 — Source located at the pin in the existing prism clone; file census verified (11 files,
  +3050/-1326); read tiled.h/tiled-kernel.h directly; content read via 2 parallel subagents over
  disjoint slices (driver+microkernel | integration+tests+removal) + my own verbatim re-verification
  of every load-bearing quote (min_batch 1202-1205, dead tail 615-642, narrow 664-679, wsize
  651-658, ACTBIAS precompute 685-692, maddubs overflow 216-226 + 540-548, NK dispatch 505-512,
  VNNI interleave 568-576, jit repack 543-546 + 815-822, zero-pad 541-562, chunk stepdown
  1096-1106, force env 578-586, consult site ggml-cpu.c 1269, MMID group 710-722 + per-expert
  1230-1244, owned transform 1068-1082, flush bench 343-357/441-443, toggle oracle 108-126/201-207,
  widen metadata 36-56 + kernel 125-133, writeback transpose 684-696, iqp removed comment).
- [x] T1 — Dedup grep BEFORE proposing: tiled mul_mat / tile dot / microtile / dpbusd|VNNI /
  activation bias / offset term / bsums / iqp / unpack-once / repack / gfni / workspace slot /
  batch floor|GEMV — all clean or non-colliding; B181 offset-family members read verbatim for
  composition (entries_gpu_50.rs).
- [x] T2 — 2 subagents proposed (A: 19 rules over driver+microkernel; B: 9 rules over
  integration+tests); merged + adjudicated to **17 landing + 5 staged** (staged quotes preserved
  in Research 218 so the next slot converts with zero re-mining).
- [x] T3 — Author corpus: `entries_gpu_53.rs` (A x6 + B x2 + C x1 = 9), `entries_gpu_54.rs`
  (D x3 + H x1 + M x1 + N x1 = 6), rust_perf `entries_10.rs` +2 measurement-honesty members
  (fallthrough-toggle-oracle, cold-l3-evict-min-of-n — the B177/B182 family precedent).
- [x] T4 — Registration + pins: mod.rs (mods/extends/category comments/KERNEL_CATEGORIES lists),
  tests.rs (batch186 parse gate + 655->670 x4 + per-category table A139->145 B69->71 C49->50
  D143->146 H29->30 M20->21 N51->52), router.rs:472, rust_perf pins (163->165, notbench 64->66,
  annotated 109->111 x3 sites).
- [x] T5 — GOAT gates: batch186 parse gate · lib default · kernel slice (--features kernel_heal)
  · rust_perf annotation pins (--features rust_perf) · retrieval floors 21/21 (isolated
  CARGO_TARGET_DIR) · clippy -D both postures · score-bench NOT owed (zero clippy-corpus delta,
  P30) · NO freeze regen (clippy corpus untouched).
- [x] T6 — Research 218 (`.research/.highwater` 217->218) + AGENTS.md (kernel_opt bullet +
  kernel_heal line + rust_perf bullet) + kernel_opt.md + rust_perf.md + README rows +
  `.distill/001_mining_queue_snapshot.md` landing row (same commit).
- [ ] T7 — Commit `feat: batch 186: ggml-cpu tiled mul_mat distill (17 rules)` + push; plan
  COMPLETE. No .raw cleanup owed (source read from the standing vendor clone — flagged-not-mine,
  left in place).

## Category ledger (16 kernel_opt + the routing calls)

| rule | cat | one-line |
|---|---|---|
| dequant-once-macrotile-stage-reuse-across-microtiles | A | unpack 256x256 int8 tile once, 256 microtile MACs reuse it |
| stage-operand-mac-layout-inplace-transpose-at-repack | A | VNNI [kg%16][kg/16][row][4] in-place butterfly transpose at stage |
| plan-time-thread-slot-scratch | A | per-thread 512KB slots in wdata, 0 when unsupported, ith-sliced |
| narrow-m-l1-panel-sizing | A | k_extent = 31KB/(16+rows), 256-floor + divisibility walk-down, 0 = refuse |
| jit-band-repack-stage-per-use-l1-hot | A | repack each 16-row band just-in-time (j0-outer), L1-hot |
| zero-pad-ragged-tail-additive-identity | A | pad tail rows to microtile with zeros; waste MAC < predication |
| maddubs-overflow-two-exact-mac-selector | B | Wmax<=64 act-bias maddubs vs sign trick (2*127*127<32767), const-selected |
| hoist-row-invariant-scale-to-subblock-epilogue | B | exact-int accumulation; d applied once per row at writeback |
| consumer-offset-absorption-precomputed-weight-row-correction | C | activation +128 at stage; corr = 128*Σs·w_bsum precomputed PER WEIGHT ROW (offset family's consumer-side member) |
| regime-batch-floor-force-escape | D | floor 8 in-callee + GGML_CPU_TILED_MM_FORCE bench-only escape |
| unified-admission-dead-tail | D | the live upstream defect: src1 gates after a total-return switch are DEAD — merge, never append |
| owned-input-transform-early-admission | D | consult at op top + own the F32->Q8_K convert (the iqp late-consult arc died) |
| moe-per-expert-mixed-dispatch-worst-case-workspace | H | per-expert floor on routed rows; hot experts tiled, cold vec_dot; worst-case plan reservation |
| const-generic-microkernel-format-unification | M | 13 formats -> ONE widened tile + const-generic kernel (iqp.cpp -1253L); NK=1/NK=0 second instance |
| widen-metadata-once-to-mac-native-widths | N | fp16->f32 d, u8->i32 scales/mins, i16->i32 bsums at unpack; no per-use cvt |
| fallthrough-toggle-oracle | rust_perf | same graph twice through one admission switch; magnitude-scaled tol |
| cold-l3-evict-min-of-n | rust_perf | 256MB flush + min-of-5 for bandwidth benches (warm/cold regime polarity) |

Staged (5): jmajor-acc-buffer-transpose-writeback (A) · register-budget-band-partition-indep-correction-offband (B) · chunk-downsize-work-stealing-microtile-floor (E) · lut16-pshufb-byte-expansion-at-stage (B) · gfni-sign-mask-expand-cmpeq-fallback (B).
