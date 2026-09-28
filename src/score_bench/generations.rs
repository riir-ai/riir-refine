//! Issue 072 T5/T6 — the M4 composite exactness ladder + the M5 generation-v2
//! fixture set (`.plans/092`; riir-train Research 450 §4.1 M4/M5).
//!
//! # M4 — amplification as an exactness schedule
//!
//! A k-composite fixture's gold IS the composition of the k atomic golds —
//! exact, no extra oracle. The atoms are NOT authored here: they come from
//! the clippy seed corpus itself (construction-first — the corpus entries'
//! `buggy_code` values are already probe-verified to fire their lint, and
//! `floor_corpus_integrity` compiles them). An atom qualifies when its
//! `buggy_code` parses standalone as a file (complete items only — composable
//! by wrapping in a module, which keeps item names collision-free and cannot
//! create cross-atom interference by construction).
//!
//! The ladder: k ∈ {1, 2, 3, 4}, deterministic stride-window selection over
//! the sorted lint set (no RNG — the corpus house rule), so the whole
//! generation is byte-stable and re-derivable from the corpus alone.
//!
//! # M5 — the generationed bench ratchet
//!
//! `generation_v2()` is the admitted fixture set: the specs, minus the ids
//! the freeze run EXCLUDED (interference composites whose real-clippy labels
//! disagreed with the union-of-atoms gold — construction proposes, the
//! oracle disposes), each carrying its oracle-verified frozen label. The
//! labels/exclusions land as data AFTER the freeze e2e
//! (`tests/generation_v2_freeze.rs`, `#[ignore]`d — spawns real clippy).
//!
//! The ratchet itself (saturation detector, the non-vacuity admission gate
//! and per-generation history series) lives in `report.rs` and the CLI; this
//! module only owns the generation.

use std::collections::{BTreeMap, BTreeSet};

use syn::spanned::Spanned;

use super::BenchFixture;

// ── the atom factory (M4, construction-first) ───────────────────────────

/// One atomic unit: a corpus span carrying exactly one vocabulary lint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    /// The vocabulary lint the atom fires (the atom's whole gold).
    pub lint: String,
    /// The corpus `buggy_code` (parses standalone as a file).
    pub code: String,
}

/// The atom table: per vocabulary lint, the FIRST corpus entry (min
/// `buggy_code` per lint — the sorted-(lint, code) first) whose `buggy_code`
/// parses standalone.
///
/// Atoms that fail freeze verification are removed from [`FROZEN_LABELS`]'s
/// coverage instead of here — the factory stays a pure function of the
/// corpus, and the freeze record documents any exclusion.
#[must_use]
pub fn atoms() -> Vec<Atom> {
    let vocab = super::vocabulary();
    let mut best: BTreeMap<String, String> = BTreeMap::new();
    for e in crate::build_seed_corpus().entries {
        if !vocab.contains(e.lint_name.as_str()) || syn::parse_file(&e.buggy_code).is_err() {
            continue;
        }
        match best.get(&e.lint_name) {
            Some(existing) if *existing <= e.buggy_code => {}
            _ => {
                best.insert(e.lint_name.clone(), e.buggy_code.clone());
            }
        }
    }
    best.into_iter()
        .map(|(lint, code)| Atom { lint, code })
        .collect()
}

// ── the deterministic k-ladder (M4) ─────────────────────────────────────

/// The k values the ladder spans (k=1 is the atom-solve-rate baseline).
pub const LADDER_KS: [usize; 4] = [1, 2, 3, 4];

/// Composite counts per k (the k=1 rung is every atom). Sized so a full
/// freeze stays a few minutes of real clippy: n + 16 + 12 + 8 fixtures.
#[must_use]
pub fn count_for_k(k: usize, n_atoms: usize) -> usize {
    match k {
        1 => n_atoms,
        2 => 16.min(n_atoms / 2),
        3 => 12.min(n_atoms / 3),
        4 => 8.min(n_atoms / 4),
        _ => 0,
    }
}

/// The lints of composite `i` at width `k` — a stride window over the sorted
/// atom set (wrapping; distinct because `k <= n_atoms`).
#[must_use]
pub fn lints_for_composite(k: usize, i: usize, lints: &[String]) -> Vec<String> {
    let n = lints.len();
    (0..k).map(|j| lints[(i * k + j) % n].clone()).collect()
}

fn mod_wrap(index: usize, code: &str) -> String {
    format!("mod gen_{index} {{\n{code}\n}}\n")
}

/// One rung of a generation ladder, fully determined by its atom table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenSpec {
    pub id: String,
    pub k: usize,
    /// The union-of-atoms gold (expected, NOT yet oracle-verified).
    pub expect_lints: Vec<String>,
    pub code: String,
}

/// The shared ladder builder behind [`specs`] (v2) and [`specs_v3`] (gen 3):
/// atoms (k=1) + stride-window composites (k=2..4) over a sorted atom-id
/// list, ids `{prefix}-k{k}-{ids joined +}`. Byte-stable — pinned by test.
///
/// # Panics
///
/// Only if the atom table loses an id between the list build and the lookup
/// (cannot happen — both derive from the same atom list; the `expect`
/// documents the invariant instead of silently skipping).
#[must_use]
fn specs_over_atoms(prefix: &str, atom_list: &[V3Atom]) -> Vec<GenSpec> {
    let ids: Vec<String> = atom_list.iter().map(|a| a.id.clone()).collect();
    let by_id: BTreeMap<&str, &V3Atom> = atom_list.iter().map(|a| (a.id.as_str(), a)).collect();
    let n = ids.len();
    let mut out = Vec::with_capacity(LADDER_KS.len());
    for &k in &LADDER_KS {
        let count = count_for_k(k, n);
        for i in 0..count {
            let picks = lints_for_composite(k, i, &ids);
            let id = format!("{prefix}-k{k}-{}", picks.join("+"));
            let mut code = String::new();
            for (index, atom_id) in picks.iter().enumerate() {
                let atom = by_id
                    .get(atom_id.as_str())
                    .expect("atom id drawn from the atom table");
                code.push_str(&mod_wrap(index, &atom.code));
            }
            let expect_lints: BTreeSet<String> = picks
                .iter()
                .flat_map(|a| by_id[a.as_str()].expect.iter().cloned())
                .collect();
            out.push(GenSpec {
                id,
                k,
                expect_lints: expect_lints.into_iter().collect(),
                code,
            });
        }
    }
    out.shrink_to_fit();
    out
}

/// The generation-2 spec ladder (corpus atoms keyed by lint — byte-identical
/// to the pre-generalization builder).
#[must_use]
pub fn specs() -> Vec<GenSpec> {
    let atom_list: Vec<V3Atom> = atoms()
        .into_iter()
        .map(|a| V3Atom {
            id: a.lint.clone(),
            expect: vec![a.lint],
            code: a.code,
        })
        .collect();
    specs_over_atoms("gen2", &atom_list)
}

// ── the freeze record (M5 — landed by the freeze e2e, 2026-09-06) ──────
//
// Freeze verdict (`tests/generation_v2_freeze.rs --ignored`, release,
// 32 s): 38 atoms (2 bad — `needless_match` also fires `manual_map`,
// `uninlined_format_args` also fires `useless_format`, so their k=1 specs
// and every composite containing them are excluded; 8 exclusions total,
// zero independent interference), 58 verified composites, union identity
// 58/58 against real clippy. Solve rates: 100.0% at EVERY k (36/36, 28/28,
// 30/30, 24/24) — the monotone-decay gate passes vacuously (flat). This is
// pre-registered branch (B): the k-ladder over the corpus's own shapes
// cannot be harder than the proposers trained on that corpus, so the M5
// non-vacuity admission gate refuses generation 2 (v2 100% > v1 96.2% =
// not harder). The record stays landed: the drift gate lives, and the
// difficulty-axis reopen (persist-shape stratification / real-repo
// interference spans) re-runs this exact freeze.
//
// P15 re-freeze (2026-09-14): adding `unnecessary_map_or` to the corpus +
// FIXABLE_LINTS grew the atom table 38→39 and reshuffled every stride
// window past it — the loudness gate caught the new unlabeled specs, both
// tables regenerated deterministically (labels = per-atom expects; windows
// containing the two bad atoms excluded). Re-verified against real clippy
// (v2 e2e 30.6 s: 67 verified / 0 newly excluded / 0 drift; v3 e2e 23.6 s:
// 73 verified / 0 newly excluded — the new atom fires exactly its own
// lint, no new interference). Exclusions 8 (v2, incl. the reshuffled
// `uninlined_format_args+unnecessary_lazy_evaluations+unnecessary_map_or`)
// + 6 (v3).
//
// Issue 120 re-freeze (2026-09-16): adding `doc_markdown` (the pedantic
// doc-lint family) grew the atom table 39→40 and reshuffled the windows
// past it again — same deterministic regeneration (68 v2 labels / 74 v3;
// exclusions unchanged at 8 + 6, the same bad-atom windows). The bench
// oracle runs `-W clippy::pedantic`, so the allow-by-default group is
// not a blind spot there.
//
// Batch 150 re-freeze (2026-09-17): the staged default-level + pedantic
// probe pack grew the atom table 40→44 (`print_with_newline`,
// `write_with_newline`, `unreadable_literal`, `legacy_numeric_constants`
// — all four verified to fire exactly their own lint under real clippy;
// no new interference atom). Windows reshuffled again; the two
// newly-excluded composites are the same needless_match/manual_map
// cascade at its new stride positions (10 v2 exclusions / 8 v3). v2
// e2e 29.6 s: 75 verified, solve 100.0% at every k (43/43, 30/30, 30/30,
// 28/28), M4 PASS. v3 e2e: 80 verified (43/47 k=1 — the same two bad
// atoms; 85.7-91.5% across k, reported not asserted for v3).
//
// Batch 151 re-freeze (2026-09-17, same day — the doc-line state-machine
// port landed on top): the atom table 44→46 (`doc_lazy_continuation`,
// `doc_overindented_list_items` — both verified to fire exactly their own
// lint; no new interference atom). Regenerated twice in one day because the
// 150 landing and the 151 landing were separate commits: 78 v2 labels /
// 82 v3; the needless_match cascade moved windows again (12 v2 exclusions
// / 10 v3 — count moved, cause did not). v2 e2e 86.4 s: 78 verified,
// solve 100.0% at every k (45/45, 30/30, 33/33, 28/28), M4 PASS. v3 e2e
// 25.2 s: 82 verified (45/49 k=1 — the same two bad atoms).
//
// Batch 155 re-freeze (2026-09-18, the frontier-lead mining pack — Plan
// 146): the atom table 47→50 (`let_and_return`, `redundant_slicing`,
// `map_flatten` — each verified to fire exactly its own lint on its own
// fixture). **let_and_return is a BROAD rule**: three PRE-EXISTING atoms'
// fixtures carry let-tail shapes (`deref_addrof`, `legacy_numeric_constants`,
// `unreadable_literal` — e.g. `let pages = 1234567; pages`), and real clippy
// fires let_and_return on them too — those three joined the bad-atom set
// (their k=1 forms now fire two vocabulary lints) and every window
// containing any of the five bad atoms excluded: 12 v2 exclusions / 14 v3
// (74 labeled / 76 labeled). v2 e2e 26.4 s: 74 verified, solve 100.0% at
// every k (45/45, 26/26, 30/30, 24/24), M4 PASS; v3 e2e 28.2 s: 76
// verified, zero drift on both. Recorded cost of admitting a broad rule:
// the ladder narrows (74 vs 78 v2) — truthful, not fought.
//
// Batch 156 re-freeze (2026-09-18, the frontier-lead mining pack II — Plan
// 147): the atom table 50→51 (`io_other_error`, FIXABLE — its corpus entry
// joins `vocabulary()`; the VERIFY-tier `wrong_self_convention` never
// enters the atom set, but both corpus entries reshuffle the stride
// windows past `io_other_error`'s sorted position). The new atom fires
// exactly its own lint (the frontier row's oracle verification — no new
// bad atom; the bad-atom set stays 5). Deterministic regeneration, both
// tables: 73 v2 labels / 14 v2 exclusions, 77 v3 labels / 14 v3 — the
// count movement vs Batch 155 (74/12 v2, 76/14 v3) is the window shuffle,
// same cause as ever (one more composite window lands on a bad-atom
// member). v2 e2e 29.6 s: 73 verified / 0 newly excluded / 0 drift, solve
// 100.0% at every k (46/46, 26/26, 27/27, 20/20), M4 PASS; v3 e2e: 77
// verified / 0 newly excluded, solve 92.0/84.6/85.2/80.0% across k
// (reported, not asserted — the pre-registered v3 posture), stratification
// gate PASS. The bad-atom set measured UNCHANGED at 5 — `io_other_error`
// fires exactly its own lint (no new interference).
//
// Batch 157 re-freeze (2026-09-18, the frontier-lead mining pack III — Plan
// 148): the atom table 51→53 (`redundant_closure_for_method_calls` +
// `iter_skip_next`, BOTH FIXABLE — the first pack where every lead joins
// the atom set). **The new rcmfc atom is BROAD as an ORACLE LINT**: clippy's
// own surface fires it on `unnecessary_sort_by`'s fixture
// (`v.sort_by(|a, b| a.cmp(b))` — a 2-param method-forwarding closure our
// parse-only proposer rightly REFUSES), so `unnecessary_sort_by` joined the
// bad-atom set (its k=1 form now fires two vocabulary lints) — bad set
// 5→6 (v2) / 6→7 (v3, +persist_array_default as ever). Recorded cost of
// admitting a broad pedantic lint: the ladder narrows (75 v2 labels / 20 v2
// exclusions vs 73/14; 76 v3 labels / 23 v3 exclusions vs 77/14) — truthful,
// not fought. v2 e2e 30.1 s: 75 verified / 6 newly excluded / 0 drift, solve
// 100.0% at every k (47/47, 26/26, 30/30, 20/20), M4 PASS; v3 e2e 27.3 s:
// 76 verified / 9 newly excluded, solve 94.0/92.3/95.8/100.0% across k
// (reported, not asserted — the pre-registered v3 posture), stratification
// gate PASS. `iter_skip_next` fires exactly its own lint on every window it
// lands in (no interference of its own).
//
// Batch 158 re-freeze (2026-09-18, the frontier-lead mining pack IV — Plan
// 149; record paragraph repaired at Batch 159 — the landing commit carries
// the numbers, the in-file paragraph was missed): the atom table 53→55
// (`to_string_in_format_args` + `manual_range_contains`, both FIXABLE).
// Tables regenerated: 77 v2 labels / 17 v2 exclusions, 78 v3 labels / 18
// v3; bad sets unchanged at 6/7 (neither new atom interferes).
//
// Batch 159 re-freeze (2026-09-18, the frontier-lead mining pack V — Plan
// 150): the atom table 55→57 (`single_char_add_str` + `manual_clamp`, both
// FIXABLE — every lead joins the atom set, the third consecutive all-pack).
// The new atoms fire exactly their own lints (the plan150 probe's full-
// oracle arms + the e2e's wrapped-form verification — no new interference;
// the bad-atom sets stay 6 v2 / 7 v3). Deterministic regeneration, both
// tables: 78 v2 labels / 15 v2 exclusions, 80 v3 labels / 17 v3 — the
// exclusion count DROP vs Batch 158 (17→15 v2, 18→17 v3) is the window
// shuffle: two composite windows that straddled bad-atom members at the
// 55-atom stride positions no longer do at 57 (a benign reshuffle — 0
// newly excluded, 0 drift on both e2es proves it). v2 e2e 35.2 s: 78
// verified / 0 newly excluded / 15 pre-excluded, solve 100.0% at every k
// (51/51, 26/26, 27/27, 20/20), M4 PASS; v3 e2e 40.9 s: 80 verified / 0
// newly excluded / 17 pre-excluded, solve 94.4/92.3/95.8/100.0% across k
// (reported, not asserted — the pre-registered v3 posture), stratification
// gate PASS.
//
// Batch 160 re-freeze (2026-09-18, the frontier-lead mining pack VI — Plan
// 151): the atom table 57→59 (`from_str_radix_10` + `assign_op_pattern`,
// both FIXABLE — the fourth consecutive all-pack). The new atoms fire
// exactly their own lints (the plan151 probe's full-oracle arms + the
// e2e's wrapped-form verification — no new interference; the bad-atom
// sets stay 6 v2 / 7 v3). Deterministic regeneration, both tables: 82 v2
// labels / 13 v2 exclusions, 83 v3 labels / 16 v3 exclusions — the count
// movement vs Batch 159 (78/15 v2, 80/17 v3) is the window shuffle again
// (0 newly excluded, 0 drift on both e2es proves it benign). v2 e2e
// 34.1 s: all label fences PASS (the assertions are the gate — M4 +
// oracle labels); v3 e2e 193.5 s: PASS (stratification posture
// unchanged). Regen-path note: the temporary example WRITES ITS OWN
// output file (`fs::write`) — this box's pty corrupts `cargo run > file`
// redirects (rows circularly rotated, section headers lost, exit 0 —
// measured at this batch; the splice script reads the file, so silent
// garbage would have spliced scrambled tables).
// Batch 161 re-freeze (2026-09-18, the frontier-lead mining pack VII —
// Plan 152): the atom table 59→61 (`unit_arg` + `string_lit_as_bytes`,
// both FIXABLE — the fifth consecutive all-pack). The new atoms fire
// exactly their own lints (the plan152 probe's full-oracle arms + the
// e2e's wrapped-form verification — no new interference; the bad-atom
// sets stay 6 v2 / 7 v3). Deterministic regeneration, both tables: 84 v2
// labels / 13 v2 exclusions, 85 v3 labels / 16 v3 exclusions — the count
// movement vs Batch 160 (82/13 v2, 83/16 v3) is PURELY ADDITIVE (the two
// new atoms' k1 labels; every composite window came out byte-identical —
// the sorted positions of `string_lit_as_bytes`/`unit_arg` shift nothing
// upstream, 0 reshuffle this round). v2 e2e + v3 e2e re-verified: all
// label fences PASS (0 drift — the assertions are the gate).
// Batch 162 re-freeze (2026-09-19, the frontier-lead mining pack VIII —
// Plan 153): the atom table 61→63 (`bytes_count_to_len` +
// `bool_comparison`, both FIXABLE — the sixth consecutive all-pack). The
// new atoms fire exactly their own lints (the plan152 probe's full-oracle
// arms + the regen run's real-oracle pass — no new interference; the
// bad-atom set stays 6). Deterministic regeneration, both tables: 87 v2
// labels / 12 v2 exclusions, 87 v3 labels / 16 v3 exclusions. The v2
// exclusion count moved 13→12: a STRIDE SHUFFLE, not a repair — the two
// new atoms' sorted positions shifted the composite windows, so 5 old
// exclusion ids dropped out and 4 new ids entered (the same interference
// composites at shifted k + atom companions; the v3 table shuffled 4↔4
// net-zero at 16). Both e2es re-verified after the splice: all label
// fences PASS (0 drift — the assertions are the gate).
// Batch 163 re-freeze (2026-09-19, the frontier-lead mining pack IX —
// Plan 154): the atom table 63→65 (`manual_string_new` +
// `needless_late_init`, both FIXABLE — the SEVENTH consecutive all-pack).
// The new atoms fire exactly their own lints on every window they land in
// (the plan154 probe arms + the regen run's real-oracle pass — no new
// interference; the bad-atom set stays 6). Deterministic regeneration,
// both tables: 90 v2 labels / 11 v2 exclusions, 89 v3 labels / 16 v3
// exclusions, 89 v3 labels / 16 v3
// exclusions — the v3 table was PURELY ADDITIVE (the two new k1 labels,
// every composite window byte-identical); the v2 exclusion count moved
// 12→11, the benign window shuffle again (0 newly excluded atoms, 0 drift
// on both e2es proves it).
// Batch 164 re-freeze (2026-09-19, the frontier-lead mining pack X — the
// coverage-report re-arm, Plan 155): the atom table 65→67
// (`match_like_matches_macro` + `excessive_precision`, both FIXABLE — the
// eighth consecutive all-pack). **TWO bad-atom joins, a first for one
// batch**: `excessive_precision` (its corpus fixture
// `2.141592653589793238462643f32`, 24 significant digits, co-fires the
// in-vocabulary `unreadable_literal` — probe-predicted AND freeze-measured)
// and `redundant_pattern_matching` (its corpus fixture is the IF-LET-ELSE
// bool form `if let Ok(_) = r { true } else { false }` — upstream mlmm's
// OTHER firing arm, the Batch-155 `let_and_return` broad-rule class; found
// by the freeze e2e's pass 1 after the first splice labeled it clean, so
// the bad-atom set was extended and the tables regenerated — construction
// proposes, the oracle disposes). Bad sets 6→8 (v2) / 7→9 (v3). Deterministic
// regeneration, both tables: 86 v2 labels / 17 v2 exclusions, 86 v3 labels
// / 21 v3 exclusions (103 + 107 specs). The mlmm atom itself is CLEAN — it
// fires exactly its own lint, its k1 is labeled, and its one composite
// window (`mlmm+needless_bool+needless_bool_assign`, k3) verified union-gold
// exact; only ep/rpm and the windows containing THEM were excluded. The stride shuffle dropped 5 dissolved old exclusion ids, all
// bad-explained (the iter_skip_next+legacy_numeric_constants family moved
// windows). v2 e2e 35.2 s: 86 verified / 0 newly excluded / 17
// pre-excluded, solve 100.0% at every k (59/59, 26/26, 27/27, 20/20), M4
// PASS; v3 e2e: 86 verified / 0 newly excluded / 21 pre-excluded, solve
// 95.2/91.7/95.8/100.0% across k (reported, not asserted — the
// pre-registered v3 posture), stratification gate PASS. Recorded cost of
// admitting a broad rule pair: the ladder narrows (86 vs 90 v2 labels) and
// TWO atoms lost their k=1 rungs — truthful, not fought.
// Batch 167 regen (2026-09-21, Plan 158 — the coverage-report re-arm pack
// XI): +2 atoms (`unnecessary_cast`, `clone_on_copy` — both CLEAN: each
// fires exactly its own lint under the plan158 full-oracle probe, and the
// freeze e2e newly-excluded count is 0 — no new interference, unlike the
// Batch-164 broad-rule pair which lost two k=1 rungs). The stride windows
// reshuffled past the sorted positions; bad sets UNCHANGED (8 v2 / 9 v3).
// Deterministic regeneration, both tables: 88 v2 labels / 17 v2 exclusions,
// 88 v3 labels / 21 v3 exclusions. v2 e2e 36.5 s: 88 verified / 0 newly
// excluded / 17 pre-excluded, solve 100.0% at every k (61/61, 26/26, 27/27,
// 20/20), M4 PASS; v3 e2e: 88 verified / 0 newly excluded / 21
// pre-excluded, stratification gate PASS. Post-splice fixture rescalar
// (u32 → u64, both batch fixtures): the retrieval A/B rank dump measured
// the u32 spellings crowding the deref_addrof eval span out of top-1
// (CLIPPY_TOP1/3 floors would have red); u64 restores the pre-batch
// rankings byte-identically (floors HELD, no re-pin) — e2es re-run green
// 19.2 s / 19.0 s.
//
// Batch 168 regen (2026-09-21, Plan 159 — the coverage-report re-arm pack
// XII): +2 atoms (`empty_line_after_doc_comments` — CLEAN, fires exactly
// its own lint in every window; `new_without_default` — a BAD ATOM BY
// PREMISE: the lint requires effective EXPORTATION, and the harness's
// private `mod gen_N` wrap structurally prevents it — the oracle returns
// [use_self] (nursery, the fixture's `W { x: 1 }` constructor body) and
// NEVER the target lint, in any fixture spelling; the needless_match
// bad-atom precedent, k1 + every composite containing it excluded). The
// stride shuffle re-picked the windows around the two new sorted
// positions: 6 v2 / 6 v3 old exclusion ids went stale (windows no longer
// produced — the iter_skip_next+legacy_numeric_constants family moved),
// and the NEW windows re-exposed ep's known `unreadable_literal` co-fire
// inside the eld+ep family (4 v2 rows) plus one genuinely new v3
// interference (gen3-k3-iter_nth_zero+iter_skip_next+legacy_numeric_constants
// — a `let_and_return` co-fire one of its members carries, a window that
// never existed before this shuffle). Tables regenerated: 92 v2 labels /
// 15 v2 exclusions (107 specs at 71 atoms), 91 v3 labels / 21 v3
// exclusions. Regen-path note: the temporary example WRITES ITS OWN
// output files (`fs::write`) — this box's pty corrupts `cargo run > file`
// redirects (the Batch-160 measured lesson); it also COLLECTS + prints
// fresh/stale exclusions instead of panicking, because a stride shuffle
// needs 2 runs to converge (exclude, then re-derive).
// Batch 169 regen (2026-09-21, Plan 160 — the coverage-report pack XIII,
// the Batch-168 staged alternates): +2 atoms (`match_ref_pats` — CLEAN,
// fires exactly its own lint in every window it lands in; `into_iter_on_ref`
// — CLEAN after a FIXTURE repair caught by the regen itself: the first
// corpus spelling returned `Option<&u64>` borrowing a fn param — E0515,
// a non-compiling atom by construction; the `.next().copied()` spelling
// verifies exactly its own lint). Bad-atom set UNCHANGED at 9 v2 / 10 v3
// (the 8 pre-B168 atoms + new_without_default; v3 adds persist_array_default
// as ever). The stride shuffle re-landed the iter_skip_next+
// legacy_numeric_constants family at 3 new v2 windows + 2 new v3 windows,
// retired 1 stale v3 id, and the v3 CASCADE (composites of unverified
// atoms — the e2e's silent exclusion) moved one window row-side to
// exclusion-side. Tables: 91 v2 labels / 18 v2 exclusions (109 specs at
// 73 atoms), 91 v3 labels / 22 v3 exclusions. Regen-path notes honored
// from Plan 159: the temp example WRITES ITS OWN output files; it also
// mirrors the v3 cascade branch (run 2's tables missed it — the e2e
// reported 1 newly-excluded with no printed reason; the cascade is the
// silent branch); 3 runs to converge (fixture repair between runs 1-2,
// cascade mirror between 2-3). e2es: v2 91 verified / 0 newly / 18
// pre-excluded, M4 PASS (31.3s); v3 91 / 0 / 22, stratification PASS
// (23.3s).
// Batch 175 regen (2026-09-24, Plan 169 — the coverage-report pack XV, the
// Plan-163 overturns): +2 atoms (`cast_abs_to_unsigned` — CLEAN, fires
// exactly its own lint in every window both gens; `len_without_is_empty` —
// a BAD ATOM BY PREMISE, the new_without_default class exactly: the lint's
// "public len" premise is EFFECTIVE EXPORTATION, and the harness's private
// `mod gen_N` wrap structurally prevents it — oracle {} on the k1 spec, in
// every spelling; k1 + every composite containing it excluded in BOTH
// gens). Note the tables 91→94 v2 / 91→94 v3 landed here include the
// B171/B174 corpus deltas (their regens landed tables without record
// paragraphs — the prose gap ends here). Tables: 94 v2 labels / 28 v2
// exclusions (122 specs at 77 atoms), 94 v3 labels / 32 v3 exclusions
// (128 specs). e2es: v2 94 verified / 10 newly excluded / 18 pre-excluded
// (the 10 = exactly the lwie k1 + its 9 stride windows), M4 PASS; v3 94
// verified / 10 newly excluded / 22 pre-excluded (same class),
// stratification PASS.
// Issue 089 orphan prune (2026-09-26, the follow-up sweep's catch): the
// Batch 175 splice had left 9 dissolved pre-shift exclusion rows in EACH
// lane's EXCLUDED table (18 total — windows the 77-atom stride no longer
// produces), so the gen-3 lane refused `orphan_excluded` at HEAD. Pruned
// data-only (admission-invariant: orphans match zero produced specs);
// tables read 94 v2 labels / 19 v2 exclusions, 94 v3 labels / 23 v3
// exclusions after. The gen-3 lane was re-verified at the prune (94
// verified / 0 newly excluded, stratification PASS — the issue-089
// landing); **the v2 lane re-verified 2026-09-26 at HEAD `c2c028f6` (the
// owed second half, run in a detached worktree)**: 94 verified / 0 newly
// excluded / 19 pre-excluded, solve 100.0% at every k (67/67, 26/26,
// 27/27, 20/20), M4 PASS, branch (A) — bad-atom set unchanged at 9 (the
// known exportation/co-fire classes; no new interference). Both lanes
// green post-prune; the record stands.
pub static FROZEN_LABELS: &[(&str, &[&str])] = &[
    ("gen2-k1-assign_op_pattern", &["assign_op_pattern"]),  // k=1
    ("gen2-k1-bool_comparison", &["bool_comparison"]),  // k=1
    ("gen2-k1-byte_char_slices", &["byte_char_slices"]),  // k=1
    ("gen2-k1-bytes_count_to_len", &["bytes_count_to_len"]),  // k=1
    ("gen2-k1-cast_abs_to_unsigned", &["cast_abs_to_unsigned"]),  // k=1
    ("gen2-k1-clone_on_copy", &["clone_on_copy"]),  // k=1
    ("gen2-k1-collapsible_else_if", &["collapsible_else_if"]),  // k=1
    ("gen2-k1-collapsible_if", &["collapsible_if"]),  // k=1
    ("gen2-k1-doc_lazy_continuation", &["doc_lazy_continuation"]),  // k=1
    ("gen2-k1-doc_markdown", &["doc_markdown"]),  // k=1
    ("gen2-k1-doc_overindented_list_items", &["doc_overindented_list_items"]),  // k=1
    ("gen2-k1-double_parens", &["double_parens"]),  // k=1
    ("gen2-k1-empty_line_after_doc_comments", &["empty_line_after_doc_comments"]),  // k=1
    ("gen2-k1-filter_next", &["filter_next"]),  // k=1
    ("gen2-k1-from_str_radix_10", &["from_str_radix_10"]),  // k=1
    ("gen2-k1-identity_op", &["identity_op"]),  // k=1
    ("gen2-k1-into_iter_on_ref", &["into_iter_on_ref"]),  // k=1
    ("gen2-k1-io_other_error", &["io_other_error"]),  // k=1
    ("gen2-k1-items_after_statements", &["items_after_statements"]),  // k=1
    ("gen2-k1-iter_next_loop", &["iter_next_loop"]),  // k=1
    ("gen2-k1-iter_nth_zero", &["iter_nth_zero"]),  // k=1
    ("gen2-k1-iter_skip_next", &["iter_skip_next"]),  // k=1
    ("gen2-k1-let_and_return", &["let_and_return"]),  // k=1
    ("gen2-k1-manual_clamp", &["manual_clamp"]),  // k=1
    ("gen2-k1-manual_let_else", &["manual_let_else"]),  // k=1
    ("gen2-k1-manual_map", &["manual_map"]),  // k=1
    ("gen2-k1-manual_range_contains", &["manual_range_contains"]),  // k=1
    ("gen2-k1-manual_range_patterns", &["manual_range_patterns"]),  // k=1
    ("gen2-k1-manual_slice_fill", &["manual_slice_fill"]),  // k=1
    ("gen2-k1-manual_string_new", &["manual_string_new"]),  // k=1
    ("gen2-k1-manual_unwrap_or", &["manual_unwrap_or"]),  // k=1
    ("gen2-k1-map_clone", &["map_clone"]),  // k=1
    ("gen2-k1-map_flatten", &["map_flatten"]),  // k=1
    ("gen2-k1-map_unwrap_or", &["map_unwrap_or"]),  // k=1
    ("gen2-k1-match_bool", &["match_bool"]),  // k=1
    ("gen2-k1-match_like_matches_macro", &["match_like_matches_macro"]),  // k=1
    ("gen2-k1-match_ref_pats", &["match_ref_pats"]),  // k=1
    ("gen2-k1-needless_bool", &["needless_bool"]),  // k=1
    ("gen2-k1-needless_bool_assign", &["needless_bool_assign"]),  // k=1
    ("gen2-k1-needless_borrowed_reference", &["needless_borrowed_reference"]),  // k=1
    ("gen2-k1-needless_late_init", &["needless_late_init"]),  // k=1
    ("gen2-k1-needless_lifetimes", &["needless_lifetimes"]),  // k=1
    ("gen2-k1-needless_range_loop", &["needless_range_loop"]),  // k=1
    ("gen2-k1-needless_return", &["needless_return"]),  // k=1
    ("gen2-k1-print_with_newline", &["print_with_newline"]),  // k=1
    ("gen2-k1-ptr_offset_with_cast", &["ptr_offset_with_cast"]),  // k=1
    ("gen2-k1-question_mark", &["question_mark"]),  // k=1
    ("gen2-k1-range_plus_one", &["range_plus_one"]),  // k=1
    ("gen2-k1-redundant_closure_for_method_calls", &["redundant_closure_for_method_calls"]),  // k=1
    ("gen2-k1-redundant_field_names", &["redundant_field_names"]),  // k=1
    ("gen2-k1-redundant_slicing", &["redundant_slicing"]),  // k=1
    ("gen2-k1-redundant_static_lifetimes", &["redundant_static_lifetimes"]),  // k=1
    ("gen2-k1-semicolon_if_nothing_returned", &["semicolon_if_nothing_returned"]),  // k=1
    ("gen2-k1-single_char_add_str", &["single_char_add_str"]),  // k=1
    ("gen2-k1-single_match", &["single_match"]),  // k=1
    ("gen2-k1-single_match_else", &["single_match_else"]),  // k=1
    ("gen2-k1-string_lit_as_bytes", &["string_lit_as_bytes"]),  // k=1
    ("gen2-k1-tabs_in_doc_comments", &["tabs_in_doc_comments"]),  // k=1
    ("gen2-k1-to_string_in_format_args", &["to_string_in_format_args"]),  // k=1
    ("gen2-k1-unit_arg", &["unit_arg"]),  // k=1
    ("gen2-k1-unnecessary_cast", &["unnecessary_cast"]),  // k=1
    ("gen2-k1-unnecessary_lazy_evaluations", &["unnecessary_lazy_evaluations"]),  // k=1
    ("gen2-k1-unnecessary_map_or", &["unnecessary_map_or"]),  // k=1
    ("gen2-k1-unnecessary_unwrap", &["unnecessary_unwrap"]),  // k=1
    ("gen2-k1-unused_unit", &["unused_unit"]),  // k=1
    ("gen2-k1-use_self", &["use_self"]),  // k=1
    ("gen2-k1-useless_format", &["useless_format"]),  // k=1
    ("gen2-k1-wildcard_in_or_patterns", &["wildcard_in_or_patterns"]),  // k=1
    ("gen2-k1-write_with_newline", &["write_with_newline"]),  // k=1
    ("gen2-k2-assign_op_pattern+bool_comparison", &["assign_op_pattern", "bool_comparison"]),  // k=2
    ("gen2-k2-byte_char_slices+bytes_count_to_len", &["byte_char_slices", "bytes_count_to_len"]),  // k=2
    ("gen2-k2-cast_abs_to_unsigned+clone_on_copy", &["cast_abs_to_unsigned", "clone_on_copy"]),  // k=2
    ("gen2-k2-collapsible_else_if+collapsible_if", &["collapsible_else_if", "collapsible_if"]),  // k=2
    ("gen2-k2-doc_markdown+doc_overindented_list_items", &["doc_markdown", "doc_overindented_list_items"]),  // k=2
    ("gen2-k2-double_parens+empty_line_after_doc_comments", &["double_parens", "empty_line_after_doc_comments"]),  // k=2
    ("gen2-k2-from_str_radix_10+identity_op", &["from_str_radix_10", "identity_op"]),  // k=2
    ("gen2-k2-into_iter_on_ref+io_other_error", &["into_iter_on_ref", "io_other_error"]),  // k=2
    ("gen2-k2-items_after_statements+iter_next_loop", &["items_after_statements", "iter_next_loop"]),  // k=2
    ("gen2-k2-iter_nth_zero+iter_skip_next", &["iter_nth_zero", "iter_skip_next"]),  // k=2
    ("gen2-k2-let_and_return+manual_clamp", &["let_and_return", "manual_clamp"]),  // k=2
    ("gen2-k2-manual_let_else+manual_map", &["manual_let_else", "manual_map"]),  // k=2
    ("gen2-k2-manual_range_contains+manual_range_patterns", &["manual_range_contains", "manual_range_patterns"]),  // k=2
    ("gen2-k3-assign_op_pattern+bool_comparison+byte_char_slices", &["assign_op_pattern", "bool_comparison", "byte_char_slices"]),  // k=3
    ("gen2-k3-bytes_count_to_len+cast_abs_to_unsigned+clone_on_copy", &["bytes_count_to_len", "cast_abs_to_unsigned", "clone_on_copy"]),  // k=3
    ("gen2-k3-doc_lazy_continuation+doc_markdown+doc_overindented_list_items", &["doc_lazy_continuation", "doc_markdown", "doc_overindented_list_items"]),  // k=3
    ("gen2-k3-filter_next+from_str_radix_10+identity_op", &["filter_next", "from_str_radix_10", "identity_op"]),  // k=3
    ("gen2-k3-into_iter_on_ref+io_other_error+items_after_statements", &["into_iter_on_ref", "io_other_error", "items_after_statements"]),  // k=3
    ("gen2-k3-iter_next_loop+iter_nth_zero+iter_skip_next", &["iter_next_loop", "iter_nth_zero", "iter_skip_next"]),  // k=3
    ("gen2-k3-manual_clamp+manual_let_else+manual_map", &["manual_clamp", "manual_let_else", "manual_map"]),  // k=3
    ("gen2-k3-manual_range_contains+manual_range_patterns+manual_slice_fill", &["manual_range_contains", "manual_range_patterns", "manual_slice_fill"]),  // k=3
    ("gen2-k3-manual_string_new+manual_unwrap_or+map_clone", &["manual_string_new", "manual_unwrap_or", "map_clone"]),  // k=3
    ("gen2-k4-assign_op_pattern+bool_comparison+byte_char_slices+bytes_count_to_len", &["assign_op_pattern", "bool_comparison", "byte_char_slices", "bytes_count_to_len"]),  // k=4
    ("gen2-k4-cast_abs_to_unsigned+clone_on_copy+collapsible_else_if+collapsible_if", &["cast_abs_to_unsigned", "clone_on_copy", "collapsible_else_if", "collapsible_if"]),  // k=4
    ("gen2-k4-from_str_radix_10+identity_op+into_iter_on_ref+io_other_error", &["from_str_radix_10", "identity_op", "into_iter_on_ref", "io_other_error"]),  // k=4
    ("gen2-k4-items_after_statements+iter_next_loop+iter_nth_zero+iter_skip_next", &["items_after_statements", "iter_next_loop", "iter_nth_zero", "iter_skip_next"]),  // k=4
    ("gen2-k4-manual_let_else+manual_map+manual_range_contains+manual_range_patterns", &["manual_let_else", "manual_map", "manual_range_contains", "manual_range_patterns"]),  // k=4
];

