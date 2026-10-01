//! Issue 082 — the disambiguating OOD experiment for the entropy-gap axis
//! (the katgpt-rs 740-promotion unblock pairing). Print-only, no gating,
//! no promotion decision; numbers land in `.benchmarks/081_ood_fresh_family_convergent_validity.md`.
//!
//! ## Question
//!
//! Run #84's `anomalous_negative` (ref = v1-frozen 3.840 nats vs ood = gen6
//! 3.110 nats, gap −0.729, d −1.11) was ATTRIBUTED to set composition — real
//! code is sharper than authored composites — not to corpus memorization.
//! That attribution was untested. This experiment holds composition constant
//! (real code vs real code) and varies ONLY provenance-family distance:
//!
//! | set | role | provenance |
//! |---|---|---|
//! | `gen6` (the 17 `RAW_SPANS_V6` spans) | reference | corpus-family — real pre-heal states of the SAME sibling repos the rule corpus was mined/authored against |
//! | `riir-neuron-db` carves | OOD-A | real, working-tree HEAD, absent from EVERY known set |
//! | `riir-dapps` carves | OOD-B | real, working-tree HEAD, absent from EVERY known set |
//!
//! If the entropy gap tracks corpus-family proximity (convergent validity —
//! the axis detects memorization), the corpus-family reference fires sharper
//! than BOTH fresh families. If the gap vanishes on real-vs-real, the axis
//! was only ever measuring authored-vs-real texture.
//!
//! ## Pre-registered readings (fixed BEFORE the first run — do not edit after)
//!
//! Sign convention: `gap = mean_ref − mean_ood`. NOTE the axis's internal
//! `mean_gap` is `mean_gen − mean_ref` — the exact NEGATION; both are
//! printed. `d` = the katgpt-core kernel's Cohen's-d-style
//! `standardized_gap`, sign-flipped to match the gap convention here, over
//! the SAME per-span entropies fed to `entropy_gap_into`.
//!
//! - **R1 — convergent validity demonstrated:** `gap < 0` AND `|d| ≥ 0.5`
//!   on BOTH fresh families ⇒ the axis tracks corpus-family proximity on
//!   composition-matched real code; run #84's anomalous_negative WAS set
//!   composition, as attributed.
//! - **R2 — convergent validity fails on real data:** `|gap| < 0.5` nats OR
//!   `gap > 0` on BOTH families ⇒ the axis measures authored-vs-real
//!   texture, not memorization; the probe stays a lab instrument (recorded
//!   honestly in the 702 addendum).
//! - **R3 — inconclusive:** the two families disagree (mixed arms), or
//!   either FRESH family yields < 20 carves (the pre-registered yield
//!   floor — it gates the carved OOD families; the reference is the frozen
//!   17-span gen6 table and is exempt by construction) ⇒ report
//!   per-family, no verdict.
//! - Per-family arm: R1-arm iff `gap < 0 && |d| ≥ 0.5`; R2-arm iff
//!   `|gap| < 0.5 || gap > 0`; otherwise ambiguous. Both R1 ⇒ R1; both R2
//!   ⇒ R2; any mix ⇒ R3.
//! - **Uncontested-pool edge (decided, pre-registered):** ALL parsed spans
//!   enter the entropy set — an all-flat edit-count vector is the
//!   legitimate "no rule fits" high-entropy signal (max ln V over the rule
//!   vocabulary), not a measurement failure. The contested/uncontested
//!   split is reported per set under BOTH censuses: rule-vector (≥ 2 rules
//!   fired) and latent pool (`span_margin`, the margin_weight access
//!   pattern — contested_domains ≥ 1). A contested-only mean entropy rides
//!   alongside as a robustness view; it does NOT gate the verdict.
//! - **Oracle-free by construction:** no clippy oracle runs anywhere in
//!   this probe — it consumes proposer edit-count score vectors
//!   ([`rule_score_vector`], the OOD axis's own statistic) + the latent
//!   census. That is a feature: the regime axis is cheap enough to ride
//!   every score-bench row precisely because it needs no oracle.
//!
//! ## Carve protocol (deterministic; no sampling, no randomness, no seeds)
//!
//! `collect_source_files_sorted` (hidden/`target`/`vendor` pruned) over the
//! sibling root; `syn::parse_file` per file (failures counted, skipped);
//! top-level `syn::Item::Fn` only, non-empty body, sliced by
//! `item.span().byte_range()` (the `score_bench_mine_v6` carve convention —
//! outer attrs/doc comments ride inside the item span); ≥ 200-byte floor;
//! per-file cap 2; stop at 24 per family. Fresh carves are measured as raw
//! item text, matching the reference (generation fixtures store RAW span
//! text; `wrap()` is an oracle-harness concern and deliberately absent).
//! Run from the repo root (the cargo default) so `../riir-neuron-db` and
//! `../riir-dapps` resolve; the carve provenance is the siblings'
//! working-tree HEAD, recorded in the bench doc.
//!
//! ## Run
//!
//! ```text
//! cargo run --example ood_fresh_family --features score_bench,latent_retrieval
//! ```
//!
//! Debug profile is deliberate: the stats are discriminative, not latency
//! (the 702 convention). Sigmoid/score-delta shapes throughout — no softmax
//! (house rule).

