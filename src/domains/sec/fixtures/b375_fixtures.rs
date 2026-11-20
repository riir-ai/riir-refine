//! Batch 375 sec fixture preflight — the laya round-2 (Plan 391 /
//! Research 425): `NandhaKishorM/laya`
//! @ `416dd9ed20da2d43fc40e23712e9adceefabf634` (Apache-2.0). ORIGINAL
//! plain-Rust stand-ins — zero repo source pasted (the laws are
//! re-authored; the source quotes stay in the corpus). Compiled AND
//! executed BEFORE entry:
//! `rustc --edition 2021 --test b375_fixtures.rs && ./b375_fixtures`
//!
//! Arms:
//! - F7 `per-item-bound-not-batch-max`: a per-row invariant (this item's
//!   target entries ≤ this item's option count) is checked against the
//!   ROW's OWN bound at the collect boundary — the batch-wide max
//!   legitimises a malformed row in a mixed-width batch and the failure
//!   surfaces later as an opaque tensor-size mismatch naming neither the
//!   item nor the mistake; the fixed arm refuses at the collect with a
//!   named error (item index + both counts + the required relation).

// ═══════════════════════════════════════════════════════════════════════
// F7 — per-item-bound-not-batch-max
// ═══════════════════════════════════════════════════════════════════════

struct Item {
    targets: Vec<u32>, // one entry per option the item answers
    options: usize,    // this item's OWN marker positions
}

/// BUGGY: validate against the BATCH-WIDE max — a longer sibling row
/// legitimises a malformed item's extra entries.
fn collate_buggy(items: &[Item]) -> Result<Vec<Vec<u32>>, String> {
    let batch_max = items.iter().map(|i| i.options).max().unwrap_or(0);
    let mut cols = Vec::with_capacity(items.len());
    for item in items {
        if item.targets.len() > batch_max {
            return Err("collate: batch-wide bound exceeded".to_string());
        }
        // pad to the batch max (the collate's contract) — the malformed
        // item's extra entries flow in here silently
        let mut row = item.targets.clone();
        row.resize(batch_max, 0);
        cols.push(row);
    }
    Ok(cols)
}

/// The downstream consumer: builds the one-hot label matrix per item —
/// the place the malformed row surfaces as an OPAQUE mismatch.
fn build_labels(cols: &[Vec<u32>], items: &[Item]) -> Result<(), String> {
    for (row, item) in cols.iter().zip(items.iter()) {
        for &t in row {
            if t != 0 && (t as usize) > item.options {
                // the opaque tensor-size class: names neither the item
                // index nor the actual mistake
                return Err(format!(
                    "the expanded size of the tensor ({}) must match the existing size ({})",
                    t, item.options
                ));
            }
        }
    }
    Ok(())
}

/// FIXED: validate each item against ITS OWN bound AT THE COLLECT, with
/// a named error (index + both counts + the required relation).
fn collate_fixed(items: &[Item]) -> Result<Vec<Vec<u32>>, String> {
    let batch_max = items.iter().map(|i| i.options).max().unwrap_or(0);
    let mut cols = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        if item.targets.len() > item.options {
            return Err(format!(
                "collate_items: item {} has {} target entries but only {} marker positions; \
                 a target needs one entry per option",
                i,
                item.targets.len(),
                item.options
            ));
        }
        let mut row = item.targets.clone();
        row.resize(batch_max, 0);
        cols.push(row);
    }
    Ok(cols)
}

#[test]
fn f7_per_item_bound_not_batch_max() {
    // the mixed-width batch: item 1 is a two-option item whose targets
    // name FOUR options — malformed against ITS OWN bound but inside the
    // batch max (item 0 carries six options).
    let items = vec![
        Item {
            targets: vec![1, 3, 6, 2, 5, 4],
            options: 6,
        },
        Item {
            targets: vec![1, 2, 4, 3],
            options: 2,
        }, // the malformed row
    ];

    // BUGGY: the batch max (6) legitimises item 1's four entries; the
    // failure surfaces downstream as the opaque mismatch.
    let cols = collate_buggy(&items).expect("the batch-max gate passes the malformed row");
    let err = build_labels(&cols, &items)
        .expect_err("the downstream consumer must hit the opaque mismatch");
    assert!(
        err.contains("expanded size") && !err.contains("item 1"),
        "the buggy failure is the opaque class (names neither the item nor the mistake): {err}"
    );

    // FIXED: the collect refuses, naming the item + both counts.
    let err2 = collate_fixed(&items).expect_err("the per-item gate must refuse");
    for needle in [
        "item 1",
        "4 target entries",
        "2 marker positions",
        "one entry per option",
    ] {
        assert!(
            err2.contains(needle),
            "the named error must carry {needle:?}: {err2}"
        );
    }

    // ANTI-REGIME: a uniform WELL-FORMED batch passes both arms identically.
    let ok = vec![
        Item {
            targets: vec![1, 2],
            options: 3,
        },
        Item {
            targets: vec![2, 1],
            options: 3,
        },
    ];
    let a = collate_buggy(&ok).expect("well-formed passes the batch gate");
    let b = collate_fixed(&ok).expect("well-formed passes the per-item gate");
    assert_eq!(a, b);
    build_labels(&a, &ok).expect("well-formed builds");

    // the single-item edge: a batch of ONE malformed item — the batch max
    // IS its own bound, so the batch-max gate catches it too (the regimes
    // coincide; the law's harm needs a mixed batch).
    let single = vec![Item {
        targets: vec![1, 2, 3],
        options: 2,
    }];
    assert!(collate_buggy(&single).is_err());
    assert!(collate_fixed(&single).is_err());
}
