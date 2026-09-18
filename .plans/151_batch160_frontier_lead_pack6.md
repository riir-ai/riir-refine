# Plan 151 — Batch 160: the frontier-lead mining pack VI (2 lints)

**Status:** COMPLETE (2026-09-18) — all tasks `[x]`; closed with the batch
landing. T1's probe measured the swapped-operand boundary commutative-only
(`y = 1 + y` fires assign_op_pattern — NOT misrefactored_assign_op as
assumed; `y = 1 - y` silent) and the proposer mirrored it. En-route
instrument finding: this box's pty corrupts `cargo run > file` redirects
(rows circularly rotated, headers lost, exit 0) — the regen example now
writes its own output file (`fs::write`) and the built exe is invoked
directly.

Source: the Batch 159 re-arm's leads (frontier rows in
`.benchmarks/frontier_miner_rows.jsonl`, probe-verified 2026-09-18 by
`scripts/plan150_frontier_probe.py`) — consumed as rules, families
graduated, frontier re-armed with fresh probe-verified leads.

## The leads → rules

| lead | shape | tier | rationale |
|---|---|---|---|
| `from_str_radix_10` | `u32::from_str_radix(s, 10)` → `s.parse::<u32>()` | FIXABLE | Construction identity: std defines `impl FromStr for every int primitive` as literally `from_str_radix(src, 10)` — `parse::<T>()` calls the SAME function with the SAME `&str` input and returns the SAME `Result<T, ParseIntError>`. The turbofish re-spells the type the original call already named, so the rewrite is RESOLUTION-IDENTICAL (no new name resolution introduced — no shadow guard needed, unlike `legacy_numeric_constants` which INTRODUCED the bare primitive path). Receiver = arg1 text verbatim (paren-wrapped when not postfix-safe — autoderef makes `.parse()` compile on every receiver shape the `&str` coercion compiled on). Radix arg must be an int LITERAL parsing to 10 (suffix rides; value-check via base10 digits). Types: the 12 int primitives (exactly where `from_str_radix` + `FromStr`-via-radix both exist). Guards: exactly 2 args, bare 2-segment `TY::from_str_radix` path (no qself/leading-colon/multi-segment), attrs refuse, comments in glue refuse (keep-list = arg1). |
| `assign_op_pattern` | `y = y + 1` → `y += 1` (LHS ident must be the binary op's LEFT operand; all 10 assign-op pairs) | FIXABLE | For BUILT-IN ops on primitives, compound assignment is DEFINED as the binary op + store — identity by construction. The overloaded-operator hole (a custom `Add`/`AddAssign` pair may differ) is closed by TYPE DERIVATION: the LHS ident must be proven an ARITHMETIC scalar (ints + f32/f64) via a new derivation admitting (a) fn/impl-method params (annotation), (b) annotated `let y: SCALAR` locals, (c) `let mut y = <proven ident>` init-transfer, (d) suffixed-literal inits (`let mut y = 0i32`) — with an EXACTLY-ONCE binding rule (the ident bound exactly once in the whole fn incl. nested items — a second binding anywhere kills it) + a span-order guard (assignment after the local's binding for let-derived idents; closes the module-shadow-before-let hole) + the matcher SKIPS nested items (fns don't capture — their `y` is not our proven `y`). Unannotated unsuffixed-literal inits REFUSE (the `{integer}` E0689 class). `y = 1 + y` (operand on the right) is `misrefactored_assign_op`'s family — refused (boundary documented). Evaluation-order note: the y-read vs RHS-operand-order swap is unobservable for Copy scalar reads — identity holds. |

Counts: corpus 76 → 78 (2 new entries), FIXABLE 57 → 59, atoms 57 → 59
(both leads FIXABLE — the fourth consecutive all-FIXABLE pack if both
graduate).

## The graduation (frontier miner)

- BOTH families flip `covered: true` and join the HeldOut per-family loop
  (12th + 13th family loops — both FIXABLE, both loop-verifiable).
- The 2 consumed frontier rows are ARCHIVED out of the active store
  (the Batch-159 measured lesson: without the archive the e2e reads
  heldout_ok=false — the KnownSet family-done guard).
- Re-arm the frontier with 2 NEW oracle-verified families (probe-first:
  `scripts/plan151_frontier_probe.py`; recorded spare: `unit_arg` with the
  side-effect-block dodge (verified plan150, re-verified this round) +
  the best of the fresh candidates: string_lit_as_bytes /
  manual_string_new / bool_comparison / bytes_count_to_len /
  needless_late_init).

## Tasks

- [x] T0 Plan file (this) + `.plans/.highwater` 150→151
- [x] T1 Probe (`scripts/plan151_frontier_probe.py`): default-groups arms
       pinned BOTH leads' lint groups (both style/default-on; the plan150
       full-oracle arms could not distinguish the group) + shape arms
       (aop float/bitop/shift/param all fire; the swapped boundary:
       `y = 1 + y`/`y = 2 * y` fire assign_op_pattern, `y = 1 - y` SILENT
       — commutative-only, mirrored in the proposer; fsr signed +
       suffixed radix fire) + the re-arm candidates (6 verified pairs;
       nli's naive seed crossfires our own let_and_return — the
       twice-used-binding dodge clears it) — ARMED `unit_arg` (the spare,
       third turn) + `string_lit_as_bytes`
