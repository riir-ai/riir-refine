//! Issue 201 — the ρ-per-embedder alignability read over the frozen
//! `format_pairs_v1` fixture (katgpt-rs Issue 978 T5 consumer lane;
//! REPORT-ONLY — nothing arms here, no exit-code coupling to any fixer
//! lane).
//!
//! # What one run prints
//!
//! 1. **Report rows** — ρ (the rule-explained variance fraction,
//!    katgpt-core `nuisance_fraction`) over the fixture's points grouped by
//!    `lint_key` rule family, one row per embedder candidate: the shipped
//!    Full-8 modelless embedder, Structural-4 (hash dims zeroed — the
//!    `LatentChoice::Structural4` zero-locality precedent), Hash-4 (the
//!    structural dims zeroed). Slicing dims is a projection of the shipped
//!    embedder's output, never a fork of it. Hash-4 doubles as the measured
//!    noise baseline: BLAKE3 buckets are content-addressed, so
//!    rule-structure there is fixture noise, not signal.
//! 2. **The pre-registered replay** — katgpt-rs Issue 978 T4's promotion bar
//!    needs a consumer showing the gate refuses a doomed transport BEFORE
//!    the transport's compute is spent. This lane's doomed transport is the
//!    Issue-162/Bench-115 projected-KNN lane: Bench 115 measured NO
//!    transferable low-rank formatting subspace (pair differences isotropic
//!    around one k=1 outlier axis, ~2% holdout energy) and the lane was NOT
//!    ARMED. The replay asks: would a ρ read have produced the same refusal
//!    at a fraction of the cost? The predictor is PINNED here and in
//!    `.issues/201_rho_embedder_alignability_lane.md` BEFORE the first run:
//!    group the pair differences (fix − shape embeds) by rule family, take
//!    ρ on the formatting-bearing dims (Structural-4), and
//!    **ρ_diff < 0.5 ⇒ predict NOT-ARMED; ≥ 0.5 ⇒ the predictor ABSTAINS**
//!    (rule-structured differences may still fail the cross-rule transfer
//!    test — the ρ gate can only refuse, never certify viability; an
//!    abstention sends the consumer to the full certificate). 0.5 is the
//!    majority-of-variance bar on a normalized variance fraction — chosen
//!    before any measurement, no tuning.
//! 3. **The cost ledger** — wall-clock(the ρ reads, post-embed) vs
//!    wall-clock(the certificate core the do-not-arm verdict skips: Σ
//!    build, ZCA whitener, whitened pair-difference moment, split, and the
//!    cross-rule holdout leakage — formatting_flip's N4 sequence, ONE
//!    rule-split direction over the SAME embeds, not a full census). The
//!    embed cost is COMMON to both paths (Bench 115's pair-flip census
//!    embeds first too), so the honest apples-to-apples halves are
//!    post-embed on both sides. Disclosed as a compute-CLASS comparison:
//!    Bench 115's trajectory stores are machine-local and not present on
//!    this box, so the FULL harness is not rerun — the certificate core is
//!    the arming decision's input.
//!
//! # Hygiene
//!
//! The fixture is the Issue-162 T1 freeze (`cea4d679`): 4,213 pairs / 37
//! families, sha256 `85dea25b…`. The run protocol verifies the file digest
//! in the shell (`shasum -a 256`) before invoking this example and records
//! it with the numbers — Bench 115's frozen-slice law (T2 fits against the
//! frozen slice, never a re-dump) binds this lane. In-binary tripwires:
//! exactly 4,213 rows and 37 families, plus the blake3 content digest
//! (sorted rows, formatting_flip's recipe) printed for the record.
//! Deterministic end to end: BTreeMap grouping, blake3 rule parity for the
//! fit/holdout split, no HashMap iteration anywhere.
//!
//! ```text
//! shasum -a 256 assets/format_pairs_v1.jsonl   # == 85dea25b… (run protocol)
//! cargo run --release --features latent_retrieval,nuisance_fraction,basis_cert \
//!   --example rho_embedder_report
//! ```

use std::collections::BTreeMap;
use std::time::Instant;

use riir_rag::{Embedder, ModellessEmbedder};

use katgpt_core::basis_split::{
    PairDifferenceMoment, Whitening, build_zca_whitener, held_out_leakage, split,
};
use katgpt_core::linalg::symmetric_eig::SymmetricEigScratch;
use katgpt_core::nuisance_fraction::{RhoReport, RhoScratch, rho_fraction};

