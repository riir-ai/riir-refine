# Plan 323 — Batch 323: the 10-30i + 10-30j staged leads conversion (kernel_opt)

**Status:** COMPLETE — all tasks [x]; landed (the landing commit is the provenance; claim `59b1ba69c`, Research 360, Session zed-m3-b323). Two sweep windows, one batch — the b322 pool shape (10-30g + 10-30h there, 10-30i + 10-30j here). Sources: the two STAGED LEADS ADDENDUM rows in `.distill/001_mining_queue_snapshot.md` (10-30i by zed-m3-leads1031c `9ec5019a1`; 10-30j by zed-m3-leads1031d `c24e59a30`). Plan-337 posture: the i1117 rho legs hold ~4.5 cores — T0 deep reads + authoring are zero-cargo; fixtures standalone rustc; gates at the isolated `/tmp/b323_target` nice'd limited-j under the legs.

## Phase 0 — T0 verification (subagents, blobless clones, byte-verify every quote)

- [x] T0.1 Clone `.raw/b323/{vllm,tensorfold,prism-fork}` at the pinned tips (vllm `d90e35cf`, tensorfold `316b3879`, prism fork master `02447444`; GIT_TERMINAL_PROMPT=0 throughout — the lithos lesson). Verify `git rev-parse HEAD` per clone.
- [x] T0.2 Subagent TF (S-i1, S-i2, S-j2, S-j3, S-j4, S-i3 rider, S-i4 face): byte-verify every quoted line against the real files at the named commits; return per-lead VERIFIED/DRIFT/REFUTED + license header check (TensorFold = Apache-2.0 per b322's correction).
- [x] T0.3 Subagent vllm (S-j1, S-j5, S-j7 face): byte-verify quotes at `6c19299fc` (#64509) and `e31cf3dec` (#64138); Apache-2.0 WITH upstream MIT attribution travels (b322 posture).
- [x] T0.4 Subagent prism-fork (S-i5, S-i6, S-i7 riders, S-i8 watch): byte-verify at fork master `02447444`; league intel is NOT corpus — only the atoms are.
- [x] T0.5 Adjudicate T1-grade calls: S-j5 face-on-gpu_51 vs own rule (the unowned atom is the CONVERT-AT-KEY-SITE placement); S-i5 composition-face-on-gpu_49/B11 vs new rule (the new atom is the ACTIVATION-side plane layout keyed to the weight's bit-serialism); S-j1 atom split: (a) rounding-reproduction = face on the gpu_99/gpu_12 family, (b) block-keyed index space = own rule candidate.
- [x] T0.6 Honest drops allowed (the oxc-negative precedent): any lead whose quotes drift or whose atom dissolves on read → drop with the reason recorded in-row.

## Phase 1 — dedup (before any entry)

- [x] T1.1 3-vocabulary grep per candidate over `src/domains/kernel_opt/` (the sweep rows' named kin + 2 more variants each): S-i1 vs gpu_143 `first_max_nan_never_wins` (opposite axis — verified distinct in-row, re-grep anyway); S-i2 vs gpu_39 expert-major-tile-order; S-j1 vs gpu_99/gpu_12 + S-i6; S-j2 vs entries_gpu_98/gpu_01/gpu_09 thread_elements+simdgroup hits; S-j3 vs the coherence-scope family (lithos R1); S-j4 vs cpasync/ICB/accumulator families; S-j5 vs gpu_51 `dtype-native-source-read-with-shared-admission-predicate`.
- [x] T1.2 Record one evidence line per candidate; a new hit = re-adjudicate before authoring.

## Phase 2 — entries (`entries_gpu_147.rs`; fixtures are ORIGINAL minimal Rust reproductions — never paste source)

- [x] T2.1 S-i1 `nan-preserving-select-over-fmaxf-in-scale-reductions` (cat N): comparison-select keeps NaN; `fmaxf` launders to 0; amax reduction `max_nan` form so a NaN block gets a NaN scale.
- [x] T2.2 S-i2 `banded-launch-order-shares-weight-tiles` (cat D/L): launch order is a pure permutation; consecutive row-blocks walk column blocks together for L2 weight sharing; output byte-identical.
- [x] T2.3 S-j1 `block-keyed-index-space` (cat D; the (b) atom — pending T0.5 split): index lattice mirrors the quant format's (block, pair, lane) nesting; reductions/packing become axis moves, zero reshuffle. The (a) atom rides the face lane if T0.5 grades it a face.
- [x] T2.4 S-j2 `simdgroup-fragment-whole-store-generation-keyed` (cat D/L, Metal): element-by-element fragment stores are a codegen cliff on M1/M2-class; store each lane's pair WHOLE via vec reinterpret; ADMISSION PER-GENERATION (13/14 whole, later by element).
- [x] T2.5 S-j3 `stream-through-load-no-l1-allocate-once-read` (cat D): buffer class keyed on REUSE (stream-through vs revisited), never read-only-ness alone; `L1::no_allocate` answers the reuse half.
- [x] T2.6 S-j4 `inner-loop-select-division-shift-mask-vgpr-pressure` (cat D/L): select/division in the K-loop body is a register-pressure cliff before an ALU cost; shift/mask + uniform branch when the divisor is static.
- [x] T2.7 Count-neutral faces (compose, never merge): S-i3 → B309 `runtime-trip-count-defeats-unroll-template-the-bound` (issue-all-loads-before-sums rider); S-j7 → gpu_111 pdl-admission family (dependents-BEFORE-loads, 2nd instance); S-i4 → gpu_146 face (pinned-staged packing instance); S-i6 → rounding family face (tail-width downgrade as refusal arm) pending T0.5; S-i7 riders (PDL→gpu_111, Hadamard→gpu_120, CTA-count→B310, fusion-context geometry).
- [x] T2.8 Every entry carries `context_notes` provenance (repo:commit:file at the verified pins + the sweep-row reference).

## Phase 3 — fixtures + standing gates

- [x] T3.1 Fixtures standalone-compiled AND executed BEFORE entry (the Batch-309 convention; standalone rustc, no cargo — Plan-337 light).
- [x] T3.2 Extend the standing syn-parse gate + needles test (`batch323_*` in `tests.rs`, mirroring `batch322_*` shape).
- [x] T3.3 Count pins: the shape-asserted python pass over tests.rs + router.rs + the Plan-003 expected-array rows + KERNEL_CATEGORIES headers + member lists + sum-comment chain + AGENTS/README/kernel_opt.md (corpus 1044 → 1044+N).

## Phase 4 — GOAT gates at `/tmp/b323_target` (nice'd, limited-j, under the i1117 legs)

- [x] T4.1 G1: batch323 needles + parse green.
- [x] T4.2 G2: kernel_opt lib tests green (isolated target; record wall time + box state).
- [x] T4.3 G3: clippy −D at the standing postures (default / kernel_opt / all as b322 ran) — 0 findings.
- [x] T4.4 Retrieval re-measure: `kernel_opt,latent_retrieval` retrieval floors HELD (28/0) + Layer 5b2 default posture (27/0). A decay FAILS the batch.
- [x] T4.5 dual_allocation gate 0/0/0; rustfmt clean in my regions.
- [x] T4.6 Freeze tables + score-bench NOT owed (P30 kernel_opt-only); binary rebuild NOT owed (kernel_opt outside the release set) — verify both exemptions still hold at landing (no rust_perf/sec files touched).

## Phase 5 — landing

- [x] T5.1 Research note `.research/360_*.md` (next free number — re-scan at write).
- [x] T5.2 AGENTS.md §kernel_opt + README + `.docs/11_domains/kernel_opt.md` count/status bumps.
- [x] T5.3 Snapshot landing row + `.raw/b323` REMOVED at close.
- [x] T5.4 Commit `feat: batch 323: ...` + push; Session trailer in the commit body.
