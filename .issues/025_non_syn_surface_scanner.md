# Issue 025 — Non-syn surface scanner (`.wgsl` files + `#[cube]` macro bodies)

**Status:** OPEN (candidate; filed 2026-08-16 from user direction) — **T1+T2
DONE 2026-08-16: surface ≈ 20.5k+ lines (NOT downgraded); the WGSL scanner
prototype ships 3 precision-vetted default patterns + 1 experimental + 1
honestly-excluded (see tasks below).** T3–T6 remain.
**Origin:** user proposed "regex wasm" as a new tool. Rescoped on filing per the
measured evidence: the *job* is real (the non-syn blind spot), but **WASM is the
wrong layer for v1** — native-first; WASM reserved for third-party sandboxing,
which no requirement exists for yet.
**Related:** [Plan 005](../.plans/005_real_file_span_healing.md) §Phase 5 (deferral
reason = this blind spot; reopen trigger = ≥5 proposer-grade kernel rules),
§Phase 8 (`wasm_pruner` blocked on rubrc — the precedent for deferring WASM
layers on external-runtime maturity).

## Why

The healer AND the mining program are blind to every surface `syn` cannot parse:

1. **`.wgsl` kernel files** — the actual GPU kernel implementations behind the
   host-side orchestration rules mined in Batches 14–51. Several corpus rules
   have literal WGSL shapes (B44: shared cube-size const, smem-staging gate,
   one-thread-per-row anti-pattern, `array<f32,256>` declared-max tax; B47: the
   m2p out-projection fix shape).
2. **`#[cube]` macro bodies** — Plan 005 Phase 5's recorded deferral reason:
   vanilla `syn` cannot parse them (Proposal 002 caveat 1).

A scanner over these surfaces opens three things, in value order:
- **New mining material** — WGSL buggy→fixed pairs from riir-gpu git history
  (the fix commits behind Issues 697/698/701 have literal before/after `.wgsl`),
  feeding Phase 5's ≥5-proposer-grade-rules reopen trigger.
- **`--suggest` coverage on real kernel code** (suggest-only, same shape as the
  kernel domain today).
- Eventual proposer-grade rules for surfaces no Rust AST tool can reach.

## Scope ruling (the regex-WASM verdict, recorded)

- **Rust-code detection stays AST-exact.** Regex/token heuristics over Rust are
  a measured regression vector: the old `needless_range_loop` token-substring
  counter false-positived on `v[v[i]]` and on `minimum` containing `i` — fixed
  by exact `Expr::Index` analysis (Plan 005 T6.4 note 3). Do not regress this.
- **Non-syn surfaces: native `regex` crate first.** No WASM boundary for
  first-party rules — serialization tax + a rubrc-class external-runtime risk,
  for zero benefit while rules are ours.
- **WASM upgrade path (deferred sub-feature, not v1):** justified only when
  third-party user-supplied rules need sandboxing. Do not build it before a
  consumer exists (the decorative-metric trap, 4th occurrence guard).

## Tasks

- [x] **T1 — Inventory the surface.** Count `.wgsl` files + total lines across
      riir-gpu (and any WGSL elsewhere in the stack); count `#[cube]` macro
      bodies + their line ranges. Output: the mining target size. If the
      surface is tiny (< ~2k lines total), downgrade the issue honestly.
      **DONE 2026-08-16 (subagent inventory + coordinator re-verification of
      every reachability claim):** 128 `.wgsl` files / 11,036 lines (riir-gpu
      kernels 123 files / 10,830 lines; riir-train 3 / 118; vendor 2 / 88 —
      excluded), 117/123 riir-gpu kernels live via `include_str!` (6 dead:
      `attention.wgsl` + `lora.wgsl` 0-byte placeholders, `gram.wgsl`,
      `speft_masked_grad.wgsl`, `wall_mla_gate_project.wgsl` 0-ref,
      `speft_forward.wgsl` comment-only — all 6 coordinator-grep-verified);
      `#[cube]` production surface 142 items / ~9,810 body lines across 38
      riir-gpu files (brace-depth estimate). **Combined non-syn surface ≈
      20.5–21.1k lines ≈ 10× the 2k threshold → the issue is NOT downgraded;
      T2 proceeds.** Bonus adjacent classes found: inline WGSL raw strings in
      `.rs` (256 prod lines), 2 live `.metal` files (305 lines), inline CUDA C
      raw strings on the cudarc path (2,879 lines / 49 `__global__`
      kernels — same non-syn family, candidate T2 scope extension).
