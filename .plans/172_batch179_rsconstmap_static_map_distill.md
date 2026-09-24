# Plan 172 — Batch 179: rsconstmap paired-probe-layout + static-map distill

**Status:** COMPLETE 2026-09-25 — all tasks landed (see HISTORY.md "Batch 179"); verdict Research 210 YES B+ → RECORD

Source: lemire/rsconstmap @ `bf7b33419d37c5d141075b9c1642fb69642a7896` (MIT OR Apache-2.0; re-clone at the pin — `.raw/rsconstmap` was removed at the verdict commit). Verdict + dedup evidence + ranked table: [Research 210](../.research/210_rsconstmap_paired_probe_layout_distill_verdict.md).

Leads (3): `paired-probe-layout-present-vs-absent` (A−, GOAT — layout) · `block-hash-before-gather-mlp` (A−/B+ — iterators) · `immutable-static-map-filter-selection` (B+ — complexity, the HashMap-selection family's missing regime cell). Riders: serialize-self-chunk-64kib (annotation on buffered-io/bufwriter-batched), header-pad-align + baseline-SIMD + bench-honesty doctrine (folds).

## Tasks

- [x] T0 Re-clone source at the pin into `.raw/rsconstmap`; re-verify the three quote sites (lib.rs:709-795, mapmany.rs:11-23+52-85, serialize.rs chunk loop + HashWriter) + read benches/constmap_bench.rs (the R3 bench-honesty doctrine informs our specs)
- [x] T1 Dedup re-verification greps at mine time (both vocabularies per Research 210 §Dedup; confirm `soa-hot-field` / `keyword-map-to-match` / `software-prefetch-storage-bound` / `buffered-io` adjacency wording still holds on current HEAD)
- [x] T2 Author 3 corpus entries (rust_perf: layout + iterators + complexity) — fixtures ORIGINAL minimal reproductions, never source-pasted; paired-slot fixed_code carries the baseline-SSE2/NEON xor3 shape + no-detection annotation
- [x] T3 Bench specs: (a) paired-layout — two-array vs paired-slots probe, BOTH cells (present-key arm + absent-key arm; the absent regression IS the regime content), working set > LLC (≥10M keys), random query order + query-order-allocated key buffer (R3 doctrine); alloc axis flat-by-construction (same bytes); (b) block-hash-before-gather — loop vs blocked two-phase, cold > LLC + the in-cache anti-regime cell; (c) static-map selection — `[dep: constmap]` (papaya precedent), HashMap vs ConstMap at 100k/1M keys, absent-key contract stated in annotation. (Adaptation on record: the absent-key cell lives in the annotation + G1, not the timed arms — the validator A/Bs one workload per rule; the small-size in-cache cells carry the anti-regime axis for (a)/(b).)
- [x] T4 G1 semantics tests: paired + batch arms bit-identical to reference op (u64 XOR — exact); static-map arm values exact on present keys (in-harness, live every bench invocation; paired + block G1 also landed as corpus tests incl. the NEON-vs-scalar xor3 pin)
- [x] T5 Standing gates: rust_perf corpus count pin 153 → 156, `corpus/mod.rs` header prose + per-category counts, `.docs/11_domains/rust_perf.md`, AGENTS.md rust_perf row (the count-prose law), corpus tests green, clippy `-D` ×2, retrieval floors, score-bench append + gen-8 snapshot + gate-calibration, `--corpus-stats` verified-live (the B174 anchor lesson). (Measured floors re-pinned: default blended 127/141·133/141·135/141, all five router arms at their new populations.)
- [x] T6 GOAT: G2 Pareto verdicts on (a)/(b); both-arms disclosure in annotations (source's own negative cells quoted); M3 aarch64 re-gate for the NEON arm (the dual-pin precedent) — the benches RAN on M3/aarch64 (the NEON xor3 path executed natively in every bench + the corpus G1 pins it scalar-identical); the x86_64 SSE2 arm mirrors the source repo's own compiled shape (upstream CI evidence; no in-tree target compiles harness strings cross-arch)
- [x] T7 Docs: research note Status → RECORD with landing commit; `.distill/001` queue line updated; HISTORY.md row; global binary rebuild + refresh if any bin-path delta (T10 rider law) — owed: the 3 new rust_perf rules are a bin-path delta
- [x] T8 Commit + push (`feat:`), rebase-none / ff per convention; `rm -rf .raw/rsconstmap` if not already gone
- [x] T9 Post-landing: consumer residue sweep NOT owed (suggestion-only rules, no proposer); bench-guard N/A (no benches/ edits)

## Defers

- [-] None at plan time. (Block-size 8-vs-16 constant: annotate, never gate — the source's own two-machine disagreement is the content.)
