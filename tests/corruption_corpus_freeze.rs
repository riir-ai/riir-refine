//! Issue 137 T1/T3 — the generation-v9 freeze/verify e2e (the CRB
//! corruption corpus: compile-clean / test-failing fixtures with
//! known-position spans).
//!
//! Two modes, one file (the v7/v8 convention):
//!
//! - **Write** (`--ignored` + `RIIR_CRB_GEN_WRITE=1`): builds the seeded
//!   candidate grid over the seed programs, runs the TWO-TIER FILTER per
//!   candidate with real cargo in a pid-isolated temp crate
//!   (`cargo check` must stay CLEAN, then `cargo test` must FAIL — only a
//!   clean non-zero test verdict counts; signal-killed and timed-out
//!   candidates are REJECTED, never kept as fixtures), keeps compile-failing
//!   candidates as the separate CONTROL arm, renders the corpus module, and
//!   writes `src/score_bench/corruption_corpus_v9.rs`. Commit the result —
//!   never hand-edit it.
//!
//! - **Verify** (`--ignored`, default): re-runs the filter VERDICTS over
//!   the COMMITTED corpus — every headline fixture must still check clean
//!   AND test fail, every control fixture must still check FAIL — and
//!   re-asserts the digest + floors. A drift (toolchain moved the verdicts,
//!   or the table was hand-edited) is a loud failure, never a pass.
//!
//! - **Always-on** (default `cargo test`): the corpus pin test lives in
//!   `corruption.rs` (`corpus_v9_pins_hold`) — digest + floors + structural
//!   sanity, no cargo.
//!
//! ```bash
//! # generate (then commit the written module):
//! RIIR_CRB_GEN_WRITE=1 cargo test --release --features score_bench \
//!     --test corruption_corpus_freeze -- --ignored --test-threads=1 --nocapture
//! # verify:
//! cargo test --release --features score_bench \
//!     --test corruption_corpus_freeze -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(feature = "score_bench")]

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use riir_refine::score_bench::corruption::{self, Candidate, SeedProgram};

/// The generator seed (fixed — the corpus must be reproducible).
const GEN_SEED: u64 = 0x4352_4239; // "CRB9"
/// Draws per (seed program, |E|).
const DRAWS_PER_K: usize = 10;
/// Admitted headline fixtures per (seed program, |E|).
const ADMIT_CAP_PER_SEED_K: usize = 2;
/// Control-arm (compile-failing) keeps, whole corpus.
const CONTROL_CAP: usize = 6;
/// Per-cargo-invocation wall cap (the hang-safety net; a kill counts as a
/// rejection, never a fixture).
const CARGO_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
#[ignore = "spawns real cargo per candidate; run with --ignored --test-threads=1"]
fn freeze_generation_v9() {
    let generate = std::env::var("RIIR_CRB_GEN_WRITE").ok().as_deref() == Some("1");
    if generate {
        write_mode();
    } else {
        verify_mode();
    }
}

// ── the temp probe crate ────────────────────────────────────────────────

struct ProbeCrate {
    root: std::path::PathBuf,
}