// Count→ratio casts over span populations (≤ 10²) and byte sums; allowed
// with cause (the margin_spectrum precedent).
#![allow(clippy::cast_precision_loss)]

use std::path::Path;

use katgpt_core::regime_probe::{conditional_entropy_nats, entropy_gap_into, EntropyGapReport};
use syn::spanned::Spanned;

use riir_refine::draft::{collect_source_files_sorted, span_margin, LatentMatcher};
use riir_refine::score_bench::generations::generation_v6;
use riir_refine::score_bench::ood::{rule_score_vector, GAP_REGIME_THRESHOLD_NATS};

/// OOD family A: absent from every known set (reference corpora come from
/// katgpt-rs / riir-ai / riir-chain / riir-train states only).
const NDB_ROOT: &str = "../riir-neuron-db";
/// OOD family B: same disjointness guarantee.
const DAPPS_ROOT: &str = "../riir-dapps";

/// Pre-registered per-family yield floor — below this the verdict is R3.
const MIN_CARVES_PER_FAMILY: usize = 20;
/// Stop walking a family at this many carves (sorted-path prefix ⇒ deterministic).
const TARGET_PER_FAMILY: usize = 24;
/// Deterministic selection: at most this many top-level fns per file.
const PER_FILE_CAP: usize = 2;
/// Trivial-getter floor (bytes) — keeps the carve scale fn-substantial like
/// the reference set.
const MIN_SPAN_BYTES: usize = 200;
/// The scale-free |d| threshold (the pre-registered "~0.5"; mirrors the role
/// [`GAP_REGIME_THRESHOLD_NATS`] plays in nats).
const D_THRESHOLD: f32 = 0.5;

/// One measured span: the axis statistic + both contest censuses + size.
struct Carve {
    label: String,
    bytes: usize,
    entropy_nats: f32,
    rules_fired: usize,
    min_margin: f32,
    contested_domains: usize,
}

/// The pre-registered per-family arm.
enum Arm {
    /// `gap < 0 && |d| ≥ 0.5` — the corpus-family reference fired sharper.
    Convergent,
    /// `|gap| < 0.5 nats || gap > 0` — no proximity signal.
    Texture,
    /// Significant in nats but not in d (or vice versa) — ambiguous.
    Ambiguous,
}

fn classify(gap_ref_minus_ood: f32, d: f32) -> Arm {
    if gap_ref_minus_ood < 0.0 && d.abs() >= D_THRESHOLD {
        Arm::Convergent
    } else if gap_ref_minus_ood.abs() < GAP_REGIME_THRESHOLD_NATS || gap_ref_minus_ood > 0.0 {
        Arm::Texture
    } else {
        Arm::Ambiguous
    }
}