/// Spec ids the freeze run EXCLUDED (real-clippy labels ≠ union gold): the
/// 2 bad atoms plus the 6 composites whose membership in them cascades.
/// Zero independent interference was measured.
pub static EXCLUDED_IDS: &[&str] = &[
    "gen2-k1-deref_addrof",
    "gen2-k1-excessive_precision",
    "gen2-k1-legacy_numeric_constants",
    "gen2-k1-len_without_is_empty",
    "gen2-k1-needless_match",
    "gen2-k1-new_without_default",
    "gen2-k1-redundant_pattern_matching",
    "gen2-k1-uninlined_format_args",
    "gen2-k1-unnecessary_sort_by",
    "gen2-k1-unreadable_literal",
    "gen2-k2-deref_addrof+doc_lazy_continuation",
    "gen2-k2-excessive_precision+filter_next",
    "gen2-k2-legacy_numeric_constants+len_without_is_empty",
    "gen2-k3-collapsible_else_if+collapsible_if+deref_addrof",
    "gen2-k3-double_parens+empty_line_after_doc_comments+excessive_precision",
    "gen2-k3-legacy_numeric_constants+len_without_is_empty+let_and_return",
    "gen2-k4-deref_addrof+doc_lazy_continuation+doc_markdown+doc_overindented_list_items",
    "gen2-k4-double_parens+empty_line_after_doc_comments+excessive_precision+filter_next",
    "gen2-k4-legacy_numeric_constants+len_without_is_empty+let_and_return+manual_clamp",
];

