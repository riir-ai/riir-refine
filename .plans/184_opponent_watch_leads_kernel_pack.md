# Plan 184 — Batch 188: convert the graded opponent-watch leads (TensorFold EXL3 wave + MLX gather_qmm family)

**Status:** IN LANDING — staged by the 2026-09-29 grading ([Research 220](../.research/220_opponent_watch_leads_tensorfold_exl3_mlx_gather_qmm.md)); conversion deferred from that session (box Plan-337-saturated — sibling measurement at ~9 cores; every task below needs cargo). Conversion slot: the next kernel/corpus batch slot on whichever box is quiet (the 4090's standing cadence or a quiet M3 window).

**Sources (pinned, standing vendor clones — no .raw):**
- TensorFold `/Users/katopz/git/vendor/tensorfold` @ `8c1291cc7e9e75ad71e127f7c557b34e6ef8985a` (MIT) — commits `b16fb91`, `e9780ed`, `38235a9`, `1bbd2dd` (all quotes in Research 220 re-verified at these).
- MLX `/Users/katopz/git/vendor/mlx` @ `37c26e5755da637255d57ea34b4879196a485301` (MIT) — commits `9008944` (lead), `a2a09fd`, `d9add9d`, `cbb4059`, `3eb822c`, `59d600b`; fixes `fc67b1d`, `d73eb75`, `dfe17ba`.

**Candidate manifest (from Research 220; the batch may land a subset — a dropped candidate is recorded with its kill reason, never silently):**

| id | rule_id (proposed) | grade | source |
|---|---|---|---|
| A1 | `low-bit-warp-run-load-shuffle-redistribute` | A− | TF `38235a9` |
| A2 | `register-prefetch-ring-measured-cutoff` | B+ | TF `38235a9` |
| A3 | `shape-only-plan-row-invariant-kernel` (+ last-arrival fixed-order split-combine rider) | B+ | TF `e9780ed` |
| A4 | `bitstream-decode-into-fragment-identity` | B− | TF `b16fb91` + `1bbd2dd` |
| A5 | `width-range-instantiation-trap-uncovered` | B− | TF `1bbd2dd` |
| A6 | `pointer-table-experts-in-place` | C+ | TF `1bbd2dd` |
| B1 | `sorted-gather-tile-compaction-warp-scan-scheduler` | A− | MLX `9008944`+`a2a09fd` |
| B2 | `batched-matvec-as-tile-matmul-dispatch` (arch-keyed crossover rider) | B− | MLX `9008944`/`d9add9d` |
| B3 | `nvfp4-two-level-scale-fold-at-loader` | B− | MLX `cbb4059`+`3eb822c`+`59d600b` |
| B4 | `ragged-tile-bounds-contract` (annotated, NotBenchable correctness) | B− | MLX `fc67b1d`+`d73eb75`+`dfe17ba` |

Expected landing: 5–8 rules (A1/B1 near-certain; A2/A3 strong; the rest adjudicated at authoring time).

## Tasks

- [x] T1 — Re-verify the dedup gate at CONVERSION time against the then-current corpus (it moved at B187; new rules may have landed): re-grep both vocabularies per retained candidate; a new collision demotes to rider/kill with the reason recorded in the batch note.
- [x] T2 — Author the corpus entries (`src/domains/kernel_opt/corpus/` — new `entries_gpu_56.rs` or append per the file-size convention): ORIGINAL minimal fixtures only (never verbatim source — license hygiene + the binary-only distribution rule); each entry carries the source-measured numbers where they exist (A1's GB/s table + bit-exactness claim; B1's mechanism-arithmetic argument + zero-source-number disclosure), the distinct-from lines from Research 220, and the pinned shas as provenance.
- [x] T3 — A1/A2/A3/B1 carry `context_notes` regime annotations (A2's bits ≤ 6 cutoff; A1's K2 ≤ 4 shuffle gate; A3's shape-only contract; B1's sorted-indices precondition).
- [x] T4 — B4 lands (if taken) as an annotated NotBenchable correctness entry (`bench_spec: None`, the Batch-136 decision-content shape).
- [x] T5 — Standing gates: the batch parse gate (batch188) extended with every new entry; syn-parse clean.
- [x] T6 — Count pins: kernel_opt 675 → N (the category ledger updated IN THIS PLAN: A entries +N_A, B +N_B…; the first pin pass must match the ledger — the B186 landing-defect lesson).
- [x] T7 — GOAT shape: these land as measured-GOAT distillations (source-measured, sha-pinned) per the B176–B187 precedent; where our validator cannot host the surface (CUDA/Metal), the entry carries the NotBenchable annotation with the source numbers as the evidence, never a fabricated local bench.
- [x] T8 — Retrieval-precision re-measure: `CARGO_TARGET_DIR=/tmp/riir-clippy-b188 cargo test --features kernel_heal,latent_retrieval --lib retrieval -- --nocapture` — the floors (kernel self-variants 100%, blended 3-domain 77.1/86.4/92.4) must HOLD; a decay fails the batch (fix retrieval or re-pin with the delta recorded — never silently).
- [x] T9 — G3 no-regression: `cargo clippy` `-D warnings` at BOTH feature states; default `cargo test --lib` green (count pins intact). Freeze tables NOT owed if the clippy corpus has zero delta (P30) — verify and state it.
- [ ] T10 — Docs: bump the kernel_opt count in AGENTS.md (the Batch-NN row), update `.docs/11_domains/` if the family adds a page-worthy cluster, append the queue-snapshot BATCH 188 row (same commit), and cross-reference Research 220 + this plan from the batch row.
- [ ] T11 — Rebuild the global binary with the RELEASE feature set (the corpus is compile-time — the standing staleness footgun) + `--corpus-stats` confirms the new kernel_opt count; remove any isolated CARGO_TARGET_DIR after.
- [ ] T12 — Commit `feat: batch 188: ...` + push; update `.distill/001_mining_queue_snapshot.md` staging section in the same commit.

## Notes

- The corpus has grown twice since the grading (B187 landed 09-28 pre-grading; nothing after as of 2026-09-29) — T1 is the guard, not a formality.
- Sources are standing vendor clones — do NOT clone into `.raw/`; cite the clone path + pinned sha. If either clone has MOVED past the pin at conversion time, re-verify the quoted lines at the PINNED commit (`git show <sha>:<path>`), never at the moved HEAD.
- The lane-intel half of Research 220 (the riir-infer exl3 reference recommendation) is NOT this batch's work — it stays a recommendation for the riir-infer-owning session.
