//! Issue 092 T1 — cross-domain rule-corpus alignment PoC (defend-wrong).
//!
//! Aligns the clippy_lints (49) × rust_perf (126) rule-similarity spaces via
//! `katgpt_core::gw_alignment` and compares the correspondence map against a
//! name/keyword-similarity baseline at matched review budget. OFFLINE corpus
//! curation only — no heal-path, no router (cross_domain_routing.md's
//! scoping verdict stands).
//!
//! Arms (Issue 092 T1):
//! - (a) GW correspondence map: trigram-profile embeddings over
//!   `buggy_code`+`fixed_code` ONLY (code shape — names are withheld from
//!   this arm, else the comparison against the name baseline is circular),
//!   cosine distance matrices, GW coupling over deterministic perf halves
//!   (GW_MAX=64 < 126; even/odd split, unioned couplings).
//! - (b) Name/keyword baseline: token Jaccard over rule identifiers
//!   (clippy lint_name tokens vs perf rule_id + name + category tokens) —
//!   today's name/convention-level dedup, as a ranking.
//! - (c) No-alignment: base rate from a deterministic stride sample.
//!
//! Ground truth (hand-labeled, criteria pre-registered in the bench note):
//! top-60 of each arm + stride sample + mechanically-extracted known
//! negatives (rust_perf `context_notes` "Distinct from <clippy-lint>"
//! sentences — corpus-authored non-correspondences).
//!
//! Modes:
//! ```text
//! cargo run --example gw_corpus_poc --features gw_corpus --sheet > sheet.md
//! #   ... hand-label pairs into labels.txt (id | id | TRUE|FALSE | note)
//! cargo run --example gw_corpus_poc --features gw_corpus --score labels.txt
//! ```
//!
//! Determinism: BTree*/sorted iteration everywhere, fixed stride sample, GW
//! solve is bit-deterministic by construction (fixed iters, no RNG).

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use katgpt_core::gw_alignment::{gw_coupling, GwScratch, GW_MAX};
use riir_refine::domains::clippy_lints::ClippyFixEntry;
use riir_refine::domains::clippy_lints::ClippyLintDomain;
use riir_refine::domains::rust_perf::PerfRule;
use riir_refine::domains::rust_perf::RustPerfDomain;
use riir_refine::domain::HealDomain;

/// Review budget per arm (matched): top-K pairs a maintainer would read.
const BUDGET: usize = 60;
/// Stride-sample size for the no-alignment base rate.
const RANDOM_N: usize = 60;
/// Per-domain distance matrices are normalized by their mean off-diagonal
/// distance before GW (scale-invariant SHAPE comparison — the two corpora's
/// snippet lengths differ, and GW's quadratic form is scale-sensitive).
const NORMALIZE_SCALE: bool = true;