/// The frozen fixture (Issue-162 T1, `cea4d679`): 4,213 usable pairs / 37
/// rule families, sha256 `85dea25b…`.
const FIXTURE_PATH: &str = "assets/format_pairs_v1.jsonl";
const EXPECT_ROWS: usize = 4_213;
const EXPECT_FAMILIES: usize = 37;

/// The pinned replay threshold (pre-registered — see the module docs).
const REFUSE_THRESHOLD: f64 = 0.5;

/// formatting_flip's disclosed shrinkage floor (the QL-phase singularity
/// guard), carried unchanged so the certificate-core stand-in is the same
/// compute class the bench ran.
const SHRINKAGE_REL: f64 = 1e-8;
/// The 4× eigengap posture (formatting_flip / reflex `basis_cert` contract).
const TAU_GAP: f64 = 4.0;
/// Below this many fit pairs the certificate refuses (formatting_flip law).
const MIN_FIT_PAIRS: usize = 8;

/// One mined pair: the rule family plus both endpoints.
struct Pair {
    lint_key: String,
    shape: String,
    fix: String,
}

/// An embedder candidate: a projection of the shipped Full-8 output (a
/// pinned subset kept), plus the dimension it measures at.
struct Candidate {
    name: &'static str,
    dims: &'static [usize],
    k: usize,
}

/// Full-8 = the shipped embedder verbatim; Structural-4 = dims 4..8 (DFT
/// rhythm, symbol density, nesting depth, line-length entropy — the
/// formatting-bearing dims); Hash-4 = dims 0..4 (content-addressed BLAKE3
/// buckets — the measured noise baseline).
const CANDIDATES: [Candidate; 3] = [
    Candidate {
        name: "full8",
        dims: &[0, 1, 2, 3, 4, 5, 6, 7],
        k: 8,
    },
    Candidate {
        name: "struct4",
        dims: &[4, 5, 6, 7],
        k: 4,
    },
    Candidate {
        name: "hash4",
        dims: &[0, 1, 2, 3],
        k: 4,
    },
];

impl Candidate {
    fn project(&self, v: &[f32; 8]) -> Vec<f32> {
        self.dims.iter().map(|&i| v[i]).collect()
    }
}

fn load_fixture() -> Vec<Pair> {
    let raw = std::fs::read_to_string(FIXTURE_PATH).expect("read fixture");
    let mut pairs: Vec<Pair> = Vec::new();
    for line in raw.lines() {
        let v: serde_json::Value = serde_json::from_str(line).expect("bad jsonl row");
        pairs.push(Pair {
            lint_key: v["lint_key"].as_str().expect("lint_key").to_owned(),
            shape: v["shape"].as_str().expect("shape").to_owned(),
            fix: v["fix"].as_str().expect("fix").to_owned(),
        });
    }
    assert_eq!(
        pairs.len(),
        EXPECT_ROWS,
        "fixture row drift — the frozen-slice law (issue 162 T1) binds this lane"
    );
    let mut families: Vec<&str> = pairs.iter().map(|p| p.lint_key.as_str()).collect();
    families.sort_unstable();
    families.dedup();
    assert_eq!(
        families.len(),
        EXPECT_FAMILIES,
        "fixture family drift — the frozen-slice law binds this lane"
    );
    // The blake3 content digest for the record (formatting_flip's recipe:
    // sorted rows, NUL-joined).
    let mut sorted: Vec<(&str, &str, &str)> = pairs
        .iter()
        .map(|p| (p.lint_key.as_str(), p.shape.as_str(), p.fix.as_str()))
        .collect();
    sorted.sort_unstable();
    let mut h = blake3::Hasher::new();
    for (k, b, a) in &sorted {
        h.update(k.as_bytes());
        h.update(b"\0");
        h.update(b.as_bytes());
        h.update(b"\0");
        h.update(a.as_bytes());
        h.update(b"\0");
    }
    println!(
        "fixture: {} pairs / {} families, row+family tripwires OK, blake3 content digest {}",
        pairs.len(),
        families.len(),
        h.finalize().to_hex()
    );
    pairs
}

