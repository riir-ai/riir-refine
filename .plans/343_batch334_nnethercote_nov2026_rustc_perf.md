# Plan 343 — Batch 334: the nnethercote Nov-2026 rustc-perf post distill (rust_perf)

**Status:** STAGED — verdict landed (Research 372 + queue row, same commit); unclaimed; claim-before-edit per protocol; claim expiry 48 h from claim; push is the arbiter.

Source: nnethercote.github.io "How to speed up the Rust compiler in November 2026" (2026-11-04) + the named rust-lang/rust PRs. Discharges the standing "nnethercote ~11-01" author-scoped dated lead. No `.raw/` clone needed — the mine surface is the blog text + PR API reads. Zero-GPU throughout.

## Tasks

- [ ] T0: re-read the two lead PRs at the API (#163628 single-entry cache; #163857 span compression + #163971 span-op micro-opts) — re-verify every quote against the blog text + PR threads; carry the 10-17 pass's #163857 title-triage + the B326 (`adaptive-sparse-blocked-table-encoding`) counter-precedent into the rule-2 adjudication (drop honestly if the fidelity gap survives the full-diff read).
- [ ] T1: dedup re-verification at authoring — 3+ vocabularies over rust_perf (single-entry/last-key/lookup-memo/temporal + inline-pack/intern/skew/niche) + kernel_opt cross-check; every candidate graded in-row with its kinships named.
- [ ] T2: author `entries_66.rs` (rust_perf) — rule 1 `single-entry-last-key-lookup-cache-hash-avoidance` (caching category, BENCHABLE; the read-stable-window correctness constraint carried in-rule; kinships: entries_49 certifying-token, entries_29 range-table-page-memoization, entries_09 owner-lifetime memo, fxhashmap proposer, entries_08 interleaved slots) + rule 2 `skewed-payload-inline-encode-intern-overflow-tail` (layout category, suggestion-tier, bench_spec None per the size-axis class; the #164291 negative-result face carried as anti-regime honesty).
- [ ] T3: GOAT G2 on rule 1 — rust_perf_bench Pareto + exact alloc-count axis at `--release`; run-alone + bench_preflight PROVENANCE line; fixture: deterministic repeated-key sequence, memo-hit arm vs direct-map arm, identical result checksums asserted, alloc-free.
- [ ] T4: count pins — `pins_manifest --write` (rust_perf 300→301/302 + benchable/notbenchable deltas) + prose sites (AGENTS.md ×3, README ×3, `.docs/11_domains/rust_perf.md` batch block, tests.rs literals) via ONE shape-asserted script pass, never fuzzy edits.
- [ ] T5: gates — batch334 needles + parse gates (keyed AFTER reading the landed text); default lib; rust_perf_bench lib; clippy −D ×4 postures; retrieval floors pre/post attribution (A/B in the detached beside-repos worktree — the /tmp worktree path-dep trap lesson); score-bench appended; freeze NOT owed (zero clippy_lints delta, P30) unless clippy_lints sneaks in.
- [ ] T6: release-set binary rebuild + install at the pinned set (rust_perf IS release-set, the §0 bin-age law) — verify the exe MTIME moved, not the build log's Finished line.
- [ ] T7: fixtures standalone rustc-compiled AND executed BEFORE entry (both arms + negative arms; fixtures ORIGINAL — no rustc source pasted).
- [ ] T8: docs — Research 372 landing addendum; queue landing row (discharges the claim); AGENTS.md Role bullet if the count moved.
- [ ] T9: commit + push on develop (named paths only — sibling WIP untouched); claim row discharged by the landing row.

## Drops already adjudicated at verdict (never re-graded)

#164484 (LANDED Batch 326 — corroboration row only) · #164236/#164341 (invent-check ambient) · #164965 (loop-invariant collapse) · #165022 (DRY refactor) · #165112 (draft, compiler-pass machinery) · #163981/#164234/#165160 (ambient micro-opt dozen) · #163971 (positive-instance, folded as rule 2's face) · #164291 (negative result → rule 2's anti-regime).
