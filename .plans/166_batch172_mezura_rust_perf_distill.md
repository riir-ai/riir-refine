# Plan 166 — Batch 172: mezura rust_perf distill (dual-mask block scan + mmap admission + crossbeam batch-steal)

**Status:** OPEN — verdict recorded (Research 202), awaiting execution

Source: subamanis/mezura @ `2dbacb756d2169d2239bcdfbe9391fcd2ed9389b` (MIT OR Apache-2.0). Verdict: YES B+ (Research 202). Route: `rust_perf` — simd/io/concurrency. Lead: the dual-mask fused block scan (GOAT 4/4). Yield estimate: 2–3 rules after coordinator falsification.

## Tasks

- [ ] T1 Re-clone `.raw/mezura` at the pinned sha; re-verify every quote/citation in Research 202 against the tree (sibling-drift discipline; the clone was deleted after the verdict commit)
- [ ] T2 Read the files marked unverified in Research 202 caveats (`producer.rs`, `identity.rs`, ScanPlan consumers in `file_parser.rs` L445–1000) — grade any NEW candidates before corpus edits; amend Research 202 in the same commit if grades move
- [ ] T3 Dedup re-verification at mine time (the queue snapshot's standing rule): re-grep `riir-clippy/src/` under both vocabularies for candidates 1–6 (a fresh batch may have landed since 2026-09-22)
- [ ] T4 Author candidate 1 `dual-mask-fused-block-scan` (simd, suggest-only): ORIGINAL fixtures (no mezura verbatim — write our own AVX2 2×32 + NEON 4×16 + scalar arms; the vector==scalar exhaustive-shape test is part of the fixture); bench spec sizes 4 KiB/64 KiB/256 KiB/1 MiB, source-like synthetic buffer, per-line `split('\n')`+`find` buggy arm vs fused-mask fixed arm; dual-arch gate (M3 NEON + 4090 AVX2)
- [ ] T5 Adjudicate candidate 2 (anchor-byte greedy set-cover): standalone rule vs B67 rider — argue the SELECTION algorithm (greedy min-cover, letters disfavoured, anchor-behind-found-byte) as the distinct half; drop to rider if the coordinator reads the arity ladder as covering it
- [ ] T6 Author candidate 3 `mmap-settled-size-gated-read` (io, suggest-only, honestly NotBenchable — page-cache state): distinct-from notes owed to prefault-admission (B139), mincore-probe (B145), parallel-disjoint-pread (B118), batch-pool-readahead (B123); the settle guard + expected+1 protocol are the load-bearing halves; keep the short-read-on-failing-disk honesty
- [ ] T7 Adjudicate candidates 4–6 (steal-batch, oversubscription, capacity clamp): expect 0–1 to land standalone (niche dep / NotBenchable / metric-mismatch); the clamp likely folds as a `reuse-allocation-clear` rider
- [ ] T8 Standing gates: corpus count pins (rust_perf 145→146/147/148), `tests.rs` category-count updates, lib count floors
- [ ] T9 Freeze-table check: measure whether the delta moves `build_seed_corpus()` stride windows; if yes, regenerate gen2/gen3 freeze tables in the SAME batch (Batch-151 law)
- [ ] T10 GOAT: G1 bit-identical positions (exhaustive vector==scalar shapes incl. every byte value × positions, short-block edges); G2 Pareto verdict dual-arch (expect ParetoDominant; a Mixed/Tie on either arch is recorded honestly); G3 no-regression on the sibling simd benches; G4 alloc-free fixed arm (mask u64s only)
- [ ] T11 Retrieval floors: `--score-bench` run appended (#N), heal_rate flat-or-better, `rule_outcomes` rows for the new rules
- [ ] T12 Docs: AGENTS.md Batch 172 entry + rust_perf count bump; Research 202 Status → RECORD; queue snapshot line → LANDED; this plan Status → COMPLETE
- [ ] T13 Commit+push (one commit: corpus + tests + benches + docs + queue + research-status), rebase non-interactive; `rm -rf .raw/mezura` after landing (a landed batch with a live `.raw/` entry is an unfinished batch)

## Defers

- [-] Lane-intel artifacts: none owed — no lane mapping (recorded in Research 202; nothing to file in riir-ai).