/// One ρ read over family-grouped members; prints the row, returns the
/// report (None = the primitive refused loud — a RESULT, never a silent 0).
/// The caller MUST pass a scratch sized to exactly `k` (the scratch is
/// grow-only: reusing a k=8 scratch on 4-wide groups makes each PAIR of
/// vectors read as one 8-dim member — measured the hard way, first run).
fn rho_over(
    name: &str,
    groups: &BTreeMap<&str, Vec<Vec<f32>>>,
    k: usize,
    scratch: &mut RhoScratch,
) -> Option<RhoReport> {
    debug_assert_eq!(scratch.dim(), k, "scratch width must match the data width");
    scratch.resize(k);
    let flat: Vec<Vec<f32>> = groups
        .values()
        .map(|members| members.iter().flatten().copied().collect())
        .collect();
    let refs: Vec<&[f32]> = flat.iter().map(|v| &v[..]).collect();
    let members: usize = refs.iter().map(|g| g.len() / k).sum();
    match rho_fraction(&refs, scratch) {
        Ok(r) => {
            println!(
                "  {name:>13}: ρ={:.4}  (G={}, k={k}, n={members}) between_trace={:.4}",
                r.rho,
                groups.len(),
                r.between_trace_corrected
            );
            Some(r)
        }
        Err(e) => {
            println!("  {name:>13}: REFUSED loud: {e}");
            None
        }
    }
}

/// Group texts by rule family under a candidate projection.
fn group_points<'a>(
    pairs: &'a [Pair],
    embeds: &[([f32; 8], [f32; 8])],
    c: &Candidate,
) -> BTreeMap<&'a str, Vec<Vec<f32>>> {
    let mut groups: BTreeMap<&str, Vec<Vec<f32>>> = BTreeMap::new();
    for (p, (b, a)) in pairs.iter().zip(embeds) {
        let g = groups.entry(p.lint_key.as_str()).or_default();
        g.push(c.project(b));
        g.push(c.project(a));
    }
    groups
}

