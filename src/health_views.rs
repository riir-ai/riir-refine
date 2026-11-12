//! The fixer/trainer lane health views (the `fixer` / `trainer` verbs).
//!
//! Two READ-ONLY local views over state the product already writes — no new
//! writes, no new network surface (the one network line reuses the points
//! aggregate read the `info` verb already makes). The verbs answer "is my
//! fixer/trainer working, and what has it done?" without opening `--info`
//! (the `kat_service` posture) or digging through `.refine/`.
//!
//! Sources, one line each:
//! - fixer: the blueprint campaign history (`blueprint_history.jsonl`, the
//!   always-on append lane — posture + findings + outcomes per `--fix` run)
//! - trainer: the self-evolve store (`.refine/trajectories.kv`, the memory
//!   half) + the corpus-lease cache (`.refine/corpus-lease`, the trained
//!   corpus epoch) + the learning posture (the build's oracle feature)
//! - both: build stamp, compiled corpus counts (the pins manifest — the ONE
//!   source of truth, never hand-typed), the settings/offline posture.
//!
//! Gating: every build renders the local halves (they read files the default
//! build writes); the network aggregate is `points`-gated (the release set
//! carries it; a slim build prints the pointer instead). Offline (`cargo
//! refine offline`) degrades the network line loudly, never blocks the
//! local halves.

use std::path::Path;

/// The compiled corpus counts, from the committed pins manifest (the ONE
/// source of truth — the same file `scripts/pins_manifest.py` derives).
/// Domains absent from the manifest are compiled-out at this manifest's
/// generation and print as `-`.
#[must_use]
pub fn corpus_counts_line() -> String {
    #[derive(serde::Deserialize)]
    struct Manifest {
        #[serde(default)]
        clippy_lints: Option<DomainCount>,
        #[serde(default)]
        rust_perf: Option<PerfCount>,
        #[serde(default)]
        kernel_opt: Option<DomainCount>,
        #[serde(default)]
        sec: Option<DomainCount>,
        #[serde(default)]
        docker_rules: Option<u64>,
        #[serde(default)]
        dist_rules: Option<u64>,
        #[serde(default)]
        shader_perf_rules: Option<u64>,
    }
    #[derive(serde::Deserialize)]
    struct DomainCount {
        rules: u64,
    }
    #[derive(serde::Deserialize)]
    struct PerfCount {
        rules: u64,
        #[serde(default)]
        benchable: Option<u64>,
    }
    let Some(text) = pins_manifest_text() else {
        return "corpus: (manifest not embedded in this build)".to_string();
    };
    let Ok(m) = serde_json::from_str::<Manifest>(text) else {
        return "corpus: (manifest unreadable — report a bug)".to_string();
    };
    let fmt = |v: Option<u64>| v.map_or_else(|| "-".to_string(), |n| n.to_string());
    format!(
        "corpus: clippy {} · perf {} ({} benchable) · kernel {} · sec {} · docker {} · dist {} · shader {}",
        fmt(m.clippy_lints.map(|d| d.rules)),
        fmt(m.rust_perf.as_ref().map(|d| d.rules)),
        fmt(m.rust_perf.as_ref().and_then(|d| d.benchable)),
        fmt(m.kernel_opt.map(|d| d.rules)),
        fmt(m.sec.map(|d| d.rules)),
        fmt(m.docker_rules),
        fmt(m.dist_rules),
        fmt(m.shader_perf_rules),
    )
}

/// The manifest rides the binary when the embed feature set includes it;
/// the include is unconditional in source — a slim build still carries the
/// bytes (it is a few KiB and the corpus line is then still honest).
#[must_use]
fn pins_manifest_text() -> Option<&'static str> {
    const MANIFEST: &str = include_str!("../.benchmarks/pins_manifest.json");
    if MANIFEST.trim().is_empty() {
        None
    } else {
        Some(MANIFEST)
    }
}

/// One fix campaign's roll-up for the history lines.
pub struct CampaignSummary {
    pub ts_unix: u64,
    pub findings_total: usize,
    pub kept_edits: usize,
    pub reverted: usize,
    pub verify_ran: bool,
    /// Citations in the compiled clippy fix set (the fix-rate denominator).
    pub fixable_clippy: usize,
}

