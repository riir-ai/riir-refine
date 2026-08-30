# Bench 061 — Plan 049 Phase 0 stop verdict: the trained-drafter band on real code is 0.94pp

**Date:** 2026-08-31 · **Plan:** [049](../.plans/049_trained_drafter_phased_rollout.md)
Phase 0 (T0.1 + T0.2 + T0.3) · **Verdict: STOP — record the negative, close the
trained-drafter track, keep decline.** Corpus-expansion is the honest successor
work, filed as a mining lead, not a selector.

## What was measured

**T0.1 dataset** (`examples/t01_real_repo_label.rs` →
`.benchmarks/data/049_t01/`, BLAKE3 `eb4aaed8d00394d6319dcf501f45b861e5ae72d243680f2d5f6859450682941f`):
2,128 finding-bearing real spans from 6 workspace repos (katgpt-rs, riir-ai,
riir-train, riir-chain, riir-neuron-db, riir-dapps; 6,343 files, 55,656 clippy
events, each repo at its recorded HEAD), labeled by **in-context** clippy
(`--workspace --all-targets -W clippy::all -W clippy::pedantic -W
clippy::nursery`, primary-span attribution to AstChunker spans, hard-error
files excluded wholesale).

**Method pivot (measured, not guessed):** the planned isolated-span labeling
(chunk → temp crate → oracle) produced ~0 labels per 200 compiled spans on
FOUR source sets — workspace repos, their unswept corners, and the untouched
sibling codebase `seal-online-remaster`. Most real-repo item spans do not
compile standalone (crate-internal references), and the self-contained
fraction is lint-clean by construction. The oracle's
`compiles=false → no labels` contract excludes exactly the code that needs
healing. In-context labeling measures the production distribution instead:
the user's file has context, the healer sees a span.

## Layer 1 — corpus coverage (the binding number)

**20 / 2,128 = 0.94pp** of real finding-bearing spans carry ANY healable
target in the current 41-rule retrieval vocabulary. Real clippy fires 99
distinct lints on our own code; the top of the distribution (the committed
`fired_histogram_top`):

| fired | lint | in corpus? |
|---:|---|---|
| 1,011 | must_use_candidate | no (nursery) |
| 723 | missing_const_for_fn | no (pedantic) |
| 518 | doc_markdown | no (pedantic) |
| 463 | cast_precision_loss | no (pedantic) |
| 206 | suboptimal_flops | no (pedantic/perf) |
| 195 | cast_possible_truncation | no |
| 130 | use_self | no |
| 90 | missing_errors_doc | no |
| … | | |

The 2026-08-16..19 clippy sweep (4,000+ heal edits across these repos) drove
exactly the 33+4 corpus rules to near-zero on real code — 20 labeled spans
survive, mostly freshly-landed or divergence-class residue. **The sweep
already owns its surface completely.** A trained prior over the existing rule
set cannot address the other 99.06% at any ranking quality: proposer-absent
at span level is not a ranking failure.

## Layer 2 — ranking decomposition over the labeled subset (for the record)

`decompose_headroom_with_spans` over the 20 labeled spans: 14/20 top-1
correct (70%), 6 selection misses → 1 pool-depth, 0 type-blind, 0
unfixable-target, **5 selector-addressable (25pp of the subset)** — with the
instrument's own caveat binding: at n=20, a "recover ≥50%" G1 gate reduces to
"recover 3 spans", not a measurable effect. Layer 2 alone says PROCEED; that
is the Plan 017 empty-denominator trap repeating (Bench 043 closed Phase 1
for exactly this shape). Layer 1 is the honest denominator and it says STOP.

## Verdict (T0.3)

- Trainable band on real code: **0.94pp < 2.0pp bar → STOP**.
- The plan's trained-drafter track (Phase 0 gate → Phase 2 selector) CLOSES.
  `decline` stays the L4 default; the Bench 049 CF arm + CodeCureAgent loop
  (T0.0d) remains owner-gated and independent of this verdict.
- Reopen condition (pinned by the test, not prose): a NEW T0.1 dataset whose
  in-vocab band clears 2pp — reachable only by corpus expansion (below), a
  real consumer pulling the trained lane, or a lint-distribution shift.

## Successor work (recorded, not scheduled here)

The dataset IS the corpus-expansion roadmap: `fired_histogram_top` ranks the
pedantic/perf long-tail by real-code frequency. `suboptimal_flops` (206) and
`missing_const_for_fn` (723) are bounded-fix candidates in the existing
rust_perf/clippy proposer pattern; `must_use_candidate`/`doc_markdown`/
`cast_*` are suggest-only classes. Filed into the mining queue snapshot as
the next Protocol-A lead — modelless proposer work, no trained component.

## En-route instrument fixes (root cause, stays)

1. **Probe/label crate-dir separation** in `ClippyOracle` (`probe_` vs
   `batch_` prefixes): sharing one keyed dir let cargo's diagnostic-cache
   replay SUPPRESS clippy warnings after a `cargo check` on the same dir
   (measured: check-then-clippy emitted ~zero lints where a fresh dir fires
   readily). This artifact had briefly masqueraded as "the workspace is
   clean" before the single-run dump exposed it.
2. **`CARGO_TARGET_DIR` stripped from oracle children**: same-name temp
   packages sharing an inherited target dir cross cargo's diagnostic replay
   (flipped the batch-agreement test red under an overridden target dir).
3. `compiles_batch` (one-cargo-run batch compile probe) ships as oracle API
   with the live test `compiles_batch_splits_a_mixed_batch_without_the_fallback_storm`.

## Gates

- `tests/t049_t01_dataset_integrity` 2/2 (BLAKE3, schema, bar, provenance).
- `tests/t049_t023_stop_verdict` 1/1 (both layers + the pinned reopen condition).
- default lib 937/0 · latent+verify lib 1071/0/7i · clippy 0 (default and
  latent `--all-targets`).
