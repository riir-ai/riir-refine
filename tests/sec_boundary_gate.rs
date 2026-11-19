//! Plan 018 T0.4 — the do-not-reimplement boundary gate.
//!
//! Proposal 004's non-negotiable: clippy already owns a restriction-lint
//! surface; a sec rule may only exist where clippy does NOT fire **with an
//! equivalent fix**. A lint that fires with NO suggestion (or a
//! MaybeIncorrect one) leaves our bounded fix additive, not duplicative —
//! that is the exact line the proposal draws.
//!
//! This gate runs REAL clippy (the live toolchain, JSON output) over every
//! sec corpus buggy fixture with the banned list enabled, and asserts the
//! measured boundary table:
//!
//! - `unwrap_used` fires on the `drop-impl-no-panic` fixture **with no
//!   machine-applicable suggestion** — our Drop-scoped `.unwrap_or_default()`
//!   fix is additive (clippy offers nothing there).
//! - No other banned lint fires with a MachineApplicable suggestion on any
//!   fixture. A future clippy that grows an auto-fix for one of our
//!   territories must FAIL this gate — that rule then moves to the
//!   do-not-reimplement side (the exact drift this gate exists to catch).
//!
//! Gated behind `--features sec`; SKIPs (not fails) when cargo is missing.
//! Slow by nature (one cargo clippy invocation) — run explicitly:
//!
//! ```bash
//! cargo test --features sec --test sec_boundary_gate -- --nocapture
//! ```

#![cfg(feature = "sec")]

use std::path::PathBuf;
use std::process::Command;

use riir_refine::domain::HealDomain;
use riir_refine::domains::sec::SecDomain;

/// The banned list (Proposal 004): clippy restriction/rustc lints whose
/// territory sec must not duplicate.
const BANNED: &[&str] = &[
    "clippy::mem_forget",
    "clippy::indexing_slicing",
    "clippy::arithmetic_side_effects",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "improper_ctypes",
    "improper_ctypes_definitions",
    "clippy::partialeq_ne_impl",
    "clippy::undocumented_unsafe_blocks",
    "clippy::disallowed_methods",
];

/// One clippy diagnostic with its code + applicability.
#[derive(Debug)]
struct Diag {
    code: String,
    applicability: Option<String>,
    file: String,
}

fn cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// One probe run: the diagnostics of every fixture that COMPILED, plus the
/// fixtures dropped because they do not compile standalone (Issue 249 B6).
struct Probe {
    diags: Vec<Diag>,
    compiled: Vec<usize>,
    /// `(fixture index, first error line)` — named loud, never silent.
    dropped: Vec<(usize, String)>,
}

/// Fixtures that must COMPILE in the probe — the pinned table's rows. A
/// dropped one is a red, never a skip.
const MUST_COMPILE: &[&str] = &["drop-impl-no-panic"];

/// Fixtures that do NOT compile standalone in the probe, pinned by
/// MEMBERSHIP with the reason (measured 2026-11-19, Issue 249 B6: 63 of 77
/// compile). Both directions red: a NEW non-compiling fixture must be
/// classified here (or made self-contained), and a pinned row that starts
/// compiling must leave — the boundary verdict ranges over the compiled set.
const EXPECTED_DROPPED: &[(&str, &str)] = &[
    (
        "sql-identifier-interpolation-unallowlisted",
        "names sqlx — too heavy for the probe",
    ),
    (
        "sql-query-value-interpolation-unbound",
        "names sqlx — too heavy for the probe",
    ),
    (
        "fp16-subnormal-product-power-of-two-rescale",
        "uses unstable f16",
    ),
    ("position-keyed-additive-cache-mask", "uses unstable f16"),
    ("wide-reduction-precision-split", "uses unstable f16"),
    (
        "decay-gate-precision-collapse-low-precision",
        "snippet: names an undefined Bf16",
    ),
    (
        "fused-activation-approximation-expansion",
        "snippet: names an undefined sigmoid",
    ),
    (
        "stale-proposal-merges-against-shown-base",
        "snippet: names an undefined WorkingSet",
    ),
    (
        "user-impl-derived-property-validate",
        "snippet: names an undefined EncodedPattern",
    ),
    (
        "agent-code-static-boundary-diagnostics-never-gate",
        "snippet: names an undefined scan_surfaces",
    ),
    (
        "server-connection-without-read-timeout",
        "snippet: names an undefined handle_connection",
    ),
    (
        "conditional-reuseaddr-after-liveness-probe",
        "socket2 API shape differs from the probe's 0.5",
    ),
    (
        "ecb-block-mode-selection",
        "aes/ecb padding API needs the block-padding feature",
    ),
    (
        "execution-artifact-trust-ladder-before-deserialize",
        "snippet: names an undefined Error conversion",
    ),
];

