# Plan 315 — Batch 310: the 10-30 staged leads kernel_opt conversion

**Status:** IN PROGRESS — claimed 2026-10-30 ~06:5x +07 (zed-m3-b310); T0+T1 done pre-claim; authoring zero-cargo (Plan-337 — the i1117 T4 dense collection holds the box); **GATES DEFERRED — the batch does NOT land until G1–G3 + retrieval run at the next quiet window.**

## Scope

The 10-30 staged leads addendum (zed-m3-leads1030, snapshot line 1702+), converted
kernel_opt-only under the B293 cross-domain precedent: S3 (rust_perf) + R1
(rust_perf/concurrency) stay behind the live rust_perf claims (B301/plan-306 to
10-31T13:41Z) per the staged row's own queue line. Box posture: the i1117 T4
128K dense collection holds this box (PID 19526, ~10 cores, 15 GB RSS, hours
remaining) — Plan-337: claim/plan/dedup/source-verify/authoring are zero-cargo;
the gate layer is DEFERRED, never skipped. No unverified corpus reaches develop
(the amo 10-30 incident class: edits must not land before their gates run).

## Candidate slate (staged grades; T0 re-verified this session)

| # | Lead | Source | Grade | Disposition |
|---|---|---|---|---|
| S1 | `mma-accumulator-run-as-one-operand-depth-matches-instruction` — the A-operand RUN-DEPTH law (m16n8k8 runs at half the m16n8k16 rate; a run of accumulators laid into A recovers k16 with the same bits) | cubecl `07cd07a` #1975 + `4549408` #1971 (full `4549408587cf464cc2ae7b3b7bc0bcf6005b5a92` / `07cd07a2da9c4405044993c6ec2dc51b9d685508`) | B+ STRONG DISTINCT | OWN rule, category B (compute); kinship B205/gpu_75 (C-operand alias — compose, never merge; the source names the boundary: `MmaData::mma` "loses its accumulator arm") |
| S2 | `barriers-declared-before-slots-one-free-run` — SMEM declaration order as allocation-fragmentation control (64→131 KiB past a 99 KiB ceiling) | same commits (the `Stages::free` atom) | B STRONG DISTINCT | OWN rule, category E (parallelism/barriers, per the staged grade); kinship B234/gpu_94 (lifetime-disjoint sharing + the free-legality face — none owns declaration ORDER as the remedy) |
| S4 | `warp-per-row-small-blocks-cta-under-load-law` — blocks-per-work-unit < threads-per-CTA idles warps 1..N; warp-per-ROW admitted only while `blocks_per_row <= warp_size` (one block per lane keeps the butterfly); bit-identical by same per-row accumulation order | PrismML-Eng/llama.cpp `9d9efeadbdd1d77458d98717a96fbfa3a016e159` #415 (the NEW scored-tag tip prism-b10831) | B DISTINCT | OWN rule, category D (dispatch); kinship gpu_49 (the plain-matvec K-blocks-few crossover — S4 is the MUL_MAT_ID/ids warp-per-row remedy + admission floor), B180 planar (the `ptq1_0_pt_block_dot` call is the planar form), gpu_09 warp-partition. League disposition rides the riir-ai watch (L1) — NOT this batch |
| R2 | queue-only execution consumes in plan order — the pending/`CompileOnly` fallback is the THIRD consumption surface beside the measurement round and the recorded-tune replay | cubecl `a72eb4e` #1697 (full `a72eb4e83cff55ab81f9cbdf24f0d2706af07a3d`) | RIDER B− | count-neutral face on `autotune-candidate-priority-orders-try-first` (B286, gpu_130) |
| R3 | resume-length is the history the next turn VERBATIM repeats — the tool-turn closure is template-dependent, probed by an assistant-render, never assumed | TensorFold `17cea0d` + `6b6f890` #1551 (the `resume_len` client + the `continuedLen` server half) | RIDER B− | count-neutral face on `prefix-cache-entry-one-token-early` (B200, gpu_79 — kernel_opt confirmed) |
| S3 | `metadata-count-query-builds-no-structure` (build-only-to-count is the anti-shape) + NEW measured datum found at T0: "cubek's attention prepare ... goes from 12.7 to 9.6 us a call on an RTX PRO 6000 host (10.3 before that change)" | cubecl `07cd07a` (the `StageStorage::bytes` atom) | B− rust_perf | DEFERRED behind B301/plan-306 (rust_perf head, to 10-31T13:41Z); the datum rides the deferral |
| R1 | `allocation-window-resolved-at-stream-birth` (pending map keyed by the absent target = nested-scope loss) | cubecl `17f429b` #1963 | B− rust_perf/concurrency | DEFERRED behind B301/plan-306 ("rider-or-small-own, home adjudicated at claim" — adjudicated rust_perf, foreign-claimed tier) |

