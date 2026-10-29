//! `cargo refine --orphan-report` — post-mining intake class C1: the config
//! knob ORPHANED BY OPTIMIZATION (Plan 126).
//!
//! The measured shape (riir-ai Issue 888 T17 watch, riir-games-civ
//! `BehaviorConfig.alarm_radius`): a perf refactor replaces a config-fed
//! path with a compile-time table and copies the values faithfully — zero
//! test failures, the knob stays `pub` and looks live. The field had a
//! reader that vanished; detection therefore needs HISTORY, not just syntax
//! (the intake's own words) — a review-time + sweep-time heuristic, never a
//! per-compile lint. Review-time usage: run the report over the tree after
//! any commit touching config inits — the sweep instrument run at HEAD IS
//! the post-hoc check.
//!
//! Two legs:
//! * **Current (syn, reused):** [`crate::config_audit::zero_external_read_fields`]
//!   — Default-impl candidate fields with `external = total − tests −
//!   own_impl == 0` (the class-A condition-2 set at field granularity).
//! * **History (git, name-global):** count read-shaped occurrences
//!   (`\.name` + a non-ident tail — literal inits and bare definitions do
//!   not match; destructuring binds do not either, the safe direction) at
//!   HEAD and at probe commits (`HEAD~N`, default [`DEFAULT_PROBE_DEPTHS`],
//!   clamped to history depth). The verdict is the DELTA: any probe count
//!   above HEAD's ⇒ read-shaped occurrences vanished. The delta formulation
//!   survives own-impl reads (a `validate()` read that existed then AND now
//!   contributes 0 to the delta) and catches the canonical case (consumer
//!   removed, validate remained). Removal attribution: `git log
//!   --pickaxe-regex -S`, newest first; a newest-hit subject matching the
//!   optimization vocabulary tags the row `LIKELY-BY-OPTIMIZATION` (a tag,
//!   not a verdict).
//!
//! Verdict classes: `ORPHANED` rows (delta evidence + commit list),
//! `[stillborn]` one-liners (zero read-shaped occurrences at HEAD and every
//! probe — never read within the window; the single-dead-field gap C2's
//! whole-struct rule misses), and silent (read-shaped occurrences exist now
//! with no vanished evidence — A/B/C2 territory; counted in the header).
//!
//! Honest limits (mirrored in every report footer): the history greps are
//! name-global and untyped (a same-named field on another type can
//! masquerade — adjudicate rows against the listed commits); comment/doc
//! mentions count (the syn leg excludes them, `git grep` cannot); readers
//! outside the scanned path set are invisible (the inherited config-audit
//! footer); a reader MOVED rather than removed still drops the count
//! (validate-refactor false positives — measure, don't assume away); probes
//! bound the lookback window (an orphan older than the deepest probe reads
//! as stillborn); fields carrying `#[serde(skip_serializing_if = …)]` are
//! excluded from the net entirely (conditionally-emitted output schema —
//! the derive reads them, invisibly; the Issue 934 measured boundary —
//! adjudicate serialized-output structs by hand if you suspect a dead
//! schema field).
//!
//! Exit contract (the frontier-report law, not config-audit's): exit 0 for
//! every real scan; **refuse exit 1** on an empty population or a blind
//! history leg ([`OrphanReport::blind`]) — never a confident green zero.
//!
//! # Wire-consumption clearance (the measured WIRE-FP repair, 2026-09-12)
//!
//! Serialize-derive field reads are invisible to a syntactic member-read
//! scan, so Ser+De wire structs serialized at call sites read stillborn —
//! measured on two families (riir-chain delegation builders, riir-dao
//! `EpochSummary`; the cross-repo sweep after `d2e39d1f`, 8 rows, ALL
//! adjudicated false positive). The repair is a THIRD leg over the same
//! sources ([`wire_consumption`]): a struct-literal of type T inside a fn
//! that also makes a `serde_json::to_*` / `serde_yaml::to_*` path call
//! marks T's fields derive-consumed (**fn-scoped literal co-occurrence**);
//! a crate-rooted `serde_json::json!` macro counts as that serialization
//! call too (**the macro-as-serializer arm** — each interpolated arg goes
//! through `to_value` at runtime; the riir-kat `shop_seed_body` FP,
//! 2026-09-17); a `Vec<T>`/`Option<T>` field of a literal-constructed wire
//! struct marks T consumed transitively (**the wrapping clause** — the
//! `SelfReportWire { targets: Vec<ReportTarget> }` shape, where the inner
//! type's literals live only in tests). Measured on the 8 pinned rows:
//! swallow rate 1/8 — the sole swallow is a field hardcoded `true` at its
//! only site, which is exactly why constant-valued fields are **NOT
//! cleared** (the carve-out): a field whose EVERY construction site is a
//! literal constant is schema-dead weight the receiver cannot be using —
//! reported at the weak `[wire-const]` severity instead. Clearance applies
//! to stillborn rows only (the measured FP class); ORPHANED rows keep their
//! independent history evidence. Limits: fn-scoped approximation (a literal
//! built in fn A and serialized in fn B is not cleared unless fn B serializes
//! its own CONCRETE param, or binds the value from a same-file fn's RETURN —
//! the two fn-A/fn-B links below); path-call form only (aliased imports
//! invisible); type names are file-global (no module resolution, same as the
//! read greps); the value classification sees literal sites only (a field
//! never set at any visible literal is treated as cleared — its value comes
//! from deserialization or defaults, and the wire IS the consumption).
//!
//! # Foreign-literal attribution + generic serde bridges (Issue 109, 2026-09-15)
//!
//! Two measured FP classes the 2026-09-15 quiet-repo sweep filed as specimen
//! evidence:
//!
//! * **Foreign literals (specimen A, riir-shader):** a bare `Name { … }`
//!   literal NEVER resolves to a sibling module's same-named struct — within
//!   one file it resolves only to that file's declaration, an explicit
//!   import, or a glob. The wire pass (and the candidate net) match type
//!   NAMES file-globally, so bevy's `Node { … }` in a file importing
//!   `use bevy::prelude::*` masqueraded as construction evidence for a local
//!   serde `Node` (4 stillborn rows, zero true findings) — and, in a serde
//!   fn, would have FALSELY CLEARED the local struct's rows. The repair:
//!   [`crate::config_audit::FileScope`] — provably-foreign literals are not
//!   evidence in either direction (skipped for sites AND wire membership).
//!   Re-sight (the bevy 0.19 migration restored the 4 rows through the
//!   documented undecidable shape — a file importing BOTH the foreign glob
//!   and a workspace-sibling glob reads local): the scan-global field sets
//!   ([`crate::config_audit::collect_struct_field_sets`]) close it — a
//!   literal naming a field that no local same-named decl has is provably
//!   foreign, by the compile argument rather than the import shape.
//! * **Generic serde bridges (specimen B, seal-game-editor):**
//!   `fn row_to_mapping<T: serde::Serialize>(row: &T)` — the bridge fn has
//!   NO struct literal (the serialized value is the generic param), and its
//!   callers have NO serde call, so fn-scoped co-occurrence is not merely
//!   missed, it inapplicable. The concrete consumed types are only nameable
//!   at the bridge's CALL SITES (`save_t::<CraftRow>(m)`,
//!   `row_to_mapping(&Repo::<CraftRow>::get(..))` — turbofish). The repair:
//!   bridge detection (Serialize-bounded param + a serde `to_*` call on it)
//!   marks the turbofish-named types [`WireConsumption::bridge_types`] —
//!   stillborn rows there count as bridge-cleared, and ORPHANED rows keep
//!   their history evidence but render at **wire-dto weak** severity (the
//!   receiver consumes the serialized field; the in-repo read loss is not
//!   the death verdict it would be for a local-only struct).
//! * **The documented-write-only downgrade (P26, the config-audit parity):**
//!   a zero-external-read row (stillborn, ORPHANED, or silent — the silent
//!   arm is the test-pinned forward surface: reads exist but are all the
//!   field's own test scaffolding) whose FIELD doc carries a deliberate-
//!   write-only marker (`documented-write-only` | `deliberate` |
//!   `forward surface` | `GOAT-gate territory` — the Issue-142/147 owner
//!   vocabulary, one home in [`crate::config_audit`]) renders at the weak
//!   `[documented-write-only]` annotation instead — DOWNGRADE-not-suppress
//!   (the detector cannot verify the doc's claim, the `[wire-const]`
//!   precedent). Scope: findings only — wire-cleared and bridge-cleared
//!   rows stay cleared (consumed beats annotated), wire-const stays its own
//!   class, and the marker counts on the FIELD doc only. A marked ORPHANED
//!   row skips the pickaxe leg (the annotation needs no commit evidence).
//!   Live evidence: the 09-16 quiet-repo sweep's four adjudicated rows
//!   (riir-chain `GuardVerdict.redirect` — flagged WITH its full marker —
//!   plus `backoff_ticks`, riir-deployer `TargetState.started_unix`,
//!   riir-auth `AccountState.created_at_unix`) re-appeared in every sweep
//!   until this landed.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::config_audit::{
    FieldCandidate, FileScope, WalkMode, ZeroReadField, ZeroReadScan, collect_struct_field_sets,
    field_member_name, literal_is_field_mismatch,
};