#[derive(Clone)]
struct Pair {
    clippy_idx: usize,
    perf_idx: usize,
    gw: f64,
    base: f64,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "--sheet".to_string());
    let clippy: Vec<ClippyFixEntry> = ClippyLintDomain::build_corpus();
    let perf: Vec<PerfRule> = RustPerfDomain::build_corpus();
    assert!(
        clippy.len() <= GW_MAX && perf.len() / 2 <= GW_MAX,
        "corpus grew past GW_MAX half-split capacity"
    );

    // ---- embeddings + distance matrices ----
    let clippy_prof: Vec<BTreeMap<[u8; 3], f64>> = clippy
        .iter()
        .map(|e| trigram_profile(&format!("{}\n=>\n{}", e.buggy_code, e.fixed_code)))
        .collect();
    let perf_prof: Vec<BTreeMap<[u8; 3], f64>> = perf
        .iter()
        .map(|e| trigram_profile(&format!("{}\n=>\n{}", e.buggy_code, e.fixed_code)))
        .collect();
    let da = dist_matrix(&clippy_prof);
    let db = dist_matrix(&perf_prof);
    let (da, db) = if NORMALIZE_SCALE {
        (normalize(&da), normalize(&db))
    } else {
        (da, db)
    };

    // ---- arm (a): GW coupling over deterministic even/odd perf halves ----
    let mut gw = vec![0.0f64; clippy.len() * perf.len()];
    let a_refs: Vec<&[f32]> = da.iter().map(|r| r.as_slice()).collect();
    let mut scratch = GwScratch::new();
    for half in 0..2 {
        let members: Vec<usize> = (0..perf.len()).filter(|j| j % 2 == half).collect();
        let d_half: Vec<Vec<f32>> = members
            .iter()
            .map(|&j| members.iter().map(|&k| db[j][k]).collect())
            .collect();
        let b_refs: Vec<&[f32]> = d_half.iter().map(|r| r.as_slice()).collect();
        let (t, loss) = gw_coupling(&a_refs, &b_refs, &mut scratch)
            .unwrap_or_else(|e| panic!("GW half {half}: {e}"));
        eprintln!(
            "# gw half {half}: n={} m={} loss={loss:.6} score={:.6}",
            clippy.len(),
            members.len(),
            katgpt_core::gw_alignment::score_from_loss(loss)
        );
        for (i, row) in t.iter().enumerate() {
            for (local, &mass) in row.iter().enumerate() {
                gw[i * perf.len() + members[local]] = f64::from(mass);
            }
        }
    }

    // ---- arm (b): name/keyword token Jaccard ----
    let clippy_tok: Vec<BTreeSet<String>> = clippy
        .iter()
        .map(|e| tokens(&e.lint_name))
        .collect();
    let perf_tok: Vec<BTreeSet<String>> = perf
        .iter()
        .map(|e| {
            let mut s = tokens(&e.rule_id);
            s.extend(tokens(&e.name));
            s.extend(tokens(&e.category));
            s
        })
        .collect();
    let mut base = vec![0.0f64; gw.len()];
    for i in 0..clippy.len() {
        for j in 0..perf.len() {
            base[i * perf.len() + j] = jaccard(&clippy_tok[i], &perf_tok[j]);
        }
    }

    let mut pairs = Vec::with_capacity(clippy.len() * perf.len());
    for i in 0..clippy.len() {
        for j in 0..perf.len() {
            pairs.push(Pair {
                clippy_idx: i,
                perf_idx: j,
                gw: gw[i * perf.len() + j],
                base: base[i * perf.len() + j],
            });
        }
    }

    // ---- known negatives: "Distinct from <clippy-lint>" in perf notes ----
    let clippy_names: BTreeSet<String> = clippy.iter().map(|e| e.lint_name.clone()).collect();
    let known_neg: BTreeSet<(usize, usize)> = perf
        .iter()
        .enumerate()
        .flat_map(|(j, r)| {
            let Some(notes) = r.context_notes.as_deref() else {
                return vec![];
            };
            let mut out = vec![];
            for sent in notes.split('.') {
                if !sent.contains("Distinct from") && !sent.contains("distinct from") {
                    continue;
                }
                for tok in sent.split(|c: char| !c.is_ascii_alphanumeric()) {
                    if tok.is_empty() {
                        continue;
                    }
                    let snake = tok.replace('-', "_");
                    if clippy_names.contains(&snake) {
                        if let Some(i) = clippy.iter().position(|e| e.lint_name == snake) {
                            out.push((i, j));
                        }
                    }
                }
            }
            out
        })
        .collect();

    // ---- ranked arms ----
    let mut by_gw = pairs.clone();
    let pair_key = |p: &Pair, clippy: &[ClippyFixEntry], perf: &[PerfRule]| {
        (
            clippy[p.clippy_idx].lint_name.clone(),
            perf[p.perf_idx].rule_id.clone(),
        )
    };
    by_gw.sort_by(|x, y| {
        y.gw.partial_cmp(&x.gw)
            .unwrap()
            .then_with(|| pair_key(x, &clippy, &perf).cmp(&pair_key(y, &clippy, &perf)))
    });
    let mut by_base = pairs.clone();
    by_base.sort_by(|x, y| {
        y.base.partial_cmp(&x.base)
            .unwrap()
            .then_with(|| pair_key(x, &clippy, &perf).cmp(&pair_key(y, &clippy, &perf)))
    });

    match mode.as_str() {
        "--sheet" => emit_sheet(&clippy, &perf, &by_gw, &by_base, &known_neg),
        "--score" => {
            let path = args.next().unwrap_or_else(|| "labels.txt".to_string());
            score(&clippy, &perf, &by_gw, &by_base, &known_neg, &path);
        }
        m => {
            eprintln!("unknown mode {m:?} (use --sheet | --score <labels>)");
            std::process::exit(2);
        }
    }
}