- [x] **T2 — WGSL scanner prototype.** Native `regex` patterns for the corpus
      rules that have literal WGSL shapes (start list: B44's four + B47's
      out-projection shape). Pattern source = the corpus rows themselves; each
      pattern must cite the rule_id it fronts.
      **DONE 2026-08-16 — `src/domains/kernel_opt/wgsl_scan.rs` (behind
      `kernel_heal` + new optional `dep:regex`) + `examples/wgsl_scan_smoke.rs`.**
      Evidence-driven disposition of the start list (live calibration smoke
      over all 123 kernel files, 28 findings / 15 files):
      - ✅ default patterns (zero observed FPs): `row-dot-split-inner-axis-
        block-reduce` (B44; `@workgroup_size(1…)` — 13 findings, real carriers
        m2p ×3 + spectralquant + attention_score family), `declared-max-array-
        taxed-per-thread-under-runtime-bound` (B46 — the issue's "B44 four"
        list named the `array<f32,256>` shape, which is B46's rule; 14
        findings incl. the m2p `scores`/`q` instances Issue 701 H-B documents
        as deferred), `share-cube-size-const-between-kernel-and-launcher`
        (B44; let-restatement form only — mux_kl_loss `let n = 64u;`).
      - ⚠️ experimental, OFF by default: `multi-head-out-proj-consumes-head-
        concat` (B47 out-projection). The calibration smoke caught its ONLY
        live firing on a legitimate `alpha`-lerp (`layer_fuse.wgsl` L61) —
        the documented imprecision class materialized on the first real
        corpus, so it ships behind `WgslScanner::with_experimental()` for
        git-history mining (where the shape IS the bug — Issue 701 pre-fix
        m2p), never the live suggest path.
      - ❌ not regex-expressible in v1: `smem-staging-gated-by-cross-thread-
        reuse` (B35 — needs within-workgroup reuse counts, i.e. dataflow).
      6 unit tests (buggy fires / fixed silent / lerp-FP demotion pinned /
      corpus-citation contract); clippy clean; 419 default / 587 kernel_heal /
      713 all-features lib tests pass. Comment-stripping applied to the
      line patterns (a `// …@workgroup_size(1,1,1)` doc mention must not
      fire — caught in the smoke by `attention_batched.wgsl` L12).
- [ ] **T3 — `#[cube]` body extraction.** Token-tree span location via
      `proc-macro2` (a `TokenStream` walk finds the attribute + item span
      without parsing the body as Rust), then the same regex pass over the
      span text. Edge cases to pin: nested macros, raw strings, `#[cube(launch)]`.
- [ ] **T4 — Wire as suggest-only.** New scanner output keyed to the `kernel`
      domain, zero proposer surface. `--fix` must NOT touch these files.
- [ ] **T5 — GOAT gate (falsifiable, precision-first):** on riir-gpu's own
      `.wgsl` files, the scanner flags ≥5 real anti-patterns drawn from the
      Batch 14–51 rule set, with **zero false positives on the clean-file
      control** (the post-fix versions of the same files must not fire —
      garbage-in guard for the mining loop). Latency: < 1ms/file (the G2 bar).
      Record in the next `.benchmarks/` slot at landing time.
- [ ] **T6 — Mining integration.** With the scanner live, mine WGSL buggy→fixed
      pairs from riir-gpu fix history toward Phase 5's reopen trigger (≥5
      proposer-grade pairs). Every batch re-runs the Issue 017 retrieval
      regression gate (corpus growth = crowding liability, Plan 005 Risk 5).

## Anti-goals

- No WASM in v1; no `wasm-*` dep additions.
- No proposers/auto-fix on non-syn surfaces (suggest-only).
- No replacement of AST analysis on parseable Rust.
- No new deps beyond `regex` (already in the tree via riir-rag) unless
  `tree-sitter-wgsl` proves necessary on T2 evidence.

## Risks

- **Regex false positives** — mitigated by the T5 clean-file control + the
  mining adaptation loop precedent (wrong suggestion → fix the pattern, never
  suppress).
- **Token-tree edge cases** (nested macros, raw strings) — pinned by T3 tests.
- **Surface too small to matter** — T1 decides; honest downgrade if so.

## References

- Plan 005 §Phase 5 + §Phase 8 + Risk 3 (the `#[cube]` parse-failure record)
- Batches 44/47 (WGSL rule shapes) — `.research/048`/`.research/052`
- Plan 005 T6.4 note 3 (the token-heuristic false-positive lesson)
- Issue 017 (the retrieval regression gate every corpus batch must re-run)
