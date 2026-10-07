# Research 244 — the 10-08 envelope re-poll: pool2d warp-window crossover (Batch 209)

**Status:** RECORD — one lead converted as a RIDER on
`row-dot-split-inner-axis-block-reduce` (Batch 209, Plan 209); zero corpus-count
delta (776 held); the R134 re-test found the October nnethercote post NOT yet up
(latest still Sep-30, already consumed by Research 238 / Batch 202).

## The envelope (10-08, ~10 h after the B208 grading poll)

| Source | Last pin | Now | Delta |
|---|---|---|---|
| `ggml-org/llama.cpp` | `448147d42a` (10-07T14:30Z) | `42c787e8c1` (10-07T17:53Z) | +2: **#29425 pool2d (THE LEAD)** + `18b5f8b1` server parse (off-lane) |
| `tracel-ai/cubecl` | `2f0d8e8df5` (#1771 readme) | unchanged | 0 |
| `bevyengine/bevy` | `5cd69c1357` | unchanged | 0 |
| `gfx-rs/wgpu` | `5c966018` (+1 dependabot, no lead) | feed 404 this pass (master + main) | transient; watch-only lane |

nnethercote R134 re-test (owed ~10-08): blog latest is still **Sep 30, 2026**
(the September post — consumed by Research 238 / Batch 202, the min-rank
worklist rule). The October post does not exist yet. Next re-test: when the
feed shows a post dated ≥ 2026-10-xx, or ~11-01.

## The lead — llama.cpp `42c787e8c1` #29425 (MIT, merged 2026-10-07T17:53:43Z)

"cuda: update uncoalesced memory reads in pool2d". Full diff read at the pin
via the ephemeral `.raw/b209-llamacpp` clone (blob-filtered; removed at close).

**The mechanism (the WORK-SIZE-GATED face of row-dot-split):** the old CUDA
pool2d kernel maps one THREAD per output element, each walking its kh x kw
window serially. With wide windows the per-thread work unit is the window, and
adjacent lanes' same-step loads sit `sw * 4B` apart — the driver shape is the
DSA-indexer block-max: `ggml_pool_2d(.., MAX, blk=128, 1, blk=128, 1, 0, 0)`
over `sc [n_ps, n_head=4, n_tokens]` (MiniMax-M3 indexer block scores) —
sw=128 f32 = 512 B between adjacent lanes' loads: a cache line per lane per
step, zero coalescing.

**The fix:** a host-side gate `if (kh * kw >= CUDA_POOL2D_WARP_KERNEL_MIN_WINDOW)`
(macro = 32 = WARP_SIZE) routes to `pool2d_nchw_kernel_warp`: one WARP per
output element (warp_id → nc/cur_oh/cur_ow), lanes tile the FLATTENED CLIPPED
window — `for (int t = lane; t < win_elems; t += WARP_SIZE)` with
`i = bh + t / win_w; j = bw + t % win_w` from the padding-clipped bounds
(`bh = max(0, start_h)`, `eh = min(ih, start_h + kh)`) — coalesced by
construction; the finish is an op-polymorphic `__shfl_xor_sync` butterfly
(`max(res, other)` for MAX, `res + other` for AVG) at warp radius — no smem,
no barrier; the AVG scale folds PER ELEMENT (`res += cur * scale`); ragged
windows (`win_elems % 32 != 0`) simply skip lanes; lane 0 stores; both
pointers `__restrict__`.

**Tests:** eval adds the non-divisible-tail shapes ("MSA indexer block-max
shape: wide 1-D window along ne0, no padding, non-divisible width", iw
4096/4160 x blk 32/64 — 4160/128 = 32.5 windows); perf adds the MiniMax-M3
driver shapes. No in-source perf numbers → NotBenchable; the decision content
is the second admission axis (per-output WORK SIZE ≥ WARP_SIZE, vs the host
rule's OUTPUT-COUNT-vs-machine axis) + the warp-radius shuffle-only form.

## Dedup verdict (rider, not rule)

- The split-the-reduction-axis fix shape is owned by
  `row-dot-split-inner-axis-block-reduce` (B44) — this is the same law at the
  opposite regime corner (outputs plentiful, per-output work too wide) with a
  work-size crossover gate; the B108 capped-occupancy rider already established
  the multi-face pattern on this rule.
- The butterfly reduce is owned by `warp-shuffle-intrinsics-for-sub-warp-reduction`
  (B22, xor-butterfly + sub-simdgroup-radius riders).
- No `__restrict__` rule exists; the hint is carried in-rule as detail, not
  promoted (mechanism-level hint, no decision content).
- Pool2d-adjacent rules checked: `layout-permute-for-coalescing` (NCHW→NHWC
  layout permute — a different axis), `op-family-type-dispatch` (family trait —
  unrelated).

## Gating record (Batch 209)

- `batch209_pool2d_rider_present` — 13 rider needles verbatim (host-side gate
  quote, macro, window-tiling loop, clipped-index derivation, butterfly
  ternary, per-element scale fold, both test-comment quotes, the 32.5-windows
  arithmetic, shuffle intrinsic, NotBenchable, batch/plan stamp).
- En-route heal: `batch208_concepts_carry_their_load_bearing_contracts` had
  crossed the clippy `too_many_lines` pedantic bar (118/100, pre-existing on
  `dddb9eb9`); its rider-needle section split into
  `batch208_rider_needles_present` (assertions unchanged) — all-features
  clippy clean again.
- kernel_opt+latent_retrieval lib **3082/0** (3 ignored = the documented
  freeze-regen e2es; 1228.9 s, one full run); default lib **2473/0** (the +1 vs
  B208 is the M3 session's payload-v4 test); clippy ×3 postures clean;
  retrieval **21/21 floors HELD** (235.0 s, no decay, no re-pin); freeze/
  score-bench NOT owed (zero clippy-corpus delta, P30).

## Queue after

- nnethercote: next re-test on the October post or ~11-01.
- Walk #14 opens Tue 10-13.
- llama.cpp watch set (nextn-flags `448147d4`, musa `ad215653`, hexagon `c479922a`,
  sycl MMVQ `78651c41`, mixed-GPU FA `005a1e12`, vulkan `4d756bc7`) + the
  Research-239 follow-up pool stay in intake.
- Owner-gated set unchanged (riir-ai 1036, SYSTEM ssh orphans, Plan 199 rescope,
  C: 43.1G free).