/// Default probe depths (commits back from HEAD), clamped to history depth.
pub const DEFAULT_PROBE_DEPTHS: [usize; 4] = [10, 50, 200, 1000];

/// How many pickaxe commits to attribute per ORPHANED row (newest first).
const PICKAXE_MAX: usize = 8;

/// Subjuncts that tag a row LIKELY-BY-OPTIMIZATION (lowercase, substring).
const OPTIMIZATION_VOCAB: [&str; 8] = [
    "perf", "optimi", "refact", "fast", "inline", "table", "cache", "simd",
];

/// One `git log -S` hit (newest first) — where the pattern's occurrence
/// count changed inside the scanned path set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHit {
    pub hash: String,
    pub date: String,
    pub subject: String,
}

/// The history leg, behind a trait so the classifier is unit-testable
/// without a git repository (the production impl is [`GitCli`]).
pub trait HistoryProbe {
    /// Resolve `HEAD~<depth>` (`depth == 0` is HEAD); `None` when the depth
    /// exceeds history (the probe is clamped out).
    fn rev_at_depth(&self, depth: usize) -> Option<String>;
    /// Count lines matching the ERE at `rev` within the scanned path set;
    /// `None` when git cannot answer (treated as no evidence).
    fn count_ere(&self, rev: &str, pattern: &str) -> Option<usize>;
    /// Newest-first commits where the ERE's occurrence count changed.
    fn pickaxe(&self, pattern: &str, max: usize) -> Vec<CommitHit>;
}

/// The production history leg: shells `git` with the repo root as `-C` and
/// the scanned paths (relativized) as the pathspec for every query.
pub struct GitCli {
    root: PathBuf,
    pathspecs: Vec<String>,
}

impl GitCli {
    /// Build from the scanned paths. `None` when the anchor is not inside a
    /// resolvable git repository with a born HEAD, or when no scanned path
    /// is inside it (the caller reports a blind history leg, never a green
    /// zero over it).
    pub fn new(paths: &[PathBuf]) -> Option<Self> {
        let anchor = paths.first()?;
        // Paths may be files; git -C wants their directory.
        let dir = if anchor.is_dir() {
            anchor.clone()
        } else {
            anchor.parent().map(Path::to_path_buf).unwrap_or_default()
        };
        let root_out = git(Some(&dir), &["rev-parse", "--show-toplevel"])?;
        if !root_out.status.success() {
            return None;
        }
        let root = PathBuf::from(String::from_utf8_lossy(&root_out.stdout).trim());
        // An unborn repository has no tree to grep — blind, not empty.
        let head = git(Some(&root), &["rev-parse", "--verify", "--quiet", "HEAD"])?;
        if !head.status.success() {
            return None;
        }
        let cwd = std::env::current_dir().ok()?;
        // Windows: `canonicalize()` yields the verbatim form (`\\?\C:\…`)
        // while `rev-parse --show-toplevel` returns `C:/…` — component-wise
        // `strip_prefix` then fails for EVERY pathspec and the whole history
        // leg reads blind (same symptom class as the macOS symlink note
        // below, Windows shape). Canonicalize the root for the COMPARISON
        // only; git calls keep the non-verbatim root (git -C accepts both,
        // the display stays clean).
        let root_cmp = root.canonicalize().unwrap_or_else(|_| root.clone());
        let mut pathspecs = Vec::new();
        for p in paths {
            let joined = if p.is_absolute() {
                p.clone()
            } else {
                cwd.join(p)
            };
            // macOS scan paths are frequently symlinked (/var/folders/…)
            // while git reports the real path — canonicalize before the
            // prefix comparison or every pathspec is dropped.
            let abs = joined.canonicalize().unwrap_or(joined);
            if let Ok(rel) = abs.strip_prefix(&root_cmp) {
                let rel = rel.display().to_string();
                // Scanning the repo root itself: rel is empty — the
                // pathspec is the whole tree ("."), never a blind leg.
                if rel.is_empty() {
                    pathspecs.push(".".to_string());
                } else {
                    pathspecs.push(rel);
                }
            }
            // Outside this repository: no history for it — the scanned-set
            // footer covers the under-report.
        }
        if pathspecs.is_empty() {
            return None;
        }
        Some(Self { root, pathspecs })
    }

    fn root_str(&self) -> &str {
        self.root.to_str().unwrap_or(".")
    }
}

fn git(dir: Option<&Path>, args: &[&str]) -> Option<Output> {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.arg("-C").arg(d);
    }
    cmd.args(args);
    cmd.output().ok()
}

impl HistoryProbe for GitCli {
    fn rev_at_depth(&self, depth: usize) -> Option<String> {
        let rev = if depth == 0 {
            "HEAD".to_string()
        } else {
            format!("HEAD~{depth}")
        };
        let out = git(
            Some(&self.root),
            &["rev-parse", "--verify", "--quiet", &rev],
        )?;
        out.status.success().then_some(rev)
    }

    fn count_ere(&self, rev: &str, pattern: &str) -> Option<usize> {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(self.root_str())
            .args(["grep", "-E", "-e", pattern, rev, "--"])
            .args(&self.pathspecs);
        let out = cmd.output().ok()?;
        // git grep exits 1 on no matches (grep convention); 0 = matched.
        match out.status.code() {
            Some(0 | 1) => {}
            _ => return None,
        }
        let text = String::from_utf8_lossy(&out.stdout);
        Some(text.lines().filter(|l| !l.trim().is_empty()).count())
    }

    fn pickaxe(&self, pattern: &str, max: usize) -> Vec<CommitHit> {
        let needle = format!("-S{pattern}");
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(self.root_str())
            .args([
                "log",
                "--pickaxe-regex",
                needle.as_str(),
                "--format=%h%x09%ad%x09%s",
                "--date=short",
                "--max-count",
            ])
            .arg(max.to_string())
            .arg("--")
            .args(&self.pathspecs);
        let Ok(out) = cmd.output() else {
            return Vec::new();
        };
        if !out.status.success() {
            return Vec::new();
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let (hash, rest) = l.split_once('\t')?;
                let (date, subject) = rest.split_once('\t')?;
                Some(CommitHit {
                    hash: hash.to_string(),
                    date: date.to_string(),
                    subject: subject.to_string(),
                })
            })
            .collect()
    }
}

/// One ORPHANED field: the candidate + the evidence that convicted it.
#[derive(Debug, Clone)]
pub struct OrphanRow {
    pub candidate: FieldCandidate,
    pub total_reads: usize,
    pub test_reads: usize,
    pub self_impl_reads: usize,
    /// `(rev-label, count)` evidence, HEAD first, deepest probe last.
    pub probes: Vec<(String, usize)>,
    /// Newest-first commits that changed the pattern's occurrence count.
    pub commits: Vec<CommitHit>,
    /// The newest count-change subject matches the optimization vocabulary.
    pub likely_by_optimization: bool,
    /// The struct is bridge-consumed (a generic serde bridge serializes it
    /// onward; Issue 109 specimen B) — the in-repo read loss is weak
    /// evidence while the receiver consumes the serialized field.
    pub bridge_weak: bool,
}

