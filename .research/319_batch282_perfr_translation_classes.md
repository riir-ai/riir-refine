# Research 319: Batch 282 — the PerfR C-to-Rust translation perf classes (rust_perf)

**Status:** RECORD — Batch 282 landed 2026-10-25 (shikuwa/4090, riir-refine idle loop; Plan 286; claim `1904cfdb8` discharged by this note)

## Source

**arXiv 2610.23044v1** — "Where Does the Performance Go? Understanding and
Recovering Performance Regressions in C-to-Rust Translation", Zhiqiang Yuan,
Chong Wang, Zhenhao Zhou, Junwei Liu, Xin Peng (Fudan University + Nanyang
Technological University), submitted 2026-10-16. Full text fetched + read at
the arXiv HTML v1 pin this session (stable v1, no artifact repo found — the
paper's concrete instances name real upstream projects: bzip2, heman, xxHash,
libzahl, miniz — cited as shapes, never pasted).

The paper's method is the corpus's own shape, externally validated: profile →
locate hot functions → match rule-specific Observed Patterns → check rewrite
preconditions (optimization cards) → rewrite → keep ONLY on functional +
performance validation. Their two-gate acceptance (incremental 1.0%
hot-function-weighted + cumulative 0.3% aggregate + severe-regression guard;
paired interleaved runs, CV ≤ 3%, Tukey 1.5×IQR, 500 ms min, 5 warm-ups,
30 pairs; min 0.1% project-wide net gain) mirrors rust_perf_bench's Pareto
validator + the Issue-940 determinism fold-order pins.

## What converted (+3 own rules, corpus 267 → 270, entries_48)

| # | rule_id | class (Table IV) | category | the law |
|---|---|---|---|---|
| 1 | `saturating-f2i-cast-proven-range-to-int-unchecked` | 2 Redundant Conversion Saturation | arithmetic | a range-PROVEN float→int `as` cast still pays the llvm.fptosi.sat machinery per evaluation; `to_int_unchecked` drops it — proof obligations (finite operand, in-range truncation) are the caller's; heman fastFloor instance (7 conversions in the island-noise loop) |
| 2 | `loop-invariant-dispatch-resolution-hoist` | 5 Repeated Invariant Dispatch | dispatch | a loop re-resolves an invariant handler/mode every iteration (Option re-unwrap, mutable-cell re-read) because the opaque indirect call's clobber assumption blocks LICM; resolve once before the loop; the mode-specialized-loop face disclosed manual |
| 3 | `raw-pointer-traversal-slice-view-recovery-load-elimination` | 9 Obscured Memory Access Properties | iterators | raw-pointer traversal hides extent + aliasing, so GVN/LICM cannot eliminate/hoist loads (libzahl zrsh: 73 vs 24 static loads, TBAA missing, 46-vs-16 GVN remarks); recover &[T]/&mut [T] views; the paper's MOST WIDESPREAD rule — 89% of hot functions, 46 of 62 accepted rewrites in this class |

All three: ANNOTATED NotBenchable, `bench_spec: None`, fixtures ORIGINAL
(standalone `rustc --edition 2021 -O` compiled + executed green before
landing), NO proposer files — rules 1 and 3 are UNSAFE-adjacent and are
**suggest-only by construction** (the healer never auto-applies them); the
proof obligations ride context_notes per the paper's own optimization-card
discipline ("Cards with unmet or unresolved preconditions are excluded").

## Riders (+3 count-neutral faces)