/// Build the probe crate + run clippy with the banned list enabled.
///
/// Every sec fixture is a module of ONE probe crate, so a single
/// non-compiling fixture (a snippet naming an undefined helper, or a crate
/// the probe does not declare) used to fail the WHOLE build — every
/// diagnostic vanished and the boundary test went vacuously green (Issue 249
/// B6). The probe now drops each fixture whose module carries a compile
/// error and re-runs until the build is clean; dropped fixtures are named,
/// [`MUST_COMPILE`] rows must survive, and the dropped set is pinned by
/// membership in [`EXPECTED_DROPPED`].
fn run_clippy() -> Option<Probe> {
    if !cargo_available() {
        eprintln!("SKIP: cargo not on PATH — the boundary gate needs the live toolchain");
        return None;
    }
    // Both gate tests run in ONE process (libtest, parallel by default) —
    // a pid-only temp name races them onto one crate dir, and the first
    // test's `remove_dir_all` lands under the second's clippy (Batch 328:
    // the zip dep tree stretched the probe build wide enough to make the
    // window a guaranteed failure). One discriminator per call.
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir: PathBuf =
        std::env::temp_dir().join(format!("sec_boundary_{}_{}", std::process::id(), seq));
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("mkdir");
    // Batch 88: the crypto fixtures import sha1/sha2/md-5/blake3/argon2/subtle —
    // the probe crate must declare them or the fixtures fail to COMPILE
    // (compile errors are not clippy diagnostics; the gate reads diagnostics).
    // Batch 258: the webhook fixture re-serializes with serde_json and the
    // egress fixture dials through ureq — same compile contract.
    // Batch 291: the CORS/JWT/cookie fixtures use tower-http (feature
    // `cors` — the crate ships NO default features), jsonwebtoken 9 (the
    // 9.x fixture's `insecure_disable_signature_validation` surface —
    // REMOVED at 10.0, so the pin is load-bearing), cookie 0.18, and
    // serde's derive feature (the Claims derive).
    // Batch 328: the archive-extract fixture names `zip::ZipArchive` —
    // the probe crate must declare `zip` or the fixture fails to
    // COMPILE (compile errors are not clippy diagnostics). zip 2.6.1
    // verified at authoring (by_index/name/enclosed_name are 2.x-era
    // API).
    // Batch 329: the mutation-gate fixture names `axum` (Body/State/
    // StatusCode/IntoResponse are core axum — no extra features) and
    // `tokio::fs` — the probe crate must declare both or the fixture
    // fails to COMPILE. axum 0.8 + tokio fs verified at authoring (the
    // identical handler compiled clean in the probe-parity run).
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname=\"sec_boundary_probe\"\nversion=\"0.0.0\"\nedition=\"2021\"\
         \n[workspace]\n[dependencies]\nsha1 = \"0.10\"\nsha2 = \"0.10\"\
         \nmd-5 = \"0.10\"\nblake3 = \"1\"\nargon2 = \"0.5\"\nsubtle = \"2\"\
         \nserde_json = \"1\"\nureq = \"2\"\nzip = \"2\"\nzeroize = \"1\"\
         \naxum = \"0.8\"\ntokio = { version = \"1\", features = [\"fs\"] }\
         \nserde = { version = \"1\", features = [\"derive\"] }\
         \ntower-http = { version = \"0.7\", features = [\"cors\"] }\
         \njsonwebtoken = \"9\"\ncookie = \"0.18\"\
         \nrand = \"0.8\"\nsocket2 = \"0.5\"\naes = \"0.8\"\necb = \"0.1\"\
         \ned25519-dalek = \"2\"\nfancy-regex = \"0.13\"\n",
    )
    .expect("write toml");
    let rules = SecDomain::build_corpus();
    for (i, rule) in rules.iter().enumerate() {
        std::fs::write(src.join(format!("r{i}.rs")), &rule.buggy_code).expect("write fixture");
    }
    let mut included: Vec<usize> = (0..rules.len()).collect();
    let mut dropped: Vec<(usize, String)> = Vec::new();
    // Each pass drops at least one fixture or ends; deps are built once.
    for _pass in 0..8 {
        let lib: String = included
            .iter()
            .map(|i| format!("#[path = \"r{i}.rs\"]\npub mod r{i};\n"))
            .collect();
        std::fs::write(src.join("lib.rs"), lib).expect("write lib");
        let (diags, errors) = clippy_pass(&dir);
        if errors.is_empty() {
            let _ = std::fs::remove_dir_all(&dir);
            for (i, why) in &dropped {
                eprintln!("dropped fixture r{i} ({}): {why}", rules[*i].rule_id);
            }
            return Some(Probe {
                diags,
                compiled: included,
                dropped,
            });
        }
        let mut progressed = false;
        for (file, first) in errors {
            let Some(idx) = fixture_index(&file) else {
                panic!("probe build error outside any fixture module ({file}): {first}");
            };
            if let Some(pos) = included.iter().position(|&i| i == idx) {
                included.remove(pos);
                dropped.push((idx, first));
                progressed = true;
            }
        }
        assert!(
            progressed,
            "probe build errors did not shrink the fixture set"
        );
    }
    panic!("probe did not reach a clean build in 8 passes");
}