/// The report (data only — rendering is [`OrphanReport::render`]).
#[derive(Debug, Default)]
pub struct OrphanReport {
    pub files_scanned: usize,
    /// How the file set was derived (P25) — rendered before the blind
    /// branch so even a refusal shows the walk mode.
    pub walk: WalkMode,
    pub candidates: usize,
    pub zero_read_fields: usize,
    pub orphaned: Vec<OrphanRow>,
    pub stillborn: Vec<ZeroReadField>,
    /// Stillborn rows CLEARED by the wire-consumption pass (serde fn
    /// co-occurrence / wrapping clause + a non-constant site value) —
    /// counted, not listed (they are the measured-FP class).
    pub wire_cleared: usize,
    /// Wire-consumed but literal-constant at every construction site —
    /// schema-dead weight, weak severity (never cleared).
    pub wire_const: Vec<ZeroReadField>,
    /// Stillborn/ORPHANED rows whose FIELD doc carries a deliberate-
    /// write-only marker — downgraded to weak severity, never suppressed
    /// (the P26 config-audit parity; the doc's claim is not verified).
    pub documented_write_only: Vec<ZeroReadField>,
    /// Stillborn rows on bridge-consumed structs (Issue 109 specimen B:
    /// generic serde bridge at the call sites) — counted, not listed (the
    /// same measured-FP posture as `wire_cleared`).
    pub bridge_cleared: usize,
    /// The probe depths actually resolved (clamped to history depth).
    pub probe_depths: Vec<usize>,
    /// Why the history leg is missing — the caller refuses with exit 1
    /// (never a confident green zero over a blind instrument).
    pub blind: Option<String>,
}

/// The read-shaped ERE for a field name: `\.name` + a non-ident tail (or
/// end-of-line). Rust idents are `[A-Za-z0-9_]+` — safe inside an ERE.
#[must_use]
pub fn read_shape_ere(field_name: &str) -> String {
    format!(r"\.{field_name}([^[:alnum:]_]|$)")
}

/// Does a commit subject match the optimization vocabulary? A tag, not a
/// verdict — the canonical orphaner reads like "perf: const-table …".
#[must_use]
pub fn subject_looks_like_optimization(subject: &str) -> bool {
    let lower = subject.to_lowercase();
    OPTIMIZATION_VOCAB.iter().any(|v| lower.contains(v))
}

// ── Wire-consumption pass (the measured WIRE-FP repair — module doc above) ──

/// Per-(struct, field) verdict of the wire-consumption analysis.
#[derive(Debug, Default)]
pub struct WireConsumption {
    /// Fields PROVEN consumed by a serialization path: the struct (or a
    /// `Vec<_>`/`Option<_>` wrapper of it) is literal-constructed inside a
    /// fn that also makes a `serde_json::to_*`/`serde_yaml::to_*` call OR a
    /// crate-rooted `serde_json::json!` macro invocation, OR
    /// the type is a CONCRETE param serde-serialized by some fn (the
    /// sibling-fn link: literal in the flow, `to_string(&intent)` in the
    /// queue writer — the fn-A/fn-B repair, extended to json! interpolation),
    /// OR the type is a same-file fn's RETURN VALUE that a serializing fn
    /// binds into a local and serializes (`let fused = recommend(..);
    /// serde_json::to_string(&fused)` — the riir-reflex test-serializes-the-
    /// producer shape; same-file free-fn resolution, unique binding), AND
    /// the field carries a
    /// non-literal-constant value at some construction site. Stillborn
    /// rows matching here are cleared.
    pub cleared: HashSet<(String, String)>,
    /// Wire-consumed but literal-constant (`true`/`5`/`"x"`, incl. negated
    /// numeric literals) at EVERY construction site — schema-dead weight
    /// the receiver cannot be using; reported at the weak `[wire-const]`
    /// severity, never cleared (the 1/8 measured swallow, kept visible).
    pub wire_const: Vec<(String, String)>,
    /// Structs whose consumption is a GENERIC SERDE BRIDGE (Issue 109
    /// specimen B): the bridge fn serializes its `T: Serialize` param, so
    /// it contains no struct literal and its callers contain no serde call
    /// — fn-scoped co-occurrence cannot fire by construction. The concrete
    /// types are the turbofish names at the bridge's call sites
    /// (`save_t::<CraftRow>(m)`). Stillborn rows on these structs count as
    /// bridge-cleared; ORPHANED rows keep their evidence but render at
    /// wire-dto weak severity.
    pub bridge_types: HashSet<String>,
}

/// Is this expression a literal constant (the wire-const class)?
/// `Lit` and unary-minus `Lit` only — a const PATH is not a literal (and a
/// named const could change), a call/variable is a value.
fn expr_is_literal_const(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Lit(_) => true,
        syn::Expr::Unary(u) if matches!(*u.expr, syn::Expr::Lit(_)) => true,
        _ => false,
    }
}

/// Does the path read like `serde_json::to_vec` / `serde_yaml::to_string_pretty`
/// (crate-rooted serializer entry)? Last-segment `to_*` on the two crates the
/// pinned evidence exercises.
fn is_serde_to_path(path: &syn::Path) -> bool {
    let idents: Vec<&syn::Ident> = path.segments.iter().map(|s| &s.ident).collect();
    idents.len() >= 2
        && matches!(idents[0].to_string().as_str(), "serde_json" | "serde_yaml")
        && idents[idents.len() - 1].to_string().starts_with("to_")
}

/// Is this macro invocation `serde_json::json!` (crate-rooted form) — the
/// macro that IS a serializer? `json!({"items": items})` interpolates each
/// non-literal arg through `serde_json::to_value` at runtime, so a wire
/// struct interpolated into it is serialized with no `to_*` path call
/// anywhere in the fn (the riir-kat `shop_seed_body` shape, recorded
/// 2026-09-17 in `.distill/001` §post-mining intake). Conservative on
/// purpose: only the crate-rooted two-segment form counts — a bare `json!`
/// behind `use serde_json::json` is one import away from ambiguity with a
/// local macro, and the pinned evidence is crate-rooted.
fn macro_is_serde_json_value(mac: &syn::Macro) -> bool {
    let segs: Vec<String> = mac
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    segs.len() == 2 && segs[0] == "serde_json" && segs[1] == "json"
}

/// One construction-site entry: the field was set at an `ExprStruct` of the
/// (file-global) type name, with a literal-constant value or not, and
/// whether the site sits in TEST context (`#[cfg(test)]` mods, `#[test]`
/// fns, a `tests/`/`benches/` file — the `config_audit` context rules). A
/// test-context site is fixture evidence: a protocol-mirror crate's tests
/// construct every wire type with literal constants BY DESIGN while the
/// production constructors live outside the scanned set, so test-only
/// sites must not arm the `[wire-const]` class (the riir-kat FP class,
/// 2026-09-21).
struct SiteEntry {
    is_const: bool,
    in_test: bool,
}

/// The wire-pass visitor. Walks every file once, tracking (a) whether we are
/// inside a fn that serializes, (b) the enclosing impl's own type (to resolve
/// `Self` literals), (c) every struct-literal field assignment, (d) calls of
/// generic serde bridges (turbofish type harvest).
#[derive(Default)]
struct WireVisitor {
    in_serde_fn: bool,
    impl_ty: Option<String>,
    /// File-level test context (a `tests/`/`benches/` path or the file's
    /// own `#![cfg(test)]`) — seeds [`WireVisitor::test_depth`].
    file_is_test: bool,
    /// `#[cfg(test)]` mod / `#[test]`-fn nesting depth (the `config_audit`
    /// context rules — `attrs_cfg_test` / `attr_is_test_fn`).
    test_depth: usize,
    /// Per-file name-resolution scope (Issue 109 specimen A) — provably
    /// foreign literals are not evidence in either direction.
    scope: FileScope,
    /// Type names literal-constructed inside a serde fn (direct evidence),
    /// plus CONCRETE param types serialized by the fn itself (the fn-B half
    /// of the fn-A/fn-B gap — both are direct wire evidence).
    direct_wire_types: HashSet<String>,
    /// (type, field) → per-site const classification, ALL contexts.
    sites: std::collections::HashMap<(String, String), Vec<SiteEntry>>,
    /// Struct-def edges: wrapper struct → inner type of a `Vec<T>`/`Option<T>` field.
    wrap_edges: HashSet<(String, String)>,
    /// Pass-1 product: generic serde bridge fn names (owned — the set is small).
    bridge_fns: HashSet<String>,
    /// Pass-2 accumulation: concrete types named at bridge call sites.
    bridge_types: HashSet<String>,
    /// Same-file free-fn return types (pass-1 product, swapped in per file):
    /// the resolution map for the serialized-return-value wire link. Same-
    /// file only by construction — a cross-file producer's return type is
    /// unknown here and the link misses (the conservative direction).
    fn_rets: BTreeMap<String, String>,
    /// Scan-global top-level struct field sets (Issue 109 re-sight): a
    /// literal naming a field that no local same-named decl has is provably
    /// foreign — neither a construction site nor wire-type membership. This
    /// closes the foreign-glob + local-glob shape the import-scope check
    /// documents as undecidable.
    local_fields: BTreeMap<String, BTreeSet<String>>,
}

