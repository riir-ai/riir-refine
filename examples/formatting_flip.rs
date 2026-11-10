//! Issue 162 T2 — the formatting-churn instrument for the healer's dense
//! retrieval channel (REPORT-ONLY; T3 decides from these numbers, nothing
//! arms here).
//!
//! # The three questions one run answers
//!
//! 1. **Pair flip (the headline)** — over Certified fix pairs mined from the
//!    trajectory stores (`cargo refine --dump-trajectories`), how often does
//!    a channel's top-1 change between the before-span and the after-span of
//!    a compile-verified mechanical fix? Channels: the SHIPPED fused matcher
//!    (own-domain ranked list), the raw Full-8 latent (cosine over the rule
//!    corpus), and the Structural-4 latent (hash dims zeroed — the
//!    `LatentChoice::Structural4` zero-locality precedent).
//! 2. **Whitespace floor** — the committed labeled eval fixtures under three
//!    token-identical mutators (indent, trailing whitespace, blank-line
//!    insertion). Content identical by construction: every flip here is
//!    pure churn. This is the floor any formatting-invariant channel must
//!    survive; the pair-flip rate minus this floor is the churn the FIX
//!    itself (token changes) contributes.
//! 3. **Formatting basis (SplitJEPA, katgpt-core `basis_split`)** — the fit
//!    family's whitened pair-difference moment under the refuse-loud
//!    identifiability verdict; `pair_leakage` evaluated on the OTHER
//!    families (the cross-generator holdout — the issue's holdout law),
//!    with the same-family parity split disclosed separately as the
//!    calibration bound it is (never purity, never folded into the holdout
//!    read). The shrinkage floor follows reflex's `basis_cert` contract.
//!
//! # Inputs + hygiene
//!
//! One or more `--dump-trajectories` JSONL files. Only
//! `EvidenceTier::Certified` rows feed the instrument (the
//! verified-mechanical class; Heuristic rows are unverified history,
//! Withdrawn absorbing failures). The slice is frozen by a BLAKE3 digest
//! over the sorted pair texts; the bench record carries NUMBERS and the
//! digest only — the raw pair texts stay machine-local, never committed.
//!
//! ```text
//! cargo run --release --features latent_retrieval,basis_cert \
//!   --example formatting_flip -- --pairs /tmp/a_trajs.jsonl /tmp/b_trajs.jsonl
//! ```
//!
//! Deterministic end to end: hash embedder, fixed mutators, sorted
//! iteration everywhere. Re-running on the same store state prints the
//! same report.

use std::collections::BTreeMap;

use riir_rag::{Embedder, ModellessEmbedder};
use riir_refine::draft::DomainId;
use riir_refine::draft::latent_matcher::LatentMatcher;
use riir_refine::draft::retrieval_eval::{canonical_rank, labeled_eval_spans};
use riir_refine::memory::EvidenceTier;
use riir_refine::rule_domains::RuleDomains;

use katgpt_core::basis_split::{
    PairDifferenceMoment, Verdict, Whitening, build_zca_whitener, held_out_leakage, split,
};
use katgpt_core::linalg::symmetric_eig::SymmetricEigScratch;

/// `τ_gap` — the 4× eigengap posture (the planted-recovery tests' and
/// reflex `basis_cert`'s shared contract).
const TAU_GAP: f64 = 4.0;

/// The disclosed shrinkage floor: `δ = SHRINKAGE_REL · tr(M)/d` added to the
/// moment diagonal before the eigensolve (katgpt-core's QL phase panics on an
/// exactly-singular moment; the floor makes the rank gate read
/// "dimension of above-floor variation"). Reflex's measured 1e-8.
const SHRINKAGE_REL: f64 = 1e-8;

/// Structural floor on a fit subset: the moment of `n` pairs has rank ≤ n,
/// so below `d` the eigengap gate is tautological (reflex `MIN_FIT_PAIRS`
/// law, at this lane's own d).
const MIN_FIT_PAIRS: usize = 8;

/// A holdout family below this size prints flagged as THIN (a residual mean
/// over a handful of pairs is disclosed, never silent).
const THIN_HOLDOUT: usize = 32;

const TOP_K: usize = 8;

