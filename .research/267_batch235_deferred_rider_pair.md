# Research 267 — Batch 235: the B234 deferred rider pair (count-neutral)

**Status:** RECORD — landed 2026-10-14 ~23:xx +07 (M3 idle loop, Plan 237); claim `d5233464` discharged by the landing commit.

## Provenance

Source: ggml-org/llama.cpp PR #29353 "cuda, hip: GDN chunked kernel" — MERGED 2026-10-14T14:00:59Z @ merge `62df1815`, head `dde6a4d93`, MIT (7 files +564/−14). This batch converts ONLY the two rider faces Batch 234 (Plan 236 / Research 266) deferred as `- [-]` T5 — the merge-moment core (six own rules, corpus 806→812) landed last night. The `.diff` was RE-FETCHED this session (never trusted from the prior session's read); every needle verbatim at the fetched surface.

## The two riders (both count-neutral — corpus stands 812)

### Rider (a) — the width-matched loader face → `ldmatrix-pair-x2-to-x4-adjacency` (entries_gpu_37.rs)

mma.cuh's shared `load_ldmatrix[_trans]_swizzled` loaders widened `static_assert(I == 16)` to `static_assert(I == 8 || I == 16, "bad tile width")` and split the PTX by tile width: I==16 keeps `.x4.b16` (four destination registers), I==8 issues `.x2.b16` (two). The lane map must WRAP for the narrow width — verbatim `const int j = ((threadIdx.x / t.I) * (t.J / 2)) % t.J;` — because the `(threadIdx.x / t.I) * (t.J / 2)` product overruns `t.J` once I < 16 (the wider lane groups compute j = 8 and j = 12 outside an 8-wide tile's column span; the modulo folds them back). The trans twin pins the 16-row addressing instead — `const int i = threadIdx.x % 16; const int j = I == 16 ? (threadIdx.x / 16) * (t.J / 2) : 0;` — and takes `.x2.trans.b16` at the narrow width.

Decision content: an x4 under an 8-wide tile would stage two 8x8 matrices the tile never consumes, with lane groups addressing rows outside the tile's span — width-matching is the load-correctness precondition, not an issue-count nicety; the swizzle math stays the only address transform on either width. Composition with the host rule (the OPPOSITE direction, per Research 266's kinship note): pick the width the tile needs first (per-loader), then fuse atom-adjacent loads when two x2s are pairable (per-pair) — both reduce ldmatrix issue count without changing any staged byte.

### Rider (b) — the state-slot guarantee face → B224 `moe-duplicate-id-compact-prepass-static-bound-tile-dispatch` (entries_gpu_88.rs)

The ggml.h declaration of `ggml_gated_delta_net` gained, beneath its slot-semantics comment, the dictated narrowing, verbatim: "when n_tokens > K only slot 0 is guaranteed; backends may leave slots 1..K-1 unwritten." Its consequence is REMOVAL, not the B233 face's disable-behind-literal: test-backend-ops.cpp drops both rows that pinned overflow behavior (`/*K=*/3` and `/*K=*/4` test_gated_delta_net rows) with their `// overflow: n_tokens > K — only the last K snapshots kept.` framing comment.

The family law, now two-faced: an API contract that states what callers may rely on SUPERSEDES any test that pins beyond it, and the disposition follows the contract's intent — REMOVE when the behavior is deliberately unspecified across backends (a surviving test would over-pin the guarantee it just lost), DISABLE-behind-literal when the behavior is desired and support is pending (the B233 duplicate-id face — those tests stay as the re-enable gate). Consumer-side corollary: reading slots 1..K-1 under n_tokens > K is contract-violating, not merely unportable.

## Gates (all green, first runs)

| gate | verdict |
|---|---|
| G1 `batch235_riders_parse_and_needles_landed` | 1/1 PASS first run (needles verbatim, both hosts found, corpus == 812 asserted) |
| G2 kernel_opt lib (full) | 3108 passed / 0 failed / 1 ignored (= B234's 3107 + this batch's test) |
| G2 default lib (full) | 2659 passed / 0 failed (== B234's standing count; the new test is kernel_opt-gated) |
| G3 clippy `-D warnings` | clean at default AND all-features (the `block v0.1.6` future-incompat note is the pre-existing third-party dep notice, not a warning) |
| T7 retrieval floors | 21 passed / 0 failed (307.15 s) — HELD, no re-pin |
| T8 freeze/score-bench | NOT owed — zero clippy-corpus delta (P30; the B229/231/233 rider precedent) |
| T9 counts | count-neutral — corpus stands 812; NO pin edits (tests/mod/router/AGENTS counts/README untouched) |

Build isolation: `CARGO_TARGET_DIR=/tmp/riir-refine-b235` (sibling lanes live on the box; removed post-verification).

## En-route

- Zero divergence, zero repair: the rider text went in through pre-escaped edits (the B233 python-replace escape lesson avoided by construction — no python replace used).
- Plan 236's T14/T15 (snapshot landing row + commit/push) were verifiably landed unticked (the landing row IS in `.distill`, commit `c83c323f` IS pushed) — hygiene-ticked in this batch's commit (the plan-230 T1/T5 precedent).

## Queue after

mlxfast floor ~10-17 claim-first · walk #15 proper ~10-20 claim-first (fresh-week window) · ai 1017 readout ~10-21 · nnethercote ~11-01 · watch 971 DISCHARGED (the merge consumed by B234 + this rider pair; the fork-tag bout + the ai-1066 port are the 4090/perf lane's). NOTHING INVENTED — every quote verbatim at the fetched `.diff` this session.