/// Load the blueprint campaign history (newest last) and summarize the
/// fleet: total campaigns, lifetime outcomes, and the last campaign's row.
/// Missing file = an empty history (an honest zero, never an error).
/// Returns the lifetime sums INCLUDING the per-campaign fixable-citation
/// denominator (the compiled clippy fix set — the honest fix-rate axis;
/// suggest-only domains are citations, not fixable failures).
#[must_use]
pub fn fixer_history(root: &Path) -> (usize, usize, usize, usize, Option<CampaignSummary>) {
    let path = root.join(".refine").join("blueprint_history.jsonl");
    let (rows, bad) = crate::blueprint::load_rows(&path);
    let campaigns = rows.len() + bad;
    let mut kept = 0usize;
    let mut reverted = 0usize;
    let mut findings = 0usize;
    let mut last = None;
    for r in &rows {
        kept += r.outcomes.kept_edits;
        reverted += r.outcomes.reverted;
        findings += r.findings.total;
        last = Some(r);
    }
    let summary = last.map(|r| CampaignSummary {
        ts_unix: r.ts,
        findings_total: r.findings.total,
        kept_edits: r.outcomes.kept_edits,
        reverted: r.outcomes.reverted,
        verify_ran: r.outcomes.verify_ran,
        fixable_clippy: r.findings.fixable_clippy,
    });
    (campaigns, findings, kept, reverted, summary)
}

/// The lifetime fix-rate arithmetic over the history file: kept edits vs
/// the compiled clippy fix-set citations (the run's own "~N% fixable"
/// denominator). `None` when the denominator is zero (no campaigns or an
/// all-suggest-only history) — never a fake 0%.
#[must_use]
pub fn fixer_rate(root: &Path) -> Option<(usize, usize, u64, u64)> {
    let path = root.join(".refine").join("blueprint_history.jsonl");
    let (rows, _bad) = crate::blueprint::load_rows(&path);
    let kept: usize = rows.iter().map(|r| r.outcomes.kept_edits).sum();
    let fixable: usize = rows.iter().map(|r| r.findings.fixable_clippy).sum();
    if fixable == 0 {
        return None;
    }
    let rate = (kept as u64) * 100 / fixable as u64;
    let unfixed = 100u64.saturating_sub(rate);
    Some((kept, fixable, rate, unfixed))
}

/// The trainer's memory-half facts, read WITHOUT opening the encrypted
/// store (an opened WAL can contend with a concurrent refine run; the
/// sizes answer the health question). Missing store = the honest zero.
pub struct TrainerMemory {
    pub store_exists: bool,
    pub wal_bytes: u64,
    pub segment_count: usize,
    pub oldest_seg: Option<String>,
    pub newest_seg: Option<String>,
}

#[must_use]
pub fn trainer_memory(root: &Path) -> TrainerMemory {
    let dir = root.join(".refine").join("trajectories.kv");
    if !dir.is_dir() {
        return TrainerMemory {
            store_exists: false,
            wal_bytes: 0,
            segment_count: 0,
            oldest_seg: None,
            newest_seg: None,
        };
    }
    let mut wal_bytes = 0u64;
    let mut segs: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            match e.file_type() {
                Ok(t) if t.is_dir() => {
                    // seg/ + wal/ subdirs: count segments inside `seg`.
                    if name == "seg" {
                        if let Ok(srd) = std::fs::read_dir(e.path()) {
                            for s in srd.flatten() {
                                segs.push(s.file_name().to_string_lossy().to_string());
                            }
                        }
                    } else if name == "wal.log" {
                        wal_bytes = e.metadata().map_or(0, |m| m.len());
                    }
                }
                Ok(t) if t.is_file() && name == "wal.log" => {
                    wal_bytes = e.metadata().map_or(0, |m| m.len());
                }
                _ => {}
            }
        }
    }
    segs.sort();
    let segment_count = segs.len();
    let oldest_seg = segs.first().cloned();
    let newest_seg = segs.last().cloned();
    TrainerMemory {
        store_exists: true,
        wal_bytes,
        segment_count,
        oldest_seg,
        newest_seg,
    }
}

/// The corpus-lease store this machine has cached (the trainer's OUTPUT —
/// the trained rules the network ships each epoch). Offline-safe, read-only.
/// Gated to the feature sets that carry the lease client; the slim posture
/// prints the honest zero.
/// The trainer's OUTPUT — the leased corpus this machine has installed.
/// `epochs_installed` counts the manifests on disk (how many training
/// upgrades this machine took); `shape_line` is the current epoch's rule
/// census (the same line `update` prints). Both read-only, offline-safe.
pub struct TrainerCorpus {
    pub epoch: Option<u64>,
    pub shape_line: Option<String>,
    pub epochs_installed: usize,
    pub oldest_epoch: Option<u64>,
    pub newest_epoch: Option<u64>,
}

