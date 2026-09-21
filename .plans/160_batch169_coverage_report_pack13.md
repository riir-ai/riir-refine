# Plan 160 — Batch 169: the coverage-report pack XIII (the staged alternates)

**Status:** COMPLETE (2026-09-21 — T13/T14 landing at commit time; all
GOAT gates green)

The Batch-168 staging (Plan 159) left the two probe-verified alternates as
the designated pack XIII leads; the default-eco wave-5 derivation is the
fallback and is NOT owed (wave-4's remainder is consumed-or-single-crate-
anecdote). Both leads fired under the plain oracle in the plan159 probe
(`scripts/plan159_pack12_probe.py`, 2026-09-21); this batch's own probe
(T1) re-verifies and extends the envelope — binding shapes, co-fire sets,
and clippy's own suggestion text — before any proposer is authored
(the probe-first law):

| lint | group | joined count | plan159 probe | bounded scope (staged) |
|---|---|---|---|---|
| `match_ref_pats` | style | 17 | FIRES | patterns-only `&` strip; NO-BINDINGS sub-patterns only (the binding-mode hazard); EVERY arm `&`-led |
| `into_iter_on_ref` | style | 14 | FIRES (co-fires `iter_count`) | the parenthesized `(&e).into_iter()` receiver only; typeck shapes out of reach |

Dedup pre-read (§2, adjudicated at admission): `match_ref_pats` vs
`redundant_pattern_matching` (Option/Result `if let Ok(_)` — different
lint, different seam) and vs `needless_borrow` (borrow removal at call
sites — different lint); `into_iter_on_ref` vs `iter_nth_zero` /
`iter_next_loop` (iterator-method families, no into_iter seam) and vs the
Batch-167 probe REJECT `needless_borrowed_reference` (a DIFFERENT clippy
lint, silent everywhere — recorded in plan158's probe).

- [x] T1 probe `scripts/plan160_pack13_probe.py` — PASS (2026-09-21).
      Envelope measured: mrf fires all-`&`-arms incl. `&_` and guards;
      SILENT on mixed/wildcard/or arms (mirrored); the BINDING shape
      co-fires `needless_borrowed_reference` (probe) — the corpus atom is
      the clean no-binding spelling; clippy suggestion = deref scrutinee +
      strip `&`s, MachineApplicable. iir fires all 5 shapes; suggestion =
      METHOD RENAME ONLY (`iter`/`iter_mut`, parens kept); `.count()`
      tails co-fire `iter_count` (the gen-8 shaped class); for-head fires
      but the rename births explicit_iter_loop → refused by us.
      Probe-harness lessons: `write_text(encoding="utf-8")` REQUIRED (the
      cp874 locale write corrupted lib.rs — the katgpt-rs locale_io
      class, reproduced); one clippy run PER FIXTURE for exact attribution
      (span-text attribution fails: fn names sit outside diagnostic
      spans); probe output ASCII-only (cp874 console).
- [x] T2 `match_ref_pats` proposer (patterns-only `&` strip, binding-free
      sub-patterns, plain-ident scrutinee, nested-ref refusal for
      fixpoint safety) + 12 module tests
- [x] T3 `into_iter_on_ref` proposer (parenthesized `(&e)`/`(&mut e)`
      receiver, signature-proven container params — the
      `param_heads_over` precedent with `body_rebinds` now pub(crate);
      for-head + closure refusal) + 13 module tests
- [x] T4 registration: doc-table rows + pub mods + FIXABLE_LINTS 71→73
      (count pin bumped) + dispatch arms + `all_lints_fire_and_parse`
      rows 88→90
- [x] T5 corpus.rs entries ×2 + count test 90→92
- [x] T6 G1: clippy_lints module + corpus tests green — 1161/0
- [x] T7 freeze regen gen2/gen3 — DONE (3-run convergence: run 1 caught
      a non-compiling iir fixture — E0515, a param-borrowing return;
      repaired to `.next().copied()`; run 2 caught the v3 CASCADE branch
      the regen had not mirrored — the e2e's silent exclusion of
      composites-of-unverified-atoms; run 3 converged). Tables: 91/18 v2
      (109 specs at 73 atoms), 91/22 v3; bad atoms UNCHANGED 9 v2 (the 8
      known + nwd) — BOTH new atoms CLEAN. e2es: v2 91/0/18 M4 PASS
      (31.3s); v3 91/0/22 stratification PASS (23.3s); generations module
      invariants 35/0. Regen record paragraph landed in generations.rs;
      temp example + Cargo.toml row + splice script deleted.
- [x] T8 G2: lib default **2115/0** (B168: 2083) + kernel_heal
      **2468/0** (2436)
- [x] T9 retrieval gate: latent_retrieval suite 17/17 + full
      latent_retrieval,score_bench lib **2428/0** (floors held, no re-pins)
- [x] T10 G3: clippy -D clean BOTH states (default --all-targets +
      all-features --all-targets; my 5 new-code mechanicals fixed —
      doc_markdown backtick, match_same_arms merge, BTreeSet not Map,
      bool_assert, plus TWO pre-existing-at-HEAD gating one-liners from the
      sibling's 56f0b0fd: the fade-self-test bool_to_int/match_same_arms +
      heal.rs:5780 bare-fn E0631 on the all-features bin lane — the fix
      their own comment described but did not apply)
- [x] T11 score-bench #149/#150 **96.4% FLAT** (0.96363634, byte-identical
      to #148; created 1.8% unchanged; vocab_size 73; the 2 drift warnings
      = the pre-existing toolchain pair) + frontier snapshot appended
      (store 865ae1da, gen 7, pop 17 — this box's store, the per-store
      clock)
- [x] T12 Research 197 + AGENTS.md counts (90→92, FIXABLE 71→73) + the
      batch entry (this file's T-list is the .distill/001 pointer's
      companion — the snapshot edit follows)
- [x] T13 commit `feat: batch 169: ...` + push — **COMPLETE `6d32c3b1`**
      (18 files; the shared-worktree sibling's three docs/intake commits
      391817eb/9ac91480/19c93f2f landed beneath it mid-batch — disjoint
      regions, no conflicts, published together)
- [x] T14 global binary rebuild — DONE: built from `6d32c3b1` in an
      isolated worktree beside the repo (release feature set,
      `latent_retrieval,kat_service,kat_transport,fix_verify`), installed
      over the COPY at `~/.cargo/bin/cargo-heal.exe` (NOT a symlink on
      this box — the 09-20 23:22 install was stale at clippy_lints 86;
      the repo `target/release` copy vanished between cp and verify —
      irrelevant, the installed copy is the load-bearing one); verified:
      release set complete + `--corpus-stats` clippy_lints **92** /
      rust_perf 145 / dist 6 / kernel_opt 620. Worktree + temp targets
      removed.
