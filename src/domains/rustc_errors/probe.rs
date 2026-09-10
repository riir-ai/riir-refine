//! The premise probe (Issue 035 T1) — ask the oracle whether rustc already
//! fixes a given error code, instead of asserting that it doesn't.
//!
//! ## Why this exists
//!
//! Plan 019 was filed claiming rustc emits NO machine-applicable suggestions
//! for the borrowck family (`E0597`/`E0596`/`E0502`/`E0499`/`E0505`/`E0507`/
//! `E0382`) and that `cargo fix` is "structurally blind" there. Measured on
//! rustc 1.93.0, that is false on 4 of the 7: `E0382`, `E0505`, `E0507` and
//! `E0596` all carry `MachineApplicable` suggestions that
//! `cargo fix --broken-code` applies unprompted.
//!
//! The failure mode is the same one the plan was written to prevent — a
//! confident narrative with no oracle in the loop. So the premise is not
//! prose here: it is a probe that runs, and (via `tests/premise_probe.rs`)
//! a test that fails when a toolchain bump moves a code across the
//! contested/uncontested line. **The contested set only ever grows** as rustc
//! improves, so this table has a shelf life and must be re-measured, never
//! cited.
//!
//! ## Method
//!
//! One isolated temp crate per case (never one crate with all cases — with
//! several broken functions in one file, `cargo fix`'s edits cannot be
//! attributed to a single code). For each:
//!
//! 1. `cargo check --message-format=json` → parse via [`super::diag`],
//!    recording suggestion spans + applicability.
//! 2. `cargo fix --broken-code --allow-no-vcs --allow-dirty` on a pristine
//!    copy → diff the source to see what, if anything, rustc changed on its
//!    own.
//! 3. `cargo check` again → which codes survive rustc's own best effort.
//!
//! `--broken-code` is required (plain `cargo fix` refuses a non-compiling
//! tree) and is available on stable — no nightly, no `-Z`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::diag::{parse_error_diags, RustcErrorDiag};

/// One minimal reproduction, declared with the code we EXPECT rustc to emit.
///
/// The expectation is load-bearing, not documentation: the `Box<dyn Fn>`
/// closure shape that Plan 019 filed as an E0597 template is actually
/// diagnosed **E0373**, and rustc auto-fixes it. A fixture that only asserted
/// "some error happened" would have hidden that.
#[derive(Debug, Clone, Copy)]
pub struct ProbeCase {
    /// Short slug — also the temp-crate name (must be a valid crate ident).
    pub name: &'static str,
    /// The code rustc is expected to emit for this source.
    pub expect_code: &'static str,
    pub source: &'static str,
    /// What the plan claims about this code, asserted by the T1 test.
    pub expect_machine_applicable: bool,
}

/// Result of probing one case against the live toolchain.
#[derive(Debug, Clone)]
pub struct ProbeOutcome {
    pub name: &'static str,
    pub expect_code: &'static str,
    /// Every error code `cargo check` emitted, sorted + deduped.
    pub observed_codes: Vec<String>,
    /// Suggestion spans on diagnostics carrying `expect_code`.
    pub suggestion_spans: usize,
    /// Of those, how many `cargo fix` would apply.
    pub machine_applicable: usize,
    /// The replacement texts rustc offered (for the bench table).
    pub replacements: Vec<String>,
    /// Did `cargo fix --broken-code` change the source at all?
    pub cargo_fix_changed: bool,
    /// The post-`cargo fix` source, when it changed.
    pub fixed_source: Option<String>,
    /// Codes still failing after `cargo fix` had its turn.
    pub codes_after_fix: Vec<String>,
}

impl ProbeOutcome {
    /// Uncontested = rustc offers no auto-fix AND `cargo fix` changed nothing.
    ///
    /// Both halves matter: a `MaybeIncorrect` suggestion shows up in the JSON
    /// but is never applied, and (the reverse trap) a code can be auto-fixed
    /// via a suggestion attached to a DIFFERENT code's diagnostic.
    #[must_use]
    pub fn is_uncontested(&self) -> bool {
        self.machine_applicable == 0 && !self.cargo_fix_changed
    }

    /// Did rustc emit the code the fixture claims? A `false` here means the
    /// fixture is mis-labeled — the E0597-vs-E0373 trap.
    #[must_use]
    pub fn code_as_declared(&self) -> bool {
        self.observed_codes.iter().any(|c| c == self.expect_code)
    }
}