fn main() {
    let pairs = load_fixture();
    let n = pairs.len();

    // ── 1. embed every endpoint (timed; common to both ledger paths) ─────
    let t_embed = Instant::now();
    let embedder = ModellessEmbedder;
    let mut embeds: Vec<([f32; 8], [f32; 8])> = Vec::with_capacity(n);
    for p in &pairs {
        embeds.push((embedder.embed(&p.shape), embedder.embed(&p.fix)));
    }
    let embed_secs = t_embed.elapsed().as_secs_f64();

    // ── 2. report rows: ρ per embedder candidate over POINTS ─────────────
    println!("\nρ per embedder candidate (points grouped by lint_key rule family):");
    // The scratch is grow-only, so one per width (full8's k=8 vs the 4-wide
    // candidates) — never shared across widths.
    let mut s8 = RhoScratch::new(8);
    let mut s4 = RhoScratch::new(4);
    let mut t_rho = 0.0f64; // the ρ reads only; the embed is the common cost
    for c in &CANDIDATES {
        let groups = group_points(&pairs, &embeds, c);
        let t0 = Instant::now();
        let report = match c.k {
            8 => rho_over(c.name, &groups, c.k, &mut s8),
            _ => rho_over(c.name, &groups, c.k, &mut s4),
        };
        t_rho += t0.elapsed().as_secs_f64();
        if c.name == "full8" {
            if let Some(r) = report {
                println!("    convention: {}", r.convention);
            }
        }
    }

    // ── 3. the pre-registered replay: ρ over PAIR DIFFERENCES ────────────
    println!(
        "\npre-registered replay (threshold {REFUSE_THRESHOLD} pinned before the first run, issue 201):"
    );
    let mut diffs: Vec<([f32; 8], &str)> = Vec::with_capacity(n);
    for (p, (b, a)) in pairs.iter().zip(&embeds) {
        let mut df = [0.0f32; 8];
        for i in 0..8 {
            df[i] = a[i] - b[i];
        }
        diffs.push((df, p.lint_key.as_str()));
    }
    let struct4 = &CANDIDATES[1];
    let full8 = &CANDIDATES[0];
    // The diff read runs on the ≥2-row families subpopulation, DISCLOSED:
    // a single-row family has exactly ONE pair difference — ρ is undefined
    // on single-view groups (the primitive refuses loud), so those families
    // are excluded BY NAME here, never silently. The points read above keeps
    // all 37 (every family has ≥2 texts).
    let mut row_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, key) in &diffs {
        *row_counts.entry(key).or_default() += 1;
    }
    let excluded: Vec<(&str, usize)> = row_counts
        .iter()
        .filter(|(_, &rows)| rows < 2)
        .map(|(&k, &rows)| (k, rows))
        .collect();
    let dg_struct: BTreeMap<&str, Vec<Vec<f32>>> = diffs
        .iter()
        .filter(|(_, key)| row_counts[*key] >= 2)
        .fold(BTreeMap::new(), |mut acc, (df, key)| {
            acc.entry(*key).or_default().push(struct4.project(df));
            acc
        });
    let dg_full: BTreeMap<&str, Vec<Vec<f32>>> = diffs
        .iter()
        .filter(|(_, key)| row_counts[*key] >= 2)
        .fold(BTreeMap::new(), |mut acc, (df, key)| {
            acc.entry(*key).or_default().push(full8.project(df));
            acc
        });
    println!(
        "  diff-level population: {} of {} families (≥2 rows); excluded: {}",
        dg_struct.len(),
        row_counts.len(),
        match excluded.is_empty() {
            true => "none".to_owned(),
            false => excluded
                .iter()
                .map(|(k, r)| format!("{k}×{r}"))
                .collect::<Vec<_>>()
                .join(", "),
        }
    );
    let t0 = Instant::now();
    let rho_struct = match struct4.k {
        8 => rho_over("diff/struct4", &dg_struct, struct4.k, &mut s8),
        _ => rho_over("diff/struct4", &dg_struct, struct4.k, &mut s4),
    };
    let rho_full = match full8.k {
        8 => rho_over("diff/full8", &dg_full, full8.k, &mut s8),
        _ => rho_over("diff/full8", &dg_full, full8.k, &mut s4),
    };
    t_rho += t0.elapsed().as_secs_f64();
    let _ = rho_full; // report row only

    // The predictor (pinned): refuse iff the formatting-bearing dims carry
    // minority rule-explained variance in the difference space.
    let predicted = match rho_struct.as_ref().map(|r| r.rho) {
        Some(rho) if rho < REFUSE_THRESHOLD => "NOT-ARMED",
        Some(_) => "ABSTAIN (rule-structured — the gate cannot certify; run the full certificate)",
        None => "NO PREDICTION (ρ read refused loud)",
    };
    println!("  predictor: PREDICT {predicted}");
    println!("  bench 115 recorded: NOT-ARMED (no transferable low-rank formatting subspace)");
    match predicted {
        "NOT-ARMED" => println!("  replay verdict: MATCH — the ρ gate reproduces the refusal"),
        _ => println!("  replay verdict: no refusal reproduced (see the pinned arm's fail branch)"),
    }

    // ── 4. the certificate core the do-not-arm verdict skips (timed) ─────
    let t_cert = Instant::now();
    certificate_core(&pairs, &embeds);
    let cert_secs = t_cert.elapsed().as_secs_f64();

    // ── 5. the ledger ────────────────────────────────────────────────────
    println!("\ncost ledger (post-embed on both sides; embed={embed_secs:.3}s common):");
    println!("  ρ reads (3 point rows + 2 diff rows): {t_rho:.3}s");
    println!("  certificate core (Σ+whitener+moment+split+holdout): {cert_secs:.3}s");
    let ratio = cert_secs / t_rho.max(1e-9);
    println!(
        "  ratio (cert/ρ): {ratio:.1}× — {}",
        match cert_secs > t_rho {
            true => "the refusal saves measurable compute",
            false => "the ρ read costs as much as the refused compute — the savings arm FAILS",
        }
    );
    println!("\nREPORT-ONLY: nothing armed here; results land under issue 201 + katgpt-rs 978.");
}