/// Freeze readiness for one generation: Ok(admitted count) when every spec
/// is labeled or excluded, Err(reason) while the generation's freeze e2e has
/// not landed its record.
fn freeze_status_for(
    all: &[GenSpec],
    gen: u8,
    test_name: &str,
    labels: &[(&'static str, &'static [&'static str])],
    excluded: &[&'static str],
) -> Result<usize, String> {
    let excluded: BTreeSet<&str> = excluded.iter().copied().collect();
    let labeled: BTreeSet<&str> = labels.iter().map(|(id, _)| *id).collect();
    let missing: Vec<&str> = all
        .iter()
        .filter(|s| !excluded.contains(s.id.as_str()) && !labeled.contains(s.id.as_str()))
        .map(|s| s.id.as_str())
        .collect();
    let orphan_excluded: Vec<&str> = excluded
        .iter()
        .filter(|id| !all.iter().any(|s| &s.id == *id))
        .copied()
        .collect();
    if !orphan_excluded.is_empty() {
        return Err(format!(
            "EXCLUDED_IDS names ids the spec builder no longer produces (the corpus moved): {orphan_excluded:?} — re-run the freeze e2e"
        ));
    }
    if missing.is_empty() {
        Ok(labeled.len())
    } else {
        Err(format!(
            "generation v{gen} is not frozen: {} spec(s) lack oracle labels — run \
             `cargo test --features score_bench --test {test_name} -- --ignored \
             --test-threads=1` and land the freeze record",
            missing.len()
        ))
    }
}

/// The admitted fixture set for one generation (the M5 ratchet's bench
/// input). Leaks the fixture strings once per call by design —
/// `BenchFixture` is `&'static`-shaped and this runs once per
/// `--generation <N>` invocation.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check the generation's
/// `*_freeze_status` first and print its error instead.
fn generation_fixtures(
    all: Vec<GenSpec>,
    gen: u8,
    labels: &[(&'static str, &'static [&'static str])],
    excluded: &[&'static str],
) -> Vec<BenchFixture> {
    let excluded: BTreeSet<&str> = excluded.iter().copied().collect();
    let label_map: BTreeMap<&str, &[&str]> = labels.iter().map(|(id, l)| (*id, *l)).collect();
    all.into_iter()
        .filter(|s| !excluded.contains(s.id.as_str()))
        .map(|s| {
            let expect: &[&str] = label_map.get(s.id.as_str()).copied().unwrap_or_else(|| {
                panic!("spec {} frozen but unlabeled — freeze_status hole", s.id)
            });
            BenchFixture {
                id: Box::leak(s.id.clone().into_boxed_str()),
                provenance: Box::leak(
                    format!(
                        "construction:gen{gen}:k{}:{}",
                        s.k,
                        s.expect_lints.join("+")
                    )
                    .into_boxed_str(),
                ),
                code: Box::leak(s.code.into_boxed_str()),
                expect_lints: Box::leak(expect.to_vec().into_boxed_slice()),
                clean: false,
            }
        })
        .collect()
}

/// Freeze readiness: Ok(admitted count) when every spec is labeled or
/// excluded, Err(reason) while the freeze e2e has not landed its record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels (or which
/// exclusion rows no longer match the corpus-derived specs).
pub fn v2_freeze_status() -> Result<usize, String> {
    freeze_status_for(
        &specs(),
        2,
        "generation_v2_freeze",
        FROZEN_LABELS,
        EXCLUDED_IDS,
    )
}

/// The admitted generation-v2 fixture set (the M5 ratchet's bench input).
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v2_freeze_status`] first
/// and print its error instead.
#[must_use]
pub fn generation_v2() -> Vec<BenchFixture> {
    generation_fixtures(specs(), 2, FROZEN_LABELS, EXCLUDED_IDS)
}

/// The ladder k of a generation fixture id (`gen2-k3-…` → 3,
/// `gen3-k2-…` → 2); `None` for non-generation ids.
#[must_use]
pub fn k_of(id: &str) -> Option<usize> {
    let rest = id.strip_prefix("gen")?;
    let (_gen, rest) = rest.split_once('-')?;
    let rest = rest.strip_prefix('k')?;
    let k: String = rest.chars().take_while(char::is_ascii_digit).collect();
    k.parse().ok()
}

// ── generation 3 — persist-shape stratification (Bench 069 reopen #1) ──
//
// The corpus-atom ladder (generation 2) measured 100% at every k: shapes
// the corpus teaches are shapes the proposers solve. Generation 3
// stratifies the atom table with AUTHORED persist-shape atoms — real-code
// shapes where a vocabulary lint FIRES and the heal lane DELIBERATELY stays
// silent, each a documented parse-only divergence class with the record
// that measured the silence. These give the bench measurement headroom the
// saturated corpus ladder lacks: when a proposer extension ever lands
// (macro-arg descent past a nested macro, the named-arg rename rewrite, a
// further array-literal relaxation), the generation's rate MOVES and the
// ratchet sees it.
//
// Pre-registered at authoring (`.benchmarks/070`), BEFORE the freeze run:
// the v2 M4 k-decay gate does NOT carry to gen 3 — its difficulty axis is
// per-atom strata, not k-composition, and the prefix-window geometry makes
// strict monotonicity a sampling artifact. The binding checks are the
// stratification gate (every persist atom resolves 0) and the M5 admission
// gate (aggregate rate strictly below v1's latest).
//
// The stratification gate already caught two stale atoms on the FIRST
// freeze run (the loop working as designed):
// - a named-arg-with-rename shape (`println!("{iters}", iters = ITERS)`)
//   RESOLVED — the P3 rename rewrite closed the class after the bench_337
//   record was written; the atom was replaced.
// - a second-level macro-nesting shape now fires clippy 1.98's NEW
//   `format_in_format_args` instead of `uninlined_format_args` — the class
//   moved off-vocabulary (labels move between toolchains, the oracle's
//   own caveat); the atom was dropped.

/// One authored persist-shape atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistAtom {
    /// Stable id (namespaced `persist_*` — disjoint from corpus lint ids).
    pub id: &'static str,
    /// The divergence class + the record that measured the silence.
    pub note: &'static str,
    /// The shape (parses standalone as a file; compiles in the stub crate).
    pub code: &'static str,
    /// The vocabulary lints the freeze oracle must confirm (the atom's gold).
    pub expect: &'static [&'static str],
}

pub static PERSIST_ATOMS: &[PersistAtom] = &[
    PersistAtom {
        id: "persist_array_default",
        note: "array-literal map_or guard: real clippy's own MachineApplicable rewrite E0308s in a plain-let position (map_or's U unifies from the array literal first), so the proposer refuses — the slice_pinned relaxation (P2, 2026-08-28) fires only when a `&[T]` annotation forces the type, which this plain let deliberately lacks (measured at forward step.rs:196)",
        code: r"fn persist_array_default(o: Option<&str>) -> Vec<u8> {
    let bytes = o.map(|s| s.as_bytes()).unwrap_or(&[]);
    bytes.to_vec()
}",
        expect: &["map_unwrap_or"],
    },
    PersistAtom {
        id: "persist_comment_arm",
        note: "comment-guarded match rewrite: real clippy fires single_match_else on the corpus's own Some/block-fallback shape, but the interior comment would be displaced by the if-let rewrite and the drops_interior_comment guard refuses — the healer is SAFER than clippy there (guards from Issue 027/029; measured at transposition.rs:67)",
        code: r"fn score2(g: u8) -> u8 {
    g * 2
}

fn persist_comment_arm(kind: Option<u8>) -> u8 {
    match kind {
        // The fallback block is the documented shape.
        Some(k) => score2(k),
        _ => {
            let fallback = 2;
            score2(fallback)
        }
    }
}",
        expect: &["single_match_else"],
    },
    PersistAtom {
        id: "persist_comment_hoist",
        note: "comment-guarded item hoist: real clippy fires items_after_statements on the fn-after-statement shape, but hoisting the item above the statement would displace the interior comment and the leading-gap guard refuses (the Batch-90 comment guards; corpus note cites the class)",
        code: r"fn persist_comment_hoist() -> i32 {
    let x = 1;
    // The helper stays below this note.
    fn h(v: i32) -> i32 {
        v * 2
    }
    h(x)
}",
        expect: &["items_after_statements"],
    },
    PersistAtom {
        id: "persist_useless_format",
        note: "the type-boundary terminal: real clippy fires useless_format on the String-typed named-capture form, but the terminal fix (clone vs to_string) needs the arg's TYPE — the single-positional proposer form deliberately does not fire, and firing would trade the persist for a created implicit_clone (measured on the composite-range_loop_plus_format_args fixture, run #3 triage)",
        code: r#"fn persist_useless_format() {
    let elem: String = "payload".to_string();
    let s: String = format!("{elem}");
    println!("{s}");
}"#,
        expect: &["useless_format"],
    },
];

/// One generation-3 atom: a corpus-derived solvable atom (id = its lint) or
/// an authored persist-shape atom (id = `persist_*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V3Atom {
    pub id: String,
    /// The atom's gold (union member of every composite containing it).
    pub expect: Vec<String>,
    pub code: String,
}

/// The generation-3 atom table: the five persist atoms FIRST (the stride
/// windows cover the list head at k≥2, so declaring the stratification up
/// front puts a persist shape in every composite rung), then the corpus
/// atoms in their sorted-lint order. Order is load-bearing and pinned.
#[must_use]
pub fn v3_atoms() -> Vec<V3Atom> {
    let mut out: Vec<V3Atom> = Vec::with_capacity(PERSIST_ATOMS.len() + 40);
    out.extend(PERSIST_ATOMS.iter().map(|p| V3Atom {
        id: p.id.to_string(),
        expect: p.expect.iter().map(ToString::to_string).collect(),
        code: p.code.to_string(),
    }));
    out.extend(atoms().into_iter().map(|a| V3Atom {
        id: a.lint.clone(),
        expect: vec![a.lint],
        code: a.code,
    }));
    out
}

/// The generation-3 spec ladder: persist-stratified atoms (k=1) + the same
/// stride-window composites (k=2..4). Byte-stable — pinned by test.
#[must_use]
pub fn specs_v3() -> Vec<GenSpec> {
    let atom_list = v3_atoms();
    specs_over_atoms("gen3", &atom_list)
}

/// Spec ids the generation-3 freeze run EXCLUDED (real-clippy labels ≠ the
/// union-of-atom gold) — landed by the freeze e2e.
// Landed by tests/generation_v3_freeze.rs --ignored (2026-09-06): the 2 known
// corpus double-fire atoms (needless_match, uninlined_format_args) + their 4
// cascade composites. Zero independent interference measured.
// Batch 156 regen (2026-09-18, Plan 147): the bad-atom set is 5 (the Batch
// 155 let_and_return broad-rule additions ride in the same tables); the
// windows reshuffled past `io_other_error`'s sorted position — 14 rows.
pub static EXCLUDED_IDS_V3: &[&str] = &[
    "gen3-k1-deref_addrof",
    "gen3-k1-excessive_precision",
    "gen3-k1-legacy_numeric_constants",
    "gen3-k1-len_without_is_empty",
    "gen3-k1-needless_match",
    "gen3-k1-new_without_default",
    "gen3-k1-persist_array_default",
    "gen3-k1-redundant_pattern_matching",
    "gen3-k1-uninlined_format_args",
    "gen3-k1-unnecessary_sort_by",
    "gen3-k1-unreadable_literal",
    "gen3-k2-deref_addrof+doc_lazy_continuation",
    "gen3-k2-excessive_precision+filter_next",
    "gen3-k2-legacy_numeric_constants+len_without_is_empty",
    "gen3-k2-persist_array_default+persist_comment_arm",
    "gen3-k3-deref_addrof+doc_lazy_continuation+doc_markdown",
    "gen3-k3-excessive_precision+filter_next+from_str_radix_10",
    "gen3-k3-iter_skip_next+legacy_numeric_constants+len_without_is_empty",
    "gen3-k3-persist_array_default+persist_comment_arm+persist_comment_hoist",
    "gen3-k4-deref_addrof+doc_lazy_continuation+doc_markdown+doc_overindented_list_items",
    "gen3-k4-double_parens+empty_line_after_doc_comments+excessive_precision+filter_next",
    "gen3-k4-legacy_numeric_constants+len_without_is_empty+let_and_return+manual_clamp",
    "gen3-k4-persist_array_default+persist_comment_arm+persist_comment_hoist+persist_useless_format",
];

/// The generation-3 frozen labels (spec id → vocabulary lints real clippy
/// fired at freeze time) — landed by the freeze e2e.
// Landed by tests/generation_v3_freeze.rs --ignored (2026-09-06): 72 verified
// specs (38 corpus atoms + 4 persist atoms, and all k>=2 composites),
// oracle labels == union-of-atom gold 72/72 against real clippy.
pub static FROZEN_LABELS_V3: &[(&str, &[&str])] = &[
    ("gen3-k1-assign_op_pattern", &["assign_op_pattern"]),  // k=1
    ("gen3-k1-bool_comparison", &["bool_comparison"]),  // k=1
    ("gen3-k1-byte_char_slices", &["byte_char_slices"]),  // k=1
    ("gen3-k1-bytes_count_to_len", &["bytes_count_to_len"]),  // k=1
    ("gen3-k1-cast_abs_to_unsigned", &["cast_abs_to_unsigned"]),  // k=1
    ("gen3-k1-clone_on_copy", &["clone_on_copy"]),  // k=1
    ("gen3-k1-collapsible_else_if", &["collapsible_else_if"]),  // k=1
    ("gen3-k1-collapsible_if", &["collapsible_if"]),  // k=1
    ("gen3-k1-doc_lazy_continuation", &["doc_lazy_continuation"]),  // k=1
    ("gen3-k1-doc_markdown", &["doc_markdown"]),  // k=1
    ("gen3-k1-doc_overindented_list_items", &["doc_overindented_list_items"]),  // k=1
    ("gen3-k1-double_parens", &["double_parens"]),  // k=1
    ("gen3-k1-empty_line_after_doc_comments", &["empty_line_after_doc_comments"]),  // k=1
    ("gen3-k1-filter_next", &["filter_next"]),  // k=1
    ("gen3-k1-from_str_radix_10", &["from_str_radix_10"]),  // k=1
    ("gen3-k1-identity_op", &["identity_op"]),  // k=1
    ("gen3-k1-into_iter_on_ref", &["into_iter_on_ref"]),  // k=1
    ("gen3-k1-io_other_error", &["io_other_error"]),  // k=1
    ("gen3-k1-items_after_statements", &["items_after_statements"]),  // k=1
    ("gen3-k1-iter_next_loop", &["iter_next_loop"]),  // k=1
    ("gen3-k1-iter_nth_zero", &["iter_nth_zero"]),  // k=1
    ("gen3-k1-iter_skip_next", &["iter_skip_next"]),  // k=1
    ("gen3-k1-let_and_return", &["let_and_return"]),  // k=1
    ("gen3-k1-manual_clamp", &["manual_clamp"]),  // k=1
    ("gen3-k1-manual_let_else", &["manual_let_else"]),  // k=1
    ("gen3-k1-manual_map", &["manual_map"]),  // k=1
    ("gen3-k1-manual_range_contains", &["manual_range_contains"]),  // k=1
    ("gen3-k1-manual_range_patterns", &["manual_range_patterns"]),  // k=1
    ("gen3-k1-manual_slice_fill", &["manual_slice_fill"]),  // k=1
    ("gen3-k1-manual_string_new", &["manual_string_new"]),  // k=1
    ("gen3-k1-manual_unwrap_or", &["manual_unwrap_or"]),  // k=1
    ("gen3-k1-map_clone", &["map_clone"]),  // k=1
    ("gen3-k1-map_flatten", &["map_flatten"]),  // k=1
    ("gen3-k1-map_unwrap_or", &["map_unwrap_or"]),  // k=1
    ("gen3-k1-match_bool", &["match_bool"]),  // k=1
    ("gen3-k1-match_like_matches_macro", &["match_like_matches_macro"]),  // k=1
    ("gen3-k1-match_ref_pats", &["match_ref_pats"]),  // k=1
    ("gen3-k1-needless_bool", &["needless_bool"]),  // k=1
    ("gen3-k1-needless_bool_assign", &["needless_bool_assign"]),  // k=1
    ("gen3-k1-needless_borrowed_reference", &["needless_borrowed_reference"]),  // k=1
    ("gen3-k1-needless_late_init", &["needless_late_init"]),  // k=1
    ("gen3-k1-needless_lifetimes", &["needless_lifetimes"]),  // k=1
    ("gen3-k1-needless_range_loop", &["needless_range_loop"]),  // k=1
    ("gen3-k1-needless_return", &["needless_return"]),  // k=1
    ("gen3-k1-persist_comment_arm", &["single_match_else"]),  // k=1
    ("gen3-k1-persist_comment_hoist", &["items_after_statements"]),  // k=1
    ("gen3-k1-persist_useless_format", &["useless_format"]),  // k=1
    ("gen3-k1-print_with_newline", &["print_with_newline"]),  // k=1
    ("gen3-k1-ptr_offset_with_cast", &["ptr_offset_with_cast"]),  // k=1
    ("gen3-k1-question_mark", &["question_mark"]),  // k=1
    ("gen3-k1-range_plus_one", &["range_plus_one"]),  // k=1
    ("gen3-k1-redundant_closure_for_method_calls", &["redundant_closure_for_method_calls"]),  // k=1
    ("gen3-k1-redundant_field_names", &["redundant_field_names"]),  // k=1
    ("gen3-k1-redundant_slicing", &["redundant_slicing"]),  // k=1
    ("gen3-k1-redundant_static_lifetimes", &["redundant_static_lifetimes"]),  // k=1
    ("gen3-k1-semicolon_if_nothing_returned", &["semicolon_if_nothing_returned"]),  // k=1
    ("gen3-k1-single_char_add_str", &["single_char_add_str"]),  // k=1
    ("gen3-k1-single_match", &["single_match"]),  // k=1
    ("gen3-k1-single_match_else", &["single_match_else"]),  // k=1
    ("gen3-k1-string_lit_as_bytes", &["string_lit_as_bytes"]),  // k=1
    ("gen3-k1-tabs_in_doc_comments", &["tabs_in_doc_comments"]),  // k=1
    ("gen3-k1-to_string_in_format_args", &["to_string_in_format_args"]),  // k=1
    ("gen3-k1-unit_arg", &["unit_arg"]),  // k=1
    ("gen3-k1-unnecessary_cast", &["unnecessary_cast"]),  // k=1
    ("gen3-k1-unnecessary_lazy_evaluations", &["unnecessary_lazy_evaluations"]),  // k=1
    ("gen3-k1-unnecessary_map_or", &["unnecessary_map_or"]),  // k=1
    ("gen3-k1-unnecessary_unwrap", &["unnecessary_unwrap"]),  // k=1
    ("gen3-k1-unused_unit", &["unused_unit"]),  // k=1
    ("gen3-k1-use_self", &["use_self"]),  // k=1
    ("gen3-k1-useless_format", &["useless_format"]),  // k=1
    ("gen3-k1-wildcard_in_or_patterns", &["wildcard_in_or_patterns"]),  // k=1
    ("gen3-k1-write_with_newline", &["write_with_newline"]),  // k=1
    ("gen3-k2-assign_op_pattern+bool_comparison", &["assign_op_pattern", "bool_comparison"]),  // k=2
    ("gen3-k2-byte_char_slices+bytes_count_to_len", &["byte_char_slices", "bytes_count_to_len"]),  // k=2
    ("gen3-k2-cast_abs_to_unsigned+clone_on_copy", &["cast_abs_to_unsigned", "clone_on_copy"]),  // k=2
    ("gen3-k2-collapsible_else_if+collapsible_if", &["collapsible_else_if", "collapsible_if"]),  // k=2
    ("gen3-k2-doc_markdown+doc_overindented_list_items", &["doc_markdown", "doc_overindented_list_items"]),  // k=2
    ("gen3-k2-double_parens+empty_line_after_doc_comments", &["double_parens", "empty_line_after_doc_comments"]),  // k=2
    ("gen3-k2-from_str_radix_10+identity_op", &["from_str_radix_10", "identity_op"]),  // k=2
    ("gen3-k2-into_iter_on_ref+io_other_error", &["into_iter_on_ref", "io_other_error"]),  // k=2
    ("gen3-k2-items_after_statements+iter_next_loop", &["items_after_statements", "iter_next_loop"]),  // k=2
    ("gen3-k2-iter_nth_zero+iter_skip_next", &["iter_nth_zero", "iter_skip_next"]),  // k=2
    ("gen3-k2-let_and_return+manual_clamp", &["let_and_return", "manual_clamp"]),  // k=2
    ("gen3-k2-persist_comment_hoist+persist_useless_format", &["items_after_statements", "useless_format"]),  // k=2
    ("gen3-k3-byte_char_slices+bytes_count_to_len+cast_abs_to_unsigned", &["byte_char_slices", "bytes_count_to_len", "cast_abs_to_unsigned"]),  // k=3
    ("gen3-k3-clone_on_copy+collapsible_else_if+collapsible_if", &["clone_on_copy", "collapsible_else_if", "collapsible_if"]),  // k=3
    ("gen3-k3-doc_overindented_list_items+double_parens+empty_line_after_doc_comments", &["doc_overindented_list_items", "double_parens", "empty_line_after_doc_comments"]),  // k=3
    ("gen3-k3-identity_op+into_iter_on_ref+io_other_error", &["identity_op", "into_iter_on_ref", "io_other_error"]),  // k=3
    ("gen3-k3-items_after_statements+iter_next_loop+iter_nth_zero", &["items_after_statements", "iter_next_loop", "iter_nth_zero"]),  // k=3
    ("gen3-k3-let_and_return+manual_clamp+manual_let_else", &["let_and_return", "manual_clamp", "manual_let_else"]),  // k=3
    ("gen3-k3-manual_map+manual_range_contains+manual_range_patterns", &["manual_map", "manual_range_contains", "manual_range_patterns"]),  // k=3
    ("gen3-k3-persist_useless_format+assign_op_pattern+bool_comparison", &["assign_op_pattern", "bool_comparison", "useless_format"]),  // k=3
    ("gen3-k4-assign_op_pattern+bool_comparison+byte_char_slices+bytes_count_to_len", &["assign_op_pattern", "bool_comparison", "byte_char_slices", "bytes_count_to_len"]),  // k=4
    ("gen3-k4-cast_abs_to_unsigned+clone_on_copy+collapsible_else_if+collapsible_if", &["cast_abs_to_unsigned", "clone_on_copy", "collapsible_else_if", "collapsible_if"]),  // k=4
    ("gen3-k4-from_str_radix_10+identity_op+into_iter_on_ref+io_other_error", &["from_str_radix_10", "identity_op", "into_iter_on_ref", "io_other_error"]),  // k=4
    ("gen3-k4-items_after_statements+iter_next_loop+iter_nth_zero+iter_skip_next", &["items_after_statements", "iter_next_loop", "iter_nth_zero", "iter_skip_next"]),  // k=4
];

/// Freeze readiness for generation 3 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
pub fn v3_freeze_status() -> Result<usize, String> {
    freeze_status_for(
        &specs_v3(),
        3,
        "generation_v3_freeze",
        FROZEN_LABELS_V3,
        EXCLUDED_IDS_V3,
    )
}

/// The admitted generation-v3 fixture set.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v3_freeze_status`] first.
#[must_use]
pub fn generation_v3() -> Vec<BenchFixture> {
    generation_fixtures(specs_v3(), 3, FROZEN_LABELS_V3, EXCLUDED_IDS_V3)
}

// ── Generation 5: the rust_perf vocabulary ladder (Plan 094, Bench 070
//    §What remains candidate #1b) ────────────────────────────────────────
// No cfg gating beyond the crate's own features: `score_bench` implies
// `rust_perf` (Cargo.toml), and `rust_perf` is default-on — the atom
// factory and the detectors are always available in a score_bench build.
// The whole freeze is CARGO-LESS (pure textual + syn): rust_perf rules have
// no external detector (clippy does not ship them — that absence is why the
// domain exists), so the oracle here is a bench-owned set of per-rule
// MECHANICAL SHAPE DETECTORS (Plan 094 §2, pre-registered), validated at
// freeze by buggy-vs-fixed discrimination and never consulted for
// addressable-atom solve behavior (that is the bench's measurement).

/// One authored persist-shape atom for generation 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistAtomV5 {
    /// Stable id (namespaced `persist_*` — disjoint from corpus rule ids).
    pub id: &'static str,
    /// The divergence class + the record that measured the silence.
    pub note: &'static str,
    /// The shape (parses standalone as a file).
    pub code: &'static str,
    /// The rule whose persist predicate the freeze must confirm.
    pub expect: &'static [&'static str],
}

pub static PERSIST_ATOMS_V5: &[PersistAtomV5] = &[
    PersistAtomV5 {
        id: "persist_ref_array_capacity",
        note: "reference-array-literal iterable guard (vec-with-capacity): `for x in &[..]` — the P2 temp-binding rewrite binds the ARRAY and would change the iteration mode (by-value vs by-reference), so the guard keeps it silent (test-pinned array_literal_reference_iterable_skipped). Bench 070's 'array-literal vec-with-capacity' class survives as this REFERENCE form; the direct `[..]` form was SOLVED by the 2026-08-28 temp-binding extension",
        code: "fn persist_ref_array_capacity() -> Vec<u32> {\n    let mut v = Vec::new();\n    for x in &[\n        1u32, 2, 3,\n        4, 5, 6,\n    ] {\n        v.push(*x);\n    }\n    v\n}",
        expect: &["vec-with-capacity"],
    },
    PersistAtomV5 {
        id: "persist_coldstart_fxhashmap",
        note: "fxhashmap Gate-0 cold start: the proposer refuses any file without an existing rustc_hash import — a span-preserving splice can rewrite uses but cannot add the rustc-hash dependency to Cargo.toml, so first-file adoption stays manual (the proposer's honest-scope limit; the game-sdk E0432 smoke that forced Gate 0). Persists in PRODUCTION, not only in the bench. The bare-path iterable keeps hashmap-with-capacity's bare-path guard between the atom and a capacity cross-heal, so the persist surface stays single-class",
        code: "fn persist_coldstart_fxhashmap(pairs: &[(u32, u32)]) -> std::collections::HashMap<u32, u32> {\n    use std::collections::HashMap;\n    let mut m = HashMap::new();\n    for (k, v) in pairs {\n        m.insert(*k, *v);\n    }\n    m\n}",
        expect: &["fxhashmap"],
    },
    PersistAtomV5 {
        id: "persist_caller_gate_str",
        note: "static-str caller gate: the signature-changing rewrite (String -> &'static str) is refused when the fn ident occurs more than once in the file — a live call site would break (the action_hoare.rs E0308 lesson encoded as count_ident_occurrences > 1 -> refuse). The single-definition corpus shape heals; the called shape persists",
        code: "fn persist_caller_gate() -> String {\n    let label = stage_label();\n    format!(\"[{label}]\")\n}\n\nfn stage_label() -> String {\n    \"ready\".to_string()\n}",
        expect: &["static-str"],
    },
];

/// The per-rule persist predicates (the generation-5 oracle — Plan 094 §2).
/// TRUE means the rule's problem shape is STILL PRESENT in the region text.
/// Pure textual, deterministic, tool-agnostic: any edit that removes the
/// shape credits, whoever authored it. Never calls a proposer.
///
/// `shrink-to-fit-after-build` is COMPOUND for a measured reason: its own
/// fix is additive (keeps `Vec::new()`, inserts `shrink_to_fit()`), and the
/// mutual-exclusion contract with `vec-with-capacity` (the exact-capacity
/// complement) means a `with_capacity(n)` fix ALSO resolves the
/// over-allocation hazard (exact capacity => no excess). A token-addition
/// detector would falsely mark vwc-healed regions unresolved.
#[must_use]
pub fn v5_persists(rule: &str, region: &str) -> bool {
    match rule {
        "vec-with-capacity" => region.contains("Vec::new()"),
        "entry-vs-contains-insert" => region.contains("contains_key("),
        "static-str" => region.contains("\".to_string("),
        "bool-to-bitpack" => region.contains(".filter(|&&"),
        "fxhashmap" => region.contains("std::collections::HashMap"),
        "hashset-insert-bool-return" => {
            region.contains(".contains(&") && region.contains(".insert(")
        }
        "collect-append-to-extend" => region.contains(".append(&mut"),
        "vec-macro-zero-fill" => region.contains(".resize("),
        "distance-sq-no-sqrt" => region.contains(".sqrt()"),
        "hashmap-with-capacity" => region.contains("HashMap::new()"),
        "shrink-to-fit-after-build" => {
            region.contains("Vec::new()")
                && region.contains(".push(")
                && !region.contains("shrink_to_fit")
        }
        _ => false,
    }
}

/// Rules whose persist predicate is a plain REMOVAL shape (used by the
/// created axis: an inserted `shrink_to_fit` is a fix, not a created
/// problem, so the compound rule is excluded from created counting).
#[must_use]
pub fn v5_is_removal_rule(rule: &str) -> bool {
    !matches!(rule, "shrink-to-fit-after-build")
}

/// The generation-5 atom table: persist atoms FIRST (the gen-3 convention —
/// stride windows cover the list head, so a persist shape sits in every
/// k>=2 rung), then the corpus atoms in sorted rule order. The corpus
/// factory takes, per FIXABLE rule, the perf-seed entry whose `buggy_code`
/// parses standalone (the corpus carries exactly one entry per rule id).
#[must_use]
pub fn v5_atoms() -> Vec<V3Atom> {
    let vocab: BTreeSet<String> = crate::domains::rust_perf::proposers::FIXABLE_RULES
        .iter()
        .map(ToString::to_string)
        .collect();
    let mut best: BTreeMap<String, String> = BTreeMap::new();
    for e in crate::build_perf_seed_corpus().entries {
        if !vocab.contains(e.rule_id.as_str()) || syn::parse_file(&e.buggy_code).is_err() {
            continue;
        }
        match best.get(&e.rule_id) {
            Some(existing) if *existing <= e.buggy_code => {}
            _ => {
                best.insert(e.rule_id.clone(), e.buggy_code.clone());
            }
        }
    }
    let mut out: Vec<V3Atom> = Vec::with_capacity(PERSIST_ATOMS_V5.len() + best.len());
    out.extend(PERSIST_ATOMS_V5.iter().map(|p| V3Atom {
        id: p.id.to_string(),
        expect: p.expect.iter().map(ToString::to_string).collect(),
        code: p.code.to_string(),
    }));
    out.extend(best.into_iter().map(|(rule, code)| V3Atom {
        expect: vec![rule.clone()], // the rule is its own gold
        id: rule,
        code,
    }));
    out
}

/// The generation-5 spec ladder: persist-stratified atoms (k=1) + the same
/// stride-window composites (k=2..4). Byte-stable — pinned by test.
#[must_use]
pub fn specs_v5() -> Vec<GenSpec> {
    let atom_list = v5_atoms();
    specs_over_atoms("gen5", &atom_list)
}

/// Member i's rule for a generation-5 spec id, in the composite's
/// `mod gen_{i}` order (the measurement runner attributes region verdicts
/// through this ordering).
#[must_use]
pub fn v5_member_rules(spec_id: &str) -> Option<Vec<String>> {
    let rest = spec_id.strip_prefix("gen5-k")?;
    let (k, names) = rest.split_once('-')?;
    let k: usize = k.parse().ok()?;
    let names: Vec<&str> = names.split('+').collect();
    if names.len() != k {
        return None;
    }
    let atoms = v5_atoms();
    let by_name: BTreeMap<&str, &V3Atom> = atoms.iter().map(|a| (a.id.as_str(), a)).collect();
    names
        .iter()
        .map(|n| by_name.get(n).map(|a| a.expect[0].clone()))
        .collect()
}

/// The territory of a generation-5 atom (pre-registered — Plan 094 §3).
/// `persist` = the authored divergence classes + the corpus fxhashmap atom
/// (bench-context dep gate); `addressable` = the rule's own fixture-pinned
/// shape. Reported side by side in the bench, never blended into the
/// headline decision.
#[must_use]
pub fn v5_territory(atom_id: &str) -> &'static str {
    if atom_id.starts_with("persist_") || atom_id == "fxhashmap" {
        "persist"
    } else {
        "addressable"
    }
}

/// The text of `mod gen_{index}`'s items in `source` (syn-span based — the
/// proposers' own `span().byte_range()` pattern; robust to braces inside
/// string literals, which a brace matcher would miscount).
///
/// # Errors
///
/// `None` when `source` does not parse or carries no `gen_{index}` module.
#[must_use]
pub fn v5_module_region(source: &str, index: usize) -> Option<String> {
    let file = syn::parse_file(source).ok()?;
    let name = format!("gen_{index}");
    for item in &file.items {
        if let syn::Item::Mod(m) = item {
            if m.ident == name {
                let (_, items) = m.content.as_ref()?;
                let first = items.first()?;
                let last = items.last()?;
                let start = first.span().byte_range().start;
                let end = last.span().byte_range().end;
                return Some(source[start..end].to_string());
            }
        }
    }
    None
}

/// Spec ids the generation-5 freeze run EXCLUDED — landed by the freeze e2e
/// (a member's shape destroyed or shadowed by composition). Landed empty:
/// regions are isolated modules and every marker lives inside its own
/// member's code — zero composition interference measured
/// (`tests/generation_v5_freeze.rs` `--ignored`, 2026-09-06).
pub static EXCLUDED_IDS_V5: &[&str] = &[];

/// The generation-5 frozen labels (spec id → the DISTINCT member rule set
/// verified present by the freeze oracle at freeze time) — landed by the
/// freeze e2e.
// Landed by tests/generation_v5_freeze.rs --ignored (2026-09-06): 28/28
// verified (11 corpus atoms + 3 persist atoms + all k>=2 composites),
// zero exclusions, zero bad atoms; the marker-discrimination fence (buggy
// TRUE / fixed FALSE) and the persist stratification gate (the production
// heal leaves every persist shape in place) both PASS. Freeze is CARGO-LESS:
// 0.05 s (gens 2-4 spawn a real oracle per spec).
pub static FROZEN_LABELS_V5: &[(&str, &[&str])] = &[
    ("gen5-k1-bool-to-bitpack", &["bool-to-bitpack"]),  // k=1
    ("gen5-k1-collect-append-to-extend", &["collect-append-to-extend"]),  // k=1
    ("gen5-k1-distance-sq-no-sqrt", &["distance-sq-no-sqrt"]),  // k=1
    ("gen5-k1-entry-vs-contains-insert", &["entry-vs-contains-insert"]),  // k=1
    ("gen5-k1-fxhashmap", &["fxhashmap"]),  // k=1
    ("gen5-k1-hashmap-with-capacity", &["hashmap-with-capacity"]),  // k=1
    ("gen5-k1-hashset-insert-bool-return", &["hashset-insert-bool-return"]),  // k=1
    ("gen5-k1-persist_caller_gate_str", &["static-str"]),  // k=1
    ("gen5-k1-persist_coldstart_fxhashmap", &["fxhashmap"]),  // k=1
    ("gen5-k1-persist_ref_array_capacity", &["vec-with-capacity"]),  // k=1
    ("gen5-k1-shrink-to-fit-after-build", &["shrink-to-fit-after-build"]),  // k=1
    ("gen5-k1-static-str", &["static-str"]),  // k=1
    ("gen5-k1-vec-macro-zero-fill", &["vec-macro-zero-fill"]),  // k=1
    ("gen5-k1-vec-with-capacity", &["vec-with-capacity"]),  // k=1
    ("gen5-k2-collect-append-to-extend+distance-sq-no-sqrt", &["collect-append-to-extend", "distance-sq-no-sqrt"]),  // k=2
    ("gen5-k2-entry-vs-contains-insert+fxhashmap", &["entry-vs-contains-insert", "fxhashmap"]),  // k=2
    ("gen5-k2-hashmap-with-capacity+hashset-insert-bool-return", &["hashmap-with-capacity", "hashset-insert-bool-return"]),  // k=2
    ("gen5-k2-persist_caller_gate_str+bool-to-bitpack", &["bool-to-bitpack", "static-str"]),  // k=2
    ("gen5-k2-persist_ref_array_capacity+persist_coldstart_fxhashmap", &["fxhashmap", "vec-with-capacity"]),  // k=2
    ("gen5-k2-shrink-to-fit-after-build+static-str", &["shrink-to-fit-after-build", "static-str"]),  // k=2
    ("gen5-k2-vec-macro-zero-fill+vec-with-capacity", &["vec-macro-zero-fill", "vec-with-capacity"]),  // k=2
    ("gen5-k3-bool-to-bitpack+collect-append-to-extend+distance-sq-no-sqrt", &["bool-to-bitpack", "collect-append-to-extend", "distance-sq-no-sqrt"]),  // k=3
    ("gen5-k3-entry-vs-contains-insert+fxhashmap+hashmap-with-capacity", &["entry-vs-contains-insert", "fxhashmap", "hashmap-with-capacity"]),  // k=3
    ("gen5-k3-hashset-insert-bool-return+shrink-to-fit-after-build+static-str", &["hashset-insert-bool-return", "shrink-to-fit-after-build", "static-str"]),  // k=3
    ("gen5-k3-persist_ref_array_capacity+persist_coldstart_fxhashmap+persist_caller_gate_str", &["fxhashmap", "static-str", "vec-with-capacity"]),  // k=3
    ("gen5-k4-collect-append-to-extend+distance-sq-no-sqrt+entry-vs-contains-insert+fxhashmap", &["collect-append-to-extend", "distance-sq-no-sqrt", "entry-vs-contains-insert", "fxhashmap"]),  // k=4
    ("gen5-k4-hashmap-with-capacity+hashset-insert-bool-return+shrink-to-fit-after-build+static-str", &["hashmap-with-capacity", "hashset-insert-bool-return", "shrink-to-fit-after-build", "static-str"]),  // k=4
    ("gen5-k4-persist_ref_array_capacity+persist_coldstart_fxhashmap+persist_caller_gate_str+bool-to-bitpack", &["bool-to-bitpack", "fxhashmap", "static-str", "vec-with-capacity"]),  // k=4
];

/// Freeze readiness for generation 5 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
pub fn v5_freeze_status() -> Result<usize, String> {
    freeze_status_for(
        &specs_v5(),
        5,
        "generation_v5_freeze",
        FROZEN_LABELS_V5,
        EXCLUDED_IDS_V5,
    )
}

// ── Generation 4: the rustc-errors-side ladder (Plan 093, Bench 069 reopen
//    #4) ──────────────────────────────────────────────────────────────────
// cfg(`rustc_errors`): `score_bench` does not imply the domain — the atom
// factory reads `corpus_gen` through it, and the whole section compiles to
// nothing without the feature (a `--features score_bench` build stays
// green; the CLI arm errors cleanly instead).

/// Deterministic atom-slice target out of `generate_corpus()` (n ≥ 200 →
/// stride 5 → ~40 atoms, family-proportional because the corpus is
/// family-ordered). Sized so the ladder matches the clippy generations
/// (40 + 16 + 12 + 8 = 76 specs) while each spec costs a real `cargo check`.
#[cfg(feature = "rustc_errors")]
pub const GEN4_ATOM_TARGET: usize = 40;

/// The generation-4 atom table: a stride slice of the generated compile-error
/// corpus (`broken` half — the driver's repair subject), round-robin
/// interleaved by expected code so the stride windows compose MIXED-code
/// composites (raw corpus order is family-clustered; name order would be
/// too). Deterministic — no RNG, no environment reads.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn v4_atoms() -> Vec<V3Atom> {
    use crate::domains::rustc_errors::corpus_gen::GeneratedCase;

    let corpus = crate::domains::rustc_errors::corpus_gen::generate_corpus();
    let stride = (corpus.len() / GEN4_ATOM_TARGET).max(1);
    let sliced: Vec<&GeneratedCase> = corpus
        .iter()
        .step_by(stride)
        .take(GEN4_ATOM_TARGET)
        .collect();

    // Round-robin interleave by expected code: one atom per code group per
    // pass, groups in sorted-code order, within-group order preserved —
    // adjacent atoms differ in code until groups exhaust at the tail.
    let mut groups: BTreeMap<&str, Vec<&GeneratedCase>> = BTreeMap::new();
    for case in &sliced {
        groups.entry(case.expect_code).or_default().push(case);
    }
    let keys: Vec<&str> = groups.keys().copied().collect();
    let mut cursors: BTreeMap<&str, usize> = keys.iter().map(|k| (*k, 0)).collect();
    let mut order: Vec<&GeneratedCase> = Vec::with_capacity(sliced.len());
    while order.len() < sliced.len() {
        let mut progressed = false;
        for key in &keys {
            let cur = cursors[key];
            if cur < groups[key].len() {
                order.push(groups[key][cur]);
                cursors.insert(key, cur + 1);
                progressed = true;
            }
        }
        if !progressed {
            break; // unreachable while order.len() < sliced.len()
        }
    }

    order
        .into_iter()
        .map(|c| V3Atom {
            id: c.name.clone(),
            expect: vec![c.expect_code.to_string()],
            code: c.broken.clone(),
        })
        .collect()
}

/// The generation-4 spec ladder: corpus-strided atoms (k=1) + the same
/// stride-window composites (k=2..4) over the code-interleaved atom list.
/// Byte-stable — pinned by test.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn specs_v4() -> Vec<GenSpec> {
    let atom_list = v4_atoms();
    specs_over_atoms("gen4", &atom_list)
}

/// Member i's expected error code for a generation-4 spec id, in the
/// composite's `mod gen_{i}` order (with duplicates — the per-member
/// multiset, not the distinct union in [`GenSpec::expect_lints`]). The
/// measurement runner attributes final-check diagnostics to member regions
/// through this ordering.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn v4_member_codes(spec_id: &str) -> Option<Vec<String>> {
    let rest = spec_id.strip_prefix("gen4-k")?;
    let (k, names) = rest.split_once('-')?;
    let k: usize = k.parse().ok()?;
    let names: Vec<&str> = names.split('+').collect();
    if names.len() != k {
        return None;
    }
    let atoms = v4_atoms();
    let by_name: BTreeMap<&str, &V3Atom> = atoms.iter().map(|a| (a.id.as_str(), a)).collect();
    names
        .iter()
        .map(|n| by_name.get(n).map(|a| a.expect[0].clone()))
        .collect()
}

/// Spec ids the generation-4 freeze run EXCLUDED (composed codes ≠ the union
/// of member codes on a real `cargo check` — the composition-interference
/// class) — landed by the freeze e2e. Empty pre-freeze (the dual-mode gate
/// then asserts the un-frozen Err).
pub static EXCLUDED_IDS_V4: &[&str] = &[];

/// The generation-4 frozen labels (spec id → the DISTINCT member error-code
/// set real rustc produced on the composed broken file at freeze time) —
/// landed by the freeze e2e. Empty pre-freeze.
pub static FROZEN_LABELS_V4: &[(&str, &[&str])] = &[
    ("gen4-k1-gen_e0373_str_00_hello", &["E0373"]),  // k=1
    ("gen4-k1-gen_e0373_str_05_box", &["E0373"]),  // k=1
    ("gen4-k1-gen_e0373_sum_00_u32", &["E0373"]),  // k=1
    ("gen4-k1-gen_e0373_sum_05_i64", &["E0373"]),  // k=1
    ("gen4-k1-gen_e0373_sum_10_i32", &["E0373"]),  // k=1
    ("gen4-k1-gen_e0382_clone_00_u8", &["E0382"]),  // k=1
    ("gen4-k1-gen_e0382_clone_05_u8", &["E0382"]),  // k=1
    ("gen4-k1-gen_e0382_clone_10_i32", &["E0382"]),  // k=1
    ("gen4-k1-gen_e0382_clone_15_u16", &["E0382"]),  // k=1
    ("gen4-k1-gen_e0499_flat_00_00_u8", &["E0499"]),  // k=1
    ("gen4-k1-gen_e0499_flat_01_00_i32", &["E0499"]),  // k=1
    ("gen4-k1-gen_e0499_flat_02_00_u64", &["E0499"]),  // k=1
    ("gen4-k1-gen_e0499_flat_03_00_i16", &["E0499"]),  // k=1
    ("gen4-k1-gen_e0499_flat_04_00_u32", &["E0499"]),  // k=1
    ("gen4-k1-gen_e0502_copy_00_u8", &["E0502"]),  // k=1
    ("gen4-k1-gen_e0502_copy_05_i32", &["E0502"]),  // k=1
    ("gen4-k1-gen_e0502_copy_10_u16", &["E0502"]),  // k=1
    ("gen4-k1-gen_e0502_copy_15_u32", &["E0502"]),  // k=1
    ("gen4-k1-gen_e0502_len_00", &["E0502"]),  // k=1
    ("gen4-k1-gen_e0502_len_05", &["E0502"]),  // k=1
    ("gen4-k1-gen_e0502_len_10", &["E0502"]),  // k=1
    ("gen4-k1-gen_e0505_clone_00_u8", &["E0505"]),  // k=1
    ("gen4-k1-gen_e0505_clone_05_u8", &["E0505"]),  // k=1
    ("gen4-k1-gen_e0505_clone_10_i32", &["E0505"]),  // k=1
    ("gen4-k1-gen_e0505_clone_15_u64", &["E0505"]),  // k=1
    ("gen4-k1-gen_e0507_field_00_v_u8", &["E0507"]),  // k=1
    ("gen4-k1-gen_e0507_field_05_w_u16", &["E0507"]),  // k=1
    ("gen4-k1-gen_e0507_field_10_items_u8", &["E0507"]),  // k=1
    ("gen4-k1-gen_e0596_mut_00_u8", &["E0596"]),  // k=1
    ("gen4-k1-gen_e0596_mut_05_i32", &["E0596"]),  // k=1
    ("gen4-k1-gen_e0596_mut_10_u32", &["E0596"]),  // k=1
    ("gen4-k1-gen_e0596_str_00", &["E0596"]),  // k=1
    ("gen4-k1-gen_e0597_as_str_00_alpha", &["E0597"]),  // k=1
    ("gen4-k1-gen_e0597_fmt_00", &["E0597"]),  // k=1
    ("gen4-k1-gen_e0597_slice_00_u8", &["E0597"]),  // k=1
    ("gen4-k1-gen_e0597_slice_05_u16", &["E0597"]),  // k=1
    ("gen4-k1-gen_e0597_str_borrow_00_hello", &["E0597"]),  // k=1
    ("gen4-k1-gen_e0597_str_borrow_05_lifetime", &["E0597"]),  // k=1
    ("gen4-k1-gen_e0597_str_idx_00", &["E0597"]),  // k=1
    ("gen4-k1-gen_e0597_str_idx_05", &["E0597"]),  // k=1
    ("gen4-k2-gen_e0373_str_00_hello+gen_e0382_clone_00_u8", &["E0373", "E0382"]),  // k=2
    ("gen4-k2-gen_e0373_str_05_box+gen_e0382_clone_05_u8", &["E0373", "E0382"]),  // k=2
    ("gen4-k2-gen_e0373_sum_00_u32+gen_e0382_clone_10_i32", &["E0373", "E0382"]),  // k=2
    ("gen4-k2-gen_e0373_sum_05_i64+gen_e0382_clone_15_u16", &["E0373", "E0382"]),  // k=2
    ("gen4-k2-gen_e0499_flat_00_00_u8+gen_e0502_copy_00_u8", &["E0499", "E0502"]),  // k=2
    ("gen4-k2-gen_e0499_flat_01_00_i32+gen_e0502_copy_05_i32", &["E0499", "E0502"]),  // k=2
    ("gen4-k2-gen_e0499_flat_02_00_u64+gen_e0502_copy_10_u16", &["E0499", "E0502"]),  // k=2
    ("gen4-k2-gen_e0499_flat_03_00_i16+gen_e0502_copy_15_u32", &["E0499", "E0502"]),  // k=2
    ("gen4-k2-gen_e0505_clone_00_u8+gen_e0507_field_00_v_u8", &["E0505", "E0507"]),  // k=2
    ("gen4-k2-gen_e0505_clone_05_u8+gen_e0507_field_05_w_u16", &["E0505", "E0507"]),  // k=2
    ("gen4-k2-gen_e0505_clone_10_i32+gen_e0507_field_10_items_u8", &["E0505", "E0507"]),  // k=2
    ("gen4-k2-gen_e0505_clone_15_u64+gen_e0596_str_00", &["E0505", "E0596"]),  // k=2
    ("gen4-k2-gen_e0596_mut_00_u8+gen_e0597_str_borrow_00_hello", &["E0596", "E0597"]),  // k=2
    ("gen4-k2-gen_e0596_mut_05_i32+gen_e0597_str_borrow_05_lifetime", &["E0596", "E0597"]),  // k=2
    ("gen4-k2-gen_e0596_mut_10_u32+gen_e0597_slice_00_u8", &["E0596", "E0597"]),  // k=2
    ("gen4-k2-gen_e0597_slice_05_u16+gen_e0373_sum_10_i32", &["E0373", "E0597"]),  // k=2
    ("gen4-k3-gen_e0373_str_00_hello+gen_e0382_clone_00_u8+gen_e0499_flat_00_00_u8", &["E0373", "E0382", "E0499"]),  // k=3
    ("gen4-k3-gen_e0373_sum_05_i64+gen_e0382_clone_15_u16+gen_e0499_flat_03_00_i16", &["E0373", "E0382", "E0499"]),  // k=3
    ("gen4-k3-gen_e0382_clone_05_u8+gen_e0499_flat_01_00_i32+gen_e0502_copy_05_i32", &["E0382", "E0499", "E0502"]),  // k=3
    ("gen4-k3-gen_e0499_flat_02_00_u64+gen_e0502_copy_10_u16+gen_e0505_clone_10_i32", &["E0499", "E0502", "E0505"]),  // k=3
    ("gen4-k3-gen_e0502_copy_00_u8+gen_e0505_clone_00_u8+gen_e0507_field_00_v_u8", &["E0502", "E0505", "E0507"]),  // k=3
    ("gen4-k3-gen_e0502_copy_15_u32+gen_e0505_clone_15_u64+gen_e0596_str_00", &["E0502", "E0505", "E0596"]),  // k=3
    ("gen4-k3-gen_e0502_len_00+gen_e0597_as_str_00_alpha+gen_e0502_len_05", &["E0502", "E0597"]),  // k=3
    ("gen4-k3-gen_e0505_clone_05_u8+gen_e0507_field_05_w_u16+gen_e0596_mut_05_i32", &["E0505", "E0507", "E0596"]),  // k=3
    ("gen4-k3-gen_e0507_field_10_items_u8+gen_e0596_mut_10_u32+gen_e0597_slice_00_u8", &["E0507", "E0596", "E0597"]),  // k=3
    ("gen4-k3-gen_e0596_mut_00_u8+gen_e0597_str_borrow_00_hello+gen_e0373_str_05_box", &["E0373", "E0596", "E0597"]),  // k=3
    ("gen4-k3-gen_e0597_slice_05_u16+gen_e0373_sum_10_i32+gen_e0499_flat_04_00_u32", &["E0373", "E0499", "E0597"]),  // k=3
    ("gen4-k3-gen_e0597_str_borrow_05_lifetime+gen_e0373_sum_00_u32+gen_e0382_clone_10_i32", &["E0373", "E0382", "E0597"]),  // k=3
    ("gen4-k4-gen_e0373_str_00_hello+gen_e0382_clone_00_u8+gen_e0499_flat_00_00_u8+gen_e0502_copy_00_u8", &["E0373", "E0382", "E0499", "E0502"]),  // k=4
    ("gen4-k4-gen_e0373_str_05_box+gen_e0382_clone_05_u8+gen_e0499_flat_01_00_i32+gen_e0502_copy_05_i32", &["E0373", "E0382", "E0499", "E0502"]),  // k=4
    ("gen4-k4-gen_e0373_sum_00_u32+gen_e0382_clone_10_i32+gen_e0499_flat_02_00_u64+gen_e0502_copy_10_u16", &["E0373", "E0382", "E0499", "E0502"]),  // k=4
    ("gen4-k4-gen_e0373_sum_05_i64+gen_e0382_clone_15_u16+gen_e0499_flat_03_00_i16+gen_e0502_copy_15_u32", &["E0373", "E0382", "E0499", "E0502"]),  // k=4
    ("gen4-k4-gen_e0505_clone_00_u8+gen_e0507_field_00_v_u8+gen_e0596_mut_00_u8+gen_e0597_str_borrow_00_hello", &["E0505", "E0507", "E0596", "E0597"]),  // k=4
    ("gen4-k4-gen_e0505_clone_05_u8+gen_e0507_field_05_w_u16+gen_e0596_mut_05_i32+gen_e0597_str_borrow_05_lifetime", &["E0505", "E0507", "E0596", "E0597"]),  // k=4
    ("gen4-k4-gen_e0505_clone_10_i32+gen_e0507_field_10_items_u8+gen_e0596_mut_10_u32+gen_e0597_slice_00_u8", &["E0505", "E0507", "E0596", "E0597"]),  // k=4
    ("gen4-k4-gen_e0505_clone_15_u64+gen_e0596_str_00+gen_e0597_slice_05_u16+gen_e0373_sum_10_i32", &["E0373", "E0505", "E0596", "E0597"]),  // k=4
];

/// Freeze readiness for generation 4 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
#[cfg(feature = "rustc_errors")]
pub fn v4_freeze_status() -> Result<usize, String> {
    freeze_status_for(
        &specs_v4(),
        4,
        "generation_v4_freeze",
        FROZEN_LABELS_V4,
        EXCLUDED_IDS_V4,
    )
}

// ── Generation 6: the real-repo shape ladder (Plan 095, Bench 069 reopen
//    candidate #2 — SALVAGED, see §1a/§1b of the plan) ────────────────────
//
// Candidate #2's premise — organically multi-lint spans are minable from
// sibling repos at the bare-stub filter — was FALSIFIED by measurement
// (the plan's mining table: 5 extraction runs, ~1,100 fn candidates, ONE
// organic multi). The SALVAGE is the real-SHAPE rung: spans carved from
// pre-heal sibling states (the parent of each repo's sweep commit), the
// file's own header context making crate-internal lint spans compile
// VERBATIM — the v1 provenance discipline ("trait scaffolding added, lint
// spans verbatim") mechanized. Four ORGANIC multi-lint carves did survive
// and carry the interference axis as a best-effort rung.
//
// `expect` labels are mining-time (batch-oracle) CLAIMS — the freeze e2e
// adjudicates them with the bench instrument; disagreement is a loud
// exclusion or label correction (the gen-3 flow).

/// One real-repo span carved from a pre-heal sibling state (generation 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSpanV6 {
    /// Stable id (`gen6-<stem>-<kind><n>-<idx>`).
    pub id: &'static str,
    /// `repo@sha:path (file-header carve)` — the mining provenance.
    pub provenance: &'static str,
    /// The span source, verbatim (the carve's header context rides INSIDE
    /// the code, so the bench/freeze wrap is the plain `wrap()`).
    pub code: &'static str,
    /// Mining-time vocabulary-lint claims (freeze-adjudicated).
    pub expect: &'static [&'static str],
}

