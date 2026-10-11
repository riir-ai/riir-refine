# Plan 051 — Batch 90: fired-histogram pedantic tail (items_after_statements + manual_let_else FIXABLE; option_if_let_else REFUTED)

Status: **CLOSED 2026-08-31 — 2 of 3 leads landed, 1 refuted.**
`items_after_statements` (39 real instances) and `manual_let_else` (~40) landed
FIXABLE with probe-matched guard sets; `option_if_let_else` (38) REFUTED (nursery
pure style + fixpoint ping-pong with our own `single_match_else` FIXABLE
proposer + manual_unwrap_or coverage overlap). Both new lints are
applicability `Unspecified` under clippy 1.97 — cargo-fix does not cover them,
so the proposers are genuinely additive. GOAT: default lib 988/0 (debug +
release), latent_retrieval 1087/0, latent+fix_verify+clippy_verify 1132/0/8i,
clippy 0 both states, retrieval floors + cross-fire + clean-span all HELD,
score-bench #21 identical to #20 (96.2/1.9/100/100), E2E through the release
binary (both shapes healed, healed output pedantic-clean). En-route: a
PRE-EXISTING Batch-89 leftover caught by the latent-state full-suite run — the
switch_cost purity gate failed on `powi-vs-powf` × `suboptimal_flops` (a
TRUE-positive fire: the perf fixture carries `d.powf(2.0)`, real clippy's own
lint shape) — fixed with a documented (fixture, lint) allowlist, the frozen
fixture untouched.

**The measurable motivation:** the Plan 049 T0.1 fired histogram (`.distill/001_mining_queue_snapshot.md` lead) ranks the
narrow pedantic tail the sweep vocabulary does not cover: `items_after_statements` 39,
`option_if_let_else` 38, `manual_let_else` ~40 — ~117 findings per sweep pass the healer
stays silent on. All three are parse-bounded candidates; probe evidence per rule below.

## Probe verdicts (clippy 1.97, pedantic+nursery, JSON; probe crate `target/probe_b90/`)

- **`items_after_statements` — FIRES on fn/const/static/struct after a non-item stmt in
  the same block; silent when items come first. Applicability `Unspecified`** — real
  `cargo clippy --fix` does NOT apply it (unlike `missing_const_for_fn`), so a proposer
  is genuinely additive. Rewrite = hoist the items above the statements in the same
  block — provably semantics-preserving (Rust block items are order-independent for
  name resolution; the hoist stays inside the block, so scope is identical).
- **`manual_let_else` — FIRES on let-bound 2-arm matches whose fallback visibly
  diverges (`return`, `panic!`, `continue`, `break`, `Err(_) => return`). Silent
  (probe-confirmed, matches our guards): binding-use fallbacks (`Err(e) => {..}` —
  clippy is binding-aware; let-else else-blocks cannot see the arm's bindings),
  arm-1 bodies that are not the bound ident (`Some(v) => v + 1` — the let-else rewrite
  would be invalid), 3-arm-with-guard shapes, non-diverging fallbacks. Applicability
  `Unspecified` — genuinely additive.
- **`option_if_let_else` — FIRES on if-let-else with a non-early-returning else
  (nursery). REFUTED for the corpus** on four grounds:
  1. **Oscillation hazard with our own FIXABLE `single_match_else`**: its rewrite
     output `match x { Some(v) => v, None => 0 }` (statement position) re-enters
     `single_match_else`'s std-ctor territory (`None` = the Issue 029 T3 no-binding
     arm class) → rewrite back to if-let-else → ping-pong in the bounded fixpoint.
     The two rewrites pull OPPOSITE directions on the same statement-position
     2-arm Option/Result shapes.
  2. Nursery-group, pure style — zero perf/correctness content (the perf/sec
     prod-grade bar; `use_self` at least aids readability of type position,
     `suboptimal_flops` is a clippy-idiom fix).
  3. The let-match half of its territory (non-diverging fallbacks) is already
     covered better by `manual_unwrap_or`/`manual_unwrap_or_default` shapes our
     proposers handle (probe: both fired on the same span).
  4. Applicability `Unspecified` — no cargo-fix parity either way.
  No corpus entry (the corpus-lockstep gate forbids entries without proposers);
  refutation recorded in `VERIFY_LINTS` note + corpus pin + queue snapshot (the
  `missing_const_for_fn` kill precedent).

## Tasks

