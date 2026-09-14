//! `cargo heal --orphan-report` — post-mining intake class C1: the config
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
//! a `Vec<T>`/`Option<T>` field of a literal-constructed wire struct marks
//! T consumed transitively (**the wrapping clause** — the
//! `SelfReportWire { targets: Vec<ReportTarget> }` shape, where the inner
//! type's literals live only in tests). Measured on the 8 pinned rows:
//! swallow rate 1/8 — the sole swallow is a field hardcoded `true` at its
//! only site, which is exactly why constant-valued fields are **NOT
//! cleared** (the carve-out): a field whose EVERY construction site is a
//! literal constant is schema-dead weight the receiver cannot be using —
//! reported at the weak `[wire-const]` severity instead. Clearance applies
//! to stillborn rows only (the measured FP class); ORPHANED rows keep their
//! independent history evidence. Limits: fn-scoped approximation (a
//! literal built in fn A and serialized in fn B is not cleared — the
//! conservative direction); path-call form only (aliased imports invisible);
//! type names are file-global (no module resolution, same as the read
//! greps); the value classification sees literal sites only (a field never
//! set at any visible literal is treated as cleared — its value comes from
//! deserialization or defaults, and the wire IS the consumption).

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::config_audit::{FieldCandidate, WalkMode, ZeroReadField, ZeroReadScan};

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
    /// fn that also makes a `serde_json::to_*`/`serde_yaml::to_*` call, AND
    /// the field carries a non-literal-constant value at some construction
    /// site. Stillborn rows matching here are cleared.
    pub cleared: HashSet<(String, String)>,
    /// Wire-consumed but literal-constant (`true`/`5`/`"x"`, incl. negated
    /// numeric literals) at EVERY construction site — schema-dead weight
    /// the receiver cannot be using; reported at the weak `[wire-const]`
    /// severity, never cleared (the 1/8 measured swallow, kept visible).
    pub wire_const: Vec<(String, String)>,
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

/// One construction-site entry: the field was set at an `ExprStruct` of the
/// (file-global) type name, with a literal-constant value or not.
struct SiteEntry {
    is_const: bool,
}

/// The wire-pass visitor. Walks every file once, tracking (a) whether we are
/// inside a fn that serializes, (b) the enclosing impl's own type (to resolve
/// `Self` literals), (c) every struct-literal field assignment.
#[derive(Default)]
struct WireVisitor {
    in_serde_fn: bool,
    impl_ty: Option<String>,
    /// Type names literal-constructed inside a serde fn (direct evidence).
    direct_wire_types: HashSet<String>,
    /// (type, field) → per-site const classification, ALL contexts.
    sites: std::collections::HashMap<(String, String), Vec<SiteEntry>>,
    /// Struct-def edges: wrapper struct → inner type of a `Vec<T>`/`Option<T>` field.
    wrap_edges: HashSet<(String, String)>,
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

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let saved = self.in_serde_fn;
        if block_has_serde_call(&i.block) {
            self.in_serde_fn = true;
        }
        syn::visit::visit_item_fn(self, i);
        self.in_serde_fn = saved;
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
                });
        }
        syn::visit::visit_expr_struct(self, e);
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
    }
    let mut f = Finder(false);
    syn::visit::visit_block(&mut f, block);
    f.0
}

