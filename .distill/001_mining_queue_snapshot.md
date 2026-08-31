# Mining queue snapshot — the distill-program resume state

> **FOLDED 2026-08-31:** the closed-batch history (every idle-loop cycle entry through
> Batch 93 + the FULL post-mining intake records + the sweep/batch-landing sections)
> lives VERBATIM at `.research/108_mining_queue_snapshot_history.md`. This file keeps
> only the live queue — keep it under ~400 lines; fold again when history re-accumulates.
> This file IS the resume state: read at session start, update it in the same commit
> as each batch. Batch DETAIL lives in `.research/NNN_*.md` — this file holds only
> the rolling queue (next targets, sibling-blocked files, corpus count, standing
> lessons). Canonical batch ledger = AGENTS.md batch entries; per-batch detail =
> `.research/NNN_*.md` + `.plans/NNN_*.md`.

## Live queue state (2026-08-31, post-Batch-93, M3 box)

- **Corpus counts:** kernel_opt 411 · rust_perf 96 · clippy_lints 45 · sec 11 · docker 31
  (AGENTS.md is the authoritative ledger — bump there per batch, snapshot counts are a convenience copy).
- **Last landed:** Batch 93 (`501fdb2` — ktransformers residue disposition, ZERO corpus
  delta, the last mined-source lane CLOSED) + the t48 substrate tie-break fix (`944741f`).
- **Next executable (cadence-gated):**
  - **The arxiv pool re-walk: DONE 2026-08-31 ~23:20 +07** (`.research/109` — walk #3,
    covered the Sunday-08-30 announcement window that walk #2 never saw; zero corpus delta,
    two doctrine citations: LAH/S4-FIFO → riir-dao, TerraceMoE gates → doctrine; the
    spec-dec league lane returned ZERO new entries). **Next window 09-02+** (Monday's
    announcement lands Tue ~07:00 +07; daily differential cadence established).
  - **09-15 MCP search quota reset** → online perf-search pool walk (queue item 11).
  - League opponent-watch: watch→intake step CURRENT as of 08-31 (the Qwen3.8-Flash-Next
    league verdict landed — history note, 5th-cycle entry; re-arm = a ≤24-GB-feasible
    Flash-Next-class variant with native MTP). **Rapid-MLX reference row PINNED 08-31**
    (riir-ai `e6b13a0cd` = Bench 831: short 30.74 dec/138.25 pre, long 14.08/124.85
    medians, PROVISIONAL window — clean re-run owed behind the 782 autofire retry;
    prefill 0.958× llama.cpp = second engine confirms the 27B prefill bar; corpus
    delta ZERO — doctrine citations only).
- **Sibling lanes (do not touch):** riir-train — sibling actively building
  (`/tmp/riir-train-499b`); the 4090 perf lanes — sibling PV work mid-flight as of 08-29
  (re-verify lane state at session start before any 4090 unit).
- **Post-mining intake: EMPTY — no open classes** (compact records below; full text in the
  history note).
- **Owner-gated tails:** `.issues/044` (paging secrets `ALERT_WEBHOOK_URL` +
  `MONTHLY_BUDGET_USD`), `.issues/045` (T7 launchctl activation) — verified 08-31.
- **Queue items 6/7** (boundary-guard + substrate-first drift audit): last run 08-29,
  quarterly — not due until ~11-29.
- **Hygiene:** the snapshot fold itself is this unit (`.research/108`); no other
  pending hygiene. Plans tree clean (0 unchecked boxes); `.issues/` = the two
  owner-gated tails only.

## Standing rules (pointers — full text in the skills + AGENTS.md)

- Plan-first per batch (Protocol A step 0): `.plans/NNN` via `.plans/.highwater`,
  re-scan the folder at WRITE time (dual-allocation discipline).
- Per batch, ONE commit: research note (with a `**Status:**` line) + AGENTS.md batch
  entry + corpus-count bump + this snapshot's one-line queue update.
