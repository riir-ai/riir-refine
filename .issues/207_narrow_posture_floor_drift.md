# Issue 207 — the narrow-posture retrieval floor silently decayed (kernel_route_narrow's blended top-3 sits below the shared pin)

**Status:** open — filed 2026-11-05 (zed-m3-i191t7, found by the Issue-191 T7 worktree validation); NOT a T7 regression — reproduced on CLEAN origin/develop (`3bc6a4d0e`) with T7's two files reverted out of the validation worktree (`riir-refine-i191t7`, `git checkout --` before both runs).
**Claimed:** zed-m3-i191t7, 1793841000, expires 2026-11-07T01:10:00+00:00 — filing + the re-pin attribution run only; the re-pin itself belongs to the next corpus lane that moves the blended numbers (the Issue-063/200 re-pin convention).

## The finding

`kernel_route_narrow` (the Issue-196 measured-negative lane, opt-in for replication, never default) FAILS the standing retrieval floor on clean develop:

```
blended top-3 0.90697676 below floor 0.9084507
```

- **Floor:** `floors::BLENDED_TOP3 = 129.0 / 142.0 = 0.9084507` (`src/draft/retrieval_eval.rs` — the shared floors module; the comments state the floors serve "the default+latent posture", Batch 328 note).
- **Measured (clean develop, narrow posture):** blended row `172 labeled · 87.2% · 90.7% · 91.9%` — top-3 ratio 0.90697676, one span below the shared floor. Byte-identical WITH the T7 scratch adoption (same ratio on both runs — the adoption is retrieval-neutral, as the substrate's `retrieve_for_scoring_into_matches_plain` contract pins).
- **Why it slipped:** the narrow lane is not in the standing per-batch gate list (batches run default + `kernel_opt` retrieval only — Batch 333's gates: "retrieval default 27/0 + kernel_opt 28/0"); since Issue 196/Bench 126 pinned the narrow posture's numbers, the corpus grew (rust_perf 292→300, kernel_opt 1049→1053, sec 44→45) and the narrow index's blended top-3 lost exactly ONE span to legitimate corpus growth — the same cross-pool-competition class the floors module documents for the flat posture (P7/B200/B332 notes).

## The repair path (per the Issue-063 convention, NOT a silent weaken)

The convention: floor = measured − ONE span, attribution measured pre/post in a sibling worktree (the B200/B327/B332 template). The next corpus-touching lane that runs the narrow battery owes either (a) the narrow-posture re-pin WITH the attribution note (which span demoted, to which rank), or (b) narrowing the floors module to the posture it actually serves (a narrow-specific constants block), so the replication lane stops piggybacking on the default posture's pins. Option (b) is the structural fix; (a) is the minimum honest step.

Until then: the narrow posture carries a KNOWN RED retrieval gate — documented here, never cited as green.
