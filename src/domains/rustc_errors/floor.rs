//! The G-floor harness (Issue 035 T2; Plan 019 T6.9) — measure any compile
//! fixer against `cargo fix --broken-code` as the baseline, per error code.
//!
//! ## Why a floor at all
//!
//! This repo already refuses to promote a UQ primitive that cannot beat the
//! conformal-naive floor. The same discipline applies here for the same
//! reason: `cargo fix --broken-code` is free, ships with the toolchain, and
//! (measured — see [`super::probe`]) already repairs 5 of the 8 error codes
//! Plan 019 set out to own. A compile fixer that does not beat it on a code
//! has no business shipping a template for that code.
//!
//! ## The corpus: mutation with a known inverse
//!
//! Each case is a **compiling seed** plus a declared source rewrite that
//! breaks it. The rewrite's inverse is therefore known exactly — it is the
//! seed. That gives every mutant a ground-truth fix for free, with no hand
//! labeling (the Issue 017 self-labeled-corpus shape).
//!
//! The rewrites are declared `from`/`to` string pairs, not random AST fuzz.
//! Stated plainly because it bounds the claim: this corpus measures the
//! mutation classes we chose, and coverage numbers computed over it are
//! numbers about those classes, not about all Rust.
//!
//! ## The metric that matters
//!
//! Fix rate alone is the wrong number, because a fixer can repair a mutant
//! with a diff that means something different. So every repair is scored
//! against the known inverse:
//!
//! - **matched** — byte-identical to the seed (modulo whitespace): the fix
//!   restored the original meaning.
//! - **diverged** — compiles, but is not the seed. rustc's own `.clone()`
//!   suggestions land here, and this is where semantics quietly change.
//! - **unfixed** — still does not compile.
//!
//! "Diverged" is not automatically wrong; it is automatically *worth
//! review*, which is exactly the distinction the revised Plan 019 thesis
//! rests on.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use super::corpus_gen::GeneratedCase;
use super::diag::parse_error_diags;
use super::driver::FixCompileReport;

/// What kind of source rewrite broke the seed. Reporting label — the actual
/// edit is the [`Mutator`]'s `from`/`to` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutatorKind {
    /// `let mut x` → `let x`.
    DropMutBinding,
    /// Remove a `.clone()` call.
    DropCloneCall,
    /// `move ||` → `||`.
    DropMoveKeyword,
    /// Bind by reference where the seed bound by value.
    AddBorrow,
    /// Remove a scope that was keeping a borrow short.
    FlattenScope,
    /// Move a binding's declaration into an inner scope.
    NarrowScope,
}

impl MutatorKind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::DropMutBinding => "drop-mut-binding",
            Self::DropCloneCall => "drop-clone-call",
            Self::DropMoveKeyword => "drop-move-keyword",
            Self::AddBorrow => "add-borrow",
            Self::FlattenScope => "flatten-scope",
            Self::NarrowScope => "narrow-scope",
        }
    }
}

/// A declared, deterministic source rewrite that breaks compilation.
#[derive(Debug, Clone, Copy)]
pub struct Mutator {
    pub kind: MutatorKind,
    /// Exact substring to replace (first occurrence only — must be unique
    /// enough in the seed to be unambiguous).
    pub from: &'static str,
    pub to: &'static str,
}

/// Apply a compile-breaking mutation to a compiling seed.
///
/// Errors when `from` is absent — a silent no-op would produce a "mutant"
/// that compiles, and the harness would score the fixer against a case with
/// nothing to fix. Corpus integrity is checked, not assumed.
///
/// The name mirrors Plan 019 T6.1. It is a NEW function, not an extension of
/// `mutate_shape` (`src/draft/retrieval_eval.rs`) — that one is a token
/// renamer for retrieval-invariance probes and never breaks compilation. The
/// plan's original "extend `mutate_shape`" wording was a mis-citation.
///
/// # Errors
///
/// Returns `Err` when `mutator.from` does not occur in `seed`.
pub fn mutate_compile_break(seed: &str, mutator: &Mutator) -> Result<String, String> {
    match seed.find(mutator.from) {
        None => Err(format!(
            "mutator {} site {:?} not found in seed",
            mutator.kind.label(),
            mutator.from
        )),
        Some(at) => {
            let mut out = String::with_capacity(seed.len() + mutator.to.len());
            out.push_str(&seed[..at]);
            out.push_str(mutator.to);
            out.push_str(&seed[at + mutator.from.len()..]);
            Ok(out)
        }
    }
}