// ================= instrumentation =================

/// Modelless rule embedding: trigram counts over the normalized code text.
fn trigram_profile(text: &str) -> BTreeMap<[u8; 3], f64> {
    let norm: Vec<u8> = {
        let lower = text.to_ascii_lowercase();
        let mut out = Vec::with_capacity(lower.len());
        let mut last_ws = false;
        for b in lower.bytes() {
            let ws = b == b' ' || b == b'\n' || b == b'\t' || b == b'\r';
            if ws {
                if !last_ws {
                    out.push(b' ');
                }
            } else {
                out.push(b);
            }
            last_ws = ws;
        }
        out
    };
    let mut m = BTreeMap::new();
    if norm.len() < 3 {
        return m;
    }
    for w in norm.windows(3) {
        *m.entry([w[0], w[1], w[2]]).or_insert(0.0) += 1.0;
    }
    m
}

fn cosine(a: &BTreeMap<[u8; 3], f64>, b: &BTreeMap<[u8; 3], f64>) -> f64 {
    let (small, big) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let mut dot = 0.0;
    for (k, v) in small {
        if let Some(w) = big.get(k) {
            dot += v * w;
        }
    }
    if dot == 0.0 {
        return 0.0;
    }
    let na: f64 = a.values().map(|v| v * v).sum();
    let nb: f64 = b.values().map(|v| v * v).sum();
    dot / (na.sqrt() * nb.sqrt())
}

fn dist_matrix(profiles: &[BTreeMap<[u8; 3], f64>]) -> Vec<Vec<f32>> {
    let n = profiles.len();
    let mut d = vec![vec![0.0f32; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let v = (1.0 - cosine(&profiles[i], &profiles[j])).clamp(0.0, 2.0) as f32;
            d[i][j] = v;
            d[j][i] = v;
        }
    }
    d
}

/// Divide by the mean off-diagonal distance (scale-invariant shape).
fn normalize(d: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let n = d.len();
    let mut sum = 0.0f64;
    for row in d {
        for v in row {
            sum += f64::from(*v);
        }
    }
    let mean = (sum / ((n * n - n) as f64)).max(1e-9);
    d.iter().map(|r| r.iter().map(|v| (f64::from(*v) / mean) as f32).collect()).collect()
}

/// Lowercase identifier tokens: split on `_`, `-`, whitespace, and strip
/// non-alphanumerics (also drops Rust sigils from prose names).
fn tokens(s: &str) -> BTreeSet<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !(c.is_ascii_alphanumeric()))
        .filter(|t| t.len() > 1)
        .map(str::to_string)
        .collect()
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    if inter == 0.0 {
        return 0.0;
    }
    inter / (a.len() + b.len() - inter as usize) as f64
}

fn one_line(s: &str, max: usize) -> String {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.len() <= max {
        t
    } else {
        format!("{}…", &t[..max - 1])
    }
}

// ================= sheet =================

