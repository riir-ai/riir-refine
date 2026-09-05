# Plan 088 — qwen-3.8-mtp-challenge submission-diff walk → Batch 115

**Status:** OPEN — plan-first, created 2026-09-05 before any corpus edit.

**Source:** `Layr-Labs/qwen-3.8-mtp-challenge` @ tip `0863b06` (Apache-2.0 harness; ~50 submission diffs 08-17..08-23, each a scored, reviewed Metal-kernel patch). **Verdict:** Research 139 — YES B+ as a diff-history mine (corrected same-day from the erroneous "dedup-closed" line; the tip-tree B100 mine is complementary). **Geometry:** Qwen 3.8 27B = 64L hybrid, hidden 5120, vocab 248320 — Bonsai-exact shape class. **Lane:** kernel_opt + the measurement-honesty axis (B109 family).

- [ ] T1 clone + pin: `git clone --filter=blob:none` (full history — the diffs ARE the data) into `.raw/qwen38-mtp`; verify `rev-parse HEAD == 0863b06ac16e26e48fc06e97444095b00feb66d4`; record sha + license in the batch record.
- [ ] T2 enumerate + classify: `git log` over 08-17..08-23; per Accept/Validate commit record: surface (seed-prefill / decode-kernel / draft-head / schedule-apparatus / manifest-only), LOC, manifest-note technique + repricing.
- [ ] T3 two-lane read: **Lane A = seed-prefill-touching diffs FIRST** (the owner's question; prefill is charged inside the arena's timed window). Lane B = decode-kernel + draft-head kernels (seed list in Research 139: two-level centroid index, nibble digit-scale fold, in-kernel offset sum, one-dispatch gather+reduce, dead-JIT factory skip, env-sanitizer gate hazard, head-trim coverage guard).
- [ ] T4 dedup greps per candidate, both vocabularies; coordinator re-verifies every quote at the pin (B51 lesson: a subagent's failed grep is not evidence of absence).
- [ ] T5 author entries — target 3–6 rules + 4–8 riders after falsification; fixtures ORIGINAL (no source text in corpus); ported-numeric candidates get the exhaustive equivalence gate (B59 mandate).
- [ ] T6 standing gates + count pins walked (corpus header, expected-array, sums, router, mod.rs doc); GOAT G1–G5 + G-det; retrieval floors; score-bench flat.
- [ ] T7 bookkeeping: Research 139 Status → RECORD; snapshot line + AGENTS.md corpus counts; `.raw/` removed at landing.
- [ ] T8 commit + push on `develop`.

**Queue position:** behind Batch 114 (oMLX ANE/GDN — the other prefill lane). Owner may reorder; the Bonsai-exact geometry argues for priority over non-prefill queue items.
