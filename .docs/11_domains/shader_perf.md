# `shader_perf` — the WGSL render-pipeline domain (Plan 134 / Proposal 014)

> Seventh retrieval `HealDomain`. Scope: **`@vertex`/`@fragment` WGSL source
> patterns with bounded fix-spaces** — loop boundedness (the TDR class),
> texture-sample budgets, branch-vs-sample divergence, stage traffic,
> derivative safety. `@compute` entry points stay `kernel_opt` territory
> (the routing split is the entry-point attribute set — mechanical).
> Opt-in (`shader_perf`, default-off); the promotion pull is the upcoming
> shader-toy editor's submit/save gate (the docker T1.6 standing-consumer
> rule) + riir-dapps Proposal 002's Stage-2 mint gate ("naga validation +
> texture/loop caps").

## Why this domain (the measured empty field)

Source-level WGSL perf analysis does not exist to adopt: tint/naga validate
semantics and stop; spirv-opt fixes at the IR where the author never sees
it; frame profilers measure after the fact. The measured workspace surface
(2026-09-16): 190 tracked `.wgsl` in riir-shader (131 fragment · 46 vertex
· 27 compute, 0 file-level mixed), 124 in riir-ai — all `@compute`.

## The scanner (third no-`syn` line scanner after docker and dist)

`src/domains/shader_perf/scanner.rs` — block-aware, zero deps (no `syn`, no
`regex`). Stage attributes open render-fn blocks; loop/branch blocks carry
`has_sample`/`has_exit`/`fetch_count` flags decided at CLOSE time on the
whole block. Comment-stripped + string-masked lines only — a `// @fragment`
doc mention is not a stage.

## Rules at landing (8; the honest ledger)

| rule | family | class |
|---|---|---|
| `unbounded-render-loop-guard` | LoopBoundedness | **bounded** (guard-insertion proposer, `FIXABLE_SHADER`) |
| `loop-carried-sample-to-explicit-lod` | SampleBudget | two-regime — suggest-only v1 (explicit LOD only where reconstructible) |
| `branch-divergent-sample-select` | BranchCoherence | decision rule — suggest-only FOREVER (the DL3059 refusal) |
| `const-fold-loop-bound` | LoopBoundedness | suggest-lean (permutation-cost decision) |
| `fragment-sample-budget-annotation` | SampleBudget | Class B — fires at ≥ 10 fetches (measured port max 9: three-tsl/surface; the first calibration at ≥ 5 fired on 4 clean ports and the negative control demoted it) |
| `vertex-stage-texture-fetch-annotation` | StageTraffic | Class B |
| `derivative-in-nonuniform-flow` | DerivativeSafety | Class B (naga's uniformity analysis owns the hard errors) |
| `invariant-recompute-to-varying` | StageTraffic | corpus-only — not line-expressible v1 (needs vertex-output dataflow) |

**Close-condition ledger (honest):** 1 bounded rule landed vs the ≥ 6 bar —
recorded as a **sec-style marginal landing**: opt-in, doc-noted, the rules
NOT folded into kernel_opt (they are render-stage vocabulary; folding them
across the routing split would be the wrong consolidation). The domain
serves `--suggest` citations + the loop-guard fix; promotion waits on the
shader-toy editor consumer.

**Dedup ledger (T1.1):** `uniform-struct-tail-pad` → citation of
kernel_opt's `pad-uniform-pod` family (not re-shipped);
`render-adjacent-compute-workgroup-size` → kernel_opt's
`row-dot-split-inner-axis-block-reduce` already fires on
`@workgroup_size(1…)` (not re-shipped).

## The oracle ladder

| Tier | Check | Gate |
|---|---|---|
| 0 | text round-trip invariants — brace balance + `@stage` entry-point multiset + `@group/@binding` multiset byte-identical | always (plain `shader_perf`) |
| 1 | `naga` parse+validate in-process (`shader_verify` → `dep:naga`; already in the lock via the riir-gpu arm — zero new packages) | `--fix --verify`-shaped checks; degrades to Tier 0 with ONE line (the Issue 018 oracle-split) |
| 2 | capture-fidelity (byte-identical pre/post at pinned frame+seed) | the CONSUMER repo's harness (riir-shader / the editor gate) — never a riir-refine dep |

The naga-vs-Tint divergence (`ptr<workgroup>` params: Tint-legal,
naga-rejected — riir-shader Plan 001) is recorded in the domain: naga
rejections on browser-legal shapes degrade to Tier 0 + the browser smoke is
final truth.

## The frame-time budget tier (T2.1, 2026-09-17)

The measurement VERDICT half of the domain: **the rules are ours, the
sidecars are riir-shader's** (issue 018 there — commits `34ff053` +
`a92000a`): every automated capture (single / sweep / burst) writes a
`<capture>.frames.json` sidecar beside the PNG — schema
`riir_shader_frame_time_v1` (effect, capture path, warmup config,
`frame_budget_ms` echo, n/mean/p50/p90/min/max/fps + the per-frame sample
ring, bounded 8192 frames, reset on effect switch).

`cargo refine --frame-time-report [PATHS…]` consumes them:

- **Budget report**: per-effect p50/p90/mean vs the carried budget
  (`--frame-budget <MS>` overrides), `OK`/`OVER` per row, and the heal
  hint — the over-budget effects are exactly where `--domain shader_perf`
  (the sample-budget / loop-boundedness corpus) should run next. The tier
  PRIORITIZES the source-scan rules; it does not replace them.
- **A/B diff** (`--vs <PATH>`): pairs pre/post by effect slug, verdicts at
  `--frame-delta-pct` (default ±5% on p50 → IMPROVED/REGRESSED/FLAT). Two
  evidence guards are load-bearing: **warmup parity** (a pair captured
  under different warmup configs is not evidence) and **pairing
  uniqueness** (several sidecars per effect on a side — the burst class —
  refuses; the tool never invents pairings).

The module (`src/frame_time_report.rs`, domain handle
`domains::shader_perf::frame_time` under the feature) is deliberately
UNGATED — the corpus_coverage/horizon_report law: pure over its inputs,
no domain machinery, lib-tested at default features. Refusal law: exit 1
on an empty scan, any unparsable sidecar, or (A/B) any mismatch with zero
usable pairs — an incomplete measurement REDs, never prints a flattering
partial as a pass. Noise discipline is documented, not mechanized: one run
is one run — confirm IMPROVED/REGRESSED with a second sweep pair before
acting on it.

Gates: 11 lib fixtures + 8 process-boundary CLI contracts
(`tests/frame_time_report_cli.rs`); live validation = the 29-effect
riir-shader sweep (29/29 parse, 9.3 ms wall) + self-A/B FLAT exit 0 +
warmup-mismatch / empty-dir exit 1 (`.benchmarks/094`). The walker matches
the FULL `.frames.json` filename suffix, not `Path::extension()` (which
returns `json` — the empty-sweep bug the CLI test caught live before any
consumer ran).

## Gates

- `tests/shader_perf_gates.rs` — fixture firing (buggy→fixed pairs),
  **the negative control: ZERO findings over the 190-file vgpu port corpus**
  (SKIP-loud when the sibling is absent; ≥ 100 render-stage population
  floor), determinism, the refusal table, proposer idempotence + Tier-0
  cleanliness, G2 latency (< 1 ms/file, release-gated).
- The router floors: `shader_perf` is opt-in — all default-state floors
  unchanged; shader-on states inherit their twins' floors until a standing
  lane carries the feature (re-measure + re-pin then, per the Issue 020
  contract).