/// One corpus entry: a compiling seed + the mutation that breaks it.
#[derive(Debug, Clone, Copy)]
pub struct MutantCase {
    pub name: &'static str,
    /// The code the mutant is expected to provoke (verified at run time).
    pub expect_code: &'static str,
    pub mutator: Mutator,
    /// Compiling source. Also the known-inverse fix.
    pub seed: &'static str,
}

impl MutantCase {
    /// The broken source.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the mutation site is absent from the seed.
    pub fn broken(&self) -> Result<String, String> {
        mutate_compile_break(self.seed, &self.mutator)
    }
}

/// The committed corpus. Stratified by territory so a coverage number cannot
/// be inflated with codes `cargo fix` already owns.
pub const MUTANT_CASES: &[MutantCase] = &[
    // ── uncontested territory (probe: no machine-applicable suggestion) ──
    MutantCase {
        name: "e0597_narrow_owner_scope",
        expect_code: "E0597",
        mutator: Mutator {
            kind: MutatorKind::NarrowScope,
            from: "let x = 5i32;\n    let r = &x;",
            to: "let r;\n    {\n        let x = 5i32;\n        r = &x;\n    }",
        },
        seed: "pub fn probe() -> i32 {\n    let x = 5i32;\n    let r = &x;\n    *r\n}\n",
    },
    MutantCase {
        name: "e0597_ref_escapes_block",
        expect_code: "E0597",
        mutator: Mutator {
            kind: MutatorKind::NarrowScope,
            // The borrow itself must outlive the owner. An earlier version of
            // this case narrowed the scope around a `usize` length instead —
            // `usize` is `Copy`, nothing was borrowed, and the "mutant"
            // compiled. The null-arm self-check caught it.
            from: "let owned = String::from(\"hello\");\n    let r: &str = &owned;",
            to: "let r: &str;\n    {\n        let owned = String::from(\"hello\");\n        r = &owned;\n    }",
        },
        seed: "pub fn probe() -> usize {\n    let owned = String::from(\"hello\");\n    let r: &str = &owned;\n    r.len()\n}\n",
    },
    MutantCase {
        name: "e0499_flatten_borrow_scope",
        expect_code: "E0499",
        mutator: Mutator {
            kind: MutatorKind::FlattenScope,
            from: "    {\n        let a = &mut *v;\n        a.push(1);\n    }\n    let b = &mut *v;\n    b.push(2);",
            to: "    let a = &mut *v;\n    let b = &mut *v;\n    a.push(1);\n    b.push(2);",
        },
        seed: "pub fn probe(v: &mut Vec<u8>) {\n    {\n        let a = &mut *v;\n        a.push(1);\n    }\n    let b = &mut *v;\n    b.push(2);\n}\n",
    },
    MutantCase {
        name: "e0502_borrow_across_mutation",
        expect_code: "E0502",
        mutator: Mutator {
            kind: MutatorKind::AddBorrow,
            // Deref at the use site too. Borrowing without it made the
            // mutant fail with E0606 (`&u8 as usize` is not a valid cast),
            // so the case measured a cast error dressed up as a borrow error
            // — caught by the declared-code integrity check.
            from: "let first = v[0];\n    v.push(1);\n    first",
            to: "let first = &v[0];\n    v.push(1);\n    *first",
        },
        seed: "pub fn probe(v: &mut Vec<u8>) -> u8 {\n    let first = v[0];\n    v.push(1);\n    first\n}\n",
    },
    MutantCase {
        name: "e0502_len_borrow_across_push",
        expect_code: "E0502",
        mutator: Mutator {
            kind: MutatorKind::AddBorrow,
            from: "let head = items[0].len();\n    items.push(String::new());\n    head",
            to: "let head = &items[0];\n    items.push(String::new());\n    head.len()",
        },
        seed: "pub fn probe(items: &mut Vec<String>) -> usize {\n    let head = items[0].len();\n    items.push(String::new());\n    head\n}\n",
    },
    // ── contested territory (probe: rustc ships a MachineApplicable fix) ──
    MutantCase {
        name: "e0596_drop_mut",
        expect_code: "E0596",
        mutator: Mutator {
            kind: MutatorKind::DropMutBinding,
            from: "let mut x = v;",
            to: "let x = v;",
        },
        seed: "pub fn probe(v: Vec<u8>) -> usize {\n    let mut x = v;\n    x.push(1);\n    x.len()\n}\n",
    },
    MutantCase {
        name: "e0596_drop_mut_param_shadow",
        expect_code: "E0596",
        mutator: Mutator {
            kind: MutatorKind::DropMutBinding,
            from: "let mut acc = Vec::new();",
            to: "let acc = Vec::new();",
        },
        seed: "pub fn probe(n: u8) -> Vec<u8> {\n    let mut acc = Vec::new();\n    acc.push(n);\n    acc\n}\n",
    },
    MutantCase {
        name: "e0382_drop_clone",
        expect_code: "E0382",
        mutator: Mutator {
            kind: MutatorKind::DropCloneCall,
            from: "let w = v.clone();",
            to: "let w = v;",
        },
        seed: "pub fn probe(v: Vec<u8>) -> usize {\n    let w = v.clone();\n    v.len() + w.len()\n}\n",
    },
    MutantCase {
        name: "e0505_drop_clone_under_borrow",
        expect_code: "E0505",
        mutator: Mutator {
            kind: MutatorKind::DropCloneCall,
            from: "let w = v.clone();",
            to: "let w = v;",
        },
        seed: "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v.clone();\n    r.len() + w.len()\n}\n",
    },
    MutantCase {
        name: "e0507_drop_clone_out_of_ref",
        expect_code: "E0507",
        mutator: Mutator {
            kind: MutatorKind::DropCloneCall,
            from: "s.v.clone()",
            to: "s.v",
        },
        seed: "pub struct S {\n    pub v: Vec<u8>,\n}\npub fn probe(s: &S) -> Vec<u8> {\n    s.v.clone()\n}\n",
    },
    MutantCase {
        name: "e0373_drop_move",
        expect_code: "E0373",
        mutator: Mutator {
            kind: MutatorKind::DropMoveKeyword,
            from: "Box::new(move || s.len())",
            to: "Box::new(|| s.len())",
        },
        seed: "pub fn probe() -> Box<dyn Fn() -> usize> {\n    let s = String::from(\"hello\");\n    Box::new(move || s.len())\n}\n",
    },
    MutantCase {
        name: "e0373_drop_move_spawn_shape",
        expect_code: "E0373",
        mutator: Mutator {
            kind: MutatorKind::DropMoveKeyword,
            from: "Box::new(move || data.iter().sum())",
            to: "Box::new(|| data.iter().sum())",
        },
        seed: "pub fn probe() -> Box<dyn Fn() -> u32> {\n    let data = vec![1u32, 2, 3];\n    Box::new(move || data.iter().sum())\n}\n",
    },
];