- Generator lessons (edit_file escape-stripping, `ensure_ascii=False`, line-initial
  `+ ` doc-markdown, CRLF check before fmt, count-pin walks) are recorded per-batch in
  AGENTS.md entries — read the last 2–3 batch entries before writing corpus content.
- kernel_opt file-mining corpus is COMPLETE (B51 declared); open intake channels:
  paper distills (the B70+ arxiv channel), measured-GOAT distillations (the B49
  reopen path (c)), online search (quota-gated), arxiv listing walks.
- Instrument note: `scripts/self_evolve_audit.sh` §3/E5 parses `- [x] P\d` items in
  the intake section below — keep that exact item format when updating.

## Post-mining intake (compact — full records in `.research/108_mining_queue_snapshot_history.md`)

All classes closed; the standing divergence (P4 residual) has no action planned.

- [x] P1 **macro-nesting family** — CONSUMED 2026-08-28 (the descent unit): match_bool +
      uninlined macro-arg descent with the oracle boundary (uninlined direct-child skip —
      format_in_format_args owns that position); 9 fixtures (5 match_bool + 4 uninlined);
      score-bench run #9 steady (96.2/1.9/100/100); live sites healed e2e in ndb /
      riir-train / riir-ai (compile-gated per target).
- [x] P2 **array-literal guard family** — CLOSED 2026-08-29: documented permanent
      divergence, verdict evidence-backed (the `2bf2a44` E0308 proof; population measured
      36 raw hits → 6 map-form → 3 must-not-fire); fails the P7 value/rate bar (machinery
      ~60–80 lines for 3 cosmetic sites); joins the we-stay-silent table.
- [x] P3 **named-arg-with-rename** — CONSUMED 2026-08-29: probe-first oracle table
      (rename + width-$ re-point + identical-delete compositing; field-access values
      SILENT; unreferenced named arg = hard rustc error), the `RenameArg` emitter with
      17 fixtures, module 61/61, e2e byte-for-byte vs oracle; adopted cross-repo
      (katgpt-rs `2e445664` 42 edits, ndb `ffb3234`); + the stale-binary mtime
      discipline (check `~/.cargo/bin/cargo-heal` mtime vs the last proposer commit
      before adoption runs); incident record 2026-08-31: the installed binary sat STALE at the
            08-30 08:48 build across five 08-31 src/ commits (P15 narrowing, P12 suppression,
            Batch 92 corpus +11, guard repair, t48 tie-break) despite the P12 commit message
            claiming "installed binary rebuilt" — cause suspected: the session built under a
            /tmp CARGO_TARGET_DIR and the claim referred to that build. Rebuilt from HEAD
            23:28 +07 (`kernel_opt 411` probe PASS + suggest smoke) — the mtime-vs-commit
            check is load-bearing, trust the mtime not the message.
- [~] P4 **created-warning pair** — PARTIALLY CONSUMED 2026-08-28 (`1941712`): the
      plain-expression closure gap was already closed in expr_is_bool; the live residue
      was RECEIVER classification (turbofish `get::<` prefix match, `.lock(` joined
      RESULT_SHAPES); 8 sites terminal-fixed manually (opaque domain-method closure
      bodies — parse-only undetectable). Remaining open: crate-local Option-producing
      helpers (`env_var(` etc.) stay Unknown BY DESIGN (allowlist overfitting) —
      **no action planned** (documented divergence).
- [x] P5 **outcome-verdict wiring** — feed `--verify`-run outcomes + post-heal clippy
      results back into memory tiers. Tracked in [Issue 037] T2 (root-caused — CLI
      records outcomes as `false` only). **LANDED 2026-08-24 via 037-T2 (`667562a`):**
      kept-edit verdict matrix + the clippy lint-resolution oracle + the durable
      `.heal/trajectories.kv` landing; legacy rows stay Heuristic (history is not
      re-judged). Self-evidence-excluded from the E5 gate by design.
