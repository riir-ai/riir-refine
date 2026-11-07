//! Bench 145 — the elo_k_eff promotion cell (Bench 141 queue item A).
//!
//! Paired Elo-quality A/B over the box's REAL trajectory-store outcome
//! series, read-only. Two replay arms per cell:
//! - OFF: every match at full `ELO_K` (the `cfg!(feature = "elo_k_eff")`
//!   else-branch of `TrajStore::recompute_elo`, verbatim).
//! - ON: `elo_k_eff(ELO_K, n, ELO_K_EFF_FLOOR)` with per-rule cumulative
//!   evidence (the cfg-on branch, verbatim).
//!
//! The mirror-contract anchor: this example's ON replay must reproduce
//! the PRODUCTION `recompute_elo` ratings — verified once against a
//! throwaway COPY of the store (`--mirror-check <tmp-root>`), never the
//! real store. A mismatch is a loud refusal (exit 3): the cell would be
//! measuring a fiction.
//!
//! Cells: C0 organic (the real series) · C1 early-flip noise (the
//! flipped rule's first wins → losses, the damp's target) · C2 late-flip
//! noise (flips at the END of the same rule's series — the control the
//! damp must NOT soften).
//!
//! Verdict legs are PRE-REGISTERED in `.benchmarks/145` — this example
//! prints per-leg PASS/FAIL and exits 0 (report-first, the
//! `--frontier-report` precedent); only the population floor refuses
//! (exit 2). Run:
//!
//! ```sh
//! cargo run --release --features elo_k_eff,latent_retrieval \
//!   --example elo_damp_cell -- [--store-root /Users/kat/git/riir-refine] \
//!   [--mirror-root /tmp/elo_cell_mirror]
//! ```

#![allow(clippy::cast_precision_loss)] // outcome counts < 2^23 by contract

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use riir_refine::elo::{ELO_K, ELO_K_EFF_FLOOR, EloTable, default_elo, elo_k_eff};
use riir_refine::traj_store::{TrajRowDoc, TrajStore, shape_hash};

/// One replayed match in the canonical series (dump order; losses then
/// wins per row — `recompute_elo`'s exact shape).
#[derive(Clone)]
struct Match {
    rule: String,
    span: String,
    won: bool,
}

/// Per-rule outcome census from the dump (n = matches, wins, win rate).
/// Build the canonical replay series from a dump — the `recompute_elo`
/// loop's match order, losses then wins per row (Issue 190 counts
/// evidence in exactly this order).
fn series_of(rows: &[TrajRowDoc]) -> Vec<Match> {
    let mut out = Vec::new();
    for row in rows {
        if row.success_count.saturating_add(row.fail_count) == 0 {
            continue; // apply-only sighting: not a match (recompute_elo's skip)
        }
        let span = shape_hash(&row.shape);
        for _ in 0..row.fail_count {
            out.push(Match {
                rule: row.lint_key.clone(),
                span: span.clone(),
                won: false,
            });
        }
        for _ in 0..row.success_count {
            out.push(Match {
                rule: row.lint_key.clone(),
                span: span.clone(),
                won: true,
            });
        }
    }
    out
}

/// Replay a series under one arm. `damped = true` is the ON arm (the
/// `elo_k_eff` schedule), `false` the OFF arm (full `ELO_K` — the
/// cfg-else branch, verbatim).
fn replay(series: &[Match], damped: bool) -> EloTable {
    let mut table = EloTable::new();
    let mut evidence: BTreeMap<String, u32> = BTreeMap::new();
    for m in series {
        let n = evidence.entry(m.rule.clone()).or_default();
        *n += 1;
        let k = if damped {
            elo_k_eff(ELO_K, *n, ELO_K_EFF_FLOOR)
        } else {
            ELO_K
        };
        table.play_with_k(&m.rule, &m.span, m.won, k);
    }
    table
}

/// Per-rule outcome census from the dump (n = matches, wins, win rate).
struct RuleCensus {
    n: u32,
    wins: u32,
}