/// Run the wire-consumption analysis over the same source set the scan ran
/// over. Deterministic (set/iteration order does not affect membership).
#[must_use]
pub fn wire_consumption(sources: &[(String, String)]) -> WireConsumption {
    let mut v = WireVisitor::default();
    for (_name, src) in sources {
        if let Ok(file) = syn::parse_file(src) {
            syn::visit::visit_file(&mut v, &file);
        }
    }

    // Transitive closure of the wrapping clause: a wire struct's Vec<T>/
    // Option<T> field consumes T; T's own wrapped fields consume further
    // (fixpoint — the graph is tiny).
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

    let mut out = WireConsumption::default();
    for ((ty, field), entries) in &v.sites {
        if !wire_types.contains(ty) {
            continue;
        }
        if entries.iter().all(|e| e.is_const) {
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
    let mut wire_cleared = 0usize;
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
        if max_other > head_count {
            // ORPHANED rows keep their independent history evidence — the
            // wire pass never clears them (it only repairs the stillborn
            // class the 8-row probe measured).
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
            });
        } else if any_count == 0 {
            if wire.cleared.contains(&wire_key) {
                wire_cleared += 1;
            } else if wire.wire_const.iter().any(|k| k == &wire_key) {
                wire_const.push(field.clone());
            } else {
                stillborn.push(field.clone());
            }
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
        let silent = self.zero_read_fields
            - self.orphaned.len()
            - self.stillborn.len()
            - self.wire_cleared
            - self.wire_const.len();
        let _ = writeln!(
            out,
            "orphan-report: {} file(s) parsed, {} Default-impl field candidate(s), {} zero-external-read field(s): {} orphaned [C1], {} stillborn, {} wire-cleared (serde co-occurrence), {} wire-const, {} silent (no vanished-reader evidence)",
            self.files_scanned,
            self.candidates,
            self.zero_read_fields,
            self.orphaned.len(),
            self.stillborn.len(),
            self.wire_cleared,
            self.wire_const.len(),
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
        let _ = writeln!(
            out,
            "scope: history greps are name-global (untyped) — a same-named field on another type can masquerade; comment mentions count (syn excludes them, grep cannot); readers outside the scanned set are invisible; a reader MOVED (not removed) can drop the count — adjudicate every row against the listed commits; #[serde(skip_serializing_if)] fields are excluded (output-schema surface, derive-read — the Issue 934 boundary); wire-clearance is fn-scoped co-occurrence (serde_json::to_*/serde_yaml::to_* path calls + literal same-fn, or a Vec/Option wrapper field of a wire literal — built-in-fn-A/serialized-in-fn-B literals stay flagged, the conservative direction) and applies to stillborn rows only"
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
    if row.likely_by_optimization {
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
mod tests {
    use super::*;
    use std::collections::BTreeMap as Map;

    /// Pattern → rev → line count.
    type FakeCounts = Map<String, Map<String, usize>>;
    /// Pattern → newest-first commit hits.
    type FakeCommits = Map<String, Vec<CommitHit>>;

    struct FakeHistory {
        resolvable: Vec<String>,
        counts: FakeCounts,
        commits: FakeCommits,
    }

    impl FakeHistory {
        fn new(depths: &[usize]) -> Self {
            Self {
                resolvable: std::iter::once("HEAD".to_string())
                    .chain(depths.iter().map(|d| format!("HEAD~{d}")))
                    .collect(),
                counts: Map::new(),
                commits: Map::new(),
            }
        }

        fn count(mut self, field: &str, rev: &str, n: usize) -> Self {
            self.counts
                .entry(read_shape_ere(field))
                .or_default()
                .insert(rev.to_string(), n);
            self
        }

        fn commit(mut self, field: &str, subject: &str) -> Self {
            self.commits
                .entry(read_shape_ere(field))
                .or_default()
                .push(CommitHit {
                    hash: format!("{:07x}", subject.len()),
                    date: "2026-09-01".to_string(),
                    subject: subject.to_string(),
                });
            self
        }
    }

    impl HistoryProbe for FakeHistory {
        fn rev_at_depth(&self, depth: usize) -> Option<String> {
            let rev = if depth == 0 {
                "HEAD".to_string()
            } else {
                format!("HEAD~{depth}")
            };
            self.resolvable.contains(&rev).then_some(rev)
        }

        fn count_ere(&self, rev: &str, pattern: &str) -> Option<usize> {
            self.counts.get(pattern)?.get(rev).copied()
        }

        fn pickaxe(&self, pattern: &str, max: usize) -> Vec<CommitHit> {
            self.commits
                .get(pattern)
                .map(|c| c.iter().take(max).cloned().collect())
                .unwrap_or_default()
        }
    }

    const DEPTHS: [usize; 2] = [10, 50];

    fn scan_of(src: &str) -> ZeroReadScan {
        crate::config_audit::zero_external_read_fields(&[("lib.rs".to_string(), src.to_string())])
    }

    const ORPHANED_SRC: &str = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
";

    #[test]
    fn orphaned_fires_with_optimization_tag() {
        let scan = scan_of(ORPHANED_SRC);
        assert_eq!(scan.fields.len(), 1, "alarm_radius is zero-external-read");
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 0)
            .count("alarm_radius", "HEAD~10", 0)
            .count("alarm_radius", "HEAD~50", 3)
            .commit("alarm_radius", "perf: const-table species behavior values");
        let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
        assert_eq!(report.orphaned.len(), 1, "the vanished reader fires");
        assert!(report.stillborn.is_empty());
        let row = &report.orphaned[0];
        assert!(row.likely_by_optimization, "perf subject tags the row");
        let rendered = report.render();
        assert!(rendered.contains("[C1]"), "{rendered}");
        assert!(rendered.contains("LIKELY-BY-OPTIMIZATION"), "{rendered}");
        assert!(rendered.contains("alarm_radius"), "{rendered}");
    }

    #[test]
    fn plain_subject_orphans_without_the_tag() {
        let scan = scan_of(ORPHANED_SRC);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 0)
            .count("alarm_radius", "HEAD~50", 1)
            .commit("alarm_radius", "docs: update config reference");
        let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
        assert_eq!(report.orphaned.len(), 1);
        assert!(!report.orphaned[0].likely_by_optimization);
    }

    #[test]
    fn never_read_is_stillborn_not_orphaned() {
        let scan = scan_of(ORPHANED_SRC);
        let fake = FakeHistory::new(&DEPTHS); // no counts anywhere
        let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
        assert!(report.orphaned.is_empty(), "no delta, no orphan");
        assert_eq!(report.stillborn.len(), 1, "never read → stillborn");
        assert!(report.render().contains("[stillborn]"));
    }

    #[test]
    fn own_impl_reads_with_no_delta_stay_silent() {
        // validate() reads self.alarm_radius NOW; the same read existed at
        // every probe — delta 0, not stillborn (head count > 0) → silent.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
impl BehaviorConfig {
    pub fn validate(&self) -> bool { self.alarm_radius > 0.0 }
}
";
        let scan = scan_of(src);
        assert_eq!(scan.fields.len(), 1);
        assert_eq!(scan.fields[0].self_impl_reads, 1);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 1)
            .count("alarm_radius", "HEAD~10", 1)
            .count("alarm_radius", "HEAD~50", 1);
        let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
        assert!(
            report.orphaned.is_empty(),
            "no delta — validate then and now"
        );
        assert!(report.stillborn.is_empty(), "head count > 0");
        let rendered = report.render();
        assert!(rendered.contains("1 silent"), "{rendered}");
    }

    #[test]
    fn external_reader_removes_the_field_from_the_candidate_set() {
        // The FN direction of name-global matching: any read of the name
        // (here a real external one) keeps external > 0 — no candidate.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
pub fn alarm_of(c: &BehaviorConfig) -> f32 { c.alarm_radius }
";
        let scan = scan_of(src);
        assert!(scan.fields.is_empty(), "external read keeps it alive");
    }

    #[test]
    fn reader_plus_validate_delta_fires() {
        // The canonical shape: consumer read existed then, only validate
        // remains now → HEAD 1, probe 2 → delta fires.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
impl BehaviorConfig {
    pub fn validate(&self) -> bool { self.alarm_radius > 0.0 }
}
";
        let scan = scan_of(src);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 1)
            .count("alarm_radius", "HEAD~10", 2)
            .count("alarm_radius", "HEAD~50", 2)
            .commit("alarm_radius", "perf: inline the species table");
        let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
        assert_eq!(
            report.orphaned.len(),
            1,
            "consumer vanished, validate stayed"
        );
        assert!(report.orphaned[0].likely_by_optimization);
    }

    #[test]
    fn same_name_on_another_type_masks_the_candidate() {
        // The documented FN direction: another struct's same-named field is
        // read — the syn tally is name-global, so BOTH look read.
        let src = r"
pub struct BehaviorConfig { pub alarm_radius: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0 } }
}
pub struct OtherConfig { pub alarm_radius: f32 }
pub fn other(o: &OtherConfig) -> f32 { o.alarm_radius }
";
        let scan = scan_of(src);
        assert!(
            scan.fields
                .iter()
                .all(|f| f.candidate.field_name != "alarm_radius"),
            "name-global read masks the orphan candidate"
        );
    }

    #[test]
    fn read_shape_ere_binds_the_tail() {
        let p = read_shape_ere("alarm_radius");
        assert_eq!(p, r"\.alarm_radius([^[:alnum:]_]|$)");
        // `.alarm_radius_longer` must NOT match the pattern as a whole word…
        // (the ERE would need a backreference to assert that; the tail class
        // is the portable approximation — pinned by the e2e test below.)
    }

    #[test]
    fn subject_vocabulary_is_case_insensitive() {
        assert!(subject_looks_like_optimization("PERF: const table"));
        assert!(subject_looks_like_optimization("Refactor the boot path"));
        assert!(!subject_looks_like_optimization("docs: readme"));
    }

    /// The canonical C1 shape (Plan 126's own control): a serde-backed
    /// designer config with NO `impl Default`, constructed by handwritten
    /// literals, its field never member-read — the field MUST enter the
    /// candidate set via the serde-literal extension.
    const SERDE_LITERAL_SRC: &str = r#"
