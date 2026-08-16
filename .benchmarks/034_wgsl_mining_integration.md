# Bench 034 — Issue 025 T6: WGSL/cube mining integration (buggy→fixed pairs from riir-gpu git history)

**Date:** 2026-08-16
**Scope:** `examples/wgsl_mine.rs` (the repeatable mining harness) + two new
patterns (`scan_single_slot_staging` P6 default; `scan_serial_row_dot` P7
experimental) + `tests/wgsl_mine_pairs.rs` (the pair fixtures) + the full
mining run over riir-gpu history with adjudication.
**Gate:** [Issue 025](../.issues/025_non_syn_surface_scanner.md) T6 — "mine
WGSL buggy→fixed pairs from riir-gpu fix history toward Phase 5's reopen
trigger (≥5 proposer-grade pairs). Every batch re-runs the Issue 017
retrieval regression gate."

## The harness (`examples/wgsl_mine.rs`)

Walks a repo's git history for non-syn-surface changes, scans each commit's
pre/post versions with the FULL pattern set (default + experimental — in
mining the DIFF adjudicates), and reports:

1. **PAIR CANDIDATES** — pre-finding with NO post-finding of the same rule
   within ±10 lines (finding-level pairing: file-level rule silence is too
   strict — a multi-kernel file keeps firing the rule on its OTHER unchanged
   kernels), NOT a move (identical excerpt does not re-appear in post), AND
   diff-associated (the fired line removed directly, or inside a changed
   hunk's old-side span — the 698 stride fix leaves the accumulation as
   unchanged context inside its hunk).
2. **MOVED** (auto-refuted) — the fired shape re-appears elsewhere in post
   (same rule, identical excerpt): relocated, not fixed.
3. **INTRODUCED** (info) — post-finding with no nearby pre: new
   anti-patterns landing, not pairs.

Surfaces: `.wgsl` files (default mode) + `#[cube]` bodies (`--rs`, cube-file
set from one `git grep -l '#[cube' HEAD` + `--extra-path` for the pre-split
`weaver_gpu.rs` rename seam). The `-G'#[cube'` log filter was REFUTED by the
run itself: it matches commits touching cube ATTR lines only — the 698 fix
(changing lines INSIDE a cube body) was invisible to it.

## The two new patterns (mined tells)

| pattern | set | fronts | tell | source pair |
|---|---|---|---|---|
| `scan_single_slot_staging` (P6) | **default** | `smem-staging-gated-by-cross-thread-reuse` (B35) | a `var<workgroup>` array whose writes are ALL bare pass-through loads (`A[i] = src[…]`, no top-level arithmetic) and whose every read uses ONE bare-identifier index that never self-increments — no thread reads two distinct slots, so the staging buys nothing a direct global read doesn't (same-address reads broadcast) while costing barriers | Issue 686 H7 (`2befc873e` — `covariance.wgsl`) |
| `scan_serial_row_dot` (P7) | **experimental** (mining-only) | `row-dot-split-inner-axis-block-reduce` (B44, body form) | a dual-index dot accumulation (`acc = acc + x[base + j] * y[base2 + j]`, common loop var) with serial-stride evidence (`j += 1u32` / `k = k + 1u`) within ±8 lines | Issue 698 A-H1 (`959b6a0a9` — `dot_per_row_f32` cube body) |

P6 upgrades B35 from "not regex-expressible in v1" (the T2 verdict) — the
686 fix pair SUPPLIED the textual tell. P6's live-tree calibration: exactly
2 findings (`gram.wgsl` s_row + s_col — the dead-file carrier carrying the
covariance-pre shape verbatim), ZERO false positives across the other 32
`var<workgroup>` files (the computed-write `role_transport.wgsl`, the
tree-reduction `rownorm_momentum`/`argmax`, the compound-index
`transpose`/matmul/gemv tiles, and the multi-phase `domino` are all
correctly silent — each guarded).

P7 stays experimental: 11 live serial-dots-under-parallel-grids instances
(serial-K GEMV shapes — `spectralquant_maxsim`, `muon_fused`,
`eigenbasis_rotation`, …) are textually unadjudicable (whether the output
count is too small to fill the machine is a workload fact). In mining the
diff adjudicates.

## The mining run (riir-ai, 2026-08-16)

| surface | (commit,file) pairs | candidates | auto-refuted moves | introduced (info) | skipped |
|---|---:|---:|---:|---:|---:|
| `.wgsl` (28 commits) | 28 | 5 | 4 | 16 | 0 |
| cube bodies (`--rs` + weaver seam) | 673 | 1 | 7 | 11 | 49 |

### Candidate adjudication (all 6 candidates adjudicated)