/// The file an error belongs to: the first span (following macro
/// expansion chains) inside a fixture module, else the first span's file —
/// an error in a registry crate is attributed to the fixture that drove it.
fn error_fixture_file(msg: &serde_json::Value) -> String {
    fn walk(span: &serde_json::Value) -> Option<String> {
        let file = span.get("file_name").and_then(|f| f.as_str())?;
        if fixture_index(file).is_some() {
            return Some(file.to_string());
        }
        walk(span.get("expansion")?.get("span")?)
    }
    let spans = msg.get("spans").and_then(|sp| sp.as_array());
    spans
        .into_iter()
        .flatten()
        .find_map(walk)
        .or_else(|| {
            spans?
                .first()?
                .get("file_name")?
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_else(|| "?".to_string())
}

/// `src/rN.rs` (either separator) → `N`.
fn fixture_index(file: &str) -> Option<usize> {
    file.replace('\\', "/")
        .strip_prefix("src/r")?
        .strip_suffix(".rs")?
        .parse()
        .ok()
}

/// One `cargo clippy` over the probe: (lint diagnostics, per-file first
/// compile error). Error-level messages are collected, never dropped — a
/// silently dropped build failure is the vacuous-green class.
fn clippy_pass(dir: &std::path::Path) -> (Vec<Diag>, Vec<(String, String)>) {
    let mut cmd = Command::new("cargo");
    cmd.arg("clippy")
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .arg("--message-format=json");
    // Lint flags go AFTER `--` (cargo flags vs rustc/clippy flags).
    cmd.arg("--");
    for lint in BANNED {
        cmd.args(["-W", lint]);
    }
    let out = cmd.output().expect("run clippy");
    let stdout = String::from_utf8_lossy(&out.stdout);

    let mut diags = Vec::new();
    let mut errors: Vec<(String, String)> = Vec::new();
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }
        let Some(msg) = v.get("message") else {
            continue;
        };
        // A build failure surfaces as error-level compiler messages with NO
        // lint code — dropped silently below, which turns a broken probe
        // into a vacuous green (the gate's own green-zero blindness). Witness
        // the failure loud instead.
        if msg.get("level").and_then(|l| l.as_str()) == Some("error") {
            let file = error_fixture_file(msg);
            let first = msg
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("?")
                .to_string();
            if !errors.iter().any(|(f, _)| *f == file) {
                errors.push((file, first));
            }
            continue;
        }
        let Some(code) = msg
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(|c| c.as_str())
        else {
            continue;
        };
        let applicability = msg
            .get("children")
            .and_then(|ch| ch.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("spans"))
            .and_then(|sp| sp.as_array())
            .and_then(|arr| arr.first())
            .and_then(|s| s.get("suggestion_applicability"))
            .and_then(|a| a.as_str())
            .map(str::to_string);
        let file = msg
            .get("spans")
            .and_then(|sp| sp.as_array())
            .and_then(|arr| arr.first())
            .and_then(|s| s.get("file_name"))
            .and_then(|f| f.as_str())
            .unwrap_or("?")
            .to_string();
        diags.push(Diag {
            code: code.to_string(),
            applicability,
            file,
        });
    }
    (diags, errors)
}