/// A fixer under test. The floor arm is [`CargoFixBaseline`]; the
/// `--fix-compile` driver (Plan 019 Phase 1) plugs in here when it lands.
pub trait CompileFixer {
    /// Arm name for the report.
    fn name(&self) -> &'static str;

    /// Repair the crate rooted at `dir` in place. `Ok` means "the arm ran",
    /// NOT "the code compiles" — the harness re-checks and decides.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the arm could not run at all (e.g. cargo failed to
    /// spawn). A crate left non-compiling is NOT an error — that is a
    /// [`FixVerdict::Unfixed`] result.
    fn fix(&self, dir: &Path) -> Result<(), String>;
}

/// The floor: `cargo fix --broken-code`, the toolchain's own repair pass.
pub struct CargoFixBaseline {
    cargo_bin: String,
}

impl Default for CargoFixBaseline {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoFixBaseline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cargo_bin: std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
        }
    }
}

impl CompileFixer for CargoFixBaseline {
    fn name(&self) -> &'static str {
        "cargo fix --broken-code"
    }

    fn fix(&self, dir: &Path) -> Result<(), String> {
        run_cargo(
            &self.cargo_bin,
            dir,
            &["fix", "--broken-code", "--allow-no-vcs", "--allow-dirty"],
        )
        .map(|_| ())
    }
}

