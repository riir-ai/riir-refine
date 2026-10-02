# Research 099 — oMLX v0.5.4rc1 → v0.6.3 release delta (Batch 84, kernel_opt)

**Status:** RECORD — Batch 84 landed (Plan 045; 2 rules + 1 rider, corpus 396 → 398).

## Source

- **Repo:** jundot/omlx (Python/MLX Apple-Silicon serving engine; the Research 082/083
  source, 8 releases further on). Delta read at distill time via the GitHub releases
  API (27 releases; primary record: riir-ai Research 353 §2.5.1) and re-verified at
  mine time by the two mining subagents against the live PR pages + diffs + release
  notes (#2966, #3133, #3110).
- **Lane:** the mining queue snapshot's 2026-08-28 entry (commit `b2a3295`) — the
  queued external batch after Batch 83.

## The provenance corrections the mine-time verification caught

1. **The "+26.3%/+22.7% PP" gloss was WRONG.** PR #3133's figures are
   **prefill-THROUGHPUT** gains (588.2/503.0 tok/s vs GPU-only 465.8/410.1 at 4K/32K),
   not perplexity. The distilled record's "PP" was ambiguous; the corpus now carries
   the correction in-rule ("throughput numbers, never cite them as quality") — which
   is the rule's own thesis (aggregate metrics are not the gate). The quality evidence
   is different and stronger: baseline-discriminating 32K retrieval/ordering/code
   restored + byte-reproducible output hashes on a second pass.
2. **The 35.8→4.7 GB / 32.1→0.2 GB figures on #3110 are the research record's, not
   the PR's.** The PR body quantifies ~0.5 GB/layer of pure-waste transient. The rider
   text carries the split provenance.
3. **PR #2966 is fully source-verified beyond the release notes:** the tuner crossover
   formula `floor(S·G/H)+1` is in the diff verbatim (2048 × 349.4 / 527.4 + 1 = 1,357),
   the tail-size probe (−5.9% at a 1,024-row tail, first positive ~1,152, +84.6% at
   2,047) isolates the dispatch-avoidance mechanism, and the intermediate-only +
   strip-before-recurrence invariants are pinned by named regression tests
   (`test_gdn_profitable_tail_is_padded_before_recurrence`,
   `test_mlp_profitable_tail_is_padded_and_sliced`).

## What landed (Batch 84 / Plan 045)

| Entry | Category | Mechanism |
|---|---|---|
| `pad-tail-to-fixed-tile` | D (83→84) | Residual tails ≥ the calibrated crossover run the resident fixed-shape tile instead of a separate small dispatch; ONLY intermediate activations padded, synthetic rows stripped BEFORE any stateful consumer; prompt/KV/positions/recurrent state never extended. +29.0% e2e at a 2,047-token tail. |
| `token-local-only-approximate-accelerator` | N (21→22) | The delegation boundary for approximate compute is the consumer's STATE HORIZON: token-local rows tolerate bounded approximation; recurrent-feeding rows stay checkpoint-precision; the configured fraction is a logged CEILING, never topped up. |
| B56 rider `assigned-columns-only-staging-dequant` | (doc on B56) | Split-device staging slices to the device boundary BEFORE dequant (packed-axis, group-boundary); bit-identical; the live-fields-only class at column granularity. |
| `latch-failed-accelerator-shed-to-fallback` | DEFERRED | Queue verdict stands: fold into a measured-GOAT from our own `status=0x9` work instead of an external-only rule. |

## Differentiations encoded

- `pad-tail-to-fixed-tile` vs B16 `clamp-tile-indices-to-sentinel-discard-at-write`
  (bounds guard inside one dispatch, not dispatch-avoidance) and B30
  `pad-uniform-pod-to-min-binding-size` (binding-size minimum, not tile reuse).
- `token-local-only-approximate-accelerator` vs `exact-op-parity-for-analytic-backward`
  (PHASE gate vs STATE-HORIZON gate) and vs the quantization/dequant-placement family
  (HOW to place the work vs WHICH consumers may take it).
- Independent corroboration encoded in-rule: the over-ceiling #3133 split diverged
  3/5 probes at 32K while each arm was internally byte-deterministic — the hazard is
  replay divergence, invisible at short lengths.

## GOAT (measured)

- **G1** parse: `batch84_pseudocode_parses` PASS (both rules, both shapes).
- **G2** lib: kernel_opt 966/0/1i (944 + 22 net), default 736/0, all-features
  1471/0/14i.
- **G3** clippy + CI guard: `cargo clippy --all-targets --all-features -- -D
  warnings` 0 errors in-repo + **ci_feature_guard.sh ALL LAYERS PASS** — after
  repairing 5 PRE-EXISTING latent breaks found by the guard (separate commit, the
  B72 precedent): the sibling kat-vote landing's `and_then` E0631, the IIFE
  `redundant_closure_call` + `match_like_matches_macro`, the B83 bench-spec
  `items_after_test_module`, kat_sync_payload doc backticks, traj_store float_cmp
  gates, groq test fixtures (needless_pass_by_value + float_cmp), kat_redact
  match_bool + doc backtick, and the t48 gp_acquisition LCG block.
- **Retrieval floors HELD zero decay:** kernel self-variants 100/100/100 (12th
  consecutive), blended 93.9/96.9/97.7 — 14/14 retrieval tests green.
- Count pins walked: entries.len 396→398 (tests L66), Plan-003 per-category table
  (D 84, N 22), the two sum asserts, the router doc header + experts-cover assert,
  mod.rs category comment headers + the history block, and this note + AGENTS +
  .docs + queue snapshot.

## Standing lessons

- **Research highwater is a shared mutable that MUST be re-read at write time even
  mid-batch:** `.research/098` was consumed by a sibling session's cflow verdict
  (Plan 046/Batch 85 queued) while this batch was in flight; this note renumbered
  098 → 099 and bumped the highwater to 100 (the 675/677/697 collision class,
  5th occurrence).
- The edit_file tool repeatedly failed to match the entries-file tail anchor (a
  200-line replacement with heavy escaped strings) — the B77 python-heredoc
  generator path landed it first try. String-heavy corpus entries: python, not the
  structured-edit tool.