fn arm_name(arm: &Arm) -> &'static str {
    match arm {
        Arm::Convergent => "R1-arm (reference sharper, |d| ≥ 0.5)",
        Arm::Texture => "R2-arm (no proximity signal)",
        Arm::Ambiguous => "ambiguous (sign/d disagreement)",
    }
}

/// Deterministic carve + measure of one fresh family. Returns the carves
/// plus (unreadable files, unparseable files) counts.
fn carve_family(
    name: &str,
    root: &Path,
    matcher: &mut LatentMatcher,
) -> (Vec<Carve>, usize, usize) {
    let files = collect_source_files_sorted(root, ".rs");
    let mut out: Vec<Carve> = Vec::new();
    let mut unreadable = 0usize;
    let mut unparseable = 0usize;
    for path in &files {
        if out.len() >= TARGET_PER_FAMILY {
            break;
        }
        let Ok(src) = std::fs::read_to_string(path) else {
            unreadable += 1;
            continue;
        };
        let Ok(file) = syn::parse_file(&src) else {
            unparseable += 1;
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let mut taken = 0usize;
        for item in &file.items {
            if taken >= PER_FILE_CAP {
                break;
            }
            let syn::Item::Fn(f) = item else { continue };
            if f.block.stmts.is_empty() {
                continue;
            }
            let Some(text) = src.get(f.span().byte_range()) else {
                continue;
            };
            if text.len() < MIN_SPAN_BYTES {
                continue;
            }
            taken += 1;
            // The carve came out of a successful parse of the same text, so
            // this cannot fail; if it ever does, count it loud and move on.
            let Some(scores) = rule_score_vector(text) else {
                unparseable += 1;
                continue;
            };
            let sm = span_margin(matcher, text);
            out.push(Carve {
                label: format!("{name}:{rel}"),
                bytes: text.len(),
                entropy_nats: conditional_entropy_nats(&scores),
                rules_fired: scores.iter().filter(|&&s| s > 0.0).count(),
                min_margin: sm.min,
                contested_domains: sm.contested_domains,
            });
            if out.len() >= TARGET_PER_FAMILY {
                break;
            }
        }
    }
    (out, unreadable, unparseable)
}

struct SetStats {
    name: String,
    n: usize,
    mean_entropy: f64,
    contested_only_mean: f64,
    vector_contested: usize,
    pool_contested: usize,
    min_bytes: usize,
    mean_bytes: usize,
}

fn stats(name: &str, carves: &[Carve]) -> SetStats {
    let n = carves.len();
    let sum: f64 = carves.iter().map(|c| f64::from(c.entropy_nats)).sum();
    let contested: Vec<f32> = carves
        .iter()
        .filter(|c| c.rules_fired >= 2)
        .map(|c| c.entropy_nats)
        .collect();
    let contested_sum: f64 = contested.iter().map(|e| f64::from(*e)).sum();
    SetStats {
        name: name.to_string(),
        n,
        mean_entropy: sum / n as f64,
        contested_only_mean: if contested.is_empty() {
            f64::NAN
        } else {
            contested_sum / contested.len() as f64
        },
        vector_contested: contested.len(),
        pool_contested: carves.iter().filter(|c| c.contested_domains >= 1).count(),
        min_bytes: carves.iter().map(|c| c.bytes).min().unwrap_or(0),
        mean_bytes: carves.iter().map(|c| c.bytes).sum::<usize>() / n.max(1),
    }
}

fn artifact_hex(report: &EntropyGapReport) -> String {
    report.artifact.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    println!("# issue 082 — OOD fresh-family pairing (composition-matched, provenance-disjoint)");
    println!("# reference: gen6 (RAW_SPANS_V6, real pre-heal sibling states) | OOD-A: {NDB_ROOT} | OOD-B: {DAPPS_ROOT}");
    println!("# PRE-REGISTERED (module doc carries the full text): gap = mean_ref − mean_ood;");
    println!("#   R1 iff gap < 0 && |d| ≥ {D_THRESHOLD} on BOTH families; R2 iff |gap| < {GAP_REGIME_THRESHOLD_NATS} nats || gap > 0 on BOTH;");
    println!("#   mixed ⇒ R3 inconclusive; < {MIN_CARVES_PER_FAMILY} fresh carves ⇒ R3 (the frozen 17-span gen6 reference is exempt); all parsed spans enter the entropy set.");

    let mut matcher = LatentMatcher::from_enabled_domains().unwrap_or_else(|e| {
        eprintln!("matcher build failed: {e}");
        std::process::exit(1);
    });
    println!("# latent census domains: {:?}", matcher.domains());

    // ── reference: gen6, sorted-id order (the ood.rs fixture_entropies shape) ──
    let mut gen6 = generation_v6();
    gen6.sort_by(|a, b| a.id.cmp(b.id));
    let gen6_carves: Vec<Carve> = gen6
        .iter()
        .filter_map(|fx| {
            let scores = rule_score_vector(fx.code)?;
            let sm = span_margin(&mut matcher, fx.code);
            Some(Carve {
                label: format!("gen6:{}", fx.id),
                bytes: fx.code.len(),
                entropy_nats: conditional_entropy_nats(&scores),
                rules_fired: scores.iter().filter(|&&s| s > 0.0).count(),
                min_margin: sm.min,
                contested_domains: sm.contested_domains,
            })
        })
        .collect();
    let gen6_skipped = gen6.len() - gen6_carves.len();
    if gen6_skipped > 0 {
        println!("# WARNING: {gen6_skipped} gen6 fixture(s) produced no score vector (expected 0 — the set is parse-pinned)");
    }

    // ── fresh families ──
    let (ndb, ndb_unreadable, ndb_unparseable) =
        carve_family("ndb", Path::new(NDB_ROOT), &mut matcher);
    let (dapps, dapps_unreadable, dapps_unparseable) =
        carve_family("dapps", Path::new(DAPPS_ROOT), &mut matcher);
    println!(
        "# carve skips: ndb unreadable={ndb_unreadable} unparseable={ndb_unparseable} | dapps unreadable={dapps_unreadable} unparseable={dapps_unparseable}"
    );

    // ── per-set stats (both contest censuses + sizes for the composition audit) ──
    let sets = [
        ("gen6", &gen6_carves),
        ("riir-neuron-db", &ndb),
        ("riir-dapps", &dapps),
    ];
    println!("\n── per-set stats (all parsed spans enter the entropy set) ──");
    println!(
        "{:<16} {:>3} {:>10} {:>12} {:>7} {:>7} {:>6} {:>6} {:>12}",
        "set", "n", "mean-H nat", "H(contest)", "vec≥2", "pool≥1", "minB", "meanB", "yield-ok(≥20)"
    );
    for (name, carves) in &sets {
        let s = stats(name, carves);
        println!(
            "{:<16} {:>3} {:>10.4} {:>12.4} {:>7} {:>6} {:>6} {:>6} {:>14}",
            s.name,
            s.n,
            s.mean_entropy,
            s.contested_only_mean,
            s.vector_contested,
            s.pool_contested,
            s.min_bytes,
            s.mean_bytes,
            if s.name == "gen6" {
                "exempt(frozen)"
            } else if s.n >= MIN_CARVES_PER_FAMILY {
                "yes"
            } else {
                "NO"
            }
        );
    }

    // ── gen6 per-repo family breakdown (provenance via the OOD-set resolver) ──
    println!("\n── gen6 per-repo family breakdown ──");
    let mut families: Vec<(&str, Vec<f32>)> = Vec::new();
    for (fx, carve) in gen6.iter().zip(gen6_carves.iter()) {
        let fam = riir_refine::score_bench::ood::ood_set_family(fx);
        match families.iter_mut().find(|(f, _)| *f == fam) {
            Some((_, ents)) => ents.push(carve.entropy_nats),
            None => families.push((fam, vec![carve.entropy_nats])),
        }
    }
    families.sort_by(|a, b| a.0.cmp(b.0));
    for (fam, ents) in &families {
        let mean = ents.iter().map(|e| f64::from(*e)).sum::<f64>() / ents.len() as f64;
        println!("{fam:<28} n={:<3} mean-H={:.4} nats", ents.len(), mean);
    }

    // ── pairings: ref = gen6 vs each fresh family (the kernel computes gap + d) ──
    let ref_ents: Vec<f32> = gen6_carves.iter().map(|c| c.entropy_nats).collect();
    println!(
        "\n── pairings (reference = gen6, {} spans) ──",
        ref_ents.len()
    );
    let mut arms: Vec<(&str, Arm)> = Vec::new();
    for (name, carves) in [("riir-neuron-db", &ndb), ("riir-dapps", &dapps)] {
        let ood_ents: Vec<f32> = carves.iter().map(|c| c.entropy_nats).collect();
        if ref_ents.is_empty() || ood_ents.is_empty() {
            println!("{name}: INSUFFICIENT (empty sample) — no pairing");
            arms.push((name, Arm::Ambiguous));
            continue;
        }
        let mut report = EntropyGapReport::default();
        entropy_gap_into(&ref_ents, &ood_ents, &mut report);
        // The kernel's mean_gap = mean_gen − mean_ref; the pre-registered
        // convention here is the negation. Same for d.
        let gap = -report.mean_gap;
        let d = -report.standardized_gap;
        let arm = classify(gap, d);
        println!(
            "{name}: mean_ref={:.4} mean_ood={:.4} gap(ref−ood)={:+.4} d={:+.3} arm={} artifact={}",
            report.mean_ref,
            report.mean_gen,
            gap,
            d,
            arm_name(&arm),
            artifact_hex(&report)
        );
        arms.push((name, arm));
    }
    println!("# (axis-convention note: ood.rs's mean_gap = mean_ood − mean_ref — the negation of the gap printed above)");

    // ── the pre-registered combined verdict ──
    let both = |want: fn(&Arm) -> bool| arms.iter().all(|(_, a)| want(a));
    let yield_ok = ndb.len() >= MIN_CARVES_PER_FAMILY && dapps.len() >= MIN_CARVES_PER_FAMILY;
    println!("\n── VERDICT ──");
    if !yield_ok {
        println!(
            "R3 INCONCLUSIVE — fresh-family yield floor: ndb={} dapps={} (floor {MIN_CARVES_PER_FAMILY}; the gen6 reference is the frozen {}-span table, exempt)",
            ndb.len(),
            dapps.len(),
            gen6_carves.len()
        );
    } else if both(|a| matches!(a, Arm::Convergent)) {
        println!(
            "R1 — CONVERGENT VALIDITY DEMONSTRATED: the axis tracks corpus-family proximity on"
        );
        println!(
            "composition-matched real code (reference sharper on BOTH fresh families); run #84's"
        );
        println!(
            "anomalous_negative WAS set composition (authored-vs-real texture), as attributed."
        );
    } else if both(|a| matches!(a, Arm::Texture)) {
        println!(
            "R2 — CONVERGENT VALIDITY FAILS ON REAL DATA: no proximity signal on either fresh"
        );
        println!(
            "family; the axis was only ever measuring authored-vs-real texture. The probe stays a"
        );
        println!("lab instrument (recorded honestly in the 702 addendum).");
    } else {
        println!(
            "R3 INCONCLUSIVE — the families disagree; per-family arms printed above. No verdict."
        );
    }
    println!("# oracle-free: no clippy oracle ran; the probe consumed proposer edit-count score");
    println!("# vectors + the latent census only.");

    // ── per-carve table (full transparency; sorted as walked/measured) ──
    println!("\n── per-carve table (set order: gen6 id-sorted, then sorted-path walk) ──");
    for (name, carves) in &sets {
        for c in carves.iter() {
            println!(
                "{name:<16} {:<58} B={:<5} rules={:<2} H={:.4} margin={} domains≥2={}",
                c.label,
                c.bytes,
                c.rules_fired,
                c.entropy_nats,
                if c.min_margin.is_infinite() {
                    "∞".to_string()
                } else {
                    format!("{:.4}", c.min_margin)
                },
                c.contested_domains,
            );
        }
    }
}