/// The `--fix-compile` arm BEFORE Plan 019 Phase 1 lands: touches nothing.
///
/// Not filler — it pins the report shape and makes the harness's own
/// scoring falsifiable. An arm that repairs nothing must score 0% fixed; if
/// this arm ever scores above zero, the harness is measuring the mutation,
/// not the fixer.
pub struct NullFixer;

impl CompileFixer for NullFixer {
    fn name(&self) -> &'static str {
        "no-op (rustc_errors driver not yet implemented)"
    }

    fn fix(&self, _dir: &Path) -> Result<(), String> {
        Ok(())
    }
}

/// The `--fix-compile` arm (Plan 019 T6.9's pending measurement): the
/// production driver + production playbook registry over the scaffold
/// crate's `src/lib.rs`. The one-line adapter the bench predicted — the
/// [`CompileFixer`] seam was built for this.
///
/// `Ok` means the driver RAN (its own report carries the detail); the
/// harness re-checks and scores, same as every arm.
#[derive(Default)]
pub struct DriverFixer {
    ordering: Option<super::driver::OrderingFn>,
}

impl DriverFixer {
    /// The production arm: static min-delta ordering, default max-iters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a candidate-ordering override (the T4.2 seam — the G6/G7
    /// A/B arms use this to compare static vs flat vs shape-conditional
    /// ordering over the same corpus).
    #[must_use]
    pub fn with_ordering(mut self, f: super::driver::OrderingFn) -> Self {
        self.ordering = Some(f);
        self
    }

    /// Run the driver and RETURN its report (the verbose path the G2
    /// latency + G6/G7 A/B measurements consume — per-iteration timings,
    /// candidate try-order evidence, misses).
    ///
    /// # Errors
    ///
    /// No `src/lib.rs` in the scaffold dir.
    pub fn fix_returning_report(&self, dir: &Path) -> Result<FixCompileReport, String> {
        let lib = dir.join("src").join("lib.rs");
        if !lib.is_file() {
            return Err(format!("no src/lib.rs in {}", dir.display()));
        }
        let mut driver = super::driver::CompileDriver::isolated();
        if let Some(f) = &self.ordering {
            driver = driver.with_ordering(f.clone());
        }
        Ok(driver.run(&[lib], &super::playbooks::Playbooks::production(), true))
    }
}

impl CompileFixer for DriverFixer {
    fn name(&self) -> &'static str {
        "cargo heal --fix-compile"
    }

    fn fix(&self, dir: &Path) -> Result<(), String> {
        self.fix_returning_report(dir).map(|_| ())
    }
}

/// How a repair compares to the mutation's known inverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixVerdict {
    /// Compiles and is the seed — original meaning restored.
    MatchedInverse,
    /// Compiles but differs from the seed — review required.
    Diverged,
    /// Still does not compile.
    Unfixed,
}

/// One arm's result on one mutant.
#[derive(Debug, Clone)]
pub struct FloorOutcome {
    pub name: String,
    pub expect_code: &'static str,
    pub mutator: MutatorKind,
    /// Codes the mutant provoked BEFORE any repair.
    pub codes_before: Vec<String>,
    /// Corpus integrity: did the mutant actually fail with the declared code?
    pub mutant_valid: bool,
    pub verdict: FixVerdict,
    /// The post-fix source when it compiles but diverged (the review artifact).
    pub diverged_source: Option<String>,
    /// Wall time of the fixer call, milliseconds (the G2 latency axis;
    /// per-iteration driver timings live in the verbose report).
    pub fix_ms: f64,
}

/// A corpus case in the borrowed form the harness runs — the committed
/// anchors ([`MUTANT_CASES`]) and the generated corpus
/// ([`super::corpus_gen`]) unify here (Plan 019 T6.1).
pub struct HarnessCase<'a> {
    pub name: &'a str,
    pub expect_code: &'static str,
    pub kind: MutatorKind,
    pub seed: &'a str,
    pub broken: String,
}

