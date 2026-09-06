//! Issue 072 T5/T6 (Plan 092) + Bench 069 reopen #1 — the generation-v3
//! freeze/verify e2e (persist-shape stratification).
//!
//! Two modes, one file:
//!
//! - **Freeze + measure** (`--ignored`, spawns real clippy via the
//!   `score_bench` oracle): label-verifies every committed v3 fixture (the
//!   construction-vs-oracle fence — an atom or composite whose real labels
//!   disagree with the declared gold gets EXCLUDED from the generation,
//!   never scored), then measures per-k solve rates and checks the
//!   PRE-REGISTERED STRATIFICATION GATE. Run:
//!   `cargo test --features score_bench --test generation_v3_freeze -- --ignored --test-threads=1 --nocapture`
//!
//! - **Always-on** (default `cargo test`): the freeze record's coverage
//!   invariants live in `generations.rs` module tests; this file adds the
//!   whole-file checks (persist table sanity vs the bench vocabulary).
//!
//! # Pre-registered gate (authored BEFORE the first freeze run)
//!
//! The v2 M4 monotone-k-decay gate does NOT carry to generation 3. Its
//! difficulty axis is per-atom strata (persist shapes vs corpus shapes),
//! not k-composition — the corpus half is solvable at every k by
//! construction, so k can only shift the persist-atom sampling fraction,
//! and the prefix-window geometry makes strict monotonicity a sampling
//! artifact, not a signal. The binding checks are:
//!
//! 1. **Stratification gate** — every persist atom resolves 0 on its own
//!    k=1 rung (the heal lane must stay silent on exactly the shapes it is
//!    documented to refuse; a resolution means the divergence class closed
//!    and the generation must be re-frozen as a new number).
//! 2. **The M5 admission gate** (CLI, re-armed generalized) — the
//!    aggregate v3 heal rate must measure strictly below v1's latest row.

#![cfg(feature = "score_bench")]

use riir_clippy::score_bench::{draft_oracle, generations, wrap};