#[derive(serde::Deserialize)]
pub struct BehaviorConfig { #[serde(default = "default_alarm_radius")] pub alarm_radius: f32 }
fn default_alarm_radius() -> f32 { 15.0 }
pub fn hardcoded_registry() -> BehaviorConfig {
    BehaviorConfig { alarm_radius: 30.0 }
}
"#;

    #[test]
    fn serde_literal_constructor_is_candidate() {
        let scan = scan_of(SERDE_LITERAL_SRC);
        assert_eq!(
            scan.fields.len(),
            1,
            "the serde-literal field enters the set"
        );
        assert_eq!(scan.fields[0].candidate.field_name, "alarm_radius");
        assert_eq!(scan.fields[0].candidate.struct_name, "BehaviorConfig");
        // And with no history evidence it reads stillborn (the canonical
        // case's honest verdict — the pre-fix tree has zero read-shaped
        // occurrences at every commit).
        let fake = FakeHistory::new(&DEPTHS);
        let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
        assert_eq!(report.stillborn.len(), 1);
        assert!(report.orphaned.is_empty());
    }

    #[test]
    fn serde_literal_orphaned_when_a_reader_vanished() {
        let scan = scan_of(SERDE_LITERAL_SRC);
        let fake = FakeHistory::new(&DEPTHS)
            .count("alarm_radius", "HEAD", 0)
            .count("alarm_radius", "HEAD~50", 2)
            .commit("alarm_radius", "perf: inline species table");
        let report = classify_with_probe(&scan, &fake, &WireConsumption::default(), &DEPTHS);
        assert_eq!(
            report.orphaned.len(),
            1,
            "the vanished reader fires on the serde-literal candidate"
        );
        assert!(report.orphaned[0].likely_by_optimization);
    }