- [x] T2 `from_str_radix_10` proposer + module tests (9 arms: u32 +
       signed/usize receivers, suffixed radix, non-postfix-safe receiver
       paren-wrap, nested-in-larger-expr, radix≠10 refusal, variable
       radix, non-int type + leading-colon + 1-arg refusals,
       comment-in-glue)
- [x] T3 `assign_op_pattern` proposer + module tests (15 arms: the seed
       init-transfer shape, all 10 op classes, float, swapped
       commutative ×2 + non-commutative refusal, annotated + suffixed
       lets, mut-param, paren rhs, 2-hop chained derivation, unsuffixed
       int/float refusals, generic + bool refusals, `&&`/`||` refusal,
       rebound ident ×2 refusals (let + closure param), nested fn own
       pass, comment-in-glue, impl-method param)
- [x] T4 Dispatch rows + FIXABLE 57→59 + tier-count test (len 59)
- [x] T5 Corpus entries ×2 + size pin 76→78 (`corpus_size_is_78`) + header
       doc + `all_lints_fire_and_parse` rows 76→78 + mod.rs doc counts
- [x] T6 Miner graduation: families flip ×2 + 2 new frontier families
       (`unit_arg` + `string_lit_as_bytes`, probe fixtures verbatim) +
       family_cases + hermetic probe markers (family_probe += 2 arms:
       `from_str_radix(` / `= y + `; frontier_probe = the new leads'
       markers) + test counts (heldout 24→28 cases / 11→13 families;
       frontier 4→4)
- [x] T7 Freeze regen v2+v3 (real oracle) + splice + record paragraph
       (the temporary example computes both tables deterministically from
       specs()/specs_v3() + the known bad sets — v2 82 labels / 13
       excluded, v3 83/16; bad sets UNCHANGED 6/7); v2 e2e 34.1 s PASS +
       v3 e2e 193.5 s PASS (assertions = label fences + M4, 0 drift)
- [x] T8 frontier-miner e2e — VERDICT ✓ (32 seeds clean; 13 family
       loops, 2 fresh heldout admissions; the re-arm's 2 leads admitted
       exact-fire; 11 committed rows verified 0 drift; 4 rows appended;
       band fold 13/13); the 2 consumed frontier rows ARCHIVED first
       (removed from the jsonl — git history the record)
- [x] T9 score-bench row #136 — heal_rate 96.36% FLAT (vocab 59) +
       frontier snapshot (generation 7)
- [x] T10 Gates: lib default 1917/0 · lib `latent_retrieval` 2070/0
       (floors held) · lib `frontier_miner,score_bench` 2234/0 · clippy
       `-D` clean in 4 states (2 doc_markdown findings in the new miner
       docs healed by our own healer — the dogfood loop; 4 structural
       findings in the new proposer fixed at root: LetScan hoisted to
       module level, counts by-ref, doc-comment blank line)
- [x] T11 Docs: Research note (187), HISTORY batch entry, `.distill/001`
       queue line, AGENTS counts (78/59) + frontier_miner bullet, README
       row; commit + push
- [x] T12 Global fleet binary rebuild (release set, isolated worktree
       beside the repo + `/tmp` target) + `--corpus-stats` verify —
       clippy_lints 78 through the installed copy

## GOAT gates

- G1: module tests + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` + corpus `all_fixed_code_parses`.
- G2: score-bench row (heal_rate flat vs the 96.4% baseline ± the 2 new
  atoms).
- G3: freeze e2e v2/v3 re-verified green (0 drift); frontier miner e2e
  VERDICT ✓ ×2 (graduation incl. the 2 new family loops + fresh leads).
- G4: no new deps; all edits span-preserving (the toolkit contract).