impl<'ast> syn::visit::Visit<'ast> for WireVisitor {
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let saved = self.impl_ty.clone();
        if let syn::Type::Path(tp) = &*i.self_ty {
            if let Some(seg) = tp.path.segments.last() {
                self.impl_ty = Some(seg.ident.to_string());
            }
        }
        syn::visit::visit_item_impl(self, i);
        self.impl_ty = saved;
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        let enter_test = crate::config_audit::attrs_cfg_test(&i.attrs);
        if enter_test {
            self.test_depth += 1;
        }
        syn::visit::visit_item_mod(self, i);
        if enter_test {
            self.test_depth -= 1;
        }
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let saved = self.in_serde_fn;
        let enter_test = i.attrs.iter().any(crate::config_audit::attr_is_test_fn)
            || crate::config_audit::attrs_cfg_test(&i.attrs);
        if enter_test {
            self.test_depth += 1;
        }
        if block_has_serde_call(&i.block) {
            self.in_serde_fn = true;
            // The fn-B half of the fn-A/fn-B gap (the riir-kat queue-writer
            // shape, 2026-09-16, 16-row concentration): a fn that
            // serializes its own CONCRETE-typed param consumes that param's
            // type on the wire — the struct literal lives in the CALLER (the
            // flow fn), which fn-scoped co-occurrence cannot see. The
            // generic `T: Serialize` twin is `is_generic_serde_bridge`'s
            // lane; this is the concrete twin.
            for ty in serialized_concrete_param_types(i) {
                self.direct_wire_types.insert(ty);
            }
            // The producer's-return-value half (the riir-reflex
            // `FusedGateRecommendation` specimen, 2026-09-23): a local bound
            // from a same-file free fn's return and fed to a serde to_* call
            // consumes that return type — neither fn-scoped co-occurrence
            // (the literal lives in the producer) nor the param link (the
            // serializing fn receives NO param) can join them.
            for ty in serialized_return_value_types(&i.block, &self.fn_rets) {
                self.direct_wire_types.insert(ty);
            }
        }
        syn::visit::visit_item_fn(self, i);
        if enter_test {
            self.test_depth -= 1;
        }
        self.in_serde_fn = saved;
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        // Methods are `syn::ImplItem::Fn` in syn 2 — `visit_item_fn` never
        // fires inside impl blocks (the config_audit C2 lesson, mirrored).
        let enter_test = i.attrs.iter().any(crate::config_audit::attr_is_test_fn)
            || crate::config_audit::attrs_cfg_test(&i.attrs);
        if enter_test {
            self.test_depth += 1;
        }
        syn::visit::visit_impl_item_fn(self, i);
        if enter_test {
            self.test_depth -= 1;
        }
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        let name = i.ident.to_string();
        for f in &i.fields {
            if let syn::Type::Path(tp) = &f.ty {
                if let Some(seg) = tp.path.segments.last() {
                    let wrapper = seg.ident.to_string();
                    if matches!(wrapper.as_str(), "Vec" | "Option") {
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                            if args.args.len() == 1 {
                                if let syn::GenericArgument::Type(syn::Type::Path(inner)) =
                                    &args.args[0]
                                {
                                    if let Some(inner_seg) = inner.path.segments.last() {
                                        self.wrap_edges
                                            .insert((name.clone(), inner_seg.ident.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        syn::visit::visit_item_struct(self, i);
    }

    fn visit_expr_struct(&mut self, e: &'ast syn::ExprStruct) {
        let last_seg = e
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        // `Self { .. }` inside an impl resolves to the impl's own type.
        let ty = if last_seg == "Self" {
            self.impl_ty.clone().unwrap_or(last_seg)
        } else {
            last_seg
        };
        // Issue 109 specimen A: a provably-foreign literal is not evidence
        // for the scanned set's same-named struct — neither a construction
        // site (inflates the candidate net's stillborn rows) nor wire-type
        // membership (a foreign literal inside a serde fn would FALSELY
        // CLEAR the local struct's rows).
        if self.scope.literal_is_foreign(&ty) {
            syn::visit::visit_expr_struct(self, e);
            return;
        }
        let named: Vec<String> = e
            .fields
            .iter()
            .filter_map(|fv| field_member_name(&fv.member))
            .collect();
        if literal_is_field_mismatch(&self.local_fields, &ty, &named) {
            // Provably not the scanned set's same-named struct (a field no
            // local decl has): neither a construction site nor wire-type
            // membership — mirrors the literal_is_foreign early return above.
            syn::visit::visit_expr_struct(self, e);
            return;
        }
        if self.in_serde_fn {
            self.direct_wire_types.insert(ty.clone());
        }
        for fv in &e.fields {
            let field_name = match &fv.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(idx) => format!("_{}", idx.index),
            };
            self.sites
                .entry((ty.clone(), field_name))
                .or_default()
                .push(SiteEntry {
                    is_const: expr_is_literal_const(&fv.expr),
                    in_test: self.file_is_test || self.test_depth > 0,
                });
        }
        syn::visit::visit_expr_struct(self, e);
    }

    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*c.func {
            if p.path
                .segments
                .last()
                .is_some_and(|s| self.bridge_fns.contains(&s.ident.to_string()))
            {
                // The call's own turbofish (`save_t::<T>(..)`) plus every
                // turbofish inside the args (`Repo::<T>::get(..)`) name the
                // bridge's concrete consumed types.
                let mut t = TurbofishTypes {
                    out: &mut self.bridge_types,
                };
                syn::visit::visit_expr_call(&mut t, c);
            }
        }
        syn::visit::visit_expr_call(self, c);
    }
}

/// Quick sub-walk: does the block contain a `serde_json::to_*`/`serde_yaml::to_*`
/// path call (anywhere inside, nested exprs included)?
fn block_has_serde_call(block: &syn::Block) -> bool {
    struct Finder(bool);
    impl<'ast> syn::visit::Visit<'ast> for Finder {
        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if let syn::Expr::Path(p) = &*c.func {
                if is_serde_to_path(&p.path) {
                    self.0 = true;
                }
            }
            syn::visit::visit_expr_call(self, c);
        }
        fn visit_expr_macro(&mut self, m: &'ast syn::ExprMacro) {
            if token_stream_has_serde_to(&m.mac.tokens) || macro_is_serde_json_value(&m.mac) {
                self.0 = true;
            }
            syn::visit::visit_expr_macro(self, m);
        }
        fn visit_item_macro(&mut self, m: &'ast syn::ItemMacro) {
            if token_stream_has_serde_to(&m.mac.tokens) || macro_is_serde_json_value(&m.mac) {
                self.0 = true;
            }
            syn::visit::visit_item_macro(self, m);
        }
    }
    let mut f = Finder(false);
    syn::visit::visit_block(&mut f, block);
    f.0
}

// ── Macro token-stream scanning (the writeln! class) ──
//
// `writeln!(f, "{}", serde_json::to_string(intent)…)` — the serde call sits
// INSIDE the macro's token stream, which syn's visitor does not descend. The
// queue-writer shape (the fn-A/fn-B intake's measured specimen) is exactly
// this: token-level scanning of the macro body for the crate-rooted `to_*`
// path, then the immediately following argument group for the param ident.

/// Does the token stream contain a `serde_json::to_*`/`serde_yaml::to_*`
/// path (recursively — nested groups included)?
fn token_stream_has_serde_to(tts: &proc_macro2::TokenStream) -> bool {
    token_stream_serde_to_mentions(tts, &[])
}

/// Token-level: does the stream contain a crate-rooted serde `to_*` path
/// whose ARGUMENT group mentions one of `names` (bare ident, recursive)?
/// With `names` empty this is just the path match. Token streams are what
/// macros hand us — `writeln!(f, "{}", serde_json::to_string(intent)…)` —
/// where syn's visitor cannot descend. Matching is conservative in the
/// operator's direction: the path must be EXACTLY `serde_json::`/`serde_yaml::`
/// (ident equality, not prefix) followed by a `to_*` ident and a
/// parenthesis group; inside the argument group a bare ident match is
/// accepted (`other.intent` at token level is indistinguishable from
/// `intent.clone()`'s receiver — inside a serializer argument the name
/// appearing at all is strong evidence the value flows into it).
fn token_stream_serde_to_mentions(tts: &proc_macro2::TokenStream, names: &[String]) -> bool {
    let tokens: Vec<proc_macro2::TokenTree> = tts.clone().into_iter().collect();
    for (i, tt) in tokens.iter().enumerate() {
        if let proc_macro2::TokenTree::Group(g) = tt {
            if token_stream_serde_to_mentions(&g.stream(), names) {
                return true;
            }
            continue;
        }
        let proc_macro2::TokenTree::Ident(root) = tt else {
            continue;
        };
        if *root != "serde_json" && *root != "serde_yaml" {
            continue;
        }
        // `::` — two Punct tokens (Joint spacing on the first)
        let colon_pair = matches!((tokens.get(i + 1), tokens.get(i + 2)), (
            Some(proc_macro2::TokenTree::Punct(a)),
            Some(proc_macro2::TokenTree::Punct(b)),
        ) if a.as_char() == ':' && b.as_char() == ':');
        if !colon_pair {
            continue;
        }
        let Some(proc_macro2::TokenTree::Ident(f)) = tokens.get(i + 3) else {
            continue;
        };
        if !f.to_string().starts_with("to_") {
            continue;
        }
        let Some(proc_macro2::TokenTree::Group(g)) = tokens.get(i + 4) else {
            continue;
        };
        if g.delimiter() == proc_macro2::Delimiter::Parenthesis
            && (names.is_empty() || group_mentions_ident(g.stream(), names))
        {
            return true;
        }
    }
    false
}

/// Does the token group mention one of `names` as a bare Ident anywhere
/// (recursively — nested groups included)? Token-level: `other.intent`
/// (a different binding's field) also matches — accepted inside a serde
/// argument group specifically, where the name appearing at all is strong
/// evidence the value flows into the serializer.
fn group_mentions_ident(tts: proc_macro2::TokenStream, names: &[String]) -> bool {
    for tt in tts {
        match tt {
            proc_macro2::TokenTree::Ident(id) => {
                if names.iter().any(|n| id == n.as_str()) {
                    return true;
                }
            }
            proc_macro2::TokenTree::Group(g) if group_mentions_ident(g.stream(), names) => {
                return true;
            }
            _ => {}
        }
    }
    false
}

// ── Generic serde bridges (Issue 109 specimen B — module doc above) ──

/// A Serialize-flavored trait bound (`serde::Serialize`, `Serialize` — last
/// path segment match, the same granularity every name here works at).
fn is_serialize_bound(b: &syn::TypeParamBound) -> bool {
    matches!(b, syn::TypeParamBound::Trait(t)
        if t.path.segments.last().is_some_and(|s| s.ident == "Serialize"))
}

/// The bridge predicate: ≥1 Serialize-bounded generic param (in the param
/// list OR a where clause — `save_t`'s bounds live there) whose VALUE is
/// serialized by a serde `to_*` call. The value link goes through fn params
/// TYPED over the bounded param (`row: &T`) — the bridge serializes `row`,
/// not `T`; the param-mention check keeps a fn that serializes an
/// unrelated CONCRETE value while merely carrying an unused Serialize
/// param out of the bridge set.
fn is_generic_serde_bridge(i: &syn::ItemFn) -> bool {
    let mut ser_params: Vec<String> = i
        .sig
        .generics
        .type_params()
        .filter(|p| p.bounds.iter().any(is_serialize_bound))
        .map(|p| p.ident.to_string())
        .collect();
    if let Some(w) = &i.sig.generics.where_clause {
        for pred in &w.predicates {
            let syn::WherePredicate::Type(pt) = pred else {
                continue;
            };
            if !pt.bounds.iter().any(is_serialize_bound) {
                continue;
            }
            if let syn::Type::Path(tp) = &pt.bounded_ty {
                if tp.path.segments.len() == 1 {
                    ser_params.push(tp.path.segments[0].ident.to_string());
                }
            }
        }
    }
    if ser_params.is_empty() {
        return false;
    }
    let mut value_names: Vec<String> = Vec::new();
    for input in &i.sig.inputs {
        let syn::FnArg::Typed(pt) = input else {
            continue;
        };
        if type_mentions_ident(&pt.ty, &ser_params) {
            if let syn::Pat::Ident(pi) = &*pt.pat {
                value_names.push(pi.ident.to_string());
            }
        }
    }
    if value_names.is_empty() {
        return false;
    }
    block_serializes_param(&i.block, &value_names)
}

/// Does the block contain a serde `to_*` call whose argument mentions one of
/// the generic param names?
fn block_serializes_param(block: &syn::Block, params: &[String]) -> bool {
    struct F<'a> {
        params: &'a [String],
        hit: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for F<'_> {
        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if let syn::Expr::Path(p) = &*c.func {
                if is_serde_to_path(&p.path) {
                    for arg in &c.args {
                        if expr_mentions_ident(arg, self.params) {
                            self.hit = true;
                        }
                    }
                }
            }
            syn::visit::visit_expr_call(self, c);
        }
        fn visit_expr_macro(&mut self, m: &'ast syn::ExprMacro) {
            if token_stream_serde_to_mentions(&m.mac.tokens, self.params)
                || (macro_is_serde_json_value(&m.mac)
                    && group_mentions_ident(m.mac.tokens.clone(), self.params))
            {
                self.hit = true;
            }
            syn::visit::visit_expr_macro(self, m);
        }
        fn visit_item_macro(&mut self, m: &'ast syn::ItemMacro) {
            if token_stream_serde_to_mentions(&m.mac.tokens, self.params)
                || (macro_is_serde_json_value(&m.mac)
                    && group_mentions_ident(m.mac.tokens.clone(), self.params))
            {
                self.hit = true;
            }
            syn::visit::visit_item_macro(self, m);
        }
    }
    let mut f = F { params, hit: false };
    syn::visit::visit_block(&mut f, block);
    f.hit
}

/// The fn-B half of the fn-A/fn-B gap (the riir-kat queue-writer shape,
/// 2026-09-16, 16-row concentration): the CONCRETE types of a fn's own
/// params that the fn's body serde-serializes. Per-param mention check — a
/// fn that serializes an unrelated value while merely RECEIVING a param of
/// the type does not clear it. Generic type params are skipped (the
/// `T: Serialize` twin belongs to `is_generic_serde_bridge`).
fn serialized_concrete_param_types(i: &syn::ItemFn) -> Vec<String> {
    let generics: std::collections::HashSet<String> = i
        .sig
        .generics
        .type_params()
        .map(|p| p.ident.to_string())
        .collect();
    let mut out = Vec::new();
    for input in &i.sig.inputs {
        let syn::FnArg::Typed(pt) = input else {
            continue;
        };
        let syn::Pat::Ident(pi) = &*pt.pat else {
            continue;
        };
        let mut ty = &*pt.ty;
        loop {
            match ty {
                syn::Type::Reference(r) => ty = &r.elem,
                syn::Type::Group(g) => ty = &g.elem,
                syn::Type::Paren(p) => ty = &p.elem,
                // `&[T]` slice params (the riir-kat `shop_seed_body` shape:
                // `items: &[ShopSeedItemWire]` interpolated into json!):
                // the SLICE is the wire vessel, T is the consumed type.
                syn::Type::Slice(s) => {
                    ty = &s.elem;
                    break;
                }
                _ => break,
            }
        }
        // `&Vec<T>` params: Vec is a path, extract the element.
        let vec_elem: Option<String> = match ty {
            syn::Type::Path(tp) => tp.path.segments.last().and_then(|seg| {
                if seg.ident != "Vec" {
                    return None;
                }
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if args.args.len() == 1 {
                        if let syn::GenericArgument::Type(syn::Type::Path(inner)) = &args.args[0] {
                            return inner.path.segments.last().map(|s| s.ident.to_string());
                        }
                    }
                }
                None
            }),
            _ => None,
        };
        let syn::Type::Path(tp) = ty else { continue };
        let Some(seg) = tp.path.segments.last() else {
            continue;
        };
        let name = seg.ident.to_string();
        if generics.contains(&name) {
            continue;
        }
        let ident = pi.ident.to_string();
        if block_serializes_param(&i.block, &[ident]) {
            match vec_elem {
                // `&Vec<T>`: only the ELEMENT is the wire type — a bare
                // `Vec` name would clear nothing real.
                Some(elem) if !generics.contains(&elem) => out.push(elem),
                Some(_) => {}
                None => out.push(name),
            }
        }
    }
    out.shrink_to_fit();
    out
}

/// The bare ident a fn's return type names, unwrapping `Option<T>` /
/// `Vec<T>` / `Result<T, _>` / `Box<T>` one wrapper level (the wrap-edge
/// convention): `-> FusedGateRecommendation` names the struct,
/// `-> Option<Report>` names `Report`. Multi-segment paths take the last
/// segment (the same granularity every name here works at); tuples, fn
/// pointers and trait objects → None (no bare name to resolve).
fn return_type_ident(ret: &syn::ReturnType) -> Option<String> {
    let syn::ReturnType::Type(_, ty) = ret else {
        return None;
    };
    let mut ty: &syn::Type = ty;
    loop {
        match ty {
            syn::Type::Reference(r) => ty = &r.elem,
            syn::Type::Group(g) => ty = &g.elem,
            syn::Type::Paren(p) => ty = &p.elem,
            _ => break,
        }
    }
    let syn::Type::Path(tp) = ty else {
        return None;
    };
    let last = tp.path.segments.last()?;
    let name = last.ident.to_string();
    if matches!(name.as_str(), "Option" | "Vec" | "Result" | "Box") {
        if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
            if let Some(syn::GenericArgument::Type(syn::Type::Path(inner))) = args.args.first() {
                return inner.path.segments.last().map(|s| s.ident.to_string());
            }
        }
        return None;
    }
    Some(name)
}

/// Pass-1 visitor: collects each free fn's return-type ident — the same-file
/// resolution map for [`serialized_return_value_types`]. Methods are not
/// collected (receiver-type resolution is a second map; add on a measured
/// specimen).
struct FnRetFinder {
    out: BTreeMap<String, String>,
}

impl<'ast> syn::visit::Visit<'ast> for FnRetFinder {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if let Some(ty) = return_type_ident(&i.sig.output) {
            self.out.insert(i.sig.ident.to_string(), ty);
        }
        syn::visit::visit_item_fn(self, i);
    }
}