pub static RAW_SPANS_V6: &[RawSpanV6] = &[
    // gen6-svr-fn3-000 | riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs
    RawSpanV6 {
        id: "gen6-svr-fn3-000",
        provenance: "riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs",
        code: r#"/// Discrete verdict parsed from the model's self-check output (paper §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Verdict {
    /// Model asserts the candidate is correct.
    Correct = 0,
    /// Model asserts the candidate is incorrect.
    Incorrect = 1,
    /// Model is uncertain.
    Unsure = 2,
}
/// One turn of an SVR trajectory — the unit the reward function scores.
#[derive(Clone, Debug)]
pub struct SvrTurn {
    /// Ground-truth correctness label `y_t` (training-only; `true` if correct).
    pub correct: bool,
    /// Parsed verdict `v_t`.
    pub verdict: Verdict,
    /// Parsed confidence `c_t ∈ [0,1]`.
    pub confidence: f32,
    /// Completion length `ℓ_t` (tokens). Used for length shaping.
    pub completion_len: u32,
    /// Truncation flag `q_t`: `true` if generation was truncated (hit max
    /// tokens without EOS).
    pub truncated: bool,
    /// Format reward `r_fmt,t` flag: `true` if both task-answer +
    /// self-check tags were present.
    pub format_ok: bool,
}
/// Reward coefficients (paper Table 6). [`SvrRewardConfig::default`] ships the
/// paper's exact values.
#[derive(Clone, Copy, Debug)]
pub struct SvrRewardConfig {
    /// `λ_abs` — absolute correctness reward.
    pub lambda_abs: f32,
    /// `λ_Δ` — turn-to-turn improvement weight.
    pub lambda_delta: f32,
    /// `α` — completion-length shaping strength.
    pub alpha: f32,
    /// `λ_keep` — correct-answer preservation bonus `φ(1,1)`.
    pub lambda_keep: f32,
    /// `λ_reg` — correct→incorrect regression penalty `φ(1,0)`.
    pub lambda_reg: f32,
    /// `λ_fail` — repeated-failure penalty `φ(0,0)`.
    pub lambda_fail: f32,
    /// `λ_trunc` — generation-truncation penalty.
    pub lambda_trunc: f32,
    /// `λ_cal` — Brier-style calibration reward.
    pub lambda_cal: f32,
    /// `λ_over` — asymmetric overconfidence penalty.
    pub lambda_over: f32,
    /// `λ_detect` — explicit error-detection reward.
    pub lambda_detect: f32,
    /// `λ_ready` — stop-readiness reward.
    pub lambda_ready: f32,
    /// `λ_fmt` — format weight.
    pub lambda_fmt: f32,
    /// `L_len` — completion-length cap for shaping.
    pub len_cap: u32,
}
/// Output of [`parse_self_check`]: parsed verdict + confidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParsedSelfCheck {
    pub verdict: Verdict,
    pub confidence: f32,
}
/// First-order refinement branch, classified from the previous turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefinementBranch {
    /// Previous generation was truncated — ask the model to restart fresh.
    Truncated,
    /// Previous verdict was Correct — ask the model to verify once more.
    Correct,
    /// Previous verdict was Incorrect or Unsure — ask the model to fix.
    IncorrectOrUnsure,
}
/// Adaptive inference scheduler implementing the stop rule (Eq. 4).
///
/// Deployment/inference only. During training the scheduler is DISABLED and
/// the trajectory continues for the forced `T_tr = 3` turns.
///
/// ```text
/// t̂_γ = min ( { t ∈ {1..T_max} : q_t = 0 ∧ v_t = C ∧ c_t ≥ γ } ∪ {T_max} )
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveScheduler {
    /// `γ` — single global confidence threshold (default 0.85).
    pub gamma: f32,
    /// `T_max` — deployment turn budget (default 10).
    pub t_max: usize,
}
/// `r_solve,t` (Eq. 15): absolute + length-shaped correctness, plus a
/// turn-to-turn progress term and a truncation penalty.
///
/// `t` is 1-indexed (the turn number within the trajectory).
/// `prev` is the previous turn (`None` for the first turn).
pub fn r_solve(t: usize, cur: &SvrTurn, prev: Option<&SvrTurn>, cfg: &SvrRewardConfig) -> f32 {
    let r_abs = cfg.lambda_abs * y_hat(cur, cfg);
    let r_prog = match (t, prev) {
        (1, _) => 0.0, // first turn has no previous state
        (_, Some(p)) => cfg.lambda_delta * (cur.y() - p.y()) + phi(p.correct, cur.correct, cfg),
        // Defensive: t ≥ 2 with no prev (malformed trajectory) degrades to
        // the first-turn case — the progress term is undefined without a
        // previous label.
        (_, None) => 0.0,
    };
    r_abs + r_prog - cfg.lambda_trunc * cur.q()
}
impl Verdict {
    /// Lowercase canonical word used in refinement prompts.
    pub fn as_word(self) -> &'static str {
        match self {
            Verdict::Correct => "correct",
            Verdict::Incorrect => "incorrect",
            Verdict::Unsure => "unsure",
        }
    }
}
impl SvrTurn {
    /// `y_t` as f32 (1.0 if correct, else 0.0).
    #[inline]
    fn y(&self) -> f32 {
        match self.correct {
            true => 1.0,
            false => 0.0,
        }
    }

    /// `q_t` as f32 (1.0 if truncated, else 0.0).
    #[inline]
    fn q(&self) -> f32 {
        match self.truncated {
            true => 1.0,
            false => 0.0,
        }
    }
}
impl Default for SvrRewardConfig {
    fn default() -> Self {
        Self {
            lambda_abs: 1.0,
            lambda_delta: 0.3,
            alpha: 0.5,
            lambda_keep: 0.3,
            lambda_reg: 0.5,
            lambda_fail: 0.3,
            lambda_trunc: 0.5,
            lambda_cal: 0.5,
            lambda_over: 0.8,
            lambda_detect: 0.2,
            lambda_ready: 0.3,
            lambda_fmt: 0.4,
            len_cap: 1024,
        }
    }
}
impl Default for AdaptiveScheduler {
    fn default() -> Self {
        Self {
            gamma: 0.85,
            t_max: 10,
        }
    }
}
impl AdaptiveScheduler {
    /// Decide whether to stop at turn `t` (1-indexed) given the just-produced
    /// turn.
    ///
    /// Stop iff: not truncated AND verdict = Correct AND confidence ≥ γ. Also
    /// returns `true` when `t >= t_max` (budget exhausted) — the scheduler
    /// never asks for an 11th turn.
    pub fn should_stop(&self, t: usize, turn: &SvrTurn) -> bool {
        if t >= self.t_max {
            return true;
        }
        !turn.truncated && turn.verdict == Verdict::Correct && turn.confidence >= self.gamma
    }

    /// Run the scheduler over a full trajectory, returning the stop turn
    /// index (1-indexed).
    ///
    /// Mirrors Eq. 4: returns the index of the first turn satisfying the stop
    /// rule, else `t_max`. The trajectory length is what was actually
    /// generated; if no turn satisfies the rule, the deployment budget `t_max`
    /// is returned regardless of trajectory length (the scheduler models the
    /// deployment budget, not the supplied sample).
    pub fn stop_turn(&self, turns: &[SvrTurn]) -> usize {
        for (i, turn) in turns.iter().enumerate() {
            let t = i + 1;
            if self.should_stop(t, turn) {
                return t;
            }
        }
        self.t_max
    }
}
/// Length-shaped correctness reward `ŷ_t` (Eq. 15 inner term).
///
/// `ŷ_t = y_t · (1 − α · min(ℓ_t, L_len) / L_len)`
///
/// At `ℓ_t = 0` the shaping leaves the raw label intact (`ŷ_t = y_t`); at
/// `ℓ_t ≥ L_len` the shaping reaches its maximum reduction factor of
/// `(1 − α)`.
#[inline]
fn y_hat(turn: &SvrTurn, cfg: &SvrRewardConfig) -> f32 {
    let len = turn.completion_len.min(cfg.len_cap) as f32;
    let cap = cfg.len_cap as f32;
    let shaping = 1.0 - cfg.alpha * (len / cap);
    turn.y() * shaping
}
/// Turn-to-turn transition term `φ(y_{t-1}, y_t)` (Eq. 15).
///
/// - `φ(1,1) = +λ_keep`  — preserve correct
/// - `φ(1,0) = −λ_reg`   — regress (correct → incorrect)
/// - `φ(0,0) = −λ_fail`  — repeated failure
/// - `φ(0,1) = 0`        — wrong → correct improvement
fn phi(prev_correct: bool, cur_correct: bool, cfg: &SvrRewardConfig) -> f32 {
    match (prev_correct, cur_correct) {
        (true, true) => cfg.lambda_keep,
        (true, false) => -cfg.lambda_reg,
        (false, false) => -cfg.lambda_fail,
        (false, true) => 0.0,
    }
}
/// `r_verify,t` (Eq. 16): the joint verdict–confidence self-verification reward.
///
/// The whole block is gated by `(1 − q_t)` — a truncated turn scores zero
/// self-verification, because the parsed verdict/confidence is unreliable when
/// generation did not reach EOS.
pub fn r_verify(cur: &SvrTurn, cfg: &SvrRewardConfig) -> f32 {
    let one_minus_q = 1.0 - cur.q();
    // Eq. 16 indicators.
    let i_over = match cur.verdict == Verdict::Correct && !cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let i_detect = match cur.verdict == Verdict::Incorrect && !cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let i_ready = match cur.verdict == Verdict::Correct && cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let y = cur.y();
    let c = cur.confidence;
    let brier = 1.0 - (c - y) * (c - y);
    one_minus_q
        * (cfg.lambda_cal * brier
            - cfg.lambda_over * c * i_over
            + cfg.lambda_detect * i_detect
            + cfg.lambda_ready * c * i_ready)
}
/// `r_fmt,t` (Eq. 17/18): 1.0 if both task-answer + self-check tags were
/// present, else 0.0.
#[inline]
pub fn r_fmt(cur: &SvrTurn) -> f32 {
    match cur.format_ok {
        true => 1.0,
        false => 0.0,
    }
}
/// Per-turn reward `r_t = r_solve,t + r_verify,t + λ_fmt · r_fmt,t` (Eq. 7
/// per-turn term, WITHOUT the `1/T_tr` averaging — the caller does averaging).
///
/// `t` is 1-indexed.
pub fn turn_reward(t: usize, cur: &SvrTurn, prev: Option<&SvrTurn>, cfg: &SvrRewardConfig) -> f32 {
    r_solve(t, cur, prev, cfg) + r_verify(cur, cfg) + cfg.lambda_fmt * r_fmt(cur)
}
/// Full trajectory reward (Eq. 7): `(1/T_tr) · Σ r_t` over `T_tr` turns.
///
/// `turns` is the trajectory; `T_tr = turns.len()`. An empty trajectory
/// returns 0.0.
pub fn trajectory_reward(turns: &[SvrTurn], cfg: &SvrRewardConfig) -> f32 {
    if turns.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    for (i, turn) in turns.iter().enumerate() {
        let t = i + 1;
        let prev = if i == 0 { None } else { Some(&turns[i - 1]) };
        sum += turn_reward(t, turn, prev, cfg);
    }
    sum / turns.len() as f32
}
"#,
        expect: &["match_bool", "use_self"],
    },
    // gen6-svr-fn6-001 | riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs
    RawSpanV6 {
        id: "gen6-svr-fn6-001",
        provenance: "riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs",
        code: r#"/// Discrete verdict parsed from the model's self-check output (paper §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Verdict {
    /// Model asserts the candidate is correct.
    Correct = 0,
    /// Model asserts the candidate is incorrect.
    Incorrect = 1,
    /// Model is uncertain.
    Unsure = 2,
}
/// One turn of an SVR trajectory — the unit the reward function scores.
#[derive(Clone, Debug)]
pub struct SvrTurn {
    /// Ground-truth correctness label `y_t` (training-only; `true` if correct).
    pub correct: bool,
    /// Parsed verdict `v_t`.
    pub verdict: Verdict,
    /// Parsed confidence `c_t ∈ [0,1]`.
    pub confidence: f32,
    /// Completion length `ℓ_t` (tokens). Used for length shaping.
    pub completion_len: u32,
    /// Truncation flag `q_t`: `true` if generation was truncated (hit max
    /// tokens without EOS).
    pub truncated: bool,
    /// Format reward `r_fmt,t` flag: `true` if both task-answer +
    /// self-check tags were present.
    pub format_ok: bool,
}
/// Reward coefficients (paper Table 6). [`SvrRewardConfig::default`] ships the
/// paper's exact values.
#[derive(Clone, Copy, Debug)]
pub struct SvrRewardConfig {
    /// `λ_abs` — absolute correctness reward.
    pub lambda_abs: f32,
    /// `λ_Δ` — turn-to-turn improvement weight.
    pub lambda_delta: f32,
    /// `α` — completion-length shaping strength.
    pub alpha: f32,
    /// `λ_keep` — correct-answer preservation bonus `φ(1,1)`.
    pub lambda_keep: f32,
    /// `λ_reg` — correct→incorrect regression penalty `φ(1,0)`.
    pub lambda_reg: f32,
    /// `λ_fail` — repeated-failure penalty `φ(0,0)`.
    pub lambda_fail: f32,
    /// `λ_trunc` — generation-truncation penalty.
    pub lambda_trunc: f32,
    /// `λ_cal` — Brier-style calibration reward.
    pub lambda_cal: f32,
    /// `λ_over` — asymmetric overconfidence penalty.
    pub lambda_over: f32,
    /// `λ_detect` — explicit error-detection reward.
    pub lambda_detect: f32,
    /// `λ_ready` — stop-readiness reward.
    pub lambda_ready: f32,
    /// `λ_fmt` — format weight.
    pub lambda_fmt: f32,
    /// `L_len` — completion-length cap for shaping.
    pub len_cap: u32,
}
/// Output of [`parse_self_check`]: parsed verdict + confidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParsedSelfCheck {
    pub verdict: Verdict,
    pub confidence: f32,
}
/// First-order refinement branch, classified from the previous turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefinementBranch {
    /// Previous generation was truncated — ask the model to restart fresh.
    Truncated,
    /// Previous verdict was Correct — ask the model to verify once more.
    Correct,
    /// Previous verdict was Incorrect or Unsure — ask the model to fix.
    IncorrectOrUnsure,
}
/// Adaptive inference scheduler implementing the stop rule (Eq. 4).
///
/// Deployment/inference only. During training the scheduler is DISABLED and
/// the trajectory continues for the forced `T_tr = 3` turns.
///
/// ```text
/// t̂_γ = min ( { t ∈ {1..T_max} : q_t = 0 ∧ v_t = C ∧ c_t ≥ γ } ∪ {T_max} )
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveScheduler {
    /// `γ` — single global confidence threshold (default 0.85).
    pub gamma: f32,
    /// `T_max` — deployment turn budget (default 10).
    pub t_max: usize,
}
/// Per-turn reward `r_t = r_solve,t + r_verify,t + λ_fmt · r_fmt,t` (Eq. 7
/// per-turn term, WITHOUT the `1/T_tr` averaging — the caller does averaging).
///
/// `t` is 1-indexed.
pub fn turn_reward(t: usize, cur: &SvrTurn, prev: Option<&SvrTurn>, cfg: &SvrRewardConfig) -> f32 {
    r_solve(t, cur, prev, cfg) + r_verify(cur, cfg) + cfg.lambda_fmt * r_fmt(cur)
}
impl Verdict {
    /// Lowercase canonical word used in refinement prompts.
    pub fn as_word(self) -> &'static str {
        match self {
            Verdict::Correct => "correct",
            Verdict::Incorrect => "incorrect",
            Verdict::Unsure => "unsure",
        }
    }
}
impl SvrTurn {
    /// `y_t` as f32 (1.0 if correct, else 0.0).
    #[inline]
    fn y(&self) -> f32 {
        match self.correct {
            true => 1.0,
            false => 0.0,
        }
    }

    /// `q_t` as f32 (1.0 if truncated, else 0.0).
    #[inline]
    fn q(&self) -> f32 {
        match self.truncated {
            true => 1.0,
            false => 0.0,
        }
    }
}
impl Default for SvrRewardConfig {
    fn default() -> Self {
        Self {
            lambda_abs: 1.0,
            lambda_delta: 0.3,
            alpha: 0.5,
            lambda_keep: 0.3,
            lambda_reg: 0.5,
            lambda_fail: 0.3,
            lambda_trunc: 0.5,
            lambda_cal: 0.5,
            lambda_over: 0.8,
            lambda_detect: 0.2,
            lambda_ready: 0.3,
            lambda_fmt: 0.4,
            len_cap: 1024,
        }
    }
}
impl Default for AdaptiveScheduler {
    fn default() -> Self {
        Self {
            gamma: 0.85,
            t_max: 10,
        }
    }
}
impl AdaptiveScheduler {
    /// Decide whether to stop at turn `t` (1-indexed) given the just-produced
    /// turn.
    ///
    /// Stop iff: not truncated AND verdict = Correct AND confidence ≥ γ. Also
    /// returns `true` when `t >= t_max` (budget exhausted) — the scheduler
    /// never asks for an 11th turn.
    pub fn should_stop(&self, t: usize, turn: &SvrTurn) -> bool {
        if t >= self.t_max {
            return true;
        }
        !turn.truncated && turn.verdict == Verdict::Correct && turn.confidence >= self.gamma
    }

