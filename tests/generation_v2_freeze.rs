//! Issue 072 T5/T6 (Plan 092) — the generation-v2 freeze/verify e2e.
//!
//! Two modes, one file:
//!
//! - **Freeze + measure** (`--ignored`, spawns real clippy via the
//!   `score_bench` oracle): label-verifies every committed v2 fixture (the
//!   G1 construction-vs-oracle fence — an interference composite whose
//!   real labels disagree with the union-of-atoms gold gets EXCLUDED from
//!   the generation, never scored), then measures per-k solve rates and
//!   checks the M4 monotone-decay gate. Run:
//!   `cargo test --features score_bench --test generation_v2_freeze -- --ignored --test-threads=1 --nocapture`
//!
//! - **Always-on** (default `cargo test`): the freeze record's coverage
//!   invariants live in `generations.rs` module tests; this file adds the
//!   whole-file checks (atom table sanity vs the bench vocabulary).

#![cfg(feature = "score_bench")]

use riir_clippy::score_bench::{draft_oracle, generations, wrap};

#[test]
#[ignore = "spawns real cargo+clippy (the freeze oracle); run with --ignored --test-threads=1"]
fn freeze_and_measure_generation_v2() {
    let oracle = draft_oracle();
    assert!(
        oracle.is_available(),
        "cargo not on PATH — cannot run the freeze oracle"
    );

    let atom_list = generations::atoms();
    assert!(!atom_list.is_empty(), "atom factory produced nothing");

    // ── Pass 1: verify every ATOM fires exactly its own lint ────────────
    let mut bad_atoms: Vec<String> = Vec::new();
    for atom in &atom_list {
        let out = oracle.lints_for(&wrap(&atom.code));
        let vocab: std::collections::BTreeSet<String> = out
            .lints
            .intersection(&riir_clippy::score_bench::vocabulary())
            .cloned()
            .collect();
        let want: std::collections::BTreeSet<String> = std::iter::once(atom.lint.clone()).collect();
        if !out.ran || !out.compiles || vocab != want {
            bad_atoms.push(atom.lint.clone());
        }
    }
    if !bad_atoms.is_empty() {
        println!(
            "⚠ atoms that failed freeze verification (exclude from FROZEN_LABELS): {bad_atoms:?}"
        );
    }

    // ── Pass 2: label-verify every composite against the union gold ─────
    let specs = generations::specs();
    let excluded: std::collections::BTreeSet<&str> =
        generations::EXCLUDED_IDS.iter().copied().collect();
    let frozen: std::collections::BTreeMap<&str, &[&str]> = generations::FROZEN_LABELS
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
        String,                             // healed code
        usize,                              // vocab_before
        usize,                              // resolved
    )> = Vec::new();
    for s in &specs {
        if excluded.contains(s.id.as_str()) {
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
        // The M4 solve measurement: heal, re-label, count per-lint resolution.
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
        labeled_specs.push((s.id.clone(), s.k, want, healed, vocab_before, resolved));
    }

    if !drift.is_empty() {
        panic!("FROZEN_LABELS drifted from the fresh oracle on: {drift:?} — re-freeze");
    }

    // ── Pass 3: the per-k solve-rate table + the M4 decay gate ──────────
    let mut agg: std::collections::BTreeMap<usize, (usize, usize)> =
        std::collections::BTreeMap::new();
    for (_id, k, _want, _healed, before, resolved) in &labeled_specs {
        let e = agg.entry(*k).or_default();
        e.0 += resolved;
        e.1 += before;
    }
    println!("\n─ generation-v2 freeze report ────────────────────────");
    println!("atoms: {} (bad: {})", atom_list.len(), bad_atoms.len());
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

    // M4 gate: solve-rate monotone NON-INCREASING in k (composition is the
    // difficulty axis). A violation is branch (B) material — record it, do
    // not fudge the ladder: the spec's own fallback re-sorts empirically.
    let mut violations: Vec<String> = Vec::new();
    for pair in rates.windows(2) {
        let (k_lo, r_lo) = pair[0];
        let (k_hi, r_hi) = pair[1];
        if r_hi > r_lo + 1e-9 {
            violations.push(format!(
                "k={k_hi} solve {r_hi:.1}% > k={k_lo} solve {r_lo:.1}%"
            ));
        }
    }
    println!(
        "\nM4 monotone-decay gate: {}",
        if violations.is_empty() {
            "PASS"
        } else {
            "FAIL"
        }
    );
    for v in &violations {
        println!("  VIOLATION: {v}");
    }
    println!(
        "\nPre-registered branch: {}",
        if violations.is_empty() {
            "(A) decay holds — `--score-bench --generation 2` may admit the ladder"
        } else {
            "(B) flat/rising — the admission gate will refuse v2; the difficulty axis needs persist-shape stratification or interference composition (record in .benchmarks/069)"
        }
    );

    // ── Pass 4: emit the freeze record literal (paste into FROZEN_LABELS) ─
    // The frozen label of a verified spec IS its union gold (pass 2 proved
    // the equality against real clippy); the exclusions ride EXCLUDED_IDS.
    let mut all_exclusions: Vec<String> = fresh_exclusions.clone();
    for id in excluded.iter() {
        all_exclusions.push((*id).to_string());
    }
    all_exclusions.sort();
    println!("\n// ── freeze record (paste into generations.rs) ──");
    println!("pub static EXCLUDED_IDS: &[&str] = &[");
    for id in &all_exclusions {
        println!("    \"{id}\",");
    }
    println!("];");
    println!("pub static FROZEN_LABELS: &[(&str, &[&str])] = &[");
    /// (id, k, oracle labels, healed code, vocab_before, resolved)
    type LabeledSpec = (
        String,
        usize,
        std::collections::BTreeSet<String>,
        String,
        usize,
        usize,
    );
    let mut labeled_sorted: Vec<&LabeledSpec> = labeled_specs.iter().collect();
    labeled_sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (id, k, want, _healed, _before, _resolved) in labeled_sorted {
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

    // The gate IS the test: a decay violation fails the run so the finding
    // is loud (never a silent green over a dead mechanism).
    assert!(
        violations.is_empty(),
        "M4 monotone decay violated: {violations:?}"
    );
}

#[test]
fn atom_table_joins_the_bench_vocabulary_and_parses() {
    // The whole-file always-on twin of the module gates: the generation is
    // benchable only when the atom table is non-trivial and vocabulary-clean.
    let atoms = generations::atoms();
    let vocab = riir_clippy::score_bench::vocabulary();
    assert!(
        atoms.len() >= 8,
        "atom table too thin for a k-ladder: {}",
        atoms.len()
    );
    for a in &atoms {
        assert!(
            vocab.contains(a.lint.as_str()),
            "{} not in vocabulary",
            a.lint
        );
    }
}