/// The producer's-return-value half of the fn-A/fn-B gap (the riir-reflex
/// `FusedGateRecommendation` specimen, 2026-09-23): a TEST fn binds a local
/// from a producer call (`let fused = recommend_fused_gate(..);`) and
/// serializes THAT local (`serde_json::to_string(&fused)`) — the struct
/// literal lives in the producer, the serializing fn receives no param, so
/// neither fn-scoped co-occurrence nor the concrete-param link can join
/// them. Resolution: same-file free-fn return types ([`FnRetFinder`]),
/// conservative at every seam — bare single-segment callees only (`Foo::new`
/// and method calls are unknown here and refuse), uniquely-bound locals only
/// (a shadowed name refuses entirely), and bare single-ident serialized args
/// only (`&fused`, `(&fused)`, `fused` — an expression like `&fused.field`
/// proves nothing about the whole value's type path).
fn serialized_return_value_types(
    block: &syn::Block,
    fn_rets: &BTreeMap<String, String>,
) -> Vec<String> {
    struct F {
        bound_from: BTreeMap<String, String>,
        bind_count: BTreeMap<String, usize>,
        serialized: BTreeSet<String>,
    }
    impl F {
        /// Peel `&x` / `(x)` / group wrappers off a to_* argument; a bare
        /// single-segment path yields the local name.
        fn bare_ident_arg(expr: &syn::Expr) -> Option<String> {
            let mut e = expr;
            loop {
                match e {
                    syn::Expr::Reference(r) => e = &r.expr,
                    syn::Expr::Paren(p) => e = &p.expr,
                    syn::Expr::Group(g) => e = &g.expr,
                    _ => break,
                }
            }
            if let syn::Expr::Path(p) = e {
                if p.path.segments.len() == 1 {
                    return Some(p.path.segments[0].ident.to_string());
                }
            }
            None
        }
    }
    impl<'ast> syn::visit::Visit<'ast> for F {
        fn visit_local(&mut self, l: &'ast syn::Local) {
            if let syn::Pat::Ident(pi) = &l.pat {
                let name = pi.ident.to_string();
                *self.bind_count.entry(name.clone()).or_insert(0) += 1;
                if let Some(init) = &l.init {
                    if let syn::Expr::Call(c) = &*init.expr {
                        if let syn::Expr::Path(p) = &*c.func {
                            if p.path.segments.len() == 1 {
                                self.bound_from
                                    .insert(name, p.path.segments[0].ident.to_string());
                            }
                        }
                    }
                }
            }
            syn::visit::visit_local(self, l);
        }

        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if let syn::Expr::Path(p) = &*c.func {
                if is_serde_to_path(&p.path) {
                    for arg in &c.args {
                        if let Some(name) = Self::bare_ident_arg(arg) {
                            self.serialized.insert(name);
                        }
                    }
                }
            }
            syn::visit::visit_expr_call(self, c);
        }
    }
    let mut f = F {
        bound_from: BTreeMap::new(),
        bind_count: BTreeMap::new(),
        serialized: BTreeSet::new(),
    };
    syn::visit::visit_block(&mut f, block);
    let mut out = Vec::new();
    for (name, callee) in &f.bound_from {
        if f.bind_count.get(name).copied().unwrap_or(0) != 1 || !f.serialized.contains(name) {
            continue;
        }
        if let Some(ty) = fn_rets.get(callee) {
            out.push(ty.clone());
        }
    }
    out.sort();
    out.dedup();
    out.shrink_to_fit();
    out
}