    /// Run the scheduler over a full trajectory, returning the stop turn
    /// index (1-indexed).
    ///
    /// Mirrors Eq. 4: returns the index of the first turn satisfying the stop
    /// rule, else `t_max`. The trajectory length is what was actually
    /// generated; if no turn satisfies the rule, the deployment budget `t_max`
    /// is returned regardless of trajectory length (the scheduler models the
    /// deployment budget, not the supplied sample).
    pub fn stop_turn(&self, turns: &[SvrTurn]) -> usize {
        for (i, turn) in turns.iter().enumerate() {
            let t = i + 1;
            if self.should_stop(t, turn) {
                return t;
            }
        }
        self.t_max
    }
}
/// Length-shaped correctness reward `ŷ_t` (Eq. 15 inner term).
///
/// `ŷ_t = y_t · (1 − α · min(ℓ_t, L_len) / L_len)`
///
/// At `ℓ_t = 0` the shaping leaves the raw label intact (`ŷ_t = y_t`); at
/// `ℓ_t ≥ L_len` the shaping reaches its maximum reduction factor of
/// `(1 − α)`.
#[inline]
fn y_hat(turn: &SvrTurn, cfg: &SvrRewardConfig) -> f32 {
    let len = turn.completion_len.min(cfg.len_cap) as f32;
    let cap = cfg.len_cap as f32;
    let shaping = 1.0 - cfg.alpha * (len / cap);
    turn.y() * shaping
}
/// Turn-to-turn transition term `φ(y_{t-1}, y_t)` (Eq. 15).
///
/// - `φ(1,1) = +λ_keep`  — preserve correct
/// - `φ(1,0) = −λ_reg`   — regress (correct → incorrect)
/// - `φ(0,0) = −λ_fail`  — repeated failure
/// - `φ(0,1) = 0`        — wrong → correct improvement
fn phi(prev_correct: bool, cur_correct: bool, cfg: &SvrRewardConfig) -> f32 {
    match (prev_correct, cur_correct) {
        (true, true) => cfg.lambda_keep,
        (true, false) => -cfg.lambda_reg,
        (false, false) => -cfg.lambda_fail,
        (false, true) => 0.0,
    }
}
/// `r_solve,t` (Eq. 15): absolute + length-shaped correctness, plus a
/// turn-to-turn progress term and a truncation penalty.
///
/// `t` is 1-indexed (the turn number within the trajectory).
/// `prev` is the previous turn (`None` for the first turn).
pub fn r_solve(t: usize, cur: &SvrTurn, prev: Option<&SvrTurn>, cfg: &SvrRewardConfig) -> f32 {
    let r_abs = cfg.lambda_abs * y_hat(cur, cfg);
    let r_prog = match (t, prev) {
        (1, _) => 0.0, // first turn has no previous state
        (_, Some(p)) => cfg.lambda_delta * (cur.y() - p.y()) + phi(p.correct, cur.correct, cfg),
        // Defensive: t ≥ 2 with no prev (malformed trajectory) degrades to
        // the first-turn case — the progress term is undefined without a
        // previous label.
        (_, None) => 0.0,
    };
    r_abs + r_prog - cfg.lambda_trunc * cur.q()
}
/// `r_verify,t` (Eq. 16): the joint verdict–confidence self-verification reward.
///
/// The whole block is gated by `(1 − q_t)` — a truncated turn scores zero
/// self-verification, because the parsed verdict/confidence is unreliable when
/// generation did not reach EOS.
pub fn r_verify(cur: &SvrTurn, cfg: &SvrRewardConfig) -> f32 {
    let one_minus_q = 1.0 - cur.q();
    // Eq. 16 indicators.
    let i_over = match cur.verdict == Verdict::Correct && !cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let i_detect = match cur.verdict == Verdict::Incorrect && !cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let i_ready = match cur.verdict == Verdict::Correct && cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let y = cur.y();
    let c = cur.confidence;
    let brier = 1.0 - (c - y) * (c - y);
    one_minus_q
        * (cfg.lambda_cal * brier
            - cfg.lambda_over * c * i_over
            + cfg.lambda_detect * i_detect
            + cfg.lambda_ready * c * i_ready)
}
/// `r_fmt,t` (Eq. 17/18): 1.0 if both task-answer + self-check tags were
/// present, else 0.0.
#[inline]
pub fn r_fmt(cur: &SvrTurn) -> f32 {
    match cur.format_ok {
        true => 1.0,
        false => 0.0,
    }
}
/// Full trajectory reward (Eq. 7): `(1/T_tr) · Σ r_t` over `T_tr` turns.
///
/// `turns` is the trajectory; `T_tr = turns.len()`. An empty trajectory
/// returns 0.0.
pub fn trajectory_reward(turns: &[SvrTurn], cfg: &SvrRewardConfig) -> f32 {
    if turns.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    for (i, turn) in turns.iter().enumerate() {
        let t = i + 1;
        let prev = if i == 0 { None } else { Some(&turns[i - 1]) };
        sum += turn_reward(t, turn, prev, cfg);
    }
    sum / turns.len() as f32
}
"#,
        expect: &["match_bool", "use_self"],
    },
    // gen6-svr-fn7-002 | riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs
    RawSpanV6 {
        id: "gen6-svr-fn7-002",
        provenance: "riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs",
        code: r#"/// Discrete verdict parsed from the model's self-check output (paper §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Verdict {
    /// Model asserts the candidate is correct.
    Correct = 0,
    /// Model asserts the candidate is incorrect.
    Incorrect = 1,
    /// Model is uncertain.
    Unsure = 2,
}
/// One turn of an SVR trajectory — the unit the reward function scores.
#[derive(Clone, Debug)]
pub struct SvrTurn {
    /// Ground-truth correctness label `y_t` (training-only; `true` if correct).
    pub correct: bool,
    /// Parsed verdict `v_t`.
    pub verdict: Verdict,
    /// Parsed confidence `c_t ∈ [0,1]`.
    pub confidence: f32,
    /// Completion length `ℓ_t` (tokens). Used for length shaping.
    pub completion_len: u32,
    /// Truncation flag `q_t`: `true` if generation was truncated (hit max
    /// tokens without EOS).
    pub truncated: bool,
    /// Format reward `r_fmt,t` flag: `true` if both task-answer +
    /// self-check tags were present.
    pub format_ok: bool,
}
/// Reward coefficients (paper Table 6). [`SvrRewardConfig::default`] ships the
/// paper's exact values.
#[derive(Clone, Copy, Debug)]
pub struct SvrRewardConfig {
    /// `λ_abs` — absolute correctness reward.
    pub lambda_abs: f32,
    /// `λ_Δ` — turn-to-turn improvement weight.
    pub lambda_delta: f32,
    /// `α` — completion-length shaping strength.
    pub alpha: f32,
    /// `λ_keep` — correct-answer preservation bonus `φ(1,1)`.
    pub lambda_keep: f32,
    /// `λ_reg` — correct→incorrect regression penalty `φ(1,0)`.
    pub lambda_reg: f32,
    /// `λ_fail` — repeated-failure penalty `φ(0,0)`.
    pub lambda_fail: f32,
    /// `λ_trunc` — generation-truncation penalty.
    pub lambda_trunc: f32,
    /// `λ_cal` — Brier-style calibration reward.
    pub lambda_cal: f32,
    /// `λ_over` — asymmetric overconfidence penalty.
    pub lambda_over: f32,
    /// `λ_detect` — explicit error-detection reward.
    pub lambda_detect: f32,
    /// `λ_ready` — stop-readiness reward.
    pub lambda_ready: f32,
    /// `λ_fmt` — format weight.
    pub lambda_fmt: f32,
    /// `L_len` — completion-length cap for shaping.
    pub len_cap: u32,
}
/// Output of [`parse_self_check`]: parsed verdict + confidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParsedSelfCheck {
    pub verdict: Verdict,
    pub confidence: f32,
}
/// First-order refinement branch, classified from the previous turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefinementBranch {
    /// Previous generation was truncated — ask the model to restart fresh.
    Truncated,
    /// Previous verdict was Correct — ask the model to verify once more.
    Correct,
    /// Previous verdict was Incorrect or Unsure — ask the model to fix.
    IncorrectOrUnsure,
}
/// Adaptive inference scheduler implementing the stop rule (Eq. 4).
///
/// Deployment/inference only. During training the scheduler is DISABLED and
/// the trajectory continues for the forced `T_tr = 3` turns.
///
/// ```text
/// t̂_γ = min ( { t ∈ {1..T_max} : q_t = 0 ∧ v_t = C ∧ c_t ≥ γ } ∪ {T_max} )
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveScheduler {
    /// `γ` — single global confidence threshold (default 0.85).
    pub gamma: f32,
    /// `T_max` — deployment turn budget (default 10).
    pub t_max: usize,
}
/// Full trajectory reward (Eq. 7): `(1/T_tr) · Σ r_t` over `T_tr` turns.
///
/// `turns` is the trajectory; `T_tr = turns.len()`. An empty trajectory
/// returns 0.0.
pub fn trajectory_reward(turns: &[SvrTurn], cfg: &SvrRewardConfig) -> f32 {
    if turns.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    for (i, turn) in turns.iter().enumerate() {
        let t = i + 1;
        let prev = if i == 0 { None } else { Some(&turns[i - 1]) };
        sum += turn_reward(t, turn, prev, cfg);
    }
    sum / turns.len() as f32
}
impl Verdict {
    /// Lowercase canonical word used in refinement prompts.
    pub fn as_word(self) -> &'static str {
        match self {
            Verdict::Correct => "correct",
            Verdict::Incorrect => "incorrect",
            Verdict::Unsure => "unsure",
        }
    }
}
impl SvrTurn {
    /// `y_t` as f32 (1.0 if correct, else 0.0).
    #[inline]
    fn y(&self) -> f32 {
        match self.correct {
            true => 1.0,
            false => 0.0,
        }
    }

    /// `q_t` as f32 (1.0 if truncated, else 0.0).
    #[inline]
    fn q(&self) -> f32 {
        match self.truncated {
            true => 1.0,
            false => 0.0,
        }
    }
}
impl Default for SvrRewardConfig {
    fn default() -> Self {
        Self {
            lambda_abs: 1.0,
            lambda_delta: 0.3,
            alpha: 0.5,
            lambda_keep: 0.3,
            lambda_reg: 0.5,
            lambda_fail: 0.3,
            lambda_trunc: 0.5,
            lambda_cal: 0.5,
            lambda_over: 0.8,
            lambda_detect: 0.2,
            lambda_ready: 0.3,
            lambda_fmt: 0.4,
            len_cap: 1024,
        }
    }
}
impl Default for AdaptiveScheduler {
    fn default() -> Self {
        Self {
            gamma: 0.85,
            t_max: 10,
        }
    }
}
impl AdaptiveScheduler {
    /// Decide whether to stop at turn `t` (1-indexed) given the just-produced
    /// turn.
    ///
    /// Stop iff: not truncated AND verdict = Correct AND confidence ≥ γ. Also
    /// returns `true` when `t >= t_max` (budget exhausted) — the scheduler
    /// never asks for an 11th turn.
    pub fn should_stop(&self, t: usize, turn: &SvrTurn) -> bool {
        if t >= self.t_max {
            return true;
        }
        !turn.truncated && turn.verdict == Verdict::Correct && turn.confidence >= self.gamma
    }

    /// Run the scheduler over a full trajectory, returning the stop turn
    /// index (1-indexed).
    ///
    /// Mirrors Eq. 4: returns the index of the first turn satisfying the stop
    /// rule, else `t_max`. The trajectory length is what was actually
    /// generated; if no turn satisfies the rule, the deployment budget `t_max`
    /// is returned regardless of trajectory length (the scheduler models the
    /// deployment budget, not the supplied sample).
    pub fn stop_turn(&self, turns: &[SvrTurn]) -> usize {
        for (i, turn) in turns.iter().enumerate() {
            let t = i + 1;
            if self.should_stop(t, turn) {
                return t;
            }
        }
        self.t_max
    }
}
/// Length-shaped correctness reward `ŷ_t` (Eq. 15 inner term).
///
/// `ŷ_t = y_t · (1 − α · min(ℓ_t, L_len) / L_len)`
///
/// At `ℓ_t = 0` the shaping leaves the raw label intact (`ŷ_t = y_t`); at
/// `ℓ_t ≥ L_len` the shaping reaches its maximum reduction factor of
/// `(1 − α)`.
#[inline]
fn y_hat(turn: &SvrTurn, cfg: &SvrRewardConfig) -> f32 {
    let len = turn.completion_len.min(cfg.len_cap) as f32;
    let cap = cfg.len_cap as f32;
    let shaping = 1.0 - cfg.alpha * (len / cap);
    turn.y() * shaping
}
/// Turn-to-turn transition term `φ(y_{t-1}, y_t)` (Eq. 15).
///
/// - `φ(1,1) = +λ_keep`  — preserve correct
/// - `φ(1,0) = −λ_reg`   — regress (correct → incorrect)
/// - `φ(0,0) = −λ_fail`  — repeated failure
/// - `φ(0,1) = 0`        — wrong → correct improvement
fn phi(prev_correct: bool, cur_correct: bool, cfg: &SvrRewardConfig) -> f32 {
    match (prev_correct, cur_correct) {
        (true, true) => cfg.lambda_keep,
        (true, false) => -cfg.lambda_reg,
        (false, false) => -cfg.lambda_fail,
        (false, true) => 0.0,
    }
}
/// `r_solve,t` (Eq. 15): absolute + length-shaped correctness, plus a
/// turn-to-turn progress term and a truncation penalty.
///
/// `t` is 1-indexed (the turn number within the trajectory).
/// `prev` is the previous turn (`None` for the first turn).
pub fn r_solve(t: usize, cur: &SvrTurn, prev: Option<&SvrTurn>, cfg: &SvrRewardConfig) -> f32 {
    let r_abs = cfg.lambda_abs * y_hat(cur, cfg);
    let r_prog = match (t, prev) {
        (1, _) => 0.0, // first turn has no previous state
        (_, Some(p)) => cfg.lambda_delta * (cur.y() - p.y()) + phi(p.correct, cur.correct, cfg),
        // Defensive: t ≥ 2 with no prev (malformed trajectory) degrades to
        // the first-turn case — the progress term is undefined without a
        // previous label.
        (_, None) => 0.0,
    };
    r_abs + r_prog - cfg.lambda_trunc * cur.q()
}
/// `r_verify,t` (Eq. 16): the joint verdict–confidence self-verification reward.
///
/// The whole block is gated by `(1 − q_t)` — a truncated turn scores zero
/// self-verification, because the parsed verdict/confidence is unreliable when
/// generation did not reach EOS.
pub fn r_verify(cur: &SvrTurn, cfg: &SvrRewardConfig) -> f32 {
    let one_minus_q = 1.0 - cur.q();
    // Eq. 16 indicators.
    let i_over = match cur.verdict == Verdict::Correct && !cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let i_detect = match cur.verdict == Verdict::Incorrect && !cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let i_ready = match cur.verdict == Verdict::Correct && cur.correct {
        true => 1.0,
        false => 0.0,
    };
    let y = cur.y();
    let c = cur.confidence;
    let brier = 1.0 - (c - y) * (c - y);
    one_minus_q
        * (cfg.lambda_cal * brier
            - cfg.lambda_over * c * i_over
            + cfg.lambda_detect * i_detect
            + cfg.lambda_ready * c * i_ready)
}
/// `r_fmt,t` (Eq. 17/18): 1.0 if both task-answer + self-check tags were
/// present, else 0.0.
#[inline]
pub fn r_fmt(cur: &SvrTurn) -> f32 {
    match cur.format_ok {
        true => 1.0,
        false => 0.0,
    }
}
/// Per-turn reward `r_t = r_solve,t + r_verify,t + λ_fmt · r_fmt,t` (Eq. 7
/// per-turn term, WITHOUT the `1/T_tr` averaging — the caller does averaging).
///
/// `t` is 1-indexed.
pub fn turn_reward(t: usize, cur: &SvrTurn, prev: Option<&SvrTurn>, cfg: &SvrRewardConfig) -> f32 {
    r_solve(t, cur, prev, cfg) + r_verify(cur, cfg) + cfg.lambda_fmt * r_fmt(cur)
}
"#,
        expect: &["match_bool", "use_self"],
    },
    // gen6-transposition-impl1-000 | riir-ai@1e050442a~1:crates/riir-engine/src/fourier/transposition.rs
    RawSpanV6 {
        id: "gen6-transposition-impl1-000",
        provenance: "riir-ai@1e050442a~1:crates/riir-engine/src/fourier/transposition.rs",
        code: r"use std::cell::Cell;
use std::collections::HashMap;
/// Entry in the Fourier MCTS transposition table.
///
/// Stores accumulated search statistics for a spatial state shape.
/// Multiple visits to the same tactical shape (at different board positions)
/// contribute to the same entry.
#[derive(Clone, Copy, Debug)]
pub struct TransEntry {
    /// Accumulated reward from all visits to this state shape.
    pub total_reward: f32,
    /// Number of times this state shape was visited.
    pub visits: u32,
    /// Best action found so far for this state shape.
    pub best_action: Option<usize>,
    /// Maximum depth this state was explored to.
    pub max_depth: usize,
    /// Whether this entry has been verified (not just from heuristic rollout).
    pub verified: bool,
}
/// Statistics returned after a search with the transposition table.
#[derive(Clone, Copy, Debug, Default)]
pub struct TranspositionStats {
    /// Number of cache hits (reuse of existing entries).
    pub hits: u64,
    /// Number of cache misses (new exploration).
    pub misses: u64,
    /// Number of entries evicted during search.
    pub evictions: u64,
    /// Final number of entries in the table.
    pub entries: usize,
}
/// Transposition table for MCTS with Fourier spatial hashing.
///
/// # Usage
///
/// ```ignore
/// let mut table = TranspositionTable::new(10_000);
///
/// // On select/expand: check table
/// if let Some(entry) = table.lookup(hash) {
///     // Warm-start from existing statistics
/// }
///
/// // On backpropagate: update table
/// table.update(hash, reward, best_action, depth, verified);
/// ```
pub struct TranspositionTable {
    /// The hash map storing entries indexed by Fourier spatial hash.
    table: HashMap<u64, TransEntry>,
    /// Maximum entries before eviction kicks in.
    max_entries: usize,
    /// Number of hits (reuse) since creation or last reset.
    hits: Cell<u64>,
    /// Number of misses (new exploration) since creation or last reset.
    misses: Cell<u64>,
    /// Total evictions performed.
    evictions: Cell<u64>,
    /// Pre-allocated eviction buffer to avoid O(n) allocation per eviction (Issue 041).
    eviction_buf: Vec<(u64, u32)>,
}
impl TransEntry {
    /// Create a new entry with a single visit's data.
    pub fn new(reward: f32, action: Option<usize>, depth: usize, verified: bool) -> Self {
        Self {
            total_reward: reward,
            visits: 1,
            best_action: action,
            max_depth: depth,
            verified,
        }
    }

    /// Average reward (Q-value) for this state shape.
    #[inline]
    pub fn avg_reward(&self) -> f32 {
        if self.visits == 0 {
            return 0.0;
        }
        self.total_reward / self.visits as f32
    }

    /// Update this entry with new visit data.
    pub fn update(&mut self, reward: f32, action: Option<usize>, depth: usize, verified: bool) {
        self.total_reward += reward;
        self.visits += 1;
        if depth > self.max_depth {
            self.max_depth = depth;
        }
        if verified {
            self.verified = true;
        }
        // Update best action if this visit has higher reward
        if let Some(a) = action {
            match self.best_action {
                None => self.best_action = Some(a),
                Some(_) => {
                    // Keep existing best_action — it was found over multiple visits
                    // The best_action is updated only when we have higher confidence
                }
            }
        }
    }

    /// Merge another entry into this one (for combining results).
    pub fn merge(&mut self, other: &TransEntry) {
        self.total_reward += other.total_reward;
        self.visits += other.visits;
        if other.max_depth > self.max_depth {
            self.max_depth = other.max_depth;
        }
        if other.verified {
            self.verified = true;
        }
        if self.best_action.is_none() {
            self.best_action = other.best_action;
        }
    }
}
",
        expect: &["single_match_else", "use_self"],
    },
    // gen6-channel_draft_generator-fn1-002 | riir-ai@1e050442a~1:crates/riir-engine/src/channel_draft_generator.rs
    RawSpanV6 {
        id: "gen6-channel_draft_generator-fn1-002",
        provenance: "riir-ai@1e050442a~1:crates/riir-engine/src/channel_draft_generator.rs",
        code: r"/// Channel prediction alongside a draft token.
#[derive(Clone, Debug)]
pub struct ChannelDraftToken<T> {
    /// The draft token from the base generator.
    pub token: T,
    /// Predicted channel activations for this token's position.
    /// `None` when channel prediction is disabled (backward compat).
    pub channel_predictions: Option<Vec<f32>>,
}
/// Result of channel-aware draft verification.
#[derive(Clone, Copy, Debug)]
pub struct ChannelVerifyResult {
    /// Whether the draft was accepted (token match + channel alignment).
    pub accepted: bool,
    /// Channel alignment score (0.0 = no alignment, 1.0 = perfect).
    pub alignment_score: f32,
    /// Whether channel prediction was used (false when disabled).
    pub channel_used: bool,
}
/// Configuration for channel-aware draft generation.
#[derive(Clone, Copy, Debug)]
pub struct ChannelDraftConfig {
    /// Whether to enable channel prediction in draft output.
    pub enabled: bool,
    /// Threshold for channel alignment acceptance (0.0-1.0).
    pub alignment_threshold: f32,
    /// Whether to use conservative mode (channel prediction only REJECTS, never accepts).
    pub conservative: bool,
}
/// Verify channel alignment for a draft token.
///
/// Returns acceptance result based on predicted vs actual channel activations.
/// In conservative mode, channel mismatch can only REJECT (never accept on match alone).
pub fn verify_draft_channels(
    predicted: &Option<Vec<f32>>,
    actual: &[f32],
    config: &ChannelDraftConfig,
) -> ChannelVerifyResult {
    if !config.enabled {
        return ChannelVerifyResult {
            accepted: true,
            alignment_score: 1.0,
            channel_used: false,
        };
    }

    let predicted = match predicted {
        Some(p) => p,
        None => {
            return ChannelVerifyResult {
                accepted: true,
                alignment_score: 1.0,
                channel_used: false,
            };
        }
    };

    // Compute alignment score as 1 - MSE (bounded to [0,1])
    // Manual `*` beats `.powi(2)` (avoid powi overhead on hot path).
    let n = predicted.len();
    let mut sum_sq = 0.0f32;
    for i in 0..n {
        let d = predicted[i] - actual[i];
        sum_sq += d * d;
    }
    let mse = sum_sq / n.max(1) as f32;

    let alignment_score = (1.0 - mse).max(0.0);
    let aligned = alignment_score >= config.alignment_threshold;

    ChannelVerifyResult {
        accepted: aligned,
        alignment_score,
        channel_used: true,
    }
}
",
        expect: &["manual_let_else"],
    },
    // gen6-economy-impl3-000 | riir-ai@1e050442a~1:crates/riir-engine/src/fourier/economy.rs
    RawSpanV6 {
        id: "gen6-economy-impl3-000",
        provenance: "riir-ai@1e050442a~1:crates/riir-engine/src/fourier/economy.rs",
        code: r"use std::f32::consts::PI;
/// Named market cycles with approximate periods in days.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CycleName {
    /// Period ≈ 1 day — daily server reset.
    DailyReset,
    /// Period ≈ 7 days — weekly maintenance.
    WeeklyReset,
    /// Period ≈ 30 days — monthly event rotation.
    MonthlyEvent,
    /// Period ≈ 90 days — major patch cycle.
    PatchCycle,
    /// Unrecognized cycle period.
    Unknown,
}
/// A single frequency component from price decomposition.
#[derive(Clone, Copy, Debug)]
pub struct FrequencyComponent {
    /// Frequency in cycles per sample period.
    pub frequency: f32,
    /// Amplitude — how strong this cycle is.
    pub amplitude: f32,
    /// Phase offset in radians.
    pub phase: f32,
    /// Named cycle classification, if recognized.
    pub name: Option<CycleName>,
}
/// Price history encoded as Fourier frequency components.
#[derive(Clone, Debug)]
pub struct FourierPriceModel {
    /// Frequency components sorted by amplitude descending.
    pub components: Vec<FrequencyComponent>,
    /// DC component (mean price).
    pub base_price: f32,
    /// Number of price data points used to fit.
    pub data_points: usize,
    /// Goodness of fit (0.0 to 1.0, higher is better).
    pub r_squared: f32,
}
/// Encodes price histories as Fourier frequency models.
pub struct PriceEncoder {
    /// Maximum frequency to analyze (cycles per sample).
    max_frequency: f32,
    /// Minimum amplitude threshold — filters noise.
    min_amplitude: f32,
}
/// Detects market cycles in price history.
pub struct CycleDetector {
    encoder: PriceEncoder,
}
/// Forecasts prices using Fourier decomposition.
pub struct PredictivePricer {
    encoder: PriceEncoder,
}
impl CycleName {
    /// Expected period in days for this cycle.
    fn expected_period(self) -> f32 {
        match self {
            Self::DailyReset => 1.0,
            Self::WeeklyReset => 7.0,
            Self::MonthlyEvent => 30.0,
            Self::PatchCycle => 90.0,
            Self::Unknown => f32::MAX,
        }
    }

    /// All known cycle names (excludes `Unknown`).
    fn known() -> &'static [CycleName] {
        &[
            CycleName::DailyReset,
            CycleName::WeeklyReset,
            CycleName::MonthlyEvent,
            CycleName::PatchCycle,
        ]
    }

    /// Classify a period (in days) to the nearest named cycle.
    fn from_period(period: f32) -> Self {
        let tolerance = 0.11; // ~10% tolerance band (0.9→Daily passes, 100→Patch fails)
        for &cycle in Self::known() {
            let expected = cycle.expected_period();
            if (period - expected).abs() / expected <= tolerance {
                return cycle;
            }
        }
        Self::Unknown
    }
}
",
        expect: &["use_self"],
    },
    // katgpt-rs-pruner_04_product_policy-fn | katgpt-rs@5b2d42970~1:examples/pruner_04_product_policy.rs
    RawSpanV6 {
        id: "gen6-pruner_04_product_policy-12",
        provenance: "katgpt-rs@5b2d42970~1:examples/pruner_04_product_policy.rs",
        code: r"/// Compute Shannon entropy of a probability distribution (in bits).
fn entropy(probs: &[f32]) -> f32 {
    let mut h = 0.0_f32;
    for &p in probs {
        if p > 0.0 {
            h -= p * p.log2();
        }
    }
    h
}
/// Find the argmax (most probable token).
fn argmax(probs: &[f32]) -> usize {
    probs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}",
        expect: &["map_unwrap_or"],
    },
    // katgpt-rs-pruner_04_product_policy-fn | katgpt-rs@5b2d42970~1:examples/pruner_04_product_policy.rs
    RawSpanV6 {
        id: "gen6-pruner_04_product_policy-13",
        provenance: "katgpt-rs@5b2d42970~1:examples/pruner_04_product_policy.rs",
        code: r"/// Find the argmax (most probable token).
fn argmax(probs: &[f32]) -> usize {
    probs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}",
        expect: &["map_unwrap_or"],
    },
    // katgpt-rs-pruner_04_product_policy-fn | katgpt-rs@5b2d42970~1:examples/pruner_04_product_policy.rs
    RawSpanV6 {
        id: "gen6-pruner_04_product_policy-8",
        provenance: "katgpt-rs@5b2d42970~1:examples/pruner_04_product_policy.rs",
        code: r"/// Compute Shannon entropy of a probability distribution (in bits).
fn entropy(probs: &[f32]) -> f32 {
    let mut h = 0.0_f32;
    for &p in probs {
        if p > 0.0 {
            h -= p * p.log2();
        }
    }
    h
}
/// Find the argmax (most probable token).
fn argmax(probs: &[f32]) -> usize {
    probs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}",
        expect: &["map_unwrap_or"],
    },
    // gen6-bridge_cap-impl5-001 | riir-chain@7421245a~1:src/economics/bridge_cap.rs
    RawSpanV6 {
        id: "gen6-bridge_cap-impl5-001",
        provenance: "riir-chain@7421245a~1:src/economics/bridge_cap.rs",
        code: r"/// Bridge cap state — tracks total exposure and enforces deposit limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BridgeCap {
    /// Maximum exposure ratio (e.g., 0.5 = bridge can hold 50% of (stake - reserve)).
    pub cap_ratio: f32,
    /// Total bridge exposure (sum of all deposits not yet withdrawn).
    pub total_exposure: u64,
    /// Maximum deposits per epoch.
    pub rate_limit_per_epoch: u64,
    /// Deposits this epoch.
    pub deposits_this_epoch: u64,
}
/// Error when bridge deposit exceeds cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCapExceeded {
    pub requested: u64,
    pub max_allowed: u64,
    pub total_exposure: u64,
}
/// Error when epoch rate limit is exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochRateLimitExceeded {
    pub requested: u64,
    pub deposits_this_epoch: u64,
    pub rate_limit: u64,
}
impl BridgeCap {
    /// Create a new bridge cap with the given ratio and per-epoch rate limit.
    pub fn new(cap_ratio: f32, rate_limit_per_epoch: u64) -> Self {
        Self {
            cap_ratio,
            total_exposure: 0,
            rate_limit_per_epoch,
            deposits_this_epoch: 0,
        }
    }