| commit | file | rule | evidence | verdict |
|---|---|---|---|---|
| `8bb716954` | m2p_attention.wgsl | multi-head-out-proj (B47) | L95+L169 Removed | ✅ **GENUINE** — Issue 701 H-C (pair 1, T5 fixture) |
| `2befc873e` | covariance.wgsl | smem-staging (B35) | L32+L33 Removed | ✅ **GENUINE** — Issue 686 H7 (pair 2, new fixture) |
| `959b6a0a9` | weaver_gpu/mod.rs | row-dot-split body (B44) | L1205 Hunk | ✅ **GENUINE** — Issue 698 A-H1 (pair 3, new fixture) |
| `b75dd53c8` | layer_fuse_backward.wgsl | multi-head-out-proj (B47) | L65 Removed | ❌ **REFUTED** — the pre-fix line `grad_alpha = grad_alpha + grad_input_d * target_hiddens[target_base + dim];` has `grad_input_d` computed INSIDE the loop from `dim`-indexed loads (it VARIES — the documented B47 imprecision class, a legit look-alike), and the removal was a feature rewrite (Plan 195 T4 softmax fusion weights — the same commit also INTRODUCED the known alpha-lerp FP in `layer_fuse.wgsl` L61) |
| (auto) | m2p L28/L41, lora_delta L52, layer_fuse_backward L55, weaver×5, gemma2 L141, qwen×4 | row-dot / declared-max | Moved | ❌ **MOVED** — identical excerpt re-appears in post (the shape relocated, not fixed) |

### Pair inventory after T6 (the proposer-grade evidence store)

| # | rule | fix commit | fixture(s) | pattern |
|---|---|---|---|---|
| 1 | `multi-head-out-proj-consumes-head-concat` (B47) | `8bb716954` (Issue 701 H-C) | `m2p_attention_{prefix,postfix}_701.wgsl` | experimental (T5) |
| 2 | `smem-staging-gated-by-cross-thread-reuse` (B35) | `2befc873e` (Issue 686 H7) | `covariance_{prefix,postfix}_686.wgsl` | **DEFAULT** (new) |
| 3 | `row-dot-split-inner-axis-block-reduce` (B44 body) | `959b6a0a9` (Issue 698 A-H1) | `dot_per_row_{prefix,postfix}_698.rs` | experimental (new) |

**3 of the ≥5 reopen-trigger pairs.** The trigger (Plan 005 Phase 5: ≥5
rules with literal buggy→fixed pairs) is NOT yet met — honestly recorded.
The corpus itself is unchanged (patterns front existing rules; no new
entries → no crowding); the Issue 017 retrieval regression gate re-run
PASSes (floors held, corpus count still 348).

## Live-tree state after the P6 default addition

Default-set live scan: **32 findings** (was 30 at Bench 033) — the +2 are
`gram.wgsl` s_row/s_col (adjudicated REAL: the exact 686-pre shape, in a
dead file — 0 `.rs` refs — but present in the scan dir). Suggest-path
findings remain citations only. The t5_control fixture counts are unchanged
(the m2p fixtures carry no `var<workgroup>`).

## Gates

| gate | result |
|---|---|
| lib tests | 419 default / 602 `kernel_heal` / 727 all-features — all pass |
| integration | `wgsl_t5_control` 4/4 + `wgsl_mine_pairs` 6/6 |
| clippy | 0 warnings in this crate (`--features kernel_heal --all-targets`); the `post_prev` katgpt-core warning is the sibling's |
| Issue 017 retrieval gate | PASS (corpus unchanged; floors held) |
| pattern-set citation contract | `every_pattern_cites_a_corpus_rule` — the 4 default + 2 experimental ids all exist in the 348-rule corpus |

## What this licenses

- **The mining loop is integrated and repeatable** — one command per surface;
  future fix commits touching these surfaces enter the next run's candidates
  automatically. The adjudication protocol is mechanized for the two easy
  refutation classes (moves auto-refuted; ±10-line persistence) with the
  semantic step (diff reading) remaining human.
- **3 proposer-grade pairs** exist as committed fixtures with
  discriminating patterns — the Phase 5 reopen evidence store is open.
- **P6 (B35) joined the DEFAULT set** — a live-suggest-visible pattern with
  zero observed FPs, mined directly from a validated fix pair.

## Reproduce

```bash
cargo run --features kernel_heal --example wgsl_mine -- ../riir-ai
cargo run --features kernel_heal --example wgsl_mine -- ../riir-ai --rs --extra-path crates/riir-gpu/src/weaver_gpu.rs
cargo test --features kernel_heal --test wgsl_mine_pairs
cargo test --features kernel_heal --test wgsl_t5_control
cargo test --features latent_retrieval,kernel_heal --lib retrieval_precision_regression_gate -- --nocapture
cargo run --features kernel_heal --example wgsl_scan_smoke -- ../riir-ai/crates/riir-gpu/src/kernels
```