/// The lease-host URL at this build's posture — the ONE resolver for the
/// health views (both call sites). `points` owns the points-plane env
/// override + the tuna default; a kat_service-only build has no points
/// module, so it rides the kat lease plane's own resolver (the same
/// `RIIR_REFINE_KAT_SERVICE_URL` > config > mainnet ladder every other kat
/// lane uses). The two defaults point at the same fleet; the env override
/// stays plane-specific by design.
#[cfg(any(feature = "kat_service", feature = "points"))]
#[must_use]
fn lease_url_at_posture() -> String {
    #[cfg(feature = "points")]
    {
        crate::points_lease::resolved_lease_url()
    }
    #[cfg(not(feature = "points"))]
    {
        crate::kat_lease_client::service_url_for_command()
    }
}

#[cfg(any(feature = "kat_service", feature = "points"))]
#[must_use]
pub fn trainer_corpus(root: &Path) -> TrainerCorpus {
    let url = lease_url_at_posture();
    let store = crate::kat_lease_client::LeaseStore::open_net(root, &url);
    let mut epochs: Vec<u64> = Vec::new();
    let man = store.dir().join("man");
    if let Ok(rd) = std::fs::read_dir(&man) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".bin") {
                if let Ok(n) = stem.parse::<u64>() {
                    epochs.push(n);
                }
            }
        }
    }
    epochs.sort_unstable();
    let epoch = store.current_epoch();
    let shape = store
        .current()
        .and_then(|_| crate::kat_lease_client::overlay_from_epoch(&store, None))
        .and_then(|(_, overlay)| crate::kat_lease_client::corpus_shape_line(&overlay));
    TrainerCorpus {
        epoch,
        shape_line: shape,
        epochs_installed: epochs.len(),
        oldest_epoch: epochs.first().copied(),
        newest_epoch: epochs.last().copied(),
    }
}

/// The slim posture: no lease surface.
#[cfg(not(any(feature = "kat_service", feature = "points")))]
#[must_use]
pub fn trainer_corpus(_root: &Path) -> TrainerCorpus {
    TrainerCorpus {
        epoch: None,
        shape_line: None,
        epochs_installed: 0,
        oldest_epoch: None,
        newest_epoch: None,
    }
}

/// The learning half's posture: TRUE when the binary carries the
/// `clippy_verify` oracle (the EMA learning half can certify), FALSE when it
/// is the designed memory-only posture.
#[must_use]
pub fn learning_enabled() -> bool {
    crate::build_stamp::has("clippy_verify")
}

/// The fixer view's inputs (a struct, not a 9-arg renderer — the renderer
/// stays a pure formatter).
pub struct FixerView<'a> {
    pub version_lines: &'a [String],
    pub corpus_line: &'a str,
    pub campaigns: usize,
    pub findings: usize,
    pub kept: usize,
    pub reverted: usize,
    pub last: Option<&'a CampaignSummary>,
    pub misses: usize,
    pub network_line: Option<&'a str>,
    /// Lifetime (kept, fixable, rate%, unfixed%) over the clippy fix-set
    /// axis; `None` = no measurable denominator yet.
    pub rate: Option<(usize, usize, u64, u64)>,
}