#[test]
#[ignore = "spawns real cargo+clippy (the freeze oracle); run with --ignored --test-threads=1"]
fn freeze_and_measure_generation_v3() {
    let oracle = draft_oracle();
    assert!(
        oracle.is_available(),
        "cargo not on PATH — cannot run the freeze oracle"
    );

    let atom_list = generations::v3_atoms();
    assert!(!atom_list.is_empty(), "atom factory produced nothing");
    let persist_ids: std::collections::BTreeSet<&str> =
        generations::PERSIST_ATOMS.iter().map(|p| p.id).collect();

    // ── Pass 1: verify every ATOM fires exactly its declared gold ───────
    let mut bad_atoms: Vec<String> = Vec::new();
    for atom in &atom_list {
        let out = oracle.lints_for(&wrap(&atom.code));
        let vocab: std::collections::BTreeSet<String> = out
            .lints
            .intersection(&riir_clippy::score_bench::vocabulary())
            .cloned()
            .collect();
        let want: std::collections::BTreeSet<String> = atom.expect.iter().cloned().collect();
        if !out.ran || !out.compiles || vocab != want {
            bad_atoms.push(atom.id.clone());
        }
    }
    if !bad_atoms.is_empty() {
        println!("⚠ atoms that failed freeze verification (exclude from FROZEN_LABELS_V3): {bad_atoms:?}");
    }
    let verified_ids: std::collections::BTreeSet<String> = atom_list
        .iter()
        .filter(|a| !bad_atoms.contains(&a.id))
        .map(|a| a.id.clone())
        .collect();

    // ── Pass 2: label-verify every composite against the union gold ─────
    let specs = generations::specs_v3();
    let excluded: std::collections::BTreeSet<&str> =
        generations::EXCLUDED_IDS_V3.iter().copied().collect();
    let frozen: std::collections::BTreeMap<&str, &[&str]> = generations::FROZEN_LABELS_V3
        .iter()
        .map(|(id, l)| (*id, *l))
        .collect();
    let vocab = riir_clippy::score_bench::vocabulary();
    let mut fresh_exclusions: Vec<String> = Vec::new();
    let mut drift: Vec<String> = Vec::new();
    let mut labeled_specs: Vec<(
        String,                             // id
        usize,                              // k
        std::collections::BTreeSet<String>, // oracle labels
        usize,                              // vocab_before
        usize,                              // resolved
    )> = Vec::new();
    for s in &specs {
        if excluded.contains(s.id.as_str()) {
            continue;
        }
        // A composite of an excluded atom is excluded by cascade (its gold
        // is no longer derivable from verified atoms).
        let constituent_ids: Vec<&str> =
            s.id.strip_prefix("gen3-k")
                .and_then(|rest| rest.split_once('-'))
                .map(|(_, ids)| ids.split('+').collect())
                .unwrap_or_default();
        if constituent_ids.iter().any(|id| !verified_ids.contains(*id)) {
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        let out = oracle.lints_for(&wrap(&s.code));
        if !out.ran || !out.compiles {
            println!("⚠ spec {} did not compile — excluding", s.id);
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        let got: std::collections::BTreeSet<String> =
            out.lints.intersection(&vocab).cloned().collect();
        let want: std::collections::BTreeSet<String> = s.expect_lints.iter().cloned().collect();
        if got != want {
            println!(
                "⚠ spec {} union-identity FAILS (oracle {got:?} ≠ union {want:?}) — excluding (interference composite)",
                s.id
            );
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        // Frozen-label drift (labels already landed → they must still match).
        if let Some(l) = frozen.get(s.id.as_str()) {
            let have: std::collections::BTreeSet<&str> = l.iter().copied().collect();
            let got_ref: std::collections::BTreeSet<&str> =
                got.iter().map(String::as_str).collect();
            if have != got_ref {
                drift.push(s.id.clone());
            }
        }
        // The solve measurement: heal, re-label, count per-lint resolution.
        let (healed, _edits, _passes) = riir_clippy::score_bench::heal_fixpoint(&s.code);
        let after = oracle.lints_for(&wrap(&healed));
        let after_ok = after.ran && after.compiles;
        let vocab_before = want.len();
        let resolved = if after_ok {
            want.difference(
                &after
                    .lints
                    .intersection(&vocab)
                    .cloned()
                    .collect::<std::collections::BTreeSet<String>>(),
            )
            .count()
        } else {
            0 // a non-compiling heal scores as NOTHING resolved
        };
        labeled_specs.push((s.id.clone(), s.k, want, vocab_before, resolved));
    }

    if !drift.is_empty() {
        panic!("FROZEN_LABELS_V3 drifted from the fresh oracle on: {drift:?} — re-freeze");
    }

    // ── Pass 3: the per-k solve-rate table + the stratification gate ────
    let mut agg: std::collections::BTreeMap<usize, (usize, usize)> =
        std::collections::BTreeMap::new();
    for (_id, k, _want, before, resolved) in &labeled_specs {
        let e = agg.entry(*k).or_default();
        e.0 += resolved;
        e.1 += before;
    }
    println!("\n─ generation-v3 freeze report ────────────────────────");
    println!(
        "atoms: {} ({} persist + {} corpus; bad: {})",
        atom_list.len(),
        persist_ids.len(),
        atom_list.len() - persist_ids.len(),
        bad_atoms.len()
    );
    println!(
        "specs: {} verified, {} newly excluded, {} pre-excluded",
        labeled_specs.len(),
        fresh_exclusions.len(),
        excluded.len()
    );
    println!("\nper-k solve rates (resolved vocabulary lints / before):");
    let mut rates: Vec<(usize, f64)> = Vec::new();
    for (k, (resolved, before)) in &agg {
        if *before == 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)]
        let pct = (*resolved as f64) / (*before as f64) * 100.0;
        println!("  k={k}: {resolved}/{before} = {pct:.1}%");
        rates.push((*k, pct));
    }
    rates.sort_by_key(|(k, _)| *k);
    println!(
        "\npre-registered: the v2 M4 monotone-k-decay gate does NOT carry to gen 3 \
         (the axis is per-atom strata, not k-composition) — the table above is \
         reported, not asserted."
    );

    // THE STRATIFICATION GATE: every persist atom resolves 0 on its own
    // k=1 rung. A resolution means the documented divergence class closed —
    // re-freeze as a new generation number, never silently.
    let mut strat_violations: Vec<String> = Vec::new();
    for (id, k, want, _before, resolved) in &labeled_specs {
        if *k != 1 {
            continue;
        }
        let atom_id = id.strip_prefix("gen3-k1-").unwrap_or(id);
        if persist_ids.contains(atom_id) && *resolved > 0 {
            strat_violations.push(format!("{atom_id} resolved {resolved} lints of {want:?}"));
        }
    }
    println!(
        "\nstratification gate (every persist atom resolves 0): {}",
        if strat_violations.is_empty() {
            "PASS"
        } else {
            "FAIL"
        }
    );
    for v in &strat_violations {
        println!("  VIOLATION: {v}");
    }

    // ── Pass 4: emit the freeze record literal (paste into generations.rs) ─
    let mut all_exclusions: Vec<String> = fresh_exclusions.clone();
    all_exclusions.extend(bad_atoms.iter().map(|id| format!("gen3-k1-{id}")));
    for id in excluded.iter() {
        all_exclusions.push((*id).to_string());
    }
    all_exclusions.sort();
    all_exclusions.dedup();
    println!("\n// ── freeze record (paste into generations.rs) ──");
    println!("pub static EXCLUDED_IDS_V3: &[&str] = &[");
    for id in &all_exclusions {
        println!("    \"{id}\",");
    }
    println!("];");
    println!("pub static FROZEN_LABELS_V3: &[(&str, &[&str])] = &[");
    labeled_specs.sort_by(|a, b| a.0.cmp(&b.0));
    for (id, k, want, _before, _resolved) in &labeled_specs {
        let lints: Vec<String> = want.iter().cloned().collect();
        println!(
            "    (\"{id}\", &[{}]),  // k={k}",
            lints
                .iter()
                .map(|l| format!("\"{l}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("];",);

    // The gate IS the test: a stratification violation fails the run so the
    // finding is loud (never a silent green over a closed divergence class).
    assert!(
        strat_violations.is_empty(),
        "stratification violated — a persist shape resolved: {strat_violations:?}"
    );
}

#[test]
fn persist_table_joins_the_bench_vocabulary_and_parses() {
    // The whole-file always-on twin of the module gates.
    let vocab = riir_clippy::score_bench::vocabulary();
    assert!(
        generations::PERSIST_ATOMS.len() >= 3,
        "persist table too thin to stratify: {}",
        generations::PERSIST_ATOMS.len()
    );
    for p in generations::PERSIST_ATOMS {
        assert!(
            !vocab.contains(p.id),
            "persist id {} collides with a lint name",
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
    }
}