/// Which 8-D sub-space the raw channel reads. Mirrors
/// `set_rerank::LatentChoice`'s two arms locally (that module sits behind
/// the refuted-PoC `set_rerank` feature — the wrong dep for this
/// instrument; the projection itself is the two lines below and the
/// zero-locality measured fact it encodes is cited, not re-measured).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatentChoice {
    /// The native `ModellessEmbedder` contract (hash dims included).
    Full8,
    /// Hash dims zeroed — only the structural DFT dims `[4..8]` carry
    /// signal (the `domain_router::structural4` precedent: the BLAKE3 dims
    /// have zero locality).
    Structural4,
}

impl LatentChoice {
    fn project(self, v: [f32; 8]) -> [f32; 8] {
        match self {
            Self::Full8 => v,
            Self::Structural4 => [0.0, 0.0, 0.0, 0.0, v[4], v[5], v[6], v[7]],
        }
    }
}

/// One trajectory-store row (the `--dump-trajectories` JSONL shape; unknown
/// fields tolerated — the dump carries more columns than this reads). The
/// same shape as the frozen T1 artifact `assets/format_pairs_v1.jsonl`.
#[derive(serde::Deserialize)]
struct PairRow {
    #[serde(default)]
    lint_key: String,
    #[serde(default)]
    shape: String,
    #[serde(default)]
    fix: String,
    #[serde(default)]
    tier: u8,
    #[serde(default)]
    success_count: u32,
    #[serde(default)]
    fail_count: u32,
}

/// The pair-acceptance menu (the frozen T1 artifact's in-band filter law).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TierFilter {
    /// `EvidenceTier::Certified` only — the tier-guard-certified class.
    Certified,
    /// The verified-lived class: `success_count > 0` (any tier).
    Success,
    /// Every row with both halves (the seeded 0/0 mass included).
    All,
}

impl TierFilter {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "certified" => Some(Self::Certified),
            "success" => Some(Self::Success),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    fn keeps(self, row: &PairRow) -> bool {
        match self {
            Self::Certified => EvidenceTier::from_u8(row.tier) == EvidenceTier::Certified,
            Self::Success => row.success_count > 0,
            Self::All => true,
        }
    }
}

/// A mined, filtered pair with its attributed family.
struct FixPair {
    lint_key: String,
    before: String,
    after: String,
    family: String,
}

/// The three retrieval channels the report reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    /// The shipped fused matcher, own-domain ranked list.
    Fused,
    /// Raw 8-D latent, cosine over the rule corpus, corpus-wide.
    Full8,
    /// Structural-4 latent (hash dims zeroed), corpus-wide.
    Structural4,
}

impl Channel {
    const ALL: [Channel; 3] = [Channel::Fused, Channel::Full8, Channel::Structural4];

    fn name(self) -> &'static str {
        match self {
            Channel::Fused => "fused",
            Channel::Full8 => "full8",
            Channel::Structural4 => "struct4",
        }
    }
}

/// Per-family, per-channel flip accumulator.
#[derive(Default)]
struct FlipTally {
    n: usize,
    flips: [usize; 3],
    /// C1 only: pairs whose own rule LEFT the own-domain top-k after the fix.
    own_rule_lost: usize,
    /// C1 only: Σ of the own rule's rank before / after (for the means).
    own_rank_sum: [u64; 2],
    own_rank_seen: usize,
}

impl FlipTally {
    fn flip_rate(&self, c: Channel) -> Option<f64> {
        match self.n {
            0 => None,
            n => Some(self.flips[c as usize] as f64 / n as f64),
        }
    }
}

fn family_domain(family: &str) -> Option<DomainId> {
    match family {
        "clippy" => Some(DomainId::Clippy),
        "rust_perf" => Some(DomainId::Perf),
        "kernel_opt" => Some(DomainId::Kernel),
        "sec" => Some(DomainId::Sec),
        "docker" => Some(DomainId::Docker),
        "dist" => Some(DomainId::Dist),
        _ => None,
    }
}

/// Token-identical whitespace mutators. All three operate ONLY by inserting
/// whitespace at line boundaries — they cannot corrupt string-literal
/// contents, so the token stream is preserved by construction.
fn mutate_indent(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4 * s.lines().count());
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if !line.trim().is_empty() {
            out.push_str("    ");
        }
        out.push_str(line);
    }
    out
}

fn mutate_trail(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 3 * s.lines().count());
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
        if !line.trim().is_empty() {
            out.push_str("   ");
        }
    }
    out
}

fn mutate_gap(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
        let t = line.trim_end();
        if t.ends_with('{') || t.ends_with(';') {
            out.push('\n');
        }
    }
    out
}

