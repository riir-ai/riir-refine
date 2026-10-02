# Batch 70 — the arxiv barrier-free-sync paper distill (kernel_opt D/B, 3 rules)

**Status:** RECORD (batch landed same day; corpus 367 → 370)

**Source:** Sung, Shyamsunder, Zhang, Kroening, Choi — "A Barrier-Free
Synchronization Algorithm for Multi-Engine AI Accelerators"
(arXiv:2608.13757, cs.PL, 2026-08-13, to appear CGO '27; Amazon + Cornell).
Backend pass at the AWS Neuron ISA level; Lean 4 bisimulation ~15K lines,
0 admitted lemmas; measured on real Trainium 2 (trn2.48xlarge).

**Intake channel:** the Research-076 "arxiv systems papers" pool (opened
2026-08-19 via the research skill, which ran the prior-art gate + the
per-candidate dedup audit at pool time). This batch is the pool's
consumption — a paper distillation (the B61/B66 external-source class),
NOT file mining: no sibling lanes were spawned because the source is a
16-page paper the coordinator read in full (§§4-6) rather than a large
code tree; every quoted claim below was re-verified against the fetched
HTML this session.

**PASS-Redirects (synthesis):** Stokes, Brown, Flynn, Jamieson, Weinzierl
[arXiv:2608.19273 "DSLHyPE—a DSL kernel language for the Exascale
Hyperbolic PDE Engine ExaHyPE"] — its §VI-D "kernel parallelization and
orchestration" future-work item (model compute steps as a DAG the runtime
deploys onto warps/cores — the horizontal/vertical parallelisation
horizon) is the compiler-side framing of B70's closed-form
per-dependency waits; the paper independently reports DAG parallelism
applied throughout an ENTIRE kernel was NOT beneficial (their ref [17]),
which agrees with B70's barrier-fallback scoping. No new decision
content beyond the corpus → PASS.

## Rules (3)

1. **`barrier-free-per-dependency-wait-from-loop-counters`** (D 71→72,
   STRONG) — loops of heterogeneous workers with cross-iteration deps
   must not drain+handshake+reset ALL workers at iteration boundaries.
   Track a monotone entry count per worker/loop and compute each
   dependency's wait threshold in closed form (the paper's §5.2 three
   allocatable cases: producer-in-S → `r_P − k` for any offset k;
   producer nested forward offset-0 / backward offset-1 → `r_P`); the
   underflowed target is a no-wait; dependencies outside the subset fall
   back to the barrier form (stated in-rule). Source evidence: 10-45%
   latency cut across production NKI kernels (tiled matmul / RMSNorm /
   FlashAttention / AdamW / Dropless MoE), 3.3× on the sync-bound
   AddManyLoop microbench (norm-lat 0.30 — a no-dependency loop where
   every barrier was pure overhead).
2. **`shared-counter-decode-producer-progress`** (D 72→73, MEDIUM) — one
   dedicated progress counter per producer does not scale; share ONE
   counter per (loop, worker) and decode: in-order retirement keeps the
   N co-body counts within 1 of each other, so after the K-th of N
   retires in iteration r the counter reads `N·r − (N−K)` exactly (Eq
   4/5). Decision content = the §6.1 granularity ladder (per-instruction
   O(N) counters / 3 ALU ops; per-loop O(L·E) / 5; per-engine O(E) /
   O(D)), mixable per region, with §6.2 LICM + CSE cutting the ALU cost
   without touching correctness.
3. **`masked-branchless-wait-collapse`** (B 52→53, MEDIUM) — the wait
   condition is a disjunction (no-wait OR counter ≥ threshold) but the
   primitive takes a single threshold: collapse via the multiplicative
   0/1 mask `isGT(r̂_S, k) · expr` (Eq 2); fixed 3-op sequence, no branch
   on the issue path. The branchless-select mechanism on a sync
   predicate; offset-0-only loops elide the trip counter + mask
   entirely. Category B per the in-corpus `branchless-select` precedent
   (the pool entry's tentative "A" label was corrected at landing — A is
   memory-access in this corpus; the branchless ALU family lives in B).

## Distinctness (coordinator-verified against the live corpus rule texts)

- `chain-dispatches-single-sync-point` (B19): acyclic straight-line, ONE
  end-sync — no loops, no counters.
