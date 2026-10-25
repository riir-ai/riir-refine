# Plan 286: Batch 282 — PerfR C-to-Rust translation perf classes → rust_perf

**Status:** COMPLETE — Batch 282 landed + pushed 2026-10-25 (see the snapshot landing row + Research 319)
**Session:** shikuwa-b282-claim, 2026-10-25

Batch 282 converts the staged B282 lead: **arXiv 2610.23044v1** ("Where Does the
Performance Go? Understanding and Recovering Performance Regressions in
C-to-Rust Translation", Yuan/Wang/Zhou/Liu/Peng — Fudan + NTU, Oct 16 2026).
The paper's nine recurring perf-issue classes (Table IV, 269 instances over 20
real C projects + c2rust translations) pair an Observed Pattern with an
Optimization Direction — the corpus drafter+pruner+verify shape; their
kept-only-if-validated two-gate acceptance IS the corpus verify discipline.

**Source pin:** arXiv 2610.23044v1 HTML (stable v1, full text fetched this
session); concrete instances name real upstream projects (bzip2 `BZ2_decompress`,
heman `fastFloor`, xxHash `XXH3_accumulate_512_scalar`, libzahl `zmul_ll`/`zrsh`,
miniz checksum) — quotes cite the paper §IV-C at the arXiv pin.

**Domain:** rust_perf (CPU-side source patterns; zero GPU surface).
**Expected yield:** ~3 own rules (pre-read LIKELY-NOVEL: class 2 saturating
conversion, class 5 invariant dispatch, class 9 raw-pointer view recovery);
rider-or-drop adjudication for the PARTIAL classes (3 vectorization, 4 inlining,
7 byte-oriented); drop-verify for LIKELY-EXISTS (1 bounds-check, 6 buffer,
8 chunk). Honest drops allowed.

**Safety classification:** classes 2 and 9 are UNSAFE-adjacent rewrites
(`to_int_unchecked`; `&[T]`/`&mut [T]` recovery from raw pointers) — the healer
divergence classes. Their rules land VERIFY-tier or suggest-only with mandatory
`context_notes`; never blind auto-fix.

## Tasks

- [x] T0: Claim row + this plan in one commit, PUSHED before authoring (`1904cfdb8`)
- [x] T0b: Full-text source read at the pin (arXiv HTML v1, this session — Table IV + §IV-C instances + §V-D optimization cards + §V-E two-gate validation + §VII-B applicability)
- [x] T1: Dedup grep each class against rust_perf (267) + kernel_opt with 3+ vocabulary variants; record verdicts (the enrichment's pre-read verdicts are the starting point, re-verify in-tree) — classes 6/7/8 DROPPED (arena/pool/copy-byte-order/chunk families own them), 1/3/4 → rider faces, 2/5/9 → own rules
- [x] T2: Author own rules via disjoint slices (entries_48 NEW, 3 rules; fixtures standalone-compiled + EXECUTED 6/6 green before landing; context_notes carry the proof obligations on the unsafe-adjacent pair; NO proposers — suggest-only by construction)
- [x] T3: Coordinator wiring (mod.rs mod decl + builder + header count; count pins via shape-asserted python passes ×9 sites incl. the annotated 214→217 triple + without_spec 146→149 + the batch265 witness; batch282 needle gate with the nine paper-class-name negative asserts)
- [x] T4: Gates — default lib 2730/0, rust_perf_bench lib 2761/0, clippy −D ×3 postures clean (4 doc_markdown PerfR backticks + 1 too_many_lines allow healed in-batch)
- [x] T5: Retrieval floors re-measure — latent_retrieval full lib 2916/0 (435 s), floors HELD no re-pin
- [x] T6: Freeze gen2/gen3 e2es run-to-confirm — both PASS, no table movement (P30 held: rust_perf-only delta moves nothing); score-bench #199 heal 0.9636 FLAT parse 1.0 + frontier snapshot appended
- [x] T7: Maintained binary rebuilt at the release set + installed; --info CORPUS serves rust_perf 270
- [x] T8: Research 319 (`.research/.highwater` 318→319) + AGENTS.md ×2 count pins + `.docs/11_domains/rust_perf.md` (header figure + Batch-282 row) + snapshot landing row, one commit
- [x] T9: Commit `feat: batch 282: ...` + push; verify `--version` stamp == landing HEAD

## Notes

- Enrichment row (`.distill/001_mining_queue_snapshot.md` L1) carries the nine
  classes with patterns/directions + pre-read verdicts + the paper's applicability
  numbers (rule 9: 89% of hot fns; 7: 35%; 3: 24%; 1: 18%; 4: 16%).
- Paper validation gates (paired interleaved runs, CV≤3%, Tukey 1.5×IQR,
  incremental 1.0% hot-fn-weighted + cumulative 0.3% aggregate + severe-regression
  guard, min 0.1% net) mirror rust_perf_bench Pareto + the Issue-940 determinism
  fold-order pins — cite as external validation in Research 319, not re-owned.
- Box state: 4090 quiet at claim time (GPU 5%, 332 MiB, no compute processes);
  zero-GPU batch throughout (CPU-class authoring + lib tests only).
