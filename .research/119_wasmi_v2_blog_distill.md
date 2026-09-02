# Wasmi v2.0 Blog — distill verdict + the workspace upgrade trigger

**Status:** DISTILLED — pending owner decision (mining batch queued as Batch 99, Plan 067; the linked workspace action — the wasmi 1.0 → 2.0 upgrade — is executing under riir-ai Plan 563)

- **Source:** https://wasmi-labs.github.io/blog/posts/wasmi-v2.0/ — "Wasmi 2.0 - Engineering of the Fastest Wasm Interpreters", Robin Freyler (wasmi author), published 2026-09-01.
- **Inspection date:** 2026-09-02.
- **Source type:** engineering blog (not a repo) — fixtures for any corpus entry must be ORIGINAL minimal reproductions per the distill constraints; the blog's assembly listings are evidence, never pasted.
- **Provenance chain:** first-party (the interpreter's author), measured (wasmi-benchmarks suite, 3 hardware setups, PR-by-PR release notes), cross-verified (the DestProp finding was independently reproduced on Stitch before being found in wasmi; upstream rustc context cited). Sponsorship note: SDF funding ends 2026-10 — a maintenance watch item for the DEPENDENCY, not the patterns.

## Why this source matters to the workspace (dual verdict)

1. **Dependency action (primary):** wasmi 2.0 (2026-09-01) is a ground-up interpreter rewrite: ~2.2× geomean execution speedup, `auto-dispatch` tail-call dispatch default, lock-free append-only `CodeMap`, new `InstanceEntity` layout, fixed 64-bit cells (the `simd` feature no longer penalizes non-simd code), stable fuel metering, `deterministic` profile support, and a ~50% single-fix win from working around a **Rust 1.92 `DestinationPropagation` deopt**. We consume wasmi in 13 pins across 7 repos (validators, cognition runtime, neuron-vessel Cold path, dialog/quest hosts, bomber pruner). **Verdict: upgrade — riir-ai Plan 563** (lockstep pins + fuel recalibration + bench evidence). The "one wasmi in the dep tree" rule (pinned in riir-neuron-db / riir-game-sdk / riir-chain comments) makes this a lockstep campaign, not per-repo bumps.
2. **Mining action (this note):** the blog's host-side Rust engineering yields 1–2 `rust_perf` rule candidates (below). This is NOT a `kernel_opt` source (host-side interpreter engineering, no GPU) and does NOT justify opening the proposed `wasm` heal domain (it teaches interpreter internals, not wasm-module healing patterns).

## Migration-relevant facts captured for Plan 563 (verified against the v2.0.0 changelog + `docs/migration-v1-to-v2.md`)

- **Breaking:** `Ref<T>` → `Nullable<T>` rename; new non-generic `Ref`/`RefType`; `Table` operates on `Ref` not `Val`; `ResourceLimiter` trait signature change. **Our grep: ZERO usage of `Ref`/`Table`/`Val`/`Nullable`/`ResourceLimiter`/`StoreLimits` across all workspace `.rs`** — our whole surface is `Config/Engine/Linker/Module/Store/TypedFunc/Memory` + `set_fuel`/`consume_fuel`. The upgrade surface is therefore version pins + fuel recalibration only.
- **Fuel model change (the real work):** wasmi 2.0 meters per **input Wasm operator** (Wasmtime-style), replacing 1.0's per-internal-IR-op metering. Absolute fuel numbers change → hard-coded budgets must be re-validated. Our tight sites: katgpt-rs bomber `FUEL_PER_CALL = 50_000` (sized vs "~40K WASM ops worst case" — model-compatible arithmetic, verify empirically), riir-validator-check `set_fuel(100_000)` smoke budget, riir-neuron-db `DEFAULT_PROJECT_FUEL = 1_000_000` (doc comment cites 1.0's "~1 fuel per op" — the ~384-fuel dot-product estimate stays approximately right under input-op metering; reword to be model-neutral), riir-wasm `FUEL_PER_CALL = 1_000_000` / `COG_FUEL_PER_CALL = 10 × FUEL_PER_CALL` (relation-based constant self-adjusts; huge headroom). `fuel_wasmi.rs` documents the wasmi/wasmtime fuel API parity table — re-verify `Error::as_trap_code()` still exists in 2.0.
- **Feature defaults:** `default = ["stable","std","wat","validate","memory64","auto-dispatch"]` — we pass `features = ["simd"]` with defaults on everywhere → no feature action required. New knobs worth a follow-up: `deterministic` (NaN canonicalization — the wasmi analog of the wasmtime cognition backend's `cranelift_nan_canonicalization`; adoption is a G3-determinism strengthening candidate, needs its own gate), `Config` disallow-`start` (validator hardening candidate), `validate`-off size trim (DECLINED for us — defense-in-depth over 200–300 kB).
- **Perf evidence to re-measure locally:** `wasm_runtime_cmp` (riir-wasm) and katgpt-moka-wasm `wasmi_bench` before/after = our G2 gain evidence; cognition tick overhead (the production `cognition_wasmi` path measured 223 ns/tick at 1.0) should improve.

## Dedup gate (mandatory — run before claiming candidates)

Grep of `riir-clippy/src/domains/rust_perf/` under both vocabularies:
- paper vocab: `destination propagation`, `branch site`, `csel`, `dispatch`, `predictor` → hits only `branchless-conditional` (data-dependent branchfulness vs vectorization), `dyn-to-enum` (devirtualization), `keyword-map-to-match` (dispatch table shape). **None covers merged-static-branch-site predictor history mixing. Novel.**
- second candidate vocab: `calling convention`, `GPR`, `register arg`, `accumulator register` → zero hits. Novel.

## Ranked candidate table

| # | Pattern | Source section | Grade | Notes |
|---|---|---|---|---|
| 1 | **`mir-destprop-branch-site-merge`** — a hot conditional dispatch where both arms load a different code pointer and converge into ONE call/branch site collapses predictor history into a single site (MIR `DestinationPropagation`, rustc ≥1.92 default); the fix duplicates the load+call into two distinct sites so each keeps its own predictor history. Measured: wasmi `i32.lt` branch handler ~2800 → >4200 CoreMark (~+50%) from this single fix; Stitch reproduced the same regression/fix. | "Accidental Rust Deoptimization" | **A− (GOAT candidate)** | Benchable: two-arm dispatch-loop microbench (merged `let f = if c {fa} else {fb}; f(x)` vs split call sites). Two honest failure modes the bench must adjudicate: (a) LLVM re-merges the "fixed" arm → Tie → honest annotation (the filter-count-to-any class); (b) win only on tail-call/indirect-dispatch shapes — regime annotation (blog: "only the tail-call-based dispatch configurations see improvements"). Suggest-only (a parse-only proposer cannot see MIR-level merging). |
| 2 | **`hot-dispatch-gpr-arg-budget`** — a hot dispatch/handler fn whose integer args exceed the target's GPR budget (sysv64: 6) pays a per-dispatch spill; move rarely-used args to the float domain (wasmi passes `instance` as f32-carrying) or pack scalars. The unstable `preserve_none` ABI is the future structural fix. | "Execution Handler Signature / Problem: Calling Conventions" | C+ | Suggest-only, architectural; microbench feasible (7-int-arg dispatch vs 6+float-pack) but the pattern is rare outside interpreter/VM loops. Queue as secondary; kill freely. |
| 3 | `Stable{Arena,Vec}` — append-only stable-address buckets + lock-free lazy materialization with per-entry atomic state (wasmi `CodeMap`) | "Lock-Free CodeMap" | drop (pattern, not rule) | No bounded Rust-idiom fix-space; recorded as a substrate-pattern pointer (katgpt-core / riir-neuron-db arena+papaya ecosystem) — revisit only if a measured consumer appears. |
| 4 | Fixed 64-bit cells (feature-flag widening of a hot struct penalizes non-users 5–10%) | "Fixed 64-Bit Cells" | drop | Architectural; no bounded fix-space; the corpus's `bool-fields-over-bitset-hot-reads` inverse-regime cousin already encodes the width-vs-traffic tension for Bools. |
| 5 | Accumulator registers / opcode fusion / `InstanceEntity` layout / `auto-dispatch` modes | engine internals | drop | Interpreter-internal design; no consumer-side Rust idiom to distill. |

## Yield estimate

1–2 rules after ~50% coordinator falsification. Lead: candidate 1. The realistic risk is candidate 1 measuring Tie on this toolchain (LLVM re-merging the fixed arm) — in that case it lands as an honest-annotation rule (the `varint-compose-word-single-store` precedent) or dies, per the verdict discipline.

## Bottom line

- **Mine as Batch 99** (Plan 067 filed): lead with `mir-destprop-branch-site-merge`, secondary `hot-dispatch-gpr-arg-budget`, route `rust_perf` (dispatch category).
- **Upgrade wasmi 1.0 → 2.0 workspace-wide** (riir-ai Plan 563): 13 pins / 7 repos in lockstep, fuel recalibration where budgets are tight, before/after bench evidence on `wasm_runtime_cmp` + moka `wasmi_bench`.
- Watch item: wasmi maintenance post-2026-10 (SDF sponsorship ends) — reassess before any further major-version jump.
