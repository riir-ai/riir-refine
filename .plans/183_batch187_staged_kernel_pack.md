# Plan 183 — Batch 187: the ggml-cpu staged kernel pack of 5 (Research 218 S1–S5)

**Status:** COMPLETE — landed `9e7178f44` + pushed (the staged pack of 5 converted; corpus
670→675; all gates green).

- [x] T0 — Pin re-verification: all 5 staged quotes VERBATIM at
  `E:/git/vendor/prismml-llama.cpp @ d834d44e` (tiled.cpp:679 + 686-695;
  tiled-kernel.cpp:91-93 + 169-170; tiled.cpp:1103-1104 + 1082-1083;
  tiled-kernel.h:92-95 + tiled.cpp:234-236; tiled-kernel.h:105-116).
- [x] T1 — Dedup grep BEFORE proposing: `writeback|j-major`, `band pass|register budget`,
  `work.steal|chunk stepdown`, `pshufb|lut16|shuffle.decod`, `gfni|gf2p8affine` —
  CLEAN (the entries_gpu_53.rs hits are the B186 file-header comment naming the staged
  pack; gpu_21 'band pass' = a G1 correctness band; rust_perf work-steal = the rayon
  queue ladder — kinship, not duplicates).
- [x] T2 — Author corpus: `entries_gpu_55.rs` — S1 `jmajor-acc-buffer-transpose-writeback`
  (A), S2 `register-budget-band-partition-indep-correction-offband` (B), S3
  `chunk-downsize-work-stealing-microtile-floor` (E), S4
  `lut16-pshufb-byte-expansion-at-stage` (B), S5 `gfni-sign-mask-expand-cmpeq-fallback` (B).
  Category deltas A 145→146, B 71→74, E 64→65 (the tests table is the truth — E was
  pinned 64, not the prose's stale 61); kernel_opt 670→675.
- [x] T3 — Registration + pins: mod.rs (mod + extend + category comments + KERNEL_CATEGORIES
  lists A/B/E + the stale 655-rule doc line → 675), corpus/tests.rs (total ×4 sites +
  per-category table + the batch187 parse gate), router.rs:472.
- [x] T4 — GOAT gates: batch187 parse gate ✓ · lib default 2385/0 ✓ · kernel slice
  2763/0 ✓ · retrieval floors HELD 21/21 ✓ · clippy -D both postures ✓ (healer took 1
  doc_markdown mechanical); score-bench + freeze not owed (zero clippy-corpus delta).
- [x] T5 — Research 218 landing-record paragraph + AGENTS.md counts (kernel_opt bullet +
  kernel_heal line) + `.docs/11_domains/kernel_opt.md` ledger row + README kernel row.
- [x] T6 — Snapshot update (.distill/001: staged pack consumed, staging queue EMPTY) +
  commit `feat: batch 187: ggml-cpu staged kernel pack (5 rules)` + push.
