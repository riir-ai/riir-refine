# Issue 191 — scope the `kernel_opt` GOAT gate: real corpus + full numeric validator + promotion bars (the pending default-promotion precondition)

**Status:** OPEN — scoping only; no campaign runs until the bars here are ratified and a session claims the campaign. Delegated-owner verdict 2026-11-01 (fallback reviewer sub-agent, the request_verdict claude_code channel auth-unavailable on that box): FILE, don't execute — the prefill-vs-llama.cpp lane stays the 4090's top priority, and kernel_opt is a heal-domain gate, not a league lane; PARK would leave a 1051-rule domain with a permanently unactionable promotion path.
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
   standalone `rustc --edition 2021 --test` compile + run (pid-suffixed temp dirs; a compile or
   test failure = the rule's G1 verdict FAIL). Floors pinned against an INDEPENDENT structural
   census (a text-level count of fixture-bearing rules/files in the corpus source, never the
   extraction path), and the harness asserts `extracted == census` — an extraction failure
   counts as FAIL, never as absent-from-population (the green-zero law). Floors fire downward
   only: `min_fixture_rules ≥ 290`, `min_fixture_files ≥ 70` (at ratification: 299/74).
   Bit-identity/tolerance per each fixture's own asserts (the corpus's own test-design laws:
   mod-13 symmetry-breaking values, must-fail twins, comparators). Parse-only rules stay on the
   parse stub, disclosed per-rule, never counted as validated. **Epistemic caveat (recorded):**
   G1 proves opt preserves ref on the fixture's own input distribution; ref correctness is
   inherited from vendored provenance and is disclosed per-rule — ref and opt can share a blind
   spot the fixture author never probed.
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
- [ ] T2 — build the real-corpus harness (T1.1's source list → fixture kernels per family;
      blake3-pinned inputs; the harness runs per GPU generation with the box-state PROVENANCE
      line — the Issue-021 law). AMENDED by the T1 verdict: the harness = the G1 standing
      fixture executor (extraction == independent structural census; per-rule rustc --test
      compile+run; floors fire downward only) + the smoke gate with floors.
- [ ] T3 — wire the full numeric validator into `kernel_verify` (replace the STUB delegation
      path for the promoted set only; the stub stays for the long tail). AMENDED: propagate a
      `validated: fixture-green | parse-only` annotation end-to-end via the `context_notes`
      channel — a suggestion from a parse-only rule must not look identical downstream to one
      from a fixture-green rule.
- [ ] T4 — the G3 score-bench byte-stability arm (reuse the existing score-bench freeze-table
      machinery; plan 137 T7.5's regeneration law applies to any corpus-touching landing).
- [ ] T5 — run the campaign on a claimed slot (claim-lease discipline), record under a fresh
      `.benchmarks/` number, and take the promotion verdict to the owner with the full table.
      AMENDED: rig exclusivity MANDATORY (`agent-mail rig run 4090 --exclusive` or verified
      quiet) + the PROVENANCE line; G5 match census on the pinned corpus in the same campaign.

## Refs

- The delegated-owner FILE verdict: 2026-11-01 session zed-shikuwa-t52-seatwire (fallback
  reviewer sub-agent; verdict text in the session record).
- Precedent gates: `rust_perf_bench` (the promotion-gate shape), Bench 045/046/056 (the domain
  GOATs on record), `RIIR_GDN_CORE_FUSED` (the stays-opt-in precedent).
- Corpus state: riir-refine AGENTS.md §Role (1051 rules at Batch 323; per-batch history in
  HISTORY.md).