    #[test]
    fn serialize_only_struct_is_not_a_candidate() {
        // Serialize-only: the derive reads the fields — every one would
        // read as stillborn. Deliberately outside the extension.
        let src = r"
#[derive(serde::Serialize)]
pub struct WireOut { pub seq: u32 }
pub fn emit() -> WireOut { WireOut { seq: 1 } }
";
        let scan = scan_of(src);
        assert!(
            scan.fields.is_empty(),
            "Serialize-only structs are excluded"
        );
    }

    /// The Issue 934 regression pin (the `ConversationEntry` false
    /// positive): a Ser+De struct whose field carries
    /// `#[serde(skip_serializing_if)]` is conditionally-emitted OUTPUT
    /// SCHEMA — the Serialize derive reads it when the condition holds,
    /// invisibly to the member-read tally. Never a C1 candidate.
    #[test]
    fn skip_serializing_if_field_is_not_a_candidate() {
        let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConversationEntry {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_tick: Option<u64>,
}
pub fn make() -> ConversationEntry {
    ConversationEntry { state: "Failed".into(), deadline_tick: None }
}
pub fn state_of(e: &ConversationEntry) -> &str { &e.state }
"#;
        let scan = scan_of(src);
        assert!(
            scan.fields.is_empty(),
            "skip_serializing_if fields are excluded (derive-read schema)"
        );
    }