/// The token-identical mutators (name, transform).
type Mutator = (&'static str, fn(&str) -> String);

const MUTATORS: [Mutator; 3] = [
    ("indent", mutate_indent),
    ("trail", mutate_trail),
    ("gap", mutate_gap),
];

/// Load dump files → filtered fix pairs, sorted (the frozen slice order).
fn load_pairs(
    paths: &[String],
    rd: &RuleDomains,
    filter: TierFilter,
) -> Result<(Vec<FixPair>, String), String> {
    let mut rows: Vec<FixPair> = Vec::new();
    let mut per_file: Vec<(String, usize, usize, u64)> = Vec::new();
    for p in paths {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        let mut total = 0usize;
        let mut kept = 0usize;
        let mut failed = 0u64;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let row: PairRow =
                serde_json::from_str(line).map_err(|e| format!("{p}: bad jsonl row: {e}"))?;
            total += 1;
            failed += u64::from(row.fail_count);
            if !filter.keeps(&row) {
                continue;
            }
            if row.shape.is_empty() || row.fix.is_empty() || row.shape == row.fix {
                continue;
            }
            let family = {
                let code = row.lint_key.split(':').next().unwrap_or("");
                let d = match rd.domain_for(&row.lint_key).is_empty() {
                    // The perf/kernel corpora spell their rule ids kebab-case;
                    // the stores carry the proposal-side snake spelling. Same
                    // authority, both spellings — an exact corpus hit either way.
                    true => rd.domain_for(&code.replace('_', "-")),
                    false => rd.domain_for(&row.lint_key),
                };
                match d.is_empty() {
                    true => "unattributed".to_string(),
                    false => d.to_string(),
                }
            };
            rows.push(FixPair {
                lint_key: row.lint_key,
                before: row.shape,
                after: row.fix,
                family,
            });
            kept += 1;
        }
        per_file.push((p.clone(), total, kept, failed));
    }
    rows.sort_by(|a, b| {
        (&a.lint_key, &a.before, &a.after).cmp(&(&b.lint_key, &b.before, &b.after))
    });
    for (p, total, kept, failed) in &per_file {
        println!(
            "  pairs: {p}: {total} rows, {kept} certified+usable, {failed} failed-trajectory mass"
        );
    }
    // Freeze: BLAKE3 over the sorted slice.
    let mut h = blake3::Hasher::new();
    for r in &rows {
        h.update(r.lint_key.as_bytes());
        h.update(b"\0");
        h.update(r.before.as_bytes());
        h.update(b"\0");
        h.update(r.after.as_bytes());
        h.update(b"\0");
    }
    let digest = h.finalize().to_hex().to_string();
    Ok((rows, digest))
}

/// The corpus-wide raw-latent top-1 rule for one query.
fn raw_top1(corpus: &RawCorpus, choice: LatentChoice, q: [f32; 8]) -> Option<(String, f32)> {
    let qv = choice.project(q);
    let qn = f32::sqrt(qv.iter().map(|v| v * v).sum());
    if qn <= 1e-9 {
        return None;
    }
    let mut best: Option<(String, f32)> = None;
    for (i, lv) in corpus.latents.iter().enumerate() {
        let lv = choice.project(*lv);
        let ln = f32::sqrt(lv.iter().map(|v| v * v).sum());
        if ln <= 1e-9 {
            continue;
        }
        let cos = qv.iter().zip(lv.iter()).map(|(a, b)| a * b).sum::<f32>() / (qn * ln);
        match &best {
            Some((_, bc)) if *bc >= cos => {}
            _ => best = Some((corpus.ids[i].clone(), cos)),
        }
    }
    best
}

/// The deterministic raw-latent rule corpus (sorted by rule id), built from
/// the matcher's rule book — the same pattern texts the index embeds, at the
/// embedder's own 8-D width. Independent of the `set_rerank` feature.
struct RawCorpus {
    ids: Vec<String>,
    latents: Vec<[f32; 8]>,
}

fn build_raw_corpus(
    book: &std::collections::HashMap<String, riir_refine::draft::latent_matcher::RuleInfo>,
) -> RawCorpus {
    let mut entries: Vec<(&String, &riir_refine::draft::latent_matcher::RuleInfo)> =
        book.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut ids = Vec::with_capacity(entries.len());
    let mut latents = Vec::with_capacity(entries.len());
    for (id, info) in entries {
        ids.push(id.clone());
        latents.push(embed8(&info.buggy_code));
    }
    RawCorpus { ids, latents }
}

struct ChannelCtx<'a> {
    matcher: &'a mut LatentMatcher,
    corpus: RawCorpus,
}