    /// Current maximum exposure allowed.
    ///
    /// `max_exposure = cap_ratio × (total_stake - treasury_reserve)`
    /// Clamped to 0 if reserve exceeds stake.
    pub fn max_exposure(&self, total_stake: u64, treasury_reserve: u64) -> u64 {
        let effective_stake = total_stake.saturating_sub(treasury_reserve);
        // f64 intermediate preserves precision for effective_stake > 2^24 (f32 limit).
        // This is a security-critical cap — precision loss could allow over-exposure.
        ((self.cap_ratio as f64) * (effective_stake as f64)) as u64
    }

    /// Check if a deposit of `amount` is allowed under the exposure cap.
    ///
    /// Returns `Ok(())` if `total_exposure + amount <= max_exposure`.
    pub fn check_deposit(
        &self,
        amount: u64,
        total_stake: u64,
        treasury_reserve: u64,
    ) -> Result<(), BridgeCapExceeded> {
        let max = self.max_exposure(total_stake, treasury_reserve);
        let new_exposure = self.total_exposure.saturating_add(amount);

        match new_exposure <= max {
            true => Ok(()),
            false => Err(BridgeCapExceeded {
                requested: amount,
                max_allowed: max.saturating_sub(self.total_exposure),
                total_exposure: self.total_exposure,
            }),
        }
    }

    /// Check if a deposit respects the per-epoch rate limit.
    pub fn check_epoch_rate(&self, amount: u64) -> Result<(), EpochRateLimitExceeded> {
        let new_deposits = self.deposits_this_epoch.saturating_add(amount);

        match new_deposits <= self.rate_limit_per_epoch {
            true => Ok(()),
            false => Err(EpochRateLimitExceeded {
                requested: amount,
                deposits_this_epoch: self.deposits_this_epoch,
                rate_limit: self.rate_limit_per_epoch,
            }),
        }
    }

    /// Record a successful deposit — increases exposure and epoch counter.
    pub fn record_deposit(&mut self, amount: u64) {
        self.total_exposure = self.total_exposure.saturating_add(amount);
        self.deposits_this_epoch = self.deposits_this_epoch.saturating_add(amount);
    }

    /// Record a withdrawal — reduces exposure.
    pub fn record_withdrawal(&mut self, amount: u64) {
        self.total_exposure = self.total_exposure.saturating_sub(amount);
    }

    /// Reset epoch counter (call at epoch boundary).
    pub fn reset_epoch(&mut self) {
        self.deposits_this_epoch = 0;
    }
}
",
        expect: &["match_bool"],
    },
    // gen6-economy-impl4-001 | riir-ai@1e050442a~1:crates/riir-engine/src/fourier/economy.rs
    RawSpanV6 {
        id: "gen6-economy-impl4-001",
        provenance: "riir-ai@1e050442a~1:crates/riir-engine/src/fourier/economy.rs",
        code: r"use std::f32::consts::PI;
/// Named market cycles with approximate periods in days.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CycleName {
    /// Period ≈ 1 day — daily server reset.
    DailyReset,
    /// Period ≈ 7 days — weekly maintenance.
    WeeklyReset,
    /// Period ≈ 30 days — monthly event rotation.
    MonthlyEvent,
    /// Period ≈ 90 days — major patch cycle.
    PatchCycle,
    /// Unrecognized cycle period.
    Unknown,
}
/// A single frequency component from price decomposition.
#[derive(Clone, Copy, Debug)]
pub struct FrequencyComponent {
    /// Frequency in cycles per sample period.
    pub frequency: f32,
    /// Amplitude — how strong this cycle is.
    pub amplitude: f32,
    /// Phase offset in radians.
    pub phase: f32,
    /// Named cycle classification, if recognized.
    pub name: Option<CycleName>,
}
/// Price history encoded as Fourier frequency components.
#[derive(Clone, Debug)]
pub struct FourierPriceModel {
    /// Frequency components sorted by amplitude descending.
    pub components: Vec<FrequencyComponent>,
    /// DC component (mean price).
    pub base_price: f32,
    /// Number of price data points used to fit.
    pub data_points: usize,
    /// Goodness of fit (0.0 to 1.0, higher is better).
    pub r_squared: f32,
}
/// Encodes price histories as Fourier frequency models.
pub struct PriceEncoder {
    /// Maximum frequency to analyze (cycles per sample).
    max_frequency: f32,
    /// Minimum amplitude threshold — filters noise.
    min_amplitude: f32,
}
/// Detects market cycles in price history.
pub struct CycleDetector {
    encoder: PriceEncoder,
}
/// Forecasts prices using Fourier decomposition.
pub struct PredictivePricer {
    encoder: PriceEncoder,
}
impl FrequencyComponent {
    /// Period of this component in sample units.
    #[inline]
    pub fn period(&self) -> f32 {
        match self.frequency > 0.0 {
            true => 1.0 / self.frequency,
            false => f32::MAX,
        }
    }

    /// Reconstruct this component's contribution at sample index `t` given `n` total samples.
    pub fn reconstruct_at(&self, t: usize, _n: usize) -> f32 {
        let t_f = t as f32;
        self.amplitude * (2.0 * PI * self.frequency * t_f + self.phase).cos()
    }
}
",
        expect: &["match_bool"],
    },
    // gen6-mod-fn1-006 | riir-train@e6084429~1:crates/riir-train-engine/src/rosetta/mod.rs
    RawSpanV6 {
        id: "gen6-mod-fn1-006",
        provenance: "riir-train@e6084429~1:crates/riir-train-engine/src/rosetta/mod.rs",
        code: r#"use std::collections::HashMap;
/// A best-buddy pair: two neurons across adapters with high Pearson correlation.
///
/// Mutual nearest neighbors — neuron_a's most correlated partner in B is neuron_b,
/// AND neuron_b's most correlated partner in A is neuron_a.
#[derive(Clone, Debug)]
pub struct LoraNeuronPair {
    /// Domain name of adapter A (e.g., "bomber").
    pub domain_a: String,
    /// LoRA neuron index in adapter A.
    pub neuron_idx_a: usize,
    /// Domain name of adapter B (e.g., "go").
    pub domain_b: String,
    /// LoRA neuron index in adapter B.
    pub neuron_idx_b: usize,
    /// Pearson r correlation between the two neurons' activation patterns.
    pub correlation: f32,
}
/// A clustered universal concept shared across game adapters.
///
/// Groups neurons from different adapters that express the same latent concept
/// (e.g., threat detection, spatial navigation, resource management).
#[derive(Clone, Debug)]
pub struct GameConcept {
    /// Unique concept identifier.
    pub id: usize,
    /// All neurons across adapters expressing this concept: (domain, neuron_index).
    pub neurons: Vec<(String, usize)>,
    /// Centroid activation pattern — mean of all member activation vectors.
    pub centroid: Vec<f32>,
}
/// The Rosetta LoRA dictionary: cross-game neuron correspondences.
///
/// Built from best-buddy pairs and self-clustered into universal concepts.
/// Used for warm-start initialization, training diagnostics, and rank allocation.
#[derive(Clone, Debug, Default)]
pub struct RosettaLoraDictionary {
    /// All best-buddy pairs across adapter pairs.
    pub pairs: Vec<LoraNeuronPair>,
    /// Clustered universal concepts mined from transitive best-buddy chains.
    pub universal_concepts: Vec<GameConcept>,
}
/// Mines cross-game LoRA neuron correlations and builds alignment dictionaries.
///
/// Usage:
/// 1. `register_adapter()` for each trained LoRA adapter
/// 2. `set_probe_activations()` with forward-pass activations on shared probe states
/// 3. `mine_correlations()` → best-buddy pairs
/// 4. `build_dictionary()` → clustered dictionary
pub struct RosettaLoraMiner {
    /// Domain → flat LoRA weights (for dimension validation).
    adapters: HashMap<String, Vec<f32>>,
    /// Domain → per-probe-state activation vectors [n_states × n_neurons].
    probe_activations: HashMap<String, Vec<Vec<f32>>>,
    /// Minimum Pearson r to consider a pair as best buddies.
    correlation_threshold: f32,
}
/// Per-checkpoint activation snapshot.
#[derive(Clone, Debug)]
pub struct CheckpointSnapshot {
    /// Training step number.
    pub step: usize,
    /// Activation patterns for probe states (n_neurons × n_states).
    pub activations: Vec<Vec<f32>>,
    /// Cross-game correlations at this checkpoint.
    pub rosetta_correlations: Vec<f32>,
}
/// Track concept stability across training checkpoints (Plan 233 T4).
///
/// Neurons maintaining >threshold correlation across checkpoints are "universal".
/// Neurons that drift are "frontier" (game-specific adaptations).
#[derive(Clone, Debug)]
pub struct ConceptEvolutionTracker {
    /// Per-checkpoint activation snapshots.
    checkpoint_activations: Vec<CheckpointSnapshot>,
    /// Neurons maintaining >threshold correlation across checkpoints.
    stable_neurons: Vec<usize>,
    /// Neurons still evolving.
    frontier_neurons: Vec<usize>,
    /// Threshold for "stable" (default: 0.9 correlation).
    stability_threshold: f32,
}
const ROSETTA_MAGIC: &[u8; 4] = b"RSTD";
const ROSETTA_VERSION: u32 = 1;
/// Header size: magic(4) + version(4) + n_pairs(4) + n_concepts(4) = 16 bytes.
const HEADER_SIZE: usize = 16;
/// blake3 hash trailer size.
const HASH_SIZE: usize = 32;
/// Compute Pearson correlation coefficient between two activation vectors.
///
/// r = cov(a, b) / (std(a) · std(b))
///
/// Returns 0.0 for degenerate cases (empty, zero variance, length mismatch).
pub fn pearson_correlation(a: &[f32], b: &[f32]) -> f32 {
    match (a.is_empty(), b.is_empty(), a.len() != b.len()) {
        (true, _, _) | (_, true, _) => return 0.0,
        (_, _, true) => return 0.0,
        _ => {}
    }

    let n = a.len() as f64;
    let sum_a: f64 = a.iter().map(|&x| x as f64).sum();
    let sum_b: f64 = b.iter().map(|&x| x as f64).sum();
    let mean_a = sum_a / n;
    let mean_b = sum_b / n;

    // Single-element vectors have zero variance.
    if n < 2.0 {
        return 0.0;
    }

    let mut cov = 0.0_f64;
    let mut var_a = 0.0_f64;
    let mut var_b = 0.0_f64;

    for (&xa, &xb) in a.iter().zip(b.iter()) {
        let da = xa as f64 - mean_a;
        let db = xb as f64 - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }

    let denom = var_a.sqrt() * var_b.sqrt();
    match denom > 1e-12 {
        true => (cov / denom) as f32,
        false => 0.0,
    }
}
"#,
        expect: &["match_bool"],
    },
    // gen6-resource_credits-impl1-002 | riir-chain@7421245a~1:src/economics/resource_credits.rs
    RawSpanV6 {
        id: "gen6-resource_credits-impl1-002",
        provenance: "riir-chain@7421245a~1:src/economics/resource_credits.rs",
        code: r"/// Resource credit state per account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceCredits {
    /// Free credits per tick (for non-staked accounts).
    pub free_credits_per_tick: u64,
    /// Credits per stake unit.
    pub credits_per_stake_unit: u64,
    /// Current available credits.
    pub available: u64,
    /// Maximum credit accumulation (burst cap).
    pub max_credits: u64,
    /// Last tick when credits were refilled.
    pub last_refill_tick: u64,
}
impl ResourceCredits {
    /// Create a new resource credit tracker.
    ///
    /// Starts with full credits available (`max_credits`).
    pub fn new(free_credits: u64, credits_per_stake: u64, max_credits: u64) -> Self {
        Self {
            free_credits_per_tick: free_credits,
            credits_per_stake_unit: credits_per_stake,
            available: max_credits,
            max_credits,
            last_refill_tick: 0,
        }
    }

    /// Create with empty credits (for testing refill logic).
    pub fn empty(free_credits: u64, credits_per_stake: u64, max_credits: u64) -> Self {
        Self {
            free_credits_per_tick: free_credits,
            credits_per_stake_unit: credits_per_stake,
            available: 0,
            max_credits,
            last_refill_tick: 0,
        }
    }

    /// Refill credits based on elapsed ticks and stake.
    ///
    /// `credits_added = elapsed_ticks × (free_credits_per_tick + stake × credits_per_stake_unit)`
    /// Clamped to `max_credits`. No-op if `current_tick <= last_refill_tick`.
    pub fn refill(&mut self, current_tick: u64, stake: u64) {
        if current_tick <= self.last_refill_tick {
            return;
        }

        let elapsed = current_tick - self.last_refill_tick;

        // stake credits: elapsed × stake × credits_per_stake_unit
        let stake_credits = self
            .credits_per_stake_unit
            .saturating_mul(stake)
            .saturating_mul(elapsed);

        // free credits: elapsed × free_credits_per_tick
        let free_credits = self.free_credits_per_tick.saturating_mul(elapsed);

        let total_refill = stake_credits.saturating_add(free_credits);
        let new_available = self.available.saturating_add(total_refill);

        self.available = new_available.min(self.max_credits);
        self.last_refill_tick = current_tick;
    }

    /// Try to consume credits for a transaction.
    ///
    /// Returns `true` if credits were available and deducted, `false` otherwise.
    pub fn try_consume(&mut self, cost: u64) -> bool {
        match self.available >= cost {
            true => {
                self.available -= cost;
                true
            }
            false => false,
        }
    }

    /// Compute cost for a transaction type.
    pub fn tx_cost(tx_type: u8) -> u64 {
        match tx_type {
            0 => 100,  // transfer
            1 => 500,  // swap
            2 => 2000, // shard write
            3 => 5000, // bridge
            _ => 1000, // default
        }
    }

    /// Effective refill rate per tick for a given stake.
    pub fn refill_rate(&self, stake: u64) -> u64 {
        self.free_credits_per_tick
            .saturating_add(self.credits_per_stake_unit.saturating_mul(stake))
    }

    /// Rate-limiter variant: try to acquire `credits` at `current_tick`.
    ///
    /// Combines refill + consume in one call. Returns `true` if the
    /// operation succeeded (credits were refilled and consumed).
    /// This is the drop-in replacement for gas-based rate limiting.
    pub fn try_acquire(&mut self, credits: u64, current_tick: u64, stake: u64) -> bool {
        self.refill(current_tick, stake);
        self.try_consume(credits)
    }

    /// Compute credits per tick using the canonical formula:
    /// `FREE_CREDITS + (stake × CREDITS_PER_STAKE)`
    ///
    /// Convenience constructor from constants.
    pub fn from_constants(_stake: u64) -> Self {
        const FREE_CREDITS: u64 = 100;
        const CREDITS_PER_STAKE: u64 = 1;
        const MAX_CREDITS: u64 = 100_000;
        Self::new(FREE_CREDITS, CREDITS_PER_STAKE, MAX_CREDITS)
    }
}
",
        expect: &["match_bool"],
    },
    // katgpt-rs-attn_match_head_budget-fn | katgpt-rs@5b2d42970~1:examples/attn_match_head_budget.rs
    RawSpanV6 {
        id: "gen6-attn_match_head_budget-0",
        provenance: "katgpt-rs@5b2d42970~1:examples/attn_match_head_budget.rs",
        code: r#"/// Format a BLAKE3 hash as a short hex prefix for display.
fn hex_short(hash: &[u8; 32]) -> String {
    let mut s = String::with_capacity(16);
    for b in &hash[..7] {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('…');
    s
}"#,
        expect: &["uninlined_format_args"],
    },
    // katgpt-rs-bandit_01_basic-fn | katgpt-rs@5b2d42970~1:examples/bandit_01_basic.rs
    RawSpanV6 {
        id: "gen6-bandit_01_basic-1",
        provenance: "katgpt-rs@5b2d42970~1:examples/bandit_01_basic.rs",
        code: r#"fn print_regret_comparison(results: &[(String, Vec<f32>, Vec<f32>)]) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              Cumulative Regret Comparison                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let checkpoints: [usize; 6] = [50, 100, 250, 500, 750, 1000];
    let checkpoint_labels = checkpoints.map(|c| format!("{c}"));

    // Header
    print!("{:<20}", "Episode");
    for label in &checkpoint_labels {
        print!("{:>10}", label);
    }
    println!();
    println!("{}", "─".repeat(20 + checkpoint_labels.len() * 10));

    for (name, regret_curve, _) in results {
        print!("{name:<20}");
        for &cp in &checkpoints {
            let idx = cp.saturating_sub(1);
            if idx < regret_curve.len() {
                print!("{:>10.1}", regret_curve[idx]);
            } else {
                print!("{:>10}", "—");
            }
        }
        println!();
    }
}"#,
        expect: &["uninlined_format_args"],
    },
    // katgpt-rs-bandit_01_basic-fn | katgpt-rs@5b2d42970~1:examples/bandit_01_basic.rs
    RawSpanV6 {
        id: "gen6-bandit_01_basic-2",
        provenance: "katgpt-rs@5b2d42970~1:examples/bandit_01_basic.rs",
        code: r#"fn print_reward_comparison(results: &[(String, Vec<f32>, Vec<f32>)]) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║            Average Reward Over Time                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let checkpoints: [usize; 6] = [50, 100, 250, 500, 750, 1000];
    let checkpoint_labels = checkpoints.map(|c| format!("{c}"));

    print!("{:<20}", "Episode");
    for label in &checkpoint_labels {
        print!("{:>10}", label);
    }
    println!();
    println!("{}", "─".repeat(20 + checkpoint_labels.len() * 10));

    for (name, _, reward_curve) in results {
        print!("{name:<20}");
        for &cp in &checkpoints {
            let idx = cp.saturating_sub(1);
            if idx < reward_curve.len() && cp > 0 {
                let avg = reward_curve[idx] / cp as f32;
                print!("{:>10.4}", avg);
            } else {
                print!("{:>10}", "—");
            }
        }
        println!();
    }
}"#,
        expect: &["uninlined_format_args"],
    },
    // katgpt-rs-clr_minimal-fn | katgpt-rs@5b2d42970~1:examples/clr_minimal.rs
    RawSpanV6 {
        id: "gen6-clr_minimal-3",
        provenance: "katgpt-rs@5b2d42970~1:examples/clr_minimal.rs",
        code: r#"fn label_str(o: u8) -> String {
    format!("answer-{}", o)
}"#,
        expect: &["uninlined_format_args"],
    },
];

/// The territory of a generation-6 span: `multi` (k≥2 — the organic
/// interference rung) vs `single` (k=1 — the real-shape rung). Reported
/// side by side, never blended (the v5 territory discipline).
#[must_use]
pub fn v6_territory(spec_id: &str) -> &'static str {
    for s in RAW_SPANS_V6 {
        if s.id == spec_id {
            return if s.expect.len() >= 2 {
                "multi"
            } else {
                "single"
            };
        }
    }
    "single"
}

/// The generation-6 spec ladder: one spec per mined span (no composition —
/// the spans ARE the organic material; k = the span's distinct-vocabulary-
/// lint count). Byte-stable — pinned by test.
#[must_use]
pub fn specs_v6() -> Vec<GenSpec> {
    RAW_SPANS_V6
        .iter()
        .map(|s| GenSpec {
            id: s.id.to_string(),
            k: s.expect.len(),
            expect_lints: s.expect.iter().map(ToString::to_string).collect(),
            code: s.code.to_string(),
        })
        .collect()
}

/// Spec ids the generation-6 freeze run EXCLUDED — landed by the freeze
/// e2e (`tests/generation_v6_freeze.rs` --ignored, 2026-09-06: 17/17 verified
/// at the BENCH instrument — single-snippet `lints_for` — with ZERO
/// exclusions; the batch and single instruments agree on every vocabulary
/// label).
pub static EXCLUDED_IDS_V6: &[&str] = &[];

/// The generation-6 frozen labels (spec id → vocabulary lints the BENCH
/// instrument fired at freeze time) — landed by the freeze e2e.
// Landed by tests/generation_v6_freeze.rs --ignored (2026-09-06): 17/17
// verified, 0 exclusions, 0 relabels; multi territory 4 (the organic
// interference rung), single territory 13. Freeze cost 3.35 s (17
// single-snippet oracle runs, release).
pub static FROZEN_LABELS_V6: &[(&str, &[&str])] = &[
    ("gen6-attn_match_head_budget-0", &["uninlined_format_args"]), // k=1
    ("gen6-bandit_01_basic-1", &["uninlined_format_args"]),        // k=1
    ("gen6-bandit_01_basic-2", &["uninlined_format_args"]),        // k=1
    ("gen6-bridge_cap-impl5-001", &["match_bool"]),                // k=1
    ("gen6-channel_draft_generator-fn1-002", &["manual_let_else"]), // k=1
    ("gen6-clr_minimal-3", &["uninlined_format_args"]),            // k=1
    ("gen6-economy-impl3-000", &["use_self"]),                     // k=1
    ("gen6-economy-impl4-001", &["match_bool"]),                   // k=1
    ("gen6-mod-fn1-006", &["match_bool"]),                         // k=1
    ("gen6-pruner_04_product_policy-12", &["map_unwrap_or"]),      // k=1
    ("gen6-pruner_04_product_policy-13", &["map_unwrap_or"]),      // k=1
    ("gen6-pruner_04_product_policy-8", &["map_unwrap_or"]),       // k=1
    ("gen6-resource_credits-impl1-002", &["match_bool"]),          // k=1
    ("gen6-svr-fn3-000", &["match_bool", "use_self"]),             // k=2
    ("gen6-svr-fn6-001", &["match_bool", "use_self"]),             // k=2
    ("gen6-svr-fn7-002", &["match_bool", "use_self"]),             // k=2
    (
        "gen6-transposition-impl1-000",
        &["single_match_else", "use_self"],
    ), // k=2
];

/// Freeze readiness for generation 6 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
pub fn v6_freeze_status() -> Result<usize, String> {
    freeze_status_for(
        &specs_v6(),
        6,
        "generation_v6_freeze",
        FROZEN_LABELS_V6,
        EXCLUDED_IDS_V6,
    )
}

/// The admitted generation-6 fixture set.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v6_freeze_status`] first.
#[must_use]
pub fn generation_v6() -> Vec<BenchFixture> {
    generation_fixtures(specs_v6(), 6, FROZEN_LABELS_V6, EXCLUDED_IDS_V6)
}

// ── generation 7 (Plan 096): the overlap-edit-chain ladder ─────────────
//
// Bench 069 candidate #3: composites whose proposers' edits overlap (the
// fixpoint's drop-and-refire semantics), forcing multi-pass convergence past
// MAX_FIX_PASSES = 4. Generations 2/3/5 compose disjoint atoms; generation 6
// mined organic interference; THIS generation manufactures maximal overlap.
//
// The mechanism (Plan 096 §2, probe-confirmed): `apply_edits_traced`
// selection is outer-first — an edit starting inside an already-selected
// region is dropped and its lint re-fires on the next pass. A spec is a
// CHAIN iff `heal_fixpoint`'s pass count ≥ 2; the depth = the pass count.
// The k dimension is the PRE-REGISTERED expected pass depth (not composition
// width): flat/ref = 1, d2 = 2, d3 = 3, d4 = 4 (converges AT the cap),
// cap = 5 (the depth the chain WOULD need — the pass cap stops it at 4).
//
// Atoms are authored claims adjudicated by the T1 probe (passes measured
// cargo-less) and the freeze e2e (labels measured with real clippy). The
// full probe record — including the two dropped atoms (question_mark's lack
// of match-form support, iter_next_loop's header-only edit) and the mined
// capacity-hint nested-block blind spot — lives in Plan 096 §T1 addendum.
// allowed with cause: the raw-string form keeps every fixture byte-visible
// (plain literals would bury the atoms in \n escapes); exactly one atom
// needs the hash form (an embedded "), so the table stays uniform.
#[allow(clippy::needless_raw_string_hashes)]
pub static RAW_ATOMS_V7: &[(&str, &str)] = &[
    // ── flat (the control: same lints, disjoint — expected 1 pass) ──
    (
        "gen7-flat-nrl-identity",
        r#"pub fn sum_all(hits: &[u32]) -> u32 {
    let mut total = 0;
    for i in 0..hits.len() {
        total += hits[i];
    }
    total
}

pub fn twice(v: u32) -> u32 {
    v * 1
}
"#,
    ),
    (
        "gen7-flat-cif-smx",
        r#"pub fn gate(a: bool, b: bool) -> u32 {
    let mut n = 0;
    if a {
        if b {
            n += 1;
        }
    }
    n
}

