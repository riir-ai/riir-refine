//! Generation-4 freeze e2e (Plan 093) — the rustc-errors-side ladder.
//!
//! The rustc analog of the clippy label fence: each non-excluded spec's
//! composed `broken` file must produce, on ONE real `cargo check`, exactly
//! the union of its member error codes (set equality) with count == k (two
//! defects must not merge, shadow, or cascade into a different code — the
//! composition-interference class gets EXCLUDED, never pinned). The drift
//! gate panics when a landed `FROZEN_LABELS_V4` entry stops matching the
//! fresh oracle. The always-on pass pins the table's structural invariants
//! (family coverage, territory mix per rung) without cargo.
//!
//! Per Plan 093's pre-registration there is NO solve-based gate here and NO
//! k-decay gate: solve behavior is the BENCH's measurement (the driver
//! never runs in the freeze — one check per spec, minutes not tens of
//! minutes), and the per-k difficulty table is REPORTED, never asserted.
//!
//! ```bash
//! cargo test --release --features score_bench,rustc_errors \
//!     --test generation_v4_freeze -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(all(feature = "score_bench", feature = "rustc_errors"))]

use std::collections::{BTreeMap, BTreeSet};

use riir_refine::domains::rustc_errors::composite;
use riir_refine::score_bench::generations as gens;

