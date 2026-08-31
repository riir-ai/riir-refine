# Plan 053 — Batch 92: ktransformers Marlin kernel_opt distill (Research 107 consumption)

**Status:** DONE — landed 2026-08-31, commit `66f1c8d` (pushed origin/develop).

## Source

- [Research 107](../.research/107_KTransformers_Graph_Stable_Gemm_Distill.md) —
  kvcache-ai/ktransformers @ `21095b9`, the modified Marlin W4A16 GEMM in
  `archive/csrc/custom_marlin/` (verdict kernel_opt A−). Raw clone at
  `.raw/ktransformers` (gitignored, disposable).
- Candidates: 14 queued in the snapshot; **this batch = the Marlin cluster
  (candidates 1–11 + the repack rider #12)** — all from the 2-file cluster
  `gptq_marlin.cu` (2034 L) + `gptq_marlin.cuh` + `gptq_marlin_dtypes.cuh`.
  Candidates 13–14 (kt-kernel CPU-origin K-blocking) DEFER to a future
  CPU-lane batch (different file cluster; the protocol's 1–2-file scope).

## Dedup verdict (grep of /tmp/krules.txt, 400 existing ids — done pre-plan)

- **No existing coverage:** cp.async pipeline, ldmatrix, lop3, xor-swizzle,
  stripe/semaphore reduction, multi-pointer break, scale-fetch gating,
  reg double-buffer, k-block sizing.
- **Partial coverage — differentiate in-rule:** candidate 1 vs
  `cuda-graph-scalar-via-device-pointer` (B22 — scalar plumbing; ours adds
  runtime m-block dispatch + grid=f(sms) M-independence); candidate 6 vs
  `byte-perm-lut-via-permutation-immediate` (B20 — general perm-LUT; ours is
  the int8→half2 dequant recipe with magic exponent embedding); candidate 7 vs
  `vectorized-smem-write-no-bank-padding` (B79 — the padding REFUTATION that
  already cites swizzle as the complement; ours encodes the static-index-table
  mechanism); candidate 10 vs `l2-grouped-tile-schedule` (scheduling for L2
  locality vs cross-block K-split reduction).
- **Folded:** candidate 12 (`pipeline-machinery-for-pure-movement`) lands as a
  rider inside candidate 2's gpu_concept (same machinery, different caller).

## Tasks

- [x] T1 Plan + dedup (this file) + subagent sweeps dispatched (2 parallel).
- [x] T2 Subagent proposals re-verified against the real files (quote check,
      coordinator reads every cited line range — 20 regions, all byte-accurate;
      incl. the source's own `maintining` typo kept verbatim).
- [x] T3 Corpus entries: `entries_gpu_09.rs` (rules 401-411, 11 rules),
      wired into `build_kernel_seed_corpus` (via the target/gen_b92.py
      generator; script deleted post-run).
- [x] T4 Category wiring: KERNEL_CATEGORIES rows updated
      (A+5 B+1 E+1 L+1 M+1 N+2), per-category count comments + header
      counts bumped (incl. the pre-existing Fusion 35→36 drift fix).
- [x] T5 Standing gate: `batch92_pseudocode_parses`
      (cfg(not(kernel_verify)), the B70 lesson) — PASS 1/0.
- [x] T6 Count pins walked: tests.rs 69 (400→411), 5362/5364/5375,
      router.rs 9/461, mod.rs header doc, AGENTS.md,
      .docs/11_domains/kernel_opt.md, per-category count test.
- [x] T7 GOAT (measured): G1 parse gate PASS; G2 kernel_heal lib 1234/0;
      default 1001/0; triple-state 1370/0/8i; clippy 0 self-crate in
      kernel_heal / kernel+latent / all-features; retrieval floors 14/14
      HELD (no re-pins). FULL ci_feature_guard: Layers 1-5b ALL GREEN;
      Layer 5c 47 targets green + 8 tail targets green + ONE failure:
      `t48_gp_acquisition_ab::ordering_ab` — PRE-EXISTING FLAKE, proven
      at clean origin/develop via a detached sibling-resolving worktree
      (3 fail / 5 runs at HEAD vs 1 fail / 3 runs in my tree — same
      signature): the verdict re-derivation straddles the
      noise-equivalence boundary (hits3 diff <= 1 vs 2 out of 100 pools
      flips `NEGATIVE` <-> `NEGATIVE-noise-equivalent` run to run; the
      recorded constant is one of the two, so ~40% of runs fail either
      tree). NOT caused by the corpus growth (direction unchanged: gp
      still loses to retrieval, z=8.488); NOT re-recorded (re-recording
      on a flaky measurement would be dishonest) — owner follow-up: pin
      the A/B seeds / pool construction so the sub-classification is
      deterministic, then re-record once.
      **RESOLVED 2026-08-31 (same-day follow-up, M3 box):** root-caused
      NOT to the test but to the SUBSTRATE — `corpus_index.rs::rank_into`
      sorted a HashMap-drain order by score with no tie-break, so two
      equal-scored rules landed in per-process (RandomState) drain-luck
      order and the `top_k` truncation boundary flipped the pool SET;
      the GP arm's σ² ordering (a different key) then moved the label
      across the rank-3 boundary while retrieval/random stayed stable —
      exactly the observed signature. Fix: canonical rule_id tie-break
      (score `total_cmp` desc → rule_id asc — the same key order
      `rerank`/`canonical_rank` already apply) in `rank_into` +
      `SpanSuggestion::all_rules`; gates: `ordering_ab_is_process_
deterministic` (5 fresh-matcher runs, the falsifiable pin) +
      8× fresh-process runs 8/8 green (was 2/5 fail); verdict constant
      UNCHANGED (canonical re-derivation = NEGATIVE-noise-equivalent, gp
      @3 0.240 / b=74 c=1 z=8.429 — doc numbers refreshed); floors HELD,
      libs green both states, clippy 0 both states.
- [x] T8 Research 107 status → LANDED (batch 92 refs); snapshot updated
      (candidates 1-12 consumed, 13-14 remain); AGENTS.md B92 entry;
      .docs/11_domains/kernel_opt.md total + table row.
- [x] T9 Commit `66f1c8d` pushed origin/develop (fast-forward, no
      sibling divergence); CARGO_TARGET_DIR /tmp/b92 cleanup pending at
      session end.

## Risks / lessons carried in

- Categories live ONLY in KERNEL_CATEGORIES — never in entries files.
- The B37 line-initial `+ `/`- ` doc-markdown lesson; backticks around
  Marlin/`cp.async`/file names in `//!` prose.
- Category-membership anchors verified by ROW INSPECTION, not rule-text
  memory (B51 lesson); count-pin comments are plain `//` inside fns.
- Sibling WIP check before builds: `git status` in sibling repos; shared
  checkout — commit only my paths (`git diff --cached --name-only | grep -v`
  guard).
