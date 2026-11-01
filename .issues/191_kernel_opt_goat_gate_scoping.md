# Issue 191 — scope the `kernel_opt` GOAT gate: real corpus + full numeric validator + promotion bars (the pending default-promotion precondition)

**Status:** OPEN — T1–T5 ALL EXECUTED; the campaign verdict is IN (Bench 124): promotion
REFUSED, `kernel_opt` stays opt-in (G1: 65 fixture defects → Issue 193; G2: tranche median
1.0000 < 1.06 on the CPU-portable surface). The issue remains open for the RE-CAMPAIGN path:
Issue-193 repairs → the owner-gated G2-convention decision → re-run G1+G2+G5. Delegated-owner
verdict 2026-11-01 (fallback reviewer sub-agent, the request_verdict claude_code channel
auth-unavailable on that box): FILE, don't execute — the prefill-vs-llama.cpp lane stays the
4090's top priority, and kernel_opt is a heal-domain gate, not a league lane; PARK would
leave a 1051-rule domain with a permanently unactionable promotion path.
**Claimed:** zed-shikuwa-i191gate, 1793523566, expires 2026-11-03T01:39:26+00:00 — T1 ratify + T2 harness build; 4090 box (shikuwa), the prefill league is between bouts with defenses banked

## The question

`kernel_opt` stands at 1051 rules (Batch 323, entries_gpu_147) with fixtures that are
standalone-compiled AND executed before entry, 15 category experts + a thermal router — and is
NOT promoted to default because "its GOAT gate (real corpus + full numeric validator) is
pending". Nothing defines that gate. This issue scopes it so the pending promotion has a
pre-registered, measurable path (pin the numbers first, measure later — the rust_perf_bench
precedent shape).

## What exists (the substrate this gate composes — consume, never fork)

- `rust_perf_bench` (opt-in): the PROMOTION GATE PRECEDENT — generated microbenchmarks with a
  deterministic alloc-count axis; Pareto + alloc-count validator; GOAT PASS on record.
- `kernel_verify` (opt-in): the compile-check oracle (STUB — delegates to the shared
  `RustAnalyzerPruner`).
- The kernel_opt fixtures: standalone-compiled AND executed before entry (every rule).
- The GOAT gate axes (workspace Feature-Flag Discipline): G1 correctness, G2 perf (`--release`
  mandatory), G3 no-regression, G4 alloc-free or equivalent; promotion to default ALSO requires
  the gain to be modelless (deterministic construction — kernel rewrites are; no training).

## Pre-registered bars (T1 RATIFIED 2026-11-01 — amended pre-measurement, never after)