| host | face | class |
|---|---|---|
| `inline-hint-not-through-unmarked-wrapper` (context_notes) | the TRANSLATION-DROPS-ATTRIBUTES face — c2rust carries no inline attributes at all; the audit discipline extends to translation boundaries (diff attribute sets after any mechanical rewrite pass); 16% of hot fns | 4 Missed Hot-Callee Inlining |
| `simd-chunk` (name) | the vectorization-BARRIER face — the corpus owns the HOW (chunk rewrites), this face owns the WHY-blocked diagnosis (control-flow/call/memory-dependence remarks; bzip2's five hot decode loops); 24% of hot fns | 3 Missed Loop Vectorization |
| `chunks-exact` (name) | the BOUNDS-CHECK-ELIMINATION face — slice/iterator traversal exposes index-length relationships so LLVM drops per-access checks (the BZ2_decompress state-machine shape); 18% of hot fns | 1 Residual Bounds Checks (+ class 3's bounds-check barrier) |

## Drops (honest, adjudicated at T1)

- **Class 6 Inefficient Buffer Management** — corpus-covered:
  `launch-cost-scales-with-allocation-count-pack-one-arena` (the arena face IS
  the libzahl zmul_ll recursion-scratch shape), `bounded-batch-buffer-pool`,
  `vec-with-capacity`, `reuse-allocation-clear`.
- **Class 7 Byte-Oriented Memory Operations** — c2rust-specific (typed values
  through memcpy/memset, strlen recomputes); Rust-native faces owned by
  `copy-from-slice`, `unaligned-word-load-from-bytes`,
  `hot-byval-move-over-memcpy-threshold`.
- **Class 8 Scalar Element-Wise Processing** — chunk/block families owned:
  `chunks-exact`, `simd-chunk`, `dual-mask-fused-block-scan`,
  `dense-table-bulk-update-via-contiguous-iter`; the miniz checksum instance
  (block-wise, CPI 0.344) is the family's measured shape.

## Paper facts worth keeping (non-corpus intel)

- Cross-project prevalence (§VII-B): rule 9 = 89% of hot functions, rule 7 =
  35%, rule 3 = 24%, rule 1 = 18%, rule 4 = 16%; every one of the nine rules
  contributed accepted rewrites; ≥3 rules matched per project (median 4).
- RQ5 ablation: rule-guided 62 accepted rewrites vs 28 unguided; 46/62 involve
  Memory Property Recovery (class 9) — the rules change the NATURE of accepted
  rewrites, not just the count.
- Safety (§VII-C): unsafe-fn share 99.8% → 60.5%; pointer arithmetic −18%,
  derefs −10% relative to the refined baseline — performance recovery and
  safety narrowing are the same move here.
- Cost decomposition (§IV-B): instruction inflation is the predominant
  overhead source (9 of 12 study regressions; 25 of 27 evaluation
  regressions) — our league-side echo: the PerfR direction set is dominated
  by extra-work elimination, matching kernel_opt's dead-FLOP families.
- Rinst geomean 1.287 → 1.012 after PerfR; 18/41 operations end faster than C.

## Gates (all green this session)

- batch282 needle gate (rules_landed): PASS — verbatim needles on all three
  rules + the three rider faces + the nine paper-class-name negative asserts
  (the drops are adjudicated, never rule ids).
- Fixtures: 6/6 standalone `rustc --edition 2021 -O` compile + execute green.
- default lib 2,730/0 (+1 the batch282 gate) · rust_perf_bench lib 2,761/0.
- clippy −D ×3 postures clean (4 doc_markdown `PerfR`→backticked + 1
  too_many_lines allow on the gate, healed in-batch).
- Retrieval floors: latent_retrieval full lib 2,916/0 (435 s) — HELD, no
  re-pin (21/21 retrieval_eval inside).
- Freeze gen2 + gen3 e2es PASS, no table movement (P30: the freeze corpus
  builder is the clippy_lints seed — a rust_perf-only delta moves nothing).
- Score-bench run #199: heal 0.9636 / created 0.0182 / parse 1.0 — FLAT vs
  the gen-1 floor (96.4/1.8); frontier snapshot appended (generation 7).
- Maintained binary rebuilt at the release set + installed; `--info` CORPUS
  serves `rust_perf 270 rules` (build stamp clean at the landing HEAD —
  rebuilt post-commit).

## Count pins moved (one shape-asserted python pass)

tests.rs ×7 sites (total 267→270 + the batch265 witness + without_spec
146→149 + annotated 214→217 at THREE sites + the two unannotated messages),
mod.rs ×3 (header, mod decl, builder list + the doc-comment total), AGENTS.md,
README, `.docs/11_domains/rust_perf.md`.

## Lessons (this batch)

1. The `Some("...")\n.to_string(),` split-across-lines form is INVALID Rust
   (the method call must sit INSIDE the parens) — the compile gate caught all
   three instances pre-test; multi-line string field assembly stays a
   byte-verified job.
2. Rider faces must live in corpus DATA (name or context_notes), never in
   comments — the gate's hay-closure (name+notes+buggy+fixed) cannot see a
   comment, and a comment-only rider is invisible to every corpus consumer.
   Caught by the batch282 gate itself on the first run.
3. The fixture-standalone-compile harness hit the cp874 system-locale hazard
   (NamedTemporaryFile without explicit encoding=utf-8 + the § character) —
   the locale_io rule class, live.
