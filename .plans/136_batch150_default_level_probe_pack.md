# Plan 136 — Batch 150: the staged default-level + pedantic probe pack

**Status:** LANDED (2026-09-17) — corpus 56 → 62, FIXABLE 41 → 45, VERIFY +2;
2 candidates declined/staged with probe evidence (see Research 177)

The staged pack from `.distill/001` (3 pedantic + 5 default-level candidates,
accumulation satisfied by Bench 094's default-eco histogram). Each candidate is
probed against real clippy FIRST (P10 discipline: a rule without probe evidence
does not land), suggestion shapes + applicability captured from JSON output, and
the fix-space tier adjudicated (FIXABLE / VERIFY-oracle / refuse) before any
authoring (P11: corpus entries carry fire + silence shapes).

Candidates (staged 2026-09-16/17):
- default-level: `write_with_newline`, `doc_lazy_continuation`,
  `doc_overindented_list_items`, `partialeq_to_none`, `legacy_numeric_constants`
- pedantic: `unreadable_literal`, `cast_lossless`, `redundant_closure_for_method_calls`

Prior evidence: Bench 094 manifest counts (write_with_newline 248/1crate
CONCENTRATION CAVEAT, doc_lazy_continuation 8/4, doc_overindented_list_items
11/5, partialeq_to_none 9/3, legacy_numeric_constants 23/5);
`unreadable_literal` is the only parse-only-PROVABLE pedantic row;
`cast_lossless` needs operand type (VERIFY-at-best);
`redundant_closure_for_method_calls` inherits the Bench 019 G5 E0631
deref-coercion risk from its demoted sibling `redundant_closure` — must probe
that specimen class before anything is authored.

## Tasks

- [x] P0: probe default-level candidates (temp crate, `cargo clippy
  --message-format=json`, toolchain pinned 0.1.98) — capture firing shapes,
  silence shapes, suggestion children, applicability per lint.
- [x] P0: probe pedantic trio (`-W clippy::pedantic`) — incl. the E0631
  deref-coercion specimen for redundant_closure_for_method_calls, the
  sign-change cast for cast_lossless, the threshold boundary for
  unreadable_literal.
- [x] Adjudicate per candidate: FIXABLE (parse-only provable) / VERIFY
  (oracle-lane consumable) / suggest-only / refuse — with probe evidence
  quoted in the research note. OUTCOME: 4 FIXABLE + 2 VERIFY landed;
  redundant_closure_for_method_calls declined (no bounded proposer path);
  doc pair staged (probe complete, port = next batch).
- [x] Dedup grep against all six corpora (name variants) — clean, 0
  collisions (also re-confirmed by --coverage-report's dedup law line).
- [x] Author admitted rules: proposers + fixtures (fire AND silence shapes),
  corpus count + FIXABLE/VERIFY tables updated.
- [x] Extend the standing syn-parse gate with new entries (the
  all_lints_fire_and_parse table now 62 rows + batch-150 comment block).
- [x] GOAT: G1 parse gate + G2 lib tests (1711/0 default; 1863/0 latent)
  + G3 clippy clean both states + retrieval floors green + score-bench run
  (96.4%, was 96.2% — slight improvement; rows #125-126 appended).
- [x] Research note `.research/177_batch150_staged_probe_pack.md` (Status
  line, probe matrix, adjudications, kills).
- [x] Docs: AGENTS.md count line (56→62 / 41→45); HISTORY.md batch entry;
  `.distill/001` queue update (same commit).
- [x] Commit `feat: batch 150: ...` + push (write set = clippy_lints + the
  Bench-094 dead-test rider; sibling's rust_perf WIP files excluded).

## Notes

- Box quiet at start (CPU 4%); sibling session active in riir-chain + holds 12
  rust_perf WIP files in this repo — compile may include their WIP; if their
  WIP breaks the build, isolate with CARGO_TARGET_DIR and report, never touch
  their hunks.
- `partialeq_to_none` and `cast_lossless` are type-aware lints: the parse-only
  lane can never fire them standalone; the honest route is the VERIFY table +
  P21 oracle lane (MachineApplicable children consumed verbatim) — only if the
  probe shows MachineApplicable.
- `write_with_newline` carries a 1-crate concentration caveat (nano-gemm 248 =
  39% of the sample's warnings) — the corpus entry must record it.