/// Does the expression mention any of the given names as a bare path
/// (`row`, `row.clone()`'s receiver — not `other::row`)?
fn expr_mentions_ident(expr: &syn::Expr, names: &[String]) -> bool {
    let mut f = NameMentionFinder { names, hit: false };
    syn::visit::visit_expr(&mut f, expr);
    f.hit
}

/// Does the type mention any of the given bare idents (`&T`, `T`, `Vec<T>`)?
fn type_mentions_ident(ty: &syn::Type, names: &[String]) -> bool {
    let mut f = NameMentionFinder { names, hit: false };
    syn::visit::visit_type(&mut f, ty);
    f.hit
}

/// The shared bare-ident mention finder (`row` yes, `other::row` no).
struct NameMentionFinder<'a> {
    names: &'a [String],
    hit: bool,
}

impl<'ast> syn::visit::Visit<'ast> for NameMentionFinder<'_> {
    fn visit_path(&mut self, p: &'ast syn::Path) {
        if p.segments.len() == 1 && self.names.iter().any(|n| p.segments[0].ident == *n) {
            self.hit = true;
        }
        syn::visit::visit_path(self, p);
    }
}

/// Pass-1 visitor: collects the names of GENERIC SERDE BRIDGE fns.
struct BridgeFinder<'a> {
    out: &'a mut HashSet<String>,
}

impl<'ast> syn::visit::Visit<'ast> for BridgeFinder<'_> {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if is_generic_serde_bridge(i) {
            self.out.insert(i.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, i);
    }
}

/// Collects every BARE type name in generic-argument position inside the
/// visited expression — the concrete `T`s a bridge call site names
/// (`save_t::<CraftRow>(m)`, `row_to_mapping(&Repo::<CraftRow>::get(..))`).
/// Multi-segment paths (`serde::Serialize`) and wrapper heads (`Vec` in
/// `Vec<CraftRow>`) insert nothing themselves — but their ARGUMENTS still
/// fire this visitor recursively, so `Vec<CraftRow>` yields `CraftRow` (and
/// the harmless `Vec`).
struct TurbofishTypes<'a> {
    out: &'a mut HashSet<String>,
}

impl<'ast> syn::visit::Visit<'ast> for TurbofishTypes<'_> {
    fn visit_generic_argument(&mut self, g: &'ast syn::GenericArgument) {
        if let syn::GenericArgument::Type(syn::Type::Path(tp)) = g {
            if tp.path.segments.len() == 1 {
                self.out.insert(tp.path.segments[0].ident.to_string());
            }
        }
        syn::visit::visit_generic_argument(self, g);
    }
}