/// The eight cases: the seven borrowck codes Plan 019 named, plus `E0373`
/// (the closure-outlives shape the plan mislabeled as E0597).
///
/// Each source is the smallest thing that provokes exactly one code.
pub const PROBE_CASES: &[ProbeCase] = &[
    ProbeCase {
        name: "e0597_local_lifetime",
        expect_code: "E0597",
        // The true E0597: a reference outliving the scope of its owner.
        // NOT the closure shape — that one is E0373 (see below).
        source: "pub fn probe() {\n    let r;\n    {\n        let x = 5;\n        r = &x;\n    }\n    println!(\"{}\", r);\n}\n",
        expect_machine_applicable: false,
    },
    ProbeCase {
        name: "e0499_double_mut",
        expect_code: "E0499",
        source: "pub fn probe(v: &mut Vec<u8>) {\n    let a = &mut *v;\n    let b = &mut *v;\n    a.push(1);\n    b.push(2);\n}\n",
        expect_machine_applicable: false,
    },
    ProbeCase {
        name: "e0502_borrow_then_mut",
        expect_code: "E0502",
        source: "pub fn probe(v: &mut Vec<u8>) -> usize {\n    let first = &v[0];\n    v.push(1);\n    *first as usize\n}\n",
        expect_machine_applicable: false,
    },
    ProbeCase {
        name: "e0382_use_after_move",
        expect_code: "E0382",
        source: "pub fn probe(v: Vec<u8>) -> usize {\n    let w = v;\n    v.len() + w.len()\n}\n",
        expect_machine_applicable: true,
    },
    ProbeCase {
        name: "e0505_move_out_of_borrowed",
        expect_code: "E0505",
        source: "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v;\n    r.len() + w.len()\n}\n",
        expect_machine_applicable: true,
    },
    ProbeCase {
        name: "e0507_move_out_of_ref",
        expect_code: "E0507",
        source: "pub struct S {\n    pub v: Vec<u8>,\n}\npub fn probe(s: &S) -> Vec<u8> {\n    s.v\n}\n",
        expect_machine_applicable: true,
    },
    ProbeCase {
        name: "e0596_borrow_as_mut",
        expect_code: "E0596",
        source: "pub fn probe(v: Vec<u8>) {\n    let x = v;\n    x.push(1);\n}\n",
        expect_machine_applicable: true,
    },
    ProbeCase {
        name: "e0373_closure_outlives",
        expect_code: "E0373",
        // Plan 019 filed this shape as "E0597 template 1 (`move` closure)".
        // rustc calls it E0373 and fixes it itself.
        source: "pub fn probe() -> Box<dyn Fn() -> usize> {\n    let s = String::from(\"hello\");\n    Box::new(|| s.len())\n}\n",
        expect_machine_applicable: true,
    },
];

/// Runs [`PROBE_CASES`] against the live toolchain.
pub struct PremiseProbe {
    cargo_bin: String,
    work_root: PathBuf,
}

impl Default for PremiseProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl PremiseProbe {
    /// Work root is keyed by pid AND a per-instance counter.
    ///
    /// The pid alone is NOT enough: `cargo test` runs test fns as parallel
    /// THREADS of one process, so two probes would share a directory and
    /// `scaffold`'s clean-slate `remove_dir_all` would delete a sibling's
    /// crate out from under it. That failure surfaces as a bogus zero
    /// measurement ("no suggestions found"), i.e. as a false product claim —
    /// so this counter is a correctness requirement, not tidiness.
    #[must_use]
    pub fn new() -> Self {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        Self {
            cargo_bin: std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
            work_root: std::env::temp_dir()
                .join(format!("riir_premise_probe_{}_{seq}", std::process::id())),
        }
    }

