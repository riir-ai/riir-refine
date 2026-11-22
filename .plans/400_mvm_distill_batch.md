# Batch 400 — the mvm (Magni VM) distill → kernel_opt corpus

**Status:** LANDED — `feat: batch 400` commit + push (T0–T8 done; the lease folds in the landing commit)
**Claimed:** zed-m3-mvm-b400, 1795381511, expires 2026-11-25T21:05Z

Executing the staged mvm distill verdict (snapshot row 2026-11-21, session
zed-m3-mvm-distill; controlling record: `../riir-infer/.research/36_*.md`, PIN
`60c42deb5628480d62a50d53296e5f8be77e9030`). Source: github.com/Bobliuuu/mvm —
Magni Community License 1.0, source-available free ≤$1M/research tiers:
**transcription-only GO, outbound-PR FAIL-CLOSED** (per-company revenue-gated
license — every row decision-content).

Queue-head claim per the snapshot's standing rows: this batch discharges the
mvm staged row; pool slices (1) mlxfast-bonsai2 re-visit and (3) the vendored
ml-explore/mlx tree stay unclaimed after it.

- [x] T0 — re-clone Bobliuuu/mvm (shallow, single-branch) into `.raw/mvm`;
      re-pin at the tip — **UNMOVED**: HEAD == `60c42deb` (the verdict pin,
      main tip 2026-11-19T16:55Z), the two-pin diff EMPTY, every staged
      quote byte-exact at the same SHA; spot-verified the load-bearing
      anchors anyway (FRAMEWORK.md:84-97 + cuh:5-36 + README.md:329/331/332
      + the adaptive/doom result READMEs + REVIEW.md C2 + the litmus
      controls table).
- [x] T1 — dedup re-verification at the live corpus (greps + in-row
      rulings recorded in Research 435): R1 set = gpu_138 (reuse-fence vs
      version-edge, compose) + gpu_61 (ring overwrite, cited) + gpu_85
      (cost axis) + entries_86 (liveness vs static refusal) + gpu_148
      epoch echo (recorded); R2 = gpu_50 (cap+overlap vs custody,
      compose) + entries_74 (class-share vs custody split) + entries_83
      singleflight (no custody axis); R4 = gpu_148 + singleflight; R5 =
      the regime-split annotation class.
- [x] T2 — authoring: R1 LANDED (A); R2 LANDED (E) after the gpu_50
      adjudication survived; R3 = count-neutral FACE on rust_perf
      entries_74 (the admission-FIFO measured instance); **R4 + R5
      honestly DROPPED** at authoring (thin rows — Research 435 carries
      the drops + the riders: MVM_MIN_BLOCKS register-cap record with the
      unverifiable +40-60% figure NOT quoted, the dispatch-ceiling
      corroboration, the gpu_86/gpu_60 boundary neighbors).
- [x] T3 — fixtures: `b400_fixtures.rs` 16 arms **standalone-compiled AND
      executed BEFORE entry at debug AND -O, zero warnings**; the gate
      caught FIVE arm defects (ids-vs-program-order, the parked-budget
      over-gate, the hardcoded need, the Held-borrow guard, the edge
      sequencing) — all fixed in the fixture AND synced into the entries'
      ref/opt shapes; the entries_74 face arm in the rust_perf corpus
      tests (count-neutral, `batch400_face_landed_on_writeclass_host`).
- [x] T4 — gates: lib **3960/0** (+3 batch-400 tests); clippy `-D
      warnings` clean at default / kernel_opt / rust_perf /
      rust_perf_bench / no-default (the `--all-targets` run carries 4
      PRE-EXISTING B399 ane/corpus/tests.rs findings, stash-verified NOT
      this batch's, disclosed); retrieval floors BOTH lanes **30/0 +
      30/0** held no re-pin; fmt gate PASSED (one unpinned router.rs
      drift formatted same session); freeze/score-bench NOT owed (zero
      clippy_lints delta); G1 store regenerated 1107 rows (276 green / 4
      FAIL pre-existing-at-HEAD stash-verified / 827 parse-only; my 2
      rules parse-only, the fixture FILE is the executed evidence).
- [x] T5 — count pins one-landing: `pins_manifest.py --write` (kernel_opt
      1105→1107, the ONLY manifest move); AGENTS §Role + README +
      `.docs/11_domains/kernel_opt.md` + health_views (kernel 1107, total
      1768) + router total + the rust_perf narrative chains + the A/E
      category headers + the MemoryAccess 210→211 / Parallelism 99→100
      test pins.
- [x] T6 — research note `.research/435_mvm_distill_batch.md` (the batch
      record: the landing, the honest drops, the riders, the gate
      receipts, the dedup evidence, the license posture).
- [x] T7 — teardown: `.raw/mvm` REMOVED at close.
- [x] T8 — landing: commit `feat: batch 400: ...` + push; snapshot LANDED
      row in the same commit; `dual_allocation_gate` + `numbering_gate`
      clean; binary rebuild NOT owed (corpus-only delta — no rulewright
      source touched, disclosed).

Standing gates this batch does NOT owe (pre-registered): score-bench /
freeze-table regeneration (zero clippy_lints delta); outbound-PR lane (license
fail-closed); GOAT Pareto bench (NotBenchable-by-domain — the CUDA host we do
not run; the Batch-136 annotation class).