/// Run the wire-consumption analysis over the same source set the scan ran
/// over. Deterministic (set/iteration order does not affect membership).
/// Two passes (parse once): pass 1 collects generic serde bridge fn names —
/// before any literal/call walk, so a bridge defined after its caller in
/// the file order still marks; pass 2 is the literal walk with per-file
/// scope resolution plus the bridge call-site turbofish harvest.
#[must_use]
pub fn wire_consumption(sources: &[(String, String)]) -> WireConsumption {
    let parsed: Vec<(String, syn::File)> = sources
        .iter()
        .filter_map(|(name, src)| syn::parse_file(src).ok().map(|f| (name.clone(), f)))
        .collect();

    let mut bridge_fns = HashSet::new();
    for (_name, file) in &parsed {
        let mut f = BridgeFinder {
            out: &mut bridge_fns,
        };
        syn::visit::visit_file(&mut f, file);
    }

    let local_fields = collect_struct_field_sets(&parsed);
    // Pass 1b: per-file free-fn return types — the same-file resolution map
    // for the serialized-return-value link (a bridge-style pre-pass, so a
    // test defined BEFORE its producer in the file still resolves).
    let mut file_fn_rets: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (name, file) in &parsed {
        let mut f = FnRetFinder {
            out: BTreeMap::new(),
        };
        syn::visit::visit_file(&mut f, file);
        file_fn_rets.insert(name.clone(), f.out);
    }
    let mut v = WireVisitor {
        bridge_fns,
        local_fields,
        ..WireVisitor::default()
    };
    let local_crates = crate::config_audit::scan_local_crate_roots(sources);
    for (name, file) in &parsed {
        v.scope = FileScope::of(file, &local_crates);
        v.file_is_test = crate::config_audit::file_is_test_path(name)
            || crate::config_audit::attrs_cfg_test(&file.attrs);
        v.fn_rets = file_fn_rets.remove(name).unwrap_or_default();
        syn::visit::visit_file(&mut v, file);
    }

    // Transitive closure of the wrapping clause: a wire struct's Vec<T>/
    // Option<T> field consumes T; T's own wrapped fields consume further
    // (fixpoint — the graph is tiny). Bridge types deliberately do NOT seed
    // this closure: the per-field const classification (wire-const vs
    // cleared) is direct-serde-fn evidence only, and a bridge type's
    // stillborn rows clear at the STRUCT level in the classifier instead.
    let mut wire_types = v.direct_wire_types.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for (wrapper, inner) in &v.wrap_edges {
            if wire_types.contains(wrapper) && wire_types.insert(inner.clone()) {
                changed = true;
            }
        }
    }

    let mut out = WireConsumption {
        bridge_types: v.bridge_types,
        ..WireConsumption::default()
    };
    for ((ty, field), entries) in &v.sites {
        if !wire_types.contains(ty) {
            continue;
        }
        // Test-context sites are fixture evidence, not schema evidence —
        // a protocol-mirror crate's tests construct every wire type with
        // literal constants BY DESIGN while the production constructors
        // live outside the scanned set (the cross-repo hint already
        // discloses downstream constructors are invisible). With ZERO
        // non-test sites the (type, field) renders NOTHING: the class's
        // premise ("serialized-but-constant = schema-dead weight the
        // receiver cannot be using") is unsupported by test-only
        // evidence (the riir-kat FP class, 2026-09-21).
        let non_test: Vec<&SiteEntry> = entries.iter().filter(|e| !e.in_test).collect();
        if non_test.is_empty() {
            continue;
        }
        if non_test.iter().all(|e| e.is_const) {
            out.wire_const.push((ty.clone(), field.clone()));
        } else {
            out.cleared.insert((ty.clone(), field.clone()));
        }
    }
    out.wire_const.sort();
    out
}

/// Scan `paths` (dirs recursively / files) and run both legs. Probe depths
/// default to [`DEFAULT_PROBE_DEPTHS`].
#[must_use]
pub fn report_paths(paths: &[PathBuf], probe_depths: &[usize]) -> OrphanReport {
    let walked = crate::config_audit::read_sources_walk(paths);
    if walked.sources.is_empty() {
        return OrphanReport {
            blind: Some("no .rs files parsed under the scanned paths".to_string()),
            walk: walked.mode,
            ..OrphanReport::default()
        };
    }
    let scan = crate::config_audit::zero_external_read_fields(&walked.sources);
    let wire = wire_consumption(&walked.sources);
    match GitCli::new(paths) {
        None => OrphanReport {
            files_scanned: scan.files_scanned,
            candidates: scan.candidates,
            zero_read_fields: scan.fields.len(),
            walk: walked.mode,
            blind: Some(
                "not a resolvable git repository (or git missing from PATH) — the C1 history leg is unavailable"
                    .to_string(),
            ),
            ..OrphanReport::default()
        },
        Some(git) => {
            let mut report = classify_with_probe(&scan, &git, &wire, probe_depths);
            report.walk = walked.mode;
            report
        }
    }
}

/// The classifier over an already-run current-state scan — the unit-test
/// entry (drive it with a fake [`HistoryProbe`]).
#[must_use]
pub fn classify_with_probe(
    scan: &ZeroReadScan,
    probe: &dyn HistoryProbe,
    wire: &WireConsumption,
    depths: &[usize],
) -> OrphanReport {
    let mut probe_depths: Vec<usize> = Vec::new();
    for d in depths.iter().copied().chain(std::iter::once(0)) {
        if probe.rev_at_depth(d).is_some() && !probe_depths.contains(&d) {
            probe_depths.push(d);
        }
    }
    probe_depths.sort_unstable();

    let mut orphaned: Vec<OrphanRow> = Vec::new();
    let mut stillborn: Vec<ZeroReadField> = Vec::new();
    let mut wire_const: Vec<ZeroReadField> = Vec::new();
    let mut documented_write_only: Vec<ZeroReadField> = Vec::new();
    let mut wire_cleared = 0usize;
    let mut bridge_cleared = 0usize;
    for field in &scan.fields {
        let name = &field.candidate.field_name;
        let pattern = read_shape_ere(name);
        let mut probes: Vec<(String, usize)> = Vec::with_capacity(probe_depths.len());
        let mut head_count = 0;
        let mut max_other = 0;
        let mut any_count = 0;
        for d in &probe_depths {
            let rev = if *d == 0 {
                "HEAD".to_string()
            } else {
                format!("HEAD~{d}")
            };
            let count = probe.count_ere(&rev, &pattern).unwrap_or(0);
            any_count += count;
            if *d == 0 {
                head_count = count;
            } else {
                max_other = max_other.max(count);
            }
            probes.push((rev, count));
        }
        let wire_key = (
            field.candidate.struct_name.clone(),
            field.candidate.field_name.clone(),
        );
        let bridge = wire.bridge_types.contains(&field.candidate.struct_name);
        // The P26 downgrade: a field-doc deliberate-write-only marker moves
        // a FINDING row to the weak annotation. Checked before the orphaned
        // branch so the marked row skips the pickaxe leg (the annotation
        // carries no commit evidence); consumed rows (wire-cleared,
        // wire-const, bridge-cleared) outrank the annotation.
        let marked = scan.marked_write_only.contains(&wire_key);
        if max_other > head_count {
            // ORPHANED rows keep their independent history evidence — the
            // wire pass never clears them (it only repairs the stillborn
            // class the 8-row probe measured). A bridge-consumed struct's
            // row keeps the evidence too, at wire-dto weak severity: the
            // receiver still consumes the serialized field.
            if marked {
                documented_write_only.push(field.clone());
                continue;
            }
            let commits = probe.pickaxe(&pattern, PICKAXE_MAX);
            let likely_by_optimization = commits
                .first()
                .is_some_and(|c| subject_looks_like_optimization(&c.subject));
            orphaned.push(OrphanRow {
                candidate: field.candidate.clone(),
                total_reads: field.total_reads,
                test_reads: field.test_reads,
                self_impl_reads: field.self_impl_reads,
                probes,
                commits,
                likely_by_optimization,
                bridge_weak: bridge,
            });
        } else if any_count == 0 {
            if wire.cleared.contains(&wire_key) {
                wire_cleared += 1;
            } else if wire.wire_const.iter().any(|k| k == &wire_key) {
                wire_const.push(field.clone());
            } else if bridge {
                bridge_cleared += 1;
            } else if marked {
                documented_write_only.push(field.clone());
            } else {
                stillborn.push(field.clone());
            }
        } else if marked {
            // The zero-read row carries reads at HEAD (grep counts them —
            // the reads are usually the field's own TEST pins), but the
            // field doc carries the owner's deliberate-write-only marker:
            // render the weak annotation instead of the count-only silent
            // bucket. The chain GuardVerdict.redirect/backoff_ticks class —
            // marker'd forward surfaces whose only readers are the
            // behavioral tests that pin them — was invisible here before
            // (the P26 check reached only the stillborn/orphaned branches).
            // The annotation is downgrade-not-suppress, so surfacing a
            // marked row is never a false positive by construction: the
            // owner's own doc wording is the trigger.
            documented_write_only.push(field.clone());
        }
        // else: read-shaped occurrences exist now with no vanished evidence
        // — silent (A/B/C2 territory), counted in the header only.
    }
    OrphanReport {
        files_scanned: scan.files_scanned,
        candidates: scan.candidates,
        zero_read_fields: scan.fields.len(),
        // classify_with_probe is the in-memory unit-test entry — the walk
        // happened elsewhere (the caller stamps the real mode).
        walk: WalkMode::Plain,
        orphaned,
        stillborn,
        wire_cleared,
        wire_const,
        documented_write_only,
        bridge_cleared,
        probe_depths,
        blind: None,
    }
}

