# Plan 242 — Batch 240: the P67–P70 conversion (kernel_opt +3)

Status: COMPLETE (landed 2026-10-15; corpus 813 → 816; see Research 271 + the snapshot landing row)

Source set: the 2026-10-15 watch poll's staged rows (`.distill/001` top row, commit
`22fe7777c`) — llama.cpp envelope `e467e5b5e5ba` → `e36df604` window:

- **P67** `9166ceaf` #30421 "opencl: fix OOB tail access for adreno noshuffle gemv
  kernels" (MIT) — RULE-GRADE A−. The subgroup TAIL-LANE law: a subgroup reduction
  needs EVERY lane participating, so an M-not-divisible grid runs tail lanes whose
  row index is OOB — the lane stays LIVE but its loads clamp to a valid row
  (`gid_s = min(gid, LINE_STRIDE_A - 1)` at every weight/scale read site) and its
  stores gate on the real bound (`if (gid < LINE_STRIDE_A)`; scalar `gid*2+n < M`
  guards on the o4/pair splitk partial stores). Host: kernel_opt, **category E
  (Parallelism** — the lane-participation contract).
- **P68** `23f83e05` #30526 "ggml-cpu: optimize pad_f32" (MIT) — RULE-GRADE B+.
  ROW CLASSIFICATION as the pad op's loop structure: one thread per row
  (`row = ith; row < nrows; row += nth`), interior rows (all pad dims inside the
  row bounds) take the `memset(lp0) + memcpy(ne00) + memset(rp0)` triple when the
  source row is contiguous (`src0->nb[0] == sizeof(float)`, the
  `src_row_contig` hoist), boundary rows stay scalar; the circular variant keeps
  its full-row wrap-around path. Host: kernel_opt, **category A (Memory access)**,
  OWN RULE (adjudicated: `ragged-chunk-pad` owns the pad-vs-bound STRATEGY +
  staging symmetry; `zero-pad-ragged-tail-additive-identity` owns the additive
  economics; the B226/B195/B213 ladder owns where producer zeros COME FROM — the
  row-classification + contig-gated memcpy + row-threading face has no owner).
- **P69** `5a638aed` #29200 "cpu: add F16 SWIGLU_OAI reference implementation"
  (MIT) — RULE-GRADE B. The f16-ACTIVATION-UPCAST-AT-FOREIGN-GRAPH-TRANSLATION
  law; the commit cites its own in-tree precedent ("translate_glu_swiglu_clamp
  already upcasts to f32 and converts back at the end; apply the same pattern
  here") — the second instance of a pattern the source already carried. Host:
  kernel_opt, **category B (Compute** — precision upcast).
- **P70** `733f57a9` #27703 "vulkan: copy strided f16 KV once on AMD" (MIT) —
  RULE-GRADE B / WATCH. The MEMORY-CHANNEL-SPREAD law. Staged WATCH-strong-B:
  "convert only if a lane finds a Rust-side host". Adjudication at T6: grep our
  trees for a strided-KV-read host; absent → watch-only, recorded in Research 271.

Corpus: kernel_opt 813 → **816** (A 166→167, B 87→88, E 76→77). Count-neutral
elsewhere. Freeze/score-bench NOT owed (zero clippy-corpus delta, P30).

## Tasks

- [x] T0 claim row spliced into `.distill/001` + this plan + highwater bumps (241→242 plans, 270→271 research), committed + PUSHED before authoring (claim `47cb067e3`, accepted first try)
- [x] T1 quote verification at the fetched `.patch` files (all three + P70), verbatim needles recorded (all verified; P68's staging phrasing tightened: `!interior` rows = whole-row memset, not "scalar")
- [x] T2 dedup greps (subgroup/tail-lane/clamp vocab; pad/memcpy/row-class vocab; upcast/translation vocab) — zero-owner confirmation + kinship ids verified present (tail-clamped-row-reuse, masked-boundary-load, role-scoped-store, warp-shuffle, ragged-chunk-pad, pad-tail-to-fixed-tile, zero-pad-ragged-tail, the popcorn upcast trio, dot-macro-family)
- [x] T3 P67 entry authored (entries_gpu_96.rs, category E) — subgroup-tail-lane-clamp-live-loads-gate-stores
- [x] T4 P68 entry authored (entries_gpu_96.rs, category A) — pad-row-classification-interior-memcpy
- [x] T5 P69 entry authored (entries_gpu_96.rs, category B) — f16-upcast-at-foreign-graph-translation
- [x] T6 P70 watch adjudication (no Rust-side host — our KV reads tiled by design; disposition in Research 271)
- [x] T7 wiring: mod.rs (KERNEL_CATEGORIES slices + category comments + exhaustive doc), router.rs expert-total, tests.rs (master count, Plan-003 sums, grouped total, per-category rows, historical batch asserts re-pin — 15 sites, FIVE more surfaced by the suite) + batch240 needle/parse gates
- [x] T8 G1: batch240 gates green (22 needles + pseudocode parse ×3; one em-dash needle drift caught + repaired)
- [x] T9 G2: kernel_opt lib 3122/0/1i + default lib 2670/0 (isolated CARGO_TARGET_DIR=/tmp/riir-refine-b240, removed at close)
- [x] T10 G3: clippy -D clean (default lib + kernel_opt --all-targets + no-default --all-targets + all-features lib; 3 doc_markdown healed en route; 1 PRE-EXISTING bin/heal.rs TtyKeys warning at all-features --all-targets disclosed, verified at HEAD)
- [x] T11 retrieval floors HELD (21/21, 252.93s, no re-pin)
- [x] T12 docs: AGENTS.md ×2, .docs/11_domains/kernel_opt.md (head + batch row), README, Research 271 (born-complete RECORD)
- [x] T13 intake hygiene: P58–P61 rows ticked CONSUMED (M3 Batch 237; the three rule ids + rider sha verified in-tree first)
- [x] T14 landing row (snapshot) + plan all-ticked + commit + push

En-route graded (no action): envelope `e36df604 → 47f5a64ae` = ONE commit
#29766 vulkan concurrent first-device-init race — NULL (backend-lifecycle
correctness, no kernel/perf law; the new envelope pin is `47f5a64ae`).
Watch legs prism tip / prism-b ladder 10777 / cubecl ≤v0.11.0 — ALL NO-FIRE
fresh this session (2026-10-15 15:43 +07).