**T1 verdict record:** delegated owner-verdict 2026-11-01, reviewer = native fallback sub-agent
(the `request_verdict` claude_code channel auth-unavailable on this box, the Issue-1115 arc),
session `544472de-ed7f-4f9b-a5c2-864d1fd4e509`: **REVISE (narrow)** — gate architecture +
pre-registration ordering RATIFIED SOUND; the four amendments below landed verbatim; proceed to
T2 without re-review. Grounded census at ratification: 151 corpus files / 1051 rules; 74 files
(~299 rules' files, ~700 embedded `#[test]` fns) carry executable std-only fixtures validated
ONCE manually at entry ("rustc --test --edition 2021, 0 failures" headers) — validated today
by NOTHING standing; only Batch 322 landed its fixtures in-tree.

1. **G1 — the full numeric validator (AMENDED surface, ratified honest).** A STANDING harness:
   extraction of every fixture-bearing rule's `ref_shape`/`opt_pattern` strings + per-rule
   compile + run under a two-posture CASCADE (`rustc --edition 2021 --test`; pid-suffixed temp
   dirs; a compile or test failure = the rule's G1 verdict FAIL). Floors pinned against an
   INDEPENDENT structural census (a text-level count of `#[test]` tokens per corpus source
   file, never the extraction path), and the harness asserts per-file RECONCILIATION —
   `raw_tokens − extracted_tokens` equals a membership pin (zero unless pinned; the
   gpu_63/64 in-tree `#[cfg(test)]` mods are the pinned rows) — an extraction failure
   counts as FAIL, never as absent-from-population (the green-zero law). Floors fire downward
   only: `min_fixture_rules ≥ 248`, `min_fixture_files ≥ 72`. **FLOOR CORRECTION (T2 first
   census, 2026-11-01):** T1's ≥290/≥70 came from the pre-harness per-file-density ESTIMATE
   (~299 rules / 74 files); the harness's own exact per-rule census measured **248
   fixture-bearing rules across 72 files**, and the ratified bar — floors PINNED TO THE
   CENSUS — pins those exact numbers. Any fixture rule or fixture file lost from the standing
   population reds. Bit-identity/tolerance per each fixture's own asserts (the corpus's own
   test-design laws: mod-13 symmetry-breaking values, must-fail twins, comparators).
   Parse-only rules stay on the parse stub, disclosed per-rule, never counted as validated.
   **Epistemic caveat (recorded):** G1 proves opt preserves ref on the fixture's own input
   distribution; ref correctness is inherited from vendored provenance and is disclosed
   per-rule — ref and opt can share a blind spot the fixture author never probed.
2. **G2 — the perf bar (bar RAISED, axis disclosed, both pre-pinned NOW).** Per-rule microbench,
   interleaved A/B (the sequential-AB law: a ratio of two sequential arms measures the box),
   **median improvement ≥ 6%** at `--release` (the measured flake band is < 6 pts — a 5% floor
   sits inside it), measured on the CPU-portable surface where the fixture exposes a ref-vs-opt
   pair (the healer never runs GPU code; a GPU-shaped rule's win is not CPU-measurable).
   **Rig exclusivity is MANDATORY, not advisory**: the campaign runs via
   `agent-mail rig run 4090 --exclusive` (or a verified-quiet box) and the bench record carries
   the `PROVENANCE:` line — a latency number without its BOX STATE is not a measurement.
   **Tranche selector is deterministic, pinned before measurement:** the largest clean
   CPU-portable ref-vs-opt families by embedded-`#[test]` count per file, tie-break by file name
   ascending, truncated at 15 files; the rest recorded `G2=NOT-MEASURED` per-rule with the
   disclosure. A GPU-host (riir-gpu/cubecl) microbench stays the named Tier-B re-gate trigger
   (the grandfathered re-gate precedent). A measured per-rule regression (median < 1.0×) exits
   the rule from the benchable set — the exit reason recorded PER-RULE, not just the demotion.
3. **G3 — no-regression.** The clippy_lints + rust_perf default surfaces' score-bench verdicts
   are byte-stable with kernel_opt enabled (the domain only ADDS a fan-out lane; the router
   must not perturb the existing domains' selections). **Disclosure boundary:** the CLI surface
   is inherently additive (findings-by-domain gains a `kernel_opt` term) — byte-stability is
   claimed for the EXISTING domains' verdicts, never for whole stdout.
4. **G4 — alloc-free.** The default build (kernel_opt OFF) stays byte-identical — the gate is
   compile-time-only cost; the ON-posture alloc axis rides the per-case fix path's existing
   counters (the rust_perf_bench deterministic alloc-count validator, reused).
5. **G5 — the utility + ON-posture cost leg (ADDED — the gain half for a suggestion domain).**
   On a PINNED real corpus (this repo's `src/**/*.rs` at the campaign commit + the `.raw`
   vendor clones at pinned SHAs named in the record): record the match census — suggestions
   emitted per category expert, the zero-match rule census (disclosed, never auto-demoted —
   retirement stays the `--necessity-bench` lane), and the added ON-posture scan latency per
   file (p50/p95) against the PRE-REGISTERED ceiling: **p50 ≤ 1.0 ms/file, p95 ≤ 5.0 ms/file**
   (pinned 2026-11-01, before any measurement). Utility floor: the domain must fire on ≥ 1 span
   through ≥ 3 distinct category experts in the pinned corpus — a domain that never fires (or
   fires through one expert only) fails the utility leg regardless of G1–G4.
6. **Promotion shape.** If G1–G5 pass: promote `kernel_opt` to `default` in ONE commit with the
   bench record + the GOAT table. Promotion promotes the SUGGESTION domain; kernel rewrites are
   never auto-applied without the per-rule validator verdict (the suggest-first discipline
   unchanged). If any gate fails, the domain stays opt-in with the failing axis named in the
   record (the G2-negative precedent: `RIIR_GDN_CORE_FUSED` — direction-positive without a
   passing gate stays opt-in).

## Tasks

- [x] T1 — ratify/amend the bars above (owner-verdict ping-pong; the numbers are the contract).
      DONE 2026-11-01: delegated verdict REVISE (narrow) → four amendments landed verbatim (G1
      census-pinned floors + epistemic caveat; G2 ≥6% + mandatory exclusivity + deterministic
      tranche; G5 utility/cost leg added; G3 disclosure boundary), no re-review required by the
      verdict.
- [x] T2 — the G1 standing fixture harness. DONE 2026-11-01 (`src/domains/kernel_opt/corpus/
      goat_g1.rs` + `examples/kernel_goat_g1.rs` + `tests/kernel_goat_g1_smoke.rs`, the
      `[[example]]`/`[[test]]` rows carry `required-features = ["kernel_opt"]`; clippy -D
      warnings clean at the gated posture; smoke PASS — census reconciled, floors hold, sampled
      execution green).
      - **Census reconciliation (the anti-circularity law, as measured):** raw-token equality
        was the wrong predicate — `entries_gpu_63/64` carry file-level `#[cfg(test)]` mods
        (1 in-tree `#[test]` each, OUTSIDE every rule string) and `gpu_concept` prose freely
        mentions `#[test]`. Extraction counts ALL THREE string fields (ref + opt + gpu_concept);
        per-file `raw − extracted` must equal a `CENSUS_DELTA_PINS` membership row (default 0,
        red both directions; orphan pins fail). gpu_63/64 pinned at 1 each with reasons; the
        in-tree mods stand under `cargo test --lib` and are recorded, never re-run.
      - **The compile cascade (measured, not assumed):** the corpus carries BOTH fixture shapes
        — cross-arm tests (the opt side calls the ref side's naive forms; 18 rules fixed by the
        PAIR posture: ref at the crate root + opt in a module with `use super::*;` — sibling
        modules + `use super::__ref::*` imports NOTHING, private items are not visible across
        siblings) and self-contained tests validated per-arm at entry (the older batches' ref
        arms are illustrative fragments). Pair tried first; on a pair COMPILE failure the
        test-bearing arms run standalone; a pair RUN failure does not fall back (a failing test
        fails everywhere). The failed pair attempt is disclosed in `RuleVerdict::discarded`.
      - **FIRST FULL RUN — the gate's first real catch (65/248 FAIL, 183 green, 803
        parse-only, 209.9 s, rustc 1.99.0):** the entry discipline was PARSE validation
        (`KernelOptDomain::validate`), not compile validation — 65 fixture rules do not
        compile or their own asserts fail: ~23 RUN failures (wrong asserts — verified
        standalone on segmented-checkpoint-cheap-replay-backward,
        componentwise-version-guard-rollover-silences-fast-path, head-dim-smem-tile-cap: each
        reproduces OUTSIDE the harness, so these are fixture bugs, not harness artifacts),
        ~10 unresolved-symbol fixtures (helpers named in no arm: real_tokens, run_once, Gate,
        text_following_tokens, find, process_tile, fold_one_slot_at_a_time, Device, Family,
        Finish), 2 fragment arms (per-block-int32-chunk-rescale-fp32-accum,
        moe-broadcast-quantize-once-scatter — mid-function snippets), 1 timeout
        (relaxed-spin-acquire-confirm-metadata-flag-wait), the rest misc compile errors
        (E0308/E0277/E0599/E0614, lifetime, Debug-derive). Full ledger + repair: Issue 193.
      - libtest failure reports go to STDOUT — the harness reads stdout first for run-failure
        diagnosis (the stderr-only read reported empty errors on the 22-run-fail class).
- [ ] T3 — wire the full numeric validator into `kernel_verify` (replace the STUB delegation
      path for the promoted set only; the stub stays for the long tail). AMENDED: propagate a
      `validated: fixture-green | parse-only` annotation end-to-end via the `context_notes`
      channel — a suggestion from a parse-only rule must not look identical downstream to one
      from a fixture-green rule.
- [x] T4 — the G3 score-bench byte-stability arm. DONE 2026-11-01
      (`scripts/g3_scorebench_stability.sh`, commit `2ff3c0a61`): the OFF posture
      (`--features score_bench`) vs ON (`+kernel_opt`), isolated target dirs, both lanes
      diffed after filtering the three documented volatile tail lines (history run-count,
      saturation verdict, frontier-snapshot state — run-order dependent, posture-independent).
      MEASURED: clippy lane 40 verdict lines byte-identical, perf lane 67 — **G3 PASS**. The
      score_history rows from the arm rides are appended (the standing record).
- [x] T5 — run the campaign on a claimed slot and take the promotion verdict. DONE 2026-11-01
      (**Bench 124**, this repo): claimed slot (lease `zed-shikuwa-i191gate`), 4090 verified
      QUIET via `agent-mail rig status`, rustc 1.99.0 repo pin.
      - G1 full: 248/72 floors hold; **183 green / 65 FAIL / 803 parse-only** — FAIL (the
        Issue-193 ledger).
      - G2 full: tranche 15 files / 74 green rules; 21 measured / 1 passing (2.05×) / 7
        regressing / 53 not-measured (no mirrored fn pair); **tranche median 1.0000 < 1.06 —
        FAIL** (the GPU-shape wins do not transfer to the CPU-portable surface — bar 2's own
        caveat). The G2 harness is STANDING (`goat_g2.rs` + the `kernel_goat_g2` example:
        interleaved 15-pair median, loud-zero, per-rule NOT-MEASURED disclosure, `--tranche`
        selector disclosure, file-borne PAIR rows under `--test` because a corpus arm may
        define its own `fn main`).
      - G3: PASS (T4 above). G4: PASS (structural — the domain compiles to nothing at
        default).
      - G5: NOT RUN this campaign (disclosed in the record — the verdict was determined by
        G1+G2; the census runs at the re-campaign on the repaired corpus).
      - **PROMOTION VERDICT: REFUSED — `kernel_opt` stays opt-in** (failing axes: G1, G2),
        per bar 6. The re-campaign path is in Bench 124: Issue-193 repairs → the G2
        convention decision (owner-gated: corpus bench-entry convention vs GPU-host benches
        vs closing the CPU-G2 axis with the disclosure) → re-run.
      - AMENDED execution note: the original T5 text named the lease + fresh `.benchmarks/`
        number + rig exclusivity + PROVENANCE — all satisfied (Bench 124 §Provenance); the
        "G5 match census in the same campaign" clause is the one disclosed deferral.

## Refs

- The delegated-owner FILE verdict: 2026-11-01 session zed-shikuwa-t52-seatwire (fallback
  reviewer sub-agent; verdict text in the session record).
- Precedent gates: `rust_perf_bench` (the promotion-gate shape), Bench 045/046/056 (the domain
  GOATs on record), `RIIR_GDN_CORE_FUSED` (the stays-opt-in precedent).
- Corpus state: riir-refine AGENTS.md §Role (1051 rules at Batch 323; per-batch history in
  HISTORY.md).