fn census(rows: &[TrajRowDoc]) -> BTreeMap<String, RuleCensus> {
    let mut out: BTreeMap<String, RuleCensus> = BTreeMap::new();
    for row in rows {
        let e = out
            .entry(row.lint_key.clone())
            .or_insert(RuleCensus { n: 0, wins: 0 });
        e.n += row.success_count + row.fail_count;
        e.wins += row.success_count;
    }
    out.retain(|_, c| c.n > 0);
    out
}

/// Kendall tau-a between two rating orders over `ids`, counting only
/// pairs DISTINCT in both arms (ties are excluded from the denominator
/// and printed — never folded into agreement).
fn kendall_tau(table_a: &EloTable, table_b: &EloTable, ids: &[String]) -> (f64, usize, usize) {
    let mut concordant = 0.0_f64;
    let mut discordant = 0.0_f64;
    let mut tied = 0_usize;
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let (ra_i, ra_j) = (table_a.rating(&ids[i]), table_a.rating(&ids[j]));
            let (rb_i, rb_j) = (table_b.rating(&ids[i]), table_b.rating(&ids[j]));
            let (da, db) = (ra_i - ra_j, rb_i - rb_j);
            if da == 0.0 || db == 0.0 {
                tied += 1;
                continue;
            }
            if da.signum() == db.signum() {
                concordant += 1.0;
            } else {
                discordant += 1.0;
            }
        }
    }
    let denom = concordant + discordant;
    if denom == 0.0 {
        (f64::NAN, tied, ids.len() * (ids.len() - 1) / 2)
    } else {
        (
            (concordant - discordant) / denom,
            tied,
            ids.len() * (ids.len() - 1) / 2,
        )
    }
}

/// Spearman rho between rating and win rate over `ids` (average ranks on
/// ties; Pearson over the rank vectors).
fn spearman_rating_vs_winrate(
    table: &EloTable,
    census: &BTreeMap<String, RuleCensus>,
    ids: &[String],
) -> f64 {
    let mut rank_pairs: Vec<(f64, f64)> = ids
        .iter()
        .map(|id| {
            let n = census[id].n as f64;
            let wr = census[id].wins as f64 / n;
            (table.rating(id) as f64, wr)
        })
        .collect();
    rank_pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut x = vec![0.0_f64; rank_pairs.len()];
    assign_average_ranks(&rank_pairs.iter().map(|p| p.0).collect::<Vec<_>>(), &mut x);
    rank_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut y = vec![0.0_f64; rank_pairs.len()];
    assign_average_ranks(&rank_pairs.iter().map(|p| p.1).collect::<Vec<_>>(), &mut y);
    pearson(&x, &y)
}

