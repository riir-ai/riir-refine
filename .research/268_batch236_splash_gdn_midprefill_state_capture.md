# Research 268 — Batch 236: the splash GDN mid-prefill state-capture law

**Status:** RECORD — landed 2026-10-15 (shikuwa/4090 idle loop, Plan 238); the 2026-10-15 league watch pass's convertible lead (riir-ai `7e330ae45`).

## Provenance

Source: **incoai/splash** (Apache-2.0, Rust/C++ — the M3 qwen3.8 DFlash2 engine, non-scoring league reference), the four-commit GDN boundary-state capture series, all 2026-10-13, read at the pins this session via git-show on `E:/git/vendor/splash` (the blobless clone fetched blobs on demand):

- `711f8e54b47844f090827f8c885f8acdebdc3b6b` "Capture GDN states inside a prefill's rows" — the ops::GDN half
- `c3dcf53` "Capture boundary states inside prefill commands" — the runtime/scheduler half
- `985463c` "Reserve a cached state's buffers before the command that writes it" — the buffer law
- `79a4329` "Keep a lane's checkpoint until its successor is published" — the retention law

Convergence: **vLLM PR #60050** "[Perf][GDN] Internal prefill checkpoints for the FlashInfer GDN prefill backend" MERGED 2026-10-14T06:49:47Z @ merge `badfcdbf9` (19 commits, +794/−151, 12 files; Apache-2.0; PR body fetched via API this session), naming Mamba2 (#57329, merged) and Kimi-K3 FlashKDA (#52789/#53614) as the same mechanism — **four independent implementations**. The prism fork's `#358` "GDN rollback by replay" is the counter-solution class (replay instead of capture) for the same boundary problem.

## The law (one own rule, corpus 812 → 813, D category)

`mid-prefill-recurrent-state-capture-boundary-unsplit` (entries_gpu_95.rs):

1. **The cost shape** — every planned state boundary (replay point, rolling checkpoints, junctions, shared-prefill boundaries) ENDED a prefill command; the engine copied the lane's state after it. The replay point sits a few rows before the prompt's end → one extra small command PER TURN: 17-24 ms (35B) / 34-60 ms (27B) on M5 Max, source-reported.
2. **The capture mechanism** — the scan kernel keeps its recurrent state in registers and stores it only after its last row; `addPrefill` now takes capture rows + buffers and runs the prepare/scan dispatches once per run BETWEEN captures, each run storing its convolution carry and recurrent state *as a prefill ending there would*, the next run starting from them, the gate once over all rows. `No kernel changes` — the splitting is dispatch-level.
3. **The numerics contract** — the scan is sequential: fp32 runs compute EXACTLY what one run does; bf16 rounds at each captured state as a prefill ending there would (so a resumed command reproduces a fresh one bit-for-bit in either dtype). Cost: one prepare + one scan dispatch per GDN layer per capture.
4. **The buffer law** — buffers RESERVED before the command (snapshot decomposed into reserve/copyState/seal); "nothing on a refusal"; a dropped-unsealed reservation returns each buffer exactly once.
5. **The retention law** — a lane's checkpoint retires only once its successor is PUBLISHED (retire-before-write loses the checkpoint on cancel: 8 cancelled long prompts — all retried from the last published checkpoint vs main restarting 4/8 from the first token).
6. **The FlashInfer face** — the boundary state is emitted IN the same call (`state_checkpoints`, `every_n` = the batch offsets' gcd on a 64-token grid); TTFT −20%/−12%/−10% at c1/c4/c16 (GB200, Qwen3.5-35B-A3B, 8000-token prompts); "each emitted state matches a separate run truncated at that position (max abs err 5e-3), and the final state is bit-identical with checkpoints on"; blocks LCM-aligned to the checkpoint grid (1056 → 1088); **"A checkpoint off it falls back to the split prefill"**; **"the scheduler takes them only when every Mamba layer can"** (Qwen4Exp's PLE short-conv layer cannot export → the whole model keeps the split — all-layers-can admission, never partial capture).