impl<'a> HarnessCase<'a> {
    /// Adapt a committed anchor. Errors when the declared mutation site is
    /// absent from the seed (corpus integrity — never a silent no-op).
    ///
    /// # Errors
    ///
    /// `MutantCase::broken` failure (mutation site missing).
    pub fn committed(c: &'a MutantCase) -> Result<Self, String> {
        Ok(Self {
            name: c.name,
            expect_code: c.expect_code,
            kind: c.mutator.kind,
            seed: c.seed,
            broken: c.broken()?,
        })
    }

    /// Adapt a generated case (the broken half is precomputed).
    #[must_use]
    pub fn generated(g: &'a GeneratedCase) -> Self {
        Self {
            name: &g.name,
            expect_code: g.expect_code,
            kind: g.kind,
            seed: &g.seed,
            broken: g.broken.clone(),
        }
    }
}

/// Per-code aggregation — the published table shape.
#[derive(Debug, Clone)]
pub struct FloorRow {
    pub code: String,
    pub mutants: usize,
    pub matched: usize,
    pub diverged: usize,
    pub unfixed: usize,
}

impl FloorRow {
    /// Fix rate: anything that compiles afterwards, matched or not.
    ///
    /// `.max(1)` guards the empty row without a branch — with a zero
    /// numerator the result is 0.0 either way.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // mutant counts are ~10^1
    pub fn fix_rate(&self) -> f64 {
        (self.matched + self.diverged) as f64 / self.mutants.max(1) as f64
    }

    /// Of the repairs that landed, the share that changed meaning.
    ///
    /// Denominator is the FIXED subset, not all mutants: "how often does a
    /// repair change the code's meaning" is a different question from "how
    /// often does a repair happen".
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // mutant counts are ~10^1
    pub fn divergence_rate(&self) -> f64 {
        self.diverged as f64 / (self.matched + self.diverged).max(1) as f64
    }
}

/// A full arm run.
#[derive(Debug, Clone)]
pub struct FloorReport {
    pub arm: String,
    pub outcomes: Vec<FloorOutcome>,
    pub rows: Vec<FloorRow>,
    /// Mutants that failed corpus integrity (compiled, or wrong code).
    pub invalid_mutants: Vec<String>,
}

impl FloorReport {
    #[must_use]
    pub fn row(&self, code: &str) -> Option<&FloorRow> {
        self.rows.iter().find(|r| r.code == code)
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)] // mutant counts are ~10^1
    pub fn total_fix_rate(&self) -> f64 {
        let fixed = self
            .outcomes
            .iter()
            .filter(|o| o.verdict != FixVerdict::Unfixed)
            .count();
        fixed as f64 / self.outcomes.len().max(1) as f64
    }
}

/// Runs [`MUTANT_CASES`] through a [`CompileFixer`] and scores each repair
/// against the known inverse.
pub struct FloorHarness {
    cargo_bin: String,
    work_root: PathBuf,
}

impl Default for FloorHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl FloorHarness {
    /// Keyed by pid AND a per-instance counter — two harnesses in one process
    /// (parallel test threads, or the baseline + null arms in one run) must
    /// not share a work root, or one arm's clean-slate scaffold deletes the
    /// other's crate and the measurement silently reports zeros.
    #[must_use]
    pub fn new() -> Self {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        Self {
            cargo_bin: std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
            work_root: std::env::temp_dir()
                .join(format!("riir_floor_{}_{seq}", std::process::id())),
        }
    }

