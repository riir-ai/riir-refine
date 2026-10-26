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

/// Build the probe crate + run clippy with the banned list enabled.
fn run_clippy() -> Option<Vec<Diag>> {
    if !cargo_available() {
        eprintln!("SKIP: cargo not on PATH — the boundary gate needs the live toolchain");
        return None;
    }
    let dir: PathBuf = std::env::temp_dir().join(format!("sec_boundary_{}", std::process::id()));
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
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname=\"sec_boundary_probe\"\nversion=\"0.0.0\"\nedition=\"2021\"\
         \n[workspace]\n[dependencies]\nsha1 = \"0.10\"\nsha2 = \"0.10\"\
         \nmd-5 = \"0.10\"\nblake3 = \"1\"\nargon2 = \"0.5\"\nsubtle = \"2\"\
         \nserde_json = \"1\"\nureq = \"2\"\
         \nserde = { version = \"1\", features = [\"derive\"] }\
         \ntower-http = { version = \"0.7\", features = [\"cors\"] }\
         \njsonwebtoken = \"9\"\ncookie = \"0.18\"\n",
    )
    .expect("write toml");
    let mut lib = String::new();
    for (i, rule) in SecDomain::build_corpus().iter().enumerate() {
        lib.push_str(&format!("#[path = \"r{i}.rs\"]\npub mod r{i};\n"));
        std::fs::write(src.join(format!("r{i}.rs")), &rule.buggy_code).expect("write fixture");
    }
    std::fs::write(src.join("lib.rs"), lib).expect("write lib");

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
    let _ = std::fs::remove_dir_all(&dir);
    Some(diags)
}

/// The boundary holds: no banned lint fires with a MachineApplicable fix on
/// any sec fixture. `unwrap_used` firing WITHOUT a suggestion on the drop
/// fixture is the pinned, allowed overlap (our fix is additive there).
#[test]
fn no_banned_lint_fires_with_an_equivalent_fix() {
    let Some(diags) = run_clippy() else { return };
    let rules = SecDomain::build_corpus();
    for d in &diags {
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
            applicability,
            "MachineApplicable",
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
    let Some(diags) = run_clippy() else { return };
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