    /// The FN guard on the same boundary (the `BehaviorConfig` canonical
    /// family): a plain `#[serde(default)]` field in a struct deriving BOTH
    /// Serialize AND Deserialize STAYS a candidate — persistence-mirror
    /// surface whose functional read is exactly what the report measures.
    /// The Issue 934 sample: every skip-attrs field was an FP, every
    /// plain-default field in the same derive shape a TP — the boundary is
    /// the attr, NOT the derive pair.
    #[test]
    fn plain_serde_default_in_ser_de_struct_stays_candidate() {
        let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BehaviorConfig {
    #[serde(default = "default_alarm_radius")]
    pub alarm_radius: f32,
}
pub fn make() -> BehaviorConfig { BehaviorConfig { alarm_radius: 15.0 } }
"#;
        let scan = scan_of(src);
        assert_eq!(
            scan.fields.len(),
            1,
            "plain serde(default) fields stay candidates"
        );
        assert_eq!(scan.fields[0].candidate.field_name, "alarm_radius");
    }

    // ── Wire-consumption fixtures (the 2026-09-12 measured WIRE-FP repair;
    // shapes pin the cross-repo probe evidence — the intake row) ──

    /// Drive the wire pass directly over one source.
    fn wire_of(src: &str) -> WireConsumption {
        wire_consumption(&[("lib.rs".to_string(), src.to_string())])
    }

    /// The delegation.rs shape: `let tx = DelegateTx { … };` then
    /// `serde_json::to_vec(&tx)` in the SAME fn — the field value is a
    /// variable/call (`delegator_pubkey` from keypair) → CLEARED.
    #[test]
    fn wire_literal_in_serde_fn_is_cleared() {
        let src = r"
#[derive(serde::Serialize)]
pub struct DelegateTx {
    pub delegator_pubkey: String,
    pub amount: u64,
}
pub fn build(keypair: &Keypair, amount: u64) -> Vec<u8> {
    let tx = DelegateTx {
        delegator_pubkey: keypair.public(),
        amount,
    };
    serde_json::to_vec(&tx).unwrap()
}
";
        let wire = wire_of(src);
        assert!(
            wire.cleared
                .contains(&("DelegateTx".to_string(), "delegator_pubkey".to_string())),
            "literal + to_vec in one fn clears the variable-valued field"
        );
        assert!(wire.wire_const.is_empty(), "no constant-valued field here");
    }

    /// The dao-soak `main()` shape: the literal and the serialize call FAR
    /// APART inside one big fn — fn scope, not adjacency.
    #[test]
    fn wire_serde_call_far_apart_in_same_fn_clears() {
        let src = r#"
#[derive(serde::Serialize)]
pub struct EpochSummary {
    pub advisory: u32,
    pub novel: u32,
}
pub fn main() {
    let verdict = decide();
    let summary = EpochSummary {
        advisory: verdict.advisory_count,
        novel: verdict.novel_count,
    };
    // … many lines …
    let artifact = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write("run.json", artifact).unwrap();
}
fn decide() -> Verdict { Verdict::default() }
"#;
        let wire = wire_of(src);
        assert!(
            wire.cleared
                .contains(&("EpochSummary".to_string(), "advisory".to_string()))
        );
    }