/// Render the fixer view. `network_line` is the points aggregate (or the
/// offline/pointer line) — the caller owns the network I/O.
#[must_use]
pub fn render_fixer(view: &FixerView<'_>) -> String {
    use std::fmt::Write as _;
    let FixerView {
        version_lines,
        corpus_line,
        campaigns,
        findings,
        kept,
        reverted,
        last,
        misses,
        network_line,
        rate,
    } = view;
    let mut out = String::new();
    let _ = writeln!(out, "fixer lane — the local heal surface");
    for l in *version_lines {
        let _ = writeln!(out, "  {l}");
    }
    let _ = writeln!(out, "  {corpus_line}");
    let _ = writeln!(
        out,
        "  posture: fix_verify {} · self_evolve {} · offline {}",
        crate::build_stamp::has("fix_verify"),
        crate::build_stamp::has("self_evolve"),
        crate::offline::offline(),
    );
    let _ = writeln!(
        out,
        "  history: {campaigns} fix campaign(s) · {findings} finding(s) cited · {kept} edit(s) kept · {reverted} reverted"
    );
    match rate {
        Some((k, f, pct, unfixed)) => {
            let _ = writeln!(
                out,
                "  fix rate: {} landed ({}/{}) · {}",
                crate::kat_footer::colorize(&["32;1"], &format!("{pct}%")),
                crate::kat_footer::colorize(&["36"], &k.to_string()),
                crate::kat_footer::colorize(&["36"], &f.to_string()),
                crate::kat_footer::colorize(&["2"], &format!("~{unfixed}% unfixed")),
            );
        }
        None => {
            let _ = writeln!(
                out,
                "  fix rate: (no fix-set citations recorded yet — the denominator is the compiled clippy fix set)"
            );
        }
    }
    if let Some(l) = last {
        let verify = if l.verify_ran {
            "verify ran"
        } else {
            "no verify (not a fix_verify build or --no-verify)"
        };
        let rate_note = if l.fixable_clippy > 0 {
            format!(
                ", {p}% of its fix-set citations",
                p = (l.kept_edits as u64) * 100 / l.fixable_clippy as u64
            )
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "  last: {} findings, kept {}, reverted {}{rate_note} ({verify})",
            l.findings_total, l.kept_edits, l.reverted
        );
    } else {
        let _ = writeln!(
            out,
            "  last: (no campaign yet — run `cargo refine fix` on a repo)"
        );
    }
    let _ = writeln!(
        out,
        "  misses: {misses} queued authoring row(s) in .refine/misses/ (the corpus-gap evidence)"
    );
    match network_line {
        Some(line) => {
            let _ = writeln!(out, "network: {line}");
        }
        None => {
            let _ = writeln!(
                out,
                "network: (this build has no points surface — rank/info live in the shipped set)"
            );
        }
    }
    out
}

/// Human bytes (the largest unit that keeps one decimal point honest).
#[must_use]
pub fn hum_bytes(b: u64) -> String {
    const UNITS: [(u64, &str); 4] = [
        (1, "B"),
        (1 << 10, "KiB"),
        (1 << 20, "MiB"),
        (1 << 30, "GiB"),
    ];
    let mut idx = 0usize;
    while idx + 1 < UNITS.len() && b >= UNITS[idx + 1].0 {
        idx += 1;
    }
    let (div, unit) = UNITS[idx];
    if div == 1 {
        format!("{b} {unit}")
    } else {
        // Display only — sizes far below 2^52; the lint's precision loss is
        // unobservable at KiB+ one-decimal rendering.
        #[allow(clippy::cast_precision_loss)]
        let b_f = b as f64;
        #[allow(clippy::cast_precision_loss)]
        let div_f = div as f64;
        format!("{:.1} {unit}", b_f / div_f)
    }
}

/// Render the trainer view. The opt-out is the env law only
/// (`RIIR_REFINE_SELF_EVOLVE=0`) — the settings.toml family owns the
/// auto-check flags, not this lane. `corpus` is the trainer's OUTPUT — the
/// leased corpus this machine installs each epoch (the network trainer's
/// training landing as new rules the fixer cites).
#[must_use]
pub fn render_trainer(
    learning: bool,
    memory: &TrainerMemory,
    corpus: &TrainerCorpus,
    env_optout: bool,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "trainer lane — self-evolve health");
    let _ = writeln!(
        out,
        "  learning half: {}",
        if learning {
            "ENABLED (clippy_verify oracle compiled — fixes certify and the EMA learns)"
        } else {
            "memory-only (learning disabled: built without the clippy_verify oracle) — the DESIGNED shipped posture"
        }
    );
    let _ = writeln!(
        out,
        "  memory half: {}",
        if memory.store_exists {
            format!(
                "live — wal {}, {} segment(s){}{}",
                hum_bytes(memory.wal_bytes),
                memory.segment_count,
                memory
                    .oldest_seg
                    .as_ref()
                    .map_or(String::new(), |s| format!(", oldest {s}")),
                memory
                    .newest_seg
                    .as_ref()
                    .map_or(String::new(), |s| format!(", newest {s}")),
            )
        } else {
            "empty — no store yet; the first `cargo refine fix` creates it".to_string()
        }
    );
    let optout = env_optout;
    let _ = writeln!(
        out,
        "  opt-out: {}",
        if optout {
            "SELF-EVOLVE OFF (env or settings) — fixes still apply, nothing records"
        } else {
            "recording on (RIIR_REFINE_SELF_EVOLVE unset/on, settings off not set)"
        }
    );
    // The trainer's output: the leased corpus. THIS is how the trainer gets
    // measurably smarter — every epoch, the network's distilled rules land here.
    match corpus.epoch {
        Some(e) => {
            let _ = writeln!(out, "  corpus (the trainer's output): epoch {e}");
            if let Some(shape) = &corpus.shape_line {
                let _ = writeln!(out, "    {shape}");
            }
            let range = match (corpus.oldest_epoch, corpus.newest_epoch) {
                (Some(o), Some(n)) if n > o => format!(
                    "    upgrades {o}→{n} ({} installed) — each epoch is a training round\n",
                    corpus.epochs_installed
                ),
                _ => format!("    {} epoch(s) installed\n", corpus.epochs_installed),
            };
            let _ = write!(out, "{range}");
        }
        None => {
            let _ = writeln!(
                out,
                "  corpus: no lease cache yet — run `cargo refine update` to install the leased corpus"
            );
        }
    }
    let _ = writeln!(
        out,
        "  loop: unfixed → .refine/misses/ → distilled rules → the next epoch → the fixer cites them"
    );
    out
}

