//! Issue 089 T7 — the generation-v8 freeze/verify e2e (the shaped-verdict
//! ladder: "target cleared ∧ new diags introduced").
//!
//! Two modes, one file (the v7 convention):
//!
//! - **Freeze** (`--ignored`): runs the PRE-REGISTERED gates over every
//!   atom with the BENCH instrument (real clippy, all+pedantic+nursery):
//!   the compile fence, the label fence (target claims, both directions),
//!   the INNOCENCE fence (claimed-created lints must NOT fire pre-heal —
//!   a created lint that already fires is a persist mislabeled as
//!   creation), the SHAPE gate (the heal must clear the target AND leave
//!   the claimed created diagnostics behind — the freeze's own verdict),
//!   and chain-truth (measured passes == pre-registered depth) — then
//!   prints the paste-ready freeze-record literals (labels, exclusions,
//!   AND the measured created table `V8_CREATED`).
//!
//! - **Always-on** (default `cargo test`): the parse fence, the innocence
//!   coherence, the chain-truth gate, the birth-marker canary and the
//!   freeze-record hole checks live as always-on module tests in
//!   `generations.rs`.
//!
//! A SHAPE-NEGATIVE outcome is a first-class freeze verdict: an atom whose
//! after-set has NO created lints (the shape dissolved — the Plan 097 T1
//! precedent) or whose target did not clear is EXCLUDED with the reason in
//! the printed record, never silently kept. The generation exists to
//! measure the half-win; an atom that cannot hold the shape measures
//! nothing at 0.5.
//!
//! ```bash
//! cargo test --release --features score_bench \
//!     --test generation_v8_freeze -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(feature = "score_bench")]

use std::collections::{BTreeMap, BTreeSet};

use riir_refine::score_bench::{draft_oracle, generations, wrap};