    #[must_use]
    pub fn with_work_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.work_root = root.into();
        self
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        Command::new(&self.cargo_bin)
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Measure one arm over the committed anchor corpus (the original
    /// Issue 035 surface — unchanged behavior).
    ///
    /// # Errors
    ///
    /// Returns `Err` on corpus or I/O failure (mutation site missing, temp
    /// crate unwritable, cargo unspawnable) — conditions under which no
    /// measurement is possible. A fixer that simply fails to fix things is a
    /// low score, not an error.
    pub fn measure(&self, fixer: &dyn CompileFixer) -> Result<FloorReport, String> {
        let cases: Vec<HarnessCase<'_>> = MUTANT_CASES
            .iter()
            .map(HarnessCase::committed)
            .collect::<Result<_, _>>()?;
        self.measure_cases(fixer, &cases)
    }

    /// Measure one arm over ANY case list (the generated corpus adapter —
    /// Plan 019 T6.1). Same error semantics as [`FloorHarness::measure`].
    ///
    /// # Errors
    ///
    /// I/O or cargo-spawn failure mid-corpus.
    pub fn measure_cases(
        &self,
        fixer: &dyn CompileFixer,
        cases: &[HarnessCase<'_>],
    ) -> Result<FloorReport, String> {
        let mut outcomes = Vec::with_capacity(cases.len());
        let mut invalid_mutants = Vec::new();
        for case in cases {
            let outcome = self.measure_one_by(case, |dir| fixer.fix(dir))?;
            if !outcome.mutant_valid {
                invalid_mutants.push(format!(
                    "{} (expected {}, got {:?})",
                    case.name, case.expect_code, outcome.codes_before
                ));
            }
            outcomes.push(outcome);
        }
        let rows = aggregate(&outcomes);
        Ok(FloorReport {
            arm: fixer.name().to_string(),
            outcomes,
            rows,
            invalid_mutants,
        })
    }

    /// The verbose driver arm (G2/G6/G7): runs the same
    /// scaffold→before-check→fix→after-check flow but keeps each run's
    /// [`FixCompileReport`] (per-iteration timings, candidate try evidence,
    /// misses) alongside the scored outcome.
    ///
    /// # Errors
    ///
    /// I/O or cargo-spawn failure mid-corpus.
    pub fn measure_driver(
        &self,
        fixer: &DriverFixer,
        cases: &[HarnessCase<'_>],
    ) -> Result<(FloorReport, Vec<(String, FixCompileReport)>), String> {
        let mut outcomes = Vec::with_capacity(cases.len());
        let mut reports = Vec::with_capacity(cases.len());
        let mut invalid_mutants = Vec::new();
        for case in cases {
            let slot: std::cell::RefCell<Option<FixCompileReport>> = std::cell::RefCell::new(None);
            let outcome = self.measure_one_by(case, |dir| {
                let r = fixer.fix_returning_report(dir)?;
                *slot.borrow_mut() = Some(r);
                Ok(())
            })?;
            if !outcome.mutant_valid {
                invalid_mutants.push(format!(
                    "{} (expected {}, got {:?})",
                    case.name, case.expect_code, outcome.codes_before
                ));
            }
            let rep = slot.borrow_mut().take().unwrap_or_default();
            reports.push((case.name.to_string(), rep));
            outcomes.push(outcome);
        }
        let rows = aggregate(&outcomes);
        let report = FloorReport {
            arm: fixer.name().to_string(),
            outcomes,
            rows,
            invalid_mutants,
        };
        Ok((report, reports))
    }

    /// The work root (scaffold dirs live at `<root>/<case-name>/` — the
    /// post-fix `cargo test` semantics pass walks them).
    #[must_use]
    pub fn work_root(&self) -> &Path {
        &self.work_root
    }

    fn measure_one_by(
        &self,
        case: &HarnessCase<'_>,
        fix: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<FloorOutcome, String> {
        let broken = case.broken.clone();
        let dir = self.scaffold(case.name, &broken)?;

        // Baseline check: the mutant MUST fail, with the declared code.
        let before = run_cargo(&self.cargo_bin, &dir, &["check", "--message-format=json"])?;
        let mut codes_before: Vec<String> = parse_error_diags(&before)
            .iter()
            .map(|d| d.code.clone())
            .collect();
        codes_before.sort();
        codes_before.dedup();
        let mutant_valid = codes_before.iter().any(|c| c == case.expect_code);

        let t0 = Instant::now();
        fix(&dir)?;
        let fix_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let after = run_cargo(&self.cargo_bin, &dir, &["check", "--message-format=json"])?;
        let compiles = parse_error_diags(&after).is_empty();
        let final_source = std::fs::read_to_string(dir.join("src").join("lib.rs"))
            .map_err(|e| format!("read back {}: {e}", case.name))?;

        let restored = normalized(&final_source) == normalized(case.seed);
        let (verdict, diverged_source) = match (compiles, restored) {
            (false, _) => (FixVerdict::Unfixed, None),
            (true, true) => (FixVerdict::MatchedInverse, None),
            (true, false) => (FixVerdict::Diverged, Some(final_source)),
        };

        Ok(FloorOutcome {
            name: case.name.to_string(),
            expect_code: case.expect_code,
            mutator: case.kind,
            codes_before,
            mutant_valid,
            verdict,
            diverged_source,
            fix_ms,
        })
    }

    /// Run `cargo test` in a scaffold dir (the G1 semantics pass — every
    /// generated seed carries a ground-truth behavior test, so a fix that
    /// compiles but changes behavior FAILS here, which textual
    /// matched-inverse scoring cannot see).
    ///
    /// # Errors
    ///
    /// cargo unspawnable / dir unreadable. A FAILING test suite is
    /// `Ok(false)`, not an error.
    pub fn cargo_test_passes(&self, case_dir: &Path) -> Result<bool, String> {
        let out = Command::new(&self.cargo_bin)
            .args(["test", "--quiet"])
            .current_dir(case_dir)
            .env("CARGO_TARGET_DIR", case_dir.join("target"))
            .env_remove("RUSTFLAGS")
            .env_remove("RUSTC_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTFLAGS")
            .output()
            .map_err(|e| format!("cargo test spawn failed: {e}"))?;
        Ok(out.status.success())
    }

    /// Seed-validity gate (the corpus's OWN ground truth): scaffold every
    /// case's SEED and run its `cargo test`. A seed whose ground-truth
    /// test fails poisons every arm's semantics pass — the first corpus
    /// shipped 10 such seeds (hand-counted string lengths) and only the
    /// post-fix test pass caught them, because the UNREPAIRED cases never
    /// get tested. Returns the failing case names.
    ///
    /// # Errors
    ///
    /// I/O or cargo-spawn failure.
    pub fn verify_seed_tests(&self, cases: &[HarnessCase<'_>]) -> Result<Vec<String>, String> {
        let mut bad = Vec::new();
        for case in cases {
            let dir = self.scaffold(case.name, case.seed)?;
            match self.cargo_test_passes(&dir) {
                Ok(true) => {}
                Ok(false) => bad.push(case.name.to_string()),
                Err(e) => return Err(format!("{}: {e}", case.name)),
            }
        }
        Ok(bad)
    }

    fn scaffold(&self, name: &str, source: &str) -> Result<PathBuf, String> {
        let dir = self.work_root.join(name);
        let src = dir.join("src");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&src).map_err(|e| format!("create {}: {e}", src.display()))?;
        // Package names must be rustc identifiers — the corpus names carry
        // literal suffixes ("world!") that make invalid crate names, and an
        // invalid manifest produces ZERO diagnostics (cargo fails before
        // rustc), which reads as a false corpus-integrity failure.
        let pkg: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let manifest = format!(
            "[package]\nname = \"{pkg}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n"
        );
        std::fs::write(dir.join("Cargo.toml"), manifest)
            .map_err(|e| format!("write manifest: {e}"))?;
        std::fs::write(src.join("lib.rs"), source).map_err(|e| format!("write lib.rs: {e}"))?;
        Ok(dir)
    }

    pub fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.work_root);
    }
}

/// Whitespace-insensitive comparison: `cargo fix` may reflow, and an
/// indentation difference is not a semantic one.
fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn aggregate(outcomes: &[FloorOutcome]) -> Vec<FloorRow> {
    let mut codes: Vec<String> = outcomes.iter().map(|o| o.expect_code.to_string()).collect();
    codes.sort();
    codes.dedup();
    codes
        .into_iter()
        .map(|code| {
            let mine: Vec<&FloorOutcome> =
                outcomes.iter().filter(|o| o.expect_code == code).collect();
            FloorRow {
                code,
                mutants: mine.len(),
                matched: mine
                    .iter()
                    .filter(|o| o.verdict == FixVerdict::MatchedInverse)
                    .count(),
                diverged: mine
                    .iter()
                    .filter(|o| o.verdict == FixVerdict::Diverged)
                    .count(),
                unfixed: mine
                    .iter()
                    .filter(|o| o.verdict == FixVerdict::Unfixed)
                    .count(),
            }
        })
        .collect()
}

fn run_cargo(cargo_bin: &str, dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cargo_bin)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_corpus_mutation_applies_and_changes_the_seed() {
        for case in MUTANT_CASES {
            let broken = case
                .broken()
                .unwrap_or_else(|e| panic!("{}: {e}", case.name));
            assert_ne!(
                normalized(&broken),
                normalized(case.seed),
                "{}: mutation was a no-op",
                case.name
            );
        }
    }

    #[test]
    fn corpus_covers_both_territories() {
        let uncontested = ["E0597", "E0499", "E0502"];
        for code in uncontested {
            assert!(
                MUTANT_CASES.iter().any(|c| c.expect_code == code),
                "uncontested code {code} missing from the corpus"
            );
        }
        for code in ["E0596", "E0382", "E0505", "E0507", "E0373"] {
            assert!(
                MUTANT_CASES.iter().any(|c| c.expect_code == code),
                "contested code {code} missing from the corpus"
            );
        }
    }

    #[test]
    fn mutation_site_missing_is_an_error_not_a_noop() {
        let m = Mutator {
            kind: MutatorKind::DropMutBinding,
            from: "let mut nope = 1;",
            to: "let nope = 1;",
        };
        let err = mutate_compile_break("pub fn f() {}", &m).unwrap_err();
        assert!(err.contains("not found"), "got {err}");
    }

    #[test]
    fn mutation_replaces_only_the_first_occurrence() {
        let m = Mutator {
            kind: MutatorKind::DropCloneCall,
            from: ".clone()",
            to: "",
        };
        let out = mutate_compile_break("let a = v.clone(); let b = v.clone();", &m).unwrap();
        assert_eq!(out, "let a = v; let b = v.clone();");
    }

    #[test]
    fn normalized_ignores_only_whitespace() {
        assert_eq!(normalized("let  x =\n1;"), normalized("let x = 1;"));
        assert_ne!(normalized("let x = 1;"), normalized("let x = 2;"));
    }

    #[test]
    fn rates_are_ratios_of_the_right_denominators() {
        let row = FloorRow {
            code: "E0382".to_string(),
            mutants: 4,
            matched: 1,
            diverged: 1,
            unfixed: 2,
        };
        // Fix rate counts matched + diverged over ALL mutants.
        assert!((row.fix_rate() - 0.5).abs() < 1e-12);
        // Divergence rate is over the FIXED subset, not all mutants.
        assert!((row.divergence_rate() - 0.5).abs() < 1e-12);
        let empty = FloorRow {
            code: "E0499".to_string(),
            mutants: 0,
            matched: 0,
            diverged: 0,
            unfixed: 0,
        };
        assert!((empty.fix_rate() - 0.0).abs() < 1e-12);
        assert!((empty.divergence_rate() - 0.0).abs() < 1e-12);
    }

    #[test]
    fn aggregate_partitions_every_outcome() {
        let outcomes = vec![
            FloorOutcome {
                name: "a".to_string(),
                expect_code: "E0382",
                mutator: MutatorKind::DropCloneCall,
                codes_before: vec!["E0382".into()],
                mutant_valid: true,
                verdict: FixVerdict::MatchedInverse,
                diverged_source: None,
                fix_ms: 0.0,
            },
            FloorOutcome {
                name: "b".to_string(),
                expect_code: "E0382",
                mutator: MutatorKind::DropCloneCall,
                codes_before: vec!["E0382".into()],
                mutant_valid: true,
                verdict: FixVerdict::Unfixed,
                diverged_source: None,
                fix_ms: 0.0,
            },
            FloorOutcome {
                name: "c".to_string(),
                expect_code: "E0597",
                mutator: MutatorKind::NarrowScope,
                codes_before: vec!["E0597".into()],
                mutant_valid: true,
                verdict: FixVerdict::Diverged,
                diverged_source: Some("x".into()),
                fix_ms: 0.0,
            },
        ];
        let rows = aggregate(&outcomes);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(
                row.matched + row.diverged + row.unfixed,
                row.mutants,
                "{} rows must partition",
                row.code
            );
        }
    }

    #[test]
    fn null_fixer_touches_nothing() {
        let dir = std::env::temp_dir().join(format!("riir_floor_null_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("marker.rs");
        std::fs::write(&f, "untouched").unwrap();
        NullFixer.fix(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "untouched");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
