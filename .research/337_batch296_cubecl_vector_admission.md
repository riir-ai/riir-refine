# Research 337 — Batch 296: the cubecl #1922 vector-admission law

**Status:** RECORD — batch landed (kernel_opt 981 → 982).

## Source

tracel-ai/cubecl PR #1922 — "fix: check every outer stride before vectorizing
tensor reads" (laggui), MERGED 2026-10-26T20:10:36Z, merge
`5bd6d17aa883276bf4cf76eadb8ad518c4799647`, 3 files +61/−15, Apache-2.0.
Provenance: GitHub API read + `.diff` fetch 2026-10-27 (this session, 4090 box
shikuwa); no clone made, nothing to sweep.

## The law (one paragraph)

A vector width admitted for a strided/overlapping tensor read is valid only if
EVERY non-axis stride is a multiple of it. The minimum non-zero stride — or
any single-stride proxy — does not certify the rest: overlapping (unfold)
views carry outer strides that are not multiples of the innermost
(`shape [1,2,4], strides [10,4,1], axis 2`: min non-axis stride 4 admits
width 4, but row 1 starts at element 10 — 10 % 4 ≠ 0 — a misvectorized read).
Broadcast strides (0) satisfy every width. The admission asymmetry:
over-strict = false negative (the vectorizer declines — safe); under-strict =
wrong results (a vectorized read lands mid-row). Admission predicates must err
toward refusal, and subset validation is not admission.

## Why the staged "thin" flag was wrong, and right

The 10-27 item-11 sweep graded the lead B− "thin decision content disclosed"
from the commit message alone. The full diff carries: a concrete matcher (the
`.all()` predicate), two new unit tests with named failure modes
(`parallel_unfold_with_odd_outer_stride_rejects_vectorization`,
`parallel_unfold_outer_stride_limits_vector_width`), and an e2e
materialization test. Re-graded **B** at the T0 read. The lesson generalizes
the B280/295 verdicts: grade at the diff, never the message — but also the
inverse held here (the sweep correctly flagged that it had NOT read the diff,
which is what forced this T0).

## Dedup adjudication (T1)

| Rule | Axis | Relationship |
|---|---|---|
| `stride-align-vectorization-key` (L, Batch 7, burn-cubecl tune.rs) | width DERIVATION (min trailing-zeros of stride×bytes — sees ALL strides, safe-by-construction) | nearest kin; composes (derive from all, or admit by checking all) |
| `smem-fill-align-chunk-not-element` (N, B137, emmy) | smem BASE alignment from the FILL CHUNK (fault class, in-kernel) | different memory space + decider |
| `ptxas-recoalesces-scalar-runs-wash` (A, B137, emmy) | the escape-arm static alignment PROOF doctrine | this rule IS the proof predicate — compose |
| `vector-width-pow2-sweep` (K, Batch 7) | width as AUTOTUNE dimension | tuning presumes admission — this is the gate |
| `contiguity-guard-before-reshape` (Batch 7) | the contiguity oracle before a relabel | different object |

Grep evidence (3+ vocabularies, both corpora): `vectoriz` → 10 kernel_opt
file hits, all other families; `next_stride|min.*stride` → ZERO; `unfold`
→ ZERO; `stride` rules → the derivation twin + layout-consumption senses, no
admission owner. OWN rule.

## The trap the rule names (why min-of-strides is not just "a different rule")

The min-trailing-zeros DERIVATION is safe because it divides by the same set
it saw. The founding failure is a planner that VALIDATES a candidate against
a subset (the min, the first, the innermost) while other strides exist. The
failure mode is silent: no panic, wrong data. That is the rule's teeth and
the reason it is a correctness-admission law, not a perf hint.

## Landing (Batch 296 / Plan 301)

- 1 own rule: `vector-width-admission-checks-every-outer-stride`
  (KernelCategory::DtypeAlignment / N), new slice `entries_gpu_132` (B295
  took 131); registration + batch296 needle/parse gate.
- Count pins: corpus total 981 → 982 (35 multi-line + 2 inline `total,`
  sites in corpus/tests.rs — the second shape found live by the failing
  gate, the en-route lesson below); router Plan-003 expert-total 982;
  `KERNEL_CATEGORIES` DtypeAlignment 86 with the rule-id list head entry +
  the N header comment 85→86.
- Gates: batch296 needle+parse 2/2 FIRST-RUN; kernel_opt lib 3261/0;
  default lib 2749/0; clippy `-D warnings` ×3 postures clean (3 doc-lint
  heals in-batch: slice-header list-indentation ×2 + `into_contiguous`
  backticks); retrieval floors HELD both postures (kernel+latent 22/0
  ~222 s, default `retrieval_` 21/0 ~55 s) — no re-pin; freeze NOT owed
  (P30 confirmed live: `generations.rs::atoms()` builds from the clippy
  seed corpus); score-bench NOT owed (no rust_perf delta); release-set
  binary NOT owed (kernel_opt outside).

## En-route lessons

1. **The count-pin census must cover BOTH assert shapes.** The 35-site regex
   (`^\s*981,$`) missed the two `total, 981,` inline sites — the failing
   `kernel_categories_counts_match_plan_003_table` gate found them. The
   shape-asserted-script + gate-first-run discipline caught the residual
   exactly as designed; a future batch's pin script should grep BOTH
   `^(\s*)981,$` and `\b981,$` non-comment forms before authoring.
2. **The KERNEL_CATEGORIES rule-id list is the real category registration**
   — the category-count row in tests.rs:5536 is derived-from-it prose. Bump
   the ID LIST first, the count rows second; a rule absent from the list
   fails the exhaustive test regardless of every numeric pin.
3. Doc-lint class (recurring): slice-header `//!` prose forms doc lists when
   a line begins after a wrap — indent continuation lines or reflow; and
   bare snake_case identifiers in doc prose need backticks
   (`into_contiguous`).

## Queue after

kernel_opt staged pool: EMPTY (the cubecl #1922 lead discharged). Next pool =
the nnethercote window (~11-01) or newly staged leads; the M3 lanes hold
bbtree (to 09:44Z today) + resin (to 08:28Z 10-28) rust_perf claims.
