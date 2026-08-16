# Issue 025 — Non-syn surface scanner (`.wgsl` files + `#[cube]` macro bodies)

**Status:** RESOLVED (T1–T6 all DONE 2026-08-16; issue file retained until the
next doc-hygiene pass — the Phase-5 reopen trigger it feeds stays honestly
open at 3/5 pairs). **T5 GOAT: Bench 033 ALL PASS** — 30→32 live findings
(after T6's P6 default pattern), 0 control FPs, 41µs/file. **T6: Bench 034**
— the repeatable mining harness + 2 mined patterns (P6 default / P7
experimental) + **3 validated buggy→fixed pairs** committed as fixtures
(701 B47, 686 B35, 698 B44-body). Surface ≈ 20.5k+ lines (NOT downgraded);
`#[cube]` body extraction live with exact T1 reconciliation; the scanner
wired into `cargo heal --suggest` as kernel-domain suggest-only.
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
- [x] **T3 — `#[cube]` body extraction.** Token-tree span location via
      `proc-macro2` (a `TokenStream` walk finds the attribute + item span
      without parsing the body as Rust), then the same regex pass over the
      span text. Edge cases to pin: nested macros, raw strings, `#[cube(launch)]`.
      **DONE 2026-08-16 — `src/domains/kernel_opt/cube_extract.rs` +
      `WgslScanner::scan_cube_rust` (absolute-line mapping) + the example's
      `--rs` mode.** Evidence:
      - Extraction reconciles EXACTLY with T1's independent grep inventory
        on the live tree (253 rs files scanned → **38 files / 142 items /
        9,810 item lines**; 130 launch + 12 plain — matching 177 text matches
        − 35 comment mentions, 134−4 launch, 43−31 plain). The two-method
        agreement is the correctness proof.
      - 9 unit tests pin the issue's edge-case list: nested macros
        (`comptime!`/`terminate!` are token groups — brace matching
        unaffected), raw strings opaque (one Literal token — unbalanced
        braces + fake `#[cube]` text inside never mis-extract), comment
        mentions dropped (lexer), attrs BETWEEN cube and fn (real shape
        `cubecl_runtime.rs:361`), attrs BEFORE cube excluded from the item,
        `mod`-nested items keep absolute lines, multi-byte chars, degenerate
        inputs, WGSL-shaped synthetic body line (tokenizes — mapping pinned).
      - **Two empirical `proc-macro2` fallback span facts discovered, now
        test-pinned** (the anticipated "token-tree edge cases"): fallback
        columns are **0-based** (a documented divergence from `proc_macro`'s
        1-based — caught by the indented-item test extracting one char
        early), and a group's `Span::end()` points **AT** the closing
        delimiter char (exclusive-end conversion required — the first run
        was missing exactly the final `}`). Both are encoded in
        `line_col_to_byte{,_exclusive}` docs + the exact-text tests.
      - Live findings: **0** on all 142 cube bodies (expected + correct —
        the precision-vetted patterns are WGSL-syntax-shaped; cube bodies
        are Rust-shaped; no speculative Rust-text pattern was added).
        The extraction + mapping pipeline is the T6 mining substrate;
        cube-shaped patterns land with the mined buggy→fixed pairs.
      - Gates: 419 default / 597 `kernel_heal` / 723 all-features lib tests;
        clippy 0 warnings in this crate (`--features kernel_heal
        --all-targets`). Zero new deps (`proc-macro2` + `span-locations`
        already unconditional).
- [x] **T4 — Wire as suggest-only.** New scanner output keyed to the `kernel`
      domain, zero proposer surface. `--fix` must NOT touch these files.
      **DONE 2026-08-16 — the `cargo heal --suggest` walk now scans `.wgsl`
      files + `#[cube]` bodies (behind `kernel_heal`; zero new deps).**
      Evidence:
      - `collect_rs_files` generalized to `collect_source_files(path, ext)`;
        the empty-path error now fires only when BOTH `.rs` and `.wgsl` are
        absent (a .wgsl-only target previously errored "no .rs files").
        Without `kernel_heal`, finding .wgsl files prints a one-line note
        pointing at the feature (no silent ignore).
      - The non-syn pass is **suggest-only by construction**: it runs ONLY in
        pure `--suggest` mode (`args.suggest` — under `--fix` the pass is
        skipped entirely), only when the domain filter is `auto` or `kernel`,
        and its output is rule citations — no proposer exists for these
        surfaces and the `--fix` edit path never consumes them.
      - Findings print in the suggest format (`path:L [kernel ] rule_id —
        excerpt`, cube-body findings suffixed) and the run summary carries
        `+N non-syn (wgsl/cube) finding(s)`.
      - E2E verified on the live tree: `--suggest --domain kernel` over
        `riir-gpu/src/kernels` → 30 findings / 24 files; `--domain perf`
        → the pass correctly skips (0 non-syn output); `fix_verify` state
        compiles; retrieval regression gate PASS (no corpus change).
      - Gates: 419 default / 597 kernel_heal / 723 all-features lib tests;
        clippy 0 warnings in this crate across `latent_retrieval`,
        `kernel_heal`, and `latent_retrieval,kernel_heal` states; no-dep
        check clean.
- [x] **T5 — GOAT gate (falsifiable, precision-first):** on riir-gpu's own
      `.wgsl` files, the scanner flags ≥5 real anti-patterns drawn from the
      Batch 14–51 rule set, with **zero false positives on the clean-file
      control** (the post-fix versions of the same files must not fire —
      garbage-in guard for the mining loop). Latency: < 1ms/file (the G2 bar).
      Record in the next `.benchmarks/` slot at landing time.
      **DONE 2026-08-16 — [Bench 033](../.benchmarks/033_non_syn_scanner_goat.md);
      ALL GATES PASS.**
      - **G1**: 30 live findings / 16 files (bar was ≥5) — the m2p findings
        cite anti-patterns the file's own header documents (Issue 701 H-B
        deferred caps + the phase-1 workgroup design).
      - **G-clean**: the git-history fix pair `8bb716954^`/`8bb716954`
        (Issue 701 H-C) committed as fixtures
        (`tests/fixtures/m2p_attention_{prefix,postfix}_701.wgsl`) +
        `tests/wgsl_t5_control.rs` (4 tests): the experimental B47 pattern
        fires pre-fix / silent post-fix (the mining pair is detectable, the
        fixed side is a genuine clean control); all 7 post-fix default
        findings adjudicated REAL + count-pinned (0 FPs). Honest caveat
        recorded: no git-history pair yet exists where a DEFAULT-pattern
        shape was removed — the default set's fixed-forms are pinned by unit
        tests; the live m2p is not a fully-clean default control (caps
        unfixed in-tree).
      - **G2**: 41.19 µs/file (201-line fixture, 2000 iters, release) — 24×
        under the 1 ms bar; release-gated test per repo convention.
      - **G3**: 419 default / 598 kernel_heal / 724 all-features lib tests +
        4 t5_control integration tests; clippy 0 this crate.
