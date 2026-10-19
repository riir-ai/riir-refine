# Research 295 — Batch 259: the uzu #978 chunked-GDN prefill pack + cubecl/llama.cpp riders

**Status:** RECORD — landed as Batch 259 (Plan 263), shikuwa/4090 idle loop, 2026-10-20.

## Source set

| Source | Pin | License | Role |
|---|---|---|---|
| trymirai/uzu #978 | `e744e7cf4dc52e14f127d201aa04d24efb07a0c7` | MIT | THE LEAD — 3 own rules (A/B/C) |
| tracel-ai/cubecl #1853 | `dbaf9b4fdf2740c8251556136c69b07ca570623c` | Apache-2.0/MIT | rider — smem-interleave legality |
| tracel-ai/cubecl #1858 | `adf237883b21479b2ec44aa229adbd7c0b38e7f1` | Apache-2.0/MIT | rider — Metal simdgroup lane layout |
| ggml-org/llama.cpp #30666 | `f174553e1621c6a22a64e7bfcbb58d3a5c49e6fb` | MIT | rider — SWA checkpoint validity |

The staged pool came from the 2026-10-19T18:4xZ item-11 upstream movement sweep
(snapshot row), which graded the uzu pack B and the three riders B−/rider-grade
at their diffs. Every quote re-verified at the pins this session: Slice A
machine-checked 73/73 curated fragments (pin tree via `git show e744e7c:<path>`
— the clone tip `e6399cb` was one unrelated commit ahead, disclosed; removed
hunks cited from the diff); Slice B verified every anchor with
`git grep -n <pattern> <sha>` at the pinned commits.

## The rules

### A — `tile-history-packed-in-place-register-window-conv-scan` (A 185→186)
The prefill causal conv1d materializes NO padded buffer. `DeltaNetConvPack`
saves only the tile frontier (KERNEL_SIZE−1 rows per token tile → packed
`[CONV_TILES, K-1, conv_dim]` scratch, O(tiles), constant in the token axis);
`DeltaNetConvScan` runs the 16 tiles IN PLACE on in_proj with a register
window — tiles independent post-pack, the pack-ALL-before-any-write phase law
(interleaved pack-after-scan provably corrupts; the fixture pins it). Conv
stage 460→47 µs @T=1024. HISTORY-pad face: the zero-pad pole (gpu_51) removes
never-carrying padding; THIS packs the REAL history. gpu_35 owns the read-only
halo-snapshot axis; THIS is destructive in-place consumption.

### B — `per-row-transcendental-hoist-not-per-element` (B 103→104)
The hoist unit is the TERM's unit: `row_beta` / `row_exp_g` / `row_decay`
(`exp(g_last−g)`) fill per-row register arrays once per row slot (mask
included), never re-derived per element; `row_exp_g` reused across phases;
the mask rides the hoist (masked rows pay zero). 386→315 µs. gpu_105 owns the
token-THRESHOLD axis (when to precompute a pass) and the broadcast form;
THIS owns the GRANULARITY axis (per-row vs per-element) inside one kernel.