/// The corpus growth the lease store can answer locally: current epoch's
/// rule total + the delta vs the PREVIOUS installed epoch (by epoch
/// number, not list order). `previous_cached = false` renders the honest
/// "previous epoch not cached" line (never a fake +0) — the verdict's law.
pub struct CorpusGrowth {
    pub epoch: Option<u64>,
    pub rules: Option<usize>,
    pub shape_line: Option<String>,
    pub delta: Option<isize>,
    pub previous_cached: bool,
    pub epochs_installed: usize,
}

#[cfg(any(feature = "kat_service", feature = "points"))]
#[must_use]
pub fn corpus_growth(root: &Path) -> CorpusGrowth {
    let url = lease_url_at_posture();
    let store = crate::kat_lease_client::LeaseStore::open_net(root, &url);
    let epoch = store.current_epoch();
    let (rules, shape, delta, prev_cached) = match epoch {
        Some(e) => {
            let overlay = crate::kat_lease_client::overlay_from_epoch(&store, None);
            let shape = overlay
                .as_ref()
                .and_then(|(_, o)| crate::kat_lease_client::corpus_shape_line(o));
            let total = overlay.map_or(0, |(_, o)| o.iter().map(|(_, v)| v.len()).sum::<usize>());
            let prev = e
                .checked_sub(1)
                .and_then(|p| crate::kat_lease_client::overlay_from_epoch(&store, Some(p)));
            match prev {
                Some((_, po)) => {
                    let ptotal = po.iter().map(|(_, v)| v.len()).sum::<usize>();
                    (
                        Some(total),
                        shape,
                        Some(total.cast_signed() - ptotal.cast_signed()),
                        true,
                    )
                }
                None => (Some(total), shape, None, false),
            }
        }
        None => (None, None, None, false),
    };
    let mut epochs = 0usize;
    let man = store.dir().join("man");
    if let Ok(rd) = std::fs::read_dir(&man) {
        epochs = rd.flatten().count();
    }
    CorpusGrowth {
        epoch,
        rules,
        shape_line: shape,
        delta,
        previous_cached: prev_cached,
        epochs_installed: epochs,
    }
}

#[cfg(not(any(feature = "kat_service", feature = "points")))]
#[must_use]
pub fn corpus_growth(_root: &Path) -> CorpusGrowth {
    CorpusGrowth {
        epoch: None,
        rules: None,
        shape_line: None,
        delta: None,
        previous_cached: false,
        epochs_installed: 0,
    }
}