impl ChannelCtx<'_> {
    /// The channel's top-1 for a span. `own_domain` scopes the fused channel
    /// to the family's own-domain list (the shipped suggest posture); the
    /// raw channels are corpus-wide by construction.
    fn top1(
        &mut self,
        span: &str,
        own_domain: Option<DomainId>,
        channel: Channel,
    ) -> Option<String> {
        match channel {
            Channel::Fused => {
                let domain = own_domain?;
                match self.matcher.has_domain(domain) {
                    false => None,
                    true => {
                        let (_routed, by_domain) = self.matcher.suggest_span(span, TOP_K);
                        let list = by_domain
                            .iter()
                            .find(|(d, _)| *d == domain)
                            .map(|(_, rules)| canonical_rank(rules))
                            .unwrap_or_default();
                        list.first().map(|r| r.rule_id.clone())
                    }
                }
            }
            Channel::Full8 => {
                raw_top1(&self.corpus, LatentChoice::Full8, embed8(span)).map(|(id, _)| id)
            }
            Channel::Structural4 => {
                raw_top1(&self.corpus, LatentChoice::Structural4, embed8(span)).map(|(id, _)| id)
            }
        }
    }

    /// The own rule's rank (0-based) in the fused own-domain list.
    fn fused_own_rank(&mut self, span: &str, domain: DomainId, rule: &str) -> Option<usize> {
        match self.matcher.has_domain(domain) {
            false => None,
            true => {
                let (_routed, by_domain) = self.matcher.suggest_span(span, TOP_K);
                let list = by_domain
                    .iter()
                    .find(|(d, _)| *d == domain)
                    .map(|(_, rules)| canonical_rank(rules))
                    .unwrap_or_default();
                list.iter().position(|r| r.rule_id == rule)
            }
        }
    }

    fn has_domain(&self, domain: DomainId) -> bool {
        self.matcher.has_domain(domain)
    }
}

fn embed8(text: &str) -> [f32; 8] {
    ModellessEmbedder.embed(text)
}

/// One N4 run: a fit index subset + its holdouts. The calibration half is
/// the odd positions of `fit` (a residual fitted on its own fit set reads 0
/// by construction and certifies nothing); holdouts are named index subsets.
struct BasisRun {
    label: String,
    fit: Vec<usize>,
    holdouts: Vec<(String, Vec<usize>)>,
}