### C — `precompute-chain-fused-intermediates-never-materialized` (C 58→59)
ONE precompute kernel replaces cumsum / gram / a_diag_inv / causal_inv —
encode 6→3 dispatches — and the chain's global intermediates (`kk`,
`a_packed`, `a_inv`) are never born (deleted from mod.rs's scratch list);
A operands land in packed threadgroup tiles, T blocks in registers.
101→78 µs. gpu_93 (B234's llama.cpp #29353 pack) owns scan-STATE-in-registers;
THIS owns chain-edge intermediates never born + the dispatch-count face.
gpu_85's fusion-scope law (barrier-cost is the axis, dispatch-count the
proxy) cited in-rule — here they coincide (4 dispatch boundaries + 3 global
round-trips vs threadgroup-local barriers, E2E-proven).
Admission: `groups_per_head > 4 → Ok(None)` (recurrent fallback),
kernel_size==4 constexpr, f16 outer-type gate.

## The riders (count-neutral)

1. **gpu_94 += SMEM-INTERLEAVE LEGALITY face** (cubecl #1853): a smem free
   releases its bytes to a later array only past a CUBE barrier that
   dominates the later array's FIRST use on every path, executing once —
   first use, not declaration, decides; InBranch/InLoop/Unsynced refuse;
   sizing "the larger of the two, not their sum" (the byte battery:
   Synced 1024 vs Never 2048). The compiler-side twin of the host rule's
   hand-written union; conservative enforcement (enclosing-block +
   walk-order index, never a full CFG dominance run) disclosed.
2. **gpu_09 += METAL SIMDGROUP LANE-LAYOUT FORMULA face** (cubecl #1858):
   lane l holds row `(l/16)*4 + (l/2)%4`, cols `((l/8)%2)*4 + (l%2)*2` and
   the next — ONE cell geometry for every operand, implementation-recovered
   (Apple does not document it; MLX's BaseMMAFrag relies on it) — plus the
   PER-OPERAND LAYOUT ORACLE: "a layout wrong in every operand alike can map
   the identity onto itself and pass all three" — identity×numbered /
   numbered×identity / zeros×numbered decompose the product per operand;
   the residual one-permutation-relabel blind spot is the B233
   reshape-relabel genus, neutralized here by position_of_nth symmetry.
   M3-side lane intel (MSL-only; WGSL/SPIR-V keep defaults).
3. **gpu_101 += SWA-CHECKPOINT SHARED-PREFIX VALIDITY face** (llama.cpp
   #30666): restore eligibility is TWO-CONJUNCT — position recency AND
   `n_tokens <= n_prefix` (the checkpoint covers only tokens the next
   request shares); the coverage measure is snapshotted BEFORE any restore
   mutates n_past (the restore itself mutates it — a post-restore count is
   self-referential, the commit's "fails with > n_past, but works with
   > n_prefix" bullet); the paired tests pin rejection of the wrong restore
   AND survival of the right one. The LIFE predicate beside the host's
   DEATH protocol (push at invalidation, borrower-gated reclamation).

## Drift findings vs the staged descriptions (the sweep → landing delta)

1. Rider 2's host: staged said "entries_gpu_38"; the defining
   `mma-fragment-layout-m16n8k16` rule_id lives in entries_gpu_09 (gpu_38
   hosts c-fragment-to-a-operand-register-repack, which only distinct-froms
   it). Adjudicated to gpu_09 per the lane-layout axis test.
2. Rider 3's conjunct sharpened: staged "covers at least the shared prefix";
   the diff's semantics is coverage-ONLY-shared (`n_tokens <= n_prefix`),
   and the pre-restore snapshot is load-bearing (restore mutates n_past).
3. Rider 1: the enforcement is conservative (not a full dominance
   computation) — disclosed in-rule.

## League lane note

The uzu pack targets Qwen3.8-27B-M chunked GDN prefill (our league model
family) on M5 Max: e2e prefill −2.6% @30k / −3.5% @102k, stage numbers
(conv 460→47, scan 386→315, precompute 101→78 µs) — ALL SOURCE-REPORTED on
different silicon, never our bench claim (NotBenchable in-rule; the
instruments are the bit-pin fixtures). Detection-only: no scoring lane
touched, no riir-* code edited.

## Gates (all run this session)

- batch259 needle gate (3 own + 3 rider faces) FIRST-RUN GREEN after two
  needle-text corrections (the gate's needles were keyed to guessed phrases;
  corrected to verified strings — the Lesson: key needles AFTER reading the
  landed text).
- kernel_opt lib 3175/0 (1 ignored, the documented freeze-regen e2e).
- default lib 2702/0.
- clippy -D ×3 postures clean (default / no-default / kernel_opt); the
  dependency `hard linking … incremental cache` warnings are the known
  Windows/MSYS infra noise, body-verified non-code.
- retrieval floors HELD 21/21 no re-pin (366 s, isolated /tmp target,
  removed after).
- freeze / score-bench NOT owed (kernel_opt-only corpus, the P30/B250
  precedent). Binary rebuild NOT owed (kernel_opt not in the release set).

## Count pins moved

corpus 917→920; A 185→186, B 103→104, C 58→59; sites: tests.rs (11
multi-line + 6 inline + 2 total + 3 category tuples + 3 messages + 17
message prefixes), router.rs expert total, mod.rs KERNEL_CATEGORIES slices
(A/B/C) + category comments, AGENTS.md ×2, .docs/11_domains/kernel_opt.md.
Zero leftover "917" in tests.rs (audit run).

## En-route notes

- The CRLF hazard did NOT fire this time: all written files verified 0 CR
  bytes pre-commit (the wire scripts were byte-level; autocrlf=false on this
  box confirmed — the prior session's whole-file CRLF churn incident
  (claim commit, amended pre-push) is the standing example).
- Slice A's header doc comment tripped `doc list item without indentation`
  + `doc_markdown` (a line starting `+`, then bare `a_packed`/`a_inv`) —
  manual two-line repair (backticks + rewording); recorded as the
  divergence-class data point (2 mechanicals healed manually for cycle
  economy; the healer remains the default for batches).
- Concurrent-lane protocol held: Batch 258 (the sibling's, plan 262) landed
  mid-claim; my claim commit rebased over it cleanly (their rows prepend at
  the file top, mine below the section header — both stacks read coherent);
  zero write-set overlap (sec vs kernel_opt).
