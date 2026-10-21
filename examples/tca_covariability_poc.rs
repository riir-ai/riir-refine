//! Issue 094 — trajectory covariability-class mining POC (defend-wrong).
//!
//! Hypothesis (`.issues/094`, Research 309 §"consumer surface"): the
//! sliceTCA **drift-class** share attributable to a corpus rule predicts
//! whether that rule needed re-mining (proposer revision/guard after its
//! landing batch). Ground truth = the 2026-09-12 re-mined vs stable label
//! snapshot (LABEL tables below, evidence in the issue's closing note).
//!
//! Tensor construction — `X[rule, day, shape-bucket]` over the per-repo
//! `TrajStore`s, mapped onto `katgpt_core::slice_tca::Tensor3`
//! `[entity, time, episode]`:
//! - axis 0 (entity) = `lint_key` (clippy lane snake_case, rust_perf kebab
//!   — the store's exact keys; rustc_errors `{code}:{strategy}` keys are
//!   EXCLUDED, they are not part of this hypothesis's population);
//! - axis 1 (time) = UTC day of the row's `last_seen_at` — the SESSION
//!   proxy. Honest caveat: the store keeps CUMULATIVE counters per
//!   (rule, shape) with only the LAST verdict's timestamp, so a row's mass
//!   lands entirely on its last-seen day. Drift therefore reads as "which
//!   days did this rule's DISTINCT shapes last fire on" — shape churn
//!   across days — not within-shape temporal drift;
//! - axis 2 (episode) = FNV-1a bucket of the shape text mod `K_BUCKET`
//!   (deterministic, rule-agnostic spreading).
//!
//! Cell = `ln(1 + success_count + fail_count)` per (rule, day, bucket),
//! aggregated across stores. Rows filtered: `tier == Withdrawn` excluded
//! (Issue 021 semantics respected — Certified + Heuristic kept),
//! `last_seen_at == 0` (pre-attribution rows) excluded.
//!
//! Scores (per rule, over the FULL union tensor):
//! - `S_tca` (primary): leave-one-rule-out `ΔEVR_time = EVR_time(X) −
//!   EVR_time(X with rule r's cells zeroed)` — rule r's signed contribution
//!   to the global time-slicing (drift) share. Classifier path only
//!   (3 SVDs per probe, no joint ALS fit — the issue's "may suffice" arm).
//!   Diagnostics: ΔEVR_entity (stable class) and ΔEVR_episode (routine).
//! - Baselines a rule-ranking must beat for the tensor to add signal:
//!   `B_mass` (total activity — the volume confound), `B_failshare`
//!   (fail ratio — the "rules that break get revised" confound),
//!   `B_dayentropy` (day-marginal entropy — the naive spread heuristic),
//!   majority class (AUC 0.5 by construction).
//!
//! Eval: exact Mann-Whitney AUC over labeled rules present in the tensor.
//! Pre-registered verdict: **PASS iff `AUC(S_tca, r=4) >
//! max(AUC(B_mass), AUC(B_failshare), AUC(B_dayentropy), 0.5)` AND ≥ 20
//! labeled rules carry mass** — the drift class must rank re-mined rules
//! above stable ones BEYOND every confound. PASS ⇒ feed the ranking into
//! `.distill/001` §"Post-mining intake"; FAIL ⇒ negative record in the
//! issue, close (the gw_corpus PoC precedent: a measured negative is a
//! result). Print-only, always exit 0 — this is a report, not a gate.
//!
//! Stores are read via a `/tmp` MIRROR (copy of `.refine/trajectories.kv` +
//! `.refine/key`) — sibling repos' `.refine/` trees are read-only to this
//! session; opening the mirrored copy guarantees zero writes upstream.
//! Roots: `RIIR_TCA094_ROOTS` (colon-separated) or the standard sibling
//! layout relative to this crate's parent directory.
//!
//! Run: `cargo run --release --features latent_retrieval --release \
//!   --example tca_covariability_poc`