/// N4 — one direction of the formatting-basis fit + holdout. Returns the
/// printed block; every refusal is a RESULT (reflex's naming law).
#[allow(clippy::too_many_lines)]
fn basis_direction(
    run: &BasisRun,
    pairs_embed: &[([f32; 8], [f32; 8])],
    rule_of: &[String],
) -> String {
    let d = 8usize;
    let mut out = String::new();
    if run.fit.len() < MIN_FIT_PAIRS {
        out.push_str(&format!(
            "  {}: fit n={} < MIN_FIT_PAIRS({MIN_FIT_PAIRS}) — refused (insufficient_pairs)\n",
            run.label,
            run.fit.len()
        ));
        return out;
    }

    // Whitening population Σ: ALL certified endpoints (the spans the
    // pipeline consumes), mean-centered, population form.
    let mut pts: Vec<[f64; 8]> = Vec::with_capacity(pairs_embed.len() * 2);
    for (b, a) in pairs_embed {
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
    // The same disclosed relative shrinkage, on Σ itself: a structured
    // small-vocabulary span population carries near-zero eigenvalues, and
    // Σ^{-1/2} explodes along them (measured: a k=1 / gap-5e10 artifact
    // reading geometry that was never there). δ = SHRINKAGE_REL·tr(Σ)/d·I
    // keeps Σ positive-definite and the whitener bounded; the floor is
    // printed with the split so the read is reproducible.
    let tr_sigma: f64 = (0..d).map(|i| sigma[i * d + i]).sum();
    let sigma_floor = SHRINKAGE_REL * tr_sigma / d as f64;
    for i in 0..d {
        sigma[i * d + i] += sigma_floor;
    }

    // Fit: the reflex `basis_cert` posture — build the ZCA whitener EXPLICITLY
    // (so the Σ spectrum is printed and the conditioning is in the record),
    // accumulate WHITENED differences (the two whitening arms are exclusive —
    // measured the hard way there), the disclosed shrinkage floors, split at
    // the largest gap. The fit subset's even half teaches; the odd half is
    // the same-fit calibration read.
    let mut deltas: Vec<[f64; 8]> = Vec::with_capacity(run.fit.len());
    for &idx in &run.fit {
        let (b, a) = &pairs_embed[idx];
        let mut dv = [0.0f64; 8];
        for i in 0..8 {
            dv[i] = f64::from(a[i]) - f64::from(b[i]);
        }
        deltas.push(dv);
    }
    let mut whitener = vec![0.0f64; d * d];
    let mut w_eigvals = vec![0.0f64; d];
    let mut w_eigvecs = vec![0.0f64; d * d];
    let mut sym0 = SymmetricEigScratch::new();
    if let Err(e) = build_zca_whitener(
        &sigma,
        d,
        &mut whitener,
        &mut w_eigvals,
        &mut w_eigvecs,
        &mut sym0,
    ) {
        out.push_str(&format!("  {}: whitener REFUSED: {e}\n", run.label));
        return out;
    }
    let mut spec = w_eigvals.clone();
    spec.sort_by(|a, b| b.total_cmp(a));
    out.push_str(&format!(
        "  {}: Σ spectrum (desc) [{}] cond={:.3e}\n",
        run.label,
        spec.iter()
            .map(|v| format!("{v:.2e}"))
            .collect::<Vec<_>>()
            .join(", "),
        spec[0] / spec[d - 1]
    ));
    let mut m = vec![0.0f64; d * d];
    {
        let mut acc = PairDifferenceMoment::new(&mut m, d).expect("moment");
        let mut w_scratch = vec![0.0f64; d];
        for (i, dv) in deltas.iter().enumerate() {
            if let 0 = i % 2 {
                acc.accumulate_whitened(&whitener, dv, &mut w_scratch)
                    .expect("finite deltas")
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
    // The Whitener arm pins W immutably; split's whitener_out param needs its
    // own buffer (it is only written on the TotalCovariance arm).
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
            out.push_str(&format!("  {}: split REFUSED: {e}\n", run.label));
            return out;
        }
    };
    out.push_str(&format!(
        "  {}: fit n={} → {verdict:?} k={} gap={:.3} pr={:.2} top1_share={:.3} sigma_floor={:.3e}\n",
        run.label,
        run.fit.len(),
        report.k,
        report.gap_ratio,
        report.participation_ratio,
        {
            // λ₁ / Σλ — when a handful of outlier pairs (huge Δ norm) dominate
            // the moment, this reads ~1.0 while typical pairs still project
            // almost nothing (the leakage lines show it); the pair is the
            // outlier mass, not a shared formatting structure.
            let total: f64 = eigvals[..d].iter().sum();
            match total > 0.0 {
                true => eigvals[..d].iter().cloned().fold(f64::MIN, f64::max) / total,
                false => f64::NAN,
            }
        },
        sigma_floor
    ));
    match &verdict {
        Verdict::SufficientVariation { .. } => {}
        _ => {
            out.push_str("    (unidentified — the certificate refusing is a RESULT: no rank-gated formatting subspace is certified in this embed)\n");
        }
    }

    // Same-family calibration residual (the odd half — the bound, never the
    // holdout read).
    let mut odd: Vec<&[f64]> = Vec::new();
    for (i, dv) in deltas.iter().enumerate() {
        if let 1 = i % 2 {
            odd.push(dv)
        }
    }
    let mut proj = vec![0.0f64; d];
    let mut bx = vec![0.0f64; d];
    let _ = held_out_leakage(&basis, report.k, d, &odd, &mut proj, &mut bx, &mut report);
    out.push_str(&format!(
        "    same-fit(calibration, n={}): leakage={:?} residual={:?}\n",
        odd.len(),
        report.pair_leakage,
        report.agreement_residual
    ));
    for (name, idxs) in &run.holdouts {
        if idxs.is_empty() {
            continue;
        }
        let mut hd_owned: Vec<[f64; 8]> = Vec::with_capacity(idxs.len());
        for &idx in idxs {
            let (b, a) = &pairs_embed[idx];
            let mut dv = [0.0f64; 8];
            for i in 0..8 {
                dv[i] = f64::from(a[i]) - f64::from(b[i]);
            }
            hd_owned.push(dv);
        }
        let hd: Vec<&[f64]> = hd_owned.iter().map(|v| &v[..]).collect();
        let _ = held_out_leakage(&basis, report.k, d, &hd, &mut proj, &mut bx, &mut report);
        let thin = match idxs.len() < THIN_HOLDOUT {
            true => " THIN",
            false => "",
        };
        out.push_str(&format!(
            "    holdout=`{name}` (n={}){}: leakage={:?} residual={:?}\n",
            idxs.len(),
            thin,
            report.pair_leakage,
            report.agreement_residual
        ));
    }
    // The rule-level composition of the fit side (the generators that
    // taught the basis) — disclosed so a fit dominated by one rule reads
    // as what it is.
    let mut fit_rules: BTreeMap<&str, usize> = BTreeMap::new();
    for &idx in &run.fit {
        *fit_rules.entry(rule_of[idx].as_str()).or_default() += 1;
    }
    let top: Vec<_> = fit_rules.iter().collect();
    out.push_str(&format!(
        "    fit generators: {} distinct; top: {}\n",
        fit_rules.len(),
        top.iter()
            .take(3)
            .map(|(r, n)| format!("{r}×{n}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut pairs_paths: Vec<String> = Vec::new();
    let mut skip_fixtures = false;
    let mut filter = TierFilter::Certified;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--pairs" => {
                // One --pairs flag may carry several whitespace-separated
                // paths; a lone path also works.
                let mut consumed = false;
                while let Some(p) = args.get(i + 1) {
                    if p.starts_with("--") {
                        break;
                    }
                    pairs_paths.push(p.clone());
                    i += 1;
                    consumed = true;
                }
                if !consumed {
                    eprintln!("--pairs needs at least one path");
                    std::process::exit(2);
                }
            }
            "--tier-filter" => match args.get(i + 1).and_then(|s| TierFilter::parse(s)) {
                Some(f) => {
                    filter = f;
                    i += 1;
                }
                None => {
                    eprintln!("--tier-filter needs certified|success|all");
                    std::process::exit(2);
                }
            },
            "--skip-fixtures" => skip_fixtures = true,
            other => {
                eprintln!(
                    "unknown arg {other}; usage: formatting_flip --pairs <dump.jsonl>... [--tier-filter certified|success|all] [--skip-fixtures]"
                );
                std::process::exit(2);
            }
        }
        i += 1;
    }
    if pairs_paths.is_empty() {
        eprintln!(
            "no --pairs files; run `cargo refine --dump-trajectories > /tmp/<repo>_trajs.jsonl` in each store repo first"
        );
        std::process::exit(2);
    }

    let rd = RuleDomains::compiled();
    let (pairs, digest) = match load_pairs(&pairs_paths, &rd, filter) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    println!(
        "slice digest: {} (n={}, filter={:?})",
        &digest[..16.min(digest.len())],
        pairs.len(),
        filter
    );

    let mut matcher = match LatentMatcher::from_enabled_domains() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("matcher: {e} (enable at least one retrieval domain feature)");
            std::process::exit(1);
        }
    };
    let corpus = build_raw_corpus(matcher.rule_book_map());
    let mut ctx = ChannelCtx {
        matcher: &mut matcher,
        corpus,
    };

    // ── family census ──────────────────────────────────────────────────────
    let mut by_family: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, p) in pairs.iter().enumerate() {
        by_family.entry(p.family.clone()).or_default().push(idx);
    }
    println!("\n── family census (certified pairs) ──");
    for (fam, idxs) in &by_family {
        println!("  {fam}: {}", idxs.len());
    }

    // ── N1: pair flip ──────────────────────────────────────────────────────
    println!("\n── pair flip: before-span vs after-span of Certified mechanical fixes ──");
    let mut tallies: BTreeMap<String, FlipTally> = BTreeMap::new();
    for p in &pairs {
        let own = family_domain(&p.family);
        let own_rule = p.lint_key.split(':').next().unwrap_or("").to_string();
        let e = tallies.entry(p.family.clone()).or_default();
        e.n += 1;
        for c in Channel::ALL {
            let t1b = ctx.top1(&p.before, own, c);
            let t1a = ctx.top1(&p.after, own, c);
            match (t1b, t1a) {
                (Some(x), Some(y)) => {
                    if x != y {
                        e.flips[c as usize] += 1;
                    }
                }
                // A channel that answers for one endpoint and not the other
                // IS a flip (the answer changed); it cannot be skipped.
                (None, None) => {}
                _ => e.flips[c as usize] += 1,
            }
        }
        if let Some(domain) = own {
            let rb = ctx.fused_own_rank(&p.before, domain, &own_rule);
            let ra = ctx.fused_own_rank(&p.after, domain, &own_rule);
            match (rb, ra) {
                (Some(x), Some(y)) => {
                    e.own_rank_sum[0] += x as u64;
                    e.own_rank_sum[1] += y as u64;
                    e.own_rank_seen += 1;
                }
                (Some(_), None) => e.own_rule_lost += 1,
                _ => {}
            }
        }
    }
    print!("  {:<14}", "family");
    for c in Channel::ALL {
        print!(" {:>9}", c.name());
    }
    println!(" {:>14} {:>10}", "own-lost/seen", "rank b→a");
    for (fam, t) in &tallies {
        print!("  {fam:<14}");
        for c in Channel::ALL {
            match t.flip_rate(c) {
                Some(r) => print!(" {:>8.1}%", r * 100.0),
                None => print!(" {:>9}", "-"),
            }
        }
        let rk = match t.own_rank_seen {
            0 => "-".to_string(),
            n => format!(
                "{:.2}→{:.2}",
                t.own_rank_sum[0] as f64 / n as f64,
                t.own_rank_sum[1] as f64 / n as f64
            ),
        };
        println!(
            " {:>14} {:>10}",
            format!("{}/{}", t.own_rule_lost, t.own_rank_seen),
            rk
        );
    }

    // ── N3: whitespace floor over the committed fixtures ───────────────────
    let mut floor_json = serde_json::Map::new();
    if !skip_fixtures {
        println!("\n── whitespace floor: committed eval fixtures, token-identical mutators ──");
        let spans = labeled_eval_spans();
        let mut per_mut: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
        let mut n_span = 0usize;
        for span in &spans {
            let own = Some(span.domain);
            if !ctx.has_domain(span.domain) {
                continue;
            }
            n_span += 1;
            for (name, f) in MUTATORS {
                let mutant = f(&span.code);
                let e = per_mut.entry(name).or_default();
                for (ci, c) in Channel::ALL.iter().enumerate() {
                    let t1b = ctx.top1(&span.code, own, *c);
                    let t1a = ctx.top1(&mutant, own, *c);
                    match (t1b, t1a) {
                        (Some(x), Some(y)) if x == y => {}
                        (Some(x), Some(y)) if x != y => e[ci] += 1,
                        (None, None) => {}
                        _ => e[ci] += 1,
                    }
                }
            }
        }
        print!("  {:<8}{:>6}", "mutator", "n");
        for c in Channel::ALL {
            print!(" {:>9}", c.name());
        }
        println!();
        for (name, e) in &per_mut {
            print!("  {name:<8}{:>6}", n_span);
            for (ci, _) in Channel::ALL.iter().enumerate() {
                println_flush_rate(e[ci], n_span);
            }
            println!();
        }
        let mut jm = serde_json::Map::new();
        for (name, e) in &per_mut {
            let mut jc = serde_json::Map::new();
            for (ci, c) in Channel::ALL.iter().enumerate() {
                jc.insert(c.name().to_string(), serde_json::json!(rate(e[ci], n_span)));
            }
            jm.insert(name.to_string(), serde_json::Value::Object(jc));
        }
        floor_json.insert("n_spans".into(), serde_json::json!(n_span));
        floor_json.insert("per_mutator".into(), serde_json::Value::Object(jm));
    }

    // ── N4: the formatting basis ────────────────────────────────────────
    println!("\n── formatting basis (basis_split, whitened pair differences) ──");
    let pairs_embed: Vec<([f32; 8], [f32; 8])> = pairs
        .iter()
        .map(|p| (embed8(&p.before), embed8(&p.after)))
        .collect();
    // The 32-D token channel contrast (span_embed): mean ΔL2 + mean cosine.
    println!("  token-channel contrast (span_embed 32-D, per family):");
    for (fam, idxs) in &by_family {
        let mut sum_d = 0.0f64;
        let mut sum_c = 0.0f64;
        for &i in idxs {
            let eb = riir_refine::span_embed::span_embed(&pairs[i].before);
            let ea = riir_refine::span_embed::span_embed(&pairs[i].after);
            let fb: Vec<f32> = eb.iter().map(|&v| f32::from(v) / 127.0).collect();
            let fa: Vec<f32> = ea.iter().map(|&v| f32::from(v) / 127.0).collect();
            let d2: f32 = fb
                .iter()
                .zip(fa.iter())
                .map(|(x, y)| (x - y) * (x - y))
                .sum();
            sum_d += f64::from(d2.sqrt());
            let dot: f32 = fb.iter().zip(fa.iter()).map(|(x, y)| x * y).sum();
            let nb = f32::sqrt(fb.iter().map(|x| x * x).sum());
            let na = f32::sqrt(fa.iter().map(|x| x * x).sum());
            if nb > 1e-9 && na > 1e-9 {
                sum_c += f64::from(dot / (nb * na));
            }
        }
        let n = idxs.len();
        println!(
            "    {fam}: mean ΔL2={:.4} mean cos={:.4} (n={n})",
            sum_d / n as f64,
            sum_c / n as f64
        );
    }

    // The rule id of every pair (the generator that produced the fix).
    let rule_of: Vec<String> = pairs
        .iter()
        .map(|p| p.lint_key.split(':').next().unwrap_or("").to_string())
        .collect();

    // The family order (size desc, name asc) — the fit family is the head.
    let mut order: Vec<(String, usize)> = by_family
        .iter()
        .map(|(f, v)| (f.clone(), v.len()))
        .collect();
    order.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // Run 1 — the issue's FAMILY-level holdout: fit the largest family,
    // hold out every other family (THIN tails disclosed as such). When the
    // workspace's certified population is one family, the run says so and
    // run 2 carries the cross-generator read.
    let mut basis_json = serde_json::Map::new();
    {
        let fit_fam = order[0].0.clone();
        let fit: Vec<usize> = by_family.get(&fit_fam).cloned().unwrap_or_default();
        let holdouts: Vec<(String, Vec<usize>)> = by_family
            .iter()
            .filter(|(f, _)| *f != &fit_fam)
            .map(|(f, v)| (f.clone(), v.clone()))
            .collect();
        let run = BasisRun {
            label: format!("family-split (fit={fit_fam})"),
            fit,
            holdouts,
        };
        print!("{}", basis_direction(&run, &pairs_embed, &rule_of));
        basis_json.insert("family_split".to_string(), serde_json::json!(run.fit.len()));
    }

    // Run 2 — the RULE-level cross-generator holdout (the available
    // instantiation when the population carries one family: each corpus
    // rule IS a generator). Even-hash rule ids fit, odd-hash rule ids hold
    // out; deterministic (blake3, first byte parity).
    {
        let mut even: Vec<usize> = Vec::new();
        let mut odd: Vec<usize> = Vec::new();
        for (i, r) in rule_of.iter().enumerate() {
            match blake3::hash(r.as_bytes()).as_bytes()[0] & 1 {
                0 => even.push(i),
                _ => odd.push(i),
            }
        }
        let fit_rules: std::collections::BTreeSet<&str> =
            even.iter().map(|&i| rule_of[i].as_str()).collect();
        let hold_rules: std::collections::BTreeSet<&str> =
            odd.iter().map(|&i| rule_of[i].as_str()).collect();
        println!(
            "  rule-split sides: fit {} rules (n={}) / holdout {} rules (n={}) — blake3 parity",
            fit_rules.len(),
            even.len(),
            hold_rules.len(),
            odd.len()
        );
        let run = BasisRun {
            label: "rule-split (cross-generator)".to_string(),
            fit: even,
            holdouts: vec![("odd-hash-rules".to_string(), odd)],
        };
        print!("{}", basis_direction(&run, &pairs_embed, &rule_of));
        basis_json.insert("rule_split".to_string(), serde_json::json!(run.fit.len()));
    }

    // ── machine-readable tail ──────────────────────────────────────────────
    let mut fj = serde_json::Map::new();
    for (fam, t) in &tallies {
        let mut jc = serde_json::Map::new();
        for c in Channel::ALL {
            if let Some(r) = t.flip_rate(c) {
                jc.insert(c.name().to_string(), serde_json::json!(r));
            }
        }
        fj.insert(fam.clone(), serde_json::Value::Object(jc));
    }
    let tail = serde_json::json!({
        "lane": "format_flip",
        "issue": 162,
        "slice_digest": digest,
        "tier_filter": format!("{filter:?}"),
        "certified_pairs": pairs.len(),
        "pair_flip": serde_json::Value::Object(fj),
        "whitespace_floor": serde_json::Value::Object(floor_json),
        "basis_runs": serde_json::Value::Object(basis_json),
    });
    println!("\nformat_flip_json: {tail}");
}

fn rate(hits: usize, n: usize) -> Option<f64> {
    match n {
        0 => None,
        _ => Some(hits as f64 / n as f64),
    }
}

/// Print one floor cell (kept tiny so the loop above stays readable).
fn println_flush_rate(hits: usize, n: usize) {
    match rate(hits, n) {
        Some(r) => print!(" {:>8.1}%", r * 100.0),
        None => print!(" {:>9}", "-"),
    }
}