pub fn pick(kind: Option<u8>) -> u8 {
    match kind {
        Some(k) => k + 1,
        _ => { let fallback = 2; fallback }
    }
}
"#,
    ),
    // ── chain d2 (the outer atomic rewrite drops the inner edit; it
    //    re-fires on pass 2) ──
    (
        // The Issue-032 canonical case.
        "gen7-d2-nrl-identity",
        r#"pub fn weighted(hits: &[u32]) -> u32 {
    let mut total = 0;
    for i in 0..hits.len() {
        total += hits[i] * 1;
    }
    total
}
"#,
    ),
    (
        "gen7-flat-nrl-format",
        // Formerly gen7-d2-nrl-format, the CREATED-chain subtype: the bare
        // `__elem` arg was what uninlined_format_args fired on. Plan 097
        // T1's universal deref splice emits `*__elem` (a Unary, not a bare
        // ident), so the created firing site no longer exists — the subtype
        // is DISSOLVED and the fixture is retained as its regression pin
        // (expected passes 1).
        r#"pub fn announce(names: &[String]) {
    for i in 0..names.len() {
        println!("{}", names[i]);
    }
}
"#,
    ),
    (
        "gen7-d2-nrl-semi",
        r#"pub fn measure(hits: &[u32]) -> u32 {
    let mut n = 0;
    for i in 0..hits.len() {
        if hits[i] > 2 {
            n += hits[i]
        }
    }
    n
}
"#,
    ),
    (
        "gen7-d2-nrl-dblparen",
        r#"pub fn scaled(hits: &[u32], k: u32) -> u32 {
    let mut total = 0;
    for i in 0..hits.len() {
        total += ((hits[i] * k));
    }
    total
}
"#,
    ),
    (
        "gen7-d2-smx-identity",
        r#"pub fn pick(kind: Option<u8>) -> u8 {
    match kind {
        Some(k) => k * 1,
        _ => { let fallback = 2; fallback }
    }
}
"#,
    ),
    (
        "gen7-d2-mm-identity",
        r#"pub fn half(x: Option<i32>) -> Option<i32> {
    match x {
        Some(v) => Some(v * 1),
        None => None,
    }
}
"#,
    ),
    (
        // Formerly gen7-ref-nrl-nrl (the collision-guard persist): Plan 097
        // T2's fresh-binding uniquing lets level 2 heal with `__elem2` at
        // pass 2 — the persist became a real 2-pass chain.
        "gen7-d2-nrl-nrl",
        r#"pub fn grid(outer: &[u32], inner: &[u32]) -> u32 {
    let mut acc = 0;
    for i in 0..outer.len() {
        for j in 0..inner.len() {
            acc += outer[i] * inner[j];
        }
    }
    acc
}
"#,
    ),
    (
        // Cross-domain (single_match_else outer, distance-sq inner): the
        // distance-sq member is a rust_perf rule — invisible to the clippy
        // oracle, so only the clippy member is scored. The probe measured
        // the CHAIN (2 passes: whole-match rewrite, then the inner refire).
        "gen7-d2-sme-distsq",
        r#"pub fn near(kind: Option<(f64, f64, f64)>) -> bool {
    match kind {
        Some((x, y, r)) => (x * x + y * y).sqrt() < r,
        _ => { let eps = 0.5; eps < 1.0 }
    }
}
"#,
    ),
    (
        "gen7-d2-cif3",
        r#"pub fn gates(a: bool, b: bool, c: bool) -> u32 {
    let mut n = 0;
    if a {
        if b {
            if c {
                n += 1;
            }
        }
    }
    n
}
"#,
    ),
    // ── chain d3 / d4 (one collapsible pair merges per pass) ──
    (
        "gen7-d3-cif4",
        r#"pub fn gates(a: bool, b: bool, c: bool, d: bool) -> u32 {
    let mut n = 0;
    if a {
        if b {
            if c {
                if d {
                    n += 1;
                }
            }
        }
    }
    n
}
"#,
    ),
    (
        // Converges EXACTLY at the cap: 4 merges, 4 passes.
        "gen7-d4-cif5",
        r#"pub fn gates(a: bool, b: bool, c: bool, d: bool, e: bool) -> u32 {
    let mut n = 0;
    if a {
        if b {
            if c {
                if d {
                    if e {
                        n += 1;
                    }
                }
            }
        }
    }
    n
}
"#,
    ),
    // ── capped (the depth the chain needs exceeds MAX_FIX_PASSES = 4) ──
    (
        "gen7-cap-cif6",
        r#"pub fn gates(a: bool, b: bool, c: bool, d: bool, e: bool, f: bool) -> u32 {
    let mut n = 0;
    if a {
        if b {
            if c {
                if d {
                    if e {
                        if f {
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    n
}
"#,
    ),
    (
        // The 4 merges consume the cap; the identity member needs pass 5 —
        // collapsible_if resolves, identity_op persists (1/2).
        "gen7-cap-cif5-identity",
        r#"pub fn layered(a: bool, b: bool, c: bool, d: bool, e: bool) -> u32 {
    let mut n = 3;
    if a {
        if b {
            if c {
                if d {
                    if e {
                        n = n * 1;
                    }
                }
            }
        }
    }
    n
}
"#,
    ),
    // ── refused (a guard refusal under overlap pressure) ──
    (
        // Formerly gen7-ref-nrl-nrl: the collision-guard persist Plan 097
        // T2 FIXED (level 2 now heals with `__elem2` — it moved to the
        // chain territory as gen7-d2-nrl-nrl). The refused territory keeps
        // a member through the place-write refusal: the inner loop writes
        // through its slot (`inner[j] += 1`), the place-write guard refuses
        // REGARDLESS of uniquing — 1 pass, needless_range_loop persists.
        "gen7-ref-nrl-write",
        r#"pub fn tally(outer: &[u32], inner: &mut [u32]) -> u32 {
    let mut acc = 0;
    for i in 0..outer.len() {
        for j in 0..inner.len() {
            acc += outer[i];
            inner[j] += 1;
        }
    }
    acc
}
"#,
    ),
];

/// The pre-freeze label CLAIMS (the T1 probe + the sweep record); the freeze
/// e2e adjudicates both directions and the fresh verdict wins at bench time.
const EXPECTED_LINTS_V7: &[(&str, &[&str])] = &[
    (
        "gen7-flat-nrl-identity",
        &["needless_range_loop", "identity_op"],
    ),
    (
        "gen7-flat-cif-smx",
        &["collapsible_if", "single_match_else"],
    ),
    ("gen7-flat-nrl-format", &["needless_range_loop"]),
    (
        "gen7-d2-nrl-identity",
        &["needless_range_loop", "identity_op"],
    ),
    (
        "gen7-d2-nrl-semi",
        &["needless_range_loop", "semicolon_if_nothing_returned"],
    ),
    (
        "gen7-d2-nrl-dblparen",
        &["needless_range_loop", "double_parens"],
    ),
    (
        "gen7-d2-smx-identity",
        &["single_match_else", "identity_op"],
    ),
    ("gen7-d2-mm-identity", &["manual_map", "identity_op"]),
    ("gen7-d2-nrl-nrl", &["needless_range_loop"]),
    // The distance-sq member is a rust_perf rule — invisible to the clippy
    // oracle (Plan 096 §4).
    ("gen7-d2-sme-distsq", &["single_match_else"]),
    ("gen7-d2-cif3", &["collapsible_if"]),
    ("gen7-d3-cif4", &["collapsible_if"]),
    ("gen7-d4-cif5", &["collapsible_if"]),
    ("gen7-cap-cif6", &["collapsible_if"]),
    ("gen7-cap-cif5-identity", &["collapsible_if", "identity_op"]),
    ("gen7-ref-nrl-write", &["needless_range_loop"]),
];

/// The generation-7 spec ladder: one spec per authored chain atom, k = the
/// expected pass depth (the depth ladder, NOT composition width). Byte-stable
/// — pinned by test.
#[must_use]
pub fn specs_v7() -> Vec<GenSpec> {
    RAW_ATOMS_V7
        .iter()
        .map(|(id, code)| GenSpec {
            id: (*id).to_string(),
            k: v7_expected_k(id),
            expect_lints: EXPECTED_LINTS_V7
                .iter()
                .find(|(i, _)| *i == *id)
                .map_or_else(Vec::new, |(_, l)| {
                    l.iter().map(ToString::to_string).collect()
                }),
            code: (*code).to_string(),
        })
        .collect()
}

/// The territory of a generation-7 spec id: `flat` (the disjoint control),
/// `chain` (overlap, expected 2..=4 passes), `capped` (the depth exceeds
/// `MAX_FIX_PASSES` — pass 4 is the last), `refused` (a guard refusal leaves
/// the shape in place after 1 pass).
#[must_use]
pub fn v7_territory(spec_id: &str) -> &'static str {
    if spec_id.contains("-flat-") {
        "flat"
    } else if spec_id.contains("-cap-") {
        "capped"
    } else if spec_id.contains("-ref-") {
        "refused"
    } else {
        "chain"
    }
}

/// The expected `heal_fixpoint` pass count for a spec id (the T1-probe
/// pre-registration the always-on chain-truth gate asserts): flat/ref = 1,
/// d2 = 2, d3 = 3, d4 = 4 (converges AT the cap), cap = 4 (the cap itself).
/// k (the depth ladder's rung label) differs for capped specs: the depth the
/// chain WOULD need is 5.
#[must_use]
pub fn v7_expected_passes(spec_id: &str) -> usize {
    if spec_id.contains("-d2-") {
        2
    } else if spec_id.contains("-d3-") {
        3
    } else if spec_id.contains("-d4-") {
        4
    } else {
        // flat, ref, cap — all measure 1, 1, and the 4-pass cap respectively.
        if spec_id.contains("-cap-") {
            4
        } else {
            1
        }
    }
}

/// The depth-ladder rung for a spec id (= expected passes, except capped
/// specs sit on the k=5 rung — the depth the chain would need).
#[must_use]
pub fn v7_expected_k(spec_id: &str) -> usize {
    if spec_id.contains("-cap-") {
        5
    } else {
        v7_expected_passes(spec_id)
    }
}

/// Spec ids the generation-7 freeze run EXCLUDED — landed by the freeze e2e
/// (`tests/generation_v7_freeze.rs` --ignored, 2026-09-06). The FIRST freeze
/// run excluded 2 atoms (a parse-only probe gap — both repaired and
/// re-verified: E0308 return-type on d2-nrl-semi, E0425 cross-arm binding on
/// d2-sme-distsq); the landed record has ZERO standing exclusions.
pub static EXCLUDED_IDS_V7: &[&str] = &[];

/// The generation-7 frozen labels (spec id → vocabulary lints the BENCH
/// instrument fired at freeze time) — landed by the freeze e2e, RE-LANDED
/// by Plan 097 T5 (the P11 proposer fixes moved two ids: the collision
/// persist became chain d2, the created-chain subtype dissolved to flat —
/// see the atom comments). 16 specs, 0 exclusions; chain 10 / flat 3 /
/// capped 2 / refused 1.
pub static FROZEN_LABELS_V7: &[(&str, &[&str])] = &[
    ("gen7-cap-cif5-identity", &["collapsible_if", "identity_op"]), // depth=4
    ("gen7-cap-cif6", &["collapsible_if"]),                         // depth=4
    ("gen7-d2-cif3", &["collapsible_if"]),                          // depth=2
    ("gen7-d2-mm-identity", &["identity_op", "manual_map"]),        // depth=2
    (
        "gen7-d2-nrl-dblparen",
        &["double_parens", "needless_range_loop"],
    ), // depth=2
    (
        "gen7-d2-nrl-identity",
        &["identity_op", "needless_range_loop"],
    ), // depth=2
    ("gen7-d2-nrl-nrl", &["needless_range_loop"]),                  // depth=2
    (
        "gen7-d2-nrl-semi",
        &["needless_range_loop", "semicolon_if_nothing_returned"],
    ), // depth=2
    ("gen7-d2-sme-distsq", &["single_match_else"]),                 // depth=2
    (
        "gen7-d2-smx-identity",
        &["identity_op", "single_match_else"],
    ), // depth=2
    ("gen7-d3-cif4", &["collapsible_if"]),                          // depth=3
    ("gen7-d4-cif5", &["collapsible_if"]),                          // depth=4
    (
        "gen7-flat-cif-smx",
        &["collapsible_if", "single_match_else"],
    ), // depth=1
    ("gen7-flat-nrl-format", &["needless_range_loop"]),             // depth=1
    (
        "gen7-flat-nrl-identity",
        &["identity_op", "needless_range_loop"],
    ), // depth=1
    ("gen7-ref-nrl-write", &["needless_range_loop"]),               // depth=1
];

/// Freeze readiness for generation 7 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
pub fn v7_freeze_status() -> Result<usize, String> {
    freeze_status_for(
        &specs_v7(),
        7,
        "generation_v7_freeze",
        FROZEN_LABELS_V7,
        EXCLUDED_IDS_V7,
    )
}

/// The admitted generation-7 fixture set.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v7_freeze_status`] first.
#[must_use]
pub fn generation_v7() -> Vec<BenchFixture> {
    generation_fixtures(specs_v7(), 7, FROZEN_LABELS_V7, EXCLUDED_IDS_V7)
}

// ── generation 8 (Issue 089 T7): the shaped-verdict ladder ─────────────
//
// The half-win generation: single-target atoms whose heal resolves the
// target AND leaves a NEW diagnostic behind — the "target cleared ∧ new
// diags introduced" shape the shaped verdict prices at
// [`super::SHAPED_VERDICT`] (0.5). The creation mechanism is deliberate:
// match-arm → closure rewrites BIRTH an expression shape that did not
// exist pre-fix (manual_map's verbatim arm splice: `match o { Some(v) =>
// Some(f(v)), None => None }` → `o.map(|v| f(v))`), and the born closure
// fires `redundant_closure` / `redundant_closure_for_method_calls` — lints
// OUTSIDE `FIXABLE_LINTS` (VERIFY-only / pedantic), so the fixpoint cannot
// dissolve them.
//
// Precedent + anti-precedent (why these atoms are authored, not mined):
// v7's `gen7-flat-nrl-format` WAS a created-lint chain until Plan 097 T1's
// universal `*__elem` deref splice dissolved it — a created shape that
// CAN be re-healed disappears from `created` after the fixpoint and stops
// being shaped. The v8 atoms target the SURVIVING creation classes; the
// freeze e2e adjudicates each claim (a dissolved claim is excluded +
// recorded, never silently kept).
#[allow(clippy::needless_raw_string_hashes)]
pub static RAW_ATOMS_V8: &[(&str, &str)] = &[
    // ── closure-birth: fn-call (redundant_closure, style/VERIFY-only) ──
    (
        "gen8-birth-fncall",
        r#"pub fn apply(o: Option<u32>, f: fn(u32) -> u32) -> Option<u32> {
    match o {
        Some(v) => Some(f(v)),
        None => None,
    }
}
"#,
    ),
    // ── closure-birth: associated-fn path call (redundant_closure) ──
    (
        "gen8-birth-pathcall",
        r#"pub fn words(o: Option<&str>) -> Option<String> {
    match o {
        Some(v) => Some(String::from(v)),
        None => None,
    }
}
"#,
    ),
    // ── closure-birth: inherent method (redundant_closure_for_method_calls,
    //    pedantic — outside the fixable vocabulary entirely) ──
    (
        "gen8-birth-method",
        r#"pub fn lens(o: Option<String>) -> Option<usize> {
    match o {
        Some(v) => Some(v.len()),
        None => None,
    }
}
"#,
    ),
    // ── closure-birth: clone method (redundant_closure_for_method_calls) ──
    (
        "gen8-birth-clone",
        r#"pub fn twin(o: Option<String>) -> Option<String> {
    match o {
        Some(v) => Some(v.clone()),
        None => None,
    }
}
"#,
    ),
    // ── closure-birth: trait method via deref (redundant_closure_for_method_calls) ──
    (
        "gen8-birth-traitmethod",
        r#"pub fn labels(o: Option<u32>) -> Option<String> {
    match o {
        Some(v) => Some(v.to_string()),
        None => None,
    }
}
"#,
    ),
    // ── closure-birth: identity arm (map_identity, pedantic — the born
    //    closure `|v| v` IS the identity) ──
    (
        "gen8-birth-identity",
        r#"pub fn passthru(o: Option<u32>) -> Option<u32> {
    match o {
        Some(v) => Some(v),
        None => None,
    }
}
"#,
    ),
    // ── iterable-splice: the needless_range_loop rewrite's emitted shape
    //    (the class that produced v7's created-chain before Plan 097 T1
    //    dissolved the `__elem` half — whatever SURVIVES here is the
    //    creation the current splice still makes; the freeze adjudicates) ──
    (
        "gen8-splice-nrl",
        r#"pub fn count_filled(names: &[String]) -> usize {
    let mut n = 0;
    for i in 0..names.len() {
        if !names[i].is_empty() {
            n += 1;
        }
    }
    n
}
"#,
    ),
];

/// The pre-freeze TARGET claims (the vocabulary lint each atom fires at
/// freeze time). The freeze e2e adjudicates both directions and the fresh
/// verdict wins at bench time.
const EXPECTED_LINTS_V8: &[(&str, &[&str])] = &[
    ("gen8-birth-fncall", &["manual_map"]),
    ("gen8-birth-pathcall", &["manual_map"]),
    ("gen8-birth-method", &["manual_map"]),
    ("gen8-birth-clone", &["manual_map"]),
    ("gen8-birth-traitmethod", &["manual_map"]),
    ("gen8-birth-identity", &["manual_map"]),
    ("gen8-splice-nrl", &["needless_range_loop"]),
];

/// The pre-freeze CREATED claims (Issue 089 T7's innocence fence input):
/// the diagnostic each atom's heal is expected to LEAVE BEHIND. A claim is
/// adjudicated by the freeze e2e's after-verdict — measured-created wins,
/// and a claim whose created set comes back EMPTY (the shape dissolved —
/// the Plan 097 T1 precedent) is EXCLUDED, not kept unmeasured.
const V8_CREATED_CLAIMS: &[(&str, &[&str])] = &[
    ("gen8-birth-fncall", &["redundant_closure"]),
    ("gen8-birth-pathcall", &["redundant_closure"]),
    // Measured by the 2026-09-12 freeze: `map_clone`, NOT the originally
    // claimed redundant_closure_for_method_calls (the oracle won).
    ("gen8-birth-clone", &["map_clone"]),
    // Measured by the 2026-09-12 freeze: BOTH fire on the born `|v| v`.
    (
        "gen8-birth-identity",
        &["map_identity", "single_option_map"],
    ),
    // The three EXCLUDED atoms' claims (inherent/trait method calls, the
    // nrl splice) measured EMPTY — dissolved, excluded, recorded.
    (
        "gen8-birth-method",
        &["redundant_closure_for_method_calls"],
    ),
    (
        "gen8-birth-traitmethod",
        &["redundant_closure_for_method_calls"],
    ),
    ("gen8-splice-nrl", &[]),
];

/// The created-claims table for the freeze e2e (owned copies — the test
/// builds its adjudication maps from this).
#[must_use]
pub fn v8_created_claims() -> Vec<(String, Vec<String>)> {
    V8_CREATED_CLAIMS
        .iter()
        .map(|(id, l)| {
            (
                (*id).to_string(),
                l.iter().map(ToString::to_string).collect(),
            )
        })
        .collect()
}

/// The generation-8 spec list: one spec per authored atom, k = 1 (the
/// generation's axis is the SHAPE, not composition width or depth).
/// Byte-stable — pinned by test.
#[must_use]
pub fn specs_v8() -> Vec<GenSpec> {
    RAW_ATOMS_V8
        .iter()
        .map(|(id, code)| GenSpec {
            id: (*id).to_string(),
            k: 1,
            expect_lints: EXPECTED_LINTS_V8
                .iter()
                .find(|(i, _)| *i == *id)
                .map_or_else(Vec::new, |(_, l)| {
                    l.iter().map(ToString::to_string).collect()
                }),
            code: (*code).to_string(),
        })
        .collect()
}

/// The creation class of a generation-8 spec id: `closure-birth` (the
/// `manual_map` arm→closure splice) or `iter-splice` (the
/// `needless_range_loop` iterable rewrite).
#[must_use]
pub fn v8_creation_class(spec_id: &str) -> &'static str {
    if spec_id.starts_with("gen8-birth-") {
        "closure-birth"
    } else {
        "iter-splice"
    }
}

/// The expected `heal_fixpoint` pass count for a generation-8 spec id
/// (the chain-truth pre-registration): every atom is a single rewrite
/// whose created lint is unfixable by the fixpoint — 1 pass, then
/// convergence (the next pass finds no fixable lint). Excluded ids answer
/// 0 (they never run).
#[must_use]
pub fn v8_expected_passes(spec_id: &str) -> usize {
    usize::from(!EXCLUDED_IDS_V8.contains(&spec_id))
}

/// Spec ids the generation-8 freeze run EXCLUDED (dissolved shapes — the
/// honest negative controls, kept as recorded evidence): `gen8-birth-method`
/// (`|v| v.len()` fires nothing) and `gen8-birth-traitmethod`
/// (`|v| v.to_string()` fires nothing) — inherent/trait method calls do NOT
/// trip `redundant_closure_for_method_calls` in the bench configuration;
/// `gen8-splice-nrl` — the `*__elem` deref splice (Plan 097 T1) confirmed
/// to leave NO created lint on this shape. Landed by the freeze e2e
/// (2026-09-12).
pub static EXCLUDED_IDS_V8: &[&str] = &[
    "gen8-birth-method",
    "gen8-birth-traitmethod",
    "gen8-splice-nrl",
];

/// The generation-8 frozen labels (spec id → vocabulary lints the BENCH
/// instrument fired at freeze time) — landed by the freeze e2e
/// (2026-09-12). 4 admitted atoms, all closure-birth class. The identity
/// atom fires TWO targets (`manual_map` + `needless_match` — the match
/// satisfies both; the `manual_map` rewrite subsumes the `needless_match`
/// edit, resolving both in one pass).
pub static FROZEN_LABELS_V8: &[(&str, &[&str])] = &[
    ("gen8-birth-clone", &["manual_map"]),
    ("gen8-birth-fncall", &["manual_map"]),
    ("gen8-birth-identity", &["manual_map", "needless_match"]),
    ("gen8-birth-pathcall", &["manual_map"]),
];

/// The generation-8 measured CREATED record (spec id → the diagnostics
/// the heal left behind at freeze time, oracle-derived) — Issue 089 T7's
/// shape record, landed by the freeze e2e (2026-09-12). Measured beat two
/// pre-registered claims (the oracle is the authority): `clone` births
/// `map_clone` (not the claimed `redundant_closure_for_method_calls`);
/// `identity` births BOTH `map_identity` AND `single_option_map`. Every
/// created lint here is OUTSIDE `FIXABLE_LINTS` — the fixpoint cannot
/// dissolve them; the bench run re-derives this table live
/// (`FixtureOutcome.created`) and the v8 table flags drift against it.
pub static V8_CREATED: &[(&str, &[&str])] = &[
    ("gen8-birth-clone", &["map_clone"]),
    ("gen8-birth-fncall", &["redundant_closure"]),
    (
        "gen8-birth-identity",
        &["map_identity", "single_option_map"],
    ),
    ("gen8-birth-pathcall", &["redundant_closure"]),
];

/// Freeze readiness for generation 8 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
pub fn v8_freeze_status() -> Result<usize, String> {
    freeze_status_for(
        &specs_v8(),
        8,
        "generation_v8_freeze",
        FROZEN_LABELS_V8,
        EXCLUDED_IDS_V8,
    )
}