/// Render the `stats` panel — the standing aggregate the owner asked for
/// (2026-11-06): the fixer's success rate (this machine's campaign ledger,
/// scope + sample size named), the trainer's output, the corpus growth,
/// and the network aggregate. Every line states its precondition when its
/// data is absent — no fake numbers, ever (the verdict's law).
#[must_use]
pub fn render_stats(root: &Path, network_line: Option<&str>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "self-evolve loop — fixer · trainer · corpus · network");

    // 🛠️ the fixer's success rate (kept / fixable-citations, THIS machine,
    // scope + sample named per the verdict).
    let (campaigns, _findings, _kept, _reverted, _last) = fixer_history(root);
    match fixer_rate(root) {
        Some((k, f, pct, _unfixed)) => {
            let _ = writeln!(
                out,
                "🛠️  {} fixer success ({}/{}) — {}",
                crate::kat_footer::colorize(&["32;1"], &format!("{pct}%")),
                crate::kat_footer::colorize(&["36"], &k.to_string()),
                crate::kat_footer::colorize(&["36"], &f.to_string()),
                crate::kat_footer::colorize(
                    &["2"],
                    &format!("this machine, {campaigns} campaign(s)")
                ),
            );
        }
        None if campaigns > 0 => {
            let _ = writeln!(
                out,
                "\u{1f6e0}\u{fe0f} fixer success rate: (no fix-set citations recorded in {campaigns} campaign(s) — the denominator is the compiled clippy fix set)"
            );
        }
        None => {
            let _ = writeln!(
                out,
                "\u{1f6e0}\u{fe0f} fixer success rate: (no campaigns yet — run `cargo refine fix` on a repo)"
            );
        }
    }

    // 🔬 the trainer: the leased corpus = its output so far. SHORT lines
    // (the owner's 2026-11-06 UX call: the one-liner was too long).
    let growth = corpus_growth(root);
    match growth.epoch {
        Some(e) => {
            let _ = writeln!(
                out,
                "🔬 {} · {} epoch{} · {}",
                crate::kat_footer::colorize(
                    &["36"],
                    &format!("{} rules", growth.rules.unwrap_or(0))
                ),
                crate::kat_footer::colorize(&["36"], &growth.epochs_installed.to_string()),
                if growth.epochs_installed == 1 {
                    ""
                } else {
                    "s"
                },
                crate::kat_footer::colorize(&["2"], &format!("epoch {e}")),
            );
            if let Some(d) = growth.delta {
                let (word, code) = if d > 0 { ("+", "32") } else { ("", "33") };
                let _ = writeln!(
                    out,
                    "  {} {}",
                    crate::kat_footer::colorize(&[code], &format!("{word}{d}")),
                    crate::kat_footer::colorize(&["2"], "vs prev epoch"),
                );
            } else if !growth.previous_cached {
                let _ = writeln!(
                    out,
                    "  {}",
                    crate::kat_footer::colorize(&["2"], "prev epoch not cached — no delta")
                );
            }
            let _ = writeln!(
                out,
                "  {}",
                crate::kat_footer::colorize(&["2"], "coverage feed pending (dapps #162)")
            );
        }
        None => {
            let _ = writeln!(
                out,
                "🔬 (no lease cache yet — run `cargo refine update` to install the trained corpus)"
            );
        }
    }

    render_corpus_growth(&growth, &mut out);

    // 📡 network aggregate (points) or its honest pointer.
    match network_line {
        Some(line) => {
            let _ = writeln!(out, "📡 {line}");
        }
        None => {
            let _ = writeln!(
                out,
                "📡 (no points surface — rank/info live in the shipped set)"
            );
        }
    }
    out
}

/// The 📚 corpus block — a cargo-tree-style subtree: one `├──` child per
/// manifest-numbered domain (emoji + colored count + a braille bar of its
/// share of the compiled corpus), the unnumbered tail as `└──`, and the
/// leased-growth delta under the tree.
fn render_corpus_growth(growth: &CorpusGrowth, out: &mut String) {
    use std::fmt::Write as _;
    writeln!(out, "📚 corpus").ok();
    let bar_w = 10usize;
    let rows = corpus_rows();
    let branch = |last: bool| crate::kat_footer::colorize(&["2"], if last { "└──" } else { "├──" });
    let last = rows.len();
    for (idx, row) in rows.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let share = if row.total > 0 {
            row.rules as f64 / row.total as f64
        } else {
            0.0
        };
        writeln!(
            out,
            "  {} {} {} {}",
            branch(idx + 1 == last),
            row.emoji,
            crate::kat_footer::colorize(&["36"], &format!("{:>4} {}", row.rules, row.label)),
            crate::kat_footer::braille_bar(share, bar_w, crate::kat_footer::color_enabled()),
        )
        .ok();
    }
    writeln!(
        out,
        "  {}",
        crate::kat_footer::colorize(
            &["2"],
            "also compiled: rustc_errors (oracle-driven, uncounted)"
        )
    )
    .ok();
    if let Some(d) = growth.delta {
        let (word, code) = if d > 0 { ("+", "32") } else { ("", "33") };
        writeln!(
            out,
            "  {} {}",
            crate::kat_footer::colorize(&[code], &format!("leased {word}{d}")),
            crate::kat_footer::colorize(&["2"], "vs prev epoch (the trainer's growth)")
        )
        .ok();
    }
}