#![cfg(feature = "latent_retrieval")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use riir_refine::memory::EvidenceTier;
use riir_refine::traj_store::{TrajRowDoc, TrajStore};

/// Shape-bucket count (episode axis size). Deterministic FNV-1a spread.
const K_BUCKET: usize = 32;

/// Top-R spectrum width for the covariability classifier (primary arm).
const R_PRIMARY: usize = 4;

/// Minimum labeled rules with mass for the verdict to claim anything.
const MIN_LABELED_WITH_MASS: usize = 20;

/// Sibling repos whose `.refine/trajectories.kv` stores feed the tensor
/// (the Issue-037 audit's store-bearing set).
const STORE_REPOS: &[&str] = &[
    "katgpt-rs",
    "riir-ai",
    "riir-train",
    "riir-neuron-db",
    "riir-chain",
    "riir-clippy",
    "riir-game-sdk",
    "riir-mmorpg-examples",
    "riir-viewbridge",
    "riir-dapps",
];

/// RE-MINED label — proposer/pattern materially revised or guarded after
/// the landing batch (2026-09-12 snapshot; per-rule commit evidence in the
/// issue's closing note — subagent ground-truth dig, `.distill/001`
/// §Post-mining intake P1–P19 mapped to commits).
const RE_MINED: &[&str] = &[
    // clippy lane (snake_case store keys)
    "map_unwrap_or",
    "uninlined_format_args",
    "needless_range_loop",
    "manual_let_else",
    "items_after_statements",
    "manual_is_multiple_of",
    "redundant_closure",
    "use_self",
    "unnecessary_sort_by",
    "match_bool",
    "single_match_else",
    "useless_format",
    "needless_return",
    "question_mark",
    "comparison_to_empty",
    "collapsible_if",
    "needless_bool",
    "needless_bool_assign",
    "collapsible_else_if",
    "range_plus_one",
    "double_parens",
    "redundant_pattern_matching",
    "manual_map",
    "unnecessary_unwrap",
    "semicolon_if_nothing_returned",
    "byte_char_slices",
    "unnecessary_lazy_evaluations",
    // rust_perf lane (kebab-case store keys)
    "vec-with-capacity",
    "hashmap-with-capacity",
    "entry-vs-contains-insert",
    "bool-to-bitpack",
    "static-str",
    "fxhashmap",
];

/// STABLE label — landed, proposer logic untouched since (Tier A
/// zero-touch + Tier B mechanical/dogfood-only; same snapshot).
const STABLE: &[&str] = &[
    // clippy lane
    "filter_next",
    "iter_next_loop",
    "iter_nth_zero",
    "single_char_pattern",
    "while_let_on_iterator",
    "identity_op",
    "deref_addrof",
    "redundant_static_lifetimes",
    "wildcard_in_or_patterns",
    "unused_unit",
    "single_match",
    "manual_unwrap_or",
    "needless_match",
    "chunks_exact_to_as_chunks",
    "manual_slice_fill",
    "manual_checked_ops",
    "suboptimal_flops",
    "useless_conversion",
    "needless_borrow",
    "needless_lifetimes",
    "redundant_field_names",
    "unnecessary_mut_passed",
    // rust_perf lane
    "shrink-to-fit-after-build",
    "collect-append-to-extend",
    "hashset-insert-bool-return",
    "vec-macro-zero-fill",
    "distance-sq-no-sqrt",
];

/// One filtered trajectory row, mapped into tensor coordinates.
#[derive(Clone, Debug)]
struct Cell {
    rule: String,
    day_index: i64,
    bucket: usize,
    mass: f32,
    fails: u32,
    total: u32,
}