- [x] P6 **match_bool guard-on-first-arm + single_match_else literal arms** — CLOSED
      2026-08-29: 20+ shapes probed vs clippy 1.97 across 5 probe crates; flat-conjunction
      guards (`true if g` → `if flag && g`), arm-order-preserving emission, BOOL literals
      confirmed single_match_else-SILENT, and the single-stmt-else OVERSHOOT gate (real
      clippy silent there — never outrun the oracle); match_bool 24 tests + sm_else 28
      tests; e2e byte-for-byte on all 3 new shapes.
- [x] P7 **no-proposer default lints** — CONSUMED 2026-08-29: `manual_is_multiple_of` →
      VERIFY_LINTS tier (corpus 40 → 41; the receiver-allowlist lane rejected — covers
      ~none of the real sites), signed-site auto-revert proven e2e; retrieval floor
      re-pinned per the floor contract (blended top-3 97/101 → 95/101, fix-before-repin,
      baseline isolated via a HEAD worktree); `if_same_then_else` stays owner-gated.
- [x] P8 **unnecessary_sort_by deref-binding shape** — CLOSED 2026-08-29: 12-shape probe
      (deref → MA `sort_by_key(|&a| a)`, plain sort_unstable_by → MA sort_unstable — was
      also proposer-skipped); `Binding::{Plain,Deref}` classification + same-form-only
      rule; 13 module tests; e2e reproduces the oracle byte-for-byte.
- [x] P8b **unnecessary_sort_by field-key family** — CLOSED 2026-08-29: the key family is
      GENERAL (fields/methods/postfix/tuples/consts/external idents); THE FALSE-ORACLE-FACT
      correction (`custom_comparator_unchanged` asserted silence where the probe shows MA —
      test now fires); Paren-grouped keys deliberately skipped (oracle de-parens — parity);
      30 module tests; e2e parity on all 4 fired shapes.
- [x] P9 **compile-breaking proposal cluster in ruliology/player.rs** — CLOSED 2026-08-25,
      commit `64820c1`: root cause = the map_unwrap_or → map_or rewrite needs an unsize
      coercion (`&fn(..)` → `&dyn Fn(..)`) that map_or's free inference variable
      structurally cannot perform; fix = skip mappers whose TAIL casts to a trait-object
      type; 5 new fixture tests pin the guard; **score-bench run #5 delta** (heal
      98.1% → 96.2% — the honest we-stay-silent pin; created 1 unchanged, parse_safety
      100% held); the two innocent single_match_else edits landed on the real file
      afterward (riir-ai `6ca4f00e3`, 70/70 tests green).
- [x] P10 **needless_borrows_for_generic_args + needless_pass_by_value** — CLOSED
      2026-08-29, architecturally out-of-scope: `buffer.extend(&items)` vs
      `buffer.extend(items)` BOTH compile with DIFFERENT semantics — the compile gate
      cannot see it, so even VERIFY-tier is unsound for these lints (semantic-and-silent
      breaks); the correct consumer is `cargo clippy --fix` or the rustc_errors domain.
- [x] P11 **`is_postfix_safe` includes `Expr::Cast`** — CLOSED 2026-08-29 (same-day):
      `x as u64.method()` groups the method into the TYPE position (broken); cast
      receivers now parenthesize; the P7 local exclusion subsumed + the two identical
      LOCAL copies in manual_map/unnecessary_unwrap deduped to the shared helper (the
      copy drift that CAUSED P11); latent `!x as u8` wrong-math closed as
      defense-in-depth; 828/0 lib, score-bench #14 steady.
- [x] **P12 — verify-revert CHURN: the verify gate does not suppress re-proposal.**
      CONSUMED 2026-08-31: the re-proposal suppression filter — the fixpoint pass drops
      any edit whose exact (rule, restore, replacement) pair has 2+ consecutive
      compile-gate fails in the verdict backend (two, not one: whole-file reverts
      attribute fails to every edit in the file — run 2 is the retry window); text
      change auto-unlocks; kill-switch `RIIR_HEAL_SUPPRESS_REVERTED=0`; e2e proven
      (`tests/p12_suppress_reverted_e2e.rs`); score-bench #23 flat (96.2/1.9/100/100).
