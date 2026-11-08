# Plan 355 — Batch 343: the sccache cache-layer tiered-backfill distill (Research 387)

**Status:** STAGED, UNCLAIMED — claim before any corpus edit per the queue discipline (claim line in `.distill/001_mining_queue_snapshot.md`, commit+push = the arbiter, 48 h expiry)
**Source:** mozilla/sccache @ `c1d7204f90a9ed1d88e6a8be39e2b8dab7ecfcd6` (Apache-2.0) — the cache-layer files B324 did not mine (full provenance + quotes: Research 387)
**Verdict:** YES B− corpus — 3 candidates (1 B, 2 B−) + 1 expected-drop, all suggestion-grade NotBenchable-by-domain (criterion 1 fails DOMAIN-side: I/O read-amplification and correctness classes live outside the CPU Pareto validator — the B324/B136 standard)

- [ ] T0 — claim row in the queue snapshot (Batch 343 head) committed+pushed BEFORE authoring; re-clone at the pin if the campaign clone moved; byte-verify every Research-387 quote against the pin (`multilevel.rs` :491-526/:571-601, `disk.rs` :126-136, `lru_disk_cache/mod.rs` :367-409/:257-288/:75-95)
- [ ] T1 — dedup re-greps at landing HEAD (3+ vocabularies per candidate; the `reserve-then-commit-capacity-accounting` name collision adjudicated IN-ROW — scan-time container capacity vs cache-capacity accounting)
- [ ] T2 — author `entries_68.rs`: `raw-bytes-with-hit-backfill-once` (B) · `advertised-size-staged-write-reconciles-at-commit` (B−) · `vanished-entry-tolerant-eviction-panic-else` (B−); every entry full `context_notes` with regime + anti-regime (oversized-entry streaming arm; fixed-size skip arm; peer-vs-self error classification)
- [ ] T3 — ORIGINAL Rust fixtures (hierarchical in-memory cache reproductions — zero sccache code pasted, license hygiene); standalone-compiled AND executed BEFORE entry (`corpus/fixtures/b343_fixtures.rs`)
- [ ] T4 — gates at isolated `CARGO_TARGET_DIR=/tmp/b343_target`: needles+parse, rust_perf lib, default lib, clippy −D × postures, retrieval floors both lanes (NO re-pin expected — suggestion-grade additions to an existing family)
- [ ] T5 — count pins via the shape-asserted python pass (rust_perf 304 → 307 expected, re-derived at landing); `pins_manifest.py` verify + prose agree
- [ ] T6 — freeze NOT owed (zero clippy_lints delta, P30); score-bench NOT owed (all three NotBenchable — benchable pin unchanged); release-set binary NOT owed (corpus-only delta, B339/B340 precedent)
- [ ] T7 — dual_allocation gate at the landing commit; docs (AGENTS/README/domain counts) + landing row in the queue snapshot + `feat: batch 343: ...` push
- [ ] T8 — `.raw/mozilla-heal/sccache` lifecycle: SHARED with the #2968 campaign lane — leave in place until that lane closes (record the exception in the landing row; the B340 rm-at-landing law is waived HERE by the shared-pin disclosure)
- [ ] T9 — outbound-PR: none (the laws are architectures, not bounded fix forms — no bounded self-contained fix exists; the B340 T9 precedent applies verbatim)

**Claimed:** — (unclaimed; the next idle lane claims via the snapshot row)
