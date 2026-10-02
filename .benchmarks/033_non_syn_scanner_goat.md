# Bench 033 — Issue 025 T5: non-syn scanner GOAT (real anti-patterns + git-history clean control + latency)

**Date:** 2026-08-16
**Scope:** `src/domains/kernel_opt/wgsl_scan.rs` (T2 patterns) + `cube_extract.rs` (T3) + the `cargo heal --suggest` wiring (T4) — the non-syn surface scanner (`.wgsl` files + `#[cube]` bodies).
**Gate:** `Issue 025` T5 — "≥5 real anti-patterns drawn from the Batch 14–51 rule set, zero false positives on the clean-file control, <1 ms/file".

## G1 — ≥5 real anti-patterns on the live tree

Live scan (`examples/wgsl_scan_smoke`, default pattern set) over
`riir-ai/crates/riir-gpu/src/kernels` (124 `.wgsl` files at run time — 123 at
T2 + 1 sibling WIP addition):

| rule (corpus id) | findings | real carriers |
|---|---:|---|
| `row-dot-split-inner-axis-block-reduce` (B44) | 14 | `m2p_attention` ×3, `attention_score` family ×6, `attention_concat`, `thinking_lora_route`, `moa_swiglu_backward`, `layer_fuse_backward` |
| `declared-max-array-taxed-per-thread-under-runtime-bound` (B46) | 15 | `m2p` q/scores ×4 (Issue 701 H-B's documented deferred caps), `flashprefill` mean_q/my_q/my_output, `attention_score*` scores, `attention_concat` 512 |
| `share-cube-size-const-between-kernel-and-launcher` (B44) | 1 | `mux_kl_loss` `let n = 64u;` |
| **total** | **30** | 16 firing files |

The load-bearing realness evidence: the `m2p_attention.wgsl` FILE HEADER
(post-fix version, committed as
`tests/fixtures/m2p_attention_postfix_701.wgsl`) itself documents the caps as
Issue 701 H-B hazards with the fix deferred ("Follow-up (not built here):
pipeline override constants per bound class would lift both caps"). The
scanner's findings on that file cite exactly those documented hazards.
**30 ≥ 5 → PASS.**

## G-clean — zero false positives on the git-history clean control

The control pair (extracted from riir-ai git history, committed as fixtures):

| fixture | source | role |
|---|---|---|
| `m2p_attention_prefix_701.wgsl` | `8bb716954^` | pre-fix: fused out-projection `attn_val * rowsum(wo)` (Issue 701 H-C wrong-math) |
| `m2p_attention_postfix_701.wgsl` | `8bb716954` | post-fix: two-phase out-projection (both operands vary) |

Results (pinned by `tests/wgsl_t5_control.rs`):

1. **G-pair (the mining garbage-in guard):** the experimental
   `multi-head-out-proj-consumes-head-concat` pattern FIRES on the pre-fix
   side (L95: `proj_val = proj_val + attn_val * out_weight[d_out * h + kk];` —
   `attn_val` invariant in `kk`) and is SILENT on the post-fix side (phase-2
   kernel: `sum = sum + attn_scratch[pos * h + k] * out_weight[d * h + k];` —
   both operands indexed by the summation var). The Issue 701 buggy→fixed
   pair is miner-detectable; the fixed side is a genuine clean control for
   that rule. PASS.
2. **Post-fix default-set findings (7) are ALL adjudicated REAL** — the
   3× `@workgroup_size(1, 1, 1)` are the documented per-(pos, head) phase-1
   design and the 2×128 + 2×256 private arrays are the documented H-B
   deferred caps. A finding citing a real, still-present anti-pattern is not
   a false positive. The count is pinned by test — any new finding on the
   fixture must be adjudicated before the test changes. PASS (0 FPs).
3. **Default set on the pair:** both sides carry the caps (they predate the
   701 fix) — only the experimental rule discriminates. This is why T6 mining
   pairs RULES WITH DIFFS, not files with history (pinned by test).

Honest caveat (from the prior session, confirmed here): the CURRENT live
`m2p_attention.wgsl` is NOT a fully-clean control for the default set — the
H-B caps remain unfixed in-tree. The default set's "fixed silent" behavior is
pinned by the unit tests' fixed forms (`let cube_size = CUBE_SIZE;`,
`var<workgroup>` shared arrays, multi-dim workgroups) rather than a
git-history default-rule pair. No git-history pair yet exists where a
DEFAULT-pattern shape was removed — T6 mining will surface them if they
exist.

## G2 — latency < 1 ms/file

`g2_scan_latency_under_1ms_per_file` (release-gated, repo convention):
**41.19 µs/file** on the largest committed fixture (201 lines), 2000
iterations after 50 warmup, M3 Max (Apple Silicon), release profile.
**24× under the 1 ms budget → PASS.** (Debug builds are ignored per the
`#[cfg_attr(debug_assertions, ignore)]` convention — regex debug timing is
meaningless for a budget gate.)

Cube-body extraction cost (T3, measured in the live smoke): 253 `.rs` files
tokenized + 142 items extracted + scanned in the same sub-second run as the
`.wgsl` pass — lexing is not the bottleneck at this tree size.

## Gates summary

| gate | bar | measured | verdict |
|---|---|---|---|
| G1 real anti-patterns | ≥5 | 30 (live tree) | ✅ PASS |
| G-clean control FPs | 0 | 0 (7/7 adjudicated real; pair rule discriminates exactly) | ✅ PASS |
| G2 latency | <1 ms/file | 41.19 µs/file | ✅ PASS |
| G3 no-regression | lib tests green | 419 default / 598 kernel_opt / 724 all-features + 4 t5_control tests; clippy 0 this crate | ✅ PASS |

## What this licenses

- **T4's suggest wiring is validated end-to-end** (30 findings through the
  real `cargo heal --suggest` path in the T4 e2e run).
- **T6 mining has its substrate + its first validated pair** (Issue 701 H-C
  via the experimental pattern). The miner's pairing rule (rule ↔ diff, not
  file ↔ history) is pinned by test.

## Reproduce

```bash
cargo test --features kernel_opt --test wgsl_t5_control
cargo test --release --features kernel_opt --lib g2_scan_latency -- --nocapture
cargo run --features kernel_opt --example wgsl_scan_smoke           # .wgsl mode
cargo run --features kernel_opt --example wgsl_scan_smoke -- --rs   # cube-body mode
cargo run --features latent_retrieval,kernel_opt --bin cargo-heal -- --suggest --domain kernel ../riir-ai/crates/riir-gpu/src/kernels
```
