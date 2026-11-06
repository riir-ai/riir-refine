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
    pub paths_head: Option<String>,
}

/// Load the blueprint campaign history (newest last) and summarize the
/// fleet: total campaigns, lifetime outcomes, and the last campaign's row.
/// Missing file = an empty history (an honest zero, never an error).
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
        paths_head: None,
    });
    (campaigns, findings, kept, reverted, summary)
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

/// The corpus-lease epoch the machine has cached (the trained corpus the
/// fixer cites). Offline-safe: reads the local store only, through the SAME
/// net-partitioned resolution `update` installs under (the stamp placement
/// law — `.refine/net/<host>/corpus-lease`). Gated to the feature sets
/// that carry the lease client; the slim posture prints the honest None.
#[cfg(any(feature = "kat_service", feature = "points"))]
#[must_use]
pub fn lease_epoch(root: &Path) -> Option<u64> {
    let url = crate::points_lease::resolved_lease_url();
    let store = crate::kat_lease_client::LeaseStore::open_net(root, &url);
    store.current_epoch()
}

/// The slim (no-points, no-kat_service) posture: no lease surface, the
/// honest None.
#[cfg(not(any(feature = "kat_service", feature = "points")))]
#[must_use]
pub fn lease_epoch(_root: &Path) -> Option<u64> {
    None
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
    if let Some(l) = last {
        let verify = if l.verify_ran {
            "verify ran"
        } else {
            "no verify (not a fix_verify build or --no-verify)"
        };
        let _ = writeln!(
            out,
            "  last: {} findings, kept {}, reverted {} ({verify})",
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
/// auto-check flags, not this lane.
#[must_use]
pub fn render_trainer(
    learning: bool,
    memory: &TrainerMemory,
    epoch: Option<u64>,
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
    match epoch {
        Some(e) => {
            let _ = writeln!(
                out,
                "  corpus: lease epoch {e} installed — run `cargo refine update` to refresh"
            );
        }
        None => {
            let _ = writeln!(
                out,
                "  corpus: no lease cache yet — run `cargo refine update` to install the leased corpus"
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_line_carries_the_pinned_counts() {
        let line = corpus_counts_line();
        assert!(line.contains("clippy 112"), "{line}");
        assert!(line.contains("kernel 1061"), "{line}");
        assert!(line.contains("perf 302"), "{line}");
        assert!(line.contains("sec 45"), "{line}");
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
        let t = render_trainer(false, &mem, None, false);
        assert!(t.contains("trainer lane"), "{t}");
        assert!(t.contains("memory-only"), "{t}");
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
        });
        assert!(f.contains("fixer lane"), "{f}");
        assert!(f.contains("cargo refine fix"), "{f}");
    }
}
