# Plan 196 — Batch 198: the PrismML fewrow/mmvq pack (kernel_opt + rust_perf)

**Status:** COMPLETE (2026-10-02, shikuwa/4090 idle loop) — all gates green; record `.research/231_batch198_prismml_pack_conversion.md`; landing row in `.distill/001`.

Source: `PrismML-Eng/llama.cpp` fork (MIT), pins read from the standing vendor
clone `E:/git/vendor/prismml-llama.cpp` @ tip `88c4bc60b9`. The pack was staged
TWICE with quotes verified at the pins (`.distill/001` rows 2026-10-01 ~19:4x
"BATCH 198 STAGED" + 2026-10-02 "+19 BATCH MINED" — the same commits graded
from the M3 and the 4090 watch lanes independently). This plan converts it.

## Tasks

- [x] T0 — Pre-flight: origin twin check (done: HEAD == origin/develop at
      `3a976920`, no batch-198 rows on origin); vendor clone pin resolution
      (done: `88c4bc60b9` / `3285e757a` / `445fa8209` / `5244ceade` all
      resolve); `.plans/.highwater` repair 194→196 (195 exists on disk
      unstamped — the B196 session's bump was lost; allocate 196).
- [x] T1 — Coordinator quote re-verification at the pins (third pass —
      coordinator + both authoring subagents via `git show`; **0 quote
      corrections**; two brief-level items recorded in the entries module
      doc: the dropped-lead kin reference + the nonexistent B196 rule name).
- [x] T2 — Author `entries_gpu_66.rs` (kernel_opt, 5 rules):
      `fewrow-tensor-tile-decode-into-rhs-operand` (B),
      `weight-decode-amortized-across-output-columns` (A — adjudicated over
      staging-B, stated in-rule),
      `ids-path-rebinds-stride-meaning` (D, NotBenchable correctness),
      `tied-hadamard-output-fold-at-load` (N — adjudicated over D),
      `single-seq-slice-contiguity-copy-elision` (D).
- [x] T3 — Author rust_perf entry: `selector-lattice-beam-over-greedy-walk`
      (complexity; `entries_12.rs` — entries_11 was B196's; subagent
      verified the NaN→−inf + lexicographic tie-break claims at the pin,
      both PRESENT, kept).
- [x] T4 — Riders: (a) gpu_09 += the 445fa8209 ggml-Metal GDN 4-rows
      instance (third backend); (b) gpu_53 dequant-once += #254 AVX-512 FA
      tile + maddubs += #256 VNNI TR=8 instance reads.
- [x] T5 — Standing gates: `batch198_pseudocode_parses` (5/5) +
      `batch198_concepts_carry_their_load_bearing_contracts` (5/5 markers) +
      `batch198_riders_landed_on_their_hosts` (3/3 hosts).
- [x] T6 — Count pins: kernel 734→739 (A 157, B 78, D 165, N 57) at tests.rs
      ×4 + mod.rs slices/headers/exhaustive-comment + router.rs:472 +
      AGENTS.md ×2 + kernel_opt.md + README ×2; rust_perf 167→168 (annotated
      114, unannotated UNCHANGED 54, without_spec 69) at tests.rs ×4 +
      AGENTS.md + rust_perf.md; one 53/115 arithmetic slip caught by the
      pins and repaired with the honest label.
- [x] T7 — Retrieval floors: **21/21 HELD** (222s, isolated target dir
      `target-b198`, removed after).
- [x] T8 — Lib suites: kernel_opt lib **2830/0**, default lib **2424/0**
      (== baseline), rust_perf slice 284/0; clippy `-D` ×3 postures clean
      (9 doc_markdown mechanicals in the new files' headers healed by hand;
      the residual build-script hardlink warning = the known box artifact);
      freeze + score-bench NOT owed (zero clippy-corpus delta, P30).
- [x] T9 — Records: `.research/231_batch198_prismml_pack_conversion.md` +
      `.research/.highwater` 229→231 (230 existed unstamped — repaired),
      AGENTS.md batch lines, `.distill/001` landing row, plan COMPLETE;
      commit + push.
- [x] T10 — Upstream defect report adjudication: NOT owed — #258 IS the
      fork's own fix (the rule lands the decision content; the defect is
      upstream-fixed at the pin commit). Recorded so no session re-derives.

## Notes

- The +19 fork campaign context (line-7 staging row): #258 fixes a LIVE
  correctness race (test-backend-ops MUL_MAT_ID m=70 8-of-10 failures on RTX
  4090); #254/#256 are the CPU-side decode twins of B182's tiled family —
  instances, not new rules; #257 tied Hadamard is model-loading decision
  content; #284/#286 are the fewrow pack (1);
  #261/#272 (DFlash2 + server visibility) are skip-class infra.
- One-batch-per-day: B196+B197 landed 10-02 (same-day precedent disclosed
  both times); B198 is this box's NEXT slot — the landing commit carries the
  disclosure.
- GPU: idle for these gates (corpus work is CPU-only; the 607 training lane
  owns the GPU and is untouched).
