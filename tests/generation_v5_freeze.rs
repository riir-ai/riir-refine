//! Plan 094 — the generation-v5 freeze/verify e2e (the rust_perf vocabulary
//! ladder).
//!
//! Two modes, one file:
//!
//! - **Freeze** (`--ignored`): runs the PRE-REGISTERED freeze gates
//!   (`.plans/094` §4) over every spec — the parse gate, the
//!   marker-discrimination fence (persist predicate TRUE on buggy, FALSE on
//!   fixed, per corpus atom), the composite-validity fence (each member's
//!   predicate TRUE in its own pre-heal region), the persist-claim
//!   stratification gate (the production heal loop leaves every persist
//!   atom's shape in place), and the drift gate against the landed record —
//!   then prints the paste-ready freeze-record literals. CARGO-LESS: the
//!   whole freeze is pure textual + syn, milliseconds (gens 2-4 spawned a
//!   real oracle per spec; gen 5's oracle is the bench-owned mechanical
//!   shape detector set, which has no external existence to spawn).
//!
//! - **Always-on** (default `cargo test`): the freeze record's coverage
//!   invariants + the discrimination/stratification fences live as
//!   always-on module tests in `generations.rs`; this file adds the
//!   whole-file persist-table sanity checks.
//!
//! # Pre-registered gates (authored BEFORE the first freeze run)
//!
//! NO solve gate on addressable atoms and NO k-decay gate (the gens 2/3/4
//! pre-registration: solve behavior is the BENCH's measurement; the M5
//! admission gate carries non-vacuity). The binding checks are the
//! stratification gate (persist claims hold) and the M5 admission gate
//! (CLI: the aggregate rate strictly below v1's latest).
//!
//! ```bash
//! cargo test --release --features score_bench \
//!     --test generation_v5_freeze -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(feature = "score_bench")]

use riir_refine::score_bench::generations;

