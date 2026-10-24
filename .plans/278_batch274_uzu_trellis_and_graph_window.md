# Plan 278 — Batch 274: uzu trellis trio + #996 GDN-tree-verify fusion + the llama.cpp graph-window + splash 122B residue

**Status:** CLAIMED — shikuwa-4090-idle, 2026-10-24, expires 2026-10-26

**Claim:** Batch 274 / plan 278 claimed by shikuwa/4090 idle loop BEFORE authoring; push is the
arbiter; other lanes re-fetch before staging and yield to this row. Sources already local
(`vendor/uzu` fetched to origin/main `14b770894`; `vendor/splash` fetched; `vendor/prismml-llama.cpp`
fetched — the fork carries both prism post-tag commits AND serves as the upstream-history host for
`280141e` after a targeted fetch).

## Scope (the 2026-10-24 watch-pass pool + this session's re-poll)

| # | Source | Commit | Candidate | Home guess |
|---|---|---|---|---|
| 1 | trymirai/uzu | `2229bf8` #998 | trellis GEMM optimizations | kernel_opt OWN (trellis family seed — zero owners) |
| 2 | trymirai/uzu | `034294c` #1002 | function-constant trellis transform dim | kernel_opt OWN or rider on #1 |
| 3 | trymirai/uzu | `333c5f2` #1004 | one reversed little-endian tape decodes all trellis formats | kernel_opt OWN (trellis decode) |
| 4 | trymirai/uzu | `43e9fe3` #996 | fuse-gdn-tree-verify (5 kernels → 1 + tri_inv) | RIDER on the verify-row family (tri-inv owned by B251 `in-register-unit-triangular-inverse-warp-substitution`; the FUSION is the new face) |
| 5 | PrismML fork (prism branch) | `b8639015` #378 | graph cache keyed by first-node shape + address | RIDER on `identity-keyed-graph-retention-stable-arena` (key-composition face) |
| 6 | PrismML fork (prism branch) | `66ae2fd8` #379 | batch runs of independent same-shape F32 copies into one launch | RIDER/OWN on the dispatch-collapse family (`merge-dispatches-into-single-compute-pass`) |
| 7 | ggml-org/llama.cpp upstream | `280141e` #30804 + `9ff0d7e` | CUDA: chunk graphs to amortize graph launch | RIDER on the capture-region family (the WHY-to-cut axis vs TensorFold's collective-cut placement) — Plan-633 corroboration lane intel |
| 8 | incoai/splash | `7b21db3` | serve pruned MoE GGUF by the routed experts its header declares | kernel_opt/rust_perf face vs `moe-unverified-rows-route-zero-experts-dense-shape` (B268) — likely rider |
| 9 | incoai/splash | `1bf2f1a` | dense attention tile's parts derived from its group in the kernels | kernel_opt candidate — tile-family dedup at read |
| 10 | incoai/splash | `0341410`/`2c645f3` | split-GGUF metadata-first-part layout | rust_perf/io candidate — dedup at read |
| 11 | incoai/splash release/12-20 wave | `28d0e98`, `772ba56`, `e88a0d0`, `84b0e16`, `720f5fd` | memory-budget refusal / start-failure reason / penalized-greedy check / pruned experts | rust_perf serving candidates — graded thin at T0, honest drops allowed |

- [x] T0 — re-verify every pin at clone/checkout; confirm sources' licenses (uzu MIT; llama.cpp MIT; splash Apache-2.0)
- [x] T1 — dedup grep (DONE pre-claim: trellis ZERO owners; graph-key face → identity-keyed arena rider; f32-copy → dispatch-collapse family; chunk-graphs → capture-region family; moe-prune → B268 neighbor)
- [x] T2 — spawn 2 parallel authoring subagents over disjoint slices (A: uzu trellis trio + #996; B: prism #378/#379 + upstream #30804 + splash residue); re-verify every quoted line at the pin
- [x] T3 — corpus entries + fixtures (standalone compile per field) + count pins via ONE shape-asserted script pass
- [x] T4 — batch274 needle gate + syn-parse gate
- [x] T5 — kernel_opt lib + default lib + clippy −D ×3 postures
- [x] T6 — retrieval floors re-measure (own rules expected → re-measure OWED)
- [x] T7 — freeze/score-bench NOT owed (kernel_opt-only corpus, P30) — verify the delta stays kernel_opt/rust_perf-rider-only; if a clippy-lints rule sneaks in, regen owed
- [x] T8 — binary rebuild NOT owed (kernel_opt outside release set) — verify; if release-set surface moves, rebuild + install
- [x] T9 — Research 312 + AGENTS.md corpus bump + .docs/11_domains/ + snapshot landing row, one commit
- [x] T10 — commit `feat: batch 274: ...` + push; `.raw/` scratch removed at close
- [x] T11 — scored-tag check recorded: `prism-b10787-73bfe6c` STILL tip at claim time (ls-remote this session) — bout re-arm NOT fired; the post-tag fork commits #378/#379 are MINING material only, zero league-matrix movement claimed

## Gates owed
needle+parse (new rules), kernel_opt lib, default lib, clippy −D ×3 postures, retrieval floors
re-measure. NOT owed unless scope drifts: freeze regen, score-bench, binary rebuild.

## Non-goals
No league-matrix or bout work (no scored tag fired). No GPU. Zero-CUDA-driver work; this is
mining-only. The vendor/prismml-llama.cpp/build dir stays deleted (disk-clean, owner-approved).

## Landing note (2026-10-25)

All tasks complete. Landing commit: (this commit). Binary rebuilt + installed at the landing
HEAD after commit (build stamp == landing). Slice A's two subagent timeouts + the coordinator
authoring pass, the four en-route lessons, and the lane intel are recorded in Research 312.
