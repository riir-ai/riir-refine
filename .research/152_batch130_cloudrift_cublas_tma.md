# Research 152 — Batch 130: the CloudRift cuBLAS-5090 TMA distill

**Status:** RECORD (2026-09-08, M3 box, CPU-only; landed with Plan 111)

## Source

- Blog: CloudRift "Surfacing a 60% performance bug in cuBLAS" (Dmitry Trifonov,
  2026-04-06) — the narrative: a learning-exercise TMA SGEMM kernel "beat cuBLAS
  50-60%" in batched mode on the RTX 5090, which turned out to be a MISSING
  DISPATCH LADDER in cuBLAS's 5090 batched FP32 path, not kernel genius.
- Code: `cloudrift-ai/emmy` @ `2aaa0328` (Apache-2.0) — the blog landing commit
  "Mini GPU compiler: TMA SGEMM kernel beating cuBLAS on RTX 5090" (#89,
  2026-04-09). **Provenance corrections at mine time:** the queue lead's
  `DeploDock/Emmy` path 404s (the org is `cloudrift-ai`, located via code search
  for the `deplodock bench` CLI string); the repo was restructured by `8764cc2e`
  (#100, 2026-04-29) so the blog's `recipes/sgemm_cublas_vs_tma/` +
  `deplodock/compiler/cuda/lower.py` exist only at the pin. Fresh clone, HEAD ==
  pin, LICENSE verified, removed at landing.

## What landed (5 rules + 7 riders, corpus 575 → 580)

| Rule | Cat | One-line decision content |
|---|---|---|
| `vendor-dispatch-verify-before-outsource` | D | A suspiciously-good win over a vendor baseline is a dispatcher-bug signal: verify ncu kernel name + FMA-pipe util + same-binary cross-SKU behavior before claiming; no escalation at ANY threshold = the ladder is missing, not mistuned |
| `sub-ms-bench-boost-clock-never-engages` | D | Below ~1 ms/call the boost governor never engages; interleaved-pair iterations + iter-0 discard + median + ratio-not-absolute + clock sampling; clocks deliberately unlocked |
| `lds-cluster-cross-warp-phase-not-static-distance` | B | Once static latency hiding suffices, the LDS temporal distribution governs cross-warp phase: clustered LDS halves dispatch_stall (44%→22%); generator-side cause is emission order |
| `tma-single-issuer-bulk-tile` | A | One thread issues the whole 2D tile; mbarrier byte-count completion + parity double-buffer; honest win = instruction count + simplicity, not FMA pipe; ragged-K truncation caveat in-rule |
| `multiplexed-grid-axis-partition-modes-exclusive` | E | Two partition modes on one grid axis (split-K × batch on blockIdx.z) must be refused at config time — the runner's precedence silently corrupts |

Riders: `l2-grouped-tile-schedule` (SWIZ=8 formula + padded launcher + DRAM-parity
boundary), `unroll-fixed-inner` (partial-unroll monotonicWORSE / no-unroll 57%
boundary), `autotune-warps-stages` (launch_bounds 20×-spill negative +
tensor-pipe family caveat + triple-buffer same-IPC ceiling), `cpasync-multistage-tile-pipeline`
(82.6% crossover + try_wait first-read), `hardcoded-fast-path-jit-specialization`
(codegen #define/#if twin, honest 98→101% magnitude), `launch-heuristic-pinned-to-calibration-arch`
(CC insufficient SKU key + DMA-generation tile optimum + same-CC library-side
divergence), `fast-math-contraction-behind-feature-flag` (fmad = exact half,
split from use_fast_math).

## Blog-vs-repo corrections (the honest-provenance half)

1. The negative-results table (82.6% cp.async, 57% no-unroll, launch_bounds 20×,
   triple-buffer same-IPC, DRAM 32→8.5%) is **blog-only** — none of it appears in
   the pinned repo (grepped docs/, recipes/, scripts/). Every rider carries the
   "blog-reported; NotBenchable at pin" caveat. Repo-verifiable numbers were
   preferred wherever both exist.
2. The blog's "260→460 MHz / 84% variance" figures are not in the repo; the
   repo-verifiable tells are ~260→~2400 MHz (recipe.yaml) + 562→1890 MHz and
   682→810 MHz samples (run reports).
3. Pro 6000 "escalates correctly" is generous — it has its OWN dispatcher bug
   (magma_sgemmEx legacy path at 256/512, 32% util); the repo counts "three
   distinct cuBLAS dispatcher bugs".
4. The FP16 `cutlass_80_tensorop_h16816gemm` kernel name is not in the pinned
   catalogues (the dump truncates tensorop at head -10) — dropped from the rules.
5. The mbarrier try_wait "exits on first read" behavioral claim is blog-only
   (the 44 KiB `bytes=45056` is repo-verified); the rider says so.
6. Lane B found a caveat the blog omits: the emitted `nt = K/bk` integer
   division **silently truncates ragged K** — the TMA kernels are exact-FP32
   only for K multiples of the tile K (M/N ragged edges ARE handled via OOB
   zero-fill + bounds-checked stores). Carried in the tma rule.

## League intel (named-not-edited — sibling acts)

- The FP16-path note (Blackwell tensor-core tuning goes to NEW low-precision
  formats — FP8/MXFP4 — while Ampere `cutlass_80` kernels are reused across most
  compute paths on sm_120; FP16 IS tensor-accelerated with the SIMT pipe <0.2%)
  is league-intel adjacent: riir-ai 884's opponent autopsy reads it directly.
- The dispatcher-bug recurrence class (2018 Maxwell 2×-slow forum report, vLLM
  #35467 "7th-best tile, 16% left on the table") is the standing caution for any
  league cell measured against a vendor baseline on consumer silicon: verify the
  dispatched kernel before recording the opponent number.
- Same-author follow-ups QUEUED as future leads: "Modern GPU Matmul
  Optimization" (2026-05-29) and "Learning FlashAttention the Hard Way" Parts
  1-2 (2026-07-06/08) — later commits in the same repo (the evolved
  `emmy/compiler/` surface carries mbarrier/interleave passes worth reading).

## GOAT (all green)

- batch130 gates 3/3 FIRST-SHOT (rules_present / concepts_covered / pseudocode_parses)
- lib default 1168/0; kernel_opt 1496/0/1i (floor 1493 + 3, zero re-pins);
  kernel_opt+latent 1613/0/2i (floor 1610 + 3)
- retrieval kernel 100/100/100, blended 94.7/96.9/97.7 — byte-identical to
  B118-B129
- Router annotation re-pinned per the Issue 020 contract: re-measured **93/131
  (71.0%)**, was 88/131 at B129 — the rich TMA/dispatch concept text SHARPENED
  the TF-cosine centroids (first measured IMPROVEMENT since the B128-era 94);
  floor 83 → 88 per measured−5
- clippy 0 ×3 states (default / kernel_opt / kernel_opt+latent)
- score-bench #79 FLAT 96.2/1.9/100/100

## Count pins walked

Totals ×5 (tests.rs ×4 + router.rs) + per-category expected-array A/B/D/E +
mod.rs KERNEL_CATEGORIES membership (verified post-write by re-parse: 580) +
5 STALE category-count comments repaired (C 45→46, H 27→28, L 14→15, M 18→19,
N 41→42 — the B129 walk updated the arrays but missed its own comment pins) +
README ×2 + AGENTS ×1 + .docs ×2 + batch23/batch29 FULL-STRING pins regenerated
with riders appended + 3 terse-concept exact pins converted to prefix+rider form
(the B125 lesson).

## Lessons

1. **Regex-based rider appliers corrupt entries.** A non-greedy
   `(gpu_concept: ")(.*?)("\.to_string\(\),)` capture doubled concept text in 3
   of 7 targets (autotune/unroll/l2-grouped in entries_popcorn.rs + 2 more) while
   4 landed clean; the corruption survived until the full-string pins +
   junction-verify caught it. The fix is LINE-ANCHORED splicing (find the
   `rule_id:` line, take the next `gpu_concept:` line, strip the known suffix)
   + a junction check (the 25 chars before ` RIDER (Batch 130` must end with the
   concept's natural tail). Recorded as the standing rider-landing protocol.
2. **A wrong doc-lint guess costs a clippy round:** the doc_markdown offender in
   the new headers was "CloudRift" (mixed-case), not cuBLAS — read the caret
   span before fixing.
3. The kernel_opt corpus tests are `kernel_opt`-gated: a default-features
   `cargo test batch130` reports a green ZERO (1168 filtered) — the count floor
   and the batch gates live at `--features kernel_opt` (the cfg-gated
   green-zero trap, per the AGENTS.md cfg-gated rule).

## Pins now current

`cloudrift-ai/emmy` @ `2aaa0328` (the blog landing). The repo tip (4735c451) is a
DIFFERENT surface (the restructured `emmy/compiler/`) — a future mine of the
same-author matmul/FA articles re-clones at the then-current tip.