/// The admitted generation-8 fixture set.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v8_freeze_status`] first.
#[must_use]
pub fn generation_v8() -> Vec<BenchFixture> {
    generation_fixtures(specs_v8(), 8, FROZEN_LABELS_V8, EXCLUDED_IDS_V8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom_lints() -> BTreeSet<String> {
        atoms().into_iter().map(|a| a.lint).collect()
    }

    #[test]
    fn atom_table_is_the_parse_standalone_vocabulary_slice() {
        let lints = atom_lints();
        let vocab = super::super::vocabulary();
        // The atom table is the parse-standalone slice of the corpus: every
        // atom is a vocabulary lint, and the table is non-trivial. The freeze
        // e2e verifies each atom individually — this gate pins the factory's
        // invariants, not a count the corpus may legitimately change.
        assert!(!lints.is_empty(), "atom table empty — factory broken");
        for a in atoms() {
            assert!(
                vocab.contains(a.lint.as_str()),
                "atom {} not in vocabulary",
                a.lint
            );
            assert!(
                syn::parse_file(&a.code).is_ok(),
                "atom {} does not parse standalone",
                a.lint
            );
            assert!(
                !a.code.contains("mod gen_"),
                "atom {} carries the composite wrapper",
                a.lint
            );
        }
    }

    #[test]
    fn specs_are_deterministic_and_ladder_shaped() {
        let s1 = specs();
        let s2 = specs();
        assert_eq!(s1, s2, "specs must be byte-stable");
        let n = atoms().len();
        let k1 = s1.iter().filter(|s| s.k == 1).count();
        assert_eq!(k1, count_for_k(1, n));
        for &k in &LADDER_KS[1..] {
            for s in s1.iter().filter(|s| s.k == k) {
                assert_eq!(s.expect_lints.len(), k, "k={k} spec {} gold size", s.id);
                assert_eq!(
                    s.id,
                    format!("gen2-k{k}-{}", s.expect_lints.join("+")),
                    "id encodes k + gold"
                );
            }
        }
    }

    #[test]
    fn composite_code_wraps_every_atom_exactly_once() {
        let by_lint: BTreeMap<String, String> =
            atoms().into_iter().map(|a| (a.lint, a.code)).collect();
        for s in specs().iter().filter(|s| s.k >= 2) {
            for lint in &s.expect_lints {
                let atom_code = &by_lint[lint];
                assert!(
                    s.code.contains(atom_code.as_str()),
                    "composite {} lost atom {lint}",
                    s.id
                );
            }
            let mods = s.code.matches("mod gen_").count();
            assert_eq!(mods, s.k, "composite {} wraps k atoms", s.id);
        }
    }

    // ── generation 8 (Issue 089 T7): the shaped-verdict ladder gates ────

    #[test]
    fn v8_specs_parse_are_single_target_and_stable() {
        let s1 = specs_v8();
        let s2 = specs_v8();
        assert_eq!(s1, s2, "v8 specs must be byte-stable");
        assert_eq!(s1.len(), RAW_ATOMS_V8.len(), "spec table hole");
        for s in &s1 {
            assert!(
                syn::parse_file(&s.code).is_ok(),
                "v8 atom {} does not parse",
                s.id
            );
            assert_eq!(
                s.expect_lints.len(),
                1,
                "v8 atom {} must be single-target (the half-count attribution law)",
                s.id
            );
            assert_eq!(s.k, 1, "the v8 axis is the shape, not width/depth");
        }
    }

    #[test]
    fn v8_created_claims_are_innocent() {
        // The innocence fence, statically: a created claim that ALSO fires
        // pre-heal is not a created lint — it would be a persisting lint
        // mislabeled as creation (the freeze e2e re-checks this against the
        // live oracle; this pins the table's internal coherence).
        for (id, created) in V8_CREATED_CLAIMS {
            let target: &[&str] = EXPECTED_LINTS_V8
                .iter()
                .find(|(i, _)| i == id)
                .map_or(&[], |(_, l)| *l);
            for c in *created {
                assert!(
                    !target.contains(c),
                    "v8 atom {id}: created claim {c} is also a target — contradiction"
                );
            }
        }
    }

    #[test]
    fn v8_chain_truth_matches_the_preregistered_passes() {
        // The cargo-less chain-truth gate (the v7 pre-registration law):
        // measured heal_fixpoint passes == v8_expected_passes per atom. A
        // drift here means a proposer changed shape — re-adjudicate the
        // freeze before trusting any v8 bench row.
        for (id, code) in RAW_ATOMS_V8 {
            if EXCLUDED_IDS_V8.contains(id) {
                continue;
            }
            let (_healed, _edits, passes) = super::super::heal_fixpoint(code);
            assert_eq!(
                passes,
                v8_expected_passes(id),
                "chain-truth drift on {id}: measured {passes}"
            );
        }
    }

    #[test]
    fn v8_closure_birth_marker_survives_the_fixpoint() {
        // The mechanism canary (cargo-less, textual — the freeze e2e's
        // oracle verdict is the REAL gate; this catches the birth itself
        // dissolving): every closure-birth atom's healed output still
        // carries a born `.map(|` closure. If a future manual_map change
        // simplifies the splice (emitting `.map(f)` directly), the creation
        // mechanism is GONE and this reds before any oracle run does.
        for (id, code) in RAW_ATOMS_V8 {
            if v8_creation_class(id) != "closure-birth" || EXCLUDED_IDS_V8.contains(id) {
                continue;
            }
            let (healed, edits, _) = super::super::heal_fixpoint(code);
            assert!(edits > 0, "birth atom {id} took no edits");
            assert!(
                healed.contains(".map(|"),
                "birth atom {id} healed without a born closure: {healed}"
            );
        }
    }

    #[test]
    fn v8_freeze_status_is_loud_pre_freeze() {
        // Pre-freeze (no labels landed): Err — a silently-empty generation
        // must never reach the bench. Post-freeze: every spec labeled or
        // excluded (freeze_status_for's own assertion).
        if FROZEN_LABELS_V8.is_empty() && EXCLUDED_IDS_V8.is_empty() {
            assert!(v8_freeze_status().is_err(), "pre-freeze must not claim readiness");
        } else {
            assert!(v8_freeze_status().is_ok());
            // The created record covers every admitted atom (the shape
            // record's hole check — an admitted atom with no created row
            // would read as "measured, created nothing" by accident).
            let excluded: BTreeSet<&str> = EXCLUDED_IDS_V8.iter().copied().collect();
            let created_ids: BTreeSet<&str> = V8_CREATED.iter().map(|(id, _)| *id).collect();
            for (id, _) in FROZEN_LABELS_V8 {
                assert!(
                    created_ids.contains(id) || excluded.contains(id),
                    "admitted atom {id} has no V8_CREATED row"
                );
            }
        }
    }

    #[test]
    fn freeze_status_holes_are_loud_v3() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS_V3.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS_V3.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            // Pre-freeze: every spec is missing → Err (a silently-empty
            // generation must never reach the bench).
            assert!(
                v3_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            // Post-freeze: the always-on coverage gate — labels ∪ excluded
            // covers every spec, and each label equals the spec's union gold.
            for s in specs_v3() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS_V3
                        .iter()
                        .find(|(id, _)| *id == s.id)
                        .map_or(&[] as &[&str], |(_, l)| *l);
                    let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
                    let got: BTreeSet<&str> = frozen.iter().copied().collect();
                    assert_eq!(want, got, "frozen label drift on {}", s.id);
                }
            }
        }
    }

    #[test]
    fn k_of_parses_generation_ids() {
        assert_eq!(k_of("gen2-k3-a+b+c"), Some(3));
        assert_eq!(k_of("gen2-k1-identity_op"), Some(1));
        assert_eq!(k_of("gen3-k2-persist_array_default+map_unwrap_or"), Some(2));
        assert_eq!(k_of("composite-return_plus_identity_op"), None);
        assert_eq!(k_of("gen2-"), None);
        assert_eq!(k_of("gen-"), None);
    }

    #[test]
    fn persist_atom_table_parses_and_is_namespaced() {
        let vocab = super::super::vocabulary();
        assert!(!PERSIST_ATOMS.is_empty(), "persist table empty");
        for p in PERSIST_ATOMS {
            assert!(
                p.id.starts_with("persist_"),
                "persist id {} not namespaced",
                p.id
            );
            assert!(
                !p.expect.is_empty(),
                "persist atom {} expects nothing",
                p.id
            );
            for l in p.expect {
                assert!(
                    vocab.contains(*l),
                    "persist atom {} expects non-vocabulary lint {l}",
                    p.id
                );
            }
            assert!(
                syn::parse_file(p.code).is_ok(),
                "persist atom {} does not parse standalone",
                p.id
            );
            assert!(
                !p.code.contains("mod gen_"),
                "persist atom {} carries the wrapper",
                p.id
            );
        }
    }

    #[test]
    fn v3_specs_are_deterministic_and_ladder_shaped() {
        let s1 = specs_v3();
        let s2 = specs_v3();
        assert_eq!(s1, s2, "gen3 specs must be byte-stable");
        let n = v3_atoms().len();
        assert!(
            n > PERSIST_ATOMS.len(),
            "gen3 atom table lost its corpus half"
        );
        let k1 = s1.iter().filter(|s| s.k == 1).count();
        assert_eq!(k1, count_for_k(1, n));
        // The stratification contract: the persist atoms sit at the list
        // head, so every composite rung samples them.
        for &k in &LADDER_KS[1..] {
            let covered = count_for_k(k, n) * k;
            assert!(
                covered >= PERSIST_ATOMS.len(),
                "k={k} windows ({covered} slots) cannot reach all {} persist atoms",
                PERSIST_ATOMS.len()
            );
            for s in s1.iter().filter(|s| s.k == k) {
                let mods = s.code.matches("mod gen_").count();
                assert_eq!(mods, s.k, "gen3 composite {} wraps k atoms", s.id);
                assert!(
                    s.id.starts_with(&format!("gen3-k{k}-")),
                    "gen3 id encodes the generation + k: {}",
                    s.id
                );
            }
        }
        // Every persist atom appears in at least one k=2 composite.
        let k2_code: String = s1
            .iter()
            .filter(|s| s.k == 2)
            .map(|s| s.code.as_str())
            .collect();
        for p in PERSIST_ATOMS {
            assert!(
                k2_code.contains(p.code),
                "persist atom {} absent from every k=2 composite",
                p.id
            );
        }
    }

    #[test]
    fn freeze_status_holes_are_loud() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            // Pre-freeze: every spec is missing → Err (a silently-empty
            // generation must never reach the bench).
            assert!(
                v2_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            // Post-freeze: the always-on coverage gate — labels ∪ excluded
            // covers every spec, and each label equals the spec's union gold.
            for s in specs() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS
                        .iter()
                        .find(|(id, _)| *id == s.id)
                        .map_or(&[] as &[&str], |(_, l)| *l);
                    let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
                    let got: BTreeSet<&str> = frozen.iter().copied().collect();
                    assert_eq!(want, got, "frozen label drift on {}", s.id);
                }
            }
        }
    }

    // ── generation 4 (rustc_errors-gated: the atom factory reads corpus_gen)

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_atom_table_slices_the_corpus_deterministically() {
        use crate::domains::rustc_errors::corpus_gen::{generate_corpus, territory_of, Territory};

        let a1 = v4_atoms();
        let a2 = v4_atoms();
        assert_eq!(a1, a2, "gen4 atom table must be byte-stable");
        let corpus = generate_corpus();
        let by_name: BTreeMap<&str, &crate::domains::rustc_errors::corpus_gen::GeneratedCase> =
            corpus.iter().map(|c| (c.name.as_str(), c)).collect();
        assert!(!a1.is_empty(), "gen4 atom table empty");
        assert!(
            a1.len() <= GEN4_ATOM_TARGET,
            "gen4 slice overshot its target"
        );
        for a in &a1 {
            let case = by_name
                .get(a.id.as_str())
                .unwrap_or_else(|| panic!("gen4 atom {} is not a corpus case", a.id));
            assert_eq!(
                a.code, case.broken,
                "gen4 atom {} must carry the broken half",
                a.id
            );
            assert_eq!(a.expect, vec![case.expect_code.to_string()]);
            assert!(
                !a.code.contains("mod gen_"),
                "gen4 atom {} carries the wrapper",
                a.id
            );
        }
        // The stride slice must span both territories — the bench reports
        // them separately, and a single-territory ladder would blend by
        // omission.
        let uncontested = a1
            .iter()
            .any(|a| matches!(territory_of(&a.expect[0]), Territory::Uncontested));
        let contested = a1
            .iter()
            .any(|a| matches!(territory_of(&a.expect[0]), Territory::Contested));
        assert!(uncontested && contested, "gen4 slice lost a territory");
    }

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_specs_are_deterministic_and_ladder_shaped() {
        use crate::domains::rustc_errors::corpus_gen::territory_of;

        let s1 = specs_v4();
        let s2 = specs_v4();
        assert_eq!(s1, s2, "gen4 specs must be byte-stable");
        let ids: BTreeSet<&str> = s1.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids.len(), s1.len(), "duplicate gen4 generation ids");
        let n = v4_atoms().len();
        let k1 = s1.iter().filter(|s| s.k == 1).count();
        assert_eq!(k1, count_for_k(1, n));
        for &k in &LADDER_KS[1..] {
            for s in s1.iter().filter(|s| s.k == k) {
                let mods = s.code.matches("mod gen_").count();
                assert_eq!(mods, s.k, "gen4 composite {} wraps k atoms", s.id);
                assert!(
                    s.id.starts_with(&format!("gen4-k{k}-")),
                    "gen4 id encodes the generation + k: {}",
                    s.id
                );
                // expect_lints is the DISTINCT code set (≤ k — same-code
                // atoms can co-window); the per-member multiset is k.
                assert!(
                    !s.expect_lints.is_empty() && s.expect_lints.len() <= k,
                    "gen4 composite {} union size vs k",
                    s.id
                );
            }
            // Territory mix at every composite rung — both territories must
            // be reachable, or the per-territory report would be vacuous.
            let codes: BTreeSet<&str> = s1
                .iter()
                .filter(|s| s.k == k)
                .flat_map(|s| s.expect_lints.iter().map(String::as_str))
                .collect();
            let has_uncontested = codes.iter().any(|c| {
                matches!(
                    territory_of(c),
                    crate::domains::rustc_errors::corpus_gen::Territory::Uncontested
                )
            });
            let has_contested = codes.iter().any(|c| {
                matches!(
                    territory_of(c),
                    crate::domains::rustc_errors::corpus_gen::Territory::Contested
                )
            });
            assert!(
                has_uncontested && has_contested,
                "k={k} rung lost a territory"
            );
        }
        // Every distinct code appears in at least one k≥2 composite.
        let composite_codes: BTreeSet<&str> = s1
            .iter()
            .filter(|s| s.k >= 2)
            .flat_map(|s| s.expect_lints.iter().map(String::as_str))
            .collect();
        for a in v4_atoms() {
            let code = a.expect[0].clone();
            assert!(
                composite_codes.contains(code.as_str()),
                "code {code} absent from every gen4 composite"
            );
        }
    }

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_member_codes_resolves_the_per_member_multiset() {
        let specs = specs_v4();
        for s in &specs {
            let codes = v4_member_codes(&s.id)
                .unwrap_or_else(|| panic!("member codes unresolvable for {}", s.id));
            assert_eq!(codes.len(), s.k, "member multiset size vs k: {}", s.id);
            let distinct: BTreeSet<&str> = codes.iter().map(String::as_str).collect();
            let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
            assert_eq!(
                distinct, want,
                "member distinct set vs union gold: {}",
                s.id
            );
        }
        assert_eq!(v4_member_codes("gen4-"), None);
        assert_eq!(v4_member_codes("gen2-k1-identity_op"), None);
        assert_eq!(v4_member_codes("gen4-k2-nonexistent+also_missing"), None);
    }

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_freeze_status_holes_are_loud() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS_V4.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS_V4.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            assert!(
                v4_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            for s in specs_v4() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS_V4
                        .iter()
                        .find(|(id, _)| *id == s.id)
                        .map_or(&[] as &[&str], |(_, l)| *l);
                    let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
                    let got: BTreeSet<&str> = frozen.iter().copied().collect();
                    assert_eq!(want, got, "frozen label drift on {}", s.id);
                }
            }
        }
    }

    #[test]
    fn ids_are_unique_across_the_generation() {
        for (name, list) in [("gen2", specs()), ("gen3", specs_v3())] {
            let ids: BTreeSet<&str> = list.iter().map(|x| x.id.as_str()).collect();
            assert_eq!(ids.len(), list.len(), "duplicate {name} generation ids");
        }
    }

    // ── generation 5 (Plan 094) — always-on structural pins ────────────

    #[test]
    fn v5_atom_table_covers_the_fixable_vocabulary_deterministically() {
        let a1 = v5_atoms();
        let a2 = v5_atoms();
        assert_eq!(a1, a2, "gen5 atom table must be byte-stable");
        let by_id: BTreeMap<&str, &V3Atom> = a1.iter().map(|a| (a.id.as_str(), a)).collect();
        assert_eq!(by_id.len(), a1.len(), "duplicate gen5 atom ids");
        // Full FIXABLE vocabulary coverage: every rule has a corpus atom
        // (exactly one — the rule's own fixture-pinned shape); persist rows
        // may claim an ALREADY-covered rule (persist_coldstart_fxhashmap
        // sits beside the corpus fxhashmap atom — context-gate vs Gate-0
        // design-gate, two different claim classes over one rule).
        let fixable = crate::domains::rust_perf::proposers::FIXABLE_RULES;
        let mut covered: BTreeSet<String> = BTreeSet::new();
        for a in &a1 {
            assert_eq!(a.expect.len(), 1, "gen5 atoms are single-rule: {}", a.id);
            assert!(
                fixable.contains(&a.expect[0].as_str()),
                "atom {} claims non-vocabulary rule {}",
                a.id,
                a.expect[0]
            );
            if !a.id.starts_with("persist_") {
                assert!(
                    !a.id.starts_with("persist_") && covered.insert(a.expect[0].clone()),
                    "corpus rule {} covered twice",
                    a.expect[0]
                );
            }
        }
        for r in fixable {
            assert!(covered.contains(*r), "FIXABLE rule {r} has no gen5 atom");
        }
        // Persist atoms first (the stride-window head convention).
        for (i, a) in a1.iter().enumerate() {
            let is_persist = a.id.starts_with("persist_");
            if i < PERSIST_ATOMS_V5.len() {
                assert!(is_persist, "atom {} must precede the corpus atoms", a.id);
            } else if is_persist {
                panic!("persist atom {} escaped the head block", a.id);
            }
        }
        // Both territories, with the pre-registered membership.
        assert_eq!(v5_territory("persist_coldstart_fxhashmap"), "persist");
        assert_eq!(v5_territory("fxhashmap"), "persist");
        assert_eq!(v5_territory("vec-with-capacity"), "addressable");
        assert!(
            a1.iter().any(|a| v5_territory(&a.id) == "persist"),
            "no persist territory atoms"
        );
    }

    #[test]
    fn v5_ladder_shape_and_territory_mix() {
        let specs = specs_v5();
        let atoms = v5_atoms();
        let n = atoms.len();
        let want: Vec<usize> = LADDER_KS.iter().map(|&k| count_for_k(k, n)).collect();
        let got_per_k: BTreeMap<usize, usize> = {
            let mut m: BTreeMap<usize, usize> = BTreeMap::new();
            for s in &specs {
                *m.entry(s.k).or_default() += 1;
            }
            m
        };
        for (k, want_n) in LADDER_KS.iter().zip(&want) {
            assert_eq!(got_per_k.get(k), Some(want_n), "k={k} rung size");
        }
        let ids: BTreeSet<&str> = specs.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids.len(), specs.len(), "duplicate gen5 spec ids");
        // Every atom appears in at least one k>=2 composite.
        for a in &atoms {
            assert!(
                specs.iter().any(|s| s.k >= 2 && s.id.contains(&a.id)),
                "atom {} never composed",
                a.id
            );
        }
        // Both territories at every k>=2 rung.
        for k in [2, 3, 4] {
            let mut terrs: BTreeSet<&str> = BTreeSet::new();
            let by_id: BTreeMap<&str, &V3Atom> = atoms.iter().map(|a| (a.id.as_str(), a)).collect();
            for s in specs.iter().filter(|s| s.k == k) {
                let members = v5_member_rules(&s.id)
                    .unwrap_or_else(|| panic!("member rules unresolvable for {}", s.id));
                assert_eq!(members.len(), s.k, "member count on {}", s.id);
                for (i, name) in
                    s.id.strip_prefix("gen5-k")
                        .unwrap()
                        .split_once('-')
                        .unwrap()
                        .1
                        .split('+')
                        .enumerate()
                {
                    let atom = by_id
                        .get(name)
                        .unwrap_or_else(|| panic!("atom {name} missing"));
                    assert_eq!(atom.expect[0], members[i], "member order on {}", s.id);
                    terrs.insert(v5_territory(name));
                }
            }
            assert!(terrs.contains("persist"), "k={k} rung lacks a persist atom");
            assert!(
                terrs.contains("addressable"),
                "k={k} rung lacks an addressable atom"
            );
        }
    }

    /// The G2 marker-discrimination fence, always-on (pure textual, no
    /// cargo): every corpus atom's persist predicate is TRUE on buggy and
    /// FALSE on fixed; every persist atom's predicate is TRUE (the class is
    /// present) and its id is not shadowed by a corpus atom.
    #[test]
    fn v5_markers_discriminate_buggy_from_fixed() {
        let atoms = v5_atoms();
        let corpus = crate::build_perf_seed_corpus();
        let fixed_by_rule: BTreeMap<&str, &str> = corpus
            .entries
            .iter()
            .map(|e| (e.rule_id.as_str(), e.fixed_code.as_str()))
            .collect();
        for a in &atoms {
            let rule = &a.expect[0];
            assert!(
                v5_persists(rule, &a.code),
                "marker for {rule} absent from its own buggy atom {}",
                a.id
            );
            if a.id.starts_with("persist_") {
                continue; // persist atoms have no fixed_code by definition
            }
            let fixed = fixed_by_rule
                .get(rule.as_str())
                .unwrap_or_else(|| panic!("rule {rule} missing from corpus"));
            assert!(
                !v5_persists(rule, fixed),
                "marker for {rule} survives its own fixed_code (falsified claim)"
            );
            assert!(
                syn::parse_file(&a.code).is_ok(),
                "atom {} does not parse standalone",
                a.id
            );
        }
    }

    /// The G3 stratification fence, always-on: the production heal loop
    /// (both domains, the fxhashmap skip, `MAX_FIX_PASSES=4`) must leave every
    /// persist atom's shape in place. This is the CLASS claim — the
    /// gen-3 discipline that adjudicates persist records, and the tripwire
    /// that fires the day a proposer extension closes one of these classes.
    #[test]
    fn v5_persist_atoms_resolve_zero_through_the_production_heal() {
        for p in PERSIST_ATOMS_V5 {
            let rule = p.expect[0];
            let (healed, _edits, _passes) = super::super::heal_fixpoint(p.code);
            assert!(
                syn::parse_file(&healed).is_ok(),
                "persist atom {} healed to a non-parsing file",
                p.id
            );
            assert!(
                v5_persists(rule, &healed),
                "persist claim FALSIFIED: the heal resolves {} on {} — reshape or drop the atom (the gen-3 discipline) and re-record",
                rule,
                p.id
            );
        }
    }

    #[test]
    fn v5_region_extractor_aligns_member_order() {
        let specs = specs_v5();
        let s = specs.iter().find(|s| s.k == 2).expect("k=2 rung exists");
        for i in 0..s.k {
            let region = v5_module_region(&s.code, i)
                .unwrap_or_else(|| panic!("region gen_{i} missing in {}", s.id));
            let rule = &v5_member_rules(&s.id).unwrap()[i];
            assert!(
                v5_persists(rule, &region),
                "member {i} ({rule}) marker missing from its own region in {}",
                s.id
            );
        }
    }

    #[test]
    fn v5_freeze_status_holes_are_loud() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS_V5.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS_V5.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            assert!(
                v5_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            for s in specs_v5() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS_V5
                        .iter()
                        .find(|(id, _)| *id == s.id)
                        .map_or(&[] as &[&str], |(_, l)| *l);
                    let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
                    let got: BTreeSet<&str> = frozen.iter().copied().collect();
                    assert_eq!(want, got, "frozen label drift on {}", s.id);
                }
            }
        }
    }

    #[test]
    fn ids_are_unique_across_all_five_generations() {
        #[cfg(feature = "rustc_errors")]
        let gen4_specs = specs_v4();
        #[cfg(not(feature = "rustc_errors"))]
        let gen4_specs: Vec<GenSpec> = Vec::new();
        let lists = [specs(), specs_v3(), gen4_specs, specs_v5()];
        let mut all: Vec<&str> = Vec::new();
        for list in &lists {
            all.extend(list.iter().map(|x| x.id.as_str()));
        }
        let set: BTreeSet<&str> = all.iter().copied().collect();
        assert_eq!(
            set.len(),
            all.len(),
            "spec ids must be unique across generations"
        );
    }

    // ── generation 6 (Plan 095): real-repo shape ladder ────────────────

    /// Curation floors, amended per Plan 095 §1b (the crust yields what it
    /// yields — the original ≥24-single floor assumed singles were the
    /// CONTROL territory; here they ARE the population).
    #[test]
    fn v6_span_set_meets_the_amended_floors() {
        let multi = RAW_SPANS_V6.iter().filter(|s| s.expect.len() >= 2).count();
        let classes: BTreeSet<&str> = RAW_SPANS_V6
            .iter()
            .flat_map(|s| s.expect.iter().copied())
            .collect();
        let repos: BTreeSet<&str> = RAW_SPANS_V6
            .iter()
            .filter_map(|s| s.provenance.split('@').next())
            .collect();
        assert!(
            RAW_SPANS_V6.len() >= 16,
            "real-shape population too thin: {} (floor 16)",
            RAW_SPANS_V6.len()
        );
        assert!(
            multi >= 4,
            "organic multi rung lost: {multi} (floor 4 — the measured yield)"
        );
        assert!(
            classes.len() >= 6,
            "lint classes: {} (floor 6)",
            classes.len()
        );
        assert!(repos.len() >= 4, "source repos: {repos:?} (floor 4)");
        // ≤4 fixtures per (repo, class) — the v1 diversity discipline.
        let mut per: BTreeSet<(&str, &str)> = BTreeSet::new();
        let mut counts: std::collections::BTreeMap<(&str, &str), usize> =
            std::collections::BTreeMap::new();
        for s in RAW_SPANS_V6 {
            let repo = s.provenance.split('@').next().unwrap_or("?");
            for c in s.expect {
                per.insert((repo, c));
                *counts.entry((repo, c)).or_insert(0) += 1;
            }
        }
        for (k, n) in &counts {
            assert!(n <= &4, "per-(repo,class) cap blown at {k:?}: {n}");
        }
    }

    /// Every span parses standalone (the carve rides inside the code) and
    /// every territory claim matches the span's own label length.
    #[test]
    fn v6_spans_parse_and_territory_is_k_truthful() {
        for s in RAW_SPANS_V6 {
            assert!(
                syn::parse_file(s.code).is_ok(),
                "span {} does not parse standalone",
                s.id
            );
            let want = if s.expect.len() >= 2 {
                "multi"
            } else {
                "single"
            };
            assert_eq!(v6_territory(s.id), want, "territory drift on {}", s.id);
        }
    }

    /// ids unique including generation 6, and no id/provenance collision
    /// with v1's real fixtures (gen-6 ids carry the `gen6-` prefix; v1's
    /// real fixtures do not).
    #[test]
    fn v6_ids_are_disjoint_from_earlier_generations() {
        let v6: BTreeSet<&str> = RAW_SPANS_V6.iter().map(|s| s.id).collect();
        assert_eq!(v6.len(), RAW_SPANS_V6.len(), "duplicate gen-6 span ids");
        let earlier_specs: Vec<Vec<GenSpec>> = vec![specs(), specs_v3(), specs_v5()];
        let earlier: Vec<&str> = earlier_specs
            .iter()
            .flatten()
            .map(|s| s.id.as_str())
            .collect();
        for id in earlier {
            assert!(
                !v6.contains(id),
                "gen-6 id collides with an earlier spec: {id}"
            );
        }
        // v1 real fixtures start `<repo>-…`; gen-6 `gen6-…` — prefix disjoint.
        for f in crate::score_bench::fixtures::fixtures() {
            assert!(
                !v6.contains(f.id),
                "gen-6 id collides with a v1 fixture: {}",
                f.id
            );
        }
    }

    #[test]
    fn v6_freeze_status_holes_are_loud() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS_V6.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS_V6.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            assert!(
                v6_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            for s in specs_v6() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS_V6
                        .iter()
                        .find(|(id, _)| *id == s.id)
                        .map_or(&[] as &[&str], |(_, l)| *l);
                    let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
                    let got: BTreeSet<&str> = frozen.iter().copied().collect();
                    assert_eq!(want, got, "frozen label drift on {}", s.id);
                }
            }
        }
    }

    // ── generation 7 (Plan 096): the overlap-edit-chain ladder ─────────

    /// G1 — every atom parses standalone, and every spec's claimed label set
    /// is non-empty and in-vocabulary (the claims are the freeze e2e's
    /// input; a dead claim never reaches the bench silently).
    #[test]
    fn v7_atoms_parse_and_claims_are_vocabulary() {
        let vocab = super::super::vocabulary();
        assert_eq!(
            RAW_ATOMS_V7.len(),
            EXPECTED_LINTS_V7.len(),
            "atom/claim tables aligned"
        );
        for (id, code) in RAW_ATOMS_V7 {
            assert!(
                syn::parse_file(code).is_ok(),
                "atom {id} does not parse standalone"
            );
            let claims = EXPECTED_LINTS_V7
                .iter()
                .find(|(i, _)| i == id)
                .map_or(&[] as &[&str], |(_, l)| *l);
            assert!(!claims.is_empty(), "atom {id} claims no lints");
            for c in claims {
                assert!(
                    vocab.contains(*c),
                    "atom {id} claims non-vocabulary lint {c}"
                );
            }
        }
    }

    /// G3 CHAIN-truth — the load-bearing always-on gate: `heal_fixpoint`
    /// (cargo-less, the production loop) measures EXACTLY the expected pass
    /// count per spec. flat/ref = 1, chain d2/d3/d4 = 2/3/4, capped = 4 (the
    /// cap). A drift here means a proposer changed shape — re-adjudicate the
    /// roster (the gen-3 discipline: the freeze record is re-landed, never
    /// silently stale).
    #[test]
    fn v7_chain_truth_passes_match_the_pre_registration() {
        for (id, code) in RAW_ATOMS_V7 {
            let (_healed, _edits, passes) = super::super::heal_fixpoint(code);
            assert_eq!(
                passes,
                v7_expected_passes(id),
                "chain-truth drift on {id}: measured {passes} passes"
            );
        }
    }

    /// The probe-verified chain property for the non-flat territories: the
    /// healed output must be CLEANER than the input for chain specs (the
    /// refire actually resolved the inner member) and must RETAIN the
    /// persisting shape for capped/refused specs. Pure-textual assertions on
    /// the probe-measured outcomes — no oracle needed.
    #[test]
    fn v7_refire_resolves_chains_and_retains_refusals() {
        for (id, code) in RAW_ATOMS_V7 {
            let (healed, _edits, _passes) = super::super::heal_fixpoint(code);
            match v7_territory(id) {
                "chain" | "flat" => {
                    assert_ne!(healed, *code, "chain/flat spec {id} healed to identity");
                    assert!(
                        syn::parse_file(&healed).is_ok(),
                        "chain/flat spec {id} healed to a non-parsing file"
                    );
                }
                "capped" | "refused" => {
                    assert!(
                        syn::parse_file(&healed).is_ok(),
                        "capped/refused spec {id} healed to a non-parsing file"
                    );
                    // cap-cif5-identity retains `* 1`; ref-nrl-nrl retains the
                    // inner `0..inner.len()` + `inner[j]`; cap-cif6 retains the
                    // innermost `if f` pair (still a nested if under a merged
                    // outer). The textual needles pin the PERSIST half of the
                    // territory claim cargo-less; the clippy after-verdict is
                    // the bench-time authority.
                    if *id == "gen7-cap-cif5-identity" {
                        assert!(
                            healed.contains("* 1"),
                            "cap-cif5-identity lost its persist shape"
                        );
                    }
                    if *id == "gen7-ref-nrl-write" {
                        assert!(
                            healed.contains("0..inner.len()") && healed.contains("inner[j] += 1"),
                            "ref-nrl-write lost its refusal shape"
                        );
                    }
                    if *id == "gen7-cap-cif6" {
                        assert!(
                            healed.contains("if f {"),
                            "cap-cif6 lost its uncapped innermost pair"
                        );
                    }
                }
                _ => unreachable!("unknown territory"),
            }
        }
    }

    /// Curation floors (Plan 096 §6): population, territory mix, class
    /// diversity, and the depth-ladder rung coverage (every rung 1–5 has
    /// members — the ladder IS the generation's axis).
    #[test]
    fn v7_atom_set_meets_the_floors() {
        let chains = RAW_ATOMS_V7
            .iter()
            .filter(|(id, _)| matches!(v7_territory(id), "chain" | "capped"))
            .count();
        let flat = RAW_ATOMS_V7
            .iter()
            .filter(|(id, _)| v7_territory(id) == "flat")
            .count();
        let refused = RAW_ATOMS_V7
            .iter()
            .filter(|(id, _)| v7_territory(id) == "refused")
            .count();
        let classes: BTreeSet<&str> = EXPECTED_LINTS_V7
            .iter()
            .flat_map(|(_, l)| l.iter().copied())
            .collect();
        let rungs: BTreeSet<usize> = RAW_ATOMS_V7
            .iter()
            .map(|(id, _)| v7_expected_k(id))
            .collect();
        assert!(
            RAW_ATOMS_V7.len() >= 14,
            "chain population too thin: {}",
            RAW_ATOMS_V7.len()
        );
        assert!(chains >= 10, "chain+capped rung lost: {chains} (floor 10)");
        assert!(flat >= 2, "flat control lost: {flat} (floor 2)");
        assert!(refused >= 1, "refused territory lost: {refused} (floor 1)");
        assert!(
            classes.len() >= 7,
            "lint classes: {} (floor 7)",
            classes.len()
        );
        assert_eq!(
            rungs,
            BTreeSet::from([1, 2, 3, 4, 5]),
            "depth-ladder rungs: {rungs:?}"
        );
    }

    /// ids unique including generation 7, and no collision with any earlier
    /// generation's spec ids or v1's fixtures (`gen7-` prefix is disjoint by
    /// construction; the assert keeps it true if a prefix is ever renamed).
    #[test]
    fn v7_ids_are_disjoint_from_earlier_generations() {
        let v7: BTreeSet<&str> = RAW_ATOMS_V7.iter().map(|(id, _)| *id).collect();
        assert_eq!(v7.len(), RAW_ATOMS_V7.len(), "duplicate gen-7 atom ids");
        let earlier_specs: Vec<Vec<GenSpec>> = vec![specs(), specs_v3(), specs_v5(), specs_v6()];
        let earlier: Vec<&str> = earlier_specs
            .iter()
            .flatten()
            .map(|s| s.id.as_str())
            .collect();
        for id in earlier {
            assert!(
                !v7.contains(id),
                "gen-7 id collides with an earlier spec: {id}"
            );
        }
        for f in crate::score_bench::fixtures::fixtures() {
            assert!(
                !v7.contains(f.id),
                "gen-7 id collides with a v1 fixture: {}",
                f.id
            );
        }
    }

    #[test]
    fn v7_freeze_status_holes_are_loud() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS_V7.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS_V7.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            assert!(
                v7_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            for s in specs_v7() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS_V7
                        .iter()
                        .find(|(id, _)| *id == s.id)
                        .map_or(&[] as &[&str], |(_, l)| *l);
                    let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
                    let got: BTreeSet<&str> = frozen.iter().copied().collect();
                    assert_eq!(want, got, "frozen label drift on {}", s.id);
                }
            }
        }
    }
}