    #[must_use]
    pub fn with_work_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.work_root = root.into();
        self
    }

    /// Is cargo on PATH? Every probe is skipped (not failed) without it.
    #[must_use]
    pub fn is_available(&self) -> bool {
        Command::new(&self.cargo_bin)
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Probe every case. Errors are per-case so one bad fixture cannot mask
    /// the rest.
    #[must_use]
    pub fn probe_all(&self) -> Vec<Result<ProbeOutcome, String>> {
        PROBE_CASES.iter().map(|c| self.probe(c)).collect()
    }

    /// Probe one case: check → parse → `cargo fix` on a pristine copy →
    /// re-check.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the temp crate cannot be written or cargo cannot be
    /// spawned. A case that simply produces no diagnostics is a valid
    /// measurement, not an error.
    pub fn probe(&self, case: &ProbeCase) -> Result<ProbeOutcome, String> {
        // Two separate crates: `check` must see the ORIGINAL source, so it
        // cannot share a directory with the one `cargo fix` rewrites.
        let check_dir = self.scaffold(&format!("{}_check", case.name), case.source)?;
        let fix_dir = self.scaffold(&format!("{}_fix", case.name), case.source)?;

        let check_json = self.cargo_json(&check_dir, &["check", "--message-format=json"])?;
        let diags = parse_error_diags(&check_json);

        let mut observed_codes: Vec<String> = diags.iter().map(|d| d.code.clone()).collect();
        observed_codes.sort();
        observed_codes.dedup();

        let for_code: Vec<&RustcErrorDiag> = diags
            .iter()
            .filter(|d| d.code == case.expect_code)
            .collect();
        let suggestion_spans: usize = for_code.iter().map(|d| d.suggestions.len()).sum();
        let machine_applicable: usize = for_code.iter().map(|d| d.machine_applicable().len()).sum();
        let replacements: Vec<String> = for_code
            .iter()
            .flat_map(|d| d.machine_applicable())
            .map(|s| s.replacement.clone())
            .collect();

        // `cargo fix` writes in place; compare against the fixture text.
        let _ = self.cargo_json(
            &fix_dir,
            &["fix", "--broken-code", "--allow-no-vcs", "--allow-dirty"],
        );
        let lib_path = fix_dir.join("src").join("lib.rs");
        let after = std::fs::read_to_string(&lib_path)
            .map_err(|e| format!("read back {}: {e}", lib_path.display()))?;
        let cargo_fix_changed = after != case.source;

        let recheck_json = self.cargo_json(&fix_dir, &["check", "--message-format=json"])?;
        let mut codes_after_fix: Vec<String> = parse_error_diags(&recheck_json)
            .iter()
            .map(|d| d.code.clone())
            .collect();
        codes_after_fix.sort();
        codes_after_fix.dedup();

        Ok(ProbeOutcome {
            name: case.name,
            expect_code: case.expect_code,
            observed_codes,
            suggestion_spans,
            machine_applicable,
            replacements,
            cargo_fix_changed,
            fixed_source: cargo_fix_changed.then_some(after),
            codes_after_fix,
        })
    }

    /// Write a standalone crate (own `[workspace]`, no deps) under the work root.
    fn scaffold(&self, name: &str, source: &str) -> Result<PathBuf, String> {
        let dir = self.work_root.join(name);
        let src = dir.join("src");
        // Stale state from an earlier run would make `cargo fix` look like a
        // no-op (the file is already fixed). Start clean every time.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&src).map_err(|e| format!("create {}: {e}", src.display()))?;
        let manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n"
        );
        std::fs::write(dir.join("Cargo.toml"), manifest)
            .map_err(|e| format!("write manifest: {e}"))?;
        std::fs::write(src.join("lib.rs"), source).map_err(|e| format!("write lib.rs: {e}"))?;
        Ok(dir)
    }

    /// Run cargo in `dir`, returning stdout.
    ///
    /// Env hygiene matters more than it looks: a nested cargo inherits
    /// `CARGO_TARGET_DIR` and `RUSTFLAGS` from whatever invoked the test. An
    /// inherited target dir puts every probe crate in one directory (lock
    /// contention + cross-case cache bleed), and inherited `RUSTFLAGS` can
    /// change which diagnostics fire — which would corrupt the very
    /// measurement this module exists to make.
    fn cargo_json(&self, dir: &Path, args: &[&str]) -> Result<String, String> {
        let out = Command::new(&self.cargo_bin)
            .args(args)
            .current_dir(dir)
            .env("CARGO_TARGET_DIR", dir.join("target"))
            .env_remove("RUSTFLAGS")
            .env_remove("RUSTC_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTFLAGS")
            .output()
            .map_err(|e| format!("cargo {args:?} spawn failed: {e}"))?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// Best-effort cleanup of the work root.
    pub fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.work_root);
    }
}