- [x] **P14 (mis-diagnosed) — MOOTED:** the "unformatted bare-block shape" was the
      manual_let_else mangle × fmt-skip interaction, not an items_after_statements
      emission defect. Closed; no code change.
- [x] **P15 — useless_format emitter can produce invalid cast-then-method text.**
      CONSUMED 2026-08-31 (probe-first, 4 rounds vs clippy 1.97): the single-arg branch
      is TYPE-GATED upstream (peeled type str/String) so cast/binary/unary/paren args
      are ALL oracle-silent — the single-positional path narrowed to the one
      syntax-provable firing shape (a `.to_string()` method-call arg, which also kills
      the double-call emission); every other non-ident arg refuses; refusal tests + 4
      new fixtures; score-bench run #22 UNCHANGED; retrieval floors 13/13.
- [x] **manual_let_else binding-name mangling** — ROOT-CAUSED + FIXED 2026-08-31
      (CRITICAL, 15 real gated breaks): the emitted pattern kept the ARM's inner binding
      instead of the OUTER let name; fix = `renamed_single_binding` byte-precise
      span-splice + the SHORTHAND-FIELD refusal (renaming rewrites the FIELD — refuse);
      4 fixtures.
- [x] **unnecessary_sort_by field keys (E0507)** — REFUSED 2026-08-31 (parse-only
      type-blind): `sort_by_key(|a| a.k)` moves the field out of `&K`, compiles only for
      Copy; the whole field subfamily now refuses (12 fixtures flipped to refusal tests
      + 5 new); whole-binding method keys keep firing.
- [x] **use_self `Self<A>` (E0109)** — FIXED 2026-08-31: `Self` cannot carry explicit
      generic args; whole-segment replacement when the matched path's args match the
      impl head; 1 new fixture.
- [x] **`neg_cmp_op_on_partial_ord`** — REFUSED-FOREVER class, no corpus rule:
      `!(a > b)` → `a <= b` is NaN-UNSAFE on floats; an autofix would be a wrong-code
      generator. Recorded in the cargo-heal skill divergence table as the canonical
      never-heal example.
- [x] **`type_complexity`** — REFUSED class, no corpus rule: the fix invents a type name
      + declaration site — no bounded parse-only rewrite exists; clippy itself does not
      suggest one.
- [x] **`unusual_byte_groupings`** — CLOSED-REFUSED 2026-08-31: below the frequency bar
      by 20–360× (2 sites vs the 39-site smallest admitted candidate); style-only with a
      fiddly fix-space; the frequency bar protects retrieval floors. Re-open only on a
      future fired-histogram population.
- [x] **`unused_assignments` + rustc-side lints** — OUT OF SCOPE by construction (not
      clippy — the clippy_verify oracle never sees them).
- [x] **P-a (guard-landing residue protocol)** — CLOSED 2026-08-31:
      `scripts/consumer_residue_sweep.sh` + `scripts/swept_repos.txt` + the AGENTS.md
      "Guard-landing protocol" section (a guard is not landed until the residue is
      counted); the script was caught lying twice (stale-build silence + the JSON shape)
      and now prints a liveness witness.
- [x] **P-b (the all-features screen)** — CLOSED 2026-08-31: the sweep screen is
      `cargo clippy --workspace --all-features --all-targets --keep-going` (all 32
      residue sites sat behind non-default features; check is structurally blind to
      both guarded classes); landed consumer-side as riir-ai `ci_feature_guard.sh`
      Layer 2.

**Standing never-heal divergence table:** the cargo-heal skill owns it (comment-guarded
matches, array-literal defaults, named-arg renames, nested macro args, NaN-unsound
negated comparisons).
