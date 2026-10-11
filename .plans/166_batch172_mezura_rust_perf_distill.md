# Plan 166 — Batch 172: mezura rust_perf distill (dual-mask block scan + mmap admission + crossbeam batch-steal)

**Status:** COMPLETE — Batch 172 LANDED 2026-09-23 (4 rules + 2 riders + 1 drop; T1/T2 pre-executed 2026-09-22, T3–T13 executed at the past-midnight cadence slot; landing record: HISTORY.md 2026-09-23 §Batch 172 + Research 202 close-out; `.raw/mezura` removed at T13)

Source: subamanis/mezura @ `2dbacb756d2169d2239bcdfbe9391fcd2ed9389b` (MIT OR Apache-2.0). Verdict: YES B+ (Research 202). Route: `rust_perf` — simd/io/concurrency. Lead: the dual-mask fused block scan (GOAT 4/4). Yield estimate: 2–3 rules after coordinator falsification.

## Tasks

- [x] T1 Re-clone `.raw/mezura` at the pinned sha; re-verify every quote/citation in Research 202 against the tree (sibling-drift discipline; the clone was deleted after the verdict commit) — **DONE 2026-09-22: ALL citations verified at `2dbacb7`, zero drift (Research 202 addendum)**
- [x] T2 Read the files marked unverified in Research 202 caveats (`producer.rs`, `identity.rs`, ScanPlan consumers in `file_parser.rs` L445–1000) — grade any NEW candidates before corpus edits; amend Research 202 in the same commit if grades move — **DONE 2026-09-22: one new candidate 7 `O_NONBLOCK probe open` B− dedup-PASS; candidates 4/5 strengthened; five annotations (Research 202 addendum)**
- [x] T3 Dedup re-verification at mine time (the queue snapshot's standing rule): re-grep `riir-clippy/src/` under both vocabularies for candidates 1–6 (a fresh batch may have landed since 2026-09-22) — **DONE 2026-09-23: PASS on all 7 candidates (only known-family hits: B67 prose, the B118 mmap-escape note, B139 prefault)**
- [x] T4 Author candidate 1 `dual-mask-fused-block-scan` (simd, suggest-only): ORIGINAL fixtures — **DONE: both AVX2+NEON arms + scalar contract + the exhaustive G1 gate (16,384-combo sweep; caught the \n-in-needle-set contract subtlety); bench 4 KiB/64 KiB/256 KiB/1 MiB; MEASURED ParetoDominant 4/0/0 verdict-stable ×2 — and the NEON runtime re-gate DONE on M3 2026-09-23 00:1x +07 (G1 NEON-arm PASS + ParetoDominant 4/0/0 ×2, alloc 0/0, G-det ×3; magnitudes loaded-box lower bounds; Research 202 close-out carries the table)**
- [x] T5 Adjudicate candidate 2 — **DONE: RIDER on B67 (byte-find-position-to-memchr); the selection algorithm joins B67's notes; B67 flipped None→Some**
- [x] T6 Author candidate 3 `mmap-settled-size-gated-read` — **DONE: NotBenchable (page-cache state); settle guard + expected+1 sentinel + Windows DirEntry freebie; distinct-from paid to B118/B123/B139/B145**
- [x] T7 Adjudicate candidates 4–7 — **DONE: cand 4 LANDED (NotBenchable contention), cand 5 DROPPED (adjacent to batch-blocking-pool-segment sizing), cand 6 RIDER on reuse-allocation-clear, cand 7 LANDED (NotBenchable hang-class)**
- [x] T8 Standing gates — **DONE: count pins 149/93/94/55; lib 2175/0; clippy -D warnings clean (healer took 5 doc_markdown)**
- [x] T9 Freeze-table check — **DONE: UNMOVED by derivation (gen2/gen3 = clippy corpus; gen5 filters to FIXABLE_RULES — proposer-less rules excluded); no regeneration owed**
- [x] T10 GOAT — **DONE: G1 exhaustive gate PASS; G2 ParetoDominant 4/0/0 ×2; G3 lib 2175/0 (was 2174, +1 gate); G4 alloc-free fixed arm (0 allocs, axis 0/0)**
- [x] T11 Retrieval floors — **DONE: score-bench #160 heal_rate 96.36% FLAT; rule_outcomes rows recorded**
- [x] T12 Docs — **DONE: HISTORY.md Batch 172 entry + AGENTS.md count bump + Research 202 → RECORD + close-out + queue line + this plan COMPLETE**
- [x] T13 Commit+push + `rm -rf .raw/mezura` — **DONE: landed at `6e168aa3` (2026-09-23 00:01:19 +0700, pushed 7be9801c..6e168aa3); `.raw/mezura` + the neon_check scratch removed**

## Defers

- [-] Lane-intel artifacts: none owed — no lane mapping (recorded in Research 202; nothing to file in riir-ai).