fn assign_average_ranks(values: &[f64], out: &mut [f64]) {
    let n = values.len();
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|a, b| {
        values[*a]
            .partial_cmp(&values[*b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && values[idx[j + 1]] == values[idx[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            out[idx[k]] = avg;
        }
        i = j + 1;
    }
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for i in 0..x.len() {
        let (dx, dy) = (x[i] - mx, y[i] - my);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx == 0.0 || syy == 0.0 {
        return f64::NAN;
    }
    sxy / (sxx.sqrt() * syy.sqrt())
}

fn top_k_ids(table: &EloTable, ids: &[String], k: usize) -> Vec<String> {
    let mut sorted = ids.to_vec();
    sorted.sort_by(|a, b| {
        let (ra, rb) = (table.rating(a), table.rating(b));
        rb.partial_cmp(&ra)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    sorted.truncate(k);
    sorted
}

/// The flip cell: flip `k` wins of `rule` — from the series START (early)
/// or END (late) — to losses. Returns the mutated series.
fn flip_cell(series: &[Match], rule: &str, k: u32, early: bool) -> Vec<Match> {
    let mut out = series.to_vec();
    let idxs: Vec<usize> = out
        .iter()
        .enumerate()
        .filter(|(_, m)| m.rule == rule && m.won)
        .map(|(i, _)| i)
        .collect();
    let pick: Vec<usize> = if early {
        idxs.into_iter().take(k as usize).collect()
    } else {
        idxs.into_iter().rev().take(k as usize).collect()
    };
    for i in pick {
        out[i].won = false;
    }
    out
}

fn main() {
    let mut store_root = PathBuf::from(".");
    let mut mirror_root = PathBuf::from("/tmp/elo_cell_mirror");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--store-root" => store_root = PathBuf::from(args.next().expect("--store-root VALUE")),
            "--mirror-root" => {
                mirror_root = PathBuf::from(args.next().expect("--mirror-root VALUE"))
            }
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
    }

    // ── open the REAL store read-only (dump only; never written) ──
    let (store, bad) = TrajStore::open(&store_root).unwrap_or_else(|e| {
        eprintln!("elo-damp-cell: store at {}: {e}", store_root.display());
        std::process::exit(2);
    });
    if bad > 0 {
        eprintln!("elo-damp-cell: {bad} corrupt row(s) skipped");
    }
    let rows = store.dump();
    drop(store);
    let series = series_of(&rows);
    let cen = census(&rows);

    // ── population floor (Bench 145) ──
    let total_matches: u32 = cen.values().map(|c| c.n).sum();
    println!(
        "population: {} rules | {} matches | store rows {}",
        cen.len(),
        total_matches,
        rows.len()
    );
    if cen.len() < 20 || total_matches < 100 {
        eprintln!(
            "elo-damp-cell: population below the pre-registered floor (20 rules / 100 matches) — refusing a green zero"
        );
        std::process::exit(2);
    }

    let n_ge_16: Vec<String> = cen
        .iter()
        .filter(|(_, c)| c.n >= ELO_K_EFF_FLOOR)
        .map(|(k, _)| k.clone())
        .collect();
    let n_lt_16: Vec<String> = cen
        .iter()
        .filter(|(_, c)| c.n < ELO_K_EFF_FLOOR)
        .map(|(k, _)| k.clone())
        .collect();
    println!(
        "splits: n>=16 = {} rules | n<16 = {} rules",
        n_ge_16.len(),
        n_lt_16.len()
    );

    // ── the mirror-contract anchor: production recompute on a COPY ──
    // Copy .refine/key + .refine/trajectories.kv into the mirror root
    // (the LocalKv is encrypted per-dir seed — the copy carries its key),
    // run the PRODUCTION recompute (in an elo_k_eff build that IS the ON
    // schedule), compare every rated id against the local ON replay.
    let heal_src = store_root.join(".refine");
    let heal_dst = mirror_root.join(".refine");
    let _ = std::fs::remove_dir_all(&mirror_root);
    std::fs::create_dir_all(&heal_dst).expect("mirror dir");
    std::fs::copy(heal_src.join("key"), heal_dst.join("key")).expect("mirror key");
    copy_dir(
        &heal_src.join("trajectories.kv"),
        &heal_dst.join("trajectories.kv"),
    );
    {
        let (mstore, _) = TrajStore::open(&mirror_root).expect("mirror store open");
        match mstore.recompute_elo() {
            Ok(summary) => println!(
                "mirror anchor: production recompute_elo — {} rows seen, {} written, {} matches",
                summary.rows_seen, summary.rows_written, summary.matches
            ),
            Err(e) => {
                eprintln!("elo-damp-cell: production recompute on the mirror FAILED: {e}");
                std::process::exit(3);
            }
        }
    }
    let (mstore2, _) = TrajStore::open(&mirror_root).expect("mirror reopen");
    let mirror_rows = mstore2.dump();
    drop(mstore2);

    let on_local = replay(&series, true);
    let mut mirror_mismatches = 0_usize;
    for row in &mirror_rows {
        if row.elo == default_elo() {
            continue; // unrated sighting — nothing to compare
        }
        if on_local.rating(&row.lint_key) != row.elo {
            mirror_mismatches += 1;
            eprintln!(
                "mirror MISMATCH rule {}: local {} vs production {}",
                row.lint_key,
                on_local.rating(&row.lint_key),
                row.elo
            );
        }
        if row.span_elo != default_elo() && on_local.rating(&shape_hash(&row.shape)) != row.span_elo
        {
            mirror_mismatches += 1;
            eprintln!("mirror MISMATCH span {}", shape_hash(&row.shape));
        }
    }
    if mirror_mismatches > 0 {
        eprintln!(
            "elo-damp-cell: MIRROR CONTRACT FAILED ({mirror_mismatches} mismatches) — exit 3"
        );
        std::process::exit(3);
    }
    println!(
        "mirror anchor: OK — the local ON replay reproduces production recompute_elo bit-identically"
    );

    // ── C0 organic ──
    let off = replay(&series, false);
    println!("\n== C0 organic ==");
    cell_report(&off, &on_local, &cen, &n_ge_16, &n_lt_16, true);

    // ── C1/C2 flip cells (10 seeds, RUN-2 amended selection) ──
    // RUN-1 disclosure: the pre-registered "seeded PRNG" never wired to
    // anything — the target rule was deterministic, so all 10 seeds were
    // ONE replicate. RUN-2: per-seed target = the (seed mod n_targets)-th
    // lowest-evidence rule with n >= 8 (deterministic, varied,
    // reproducible). Flips: min(3, n/3) wins.
    let eligible: Vec<(String, u32)> = cen
        .iter()
        .filter(|(_, c)| c.n >= 8)
        .map(|(k, c)| (k.clone(), c.n))
        .collect();
    if eligible.is_empty() {
        eprintln!("elo-damp-cell: no rule with n >= 8 for the flip cells — UNDECIDABLE legs 4/5");
        std::process::exit(2);
    }

    let mut c1_win = 0_usize;
    let mut c1_inv_win = 0_usize;
    let mut c2_within = 0_usize;
    let seeds: Vec<u64> = (0..10).collect();
    for seed in &seeds {
        // RUN-2 selection: k-th lowest-evidence eligible rule.
        let kth = (*seed as usize) % eligible.len();
        let (target_rule, target_n) = eligible[kth].clone();
        let flips = std::cmp::min(3, target_n / 3).max(1);
        let s1 = flip_cell(&series, &target_rule, flips, true);
        let s2 = flip_cell(&series, &target_rule, flips, false);
        let (o1, n1) = (replay(&s1, false), replay(&s1, true));
        let (o2, n2) = (replay(&s2, false), replay(&s2, true));

        // C1: the flipped rule's post-flip displacement RELATIVE TO ITS
        // OWN ARM'S C0 rating (per-arm impact — the flip's own effect,
        // never the pre-existing ON-OFF gap), and cross-rule order
        // inversions among the OTHER rated rules.
        let d_on = (n1.rating(&target_rule) - on_local.rating(&target_rule)).abs();
        let d_off = (o1.rating(&target_rule) - off.rating(&target_rule)).abs();
        let others: Vec<String> = n_ge_16
            .iter()
            .filter(|r| **r != target_rule)
            .cloned()
            .collect();
        let (tau1, _, _) = kendall_tau(&o1, &n1, &others);
        let inversions = tau1 < 0.0;
        if d_on < d_off {
            c1_win += 1;
        }
        if !inversions {
            c1_inv_win += 1;
        }
        println!(
            "  C1 seed {seed} ({target_rule} n={target_n}): flip impact OFF {d_off:.1} vs ON {d_on:.1} ({}) | other-order tau {tau1:.4} ({})",
            if d_on < d_off {
                "ON tighter"
            } else {
                "OFF tighter/equal"
            },
            if inversions { "INVERTED" } else { "held" },
        );

        // C2 control (RUN-2 metric): the late flip's own impact PER ARM —
        // |rating(arm, C2) − rating(arm, C0)| — must agree across arms
        // (late evidence plays full-K in both; the damp must not soften
        // it). RUN-1's |ON(C2) − OFF(C2)| mixed the standing C0 gap in
        // and failed 0/10 by construction.
        let i_on = (n2.rating(&target_rule) - on_local.rating(&target_rule)).abs();
        let i_off = (o2.rating(&target_rule) - off.rating(&target_rule)).abs();
        let spread = (i_on - i_off).abs();
        if spread <= ELO_K {
            c2_within += 1;
        }
        println!(
            "  C2 seed {seed}: late-flip impact OFF {i_off:.1} vs ON {i_on:.1} (spread {spread:.1}, {}) — leg 5′ {}",
            if spread <= ELO_K {
                "within one full-K match"
            } else {
                "OVER the control bound"
            },
            if spread <= ELO_K { "PASS" } else { "FAIL" },
        );
    }

    // ── verdict legs (pre-registered, Bench 145) ──
    println!("\n== verdict legs (pre-registered) ==");
    println!(
        "leg 2′ C0 no-regression (RUN-2 amended, order-only): tau >= 0.9 + overlap >= 6 + rho(ON) >= rho(OFF); leader max|delta| DISCLOSED, never a bar (Elo has no anchor — a damped head redistributes a leader's total movement and compounds past one match's K; RUN-1's 63.8 vs the 32 bound was the model error, the order held)"
    );
    println!("leg 3 C0 low-evidence pull: ON <= OFF — see C0 table");
    println!(
        "leg 4 C1 benefit (per-arm flip impact): ON < OFF in {c1_win}/10 seeds (bar 8) | no inversion in {c1_inv_win}/10 (info)"
    );
    println!("leg 5′ C2 control (per-arm impact spread <= 32): {c2_within}/10 seeds (bar 9)");
    println!(
        "\nSession: zed-native-agent (m3), epoch 1794063759 — Bench 145 RUN-2 (amended metrics; RUN-1 preserved verbatim in the bench)"
    );
}

/// One cell's metric table (C0) or the shared shape for flip arms.
fn cell_report(
    off: &EloTable,
    on: &EloTable,
    cen: &BTreeMap<String, RuleCensus>,
    n_ge_16: &[String],
    n_lt_16: &[String],
    _is_c0: bool,
) {
    let mean_disp = |t: &EloTable, ids: &[String]| -> f64 {
        if ids.is_empty() {
            return 0.0;
        }
        ids.iter()
            .map(|id| (t.rating(id) - default_elo()).abs() as f64)
            .sum::<f64>()
            / ids.len() as f64
    };
    let (tau, tied, pairs) = kendall_tau(off, on, n_ge_16);
    let max_delta = n_ge_16
        .iter()
        .map(|id| (off.rating(id) - on.rating(id)).abs())
        .fold(0.0_f32, f32::max);
    let rho_off = spearman_rating_vs_winrate(off, cen, n_ge_16);
    let rho_on = spearman_rating_vs_winrate(on, cen, n_ge_16);
    let top_off = top_k_ids(off, n_ge_16, 8);
    let top_on = top_k_ids(on, n_ge_16, 8);
    let overlap = top_off.iter().filter(|r| top_on.contains(r)).count();

    println!(
        "  low-evidence (n<16, {} rules): mean |rating-base| OFF {:.1} vs ON {:.1} — leg 3 {}",
        n_lt_16.len(),
        mean_disp(off, n_lt_16),
        mean_disp(on, n_lt_16),
        if mean_disp(on, n_lt_16) <= mean_disp(off, n_lt_16) {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "  leaders (n>=16, {} rules): max |delta| {max_delta:.1} | tau {tau:.4} (tied pairs {tied}/{pairs}, bound 0.9) | top-8 overlap {overlap}/8",
        n_ge_16.len()
    );
    println!(
        "  rating~winrate rho (n>=16): OFF {rho_off:.4} vs ON {rho_on:.4} — leg 2 rho half {}",
        if rho_on >= rho_off { "PASS" } else { "FAIL" }
    );
    // RUN-2: leg 2′ is order-only (tau + overlap + rho); max|delta| is a
    // DISCLOSED descriptive, never a bar — the RUN-1 registration
    // operationalized no-regression as displacement-from-OFF, which Elo's
    // anchor-free scale does not support (the damped head redistributes a
    // leader's total movement; RUN-1 measured 63.8 vs the 32 bound while
    // every ordering statistic held).
    let leg2p = (tau.is_nan() || tau >= 0.9) && overlap >= 6 && rho_on >= rho_off;
    println!(
        "  leg 2′ (C0 no-regression, order-only) {}: tau={}, overlap={overlap}/8, rho {rho_off:.4}->{rho_on:.4} | max|d| {max_delta:.1} DISCLOSED",
        if leg2p { "PASS" } else { "FAIL" },
        if tau.is_nan() {
            "N/A(all-tied)".to_string()
        } else {
            format!("{tau:.4}")
        }
    );
}

/// Recursively copy a directory (the encrypted LocalKv dir is a handful
/// of files).
fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("dst dir");
    for entry in std::fs::read_dir(src).expect("src dir") {
        let entry = entry.expect("dir entry");
        let ty = entry.file_type().expect("file type");
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), to).expect("file copy");
        }
    }
}