#[test]
#[ignore = "runs the freeze gates + prints the paste-ready record; run with --ignored --test-threads=1"]
fn freeze_and_verify_generation_v8() {
    let oracle = draft_oracle();
    assert!(
        oracle.is_available(),
        "cargo not on PATH — the freeze needs the real oracle"
    );

    let specs = generations::specs_v8();
    assert!(!specs.is_empty(), "atom table produced nothing");
    let excluded: BTreeSet<&str> = generations::EXCLUDED_IDS_V8.iter().copied().collect();
    let frozen: BTreeMap<&str, &[&str]> = generations::FROZEN_LABELS_V8
        .iter()
        .map(|(id, l)| (*id, *l))
        .collect();
    let frozen_created: BTreeMap<&str, &[&str]> = generations::V8_CREATED
        .iter()
        .map(|(id, l)| (*id, *l))
        .collect();
    let claims: BTreeMap<String, Vec<String>> =
        generations::v8_created_claims().into_iter().collect();

    let mut fresh_exclusions: Vec<String> = Vec::new();
    let mut drift: Vec<String> = Vec::new();
    let mut created_drift: Vec<String> = Vec::new();
    let mut verified: Vec<(String, Vec<String>)> = Vec::new();
    let mut verified_created: Vec<(String, Vec<String>)> = Vec::new();
    let mut relabeled: Vec<String> = Vec::new();

    for s in &specs {
        if excluded.contains(s.id.as_str()) {
            continue;
        }
        // ── G1: the compile fence ──────────────────────────────────────
        let before = oracle.lints_for(&wrap(&s.code));
        if !before.ran {
            println!(
                "⚠ spec {}: oracle did not run ({:?}) — excluding",
                s.id, before.error
            );
            fresh_exclusions.push(format!("{}  # oracle missing", s.id));
            continue;
        }
        if !before.compiles {
            println!(
                "⚠ spec {}: does not compile in the bare stub — excluding",
                s.id
            );
            fresh_exclusions.push(format!("{}  # no compile", s.id));
            continue;
        }
        // ── G2: the label fence (target claims, both directions) ───────
        let claimed: BTreeSet<String> = s.expect_lints.iter().cloned().collect();
        let vocab = riir_refine::score_bench::vocabulary();
        let fired: Vec<&str> = before
            .lints
            .iter()
            .map(String::as_str)
            .filter(|l| vocab.contains(*l))
            .collect();
        let claimed_only: Vec<&str> = claimed
            .iter()
            .filter(|l| !before.lints.contains(l.as_str()))
            .map(String::as_str)
            .collect();
        if !claimed_only.is_empty() {
            println!(
                "⚠ spec {}: label fence — claimed {claimed_only:?} did not fire; fired-in-vocab {fired:?}",
                s.id
            );
        }
        let mut label: Vec<String> = claimed.intersection(&before.lints).cloned().collect();
        // In-vocabulary bench hits the roster missed count toward the label
        // (the oracle is the authority, both directions).
        for extra in fired.iter().copied().filter(|l| !claimed.contains(*l)) {
            println!("⚠ spec {}: bench fired unclaimed in-vocab {extra}", s.id);
            label.push(extra.to_string());
            relabeled.push(s.id.clone());
        }
        label.sort();
        label.dedup();
        if label.is_empty() {
            println!(
                "⚠ spec {}: zero vocabulary lints at freeze — excluding",
                s.id
            );
            fresh_exclusions.push(format!("{}  # zero vocab lints", s.id));
            continue;
        }
        // ── G3: the INNOCENCE fence — the created set must be born, not ──
        //    pre-existing. Everything the heal leaves that already fired
        //    pre-heal is a PERSIST, not a creation.
        // ── G4: the SHAPE gate — heal, re-derive, adjudicate ────────────
        let (healed, _edits, passes) = riir_refine::score_bench::heal_fixpoint(&s.code);
        assert_eq!(
            passes,
            generations::v8_expected_passes(&s.id),
            "chain-truth drift on {}: measured {passes} passes at freeze",
            s.id
        );
        let after = oracle.lints_for(&wrap(&healed));
        let after_ok = after.ran && after.compiles;
        if !after_ok {
            println!(
                "⚠ spec {}: healed output does not compile/run — excluding (broken output scores 0.0, not 0.5)",
                s.id
            );
            fresh_exclusions.push(format!("{}  # healed output broken", s.id));
            continue;
        }
        let created: Vec<String> = after
            .lints
            .difference(&before.lints)
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let resolved: Vec<String> = label
            .iter()
            .filter(|l| !after.lints.contains(l.as_str()))
            .cloned()
            .collect();
        let claimed_created: Vec<String> = claims.get(s.id.as_str()).cloned().unwrap_or_default();
        if resolved.is_empty() {
            println!(
                "⚠ spec {}: SHAPE-NEGATIVE — target did not clear (persist: {:?}) — excluding",
                s.id, label
            );
            fresh_exclusions.push(format!("{}  # target persisted", s.id));
            continue;
        }
        if created.is_empty() {
            println!(
                "⚠ spec {}: SHAPE-NEGATIVE — target cleared but NOTHING created (the shape dissolved — the Plan 097 T1 precedent) — excluding",
                s.id
            );
            fresh_exclusions.push(format!("{}  # shape dissolved (created empty)", s.id));
            continue;
        }
        // The claim adjudication (report both directions; measured wins).
        let measured: BTreeSet<&str> = created.iter().map(String::as_str).collect();
        let missed_claims: Vec<&str> = claimed_created
            .iter()
            .map(String::as_str)
            .filter(|c| !measured.contains(c))
            .collect();
        let unclaimed_creations: Vec<&str> = measured
            .iter()
            .copied()
            .filter(|c| !claimed_created.iter().any(|x| x.as_str() == *c))
            .collect();
        if !missed_claims.is_empty() || !unclaimed_creations.is_empty() {
            println!(
                "⚠ spec {}: created-claim adjudication — claimed {claimed_created:?} vs measured {created:?} (missed {missed_claims:?}, unclaimed {unclaimed_creations:?})",
                s.id
            );
        }
        // Innocence re-check on the MEASURED set (the static fence checked
        // claims; this checks truth): a measured creation that fired
        // pre-heal is impossible by the difference — assert anyway.
        for c in measured.iter().copied() {
            assert!(
                !before.lints.contains(c),
                "spec {}: measured creation {c} fired pre-heal — set arithmetic broken",
                s.id
            );
        }
        verified.push((s.id.clone(), label));
        verified_created.push((s.id.clone(), created));
    }

    // ── Drift gates: landed records must still match the fresh derivation ──
    for (id, label) in &verified {
        if let Some(l) = frozen.get(id.as_str()) {
            let have: BTreeSet<&str> = l.iter().copied().collect();
            let want: BTreeSet<&str> = label.iter().map(String::as_str).collect();
            if have != want {
                drift.push(id.clone());
            }
        }
        if let Some(c) = frozen_created.get(id.as_str()) {
            let have: BTreeSet<&str> = c.iter().copied().collect();
            let want: BTreeSet<&str> = verified_created
                .iter()
                .find(|(i, _)| i == id)
                .map(|(_, l)| l.iter().map(String::as_str).collect())
                .unwrap_or_default();
            if have != want {
                created_drift.push(id.clone());
            }
        }
    }
    if !drift.is_empty() {
        panic!("FROZEN_LABELS_V8 drifted from the fresh derivation on: {drift:?} — re-freeze");
    }
    if !created_drift.is_empty() {
        panic!(
            "V8_CREATED drifted from the fresh derivation on: {created_drift:?} — re-freeze (a shape changed)"
        );
    }

    // ── Report + the paste-ready freeze record ──────────────────────────
    println!("\n─ generation-v8 freeze report ────────────────────");
    println!(
        "atoms: {} verified, {} newly excluded, {} pre-excluded, {} relabeled",
        verified.len(),
        fresh_exclusions.len(),
        excluded.len(),
        relabeled.len()
    );
    let per_class: BTreeMap<&str, usize> =
        verified.iter().fold(BTreeMap::new(), |mut m, (id, _)| {
            *m.entry(generations::v8_creation_class(id)).or_default() += 1;
            m
        });
    for (class, n) in &per_class {
        println!("  creation class {class}: {n} verified atom(s)");
    }
    println!(
        "pre-registered: the SHAPE gate IS the admission (target cleared ∧ created non-empty per atom)"
    );

    let mut all_exclusions: Vec<String> = fresh_exclusions.clone();
    all_exclusions.extend(excluded.iter().map(|s| (*s).to_string()));
    all_exclusions.sort();
    all_exclusions.dedup();
    println!("\n// ── freeze record (paste into generations.rs) ──");
    println!("pub static EXCLUDED_IDS_V8: &[&str] = &[");
    for id in &all_exclusions {
        println!("    \"{id}\",");
    }
    println!("];");
    println!("pub static FROZEN_LABELS_V8: &[(&str, &[&str])] = &[");
    let mut sorted_verified = verified.clone();
    sorted_verified.sort();
    for (id, label) in &sorted_verified {
        println!(
            "    (\"{id}\", &[{}]),",
            label
                .iter()
                .map(|l| format!("\"{l}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("];");
    let mut sorted_created = verified_created.clone();
    sorted_created.sort();
    println!("pub static V8_CREATED: &[(&str, &[&str])] = &[");
    for (id, created) in &sorted_created {
        println!(
            "    (\"{id}\", &[{}]),",
            created
                .iter()
                .map(|l| format!("\"{l}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("];");
}