/// The landed freeze record drift check: fresh labels for a landed spec must
/// equal the frozen table exactly (set AND count).
#[test]
#[ignore = "spawns real cargo per spec (the freeze oracle); run with --ignored --test-threads=1"]
fn freeze_and_verify_generation_v4_labels() {
    let specs = gens::specs_v4();
    assert!(!specs.is_empty(), "gen4 ladder empty");

    let work_root = std::env::temp_dir().join(format!("riir_gen4_freeze_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&work_root);

    let mut labeled: Vec<(String, usize, BTreeSet<String>)> = Vec::new();
    let mut fresh_exclusions: Vec<String> = Vec::new();
    let mut drift: Vec<String> = Vec::new();
    // Per-k label-table counters (reported; no decay assertion — Plan 093).
    let mut per_k: BTreeMap<usize, (usize, usize)> = BTreeMap::new(); // k -> (ok, total)
                                                                      // Family + territory coverage over verified specs.
    let mut family_specs: BTreeMap<String, usize> = BTreeMap::new();
    let mut terr_by_k: BTreeMap<usize, BTreeSet<&'static str>> = BTreeMap::new();

    for spec in &specs {
        let Some(codes) = gens::v4_member_codes(&spec.id) else {
            panic!("member codes unresolvable for {}", spec.id);
        };
        let want: BTreeSet<String> = codes.iter().cloned().collect();
        per_k.entry(spec.k).or_insert((0, 0)).1 += 1;
        let observed = composite::check_composite_labels(&spec.id, &spec.code, &work_root);
        let Some(observed) = observed else {
            fresh_exclusions.push(spec.id.clone());
            eprintln!("  EXCLUDED (check could not run): {}", spec.id);
            continue;
        };
        let observed_set: BTreeSet<String> = observed.iter().map(|(c, _)| c.clone()).collect();
        let count_ok = observed.len() == spec.k;
        if observed_set != want || !count_ok {
            fresh_exclusions.push(spec.id.clone());
            eprintln!(
                "  EXCLUDED (interference): {} — want {want:?}, got {:?} (count {})",
                spec.id,
                observed_set,
                observed.len()
            );
            continue;
        }
        // Drift gate: a landed entry must match the fresh oracle exactly.
        if let Some((_, frozen)) = gens::FROZEN_LABELS_V4.iter().find(|(id, _)| *id == spec.id) {
            let frozen_set: BTreeSet<&str> = frozen.iter().copied().collect();
            let fresh: BTreeSet<&str> = want.iter().map(String::as_str).collect();
            if frozen_set != fresh {
                drift.push(format!(
                    "{}: frozen {frozen_set:?} vs fresh {fresh:?}",
                    spec.id
                ));
            }
        }
        let e = per_k.get_mut(&spec.k).expect("k entry inserted above");
        e.0 += 1;
        labeled.push((spec.id.clone(), spec.k, want.clone()));
        for code in &want {
            *family_specs.entry(code.clone()).or_insert(0) += 1;
            let terr = riir_refine::domains::rustc_errors::corpus_gen::territory_of(code);
            let terr_name = match terr {
                riir_refine::domains::rustc_errors::corpus_gen::Territory::Uncontested => {
                    "uncontested"
                }
                riir_refine::domains::rustc_errors::corpus_gen::Territory::Contested => "contested",
            };
            terr_by_k.entry(spec.k).or_default().insert(terr_name);
        }
        eprintln!("  ok: {}", spec.id);
    }

    // ── The freeze record gates ─────────────────────────────────────────
    // 1. Drift: landed entries must equal fresh labels (the panic IS the
    //    gate — a drift means the record is stale against this toolchain).
    assert!(
        drift.is_empty(),
        "FROZEN_LABELS_V4 drifted from the fresh oracle on: {drift:?} — re-freeze"
    );
    // 2. Coverage: every landed (labeled ∪ excluded) spec id must exist in
    //    the current ladder — an id that no longer resolves means the corpus
    //    moved and the record is stale ("re-run the freeze e2e" class).
    let ids: BTreeSet<&str> = specs.iter().map(|s| s.id.as_str()).collect();
    for id in gens::EXCLUDED_IDS_V4 {
        assert!(
            ids.contains(id),
            "landed exclusion {id} is not a current spec id — corpus moved, re-freeze"
        );
    }
    for (id, _) in gens::FROZEN_LABELS_V4 {
        assert!(
            ids.contains(id),
            "landed label {id} is not a current spec id — corpus moved, re-freeze"
        );
    }
    // 3. Family coverage: every expected code appears in ≥1 VERIFIED k≥2
    //    composite (the rustc analog of v3's persist-in-every-rung gate).
    let composite_codes: BTreeSet<String> = labeled
        .iter()
        .filter(|(_, k, _)| *k >= 2)
        .flat_map(|(_, _, want)| want.iter().cloned())
        .collect();
    for a in gens::v4_atoms() {
        assert!(
            composite_codes.contains(&a.expect[0]),
            "code {} absent from every verified gen4 composite — composition structurally broken",
            a.expect[0]
        );
    }
    // 4. Territory mix per composite rung (both territories verified at
    //    every k≥2 — the per-territory bench report must not be vacuous).
    for &k in &gens::LADDER_KS[1..] {
        let t = terr_by_k.get(&k).cloned().unwrap_or_default();
        assert!(
            t.contains("uncontested") && t.contains("contested"),
            "k={k} verified rungs lost a territory: {t:?}"
        );
    }

    // ── The report (per-k labels; solve behavior is the BENCH's job) ────
    println!(
        "\ngen-4 label fence: {}/{} specs verified",
        labeled.len(),
        specs.len()
    );
    let mut excluded_total = gens::EXCLUDED_IDS_V4.len();
    excluded_total += fresh_exclusions.len();
    println!(
        "exclusions: {excluded_total} (landed {} + fresh {})",
        gens::EXCLUDED_IDS_V4.len(),
        fresh_exclusions.len()
    );
    for (k, (ok, total)) in &per_k {
        println!("  k={k}: {ok}/{total} labeled");
    }
    println!("families (verified-spec appearances):");
    for (code, n) in &family_specs {
        println!("  {code:<8} x{n}");
    }

    // ── Emit the freeze record literal (paste into generations.rs) ──────
    let mut all_exclusions: Vec<String> = fresh_exclusions;
    all_exclusions.extend(gens::EXCLUDED_IDS_V4.iter().map(|s| s.to_string()));
    all_exclusions.sort();
    all_exclusions.dedup();
    println!("\n// ── freeze record (paste into generations.rs) ──");
    println!("pub static EXCLUDED_IDS_V4: &[&str] = &[");
    for id in &all_exclusions {
        println!("    \"{id}\",");
    }
    println!("];");
    println!("pub static FROZEN_LABELS_V4: &[(&str, &[&str])] = &[");
    labeled.sort_by(|a, b| a.0.cmp(&b.0));
    for (id, k, want) in &labeled {
        let codes: Vec<String> = want.iter().map(|c| format!("\"{c}\"")).collect();
        println!("    (\"{id}\", &[{}]),  // k={k}", codes.join(", "));
    }
    println!("];",);
}

/// Always-on: the table properties that need no cargo — the atom slice is
/// non-trivial, spec-count arithmetic holds, and the frozen record (when
/// landed) covers every spec with labels equal to the union gold. The
/// heavier per-spec label identity is the `--ignored` e2e above.
#[test]
#[cfg(all(feature = "score_bench", feature = "rustc_errors"))]
fn generation_v4_table_properties() {
    let specs = gens::specs_v4();
    let atoms = gens::v4_atoms();
    assert!(!atoms.is_empty(), "gen4 atom table empty");
    assert_eq!(
        specs.iter().filter(|s| s.k == 1).count(),
        atoms.len(),
        "k=1 rung must be every atom"
    );
    for s in &specs {
        assert!(
            s.k >= 1 && s.k <= 4,
            "gen4 spec {} has k out of range",
            s.id
        );
        assert!(
            s.code.matches("mod gen_").count() == s.k,
            "gen4 spec {} wraps the wrong member count",
            s.id
        );
    }
    // Landed-record coverage (dual-mode: pre-freeze both tables are empty
    // and this arm is vacuous — the module test in generations.rs asserts
    // the pre-freeze Err).
    if !gens::FROZEN_LABELS_V4.is_empty() || !gens::EXCLUDED_IDS_V4.is_empty() {
        let excluded: BTreeSet<&str> = gens::EXCLUDED_IDS_V4.iter().copied().collect();
        let labeled: BTreeSet<&str> = gens::FROZEN_LABELS_V4.iter().map(|(id, _)| *id).collect();
        for s in &specs {
            assert!(
                excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                "spec {} is neither labeled nor excluded",
                s.id
            );
        }
    }
}