fn emit_sheet(
    clippy: &[ClippyFixEntry],
    perf: &[PerfRule],
    by_gw: &[Pair],
    by_base: &[Pair],
    known_neg: &BTreeSet<(usize, usize)>,
) {
    let mut sheeted: BTreeSet<(usize, usize)> = BTreeSet::new();
    println!("# GW corpus PoC — labeling sheet (deterministic; Issue 092 T1)");
    println!("# Label criteria: TRUE = same trigger-shape AND/OR same fix-transform family");
    println!("# (a maintainer shown both says \"same rule wearing domain vocabulary\").");
    println!("# Label format: `<clippy_lint> | <perf_rule> | TRUE|FALSE | <one-line note>`");

    println!("\n## Section A — GW arm top {BUDGET}");
    for (rank, p) in by_gw.iter().take(BUDGET).enumerate() {
        print_pair(clippy, perf, p, "gw", rank, known_neg);
        sheeted.insert((p.clippy_idx, p.perf_idx));
    }

    println!("\n## Section B — baseline arm top {BUDGET}");
    for (rank, p) in by_base.iter().take(BUDGET).enumerate() {
        print_pair(clippy, perf, p, "base", rank, known_neg);
        sheeted.insert((p.clippy_idx, p.perf_idx));
    }

    println!("\n## Section C — deterministic stride sample ({RANDOM_N}, base rate)");
    let stride = by_gw.len() / RANDOM_N;
    let mut taken = 0;
    for k in 0.. {
        if taken >= RANDOM_N || k * stride >= by_gw.len() {
            break;
        }
        let p = &by_gw[(k * stride + stride / 2) % by_gw.len()];
        if sheeted.insert((p.clippy_idx, p.perf_idx)) {
            print_pair(clippy, perf, p, "rand", taken, known_neg);
            taken += 1;
        }
    }
    eprintln!("# stride sample: {taken} pairs (overlap-skipped)");

    println!("\n## Section D — corpus-authored known negatives (prefalse)");
    for &(i, j) in known_neg.iter() {
        if sheeted.insert((i, j)) {
            println!(
                "- KNOWNNEG `{}` × `{}` — perf context_notes asserts Distinct-from",
                clippy[i].lint_name, perf[j].rule_id
            );
        }
    }
}

fn print_pair(
    clippy: &[ClippyFixEntry],
    perf: &[PerfRule],
    p: &Pair,
    arm: &str,
    rank: usize,
    _known_neg: &BTreeSet<(usize, usize)>,
) {
    let c = &clippy[p.clippy_idx];
    let r = &perf[p.perf_idx];
    println!(
        "\n### [{arm}#{rank}] `{}` ({}: {}) × `{}` ({}) — gw={:.5} base={:.3}",
        c.lint_name,
        c.lint_group,
        one_line(&c.buggy_code, 90),
        r.rule_id,
        one_line(&r.name, 60),
        p.gw,
        p.base
    );
    println!("- C buggy: {}", one_line(&c.buggy_code, 160));
    println!("- C fixed: {}", one_line(&c.fixed_code, 160));
    println!("- P buggy: {}", one_line(&r.buggy_code, 160));
    println!("- P fixed: {}", one_line(&r.fixed_code, 160));
}

// ================= scoring =================