## Tasks

- [x] T0 sources fetched as per-commit `.patch` into `.raw/b310/` (cubecl 07cd07a /
      4549408 / 17f429b / a72eb4e, prism 9d9efea, tf 17cea0d / 6b6f890); every staged
      quote byte-verified against the patches (S1 run-depth body + `accumulators_as_a` +
      "loses its accumulator arm"; S2 barriers-before-slots + the 64/131/99 KiB sentence;
      S4 commit body + `blocks_per_row_x <= warp_size` in-diff; R2 in-diff comment +
      `order()` contract + the three tests; R3 `resume_len` + `continuedLen` verbatim)
- [x] T1 dedup greps at 3+ vocabularies over the kernel_opt corpus at origin `103ea51b`:
      S1 — B205/gpu_75 owns the C-operand alias (kinship), ZERO run-depth owners
      ("run-as-one-operand|run_depth|k16|m16n8k" → none); S2 — B234/gpu_94 owns
      lifetime-disjoint sharing + free-legality (kinship), ZERO declaration-order owners
      ("fragmentation|declared before|declaration order" → none); S4 — gpu_49 owns the
      plain-matvec K-blocks-few crossover (kinship), B180 the planar call, gpu_09 the
      warp-partition family — the ids-path warp-per-row admission floor unowned;
      R2 — B286/gpu_130 confirmed owner; R3 — B200/gpu_79 confirmed owner (kernel_opt)
- [ ] T2 authoring: entries_gpu_141.rs (3 own rules — pattern, bounded fix-space,
      category, rationale, source quotes, distinct-from kinships, grep evidence) + 2
      count-neutral rider faces (gpu_130 R2, gpu_79 R3); fixtures carry no string
      literals (the B274 lesson); needles keyed to landed text AFTER entries are
      written (the Batch-259 lesson)
- [ ] T3 wiring: mod.rs mod-decl + extend + the B/D/E slice rule_id lists +3; the D/E
      slice headers repaired to measured truth while being touched (240+1 / 94+1 — the
      Research-353-flagged stale-header class, the B309 repair precedent, miss disclosed)
- [ ] T4 count pins: tests.rs count-pin sites (1023 -> 1026; B 116->117, D 240->241,
      E 94->95) + the category-sum + the grouped-total + the router expert-total +
      AGENTS.md + README + .docs/11_domains/kernel_opt.md
- [-] T5 GATES — DEFERRED (Plan-337: i1117 T4 holds the box, ~10 cores / 15 GB RSS,
      hours remaining; a cargo build steals CPU from the measurement and risks the swap
      tax): G1 batch310 needles+parse; G2 kernel_opt lib + default lib; G3 clippy -D x3
      postures; retrieval floors both postures; freeze/score-bench NOT owed (zero
      clippy-corpus delta, P30); binary rebuild NOT owed (kernel_opt outside the release
      set). UNBLOCK: the i1117 collection completing + a quiet-box check
      (ps/uptime witness in the landing row)
- [ ] T6 records: Research note (.research/354); AGENTS.md; snapshot landing row + the
      S3/R1 deferrals + the C1-C8 consumed rows' standing (no re-mine); this file's
      tasks ticked
- [ ] T7 landing commit `feat: batch 310: ...` + push; `.raw/b310/` + the worktree
      removed at close

## Gates, criteria, risks

- The embedding-quote defect class (B299's twice-fired lesson): never replace an old
  prefix WITH its leading quote when appending rider notes to gpu_130/gpu_79.
- The category E assignment follows the staged grade (barriers/parallelism); A would
  also fit (smem allocation) — the staged grade stands unless a gate argues otherwise.
- The league leg (S4 = the b10831 tip) is INTEL here — the riir-ai watch owns the
  bout disposition; the rule text stays league-neutral.
- Worktree-isolated authoring (shared checkout stays clean at origin; the i1117 lane
  and any sibling commit freely); the landing is gated on T5's UNBLOCK condition.
- `.plans/.highwater` was STALE at 313 while 314 existed (the B309 session's miss) —
  repaired to 315 by this claim's commit.

## Session

zed-m3-b310, epoch 1793317704 (2026-10-30 ~06:48 +07). Claim expiry 48 h
(2026-11-01T~23:48Z); the push is the arbiter.