- [x] T0 Probe (temp crate, clippy 1.97 all-groups JSON): verdicts above.
- [x] T1 Dedup grep — all three absent from the clippy domain; adjacency analysis done
  (option_if_let_else vs single_match_else = inverse-direction; manual_let_else vs
  single_match_else = disjoint positions; clean spans pre-scanned = 0 `= match`
  bindings, 0 block-internal items).
- [x] T2 Corpus entries: `items_after_statements` (pedantic) + `manual_let_else`
  (pedantic) landed; `option_if_let_else` WITHHELD with the refutation note. Pin
  `corpus_size_is_43` → 45; header count comment updated.
- [x] T3 Proposers + fixtures:
  - `items_after_statements` (FIXABLE): block-level hoist — one atomic edit per block
    region `[first_non_item_stmt .. last_moved_item]`, replacement = items (original
    order) + `\n\n` + region-minus-item-spans. Guards: skip `Item::Macro` (token
    opacity), per-item comment guards (gap above each moved item + same-line trailing
    comment below = refuse that item's hoist; the `drops_interior_comment` class),
    guarded items stay in place (partial hoist = still an improvement; fixpoint
    idempotent). Overlapping nested-block edits resolve outer-first (Issue 032
    semantics).
  - `manual_let_else` (FIXABLE): `let v = match e { P => v, Q => DIVERGE };` →
    `let P = e else { DIVERGE };` — one atomic edit over the Local stmt (Stmt span
    includes the `;`). Guards: plain ident target (no mut/ty/attrs/no let-else
    already), exactly 2 arms, arm-1 pattern binds exactly 1 ident and its body src ==
    that ident (trim), arm-2 pattern binds 0 idents, arm-2 body visibly diverging
    (`return`/`break`/`continue`/panic-family macro: `panic!`/`unreachable!`/`todo!`/
    `unimplemented!`), no arm attrs, `drops_interior_comment` over the Local span with
    kept fragments = [arm1 pat, scrutinee, arm2 body].
- [x] T4 Standing gates: `fixable_lints_exclude_type_blind_rules` pin 34→36;
  `all_lints_fire_and_parse` fixture table +2 rows; dispatch arms + module docs table
  rows; FIXABLE_LINTS rows.
- [x] T5 GOAT — G1/G2: lib tests (default, debug+release); G3: clippy 0
  `--all-targets` in default + latent_retrieval,fix_verify,clippy_verify states;
  retrieval gates 13/0 incl. cross-fire (`clippy_fixture_labels_verify_via_proposers`
  — new fixtures must fire ONLY their own proposer) + `clean_spans_fire_no_lint` +
  floors; composite G1 (`uncontaminated_g1`) still green. **Measured: default lib
  988/0 debug + release; latent_retrieval 1087/0; latent+fix_verify+clippy_verify
  1132/0/8i; clippy 0 warnings in both states (all 5 findings in the new files fixed:
  2× unnested-or-patterns, 2× doc-backticks, 1× redundant-closure); E2E through the
  release binary: both shapes healed + the healed file compiles and is pedantic-clean.
  EN-ROUTE (pre-existing at HEAD, found by the latent-state full-suite run): the
  `switch_cost::cross_domain_constituents_carry_no_cross_domain_proposer` purity gate
  failed on `powi-vs-powf` × `suboptimal_flops` — a Batch-89 leftover: the perf
  fixture legitimately carries `d.powf(2.0)`, the exact shape real clippy's
  `suboptimal_flops` targets, so the fire is a TRUE positive, not a cross-domain
  leak; fixed with a documented (fixture, lint) allowlist in the gate, frozen fixture
  untouched.
- [x] T6 Score-bench run #21: floors held — heal 96.2% / created 1.9% / parse 100% /
  decline 100% (identical to run #20; persists unchanged: map_unwrap_or ×1 +
  useless_format ×1, the two documented boundaries).
- [x] T7 Docs: AGENTS.md batch record + `.docs/02_corpus/seed_corpus.md` table +
  README counts + queue snapshot (lead consumed, refutation recorded).
- [x] T8 Commit + push; clean up `target/probe_b90/`. — VERIFIED DONE 2026-08-31 (idle-loop queue-2 hygiene): commit `a657f4f` landed on origin/develop (batch record + GOAT counts in the message); `target/probe_b90/` already absent. Box was the only stale `- [ ]` in the plans tree.