    /// The `payload_json` shape (the wrapping clause): the INNER type has NO
    /// direct literal in any serde fn (its literals live only in builders /
    /// tests) — it is consumed transitively via a `Vec<T>` field of a
    /// wrapper that IS literal-constructed in a serde fn.
    #[test]
    fn wire_vec_wrapped_type_cleared_transitively() {
        let src = r"
#[derive(serde::Serialize)]
pub struct ReportTarget {
    pub manifest_hash: String,
}
#[derive(serde::Serialize)]
pub struct SelfReportWire {
    pub targets: Vec<ReportTarget>,
}
pub fn payload_json(targets: &[ReportTarget]) -> String {
    let wire = SelfReportWire { targets: targets.to_vec() };
    serde_json::to_string(&wire).unwrap()
}
pub fn probe_target() -> ReportTarget {
    ReportTarget { manifest_hash: compute_manifest_hash() }
}
fn compute_manifest_hash() -> String { String::new() }
";
        let wire = wire_of(src);
        assert!(
            wire.cleared
                .contains(&("ReportTarget".to_string(), "manifest_hash".to_string())),
            "the Vec-wrapped inner type's call-valued field clears transitively"
        );
    }

    /// The `provenance_verified` shape (the 1/8 measured swallow, kept
    /// VISIBLE): wire-consumed but HARDCODED at its only construction site —
    /// `[wire-const]`, never cleared.
    #[test]
    fn wire_const_constant_at_every_site_is_flagged_weak() {
        let src = r#"
#[derive(serde::Serialize)]
pub struct EpochSummary {
    pub advisory: u32,
    pub provenance_verified: bool,
}
pub fn main() {
    let verdict = decide();
    let summary = EpochSummary {
        advisory: verdict.count,
        provenance_verified: true,
    };
    let artifact = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write("run.json", artifact).unwrap();
}
"#;
        let wire = wire_of(src);
        assert!(
            wire.wire_const.iter().any(|k| k
                == &(
                    "EpochSummary".to_string(),
                    "provenance_verified".to_string()
                )),
            "constant-at-every-site → wire-const, not cleared"
        );
        assert!(!wire.cleared.contains(&(
            "EpochSummary".to_string(),
            "provenance_verified".to_string()
        )));
        assert!(
            wire.cleared
                .contains(&("EpochSummary".to_string(), "advisory".to_string())),
            "the variable-valued sibling field still clears"
        );
    }

    /// A struct literal in a NON-serde fn is NOT wire evidence — the field
    /// stays stillborn (built-in-fn-A/serialized-in-fn-B is the documented,
    /// conservative gap).
    #[test]
    fn wire_literal_in_plain_fn_stays_flagged() {
        let src = r"
#[derive(serde::Serialize)]
pub struct PlainStruct {
    pub knob: f32,
}
pub fn build() -> PlainStruct {
    PlainStruct { knob: 1.5 }
}
";
        let wire = wire_of(src);
        assert!(wire.cleared.is_empty(), "no serde call → no wire evidence");
        assert!(wire.wire_const.is_empty());
    }

    /// Constant at one site, variable at another → NOT wire-const (the
    /// receiver can be seeing the variable value; the 7/8 measured class).
    #[test]
    fn wire_mixed_const_and_variable_sites_clear() {
        let src = r"
#[derive(serde::Serialize)]
pub struct Tx {
    pub gas_budget: u64,
}
pub fn emit(gas: u64) -> Vec<u8> {
    let tx = Tx { gas_budget: gas };
    serde_json::to_vec(&tx).unwrap()
}
pub fn fallback() -> Tx {
    Tx { gas_budget: 5 }
}
";
        let wire = wire_of(src);
        assert!(
            wire.cleared
                .contains(&("Tx".to_string(), "gas_budget".to_string())),
            "any non-constant site defeats wire-const"
        );
        assert!(wire.wire_const.is_empty());
    }