impl ProbeCrate {
    fn create() -> Self {
        let root = std::env::temp_dir().join(format!("crb_gen_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("create probe crate");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"crb_probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write probe manifest");
        Self { root }
    }

    fn write_lib(&self, code: &str) {
        std::fs::write(self.root.join("src/lib.rs"), code).expect("write probe lib");
    }

    /// One cargo invocation with the pid-local target dir + a wall cap.
    /// Returns `Ok(exit)` or `Err(())` on timeout (the child is killed —
    /// a timed-out candidate is a rejection, never a fixture).
    fn cargo(&self, subcommand: &str) -> Result<std::process::ExitStatus, ()> {
        let mut child = Command::new("cargo")
            .arg(subcommand)
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(self.root.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", self.root.join("target"))
            .env("CARGO_TERM_COLOR", "never")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn cargo");
        let deadline = Instant::now() + CARGO_TIMEOUT;
        loop {
            match child.try_wait().expect("poll cargo") {
                Some(status) => return Ok(status),
                None => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }

    /// The seed gate: the uncorrupted program must check clean AND pass its
    /// tests — else the seed table is broken and the run aborts.
    fn verify_seed(&self, program: &SeedProgram) {
        self.write_lib(program.code);
        let check = self.cargo("check").expect("seed check timed out (?!)");
        assert!(
            check.success(),
            "seed {} does not check clean — the seed table is broken",
            program.id
        );
        let test = self.cargo("test").expect("seed test timed out (?!)");
        assert!(
            test.success(),
            "seed {} does not pass its own tests — the seed table is broken",
            program.id
        );
    }
}

// ── verdicts ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Verdict {
    /// check clean + test fails — the headline bucket.
    Admitted,
    /// check fails — the control arm (rustc marks those spans).
    Control,
    /// check clean + tests still PASS — not broken, rejected.
    TestPassed,
    /// killed by a signal (abort / stack overflow) — rejected: the failure
    /// mode is the whole program, not a localizable position.
    /// Constructed only under `#[cfg(unix)]` (the `ExitStatusExt::signal`
    /// probe) — on non-unix hosts the classifier cannot observe signals, so
    /// the variant is deliberately never constructed there.
    #[cfg_attr(not(unix), allow(dead_code))]
    Signal,
    /// exceeded the wall cap — rejected (the hang-safety net).
    TimedOut,
}

fn classify(probe: &ProbeCrate, candidate: &Candidate) -> Verdict {
    probe.write_lib(&candidate.code);
    let check = match probe.cargo("check") {
        Ok(status) => status,
        Err(()) => return Verdict::TimedOut,
    };
    if !check.success() {
        return Verdict::Control;
    }
    let test = match probe.cargo("test") {
        Ok(status) => status,
        Err(()) => return Verdict::TimedOut,
    };
    if test.success() {
        return Verdict::TestPassed;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt as _;
        if test.signal().is_some() {
            return Verdict::Signal;
        }
    }
    Verdict::Admitted
}

// ── corpus rendering ────────────────────────────────────────────────────

struct Record {
    id: String,
    k: usize,
    code: String,
    spans: Vec<(usize, usize)>,
    classes: Vec<&'static str>,
    blake3: String,
    seed_id: &'static str,
}

fn record_of(candidate: &Candidate, seq: usize) -> Record {
    Record {
        id: format!("gen9-{}-k{}-d{seq:02}", candidate.seed_id, candidate.k),
        k: candidate.k,
        code: candidate.code.clone(),
        spans: candidate.spans.clone(),
        classes: candidate.classes.iter().map(|c| c.tag()).collect(),
        blake3: String::new(), // filled by the renderer digest pass
        seed_id: candidate.seed_id,
    }
}

fn spec_blake3_hex(record: &Record) -> String {
    corruption::spec_blake3(
        &record.id,
        record.k,
        &record.code,
        &record.spans,
        &record.classes,
    )
}

fn render_code_literal(code: &str) -> String {
    // Minimal raw-string delimiter: `r"..."` only when the code contains NO
    // `"` at all (a zero-hash raw string terminates at any quote);
    // otherwise 1 + the longest run of `#`s directly after a quote, so the
    // delimiter cannot occur inside the code. Keeps the generated file
    // clippy-clean under `unnecessary_hashes`.
    let mut longest = 0usize;
    let mut has_quote = false;
    let bytes = code.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'"' {
            has_quote = true;
            let mut run = 0usize;
            let mut cursor = index + 1;
            while cursor < bytes.len() && bytes[cursor] == b'#' {
                run += 1;
                cursor += 1;
            }
            longest = longest.max(run);
        }
    }
    let hashes = if has_quote { longest + 1 } else { 0 };
    let delimiters = "#".repeat(hashes);
    format!("r{delimiters}\"{code}\"{delimiters}")
}

fn render_table(name: &str, records: &[Record]) -> String {
    let mut out = String::new();
    out.push_str(&format!("pub static {name}: &[CorruptionSpec] = &[\n"));
    for record in records {
        let classes: Vec<String> = record
            .classes
            .iter()
            .map(|class| format!("{class:?}"))
            .collect();
        let spans: Vec<String> = record
            .spans
            .iter()
            .map(|(s, e)| format!("({s}, {e})"))
            .collect();
        out.push_str("    CorruptionSpec {\n");
        out.push_str(&format!("        id: {:?},\n", record.id));
        out.push_str(&format!("        k: {},\n", record.k));
        out.push_str(&format!(
            "        code: {},\n",
            render_code_literal(&record.code)
        ));
        out.push_str(&format!("        spans: &[{}],\n", spans.join(", ")));
        out.push_str(&format!("        classes: &[{}],\n", classes.join(", ")));
        out.push_str(&format!("        blake3: {:?},\n", record.blake3));
        out.push_str(&format!("        seed_program: {:?},\n", record.seed_id));
        out.push_str("    },\n");
    }
    out.push_str("];\n");
    out
}

/// Render the whole corpus module (the write mode's artifact).
fn render_corpus_module(headline: &[Record], control: &[Record]) -> String {
    let mut out = String::new();
    out.push_str(
        "//! GENERATED by `tests/corruption_corpus_freeze.rs` (Issue 137 T1/T3) —\n\
         //! do not hand-edit; regenerate with `RIIR_CRB_GEN_WRITE=1` and commit\n\
         //! the result (the digest pin reds on any in-place edit).\n\n\
         use super::corruption::CorruptionSpec;\n\n",
    );
    out.push_str(&render_table("CORRUPTION_SPECS_V9", headline));
    out.push('\n');
    out.push_str(&render_table("CONTROL_SPECS_V9", control));
    out.push('\n');
    let digest = {
        // mirror of corruption::corpus_digest over the NEW records (the
        // module's own fn reads the committed statics — here they are not
        // yet committed, so the digest is computed over the same canonical
        // bytes here).
        let mut input = Vec::new();
        let mut rows: Vec<(bool, &Record)> = Vec::new();
        for record in headline {
            rows.push((false, record));
        }
        for record in control {
            rows.push((true, record));
        }
        rows.sort_by_key(|(control, record)| (record.id.as_str(), *control));
        for (control, record) in rows {
            input.extend_from_slice(if control { b"C" } else { b"H" });
            input.extend_from_slice(record.id.as_bytes());
            input.push(0);
            input.extend_from_slice(&(record.k as u64).to_le_bytes());
            input.extend_from_slice(record.code.as_bytes());
            input.push(0);
            for (start, end) in &record.spans {
                input.extend_from_slice(&(*start as u64).to_le_bytes());
                input.extend_from_slice(&(*end as u64).to_le_bytes());
            }
            input.push(0);
            for class in &record.classes {
                input.extend_from_slice(class.as_bytes());
                input.push(0);
            }
        }
        blake3::hash(&input).to_hex().to_string()
    };
    out.push_str(&format!(
        "/// The blake3 append-refusal pin over both arms (mirrors\n\
         /// `corruption::corpus_digest` over the committed statics).\n\
         pub const CORPUS_V9_DIGEST: &str = \"{digest}\";\n"
    ));
    out
}

// ── the two modes ───────────────────────────────────────────────────────

fn write_mode() {
    let probe = ProbeCrate::create();
    let candidates = corruption::build_candidates(GEN_SEED, DRAWS_PER_K);
    assert!(
        !candidates.is_empty(),
        "candidate grid empty — the engine is broken"
    );

    // the seed gate first
    for program in corruption::seed_programs() {
        probe.verify_seed(program);
    }
    println!(
        "seed gate: {} seed program(s) check clean and pass their tests",
        corruption::seed_programs().len()
    );

    let mut counters: std::collections::BTreeMap<Verdict, usize> =
        std::collections::BTreeMap::new();
    let mut noop_rejected = 0usize;
    let mut text_dup_rejected = 0usize;
    let mut skipped_by_cap = 0usize;
    let mut seen_text: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut admitted_per_seed_k: std::collections::BTreeMap<(&'static str, usize), usize> =
        std::collections::BTreeMap::new();
    let mut headline: Vec<Record> = Vec::new();
    let mut control: Vec<Record> = Vec::new();
    let mut seq = 0usize;

    // iterate candidates grouped so the admit cap binds per (seed, k)
    let mut ordered: Vec<&Candidate> = candidates.iter().collect();
    ordered.sort_by_key(|c| (c.seed_id, c.k, c.draw));
    for candidate in ordered {
        // the pre-cargo no-op gate (the planted no-op proof — before any
        // cargo run, identical text is rejected)
        let program = corruption::seed_programs()
            .iter()
            .find(|p| p.id == candidate.seed_id)
            .expect("seed id");
        if corruption::is_noop(&candidate.code, program.code) {
            noop_rejected += 1;
            continue;
        }
        if !seen_text.insert(candidate.code.clone()) {
            text_dup_rejected += 1;
            continue;
        }
        let key = (candidate.seed_id, candidate.k);
        let admitted_here = admitted_per_seed_k.get(&key).copied().unwrap_or(0);
        if admitted_here >= ADMIT_CAP_PER_SEED_K {
            skipped_by_cap += 1;
            continue; // per-(seed, |E|) cap — no cargo burned past it
        }
        let verdict = classify(&probe, candidate);
        *counters.entry(verdict).or_default() += 1;
        match verdict {
            Verdict::Admitted => {
                let mut record = record_of(candidate, seq);
                seq += 1;
                record.blake3 = spec_blake3_hex(&record);
                headline.push(record);
                *admitted_per_seed_k.entry(key).or_default() += 1;
            }
            Verdict::Control => {
                if control.len() < CONTROL_CAP {
                    let mut record = record_of(candidate, seq);
                    seq += 1;
                    record.blake3 = spec_blake3_hex(&record);
                    control.push(record);
                }
            }
            Verdict::TestPassed | Verdict::Signal | Verdict::TimedOut => {}
        }
    }

    // floors (same table as corruption::PER_K_FLOORS — asserted here so a
    // regeneration that breaches them fails loudly instead of landing a
    // thinner corpus)
    let mut summary = String::new();
    for &(k, floor) in corruption::PER_K_FLOORS {
        let count = headline.iter().filter(|r| r.k == k).count();
        assert!(
            count >= floor,
            "per-|E| floor breached at k={k}: {count} < {floor} — widen DRAWS_PER_K or the seed table"
        );
        summary.push_str(&format!("  k={k}: {count} admitted (floor {floor})\n"));
    }

    let module = render_corpus_module(&headline, &control);
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/score_bench/corruption_corpus_v9.rs");
    std::fs::write(&target, &module).expect("write corpus module");

    println!("generation v9 corpus written to {}", target.display());
    println!("verdicts: {counters:?}");
    println!("noop rejected (pre-cargo): {noop_rejected}");
    println!("text dups rejected: {text_dup_rejected}");
    println!("skipped by cap: {skipped_by_cap}");
    println!("headline: {}  control: {}", headline.len(), control.len());
    if control.is_empty() {
        println!(
            "control arm empty — the type-preserving v1 table produced no compile-failing \
             candidate on these seeds (disclosed; the bucket + verify arm stay wired)"
        );
    }
    println!("{summary}");
    let _ = std::io::stdout().flush();
    println!("commit the written module (never hand-edit it).");
}

fn verify_mode() {
    match corruption::v9_freeze_status() {
        Ok(admitted) => println!(
            "generation v9: {admitted} admitted + {} control fixture(s) — pins verified, executing cargo verdicts…",
            corruption::control_corpus().len()
        ),
        Err(why) => {
            eprintln!("generation v9: {why}");
            std::process::exit(1);
        }
    }
    let probe = ProbeCrate::create();
    for fixture in corruption::corpus() {
        probe.write_lib(fixture.code);
        let check = match probe.cargo("check") {
            Ok(status) => status,
            Err(()) => panic!("{}: check TIMED OUT — drift or hang class", fixture.id),
        };
        assert!(
            check.success(),
            "{}: no longer check-clean (toolchain drift?) — regenerate",
            fixture.id
        );
        let test = match probe.cargo("test") {
            Ok(status) => status,
            Err(()) => panic!("{}: test TIMED OUT — hang class, regenerate", fixture.id),
        };
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt as _;
            assert!(
                test.signal().is_none(),
                "{}: test died by signal — the corpus must hold only clean test failures",
                fixture.id
            );
        }
        assert!(
            !test.success(),
            "{}: tests now PASS — the corruption no longer bites, regenerate",
            fixture.id
        );
    }
    for fixture in corruption::control_corpus() {
        probe.write_lib(fixture.code);
        let check = match probe.cargo("check") {
            Ok(status) => status,
            Err(()) => panic!("control {}: check TIMED OUT", fixture.id),
        };
        assert!(
            !check.success(),
            "control {}: now checks clean — control-arm drift, regenerate",
            fixture.id
        );
    }
    println!(
        "generation v9 frozen & execution-verified: {} headline (check clean ∧ test fail) + {} control (check fails)",
        corruption::corpus().len(),
        corruption::control_corpus().len()
    );
}