/// FNV-1a 64-bit — stable across runs/platforms (std hash is not).
fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Mirror one repo's `.refine/trajectories.kv` + `.refine/key` into a temp
/// dir and open the MIRROR (never the original — sibling `.refine` trees are
/// read-only to this session; opening a LocalKvStore may replay/compact).
fn open_mirrored(
    repo_root: &std::path::Path,
    mirror_base: &std::path::Path,
) -> Option<(TrajStore, String)> {
    let heal = repo_root.join(".refine");
    let kv_dir = heal.join("trajectories.kv");
    if !kv_dir.is_dir() {
        return None;
    }
    let name = repo_root.file_name()?.to_string_lossy().to_string();
    let mirror = mirror_base.join(&name);
    let mirror_heal = mirror.join(".refine");
    std::fs::create_dir_all(&mirror_heal).ok()?;
    // Recursive copy of the store dir (WAL segments + manifests).
    copy_dir_recursive(&kv_dir, &mirror_heal.join("trajectories.kv"))?;
    if let Ok(key) = std::fs::read_to_string(heal.join("key")) {
        let _ = std::fs::write(mirror_heal.join("key"), key);
    }
    let (store, _bad) = TrajStore::open(&mirror).ok()?;
    Some((store, name))
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Option<()> {
    std::fs::create_dir_all(dst).ok()?;
    for entry in std::fs::read_dir(src).ok()? {
        let entry = entry.ok()?;
        let ty = entry.file_type().ok()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to).ok()?;
        }
    }
    Some(())
}

/// Filter + map one store's rows into tensor cells.
fn rows_to_cells(rows: &[TrajRowDoc], repo: &str) -> (Vec<Cell>, usize, usize, usize) {
    let mut cells = Vec::new();
    let mut skipped_rustc = 0usize;
    let mut skipped_withdrawn = 0usize;
    let mut skipped_no_ts = 0usize;
    for r in rows {
        // rustc_errors lane: not this hypothesis's population.
        if r.lint_key.contains(':') {
            skipped_rustc += 1;
            continue;
        }
        if EvidenceTier::from_u8(r.tier) == EvidenceTier::Withdrawn {
            skipped_withdrawn += 1;
            continue;
        }
        if r.last_seen_at == 0 {
            skipped_no_ts += 1;
            continue;
        }
        let day = (r.last_seen_at / 86_400) as i64;
        let bucket = (fnv1a64(&r.shape) % K_BUCKET as u64) as usize;
        let total = r.success_count + r.fail_count;
        cells.push(Cell {
            rule: r.lint_key.clone(),
            day_index: day,
            bucket,
            mass: (total as f32).ln_1p(),
            fails: r.fail_count,
            total,
        });
    }
    let _ = repo;
    (cells, skipped_rustc, skipped_withdrawn, skipped_no_ts)
}

/// Exact Mann-Whitney AUC: P(score_pos > score_neg) + 0.5·P(tie).
fn auc(pairs: &[(u8, f32)]) -> f32 {
    let pos: Vec<f32> = pairs
        .iter()
        .filter(|(l, _)| *l == 1)
        .map(|(_, s)| *s)
        .collect();
    let neg: Vec<f32> = pairs
        .iter()
        .filter(|(l, _)| *l == 0)
        .map(|(_, s)| *s)
        .collect();
    if pos.is_empty() || neg.is_empty() {
        return f32::NAN;
    }
    let mut concordant = 0.0f32;
    for &p in &pos {
        for &n in &neg {
            if p > n {
                concordant += 1.0;
            } else if p == n {
                concordant += 0.5;
            }
        }
    }
    concordant / (pos.len() * neg.len()) as f32
}