/// The boundary holds: no banned lint fires with a MachineApplicable fix on
/// any sec fixture. `unwrap_used` firing WITHOUT a suggestion on the drop
/// fixture is the pinned, allowed overlap (our fix is additive there).
#[test]
fn no_banned_lint_fires_with_an_equivalent_fix() {
    let Some(probe) = run_clippy() else { return };
    let rules = SecDomain::build_corpus();
    let mut got: Vec<&str> = probe
        .dropped
        .iter()
        .map(|(i, _)| rules[*i].rule_id.as_str())
        .collect();
    got.sort_unstable();
    let mut want: Vec<&str> = EXPECTED_DROPPED.iter().map(|(r, _)| *r).collect();
    want.sort_unstable();
    assert_eq!(
        got, want,
        "the probe's non-compiling fixture set moved (pinned by membership in \
         EXPECTED_DROPPED) — classify a new row or remove a fixed one; \
         first errors: {:?}",
        probe.dropped
    );
    for name in MUST_COMPILE {
        let idx = rules
            .iter()
            .position(|r| r.rule_id == *name)
            .unwrap_or_else(|| panic!("{name} missing from the sec corpus"));
        assert!(
            probe.compiled.contains(&idx),
            "{name} no longer compiles in the probe — the pinned table cannot be measured"
        );
    }
    for d in &probe.diags {
        let is_banned = BANNED.iter().any(|b| {
            b.strip_prefix("clippy::") == Some(d.code.strip_prefix("clippy::").unwrap_or(&d.code))
                || *b == d.code
        });
        if !is_banned {
            continue;
        }
        let idx: usize = d
            .file
            .replace('\\', "/")
            .trim_start_matches("src/r")
            .trim_end_matches(".rs")
            .parse()
            .unwrap_or(usize::MAX);
        let rule = rules.get(idx).map_or("?", |r| r.rule_id.as_str());
        let applicability = d.applicability.as_deref().unwrap_or("none");
        let d_code = &d.code;
        assert_ne!(
            applicability, "MachineApplicable",
            "BOUNDARY BREAK: {d_code} fires MachineApplicable on {rule} — \n\n             clippy now owns that fix; the rule moves to the \
             do-not-reimplement side (remove it or narrow past the overlap)"
        );
        eprintln!(
            "allowed overlap: {d_code} fires on {rule} with no machine fix (additive boundary)",
            d_code = d.code
        );
    }
}

/// The pinned measured table: `unwrap_used` DOES fire on the drop fixture
/// (with no fix) — if clippy stops firing there, or grows a fix, this
/// documents the change loudly.
#[test]
fn unwrap_used_fires_without_fix_on_drop_fixture() {
    let Some(probe) = run_clippy() else { return };
    let diags = probe.diags;
    let rules = SecDomain::build_corpus();
    let drop_idx = rules
        .iter()
        .position(|r| r.rule_id == "drop-impl-no-panic")
        .expect("drop rule present");
    let file = format!("src/r{drop_idx}.rs");
    // Windows renders the path with backslashes (`src\r2.rs`) — normalize
    // before the byte comparison (the pinned file identity is `src/rN.rs`).
    let hits: Vec<&Diag> = diags
        .iter()
        .filter(|d| d.file.replace('\\', "/") == file && d.code == "clippy::unwrap_used")
        .collect();
    assert!(
        !hits.is_empty(),
        "unwrap_used no longer fires on the drop fixture — the pinned boundary table moved; \
         re-measure + update the table (and re-check the no-equivalent-fix gate)"
    );
    for h in hits {
        assert_ne!(
            h.applicability.as_deref(),
            Some("MachineApplicable"),
            "unwrap_used grew a machine fix — drop-impl-no-panic is now duplicative; retire it"
        );
    }
}