- [x] **T6 — Mining integration.** With the scanner live, mine WGSL buggy→fixed
      pairs from riir-gpu fix history toward Phase 5's reopen trigger (≥5
      proposer-grade pairs). Every batch re-runs the Issue 017 retrieval
      regression gate (corpus growth = crowding liability, Plan 005 Risk 5).
      **DONE 2026-08-16 — [Bench 034](../.benchmarks/034_wgsl_mining_integration.md);
      3 validated pairs (was 1), trigger honestly at 3/5.**
      - **The harness** (`examples/wgsl_mine.rs`, repeatable): walks git
        history per surface (`.wgsl` default; `#[cube]` bodies via `--rs` +
        `--extra-path` for the pre-split `weaver_gpu.rs` seam), scans each
        commit's pre/post with the FULL pattern set, and pairs at FINDING
        level (±10-line persistence window) with two mechanized refutation
        classes — MOVED auto-refuted (identical excerpt re-appears in post)
        and diff-association required (removed directly OR inside a changed
        hunk's span — the 698 stride fix leaves the accumulation as unchanged
        context). Two enumeration traps found by the run itself: `-G'#[cube'`
        matches attr-line touches only (the 698 body fix was invisible to
        it), and file-level rule silence never pairs in multi-kernel files.
      - **Two new patterns, mined from the fix pairs themselves:**
        `scan_single_slot_staging` (P6, **DEFAULT** — fronts B35
        `smem-staging-gated-by-cross-thread-reuse`, upgraded from the T2
        "not regex-expressible" verdict: bare-load writes + single-bare-ident
        non-varying reads = zero-reuse staging; live calibration 2 findings
        / 0 FPs across the 33 `var<workgroup>` files — the computed-write
        `role_transport`, tree reductions, compound transpose indexes, and
        `domino` multi-phase staging are all guarded silent) and
        `scan_serial_row_dot` (P7, **experimental** mining-only — fronts B44
        body form; 11 live serial-K-under-parallel-grid instances are
        textually unadjudicable).
      - **The mining run** (28 `.wgsl` + 673 cube-body pairs): 6 candidates →
        **3 GENUINE pairs** — B47×`8bb716954` (701 H-C, T5 fixture), B35×
        `2befc873e` (686 H7, new `covariance_{prefix,postfix}_686.wgsl`
        fixtures), B44-body×`959b6a0a9` (698 A-H1, new
        `dot_per_row_{prefix,postfix}_698.rs` cube-body fixtures) — + 1
        human-refuted (b75dd53c8: B47's documented legit look-alike —
        `grad_input_d` computed inside the loop — in a feature rewrite) +
        11 auto-refuted moves. **Phase 5 reopen trigger: 3/5, honestly
        open.**
      - **Corpus unchanged** (patterns front existing rules; no new entries
        → no crowding); Issue 017 retrieval gate re-run PASS (floors held).
        Live default-set count 30 → **32** (the +2 `gram.wgsl` s_row/s_col
        findings — the dead-file carrier of the exact 686-pre shape,
        adjudicated REAL).
      - Gates: 419 default / 602 kernel_heal / 727 all-features lib tests +
        wgsl_t5_control 4/4 + wgsl_mine_pairs 6/6; clippy 0 this crate;
        pattern-set citation contract held (6 ids, all in the 348-rule
        corpus).

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
