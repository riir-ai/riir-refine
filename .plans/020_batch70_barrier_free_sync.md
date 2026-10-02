# Plan 020 — Batch 70: barrier-free sync paper distill (kernel_opt)

Source: Sung, Shyamsunder, Zhang, Kroening, Choi — "A Barrier-Free
Synchronization Algorithm for Multi-Engine AI Accelerators"
(arXiv:2608.13757, cs.PL, 2026-08-13, CGO '27; Amazon + Cornell; Lean 4
bisimulation ~15K lines 0 admitted + measured on real Trainium 2).
Pool opened 2026-08-19 via the research skill (Research 076 intake channel:
"arxiv systems papers"); verdict YES B+, route kernel_opt (D dispatch/sync),
yield 2–3. The pool's per-candidate dedup audit found NO wait-thresholds-
from-counters and NO shared-counter decode anywhere in kernel_opt/rust_perf.

This is a **paper distillation** (the B61/B66-class external-source batch),
not file mining: the research skill's prior-art gate ran at pool time; the
coordinator re-verifies every quoted claim against the fetched paper
(§§4–6 read in full this session) instead of spawning file-reading lanes.

## Rules (3)

1. `barrier-free-per-dependency-wait-from-loop-counters` (D) — STRONG.
   Loops of heterogeneous workers with cross-iteration deps: do NOT
   drain+handshake+reset all workers at iteration boundaries; track a
   monotone entry counter per worker/loop and compute each dependency's
   wait threshold in closed form (`r_P − k` producer-in-loop; `r_P` for
   nested forward offset-0 / backward offset-1); no-wait disjunct for
   underflowed targets. Source: 10–45% latency cut on production NKI
   kernels, 3.3× on a sync-bound microbench (AddManyLoop 0.30 norm-lat).
   Honest applicability: wgpu/CubeCL/Metal host APIs expose no fine-grained
   semaphore — live surfaces are cudarc CUDA events + multi-stream
   (`cudaStreamWaitEvent` IS a per-dependency wait), host orchestration
   loops, multi-threaded worker loops. Encode the MECHANISM.
2. `shared-counter-decode-producer-progress` (D) — MEDIUM. One shared
   counter per (loop, engine) + in-order retirement → decode per-producer
   progress via `s ≥ N·r − (N−K)` (Eq 4/5); decision content = the
   granularity ladder (per-instruction O(N) semas / 3 ALU; per-loop
   O(L·E) / 5 ALU; per-engine O(E) / O(D) ALU), mixable per region,
   LICM/CSE-reducible (§6.2).
3. `masked-branchless-wait-collapse` (B — the `branchless-select` family
   precedent) — MEDIUM. Disjunctive wait predicate (no-wait OR threshold)
   → single threshold via 0/1 `isGT(r̂_S, k)` mask multiply (Eq 2); fixed
   3-op sequence, no branch on the issue path.

Category deltas: D 71→73, B 52→53. Corpus 367→370.

## Distinctness (coordinator-verified against corpus rule texts)

- vs `chain-dispatches-single-sync-point` (B19, D): acyclic single end-sync,
  no loops/counters — rule 1 is per-dependency thresholds ACROSS loop
  iterations.
- vs `expert-loop-two-phase-split-batch-across-iterations` (B40, H):
  RESTRUCTURES the loop into 2 batched phases — rule 1 computes the exact
  per-dependency moment without restructuring.
- vs `sibling-dispatches-before-any-readback` (B36, D): queue ordering —
  no thresholds.
- vs `mid-pipeline-device-alloc-is-implicit-sync` (B58, D): a hidden sync
  to AVOID, not a computed wait.
- vs `branchless-select` (B, kernel_opt): same ALU family, different
  operand — data select vs sync predicate collapse.
- vs rust_perf `branchless-conditional` / `sortedness-check-branchless-
  accumulate`: different domain corpora; rule 3's trigger is a WAIT
  primitive taking one threshold, not a data branch.

## Tasks

- [x] T1 Dedup re-verification at batch time (grep corpus for
      wait-threshold / shared-counter / isGT / per-dependency concepts)
- [x] T2 Author 3 entries in `entries_gpu_07.rs` (270 lines → room; header
      refresh) — fixtures Rust-shaped pseudocode from the paper's §5
      closed forms (Eq 2/4/5 + the §5.2 three cases)
- [x] T3 Wire `KERNEL_CATEGORIES`: D += 2, B += 1; update ALL count pins —
      tests.rs (len 367→370, per-category D 71→73 + B 52→53, sum ×2),
      router.rs (2 sites), mod.rs docs + category comments
- [x] T4 Standing gates: `batch70_rules_present` + `batch70_concepts_covered`
      (5 needles each, case-exact) + `batch70_pseudocode_parses`
- [x] T5 GOAT G1: syn-parse gate green (needed the `not(kernel_verify)`
      cfg every prior `*_pseudocode_parses` gate carries — caught by CI
      Layer 5a, fixed in-batch)
- [x] T6 GOAT G2: lib tests pass (kernel_opt 864/0/1i + default 657/0 +
      all-features 1115/0/13i)
- [x] T7 GOAT G3: CI guard ALL LAYERS PASS + clippy 0 both states. Drive-by
      (pre-existing at HEAD, proven): fixed the sec landing's Layer-5a red
      `ROUTER_ANNOTATION_3D` cfg mismatch (separate `fix:` commit)
- [x] T8 Retrieval re-measure: floors HELD — kernel self-variants
      100/100/100, blend@1 94.7, perf@1 96.2, clippy@1 89.6 (no re-pins)
- [x] T9 Research note `.research/080_barrier_free_sync_distill.md`
      (Status: RECORD)
- [x] T10 Docs: AGENTS.md corpus count 344 → 370 + Batch 70 record;
      `.docs/11_domains/kernel_opt.md` total 364 → 370 + B63 + B70 rows;
      queue snapshot updated (pool marked LANDED + queue-state header)
- [x] T11 Commit + push (batch commit + the separate cfg-fix commit)

## Honest caveats to encode

- The 3.3× / 10–45% numbers are Trainium-specific source evidence — the
  rules encode the MECHANISM, not the numbers as promised gains.
- Rule 3 is MEDIUM: the mask collapse pays only where the wait path is
  hot AND the wait primitive takes a single register threshold; on hosts
  with a "wait or skip" API the branch form is equivalent.
- The paper's fallback: dependencies outside the allocatable subset
  (multi-level gap, nested offset > 1) still need the barrier form —
  rule 1 must say when it does NOT apply.
