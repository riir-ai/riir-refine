# Plan 178 — Batch 183: P40/P41 receiver-scrutinee widening (clippy_lints corpus)

**Status:** COMPLETE (T9 = the landing commit)
**Date:** 2026-09-28
**Source:** `.distill/001_mining_queue_snapshot.md` post-mining intake P40 + P41 (probe halves closed 2026-09-27, box-default clippy 0.1.98; fix halves cadence-held to this batch slot).

## What

Four new corpus atoms widening receiver/scrutinee coverage on EXISTING lints (no new lints — the 96-lint count is unchanged), plus one latent-defect repair the atom work surfaced:

1. **manual_is_multiple_of** — variable-divisor atom (`n % d == 0` → `n.is_multiple_of(d)`); closes the P40 apply gap (riir-clippy L=2, riir-ai L=4 recorded misses).
2. **chunks_exact_to_as_chunks** — offset-subscript receiver atom (`payload[5..].chunks_exact(4)` → `payload[5..].as_chunks::<4>().0`); closes the katgpt-rs/riir-train/riir-ai receiver class.
3. **map_unwrap_or** — method-chain receiver + fn-path mapper atom (`strip_suffix(..).map(str::to_string).unwrap_or(..)` → `.map_or(.., str::to_string)`); the Elo-1222 toughest-span class.
4. **manual_let_else** — unsafe-block-scrutinee atom (`match unsafe { p.as_ref() } { … }` → parenthesized let-else); the riir-chain scrutinee class.
5. **REPAIR (found during atom authoring)** — the EXISTING chunks_exact atom's `fixed_code` does not compile (E0308: `from_le_bytes(chunk)` receives `&[u8; 4]`). Fix to `*chunk`. The pipeline never compiled `fixed_code` (freeze labels buggy-only; the standing gate is syn-parse) — the apply leg would paste non-compiling code and burn a verify-revert. The new atoms are authored compile-verified so the class stops here.

## Design constraints (why the entries are shaped as they are)

- **Sort-after placement:** `atoms()` picks the min-`buggy_code` entry per lint. Every new `buggy_code` sorts AFTER its lint's current minimum (`fn g…`/`fn tails…` vs `fn f…`/`fn ids…`), so the atom TABLE is unchanged → generation v2–v8 fixtures are unchanged → **no freeze-table regen owed** (asserted mechanically by the pin tests; if any pins red, run the regen in this batch instead).
- **Compiles standalone + under `mod gen_N { }` wrap:** each buggy/fixed pair is rustc-verified (edition 2021) before landing; the unsafe-scrutinee atom uses a LOCAL raw pointer in a safe fn (an `unsafe fn` + explicit block trips `unused_unsafe` at edition 2021; a pointer ARG deref trips deny-by-default `not_unsafe_ptr_arg_deref` — both fixture lessons recorded in P41).
- **Fixed forms match proposer output verbatim:** map_unwrap_or emits `{recv}.map_or({default}, {mapper})` (postfix-safe receiver, no parens); manual_let_else parenthesizes `}`-ending scrutinees; chunks_exact splices the receiver verbatim into `{recv}.as_chunks::<N>().0`.

## Tasks

- [x] T1 — rustc-verify all 8 snippets (4 buggy + 4 fixed) + the repaired existing fixed form (edition 2021, standalone file) — all compile clean
- [x] T2 — clippy probe: upstream fires on all 4 buggy shapes (1.98.1: chunks_exact_to_as_chunks + manual_is_multiple_of at default; manual_let_else + map_unwrap_or via targeted -W — map_unwrap_or REQUIRED the `unwrap_or_else` variant, the recorded `unwrap_or` store spelling is SILENT upstream, recorded in the atom comment)
- [x] T3 — corpus edits (found the one-entry-per-lint schema mid-flight: `seed_corpus_has_unique_lint_names` — the widening lands as multi-fn FILES inside the EXISTING entries, not second entries; plan + snapshot updated to match) + the E0308 repair
- [x] T4 — `cargo test --lib` (default) 2340/0 — incl. the freeze pins (ids unchanged: no new lints, no displacement) + freeze v2 e2e 30.8s 0-drift + freeze v3 e2e 26.3s 0-drift → NO table change, no re-freeze (only v2/v3 derive from the clippy seed corpus — v4 rustc_errors, v5 rust_perf, v6/v8 raw spans, verified by source)
- [x] T5 — retrieval floors: 21/21 HELD (212s, isolated CARGO_TARGET_DIR)
- [x] T6 — clippy ×2 clean (default + --all-features: zero lint findings; the grep-count noise was cargo's hard-linking cache warnings, 54 echo pairs, not lints)
- [x] T7 — score-bench ×2 appended (gen-1, heal 96.4% FLAT, parse_safety 1.0, decline_correctness 1.0 — floors HELD; the 2 "frozen labels stale" drift warnings are the pre-existing v1-lane state, unchanged by this batch — the v2/v3 freeze e2es that own label truth ran 0-drift)
- [x] T8 — snapshot update: P40/P41 fix halves marked `[x]` with the landing evidence; AGENTS.md count unchanged (96 lints — the corpus-size-pin doc carries the Batch-183 note)
- [x] T9 — commit `feat: batch 183: …` + push (same commit: corpus, tests, docs, snapshot) — LANDED `2fba14678` + pushed (verified: origin in sync; the box was unticked at commit time — the self-referential checkbox, ticked 09-28 by the follow-on idle session)

## Gates

- G1: syn-parse gate green for all entries (standing batch gate).
- G2: `cargo test --lib` green (incl. FROZEN_LABELS pins — the no-regen assertion).
- G3: clippy 0 findings in both feature states.
- Retrieval: floors HELD (improvement allowed, decay fails).
- Score-bench: run + append; heal_rate floors held.