/// The refused-compute stand-in: formatting_flip's N4 certificate sequence,
/// ONE rule-split direction over the committed fixture and the SAME embeds.
/// This is the compute a do-not-arm ρ verdict skips.
fn certificate_core(pairs: &[Pair], embeds: &[([f32; 8], [f32; 8])]) {
    let d = 8usize;
    // Whitening population Σ over ALL endpoints (formatting_flip posture).
    let mut pts: Vec<[f64; 8]> = Vec::with_capacity(embeds.len() * 2);
    for (b, a) in embeds {
        let mut pb = [0.0f64; 8];
        let mut pa = [0.0f64; 8];
        for i in 0..8 {
            pb[i] = f64::from(b[i]);
            pa[i] = f64::from(a[i]);
        }
        pts.push(pb);
        pts.push(pa);
    }
    let mut mean = [0.0f64; 8];
    for p in &pts {
        for i in 0..8 {
            mean[i] += p[i];
        }
    }
    for m in &mut mean {
        *m /= pts.len() as f64;
    }
    let mut sigma = vec![0.0f64; d * d];
    for p in &pts {
        for i in 0..d {
            for j in 0..d {
                sigma[i * d + j] += (p[i] - mean[i]) * (p[j] - mean[j]);
            }
        }
    }
    for v in &mut sigma {
        *v /= pts.len() as f64;
    }
    let tr_sigma: f64 = (0..d).map(|i| sigma[i * d + i]).sum();
    let sigma_floor = SHRINKAGE_REL * tr_sigma / d as f64;
    for i in 0..d {
        sigma[i * d + i] += sigma_floor;
    }

    // blake3 rule parity assigns fit/holdout rules (formatting_flip's
    // rule-split direction; deterministic — hash the family name, one bit).
    let mut fit = std::collections::BTreeSet::new();
    let mut hold = std::collections::BTreeSet::new();
    for p in pairs {
        let mut h = blake3::Hasher::new();
        h.update(p.lint_key.as_bytes());
        let bit = h.finalize().as_bytes()[0] & 1;
        if bit == 0 {
            fit.insert(p.lint_key.as_str());
        } else {
            hold.insert(p.lint_key.as_str());
        }
    }
    let mut deltas_fit: Vec<[f64; 8]> = Vec::new();
    let mut holdout: Vec<Vec<f64>> = Vec::new();
    for (p, (b, a)) in pairs.iter().zip(embeds) {
        let mut dv = [0.0f64; 8];
        for i in 0..8 {
            dv[i] = f64::from(a[i]) - f64::from(b[i]);
        }
        if fit.contains(p.lint_key.as_str()) {
            deltas_fit.push(dv);
        } else {
            holdout.push(dv.to_vec());
        }
    }
    if deltas_fit.len() < MIN_FIT_PAIRS || holdout.is_empty() {
        println!("  certificate core: refused loud (insufficient split) — a RESULT, never a 0");
        return;
    }

    let mut whitener = vec![0.0f64; d * d];
    let mut w_eigvals = vec![0.0f64; d];
    let mut w_eigvecs = vec![0.0f64; d * d];
    let mut sym0 = SymmetricEigScratch::new();
    let build = build_zca_whitener(
        &sigma,
        d,
        &mut whitener,
        &mut w_eigvals,
        &mut w_eigvecs,
        &mut sym0,
    );
    if build.is_err() {
        println!("  certificate core: whitener REFUSED — a RESULT");
        return;
    }
    let mut m = vec![0.0f64; d * d];
    {
        let mut acc = PairDifferenceMoment::new(&mut m, d).expect("moment");
        let mut w_scratch = vec![0.0f64; d];
        for (i, dv) in deltas_fit.iter().enumerate() {
            if i % 2 == 0 {
                acc.accumulate_whitened(&whitener, dv, &mut w_scratch)
                    .expect("finite deltas");
            }
        }
        acc.finish();
    }
    let tr: f64 = (0..d).map(|i| m[i * d + i]).sum();
    let floor = SHRINKAGE_REL * tr / d as f64;
    for i in 0..d {
        m[i * d + i] += floor;
    }
    let mut eigvals = vec![0.0f64; d];
    let mut eigvecs = vec![0.0f64; d * d];
    let mut basis = vec![0.0f64; d * d];
    let mut sym = SymmetricEigScratch::new();
    let mut jacobi = vec![0.0f64; d * d];
    let mut m_scr = vec![0.0f64; d * d];
    let mut whitener_out = vec![0.0f64; d * d];
    let split_res = split(
        &m,
        d,
        Some(Whitening::Whitener(&whitener)),
        None,
        TAU_GAP,
        &mut eigvals,
        &mut eigvecs,
        &mut basis,
        &mut sym,
        &mut jacobi,
        &mut m_scr,
        &mut whitener_out,
    );
    let (verdict, mut report) = match split_res {
        Ok(v) => v,
        Err(e) => {
            println!("  certificate core: split REFUSED: {e} — a RESULT");
            return;
        }
    };
    let hd: Vec<&[f64]> = holdout.iter().map(|v| &v[..]).collect();
    let mut proj = vec![0.0f64; d];
    let mut bx = vec![0.0f64; d];
    let _ = held_out_leakage(&basis, report.k, d, &hd, &mut proj, &mut bx, &mut report);
    let leakage = match report.pair_leakage {
        Some(l) => format!("{l:.4}"),
        None => "n/a".to_owned(),
    };
    println!(
        "  certificate core: fit_rules={} fit_n={} holdout_n={} → {verdict:?} k={} holdout_leakage={leakage}",
        fit.len(),
        deltas_fit.len(),
        holdout.len(),
        report.k,
    );
    println!(
        "    (cost stand-in only: population = all {} pairs, split = blake3 rule parity — NOT \
         Bench 115's postures; the comparable signal is the held-out leakage, theirs ~2%)",
        embeds.len()
    );
}