impl OrphanReport {
    /// Render the human report (what `--orphan-report` prints).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{}", self.walk.note());
        if let Some(reason) = &self.blind {
            let _ = writeln!(out, "orphan-report: BLIND — {reason}");
            let _ = writeln!(
                out,
                "scope: refusing to report findings over a blind history leg (the frontier-report law)"
            );
            return out;
        }
        let probe_labels: Vec<String> = self
            .probe_depths
            .iter()
            .map(|d| {
                if *d == 0 {
                    "HEAD".to_string()
                } else {
                    format!("HEAD~{d}")
                }
            })
            .collect();
        let bridge_weak_count = self.orphaned.iter().filter(|r| r.bridge_weak).count();
        let silent = self.zero_read_fields
            - self.orphaned.len()
            - self.stillborn.len()
            - self.wire_cleared
            - self.wire_const.len()
            - self.documented_write_only.len()
            - self.bridge_cleared;
        let _ = writeln!(
            out,
            "orphan-report: {} file(s) parsed, {} Default-impl field candidate(s), {} zero-external-read field(s): {} orphaned [C1] ({} wire-dto weak), {} stillborn, {} wire-cleared (serde co-occurrence), {} bridge-cleared (generic serde bridge), {} wire-const, {} documented-write-only (field-doc marker), {} silent (no vanished-reader evidence)",
            self.files_scanned,
            self.candidates,
            self.zero_read_fields,
            self.orphaned.len(),
            bridge_weak_count,
            self.stillborn.len(),
            self.wire_cleared,
            self.bridge_cleared,
            self.wire_const.len(),
            self.documented_write_only.len(),
            silent
        );
        let _ = writeln!(
            out,
            "history probes: {} (clamped to repository depth)",
            probe_labels.join(", ")
        );
        for row in &self.orphaned {
            write_orphan_row(&mut out, row);
        }
        for s in &self.stillborn {
            let _ = writeln!(
                out,
                "  [stillborn] {} — {}.{} (0 read-shaped occurrences at HEAD and every probe — never read within the window)",
                s.candidate.origin, s.candidate.struct_name, s.candidate.field_name
            );
        }
        for s in &self.wire_const {
            let _ = writeln!(
                out,
                "  [wire-const] {} — {}.{} (serialized via a serde-fn literal but CONSTANT at every construction site — schema-dead weight the receiver cannot be using; weak severity)",
                s.candidate.origin, s.candidate.struct_name, s.candidate.field_name
            );
        }
        for s in &self.documented_write_only {
            let _ = writeln!(
                out,
                "  [documented-write-only] {} — {}.{} (the FIELD doc carries a deliberate-write-only marker — weak severity, the doc's claim is NOT verified; if it is not deliberate, drop the marker wording)",
                s.candidate.origin, s.candidate.struct_name, s.candidate.field_name
            );
        }
        let _ = writeln!(
            out,
            "scope: history greps are name-global (untyped) — a same-named field on another type can masquerade; comment mentions count (syn excludes them, grep cannot); readers outside the scanned set are invisible; a reader MOVED (not removed) can drop the count — adjudicate every row against the listed commits; #[serde(skip_serializing_if)] fields are excluded (output-schema surface, derive-read — the Issue 934 boundary); wire-clearance is fn-scoped co-occurrence (serde_json::to_*/serde_yaml::to_* path calls + literal same-fn, or a Vec/Option wrapper field of a wire literal, OR a sibling-fn link: a fn that serde-serializes its own CONCRETE-typed param consumes that param's type — the literal sits in the caller, the riir-kat queue-writer shape — OR a crate-rooted serde_json::json! macro whose tokens mention the param: the macro IS a serializer, each interpolated arg goes through to_value at runtime, the riir-kat shop_seed_body shape — OR a same-file fn's RETURN VALUE bound into a local the serializing fn serializes (the riir-reflex test-serializes-the-producer shape: `let fused = recommend(..)` + `to_string(&fused)` in the test, literal in the producer; same-file free-fn resolution, unique-binding required)) and applies to stillborn rows only; struct-literal attribution is per-file scope-resolved (a literal naming a type the file imports from a foreign path, or reachable only via a foreign glob with no local decl/import, is foreign evidence — skipped); a literal naming a field that NO local same-named top-level struct declares is skipped as provably foreign too — the foreign-glob + local-glob import shape is undecidable, the field sets are not (Issue 109 re-sight); generic serde bridges (Serialize-bounded generic param + serde to_* on it) mark their call sites' turbofish-named types bridge-consumed — stillborn rows there count as bridge-cleared, orphaned rows render wire-dto weak; a zero-external-read row whose FIELD doc carries a deliberate-write-only marker (documented-write-only | deliberate | forward surface | GOAT-gate territory — the config-audit vocabulary) renders at the weak [documented-write-only] annotation in every bucket (stillborn, orphaned, AND the silent arm whose reads at HEAD are the field's own test pins — the riir-chain GuardVerdict class); downgrade-not-suppress — the doc's claim is not verified; consumed rows outrank it, wire-const is a different class and stays)"
        );
        out
    }
}

/// One [C1] row's evidence block (extracted from [`OrphanReport::render`] —
/// the whole-render fn trips `too_many_lines`).
fn write_orphan_row(out: &mut String, row: &OrphanRow) {
    let evidence: Vec<String> = row
        .probes
        .iter()
        .map(|(rev, c)| format!("{rev} {c}"))
        .collect();
    let _ = writeln!(
        out,
        "  [C1] {} — {}.{}: read-shaped occurrences VANISHED ({}); syn reads {} total / {} test / {} own-impl → external 0",
        row.candidate.origin,
        row.candidate.struct_name,
        row.candidate.field_name,
        evidence.join("; "),
        row.total_reads,
        row.test_reads,
        row.self_impl_reads
    );
    if row.commits.is_empty() {
        let _ = writeln!(
            out,
            "       no pickaxe attribution within the scanned paths (the removal predates the walk or sat outside it)"
        );
    } else {
        let _ = writeln!(out, "       occurrence-count changes (newest first):");
        for c in row.commits.iter().take(3) {
            let _ = writeln!(out, "         {} {} {}", c.hash, c.date, c.subject);
        }
        if row.commits.len() > 3 {
            let _ = writeln!(
                out,
                "         … {} more (see git log)",
                row.commits.len() - 3
            );
        }
    }
    if row.bridge_weak {
        let _ = writeln!(
            out,
            "       verdict: ORPHANED-BY-TIME — wire-dto WEAK (a generic serde bridge serializes this struct onward; the receiver consumes the field over the wire — the in-repo read loss is evidence, not a death verdict)"
        );
    } else if row.likely_by_optimization {
        let _ = writeln!(
            out,
            "       verdict: ORPHANED-BY-TIME — newest count-change subject matches the optimization vocabulary → LIKELY-BY-OPTIMIZATION"
        );
    } else {
        let _ = writeln!(out, "       verdict: ORPHANED-BY-TIME");
    }
    let _ = writeln!(
        out,
        "       fix: re-wire the consumer the change replaced, or delete the knob"
    );
}

#[cfg(test)]
#[path = "orphan_report_tests.rs"]
mod tests;