## Dedup adjudication

- entries_gpu_08's GDN-state family (gdn-transition-replay-log-commit-many; the triangular-solve tree-verify rule): **speculative-verify state algebra** — a different problem (draft verification) on the same "a recurrent state is a lossy summary of the whole prefix" ground. Orthogonal; composes.
- rust-side B219 prompt-cache-rewind-states-min-gap-eviction: KV **rewind-state placement** — owns where states sit; this owns where states COME FROM (capture mechanics). The captured checkpoint becomes a rewind state — compose.
- B215 zero-anchored-prompt-grid-chunk-identity: the resumed-cut-equals-fresh-cut grid identity for CHUNKED passes; this owns the same identity for the sequential scan's capture points + the boundary-unsplit law. Kinship noted in-rule.
- B223 gpu-resident-control-loop-static-icb-replay: GPU-resident control vs host-scheduled boundary planning. Different residents.

## Deferred candidates (recorded, NOT authored)

- The splash SSD-cache volume-management series (`13f6e11`/`a212b81`/`1af85ef`) — persistence-tier laws (default-on SSD cache, volume bounds, write budget, stale-cache removal at start). Watch-grade; a future batch if a corpus family fits (the warm/cold tier family may).
- `3903390` "Compare the GDN rows budget and mask commits retain in the runtime oracle" — the content-vs-count oracle law (byte-compare retained GDN cells + rows between budgeted and masked commits; catches a 1-ulp bf16 row diff main's oracle missed). Kinship with B233's reshape-relabel content-oracle ("the comparison is on the contents") — different domain (runtime oracle vs layout); a future rider if a host rule fits.
- uzu `ce6298c` "speedup prefill" — GDN chunked `output_and_state.metal` + quantized-GEMM cursor/integer-schedule tuning. M3-lane intel; no law text in the commit; a deep-diff read is the M3 lane's call.
- vLLM `0fae39a0` (per-element divide → reciprocal multiply) — family-owned by entries_gpu_03's one-division law ("ONE post-loop sweep multiplies by a single precomputed reciprocal"). NULL on dedup.
- vLLM `c19c263a` (two elements at a time, per-token-group quant, +2.8% ITL) — the unroll/ILP face; the quant families own the surrounding law; NULL-to-rider at best, priced not worth a slot.

## Gates (all green)

| gate | verdict |
|---|---|
| G1 `batch236_rule_parse_and_needles_landed` | PASS — first run failed on the write-escape class (`\\'` in the corpus string produced a stray backslash — 9 occurrences, all repaired); the repaired run green first-try |
| G2 kernel_opt lib | 3109 passed / 0 failed / 1 ignored (= B235's 3108 + this test) |
| G2 default lib | 2659 / 0 (standing; the new test is kernel_opt-gated) |
| G3 clippy `-D warnings` | clean at default, `kernel_opt`, and `--all-features` (3 doc_markdown rounds on the `//!` header — the B192 class, 10th strike) |
| T8 retrieval floors (`--features kernel_opt,latent_retrieval --lib`) | 3295 passed / 0 failed / 3 ignored (1459.45 s) — HELD, no re-pin |
| T9 freeze/score-bench | NOT owed — zero clippy-corpus delta (P30) |
| counts | corpus 812 → 813; D 190 → 191; pin sites: tests.rs master count + batch116 + batch-227/231/232/233/234/235 asserts + the two Plan-003 sums + the per-category D row + router.rs expert-total |

Build isolation: `CARGO_TARGET_DIR=/tmp/riir-refine-b236` (removed post-verification).

## En-route lessons

- The write_file JSON-escape doubling (`\\'` → file `\\'` → string content `\'`): every authored corpus string with an apostrophe inside a QUOTED sentence needs a post-write grep for `\\\\'` — add it to the batch checklist.
- The `-` character in my `-*-D warnings` clippy runs: use plain `-D warnings` (the shell ate nothing, but keep commands ASCII-clean on this box — see the riir-ai commit-message mojibake this session, amended away).
