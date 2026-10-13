# Plan 234 — Batch 232: the TensorFold 1.0.9 leads conversion (intake (1a)+(1b))

**Status:** COMPLETE (Batch 232 landed — see the snapshot landing row for the commit)

Source: ashhart/TensorFold — (1a) `c724dd9d` "Nemotron CUDA: a reply's background pass
waits for the next request, then runs short chunks" (MIT/Apache-2.0 per repo) + (1b)
`b264f534` "The 27B's Neural Engine share loads only inside the 70% load limit" + (1c)
the v1.0.9 release `72b45368` changelog read (league intel, no rule — the DeltaNet-state-copy
elimination + prefix-kernel dispatch fold are owned by existing state-reuse/dispatch families).
Quotes verified verbatim at the fetched .patch files + the release body this session
(/tmp/tf_1a.patch, /tmp/tf_1b.patch, /tmp/tf_rel.json — ephemeral, removed at close).

## Tasks

- [x] Claim row pushed BEFORE authoring (`b1682e12`; incl. the .research/.highwater repair 233→262)
- [x] Quote re-verification at the pins (both commit messages + in-patch: `warm_quiet`,
  `yield_rows = 256`, the `.waits` warm mode, `prefill_rows = 2048` exception, the quiet-hold
  cancel/drop line, `fitsLoadLimit` + the 32/48 GB test rows + the fail-closed unknown-RAM arm)
- [x] Dedup grep: B182 `verify-window-prework-loads-before-stores` = INTRA-kernel ordering
  (entries_gpu_60 kinship lines confirm) → (1a) UNOWNED, own-rule grade confirmed; (1b) homes
  adjudicated: `degrade-not-fail-load-qualification-probe` (entries_21, the structural twin —
  load-time admission, two-class outcome, loud degradation) over thermal-hysteresis (the
  runtime-axis sibling, cited inside the rider text) and over prefault-admission B139
  (page-fault eager-vs-lazy axis, not accelerator-copy admission)
- [x] Subagent A: the OWN kernel_opt D-class rule `warm-background-pass-quiet-hold-yield-chunking-cut-invariance`
  in NEW `src/domains/kernel_opt/corpus/entries_gpu_91.rs` (full house style; verbatim needles)
- [x] Subagent B: the count-neutral RIDER on rust_perf `degrade-not-fail-load-qualification-probe`
  (entries_21.rs context_notes — the HEADROOM-ADMISSION face: fitsLoadLimit, the served-host
  70% limit + 2 GiB margin, degrade-loud "runs prompts on the GPU alone and says so", the
  direct-loader carve-out, the swap-economics rationale, fail-closed unknown RAM)
- [x] Coordinator wiring: mod.rs `mod entries_gpu_91` + extend + the D-category list append +
  count pins (804→805; D 188→189; the category doc comment + the exhaustive-invariant doc line)
- [x] tests.rs: `batch232_rules_parse_and_needles_landed` (count 805; the (1a) needle set;
  the (1b) rider needles on the entries_21 host — pinned by the rust_perf corpus test pattern)
- [x] G1: batch232 needle+parse test green; rust_perf corpus tests green
- [x] G2: kernel_opt lib tests green (isolated CARGO_TARGET_DIR) + default lib count unchanged+1
- [x] G3: clippy -D warnings at default + all-features (both domains touched)
- [x] Retrieval floors: kernel self-variants 100.0% + blended 91.2/94.7/95.3 held (no re-pin expected)
- [x] No freeze regen owed (zero clippy-corpus delta, P30); no score-bench owed (batch-229 precedent)
- [x] Research note `.research/263_batch232_tensorfold_109_leads.md` (born-complete, Status: DISTILLED)
- [x] Docs: AGENTS.md kernel_opt 804→805 + rider note; `.docs/11_domains/kernel_opt.md` B232 row;
  rust_perf.md Batch 232 rider line; snapshot landing row; intake rows (1a)/(1b) → [x]
- [ ] Commit `feat: batch 232: ...` + push (the claim discharged by the landing row)
