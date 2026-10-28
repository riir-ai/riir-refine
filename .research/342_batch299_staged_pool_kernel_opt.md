# Research 342 — Batch 299: the staged kernel_opt pool (TensorFold + uzu + cubecl + llama.cpp)

**Status:** RECORD — batch landed (Plan 304; claim `cfb7babe` discharged)

- **Batch:** 299 (kernel_opt), corpus **986 → 994** (+8 own rules, +1 count-neutral rider face), `entries_gpu_134.rs`.
- **Method:** two parallel coordinator proposal lanes (A: TensorFold+uzu; B: cubecl+llama.cpp) over blobless clones at the lead pins, every quote re-verified byte-for-byte by the coordinator against the pinned trees (plan 304 T2/T3, 2026-10-28, M3 box) — the B275 verify_quotes discipline. Dedup-at-mining greps (3+ vocabularies per candidate) ran BEFORE authoring; the R6 degenerate-dim space returned ZERO corpus hits (novel), the others carry kinship cites in-rule.
- **Sources:** ashhart/TensorFold @ 893c8d7b + 85571acb (Apache-2.0), trymirai/uzu @ 13b138eb #1022 (MIT), tracel-ai/cubecl @ a5e81431/f4055ea/01ae331/2774afa/50041a2 (Apache-2.0 OR MIT), ggml-org/llama.cpp @ de5122d879 #31146 (MIT). Clone set `.raw/b299/` removed at close (the ephemeral law).

## Landed (8 + rider)

| # | Rule | Cat | Source | Grade |
|---|---|---|---|---|
| 1 | `stream-table-consolidates-short-kernel-storms` | D | TF 893c8d7b | HIGH |
| 2 | `table-reached-buffers-declare-residency` | A | TF 893c8d7b | rider-grade, stood alone (Metal-specific, cross-cutting) |
| 3 | `mma-window-admission-output-width-occupancy-floor` | E | TF 85571acb | MED-HIGH |
| 4 | `adjacent-step-kernels-fuse-mode-specialized` | C | uzu 13b138eb #1022 | MED |
| 5 | `contraction-partials-sum-once-at-drain` | B | cubecl a5e81431 #1937 | HIGH |
| 6 | `extent-one-dim-skips-every-layout-check-symmetrically` | N | cubecl f4055ea + 01ae331 #1947/#1936 | MED-HIGH |
| 7 | `compile-run-rejection-persists-patience-compiles-on-reach` | K | cubecl 2774afa #1942 | MED-HIGH |
| 8 | `stage-fill-shared-physical-axis-reads-through-projection` | A | cubecl 50041a2 #1940 | MED |
| R | `fusion-admission-rechecks-upstream-contract-widening` face on `fused-read-write-dst-input-overlap-refusal` (gpu_109) | — | llama.cpp de5122d879 #31146 | count-neutral rider |

## Adjudications (drops + folds)

- **llama.cpp #31255** (pragma-unroll removal from conv2d stage-fill loops): **DROPPED** — zero measurement in the commit, no admission predicate beyond the pragma itself, no magnitude story; the nearest host (`constexpr-unroll-capacity-bound-zero-guard-holds-array-in-registers`, B268/O) owns the WIN direction with a capacity bound — a directionless removal face would dilute it.
- **llama.cpp #31230** (TOP_K_MASK Metal port): rider faces only — the FC-dropping half folded into rule 4's kinship; the cache-key half is `pipeline-cache-key-covers-every-function-constant` (B293, already owned); the sentinel-fill-then-TG-overwrite half is a fill/memset micro-atom below the corpus bar (recorded in-rule, not landed).
- **cubecl ed21f81 #1941** (zero-sample disqualifies its candidate): RECORD-ONLY corroboration — the second independent instance of unmeasured-must-not-win, feeding the B284 instrument-validity family's provenance strata (cross-domain: the timed-zero refusal lives in rust_perf entries_50).

## En-route findings (honest)

- **The count-pin surface is larger than the landing docs describe**: 37 corpus-count asserts across tests.rs (not 5) + the Plan-003 per-category table + router.rs's total. All re-pinned this batch (986 → 994); every assert now carries the batch-299 chain note. The batch-count-pin grep lesson: grep for the count VALUE alone misses pins formatted `total, 986,` vs `986,` vs mid-test — grep the whole tests.rs + router.rs + the Plan-003 table for the exact prior number.
- **The embedded-quote defect class (twice in one batch)**: prepending a chain note by replacing `"batch 297 re-pin: ...` (quote included) with `new-note + old-prefix` embeds a closing quote mid-string — 5 sites then 32 sites, the same defect twice because the first fix was manual and not generalized. The standing fix: the chain-note replacement must DROP the leading quote of the old prefix (`prior: batch 297 re-pin: ...`), never carry it.
- **Needle drift**: one batch-gate needle paraphrased the source doc ("runs along" → "runs along" vs actual "run along"); caught by the gate itself at first run — the needles-are-verbatim discipline held (fix the needle, never the corpus quote).
- **Lane-B citation drift**: the subagent cited `rank-partial-fp32-travel-round-once-at-epilogue` as "gpu_64" — the corpus grep put it in entries_gpu_102. The coordinator's grep wins; cite only verified file numbers.

## Gates (all run on the M3, RUSTUP_TOOLCHAIN=1.99.0 per the 963-floor override, CARGO_TARGET_DIR=/tmp/b299-gate)

- G1: `batch299_rules_landed` + `batch299_pseudocode_parses` (all 8 ref_shape + opt_pattern parse) — PASS.
- G2: kernel_opt lib **3265/0**; default lib **2749/0** — PASS (includes the 37 count pins, the Plan-003 table, router total, uniqueness, category wiring).
- G3: clippy `-D warnings` × 3 postures (default / kernel_opt / all-features) — PASS (4 pedantic doc-backtick findings on the new header fixed en route).
- Freeze NOT owed (kernel_opt-only, the P30/B293 precedent); score-bench NOT owed (no rust_perf delta); binary rebuild NOT owed (kernel_opt outside the release set).
- Retrieval re-measure BOTH postures (T7): `kernel_opt,latent_retrieval` 28/0 (279.55s) + the Layer-5b2 default posture 27/0 (66.16s) — floors HELD, no re-pin.

## Queue state

Staged pool AFTER this batch: the rust_perf rows (TF borrowed-pool fallback row 20; bevy #26336 row 7) — behind the rust_perf slot (bbtree B298 lane; the perfect-hash lane took nominal Batch 300); the measurement rows (vllm Zipf/warmup 8–9); the serving rows (TF 17–19); the numerics row (TF relu2 NaN 16); the p11 lower-tier (vllm #58305 not-actionable). Nothing kernel_opt remains staged — the pool is EMPTY; refill = the next item-11 sweep or the nnethercote window.