    /// End-to-end through the classifier: a wire-cleared field drops out of
    /// stillborn; a wire-const field renders its own weak row. (Ser+De —
    /// the measured family: the candidate net's serde-literal extension
    /// covers Ser+De structs; pure-Serialize stays outside it.)
    #[test]
    fn wire_clears_stillborn_and_reports_wire_const_end_to_end() {
        let src = r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EpochSummary {
    pub advisory: u32,
    pub provenance_verified: bool,
}
pub fn main() {
    let verdict = decide();
    let summary = EpochSummary {
        advisory: verdict.count,
        provenance_verified: true,
    };
    let artifact = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write("run.json", artifact).unwrap();
}
"#;
        let scan = scan_of(src);
        assert_eq!(
            scan.fields.len(),
            2,
            "both serde-literal fields are candidates"
        );
        let wire = wire_of(src);
        let fake = FakeHistory::new(&DEPTHS); // zero read-shaped occurrences everywhere
        let report = classify_with_probe(&scan, &fake, &wire, &DEPTHS);
        assert_eq!(report.wire_cleared, 1, "advisory (variable-valued) clears");
        assert!(report.stillborn.is_empty(), "cleared rows leave stillborn");
        assert_eq!(
            report.wire_const.len(),
            1,
            "provenance_verified stays as wire-const"
        );
        let rendered = report.render();
        assert!(rendered.contains("wire-cleared"));
        assert!(rendered.contains("[wire-const]"));
        assert!(rendered.contains("provenance_verified"));
    }

    #[test]
    fn self_literal_in_default_impl_not_duplicated() {
        // A struct with BOTH a Default impl and serde markers: the `Self {}`
        // literal inside the impl is collected once (scan_core resolves the
        // impl type; the serde collector skips `Self`).
        let src = r"
#[derive(serde::Deserialize)]
pub struct Dual { pub knob: f32 }
impl Default for Dual {
    fn default() -> Self { Self { knob: 1.0 } }
}
";
        let scan = scan_of(src);
        assert_eq!(scan.fields.len(), 1, "exactly one candidate row");
    }

    /// The end-to-end leg over a REAL throwaway git repository: init →
    /// config+consumer → the optimization removes the consumer → the report
    /// fires ORPHANED with the perf tag, and the never-read twin reads
    /// stillborn. Skips (passes silently) only when git is unavailable.
    #[test]
    fn real_git_e2e_orphan_lifecycle() {
        if git(None, &["--version"]).is_none_or(|o| !o.status.success()) {
            return; // no git on PATH — the unit arms above carry the classifier
        }
        let dir = std::env::temp_dir().join(format!("orphan-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .expect("git spawn");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
            out
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "orphan-e2e@test"]);
        run(&["config", "user.name", "orphan-e2e"]);
        let with_consumer = r"
pub struct BehaviorConfig { pub alarm_radius: f32, pub calm: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0, calm: 1.0 } }
}
pub fn alarm_of(c: &BehaviorConfig) -> f32 { c.alarm_radius }
";
        std::fs::write(dir.join("lib.rs"), with_consumer).unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "feat: config with a consumer"]);
        let optimized = r"
pub struct BehaviorConfig { pub alarm_radius: f32, pub calm: f32 }
impl Default for BehaviorConfig {
    fn default() -> Self { Self { alarm_radius: 3.0, calm: 1.0 } }
}
";
        std::fs::write(dir.join("lib.rs"), optimized).unwrap();
        run(&["add", "."]);
        run(&[
            "commit",
            "-q",
            "-m",
            "perf: const-table species behavior values",
        ]);
        let report = report_paths(std::slice::from_ref(&dir), &[1]);
        assert!(
            report.blind.is_none(),
            "a real repo resolves the history leg"
        );
        let alarm = report
            .orphaned
            .iter()
            .find(|r| r.candidate.field_name == "alarm_radius");
        let alarm = alarm.expect("alarm_radius orphaned after the optimization");
        assert!(alarm.likely_by_optimization);
        assert!(
            report
                .stillborn
                .iter()
                .any(|f| f.candidate.field_name == "calm"),
            "calm was never read — stillborn"
        );
        assert!(
            report
                .stillborn
                .iter()
                .all(|f| f.candidate.field_name != "alarm_radius")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