fn score(
    clippy: &[ClippyFixEntry],
    perf: &[PerfRule],
    by_gw: &[Pair],
    by_base: &[Pair],
    known_neg: &BTreeSet<(usize, usize)>,
    labels_path: &str,
) {
    let raw = std::fs::read_to_string(labels_path)
        .unwrap_or_else(|e| panic!("read {labels_path}: {e}"));
    // (clippy_lint, perf_rule) -> true?
    let mut labels: BTreeMap<(String, String), bool> = BTreeMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(str::trim).collect();
        if parts.len() < 3 {
            eprintln!("# skipping malformed label line: {line}");
            continue;
        }
        let verdict = match parts[2].to_ascii_uppercase().as_str() {
            "TRUE" => true,
            "FALSE" => false,
            other => {
                eprintln!("# skipping label with verdict {other:?}: {line}");
                continue;
            }
        };
        labels.insert((parts[0].to_string(), parts[1].to_string()), verdict);
    }
    // Known negatives are FALSE by corpus evidence even if unlabeled.
    for &(i, j) in known_neg {
        labels.entry((clippy[i].lint_name.clone(), perf[j].rule_id.clone()))
            .or_insert(false);
    }
    eprintln!("# labels loaded: {} (incl. {} corpus known-negatives)", labels.len(), known_neg.len());

    let universe = (clippy.len() * perf.len()) as f64;
    let key = |p: &Pair| {
        (
            clippy[p.clippy_idx].lint_name.clone(),
            perf[p.perf_idx].rule_id.clone(),
        )
    };
    fn labeled_true(list: &[Pair], labels: &BTreeMap<(String, String), bool>, key: impl Fn(&Pair) -> (String, String)) -> usize {
        list.iter()
            .filter(|p| labels.get(&key(p)) == Some(&true))
            .count()
    }

    let gw_true = labeled_true(&by_gw[..BUDGET.min(by_gw.len())], &labels, key);
    let base_true = labeled_true(&by_base[..BUDGET.min(by_base.len())], &labels, key);

    // base rate over the stride section: pairs whose label exists and that
    // appeared ONLY via the random section can't be distinguished here, so
    // estimate from labeled pairs whose gw rank is beyond BOTH arms' budgets
    // — i.e. unlabeled-by-arms pairs found in the sheet's section C. Those
    // are exactly the pairs with labels but rank >= BUDGET in both arms.
    let in_budget = |p: &Pair| -> bool {
        let k = key(p);
        let g = by_gw.iter().take(BUDGET).any(|q| key(q) == k);
        let b = by_base.iter().take(BUDGET).any(|q| key(q) == k);
        g || b
    };
    let mut rand_n = 0usize;
    let mut rand_true = 0usize;
    let stride = by_gw.len() / RANDOM_N;
    for k in 0.. {
        if rand_n >= RANDOM_N || k * stride >= by_gw.len() {
            break;
        }
        let p = &by_gw[(k * stride + stride / 2) % by_gw.len()];
        if in_budget(p) {
            continue; // overlap pair — its label is biased by arm membership
        }
        if let Some(&v) = labels.get(&key(p)) {
            rand_n += 1;
            if v {
                rand_true += 1;
            }
        }
    }
    let hat_p = if rand_n > 0 { rand_true as f64 / rand_n as f64 } else { 0.0 };
    let total_true_est = universe * hat_p;

    println!("== Issue 092 T1 PoC — defend-wrong score ==");
    println!("universe: {} × {} = {} pairs; budget/arm = {BUDGET}", clippy.len(), perf.len(), universe as usize);
    println!("labels: {} (known-negatives folded as FALSE)", labels.len());
    println!();
    println!("| arm | trues in top-{BUDGET} | precision@{BUDGET} | recall est. |");
    println!("|---|---|---|---|");
    println!("| (a) GW coupling | {gw_true} | {:.3} | {} |", gw_true as f64 / BUDGET as f64, recall_str(gw_true, total_true_est, hat_p));
    println!("| (b) name/keyword | {base_true} | {:.3} | {} |", base_true as f64 / BUDGET as f64, recall_str(base_true, total_true_est, hat_p));
    println!("| (c) no-alignment | — | base rate {hat_p:.3} | expected finds {:.1} |", BUDGET as f64 * hat_p);
    println!();
    println!("base-rate sample: {rand_true}/{rand_n} stride pairs TRUE → p̂ = {hat_p:.3}, total-true est ≈ {total_true_est:.0}");
    if rand_n < 20 {
        println!("⚠ small base-rate sample ({rand_n}) — recall estimates are weak; read precision first.");
    }

    // capability axis: GW-exclusive finds (baseline score 0)
    let gw_excl: Vec<&Pair> = by_gw
        .iter()
        .take(BUDGET)
        .filter(|p| p.base == 0.0 && labels.get(&key(p)) == Some(&true))
        .collect();
    println!("\nGW-exclusive TRUEs (baseline score 0): {}", gw_excl.len());
    for p in gw_excl.iter().take(15) {
        println!("  - `{}` × `{}` (gw={:.5})", clippy[p.clippy_idx].lint_name, perf[p.perf_idx].rule_id, p.gw);
    }

    // known-negative violations in either arm's budget
    for (name, list) in [("gw", &by_gw[..BUDGET]), ("base", &by_base[..BUDGET])] {
        let viol: Vec<&Pair> = list
            .iter()
            .filter(|p| known_neg.contains(&(p.clippy_idx, p.perf_idx)))
            .collect();
        println!("\n{name}-arm known-negative violations (corpus asserts distinct): {}", viol.len());
        for p in viol.iter().take(10) {
            println!("  - `{}` × `{}` ({:.5})", clippy[p.clippy_idx].lint_name, perf[p.perf_idx].rule_id, if name == "gw" { p.gw } else { p.base });
        }
    }
}

fn recall_str(found: usize, total_est: f64, hat_p: f64) -> String {
    if total_est <= 0.0 {
        if found == 0 {
            "n/a (no positives anywhere)".to_string()
        } else {
            "denominator est. 0 (p̂=0) — precision only".to_string()
        }
    } else {
        format!("{:.3} (est; p̂={hat_p:.3})", found as f64 / total_est)
    }
}