#[test]
#[ignore = "runs the freeze gates + prints the paste-ready record; run with --ignored --test-threads=1"]
fn freeze_and_verify_generation_v5() {
    let atom_list = generations::v5_atoms();
    assert!(!atom_list.is_empty(), "atom factory produced nothing");
    let persist_ids: std::collections::BTreeSet<&str> =
        generations::PERSIST_ATOMS_V5.iter().map(|p| p.id).collect();

    // ── G1 + G2: parse gate + the marker-discrimination fence ───────────
    let corpus = riir_refine::build_perf_seed_corpus();
    let fixed_by_rule: std::collections::BTreeMap<&str, &str> = corpus
        .entries
        .iter()
        .map(|e| (e.rule_id.as_str(), e.fixed_code.as_str()))
        .collect();
    let mut bad_atoms: Vec<String> = Vec::new();
    for atom in &atom_list {
        let rule = &atom.expect[0];
        if syn::parse_file(&atom.code).is_err() {
            println!("⚠ atom {} does not parse standalone — excluding", atom.id);
            bad_atoms.push(atom.id.clone());
            continue;
        }
        if !generations::v5_persists(rule, &atom.code) {
            println!(
                "⚠ atom {}: marker for {rule} absent from its own buggy code — excluding",
                atom.id
            );
            bad_atoms.push(atom.id.clone());
            continue;
        }
        if persist_ids.contains(atom.id.as_str()) {
            continue; // persist atoms have no fixed_code by definition
        }
        let Some(fixed) = fixed_by_rule.get(rule.as_str()) else {
            println!(
                "⚠ atom {}: rule {rule} missing from the corpus — excluding",
                atom.id
            );
            bad_atoms.push(atom.id.clone());
            continue;
        };
        if generations::v5_persists(rule, fixed) {
            println!(
                "⚠ atom {}: marker for {rule} SURVIVES its own fixed_code (falsified claim) — excluding",
                atom.id
            );
            bad_atoms.push(atom.id.clone());
        }
    }
    if !bad_atoms.is_empty() {
        println!("⚠ atoms that failed freeze verification (exclude from FROZEN_LABELS_V5): {bad_atoms:?}");
    }
    let verified_ids: std::collections::BTreeSet<String> = atom_list
        .iter()
        .filter(|a| !bad_atoms.contains(&a.id))
        .map(|a| a.id.clone())
        .collect();

    // ── G3: the persist-claim stratification gate (the production heal) ─
    let mut strat_violations: Vec<String> = Vec::new();
    for p in generations::PERSIST_ATOMS_V5 {
        let rule = p.expect[0];
        let (healed, _edits, _passes) = riir_refine::score_bench::heal_fixpoint(p.code);
        if syn::parse_file(&healed).is_err() {
            strat_violations.push(format!("{} healed to a non-parsing file", p.id));
            continue;
        }
        if !generations::v5_persists(rule, &healed) {
            strat_violations.push(format!(
                "{p_id} RESOLVES {rule} through the production heal — the divergence class closed; reshape or drop the atom and re-freeze",
                p_id = p.id
            ));
        }
    }
    println!(
        "\nstratification gate (every persist atom resolves 0 through the production heal): {}",
        if strat_violations.is_empty() {
            "PASS"
        } else {
            "FAIL"
        }
    );
    for v in &strat_violations {
        println!("  VIOLATION: {v}");
    }

    // ── G4: the composite-validity fence + drift gate over every spec ───
    let specs = generations::specs_v5();
    let excluded: std::collections::BTreeSet<&str> =
        generations::EXCLUDED_IDS_V5.iter().copied().collect();
    let frozen: std::collections::BTreeMap<&str, &[&str]> = generations::FROZEN_LABELS_V5
        .iter()
        .map(|(id, l)| (*id, *l))
        .collect();
    let mut fresh_exclusions: Vec<String> = Vec::new();
    let mut drift: Vec<String> = Vec::new();
    let mut verified_specs: Vec<(String, usize)> = Vec::new();
    for s in &specs {
        if excluded.contains(s.id.as_str()) {
            continue;
        }
        let constituent_ids: Vec<&str> =
            s.id.strip_prefix("gen5-k")
                .and_then(|rest| rest.split_once('-'))
                .map(|(_, ids)| ids.split('+').collect())
                .unwrap_or_default();
        if constituent_ids.iter().any(|id| !verified_ids.contains(*id)) {
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        if syn::parse_file(&s.code).is_err() {
            println!("⚠ spec {} does not parse — excluding", s.id);
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        // Each member's predicate TRUE in its own pre-heal region (the
        // composition must not destroy or shadow a member's shape).
        let mut member_ok = true;
        for i in 0..s.k {
            let Some(region) = generations::v5_module_region(&s.code, i) else {
                println!(
                    "⚠ spec {}: region gen_{i} not extractable — excluding",
                    s.id
                );
                member_ok = false;
                break;
            };
            let Some(members) = generations::v5_member_rules(&s.id) else {
                panic!("member rules unresolvable for {}", s.id);
            };
            if !generations::v5_persists(&members[i], &region) {
                println!(
                    "⚠ spec {}: member {i} ({}) marker missing from its own region — excluding",
                    s.id, members[i]
                );
                member_ok = false;
                break;
            }
        }
        if !member_ok {
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        // Frozen-label drift (labels already landed → they must still match).
        if let Some(l) = frozen.get(s.id.as_str()) {
            let have: std::collections::BTreeSet<&str> = l.iter().copied().collect();
            let want: std::collections::BTreeSet<&str> =
                s.expect_lints.iter().map(String::as_str).collect();
            if have != want {
                drift.push(s.id.clone());
            }
        }
        verified_specs.push((s.id.clone(), s.k));
    }

    if !drift.is_empty() {
        panic!("FROZEN_LABELS_V5 drifted from the fresh derivation on: {drift:?} — re-freeze");
    }

    // ── Report + the paste-ready freeze record ─────────────────────────
    println!("\n─ generation-v5 freeze report ────────────────────");
    println!(
        "atoms: {} ({} persist + {} corpus; bad: {})",
        atom_list.len(),
        persist_ids.len(),
        atom_list.len() - persist_ids.len(),
        bad_atoms.len()
    );
    println!(
        "specs: {} verified, {} newly excluded, {} pre-excluded",
        verified_specs.len(),
        fresh_exclusions.len(),
        excluded.len()
    );
    println!(
        "pre-registered: NO solve gate on addressable atoms, NO k-decay gate (Plan 094 §4) — \
         solve behavior is the BENCH's measurement."
    );
    let mut per_territory: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for a in &atom_list {
        if verified_ids.contains(&a.id) {
            *per_territory
                .entry(generations::v5_territory(&a.id))
                .or_default() += 1;
        }
    }
    for (terr, n) in &per_territory {
        println!("  territory {terr}: {n} verified atom(s)");
    }

    let mut all_exclusions: Vec<String> = fresh_exclusions.clone();
    all_exclusions.extend(bad_atoms.iter().map(|id| format!("gen5-k1-{id}")));
    for id in excluded.iter() {
        all_exclusions.push((*id).to_string());
    }
    all_exclusions.sort();
    all_exclusions.dedup();
    println!("\n// ── freeze record (paste into generations.rs) ──");
    println!("pub static EXCLUDED_IDS_V5: &[&str] = &[");
    for id in &all_exclusions {
        println!("    \"{id}\",");
    }
    println!("];");
    println!("pub static FROZEN_LABELS_V5: &[(&str, &[&str])] = &[");
    verified_specs.sort();
    for (id, k) in &verified_specs {
        let members = generations::v5_member_rules(id).unwrap_or_default();
        let mut distinct: Vec<String> = members.clone();
        distinct.sort();
        distinct.dedup();
        println!(
            "    (\"{id}\", &[{}]),  // k={k}",
            distinct
                .iter()
                .map(|l| format!("\"{l}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("];");

    // The gate IS the test: a stratification violation fails the run so the
    // finding is loud (never a silent green over a closed divergence class).
    assert!(
        strat_violations.is_empty(),
        "stratification violated — a persist shape resolved: {strat_violations:?}"
    );
}

#[test]
fn persist_table_joins_the_perf_vocabulary_and_parses() {
    // The whole-file always-on twin of the module gates.
    let fixable = riir_refine::domains::rust_perf::proposers::FIXABLE_RULES;
    let vocab: std::collections::BTreeSet<&str> = fixable.iter().copied().collect();
    assert!(
        generations::PERSIST_ATOMS_V5.len() >= 3,
        "persist table too thin to stratify: {}",
        generations::PERSIST_ATOMS_V5.len()
    );
    for p in generations::PERSIST_ATOMS_V5 {
        assert!(
            !vocab.contains(p.id),
            "persist id {} collides with a rule id",
            p.id
        );
        for l in p.expect {
            assert!(
                vocab.contains(l),
                "persist atom {} expects non-vocabulary rule {l}",
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
