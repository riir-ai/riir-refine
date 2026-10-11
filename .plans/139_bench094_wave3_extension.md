# Plan 139 — Bench 094 wave-3 extension (pages 13/33/53/78/98 + merged t03)

**Status:** COMPLETE — wave 3 sampled + merged + joined; all tasks done 2026-09-17
**Date:** 2026-09-17
**Bench:** `.benchmarks/094_default_level_fired_histogram.md` (§Extend executed — wave 3)
**Driver:** `scripts/sample_default_fired_histogram.py` (the Plan-138 tracked driver; the
Extend protocol is explicitly rerunnable — this is its second extension)

## What

The second Bench 094 Extend run: one page deeper per stratum (pages
13/33/53/78/98 → ranks 1201-1300, 3201-3300, 5201-5300, 7701-7800,
9701-9800), same mechanical filters, same pinned toolchain (clippy 0.1.98 —
verified byte-identical against the t02 manifest BEFORE launch), merge
waves 1+2+3 into `.benchmarks/data/094_t03/manifest.json`, re-pin the
blake3 in the doc, re-run `cargo heal --coverage-report`.

## Tasks

- [x] T1 — pre-flight known-dirty control: akin-0.4.0 through the driver's
      own `download_crate`/`extract_crate`/`run_clippy` → exactly its 3
      recorded warnings (`empty_line_after_doc_comments`,
      `manual_inspect`, `let_and_return`). PASS.
- [x] T2 — sample 200 (base t02 for dedup; 499 candidates, 0 base-name
      collisions — fresh disjoint bands); 159 ok / 41 compile_error, 467
      warnings, 75 distinct lints. One launcher incident (MSYS nohup killed
      with its shell — resume path exercised for real) and one measurement
      incident (T4) both recovered.
- [x] T3 — merge → `094_t03` (population 580 sampled / **501 clean** /
      78 err / 1 timeout / **1801 warnings / 164 distinct lints**; blake3
      `dd1bebee542d787eca4c60649e309534f9de340d22604ed590559be730b14d77`);
      loader repoint `bin/heal.rs` REL → t03; `corpus_coverage.rs` module
      doc path + figures (also fixes the stale "452" the wave-2 docstring
      carried — the t02 manifest said 342).
- [x] T4 — the resume prefix-collision fix: the wave-2 resume repair
      (prefix-match) over-matched — done key `gsk4-sys-0.11.4`
      startswith(`gsk4-`) silently skipped the `gsk4` crate. Caught by the
      reconstruction diff (re-fetch, replay the cut, diff against the
      file); fixed to the exact constructed key `f"{name}-{ver}"` (O(1));
      the lost crate measured through the same driver functions and
      appended. Full record: bench doc §"The wave-3 measurement incident".
- [x] T5 — docs: bench doc wave-3 section + status line; AGENTS.md
      coverage-report bullet (path, population, flip chain, spread head);
      HISTORY entry; `.distill/001` post-mining intake entry.

## Outcome

**Flip: 94 of 442 → 110 of 442** default-firing uncovered lints with
evidence (+16). `needless_borrow` 53/20 → **92/30** (the widest-spread row,
+73%); `redundant_field_names` 58/9 is wave 3's fresh mining-grade row (37
of its count arrived with this wave). Extend floor held: 501 ≫ 150.