/// One corpus row for the 🧬 block: emoji + count + label + the share
/// denominator. Counts from the pins manifest (the ONE source of truth);
/// the nested domains are the prose-gated four, the also-compiled trio
/// (docker/dist/shader) rides flat `*_rules` keys.
struct CorpusRow {
    emoji: &'static str,
    label: &'static str,
    rules: usize,
    total: usize,
}

#[must_use]
fn corpus_rows() -> Vec<CorpusRow> {
    // Same embedded manifest corpus_counts_line reads.
    #[derive(serde::Deserialize)]
    struct Manifest {
        #[serde(default)]
        clippy_lints: Option<Count>,
        #[serde(default)]
        rust_perf: Option<Count>,
        #[serde(default)]
        kernel_opt: Option<Count>,
        #[serde(default)]
        sec: Option<Count>,
        #[serde(default)]
        docker_rules: Option<u64>,
        #[serde(default)]
        dist_rules: Option<u64>,
        #[serde(default)]
        shader_perf_rules: Option<u64>,
    }
    #[derive(serde::Deserialize)]
    struct Count {
        rules: u64,
    }
    let (clippy, perf, kernel, sec, docker, dist, shader) =
        match serde_json::from_str::<Manifest>(include_str!("../.benchmarks/pins_manifest.json")) {
            Ok(m) => (
                m.clippy_lints
                    .map(|c| usize::try_from(c.rules).unwrap_or(0)),
                m.rust_perf.map(|c| usize::try_from(c.rules).unwrap_or(0)),
                m.kernel_opt.map(|c| usize::try_from(c.rules).unwrap_or(0)),
                m.sec.map(|c| usize::try_from(c.rules).unwrap_or(0)),
                m.docker_rules.map(|n| usize::try_from(n).unwrap_or(0)),
                m.dist_rules.map(|n| usize::try_from(n).unwrap_or(0)),
                m.shader_perf_rules.map(|n| usize::try_from(n).unwrap_or(0)),
            ),
            Err(_) => (None, None, None, None, None, None, None),
        };
    let sum = [clippy, perf, kernel, sec, docker, dist, shader]
        .into_iter()
        .flatten()
        .sum::<usize>();
    let mut rows = Vec::new();
    if let Some(n) = clippy {
        rows.push(CorpusRow {
            emoji: "📎",
            label: "clippy",
            rules: n,
            total: sum,
        });
    }
    if let Some(n) = perf {
        rows.push(CorpusRow {
            emoji: "⚡",
            label: "perf",
            rules: n,
            total: sum,
        });
    }
    if let Some(n) = kernel {
        rows.push(CorpusRow {
            emoji: "🧬",
            label: "kernel",
            rules: n,
            total: sum,
        });
    }
    if let Some(n) = sec {
        rows.push(CorpusRow {
            emoji: "🛡️",
            label: "sec",
            rules: n,
            total: sum,
        });
    }
    if let Some(n) = docker {
        rows.push(CorpusRow {
            emoji: "🐳",
            label: "docker",
            rules: n,
            total: sum,
        });
    }
    if let Some(n) = dist {
        rows.push(CorpusRow {
            emoji: "📦",
            label: "dist",
            rules: n,
            total: sum,
        });
    }
    if let Some(n) = shader {
        rows.push(CorpusRow {
            emoji: "🎨",
            label: "shader",
            rules: n,
            total: sum,
        });
    }
    // Biggest first (the roll-up's convention) — the owner's sort call.
    rows.sort_by_key(|r| std::cmp::Reverse(r.rules));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_line_carries_the_pinned_counts() {
        let line = corpus_counts_line();
        assert!(line.contains("clippy 120"), "{line}");
        assert!(line.contains("kernel 1078"), "{line}");
        assert!(line.contains("perf 318"), "{line}");
        assert!(line.contains("sec 61"), "{line}");
        assert!(line.contains("docker 31"), "{line}");
        assert!(line.contains("dist 8"), "{line}");
        assert!(line.contains("shader 8"), "{line}");
    }

    #[test]
    fn corpus_tree_carries_the_also_compiled_trio() {
        let rows = corpus_rows();
        let by_label = |label: &str| {
            rows.iter()
                .find(|r| r.label == label)
                .unwrap_or_else(|| panic!("corpus tree missing {label}"))
                .rules
        };
        assert_eq!(by_label("docker"), 31);
        assert_eq!(by_label("dist"), 8);
        assert_eq!(by_label("shader"), 8);
        // Biggest first, ties stable (dist before shader): the seven
        // compiled domains, no unnumbered survivors in the bars.
        let labels: Vec<&str> = rows.iter().map(|r| r.label).collect();
        assert_eq!(
            labels,
            vec![
                "kernel", "perf", "clippy", "sec", "docker", "dist", "shader"
            ]
        );
        assert_eq!(rows[0].total, 1624); // Batch 356: rust_perf +4 (entries_72, the r/rust AI-appeasement quartet, Plan 366 / Research 400); prior: Batch 357: kernel +2 (the moeshift distill, Plan 367 / Research 401); prior: Batch 351: sec +5 (the foxguard distill, Plan 361 / Research 394); prior: Batch 350: sec +5 (the 0sec distill, Plan 356 / Research 388); prior: Batch 349: rust_perf +2 (entries_71, Plan 360 / Research 393); prior: Batch 348: clippy +2 (approx_constant + useless_vec — the P111 + P112 corpus gaps, plan 359); prior: Batch 347: clippy +1 (let_unit_value — the P120 rulewright-miss trio, plan 358); prior: plan 354: clippy +2 (the doc-section pair, the Bench-141 dual-evidence heads); prior: Batch 345: perf +5 (entries_70, the LocalSR distill, Plan 357 / Research 389) + sec +1 (the execution-artifact trust ladder) + kernel +1 (entries_gpu_154, the backend acceptance gate); prior: Batch 343: perf +2 (entries_69, the sccache cache-layer pair, Plan 355 / Research 387); prior: Batch 342: sec +2 (the harness-adapters pair, Plan 352 / Research 385); prior: Batch 341: kernel +2 (entries_gpu_153) + perf +1 (entries_68, the metal-graph pair + the measurement-honesty rule, Plan 350 / Research 383); prior: Batch 340: perf +2 (entries_67) + sec +2 (the SPILLWAY pair, Plan 349 / Research 381); P118: clippy +3 (plan 353)
    }

    #[test]
    fn empty_history_is_an_honest_zero() {
        let dir = std::env::temp_dir().join(format!("refine_health_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        let (c, f, k, r, last) = fixer_history(&dir);
        assert_eq!((c, f, k, r, last.is_none()), (0, 0, 0, 0, true));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_trainer_store_is_the_honest_zero() {
        let dir = std::env::temp_dir().join(format!("refine_health_test2_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        let m = trainer_memory(&dir);
        assert!(!m.store_exists);
        assert_eq!(m.wal_bytes, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn renderers_name_the_lane_and_the_pointers() {
        let mem = TrainerMemory {
            store_exists: false,
            wal_bytes: 0,
            segment_count: 0,
            oldest_seg: None,
            newest_seg: None,
        };
        let corpus = TrainerCorpus {
            epoch: Some(2971),
            shape_line: Some("corpus: 74 rules — clippy 74".into()),
            epochs_installed: 15,
            oldest_epoch: Some(2957),
            newest_epoch: Some(2971),
        };
        let t = render_trainer(false, &mem, &corpus, false);
        assert!(t.contains("trainer lane"), "{t}");
        assert!(t.contains("memory-only"), "{t}");
        assert!(t.contains("epoch 2971"), "{t}");
        assert!(t.contains("2957→2971"), "{t}");
        assert!(t.contains("loop:"), "{t}");
        let empty = TrainerCorpus {
            epoch: None,
            shape_line: None,
            epochs_installed: 0,
            oldest_epoch: None,
            newest_epoch: None,
        };
        let t2 = render_trainer(false, &mem, &empty, false);
        assert!(t2.contains("no lease cache yet"), "{t2}");
        let f = render_fixer(&FixerView {
            version_lines: &["cargo-refine 0.0.0".into()],
            corpus_line: &corpus_counts_line(),
            campaigns: 0,
            findings: 0,
            kept: 0,
            reverted: 0,
            last: None,
            misses: 0,
            network_line: None,
            rate: Some((7, 10, 70, 30)),
        });
        assert!(f.contains("fixer lane"), "{f}");
        assert!(f.contains("70%"), "{f}");
        assert!(f.contains("~30% unfixed"), "{f}");
    }

    #[test]
    fn fixer_rate_denominator_law() {
        // A history with zero fix-set citations yields None (never a fake 0%).
        let dir = std::env::temp_dir().join(format!("refine_health_rate_{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".refine")).expect("tmpdir");
        assert_eq!(fixer_rate(&dir), None);
        std::fs::remove_dir_all(&dir).ok();
    }
}