fn main() {
    // ── 1. Roots: env override or the sibling layout ────────────────────
    let default_parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .expect("manifest dir has a parent");
    let roots: Vec<PathBuf> = match std::env::var("RIIR_TCA094_ROOTS") {
        Ok(v) if !v.trim().is_empty() => v.split(':').map(PathBuf::from).collect(),
        _ => STORE_REPOS.iter().map(|r| default_parent.join(r)).collect(),
    };

    // ── 2. Mirror + open every store, dump rows ─────────────────────────
    let mirror_base = std::env::temp_dir().join(format!("tca094-{}", std::process::id()));
    let mut all_cells: Vec<Cell> = Vec::new();
    let mut stores_read = 0usize;
    let mut rows_seen = 0usize;
    let (mut sk_rustc, mut sk_wd, mut sk_ts) = (0usize, 0usize, 0usize);
    for root in &roots {
        match open_mirrored(root, &mirror_base) {
            Some((store, name)) => {
                let rows = store.dump();
                rows_seen += rows.len();
                let (cells, a, b, c) = rows_to_cells(&rows, &name);
                println!(
                    "[store] {name}: {} rows → {} cells (skipped rustc_errors {a}, withdrawn {b}, no-ts {c})",
                    rows.len(),
                    cells.len()
                );
                sk_rustc += a;
                sk_wd += b;
                sk_ts += c;
                all_cells.extend(cells);
                stores_read += 1;
            }
            None => println!(
                "[store] {}: no .refine/trajectories.kv — skipped",
                root.display()
            ),
        }
    }
    println!(
        "\nstores read: {stores_read} | rows: {rows_seen} | cells: {} | skipped: rustc_errors {sk_rustc}, withdrawn {sk_wd}, no-ts {sk_ts}",
        all_cells.len()
    );
    if all_cells.is_empty() {
        println!("NO DATA — nothing to decompose. Verdict: INSUFFICIENT DATA.");
        return;
    }

    // ── 3. Axes ──────────────────────────────────────────────────────────
    let mut rules: Vec<String> = all_cells.iter().map(|c| c.rule.clone()).collect();
    rules.sort();
    rules.dedup();
    let mut days: Vec<i64> = all_cells.iter().map(|c| c.day_index).collect();
    days.sort();
    days.dedup();
    let rule_idx: BTreeMap<String, usize> = rules
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, r)| (r, i))
        .collect();
    let day_idx: BTreeMap<i64, usize> = days
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, d)| (d, i))
        .collect();

    let n = rules.len();
    let t = days.len();
    let k = K_BUCKET;
    println!(
        "tensor: [rule {n}] × [day {t}] × [bucket {k}]  (days {}..={})",
        days.first().map(|d| date_of(*d)).unwrap_or_default(),
        days.last().map(|d| date_of(*d)).unwrap_or_default(),
    );

    // ── 4. Fill the tensor (sum log-mass per cell across stores) ────────
    let mut data = vec![0.0f32; n * t * k];
    // Rule-level aggregates for the baselines.
    let mut mass_total = vec![0.0f32; n];
    let mut fails = vec![0u64; n];
    let mut totals = vec![0u64; n];
    let mut day_mass = vec![vec![0.0f32; t]; n];
    for c in &all_cells {
        let ri = rule_idx[&c.rule];
        let di = day_idx[&c.day_index];
        data[(ri * t + di) * k + c.bucket] += c.mass;
        mass_total[ri] += c.mass;
        fails[ri] += c.fails as u64;
        totals[ri] += c.total as u64;
        day_mass[ri][di] += c.mass;
    }
    let shape = [n, t, k];

    // ── 5. Global covariability shares + routing ────────────────────────
    let shares_full = katgpt_core::slice_tca::covariability_shares(&data, shape, R_PRIMARY);
    let route = katgpt_core::slice_tca::route_default(shares_full);
    println!(
        "\nglobal covariability shares (r={R_PRIMARY}): entity/stable {:.4}  time/drift {:.4}  episode/routine {:.4}",
        shares_full[0], shares_full[1], shares_full[2]
    );
    println!(
        "route_default: stable {:.3}  drift {:.3}  routine {:.3}",
        route[0], route[1], route[2]
    );

    // ── 6. Leave-one-rule-out ΔEVR per rule ─────────────────────────────
    // Zero rule r's cells in a scratch copy, re-run the classifier.
    let mut delta_time = vec![0.0f32; n];
    let mut delta_entity = vec![0.0f32; n];
    let mut delta_episode = vec![0.0f32; n];
    let mut scratch = data.clone();
    for (ri, rule) in rules.iter().enumerate() {
        let has_mass = mass_total[ri] > 0.0;
        if !has_mass {
            continue;
        }
        for di in 0..t {
            let base = (ri * t + di) * k;
            scratch[base..base + k].fill(0.0);
        }
        let shares_wo = katgpt_core::slice_tca::covariability_shares(&scratch, shape, R_PRIMARY);
        delta_time[ri] = shares_full[1] - shares_wo[1];
        delta_entity[ri] = shares_full[0] - shares_wo[0];
        delta_episode[ri] = shares_full[2] - shares_wo[2];
        // restore
        for di in 0..t {
            let base = (ri * t + di) * k;
            scratch[base..base + k].copy_from_slice(&data[base..base + k]);
        }
        let _ = rule;
    }

    // Day-entropy baseline: normalized entropy of the rule's day marginal.
    let day_entropy: Vec<f32> = (0..n)
        .map(|ri| {
            let m = &day_mass[ri];
            let tot: f32 = m.iter().sum();
            if tot <= 0.0 {
                return 0.0;
            }
            let support = m.iter().filter(|&&v| v > 0.0).count().max(1) as f32;
            let h: f32 = -m
                .iter()
                .filter(|&&v| v > 0.0)
                .map(|&v| {
                    let p = v / tot;
                    p * p.ln()
                })
                .sum::<f32>();
            (-h) / support.ln()
        })
        .collect();

    // ── 7. AUC over labeled rules present in the tensor ─────────────────
    let mut pairs_tca: Vec<(u8, f32)> = Vec::new();
    let mut pairs_entity: Vec<(u8, f32)> = Vec::new();
    let mut pairs_mass: Vec<(u8, f32)> = Vec::new();
    let mut pairs_fail: Vec<(u8, f32)> = Vec::new();
    let mut pairs_ent: Vec<(u8, f32)> = Vec::new();
    let mut labeled_missing: Vec<&str> = Vec::new();
    for &name in RE_MINED.iter().chain(STABLE.iter()) {
        let label: u8 = if RE_MINED.contains(&name) { 1 } else { 0 };
        match rule_idx.get(name) {
            Some(&ri) if mass_total[ri] > 0.0 => {
                pairs_tca.push((label, delta_time[ri]));
                pairs_entity.push((label, delta_entity[ri]));
                pairs_mass.push((label, mass_total[ri]));
                let failshare = if totals[ri] > 0 {
                    fails[ri] as f32 / totals[ri] as f32
                } else {
                    0.0
                };
                pairs_fail.push((label, failshare));
                pairs_ent.push((label, day_entropy[ri]));
            }
            _ => labeled_missing.push(name),
        }
    }
    let n_pos = pairs_tca.iter().filter(|(l, _)| *l == 1).count();
    let n_neg = pairs_tca.iter().filter(|(l, _)| *l == 0).count();
    println!(
        "\nlabeled rules with mass: {} (re-mined {n_pos}, stable {n_neg}); zero-mass/absent: {}",
        pairs_tca.len(),
        labeled_missing.len()
    );
    if !labeled_missing.is_empty() {
        println!("  zero-mass labels: {}", labeled_missing.join(", "));
    }

    let auc_tca = auc(&pairs_tca);
    let auc_entity = auc(&pairs_entity);
    let auc_mass = auc(&pairs_mass);
    let auc_fail = auc(&pairs_fail);
    let auc_ent = auc(&pairs_ent);
    println!("\n── AUC (re-mined ranked above stable) ──");
    println!("  S_tca  ΔEVR_time   (primary, r={R_PRIMARY}): {auc_tca:.4}");
    println!("  diag   ΔEVR_entity (stable-class)         : {auc_entity:.4}");
    println!("  B_mass total log-mass                     : {auc_mass:.4}");
    println!("  B_fail fail share                         : {auc_fail:.4}");
    println!("  B_ent  day-marginal entropy               : {auc_ent:.4}");
    println!("  B_maj  majority class                     : 0.5000");

    // r-robustness: the primary arm at r=2 and r=8.
    for r_probe in [2usize, 8usize] {
        let full = katgpt_core::slice_tca::covariability_shares(&data, shape, r_probe);
        let mut probe: Vec<(u8, f32)> = Vec::new();
        let mut scratch2 = data.clone();
        for &name in RE_MINED.iter().chain(STABLE.iter()) {
            let label: u8 = if RE_MINED.contains(&name) { 1 } else { 0 };
            if let Some(&ri) = rule_idx.get(name) {
                if mass_total[ri] <= 0.0 {
                    continue;
                }
                for di in 0..t {
                    let base = (ri * t + di) * k;
                    scratch2[base..base + k].fill(0.0);
                }
                let wo = katgpt_core::slice_tca::covariability_shares(&scratch2, shape, r_probe);
                probe.push((label, full[1] - wo[1]));
                for di in 0..t {
                    let base = (ri * t + di) * k;
                    scratch2[base..base + k].copy_from_slice(&data[base..base + k]);
                }
            }
        }
        println!(
            "  S_tca  ΔEVR_time   (robustness, r={r_probe}): {:.4}",
            auc(&probe)
        );
    }

    // ── 8. Per-rule table for the labeled set ───────────────────────────
    println!("\n── labeled rules (sorted by ΔEVR_time desc) ──");
    println!(
        "{:<32} {:<8} {:>10} {:>10} {:>10} {:>8} {:>8}",
        "rule", "label", "dTime", "dEntity", "dEpisode", "mass", "fail%"
    );
    let mut table: Vec<(usize, u8)> = rule_idx
        .iter()
        .filter(|(name, &ri)| {
            mass_total[ri] > 0.0
                && (RE_MINED.contains(&name.as_str()) || STABLE.contains(&name.as_str()))
        })
        .map(|(name, &ri)| {
            let label: u8 = if RE_MINED.contains(&name.as_str()) {
                1
            } else {
                0
            };
            (ri, label)
        })
        .collect();
    table.sort_by(|a, b| delta_time[b.0].partial_cmp(&delta_time[a.0]).unwrap());
    for (ri, label) in &table {
        let failpct = if totals[*ri] > 0 {
            100.0 * fails[*ri] as f32 / totals[*ri] as f32
        } else {
            0.0
        };
        println!(
            "{:<32} {:<8} {:>10.5} {:>10.5} {:>10.5} {:>8.1} {:>7.1}%",
            rules[*ri],
            if *label == 1 { "REMINED" } else { "stable" },
            delta_time[*ri],
            delta_entity[*ri],
            delta_episode[*ri],
            mass_total[*ri],
            failpct
        );
    }

    // ── 9. Verdict (pre-registered) ─────────────────────────────────────
    println!("\n── verdict (pre-registered) ──");
    let enough = pairs_tca.len() >= MIN_LABELED_WITH_MASS;
    let best_baseline = auc_mass.max(auc_fail).max(auc_ent).max(0.5);
    if !enough {
        println!(
            "INSUFFICIENT DATA — {} labeled rules with mass (< {MIN_LABELED_WITH_MASS}). Accumulate + re-run.",
            pairs_tca.len()
        );
    } else if auc_tca > best_baseline {
        println!(
            "PASS — AUC(S_tca) {auc_tca:.4} > best baseline {best_baseline:.4} (mass {auc_mass:.4}, fail {auc_fail:.4}, entropy {auc_ent:.4}). Feed the ranking into .distill/001 §Post-mining intake."
        );
    } else {
        println!(
            "FAIL — AUC(S_tca) {auc_tca:.4} does NOT beat best baseline {best_baseline:.4} (mass {auc_mass:.4}, fail {auc_fail:.4}, entropy {auc_ent:.4}). Negative record in the issue; close."
        );
    }

    let _ = mirror_base; // tempdir content intentionally left to the OS
}

/// UTC date string of a day index (for the axis print only).
fn date_of(day_index: i64) -> String {
    // Civil-from-days (Howard Hinnant's algorithm) — display only.
    let z = day_index + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}