- `expert-loop-two-phase-split-batch-across-iterations` (B40): RESTRUCTURES
  the loop into two batched phases; rule 1 computes the exact
  per-dependency moment with no restructuring.
- `sibling-dispatches-before-any-readback` (B36): queue ordering of
  independent work — no thresholds.
- `mid-pipeline-device-alloc-is-implicit-sync` (B58): a hidden sync to
  AVOID vs a computed wait to INSTALL.
- The intra-kernel smem barrier family (`smem-slot-role-reversal-needs-
  barrier`, B57): barriers that can never be removed WITHIN a workgroup —
  rule 1 removes INTER-worker barriers between separately-scheduled
  workers. (The 129 `workgroupBarrier` sites in riir-gpu are that level.)
- `branchless-select` (B): same ALU mechanism class, different operand —
  a data select inside a compute kernel vs a sync predicate feeding a
  wait primitive.
- Dedup re-verification at batch time: zero hits for wait-threshold /
  shared-counter-decode / isGT-mask-wait concepts across BOTH domain
  corpora (kernel_opt + rust_perf) — the pool's audit held.

## Honest caveats encoded in-rule

- The 3.3× / 10-45% numbers are Trainium-specific source evidence; the
  rules encode the MECHANISM. The pool's applicability audit is quoted
  in-rule: wgpu/CubeCL/Metal host APIs expose no fine-grained semaphore —
  live surfaces are cudarc CUDA events + multi-stream
  (`cudaStreamWaitEvent` IS a per-dependency wait), host orchestration
  loops, multi-threaded worker loops.
- Rule 3 regime honesty: pays where the wait path is hot AND the
  primitive truly takes one register threshold; a native wait-or-skip
  API makes the branch form equivalent.
- Rule 1 states its own fallback subset (multi-level gaps, nested
  offsets > 1 → barrier form) — never mis-encode outside the allocatable
  cases.

## GOAT

- **G1** `batch70_pseudocode_parses` green (fixtures from the paper's §5
  closed forms; Eq 2/4/5 + the three §5.2 cases, host-orchestration
  pseudocode). Presence + concepts gates (5 needles each) green.
- **G2** lib tests: kernel_opt 864/0/1i · default 657/0 · all-features
  1115/0/13i.
- **G3** `ci_feature_guard.sh` ALL LAYERS PASS (24 single-feature combos,
  both extremes clippy-clean).
- **Retrieval floors HELD:** kernel self-variants 30/30 = 100.0/100.0/
  100.0; blend@1 (op-bigram) 94.7 / blend@3 96.9; perf@1 96.2; clippy@1
  89.6 — every floor at or above its pin; no re-pins needed.

## Drive-by (pre-existing, proven at HEAD)

Layer 5a (all-features clippy, `-D warnings`) was RED at clean HEAD
`0839fe0`: `floors::ROUTER_ANNOTATION_3D` is declared
`#[cfg(feature = "kernel_opt")]` but its only use-arm requires
`not(sec)` — under all-features (sec + kernel_opt) the const exists
unused → dead_code. The sec session's GOAT validated default + sec +
sec/kernel/latent test states but not the no-default/all-features clippy
extremes with `-D warnings`. Fix (separate `fix:` commit): narrow the
declaration cfg to `all(feature = "kernel_opt", not(feature = "sec"))`
— the only mismatched cfg of the four annotation-floor consts (the
other three match their use-arms exactly). Also caught by the same
layer: the new `batch70_pseudocode_parses` initially lacked the
`#[cfg(not(feature = "kernel_verify"))]` attribute every prior
`*_pseudocode_parses` gate carries (the `.parses` field does not exist
under kernel_verify) — fixed in-batch; both catches are the CI guard
doing its job.

## Generator/landing notes

- Entries authored via the proven one-shot Python generator
  (`json.dumps` with `ensure_ascii=False`); zero post-write fixes needed
  (all `//!` backticks + comment shapes pre-empted).
- Count pins touched (all sites): tests.rs ×4 (len/per-category/sum ×2)
  + batch comment list, router.rs ×2, mod.rs docs + 2 category comments,
  entries_gpu_07.rs header (354-364 → 354-370). No 20th count-pin site
  surfaced — the full-suite run found none beyond the known set.
- corpus_eval: the 30-probe kernel self-variant set is unchanged (prior
  batches did not add probes either); the corpus grew to 370 with the
  eval fixed at 30 — consistent with B55-B63 practice.
