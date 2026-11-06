//! `cargo refine --platform-audit` — post-mining intake class P22: the
//! platform-dead_code decl-vs-use cfg lattice.
//!
//! The measured shape (5 specimens across 2 repos in 2 days, all caught by
//! `x86_64`/Windows gate runs that no CI lane owns — katgpt-rs `NEON_U8`
//! fixed `ea4c2873`, zed `KEYCHAIN_SERVICE` + 3 more fixed `a4f70e64a7`):
//! an item declared UNCONDITIONALLY whose every use sits inside a
//! platform-gated context. rustc's `dead_code` cannot see it — the gate is
//! ON on the platform CI compiles, so the use exists there, while on every
//! other platform the item compiles to dead weight nobody warns about. The
//! whole class is invisible from any single-platform build matrix; the
//! fix is a lattice read, not a compile.
//!
//! The rule: for every module-level `const` / `static` / `fn` / `use`
//! declaration D with ≥1 use site, compute the intersection I of the
//! platform atoms GUARANTEED by each use site's cfg context (enclosing
//! `#[cfg]` attrs unioned through the mod stack). If I carries an atom the
//! declaration's own cfg does not (I \ D ≠ ∅), the item is dead on every
//! platform that atom excludes — flag it. `any(...)` arms intersect (only
//! atoms common to every arm are guaranteed); `not(...)` contributes no
//! positive atom (the conservative direction — a `not(windows)` use never
//! proves a platform, so it can only SAVE an item, never condemn it);
//! `feature = "..."` atoms are dropped (feature-gated use + ungated decl
//! is the existing cfg-audit family's territory, not the platform class).
//!
//! Report-first, gates nothing (the config-audit law). Ungated, zero new
//! deps (syn + std only). Exit contract (the frontier-report law): exit 0
//! on every real scan, refuse exit 1 on an EMPTY one — never a confident
//! green zero.
//!
//! # Honest limits (mirrored in the report footer)
//!
//! * **Uses are name-global last-segment matches** — no module resolution.
//!   A same-named item in another module masquerades; names declared 2+
//!   times are skipped as ambiguous rather than guessed (counted in the
//!   footer). Adjudicate rows against the listed atoms, same as every
//!   sibling audit (config-audit / orphan-report). **USE rows are
//!   scope-attributed** (2026-09-16, the chaind `BrowserNodeState`
//!   specimen): a module-PRIVATE import's binding is visible only inside
//!   its own file's module subtree, so its evidence reads only mentions
//!   at its scope or DESCENDANT scopes in its own file (child scopes see
//!   parent bindings; ancestors, siblings and other files cannot).
//!   Cross-file and out-of-scope same-name mentions were attributing
//!   atoms to test-scoped imports their scope does not guarantee — the
//!   internally-inconsistent pair (enum clean + its test import flagged).
//!   Non-private imports (`pub`/`pub(crate)`/… — the re-export surface)
//!   keep the name-global evidence: they are reachable from any module.
//! * **Module-DECLARATION gates (`#[cfg] mod x;` in the parent) ARE
//!   folded** (P24 (b), 2026-09-15): each file's unique incoming `mod
//!   x;` row — resolved by the standard rustc path rules (root files and
//!   `mod.rs` resolve children in their own directory; other files in a
//!   directory named after their stem; inline `mod a { mod b; }` nests
//!   `<base>/a/`) — unions its guaranteed atoms (plus the parent's own,
//!   transitively) into the child's root context, on both the decl and
//!   the use side. The first workspace sweep measured 6 FP rows from the
//!   pre-fold blindness (seal-view `web_link`/`web_mirror`, mmorpg
//!   `wasm.rs` decls reading ungated). Skipped — the no-fold direction:
//!   rows carrying `#[path]`, children absent from the scanned set,
//!   fn-local `mod x;` statements, and files whose incoming rows disagree
//!   (the lib.rs + main.rs ambiguity). **File-level `#![cfg]` INNER
//!   attrs are folded too** (P24 (a)): the file's atoms seed the root
//!   context for both decls and uses — the first workspace sweep
//!   measured 5 FP rows (decls in `#![cfg(all(macos, aarch64))]` files
//!   read ungated) and 1 rustc-proven miss (file-gated uses read
//!   UNGATED) from that one blindness, in both directions.
//! * **Macro token streams are not descended** (the P13 lesson) — a use
//!   inside an arbitrary macro body is invisible. The dangerous
//!   direction: a macro-only UNGATED use breaks the intersection silently
//!   and suppresses a flag (a miss, not a false positive). A macro-only
//!   GATED use simply reads as fewer uses. **Two families ARE arg-scanned
//!   at the token level — never expanded** (P24 (d), 2026-09-15): the
//!   format family (`format!`/`println!`/`write!`/`panic!`/tracing/…)
//!   counts inline `{ident}` captures in its string literals, and both it
//!   and the `assert*`/`matches` family count ident tokens in their
//!   EXPRESSION args (args of these macros are ordinary Rust — the v3
//!   sweep's chaind FPs were native test uses hidden inside `assert_eq!`).
//!   Unknown macros are not scanned (a braced word in a shader/yaml macro
//!   is data, not a capture — the miss direction). Overcounting is the
//!   safe direction (a phantom use can only suppress a flag, never raise
//!   one).
//! * **Nested fn-local items** (a const inside a fn body) are not tracked
//!   as declarations; their uses attribute to the outer item's context —
//!   statement-level `#[cfg]` inside bodies under-approximates (misses).
//! * **Re-exports**: `pub use a::b::C;` declares an import AND tallies a
//!   reference to the source name `C` in the import's cfg context — so a
//!   gated import condemns its source when it is the only reference (and
//!   the import itself stays silent: it does not compile where the gate
//!   is off). The local binding and the source share a name for plain
//!   imports, so such a pair reads as AMBIGUITY and is skipped — the
//!   rename form (`use a::C as D`) is the one that separates the names.
//!   Downstream consumption of a re-exported path is name-global like
//!   everything else.
//! * **Types are not tracked** (structs/enums/traits) — type-position name
//!   matching is far noisier and `pub` types are re-export surface; v1
//!   tracks the value namespace the specimens live in.
//! * `pub` declarations are flagged WITH a marker — crate-external
//!   consumers are invisible to a path scan (read the downstream hint).
//! * Uses inside `#[cfg(test)]` / `#[test]` context COUNT (a test use is
//!   a use for platform purposes) but mark the row `test-only` when every
//!   use is test context — the item is then prod-dead outside the atoms.
//! * **The walk is GIT-DERIVED inside repositories** (P25, 2026-09-15):
//!   `read_sources_walk` excludes GITIGNORED files (tracked +
//!   untracked-non-ignored only) — the 3 standing riir-train rows of the
//!   v3 sweep were name-global masquerades against a gitignored
//!   rustc-snapshot mining corpus no repo owns. Roots outside a repo fall
//!   back to the v2 filesystem walk; every report prints the walk mode.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::config_audit::{attrs_cfg_test, attrs_test_fn, read_sources_walk, WalkMode};

/// cfg predicate keys that carry platform meaning. `feature`, `test`,
/// `debug_assertions` and friends are deliberately absent — they are not
/// platforms, and their classes are owned by other audits.
const PLATFORM_KEYS: [&str; 4] = ["target_os", "target_arch", "target_env", "target_family"];

/// One guaranteed platform atom: `target_arch = "aarch64"`, or
/// `target_family = "unix"` (the `cfg(unix)` / `cfg(windows)` sugar
/// normalizes here).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atom {
    pub key: String,
    pub value: String,
}

/// The set of atoms a cfg context GUARANTEES (conjunction semantics —
/// every atom in the set holds).
type AtomSet = BTreeSet<Atom>;

/// One finding: a declaration dead on every platform its uses' shared
/// atoms exclude.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformDeadRow {
    pub name: String,
    pub kind: &'static str,
    pub origin: String,
    /// The declaration's own guaranteed atoms (rendered `-` when none).
    pub decl_atoms: AtomSet,
    /// Atoms shared by EVERY use site's cfg context.
    pub shared_atoms: AtomSet,
    /// `shared_atoms \ decl_atoms` — the witness set.
    pub missing: AtomSet,
    pub uses: usize,
    pub is_pub: bool,
    /// Every use site sits in test context (`#[cfg(test)]` mod or
    /// `#[test]` fn).
    pub test_only: bool,
}

/// The report over one scanned path set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlatformAuditReport {
    pub files_scanned: usize,
    /// How the file set was derived (P25) — git-derived walks exclude
    /// gitignored corpora (the 3 standing riir-train masquerade rows).
    pub walk: WalkMode,
    pub files_unparsed: usize,
    pub decls_tracked: usize,
    pub findings: Vec<PlatformDeadRow>,
    /// Names declared 2+ times across the scanned set — uses cannot be
    /// attributed; skipped, not guessed.
    pub ambiguous_names: usize,
    /// Tracked names with zero uses in the scanned set. Non-pub rows are
    /// rustc's own `dead_code` class on the platform it compiles; pub
    /// zero-use rows are config-audit / orphan-report territory.
    pub no_use_names: usize,
    /// Files whose root context gained atoms (or test context) from a
    /// parent-side `#[cfg] mod x;` row (P24 (b)) — the evidence line for
    /// the fold.
    pub files_modrow_gated: usize,
    /// Refusal reason (the frontier-report law): empty scan, never a
    /// confident green zero.
    pub blind: Option<String>,
}

/// Scan `paths` (dirs recursively / files) and classify.
#[must_use]
pub fn audit_paths(paths: &[PathBuf]) -> PlatformAuditReport {
    let walked = read_sources_walk(paths);
    let mut report = audit_sources(&walked.sources);
    report.walk = walked.mode;
    report
}

/// The testable core over in-memory `(file, src)` pairs.
pub fn audit_sources(sources: &[(String, String)]) -> PlatformAuditReport {
    let mut report = PlatformAuditReport {
        files_scanned: sources.len(),
        ..PlatformAuditReport::default()
    };
    if sources.is_empty() {
        report.blind = Some("no .rs files found under the scanned path(s)".into());
        return report;
    }

    // Phase 1 — parse everything and KEEP the trees: the module lattice
    // (below) needs every parent file available before any child can be
    // seeded, so the one-file-at-a-time drop of v1 is gone (memory note:
    // the walk prune of v2 is what keeps the pathological scans out).
    let mut parsed_files: Vec<(String, syn::File)> = Vec::with_capacity(sources.len());
    for (file, src) in sources {
        match syn::parse_file(src) {
            Ok(parsed) => parsed_files.push((file.clone(), parsed)),
            Err(_) => report.files_unparsed += 1,
        }
    }
    if report.files_unparsed == sources.len() {
        report.blind = Some("every scanned file failed to parse".into());
        return report;
    }

    // Phase 2 — the parent-side `#[cfg] mod x;` lattice (P24 (b)): each
    // file's unique incoming mod row (plus the parent's own inherited
    // gates, transitively) folds into its root context, exactly like the
    // file-level `#![cfg]` fold of v2 — both the decl side and the use
    // side, by construction.
    let lattice = ModRowLattice::build(&parsed_files);
    report.files_modrow_gated = lattice.folded_count();

    // name → decl records; name → use evidence, split by REFERENCE KIND:
    // code mentions (path occurrences) vs import references. A `use a::C`
    // resolves on every platform the IMPORT compiles, so the import's own
    // path is a real use of the SOURCE const — but it says nothing about
    // the LOCAL binding (an ungated import whose only code mentions are
    // gated is exactly the ProcessStatus specimen). use-decls therefore
    // read only code evidence; value decls read the union.
    let mut decls: BTreeMap<String, Vec<DeclRec>> = BTreeMap::new();
    let mut sinks = UseSinks::default();

    for (file, parsed) in &parsed_files {
        // File-level `#![cfg]` INNER attrs gate the whole file (the
        // ane_prefill bank/exec/exec_zc/headroom shape — the P24 (a)
        // fold), UNIONED with the inherited mod-row gates from the
        // lattice (P24 (b)). Seeding the root context fixes BOTH
        // directions at once: decls recorded under it inherit the gate
        // into their atoms (a gated file's items are not platform-dead —
        // they do not even exist elsewhere), and uses inside it carry
        // the gate into their evidence (a file-gated use is NOT an
        // ungated use — the note_ane_dispatch miss). `#![cfg(test)]` at
        // file level is test context, same as an item-level one.
        let ctx = lattice.effective_ctx(file);
        walk_items(
            &parsed.items,
            &ctx,
            &mut decls,
            &mut sinks,
            file,
            &Vec::new(),
        );
    }

    report.decls_tracked = decls.values().map(Vec::len).sum();
    for (name, ds) in &decls {
        if ds.len() != 1 {
            report.ambiguous_names += 1;
            continue;
        }
        let d = &ds[0];
        let merged: Vec<UseEvidence> = if d.kind == "use" && !d.is_pub {
            // Module-private import: the binding is visible only inside
            // its own file's scope subtree — in-scope evidence only (the
            // chaind BrowserNodeState specimen: the wasm32 type-path uses
            // at file root referenced the ENUM, not the test-scoped
            // binding, and were attributing atoms its scope does not
            // guarantee).
            in_scope_evidence(&sinks.scoped, &d.file, &d.scope, name)
        } else if d.kind == "use" {
            // Re-export surface: reachable from any module, so the
            // name-global evidence stands.
            sinks.code.get(name).cloned().into_iter().collect()
        } else {
            let mut v: Vec<UseEvidence> = Vec::new();
            if let Some(e) = sinks.code.get(name) {
                v.push(e.clone());
            }
            if let Some(e) = sinks.src.get(name) {
                v.push(e.clone());
            }
            v
        };
        if merged.is_empty() {
            report.no_use_names += 1;
            continue;
        }
        // Intersection of every distinct use context's atoms.
        let mut shared: Option<AtomSet> = None;
        let mut total = 0usize;
        let mut all_test = true;
        for e in &merged {
            total += e.total;
            for (atoms, in_test) in &e.contexts {
                all_test &= *in_test;
                shared = Some(match shared.take() {
                    Some(s) => s.intersection(atoms).cloned().collect(),
                    None => atoms.clone(),
                });
            }
        }
        let shared = shared.unwrap_or_default();
        let missing: AtomSet = shared.difference(&d.atoms).cloned().collect();
        if missing.is_empty() {
            continue;
        }
        report.findings.push(PlatformDeadRow {
            name: name.clone(),
            kind: d.kind,
            origin: d.origin.clone(),
            decl_atoms: d.atoms.clone(),
            shared_atoms: shared,
            missing,
            uses: total,
            is_pub: d.is_pub,
            test_only: all_test,
        });
    }
    report.findings.sort_by(|a, b| a.origin.cmp(&b.origin));
    report
}

struct DeclRec {
    /// The declared name (the decl-map key — carried so the record is
    /// self-contained and `record_decl` stays single-argument).
    name: String,
    kind: &'static str,
    origin: String,
    atoms: AtomSet,
    is_pub: bool,
    /// Declaring file + lexical scope (inline-mod path from the file
    /// root) — the use-row attribution half: a private import's binding
    /// is visible only inside this subtree of its own file.
    file: String,
    scope: Vec<String>,
}

/// Mention-evidence sinks for one scan. `code`/`src` are the name-global
/// maps (value decls + non-private re-export rows read them); `scoped` is
/// the (file → lexical scope → name) map the private-use-row half reads.
#[derive(Default)]
struct UseSinks {
    code: BTreeMap<String, UseEvidence>,
    src: BTreeMap<String, UseEvidence>,
    scoped: BTreeMap<String, BTreeMap<Vec<String>, BTreeMap<String, UseEvidence>>>,
}

#[derive(Default, Clone)]
struct UseEvidence {
    /// Distinct (cfg context, test flag) pairs — the intersection folds
    /// over these; the count does not.
    contexts: BTreeSet<(AtomSet, bool)>,
    total: usize,
}

/// Walking context: the union of guaranteed atoms through the mod stack
/// (plus the enclosing item's own cfg), and whether we are inside test
/// context.
struct Ctx {
    atoms: AtomSet,
    in_test: bool,
}

/// One external `mod name;` row as indexed from its declaring file: the
/// row's own guaranteed atoms (the declaring file's root `#![cfg]` + any
/// inline-mod nesting + the row's own cfg, all unioned — the LOCAL
/// guarantee; the parent's inherited gates fold in at resolution time)
/// and the directory the child resolves against.
struct ModRowIndexRow {
    dir: PathBuf,
    name: String,
    atoms: AtomSet,
    in_test: bool,
    has_path_attr: bool,
}

/// The incoming edge record: which parent file declares this child, and
/// the row's local guarantee.
struct IncomingRow {
    parent: String,
    atoms: AtomSet,
    in_test: bool,
}

/// The parent-side module-declaration gate lattice (P24 (b), 2026-09-15):
/// which file is declared by which `mod x;` row, resolved by the standard
/// rustc path rules, and the effective root context each file walks under
/// (own `#![cfg]` ∪ the unique incoming row's atoms ∪ the parent's own
/// effective atoms, transitively).
///
/// Resolution rules (rustc 2018):
/// * root files (`lib.rs` / `main.rs` / `build.rs`, and direct `.rs`
///   children of `bin` / `tests` / `examples` / `benches` dirs) resolve
///   children in their own directory;
/// * `mod.rs` files resolve children in their own directory;
/// * every other file resolves children in a directory named after its
///   stem;
/// * an inline `mod a { mod b; }` nests: `b` resolves against
///   `<base>/a/`.
///
/// Divergences (all in the no-fold / miss direction — never a wrong
/// fold): rows carrying `#[path]` are skipped (the child lives
/// elsewhere); children not present in the scanned set are skipped;
/// children with disagreeing incoming rows are skipped (the ambiguity
/// law — a crate carries both `lib.rs` and `main.rs` roots and both may
/// declare the same name); `mod x;` statements inside fn bodies are not
/// indexed. A wrong root classification can only MIS-resolve a child
/// into the no-fold direction (the candidate paths then match nothing).
struct ModRowLattice {
    effective: HashMap<String, (AtomSet, bool)>,
    folded: HashSet<String>,
}

impl ModRowLattice {
    fn build(parsed_files: &[(String, syn::File)]) -> Self {
        // Own root context per file (the v2 `#![cfg]` fold's input).
        let mut own: HashMap<String, (AtomSet, bool)> = HashMap::with_capacity(parsed_files.len());
        for (file, pf) in parsed_files {
            own.insert(
                file.clone(),
                (guaranteed_from_attrs(&pf.attrs), attrs_cfg_test(&pf.attrs)),
            );
        }

        // Index external mod rows and resolve them to child files in the
        // scanned set.
        // ⛔ WINDOWS: the candidate keys below are built with `Path::join`,
        // which emits `\`, while every source key here uses `/` — they come
        // from `git ls-files` (P25) or from a caller's literal
        // `src/util/win.rs`. A real walk produces MIXED keys
        // (`E:\repo\src/util/mod.rs`), so normalising only one side is not
        // enough. The index is therefore keyed by the separator-insensitive
        // form and maps back to the ORIGINAL string, so every other map here
        // stays keyed by the file as the caller spelled it.
        //
        // Unnormalised, NO mod row resolves on Windows: the lattice silently
        // degrades to zero, every mod-row-gated item reads as ungated, and
        // the audit invents platform-dead findings. 8 of this module's own
        // fixtures were red here and green everywhere else — no lane in this
        // workspace runs this crate's tests on Windows.
        let norm = |s: &str| s.replace('\\', "/");
        let key_set: HashMap<String, &str> = parsed_files
            .iter()
            .map(|(f, _)| (norm(f), f.as_str()))
            .collect();
        let mut incoming: HashMap<String, Vec<IncomingRow>> = HashMap::new();
        for (file, pf) in parsed_files {
            let base = resolution_base(file);
            let (root_atoms, root_test) = own.get(file).cloned().unwrap_or_default();
            let mut rows = Vec::new();
            index_mod_items(&pf.items, &root_atoms, root_test, &base, &mut rows);
            for row in rows {
                // `#[path]` rows: the child lives at a nonstandard path —
                // skipped (the no-fold direction).
                if row.has_path_attr {
                    continue;
                }
                let candidates = [
                    row.dir.join(format!("{}.rs", row.name)),
                    row.dir.join(&row.name).join("mod.rs"),
                ];
                for candidate in candidates {
                    let key = norm(&candidate.display().to_string());
                    if let Some(orig) = key_set.get(&key) {
                        incoming
                            .entry((*orig).to_string())
                            .or_default()
                            .push(IncomingRow {
                                parent: file.clone(),
                                atoms: row.atoms.clone(),
                                in_test: row.in_test,
                            });
                    }
                }
            }
        }

        // Effective root ctx per file: own ∪ (unique consistent incoming
        // row ∪ parent effective, transitively). Memoized recursion; the
        // visiting set is a defensive cycle guard (module edges strictly
        // deepen in valid Rust, so it never fires on real trees).
        let mut memo: HashMap<String, (AtomSet, bool)> = HashMap::new();
        let mut visiting: HashSet<String> = HashSet::new();
        let mut folded: HashSet<String> = HashSet::new();
        for (file, _) in parsed_files {
            effective_root(file, &own, &incoming, &mut memo, &mut visiting, &mut folded);
        }
        Self {
            effective: memo,
            folded,
        }
    }

    /// The context a file's items walk under: its effective root ctx
    /// (own `#![cfg]` ∪ inherited mod-row gates).
    fn effective_ctx(&self, file: &str) -> Ctx {
        let (atoms, in_test) = self.effective.get(file).cloned().unwrap_or_default();
        Ctx { atoms, in_test }
    }

    fn folded_count(&self) -> usize {
        self.folded.len()
    }
}

/// The recursive half of the lattice: a file's effective root context.
fn effective_root(
    file: &str,
    own: &HashMap<String, (AtomSet, bool)>,
    incoming: &HashMap<String, Vec<IncomingRow>>,
    memo: &mut HashMap<String, (AtomSet, bool)>,
    visiting: &mut HashSet<String>,
    folded: &mut HashSet<String>,
) -> (AtomSet, bool) {
    if let Some(hit) = memo.get(file) {
        return hit.clone();
    }
    let (mut atoms, mut in_test) = own.get(file).cloned().unwrap_or_default();
    if visiting.insert(file.to_string()) {
        if let Some(rows) = incoming.get(file) {
            // Full contribution per row: the parent's own effective atoms
            // unioned with the row's local guarantee (conjunction of
            // gates — the child compiles only when BOTH hold).
            let contributions: Vec<(AtomSet, bool)> = rows
                .iter()
                .map(|r| {
                    let (pa, pt) = effective_root(&r.parent, own, incoming, memo, visiting, folded);
                    let mut a = pa;
                    a.extend(r.atoms.iter().cloned());
                    (a, pt || r.in_test)
                })
                .collect();
            // Disagreeing rows = ambiguous parenthood (the lib.rs +
            // main.rs shape) — skip the fold, keep own only.
            if let Some(first) = contributions.first() {
                if contributions.iter().all(|c| c == first) {
                    if !first.0.is_empty() || first.1 {
                        folded.insert(file.to_string());
                    }
                    atoms.extend(first.0.iter().cloned());
                    in_test |= first.1;
                }
            }
        }
        visiting.remove(file);
        memo.insert(file.to_string(), (atoms.clone(), in_test));
    }
    (atoms, in_test)
}

/// Index a file's external `mod x;` rows with their full nesting context.
/// `dir` carries the resolution base through inline mod nesting (rustc's
/// rule: an inline `mod a { mod b; }` resolves b against `<base>/a/`).
/// Only module-level mods are indexed — `mod x;` inside a fn body is not
/// walked (the no-fold direction).
fn index_mod_items(
    items: &[syn::Item],
    atoms: &AtomSet,
    in_test: bool,
    dir: &Path,
    out: &mut Vec<ModRowIndexRow>,
) {
    for item in items {
        let item_atoms = guaranteed_from_attrs(item_attrs(item));
        let mut row_atoms = atoms.clone();
        row_atoms.extend(item_atoms);
        let row_test =
            in_test || attrs_cfg_test(item_attrs(item)) || attrs_test_fn(item_attrs(item));
        if let syn::Item::Mod(m) = item {
            match &m.content {
                Some((_, inner)) => {
                    let sub = dir.join(m.ident.to_string());
                    index_mod_items(inner, &row_atoms, row_test, &sub, out);
                }
                None => out.push(ModRowIndexRow {
                    dir: dir.to_path_buf(),
                    name: m.ident.to_string(),
                    atoms: row_atoms,
                    in_test: row_test,
                    has_path_attr: has_path_attr(&m.attrs),
                }),
            }
        }
    }
}

/// Does this attr list carry `#[path = "..."]` (or the bare path form)?
fn has_path_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| match &a.meta {
        syn::Meta::NameValue(nv) => last_segment(&nv.path).as_deref() == Some("path"),
        syn::Meta::Path(p) => last_segment(p).as_deref() == Some("path"),
        syn::Meta::List(_) => false,
    })
}

/// Is this file its own crate root? Approximates cargo's autodiscovery:
/// `lib.rs` / `main.rs` / `build.rs` anywhere, plus the direct `.rs`
/// children of `bin` / `tests` / `examples` / `benches` directories. A
/// wrong guess here can only push a child into the no-fold direction
/// (its candidate paths then match nothing in the scanned set).
fn is_root_file(file: &str) -> bool {
    let p = Path::new(file);
    match p.file_name().and_then(|n| n.to_str()) {
        Some("lib.rs" | "main.rs" | "build.rs") => true,
        _ => {
            matches!(
                p.parent()
                    .and_then(|d| d.file_name())
                    .and_then(|n| n.to_str()),
                Some("bin" | "tests" | "examples" | "benches")
            ) && p.extension().is_some_and(|e| e == "rs")
        }
    }
}

/// The directory this file's external `mod name;` rows resolve against.
fn resolution_base(file: &str) -> PathBuf {
    let p = Path::new(file);
    let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if fname == "mod.rs" || is_root_file(file) {
        p.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        p.parent().map(|d| d.join(stem)).unwrap_or_default()
    }
}

/// Recursively walk module-level items: record tracked declarations,
/// recurse inline mods, and collect path uses from everything else.
fn walk_items(
    items: &[syn::Item],
    ctx: &Ctx,
    decls: &mut BTreeMap<String, Vec<DeclRec>>,
    sinks: &mut UseSinks,
    file: &str,
    scope: &[String],
) {
    for item in items {
        let item_atoms = guaranteed_from_attrs(item_attrs(item));
        let in_test =
            ctx.in_test || attrs_cfg_test(item_attrs(item)) || attrs_test_fn(item_attrs(item));
        let mut atoms = ctx.atoms.clone();
        atoms.extend(item_atoms);
        let ctx2 = Ctx { atoms, in_test };

        match item {
            syn::Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    let mut inner_scope = scope.to_vec();
                    inner_scope.push(m.ident.to_string());
                    walk_items(inner, &ctx2, decls, sinks, file, &inner_scope);
                }
                // External `mod x;` — the child file's inherited gate is
                // resolved by the ModRowLattice (P24 (b)) and folded into
                // its root ctx when that file is walked.
            }
            syn::Item::Const(c) => {
                record_decl(
                    decls,
                    DeclRec {
                        name: c.ident.to_string(),
                        kind: "const",
                        origin: format!("{}:{}", file, c.ident.span().start().line),
                        atoms: ctx2.atoms.clone(),
                        is_pub: !matches!(c.vis, syn::Visibility::Inherited),
                        file: file.to_string(),
                        scope: scope.to_vec(),
                    },
                );
                tally_item_uses(sinks, item, &ctx2, file, scope, &BTreeSet::new());
            }
            syn::Item::Static(s) => {
                record_decl(
                    decls,
                    DeclRec {
                        name: s.ident.to_string(),
                        kind: "static",
                        origin: format!("{}:{}", file, s.ident.span().start().line),
                        atoms: ctx2.atoms.clone(),
                        is_pub: !matches!(s.vis, syn::Visibility::Inherited),
                        file: file.to_string(),
                        scope: scope.to_vec(),
                    },
                );
                tally_item_uses(sinks, item, &ctx2, file, scope, &BTreeSet::new());
            }
            syn::Item::Fn(f) => {
                let name = f.sig.ident.to_string();
                record_decl(
                    decls,
                    DeclRec {
                        name: name.clone(),
                        kind: "fn",
                        origin: format!("{}:{}", file, f.sig.ident.span().start().line),
                        atoms: ctx2.atoms.clone(),
                        is_pub: !matches!(f.vis, syn::Visibility::Inherited),
                        file: file.to_string(),
                        scope: scope.to_vec(),
                    },
                );
                // Recursion inside the fn's own body is not a use.
                let mut skip = BTreeSet::new();
                skip.insert(name);
                tally_item_uses(sinks, item, &ctx2, file, scope, &skip);
            }
            syn::Item::Use(u) => {
                for leaf in use_leaves(&u.tree) {
                    record_decl(
                        decls,
                        DeclRec {
                            name: leaf.local.clone(),
                            kind: "use",
                            origin: format!("{}:{}", file, u.span().start().line),
                            atoms: ctx2.atoms.clone(),
                            is_pub: !matches!(u.vis, syn::Visibility::Inherited),
                            file: file.to_string(),
                            scope: scope.to_vec(),
                        },
                    );
                    // The import IS a reference to its source name, in
                    // the import's cfg context — a gated import is the
                    // source's only-gated-reference shape. It does NOT
                    // count for the local binding (an ungated import
                    // whose only code mentions are gated is the specimen
                    // class and must stay flaggable).
                    tally_use(&mut sinks.src, &leaf.source, &ctx2, 1);
                }
            }
            _ => tally_item_uses(sinks, item, &ctx2, file, scope, &BTreeSet::new()),
        }
    }
}

/// A `use` leaf: the local name it binds + the source's last segment
/// (identical unless renamed).
struct UseLeaf {
    local: String,
    source: String,
}

/// Every `syn::Item` variant's attributes (syn exposes no uniform
/// accessor).
fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        // syn::Item is non-exhaustive; unreachable variants carry no
        // visible cfg (under-approximation — the miss direction, safe).
        _ => &[],
    }
}

/// Flatten a use tree into its leaves (globs contribute nothing).
fn use_leaves(tree: &syn::UseTree) -> Vec<UseLeaf> {
    let mut out = Vec::new();
    walk_use_tree(tree, &mut out);
    out
}

fn walk_use_tree(tree: &syn::UseTree, out: &mut Vec<UseLeaf>) {
    match tree {
        syn::UseTree::Path(p) => walk_use_tree(&p.tree, out),
        syn::UseTree::Name(n) => out.push(UseLeaf {
            local: n.ident.to_string(),
            source: n.ident.to_string(),
        }),
        syn::UseTree::Rename(r) => out.push(UseLeaf {
            local: r.rename.to_string(),
            source: r.ident.to_string(),
        }),
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(g) => {
            for item in &g.items {
                walk_use_tree(item, out);
            }
        }
    }
}

fn record_decl(decls: &mut BTreeMap<String, Vec<DeclRec>>, rec: DeclRec) {
    decls.entry(rec.name.clone()).or_default().push(rec);
}

fn tally_use(uses: &mut BTreeMap<String, UseEvidence>, name: &str, ctx: &Ctx, n: usize) {
    let e = uses.entry(name.to_string()).or_default();
    e.contexts.insert((ctx.atoms.clone(), ctx.in_test));
    e.total += n;
}

/// Collect an item's path mentions (name → count) under one cfg context;
/// callers tally them into the evidence sinks.
fn collect_uses(item: &syn::Item, skip: &BTreeSet<String>) -> BTreeMap<String, usize> {
    let mut visitor = UseVisitor {
        found: BTreeMap::new(),
        skip,
    };
    visitor.visit_item(item);
    visitor.found
}

/// Tally one item's mentions into BOTH evidence maps: the name-global
/// `code` map (value decls + non-private re-export rows) and the
/// file/scope-scoped map the private-use-row half reads.
fn tally_item_uses(
    sinks: &mut UseSinks,
    item: &syn::Item,
    ctx: &Ctx,
    file: &str,
    scope: &[String],
    skip: &BTreeSet<String>,
) {
    let found = collect_uses(item, skip);
    let UseSinks { code, scoped, .. } = sinks;
    let scope_entry = scoped
        .entry(file.to_string())
        .or_default()
        .entry(scope.to_vec())
        .or_default();
    for (name, n) in found {
        tally_use(code, &name, ctx, n);
        tally_use(scope_entry, &name, ctx, n);
    }
}

/// Evidence for a module-private import binding: mentions inside its own
/// file whose lexical scope is the import's scope or a DESCENDANT (child
/// scopes see parent bindings; ancestors, siblings and other files
/// cannot). An import with no in-scope mention is rustc's
/// unused-import class, not a platform row — it reads as no-use here.
fn in_scope_evidence(
    scoped: &BTreeMap<String, BTreeMap<Vec<String>, BTreeMap<String, UseEvidence>>>,
    file: &str,
    scope: &[String],
    name: &str,
) -> Vec<UseEvidence> {
    let mut v = Vec::new();
    if let Some(scopes) = scoped.get(file) {
        for (s, names) in scopes {
            if s.starts_with(scope) {
                if let Some(e) = names.get(name) {
                    v.push(e.clone());
                }
            }
        }
    }
    v
}

/// Format-family macros whose string literals carry inline `{name}`
/// captures (P24 (d)): `println!("{X:?}")` is an ungated use the syn
/// path walk cannot see. A cheap TOKEN scan of the macro's string
/// literals — never expansion. Unknown macros are not scanned (a
/// braced word in a shader/yaml macro is data, not a capture — the miss
/// direction).
const FORMAT_MACROS: [&str; 21] = [
    "format",
    "format_args",
    "println",
    "eprintln",
    "print",
    "eprint",
    "write",
    "writeln",
    "panic",
    "assert",
    "assert_eq",
    "assert_ne",
    "todo",
    "unreachable",
    "info",
    "warn",
    "error",
    "debug",
    "trace",
    "bail",
    "ensure",
];

/// Expression-argument macros whose ARGS are ordinary Rust expressions —
/// the `assert*` family (the v3-sweep chaind FPs: the native test uses of
/// `stream_type::SIGNALING` / `BrowserNodeState` sit inside `assert_eq!`
/// args and were invisible). Ident tokens in these args are real path/call
/// references; path PREFIXES (`a::` in `a::b`) and macro names (`foo!`)
/// are skipped so a multi-segment path counts once, on its last segment —
/// the same rule as `visit_path`.
const EXPR_ARG_MACROS: [&str; 7] = [
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "matches",
];

/// Path-tally visitor: counts occurrences per last path segment.
struct UseVisitor<'a> {
    found: BTreeMap<String, usize>,
    skip: &'a BTreeSet<String>,
}

impl<'ast> Visit<'ast> for UseVisitor<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(seg) = path.segments.last() {
            let name = seg.ident.to_string();
            if !name.is_empty() && !self.skip.contains(&name) {
                *self.found.entry(name).or_insert(0) += 1;
            }
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(name) = last_segment(&mac.path) {
            let format_family = FORMAT_MACROS.contains(&name.as_str());
            let expr_family = EXPR_ARG_MACROS.contains(&name.as_str());
            if format_family || expr_family {
                let mut captured = BTreeMap::new();
                if format_family {
                    scan_inline_captures(&mac.tokens, &mut captured);
                }
                // Both families take expression args (`println!("{}", X)` —
                // X is a real use too).
                scan_expr_idents(&mac.tokens, &mut captured);
                for (name, n) in captured {
                    if !self.skip.contains(&name) {
                        *self.found.entry(name).or_insert(0) += n;
                    }
                }
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

/// Walk a format-family macro's token tree and count inline `{ident}`
/// captures inside its string literals (P24 (d)). Doubled `{{` is a
/// literal brace, not a capture; `{0}`/`{}` positional forms carry no
/// identifier. Overcounting is the documented safe direction (a phantom
/// use can only suppress a flag, never raise one).
fn scan_inline_captures(tokens: &proc_macro2::TokenStream, found: &mut BTreeMap<String, usize>) {
    use proc_macro2::TokenTree;
    for t in tokens.clone() {
        match t {
            TokenTree::Literal(lit) => {
                let s = lit.to_string();
                if s.starts_with('"') || s.starts_with('r') || s.starts_with('b') {
                    scan_capture_str(&s, found);
                }
            }
            TokenTree::Group(g) => scan_inline_captures(&g.stream(), found),
            TokenTree::Punct(_) | TokenTree::Ident(_) => {}
        }
    }
}

/// Walk one rendered string literal for `{ident` capture openings.
fn scan_capture_str(s: &str, found: &mut BTreeMap<String, usize>) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        // Doubled brace — a literal `{`, not a capture.
        if bytes.get(i + 1) == Some(&b'{') {
            i += 2;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
            j += 1;
        }
        if j > start {
            let ident = &s[start..j];
            // Positional `{0}` args and a bare `{_}` are not captures.
            if !ident.as_bytes()[0].is_ascii_digit() && ident != "_" {
                *found.entry(ident.to_string()).or_insert(0) += 1;
            }
        }
        i = j.max(i + 1);
    }
}

/// Tally ident tokens in an expression-argument macro's args: real path
/// last-segments and call names. A `::`-followed ident is a path prefix
/// (the segment after the last `::` counts instead); a `!`-followed ident
/// is a macro name. Overcounting is the documented safe direction.
fn scan_expr_idents(tokens: &proc_macro2::TokenStream, found: &mut BTreeMap<String, usize>) {
    use proc_macro2::TokenTree;
    let flat: Vec<TokenTree> = flatten_tokens(tokens);
    let mut i = 0;
    while i < flat.len() {
        if let TokenTree::Ident(id) = &flat[i] {
            let is_path_prefix =
                matches!(flat.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == ':');
            let is_macro_name =
                matches!(flat.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!');
            let s = id.to_string();
            if !is_path_prefix && !is_macro_name {
                *found.entry(s).or_insert(0) += 1;
            }
        }
        i += 1;
    }
}

/// Flatten a token tree into a linear token vec (macro args nest in
/// groups; expression idents live at every level).
fn flatten_tokens(tokens: &proc_macro2::TokenStream) -> Vec<proc_macro2::TokenTree> {
    use proc_macro2::TokenTree;
    let mut out = Vec::new();
    for t in tokens.clone() {
        match t {
            TokenTree::Group(g) => {
                out.extend(flatten_tokens(&g.stream()));
            }
            other => out.push(other),
        }
    }
    out.shrink_to_fit();
    out
}

/// Guaranteed atoms from an item's `#[cfg(...)]` attributes (multiple
/// cfg attrs AND — union; each predicate parsed recursively).
fn guaranteed_from_attrs(attrs: &[syn::Attribute]) -> AtomSet {
    let mut set = AtomSet::new();
    for attr in attrs {
        if let syn::Meta::List(list) = &attr.meta {
            if last_segment(&list.path).as_deref() == Some("cfg") {
                if let Ok(meta) = syn::parse2::<syn::Meta>(list.tokens.clone()) {
                    set.extend(guaranteed(&meta));
                }
            }
        }
    }
    set
}

/// The guaranteed-atom semantics of one cfg predicate.
fn guaranteed(meta: &syn::Meta) -> AtomSet {
    match meta {
        syn::Meta::Path(p) => match last_segment(p).as_deref() {
            // The family sugar normalizes to target_family atoms.
            Some("unix") => one_atom("target_family", "unix"),
            Some("windows") => one_atom("target_family", "windows"),
            _ => AtomSet::new(),
        },
        syn::Meta::NameValue(nv) => {
            let Some(key) = last_segment(&nv.path) else {
                return AtomSet::new();
            };
            if !PLATFORM_KEYS.contains(&key.as_str()) {
                return AtomSet::new();
            }
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                return one_atom(&key, &s.value());
            }
            AtomSet::new()
        }
        syn::Meta::List(list) => {
            let head = last_segment(&list.path).unwrap_or_default();
            let children = parse_metas(&list.tokens);
            match head.as_str() {
                // all(a, b): both hold — union of guarantees.
                "all" => {
                    let mut set = AtomSet::new();
                    for c in &children {
                        set.extend(guaranteed(c));
                    }
                    set
                }
                // any(a, b): only atoms common to EVERY arm are guaranteed.
                "any" => {
                    let mut shared: Option<AtomSet> = None;
                    for c in &children {
                        let g = guaranteed(c);
                        shared = Some(match shared.take() {
                            Some(s) => s.intersection(&g).cloned().collect(),
                            None => g,
                        });
                    }
                    shared.unwrap_or_default()
                }
                // not(...) proves no positive platform — conservative.
                _ => AtomSet::new(),
            }
        }
    }
}

/// Parse a cfg list body as comma-separated metas (best effort — a
/// malformed body yields nothing, the conservative direction).
fn parse_metas(tokens: &proc_macro2::TokenStream) -> Vec<syn::Meta> {
    use syn::parse::Parser;
    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
        .parse2(tokens.clone())
        .map(|p| p.into_iter().collect())
        .unwrap_or_default()
}

fn last_segment(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .filter(|s| !s.is_empty())
}

fn one_atom(key: &str, value: &str) -> AtomSet {
    let mut set = AtomSet::new();
    set.insert(Atom {
        key: key.to_string(),
        value: value.to_string(),
    });
    set
}

fn render_atoms(set: &AtomSet) -> String {
    if set.is_empty() {
        return "-".to_string();
    }
    set.iter()
        .map(|a| format!("{} = \"{}\"", a.key, a.value))
        .collect::<Vec<_>>()
        .join(" + ")
}

impl PlatformAuditReport {
    /// Plain-text render (the config-audit house style).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "platform-audit: the platform-dead_code lattice (intake class P22)"
        );
        let _ = writeln!(
            out,
            "scanned {} file(s) ({} unparsed), {} tracked decl(s) (const/static/fn/use){}",
            self.files_scanned,
            self.files_unparsed,
            self.decls_tracked,
            if self.files_modrow_gated > 0 {
                format!(
                    ", {} file(s) under parent mod-row gates",
                    self.files_modrow_gated
                )
            } else {
                String::new()
            }
        );
        let _ = writeln!(out, "{}", self.walk.note());
        if let Some(reason) = &self.blind {
            let _ = writeln!(
                out,
                "\n⛔ REFUSING: {reason} — never a confident green zero."
            );
            return out;
        }
        if self.findings.is_empty() {
            let _ = writeln!(
                out,
                "\nno platform-dead declarations: every tracked item with uses \
                 has an ungated use or a decl gate covering its uses' platforms"
            );
        }
        for f in &self.findings {
            let markers = [
                if f.is_pub { Some("[pub]") } else { None },
                if f.test_only {
                    Some("[test-only]")
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
            let markers = if markers.is_empty() {
                String::new()
            } else {
                format!(" {markers}")
            };
            let _ = writeln!(
                out,
                "\n[PLATFORM-DEAD] {} {} — {}{markers}",
                f.kind, f.name, f.origin
            );
            let _ = writeln!(
                out,
                "    decl cfg: {} | {} use(s), all under: {}",
                render_atoms(&f.decl_atoms),
                f.uses,
                render_atoms(&f.shared_atoms)
            );
            let _ = writeln!(
                out,
                "    dead on every platform outside: {}",
                render_atoms(&f.missing)
            );
        }
        let _ = writeln!(
            out,
            "\nfooter: {} ambiguous name(s) skipped (declared 2+ times — uses not \
             attributable), {} name(s) with zero uses in scope (rustc's own unused \
             class on the platform it compiles; pub zero-use rows belong to \
             config-audit / orphan-report)",
            self.ambiguous_names, self.no_use_names
        );
        let _ = writeln!(
            out,
            "divergences: uses are name-global last-segment matches — adjudicate \
             rows against the listed atoms; file-level `#![cfg]` inner attrs AND \
             parent-side `#[cfg] mod x;` declaration gates are folded into the root \
             context (P24 (a)+(b)) — `#[path]`-carrying mod rows, fn-local `mod x;` \
             statements, and files with disagreeing incoming rows stay unfolded (the \
             no-fold direction); macro token streams not \
             descended (a macro-only ungated use suppresses a flag — a miss, not \
             an FP) but the format + assert families ARE arg-scanned — inline \
             `{{name}}` captures in format strings AND ident tokens in expression \
             args (P24 (d); unknown macros still miss); fn-local items walk under the outer context; not() proves \
             no platform; any() arms intersect; feature atoms dropped (the \
             cfg-audit family owns that class); types (struct/enum/trait) not \
             tracked, so a `use` row on a trait name (anyhow's `Context`) may be \
             method-consumed and same-named types elsewhere can masquerade — \
             adjudicate `use` rows before acting; [pub] rows may serve \
             crate-external consumers the scan cannot see — read the downstream hint."
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str) -> PlatformAuditReport {
        audit_sources(&[("t.rs".to_string(), src.to_string())])
    }

    fn missing_of(report: &PlatformAuditReport, name: &str) -> AtomSet {
        report
            .findings
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.missing.clone())
            .unwrap_or_default()
    }

    /// The katgpt-rs specimen, verbatim shape (fixed ea4c2873): ungated
    /// const, sole use inside a double-gated aarch64 fn. The feature atom
    /// drops; the arch atom condemns.
    #[test]
    fn neon_specimen_pre_fix_flags() {
        let report = one(r#"
const NEON_U8: usize = 16;
#[cfg(target_arch = "x86_64")]
const AVX2_U8: usize = 32;

#[cfg(target_arch = "aarch64")]
#[cfg(feature = "interval_pruner")]
unsafe fn neon_is_interval_closed(mask: &[bool]) -> bool {
    word_is_interval_closed(mask, NEON_U8)
}

fn word_is_interval_closed(mask: &[bool], width: usize) -> bool { mask.len() == width }
"#);
        let missing = missing_of(&report, "NEON_U8");
        assert_eq!(
            missing,
            one_atom("target_arch", "aarch64"),
            "NEON_U8 must flag dead outside aarch64"
        );
        // The properly gated sibling stays silent.
        assert!(missing_of(&report, "AVX2_U8").is_empty());
    }

    /// Post-fix shape: the decl carries the gate — silent (the `x86_64`
    /// caller keeps `word_is_interval_closed` two-platform).
    #[test]
    fn neon_specimen_post_fix_silent() {
        let report = one(r#"
#[cfg(target_arch = "aarch64")]
const NEON_U8: usize = 16;
#[cfg(target_arch = "x86_64")]
const AVX2_U8: usize = 32;

#[cfg(target_arch = "aarch64")]
#[cfg(feature = "interval_pruner")]
unsafe fn neon_is_interval_closed(mask: &[bool]) -> bool {
    word_is_interval_closed(mask, NEON_U8)
}

#[cfg(target_arch = "x86_64")]
unsafe fn avx2_is_interval_closed(mask: &[bool]) -> bool {
    word_is_interval_closed(mask, AVX2_U8)
}

fn word_is_interval_closed(mask: &[bool], width: usize) -> bool { mask.len() == width }
"#);
        assert_eq!(report.findings, []);
    }

    /// Uses on two different platforms intersect to nothing — silent.
    #[test]
    fn two_platform_uses_silent() {
        let report = one(r#"
const KEY: u32 = 7;
#[cfg(target_os = "macos")]
fn a() -> u32 { KEY }
#[cfg(target_os = "linux")]
fn b() -> u32 { KEY }
"#);
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// One ungated use saves the item even alongside gated uses.
    #[test]
    fn ungated_use_saves() {
        let report = one(r#"
const KEY: u32 = 7;
#[cfg(target_os = "macos")]
fn a() -> u32 { KEY }
fn b() -> u32 { KEY }
"#);
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// `any(macos, feature)` guarantees no platform — an any-arm use can
    /// only save, never condemn.
    #[test]
    fn any_arm_gives_no_guarantee() {
        let report = one(r#"
const KEY: u32 = 7;
#[cfg(any(target_os = "macos", feature = "gpu"))]
fn a() -> u32 { KEY }
"#);
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// `not(windows)` proves no positive atom — silent, and it breaks the
    /// intersection for other uses too (conservative).
    #[test]
    fn not_contributes_nothing() {
        let report = one(r#"
const KEY: u32 = 7;
#[cfg(not(target_os = "windows"))]
fn a() -> u32 { KEY }
"#);
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// Feature atoms drop; a surviving platform atom inside `all(...)`
    /// still condemns.
    #[test]
    fn feature_atoms_drop_but_all_keeps_platform() {
        let report = one(r#"
const KEY: u32 = 7;
#[cfg(all(unix, feature = "z"))]
fn a() -> u32 { KEY }
"#);
        assert_eq!(
            missing_of(&report, "KEY"),
            one_atom("target_family", "unix")
        );
    }

    /// Mod-stack cfgs attribute to decls and uses inside the mod.
    #[test]
    fn mod_cfg_stack() {
        let report = one(r#"
#[cfg(unix)]
mod m {
    pub const X: u32 = 3;
}
#[cfg(target_os = "macos")]
fn user() -> u32 { m::X }
"#);
        // Decl D = {family unix}; the only use sits under macos — the
        // item is dead on the non-macos unix hosts (linux).
        assert_eq!(missing_of(&report, "X"), one_atom("target_os", "macos"));
    }

    /// Recursion inside the fn's own body is not a use; one gated external
    /// call still condemns.
    #[test]
    fn fn_recursion_not_a_use() {
        let report = one(r#"
fn walk(n: u32) -> u32 {
    if n == 0 { 0 } else { walk(n - 1) }
}
#[cfg(target_arch = "wasm32")]
fn caller() -> u32 { walk(3) }
"#);
        assert_eq!(
            missing_of(&report, "walk"),
            one_atom("target_arch", "wasm32")
        );
    }

    /// Same name declared twice — ambiguous, skipped, never guessed.
    #[test]
    fn ambiguous_name_skipped() {
        let report = one(r#"
mod a { const MASK: u32 = 1; }
mod b { const MASK: u32 = 2; }
#[cfg(target_os = "macos")]
fn user() -> u32 { a::MASK + b::MASK }
"#);
        assert_eq!(report.findings, []);
        assert_eq!(report.ambiguous_names, 1);
    }

    /// The `ProcessStatus` specimen shape: an UNGATED import whose only
    /// code mentions sit in gated context — the binding is dead weight on
    /// every other platform. The import's own path must NOT count as a use
    /// of the local binding (it counts only for the source, below).
    #[test]
    fn ungated_import_with_gated_code_uses_flags() {
        let report = one(r#"
use std::sync::Mutex;

#[cfg(target_os = "macos")]
fn a(m: Mutex<()>) {}
"#);
        let f = report
            .findings
            .iter()
            .find(|f| f.name == "Mutex")
            .expect("the ungated import with gated uses must flag");
        assert_eq!(f.kind, "use");
        assert_eq!(f.missing, one_atom("target_os", "macos"));
    }

    /// A GATED import is silent as a decl (it does not compile where the
    /// gate is off — nothing is dead there), but its reference condemns
    /// the SOURCE when it is the only one. Through a RENAME only: a
    /// same-name import shares the name with the source, reads as
    /// ambiguity, and is skipped (the conservative direction — code
    /// mentions of that name are not attributable to either decl).
    #[test]
    fn gated_import_condemns_source_not_itself() {
        let report = audit_sources(&[
            (
                "file1.rs".to_string(),
                "pub const SPEED: u32 = 3;".to_string(),
            ),
            (
                "file2.rs".to_string(),
                "#[cfg(target_os = \"macos\")]
                 use file1::SPEED as CRUISE;"
                    .to_string(),
            ),
        ]);
        assert!(
            report.findings.iter().all(|f| f.name == "SPEED"),
            "the import decl itself stays silent"
        );
        let f = report
            .findings
            .iter()
            .find(|f| f.name == "SPEED")
            .expect("the source const must flag — its only reference is gated");
        assert_eq!(f.missing, one_atom("target_os", "macos"));
    }

    /// The same-name twin of the above: import and source share the name,
    /// the pair reads as ambiguous, and NEITHER flags (skipped, counted).
    #[test]
    fn same_name_import_and_source_is_ambiguous() {
        let report = audit_sources(&[
            (
                "file1.rs".to_string(),
                "pub const SPEED: u32 = 3;".to_string(),
            ),
            (
                "file2.rs".to_string(),
                "#[cfg(target_os = \"macos\")]
                 use file1::SPEED;"
                    .to_string(),
            ),
        ]);
        assert_eq!(report.findings, []);
        assert_eq!(report.ambiguous_names, 1);
    }

    /// A rename binds a different local name than the source it uses:
    /// the LOCAL binding flags when its code mentions are gated (the
    /// ungated-import shape under a rename).
    #[test]
    fn rename_import_tracks_both_names() {
        let report = one(r#"
use std::sync::Mutex as Locked;

#[cfg(target_os = "macos")]
fn a(m: Locked) {}
"#);
        assert_eq!(
            missing_of(&report, "Locked"),
            one_atom("target_os", "macos")
        );
    }

    /// The chaind `BrowserNodeState` specimen (2026-09-16 intake): a
    /// PRIVATE import inside `#[cfg(test)] mod tests` must not inherit
    /// platform atoms from out-of-scope same-name mentions — the wasm32
    /// type-path uses at file root reference the ENUM, not the test-scoped
    /// binding. Pre-fix the pair was internally inconsistent: the enum
    /// stayed clean BECAUSE the ungated import counted as an ungated
    /// reference to it, while the import itself flagged off atoms its
    /// scope does not guarantee.
    #[test]
    fn cfg_test_import_ignores_out_of_scope_uses() {
        let report = one(r#"
enum X { A, B }

#[cfg(target_arch = "wasm32")]
struct S { pub st: X }

#[cfg(target_arch = "wasm32")]
impl S { pub fn st(&self) -> X { X::A } }

#[cfg(test)]
mod tests {
    use super::X;

    #[test]
    fn t() {
        assert!(!X::A.eq(&X::B));
    }
}
"#);
        assert!(
            !report.findings.iter().any(|f| f.name == "X"),
            "the test-scoped import cannot carry wasm32 atoms from out-of-scope uses"
        );
    }

    /// The scoping cut must not lose the true positive — and it must
    /// RESTORE one the name-global law hid: a private import whose
    /// IN-SCOPE mentions are all gated flags even when an out-of-scope
    /// UNGATED same-name mention exists (that mention references the
    /// enum, not the binding; pre-fix it broke the intersection and
    /// suppressed the flag).
    #[test]
    fn private_import_in_scope_gated_uses_flag_despite_out_of_scope_ungated() {
        let report = one(r#"
enum Engine { A }

mod m {
    use super::Engine;

    #[cfg(target_os = "linux")]
    fn go(e: Engine) {}
}

fn root(e: Engine) {}
"#);
        assert_eq!(
            missing_of(&report, "Engine"),
            one_atom("target_os", "linux"),
            "in-scope gated evidence stands; the out-of-scope ungated mention no longer suppresses"
        );
    }

    /// A non-private import is a re-export surface — reachable from any
    /// module — so it keeps the name-global evidence: the out-of-scope
    /// ungated mention keeps the row silent (the private twin above
    /// flags).
    #[test]
    fn pub_reexport_keeps_name_global_evidence() {
        let report = one(r#"
enum Engine { A }

mod m {
    pub use super::Engine;

    #[cfg(target_os = "linux")]
    fn go(e: Engine) {}
}

fn root(e: Engine) {}
"#);
        assert!(
            !report.findings.iter().any(|f| f.name == "Engine"),
            "pub use rows keep the name-global evidence"
        );
    }

    /// Cross-file same-name mentions cannot reference a module-private
    /// import (it is invisible outside its file) — pre-fix a gated
    /// mention in ANOTHER file condemned the import off atoms from a
    /// binding that file cannot even see.
    #[test]
    fn private_import_ignores_cross_file_mentions() {
        let report = audit_sources(&[
            (
                "a.rs".to_string(),
                "enum E { A }\nmod m { use super::E; }".to_string(),
            ),
            (
                "b.rs".to_string(),
                "#[cfg(target_os = \"macos\")]\nfn f(e: E) {}".to_string(),
            ),
        ]);
        assert!(
            !report.findings.iter().any(|f| f.name == "E"),
            "cross-file mentions are not evidence for a private import"
        );
    }

    /// All uses in test context still flag (platform semantics), with the
    /// test-only marker for adjudication.
    #[test]
    fn test_only_marker() {
        let report = one(r#"
const KEY: u32 = 7;
#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    fn t() -> u32 { super::KEY }
}
"#);
        let f = report
            .findings
            .iter()
            .find(|f| f.name == "KEY")
            .expect("must flag");
        assert!(f.test_only);
    }

    /// pub declarations flag WITH the marker (external consumers are the
    /// reader's to check — the downstream hint).
    #[test]
    fn pub_marker() {
        let report = one(r#"
pub const KEY: u32 = 7;
#[cfg(target_os = "macos")]
fn a() -> u32 { KEY }
"#);
        let f = report
            .findings
            .iter()
            .find(|f| f.name == "KEY")
            .expect("must flag");
        assert!(f.is_pub);
    }

    /// Zero uses is rustc's own class, not ours — footer count only
    /// (both the const and the fn here are unused).
    #[test]
    fn zero_uses_is_footer_only() {
        let report = one(r#"
const UNUSED_HERE: u32 = 1;
#[cfg(target_os = "macos")]
fn a() -> u32 { 0 }
"#);
        assert_eq!(report.findings, []);
        assert_eq!(report.no_use_names, 2);
    }

    /// The frontier-report law: an empty scan refuses, never a green zero.
    #[test]
    fn empty_scan_refuses() {
        let report = audit_sources(&[]);
        assert!(report.blind.is_some());
    }

    /// Every file unparsed = blind (the parse leg died wholesale).
    #[test]
    fn all_unparsed_is_blind() {
        let report = one("this is not rust {{{");
        assert!(report.blind.is_some());
    }

    /// The static + bare-path + multi-segment use shapes all tally.
    #[test]
    fn static_and_qualified_use() {
        let report = one(r#"
mod constants { pub static POOL: u32 = 4; }
#[cfg(target_os = "windows")]
fn a() -> u32 { constants::POOL }
"#);
        assert_eq!(
            missing_of(&report, "POOL"),
            one_atom("target_os", "windows")
        );
    }

    /// P24 (a), DECL side: a `#![cfg]`-gated file's items are NOT
    /// platform-dead — the whole file compiles away elsewhere, so the
    /// first workspace sweep's 5 FP rows (the `ane_prefill` bank/exec/
    /// `exec_zc`/headroom shape) must go silent once the file gate folds
    /// into the decl's atoms.
    #[test]
    fn file_gate_shelters_whole_file_decls() {
        let report = one(r#"
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

pub const DEFAULT_MAX_ANE_BYTES: u64 = 13_000_000_000;

pub fn ane_stage_dump() -> String {
    format!("{}", DEFAULT_MAX_ANE_BYTES)
}
"#);
        assert!(
            report.findings.is_empty(),
            "a whole-file gate covers the decl AND its uses: {:?}",
            report.findings
        );
    }

    /// P24 (a), USE side (the rustc-proven miss): uses inside a
    /// `#![cfg]`-gated file are NOT ungated — an ungated decl in another
    /// file whose every use lives in file-gated modules must flag with
    /// the FILE's atoms (the `note_ane_dispatch` specimen).
    #[test]
    fn file_gate_on_use_side_flags_cross_file_row() {
        let report = audit_sources(&[
            (
                "mod.rs".to_string(),
                "pub(crate) fn note_ane_dispatch() { COUNT.fetch_add(1, Ordering::Relaxed); }"
                    .to_string(),
            ),
            (
                "exec.rs".to_string(),
                r#"#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn arm() { super::note_ane_dispatch(); }
"#
                .to_string(),
            ),
        ]);
        let missing = missing_of(&report, "note_ane_dispatch");
        let mut both = one_atom("target_os", "macos");
        both.insert(Atom {
            key: "target_arch".to_string(),
            value: "aarch64".to_string(),
        });
        assert_eq!(
            missing, both,
            "a file-gated use is a gated use — the decl must flag with BOTH file atoms"
        );
    }

    /// P24 (a) test leg: `#![cfg(all(test, target_arch = "aarch64"))]` at
    /// file level is BOTH a platform gate (the atoms fold) and test
    /// context (the row marks test-only).
    #[test]
    fn file_cfg_test_marks_test_only() {
        let report = audit_sources(&[
            (
                "lib.rs".to_string(),
                "pub fn dump() -> String { String::new() }".to_string(),
            ),
            (
                "probe.rs".to_string(),
                "#![cfg(all(test, target_arch = \"aarch64\"))]\nmod t { #[test] fn a() { super::super::dump(); } }"
                    .to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "dump"),
            one_atom("target_arch", "aarch64"),
            "the file gate's platform atoms fold into the use context"
        );
        let row = report
            .findings
            .iter()
            .find(|f| f.name == "dump")
            .expect("row present");
        assert!(row.test_only, "#![cfg(test)] file marks its uses test-only");
    }

    /// P24 (b), DECL side (the seal-view `web_link` / mmorpg `wasm.rs`
    /// FP class): a file declared by a parent's gated `mod x;` row
    /// inherits the row's atoms — its decls are gated, not platform-dead.
    #[test]
    fn parent_mod_row_gate_shelters_child_decls() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                r#"#[cfg(all(target_arch = "wasm32", feature = "mesh_visuals"))]
pub mod web_link;
"#
                .to_string(),
            ),
            (
                "src/web_link.rs".to_string(),
                "pub fn wire_connect() -> u8 { 7 }".to_string(),
            ),
            (
                "src/main.rs".to_string(),
                "#[cfg(target_arch = \"wasm32\")] fn boot() { crate::web_link::wire_connect(); }"
                    .to_string(),
            ),
        ]);
        assert!(
            report.findings.is_empty(),
            "the parent's mod-row gate covers the child file's decls: {:?}",
            report.findings
        );
        assert_eq!(
            report.files_modrow_gated, 1,
            "exactly web_link.rs gained atoms from its incoming row"
        );
    }

    /// Stacked cfg attrs on one mod row (the mmorpg shape:
    /// `#[cfg(wasm32)]` + `#[cfg(feature)]` on separate lines) fold the
    /// same way — multiple cfg attrs union.
    #[test]
    fn stacked_cfg_attrs_on_mod_row_fold() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                "#[cfg(target_arch = \"wasm32\")]
#[cfg(feature = \"bevy_backend\")]
pub mod wasm;"
                    .to_string(),
            ),
            (
                "src/wasm.rs".to_string(),
                "pub fn run_wasm() -> u8 { 3 }".to_string(),
            ),
            (
                "src/main.rs".to_string(),
                "#[cfg(target_arch = \"wasm32\")] fn b() { crate::wasm::run_wasm(); }".to_string(),
            ),
        ]);
        assert!(
            report.findings.is_empty(),
            "stacked cfg attrs on the mod row gate the child file: {:?}",
            report.findings
        );
    }

    /// P24 (b), USE side (the reverse direction): uses inside a
    /// mod-row-gated file are NOT ungated — an ungated decl elsewhere
    /// whose every use lives in the gated child must flag with the row's
    /// atoms.
    #[test]
    fn parent_mod_row_gate_on_use_side_flags_cross_file_row() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                "#[cfg(target_os = \"macos\")] pub mod ane;".to_string(),
            ),
            (
                "src/ane.rs".to_string(),
                "pub fn arm() -> u8 { crate::POLL }".to_string(),
            ),
            (
                "src/other.rs".to_string(),
                "pub const POLL: u8 = 1;".to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "POLL"),
            one_atom("target_os", "macos"),
            "a use inside a mod-row-gated file carries the row's atoms"
        );
    }

    /// Transitivity: a chain of mod rows accumulates gates down the tree
    /// (grandparent's gate ∪ parent's gate), including the non-root
    /// `src/a.rs` → `src/a/b.rs` resolution.
    #[test]
    fn mod_row_gates_accumulate_down_the_chain() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                "#[cfg(unix)] pub mod a;".to_string(),
            ),
            (
                "src/a.rs".to_string(),
                "#[cfg(target_arch = \"aarch64\")] pub mod b;".to_string(),
            ),
            (
                "src/a/b.rs".to_string(),
                "pub fn deep() -> u8 { crate::SEED }".to_string(),
            ),
            (
                "src/other.rs".to_string(),
                "pub const SEED: u8 = 2;".to_string(),
            ),
        ]);
        let mut expect = one_atom("target_family", "unix");
        expect.insert(Atom {
            key: "target_arch".to_string(),
            value: "aarch64".to_string(),
        });
        assert_eq!(
            missing_of(&report, "SEED"),
            expect,
            "the child inherits BOTH the grandparent and parent mod-row gates"
        );
    }

    /// The `mod.rs` shape: `src/util/mod.rs` declaring `mod win;` resolves
    /// to `src/util/win.rs` (children in the mod.rs's own directory).
    #[test]
    fn mod_rs_parent_resolves_sibling_dir() {
        let report = audit_sources(&[
            ("src/lib.rs".to_string(), "pub mod util;".to_string()),
            (
                "src/util/mod.rs".to_string(),
                "#[cfg(windows)] pub mod win;".to_string(),
            ),
            (
                "src/util/win.rs".to_string(),
                "pub fn w() -> u8 { crate::K }".to_string(),
            ),
            (
                "src/other.rs".to_string(),
                "pub const K: u8 = 3;".to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "K"),
            one_atom("target_family", "windows"),
            "`#[cfg(windows)]` on the mod row folds through the mod.rs shape"
        );
    }

    /// ⛔ The SEPARATOR axis. `Path::join` emits `\` on Windows while every
    /// source key here uses `/` — from `git ls-files` (P25) or a caller's
    /// literal — and a real walk produces MIXED keys
    /// (`E:\repo\src/util/mod.rs`), so normalising one side is not enough.
    /// Unresolved mod rows do not error: the lattice degrades to zero, every
    /// gated item reads as ungated, and the audit INVENTS platform-dead
    /// findings. 8 fixtures in this module were red on Windows and green
    /// everywhere else, silently, because nothing in this workspace runs
    /// this crate's tests there.
    ///
    /// ⚠ This arm is itself PLATFORM-DEPENDENT and that is a limit, not an
    /// oversight: perturbation reds it on Windows and it is VACUOUS on
    /// POSIX, where `\` is an ordinary filename character and `Path::join`
    /// reproduces the `/` form regardless. An arm that red everywhere would
    /// need an all-`\` key set, and that one passes on Windows and reds on
    /// POSIX for the opposite reason — a false failure, not a canary. There
    /// is no separator fixture that is sound on both.
    #[test]
    fn mod_rows_resolve_through_mixed_path_separators() {
        let report = audit_sources(&[
            (
                r"E:\repo\src/lib.rs".to_string(),
                "pub mod util;".to_string(),
            ),
            (
                r"E:\repo\src/util/mod.rs".to_string(),
                "#[cfg(windows)] pub mod win;".to_string(),
            ),
            (
                r"E:\repo\src/util/win.rs".to_string(),
                "pub fn w() -> u8 { crate::K }".to_string(),
            ),
            (
                r"E:\repo\src/other.rs".to_string(),
                "pub const K: u8 = 3;".to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "K"),
            one_atom("target_family", "windows"),
            "a mixed-separator key set must resolve its mod rows"
        );
    }

    /// Inline-mod nesting in the parent: `mod shell { pub mod inner; }`
    /// resolves the child to `src/shell/inner.rs`, carrying the inline
    /// mod's cfg into the bargain.
    #[test]
    fn inline_mod_nesting_resolves_and_carries_cfg() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                "#[cfg(unix)] mod shell { pub mod inner; }".to_string(),
            ),
            (
                "src/shell/inner.rs".to_string(),
                "pub fn i() -> u8 { crate::K2 }".to_string(),
            ),
            (
                "src/other.rs".to_string(),
                "pub const K2: u8 = 5;".to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "K2"),
            one_atom("target_family", "unix"),
            "the inline mod's cfg AND its directory nesting both resolve"
        );
    }

    /// Ambiguity law: disagreeing incoming rows (the lib.rs + main.rs
    /// shape — a crate carries both roots, both may declare the same
    /// module name) skip the fold; the conservative direction.
    #[test]
    fn disagreeing_incoming_rows_skip_the_fold() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                "#[cfg(unix)] pub mod dup;".to_string(),
            ),
            (
                "src/main.rs".to_string(),
                "#[cfg(windows)] pub mod dup;".to_string(),
            ),
            (
                "src/dup.rs".to_string(),
                "pub fn d() -> u8 { 1 }".to_string(),
            ),
            (
                "src/user.rs".to_string(),
                "#[cfg(unix)] fn u() { crate::dup::d(); }".to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "d"),
            one_atom("target_family", "unix"),
            "ambiguous parenthood leaves the decl ungated — still flaggable"
        );
        assert_eq!(
            report.files_modrow_gated, 0,
            "the disagreeing edge set folds nothing"
        );
    }

    /// `#[path]`-carrying rows are skipped — the child lives at a
    /// nonstandard path this fold does not resolve (no-fold direction).
    #[test]
    fn path_attr_rows_are_skipped() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                "#[cfg(unix)] #[path = \"elsewhere.rs\"] pub mod gone;".to_string(),
            ),
            (
                "src/gone.rs".to_string(),
                "pub fn g() -> u8 { 1 }".to_string(),
            ),
            (
                "src/user.rs".to_string(),
                "#[cfg(unix)] fn u() { crate::gone::g(); }".to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "g"),
            one_atom("target_family", "unix"),
            "a #[path] row leaves the standard-path file unfolded"
        );
    }

    /// The test-flag leg: `all(test, arch)` on the mod row marks the
    /// child's uses test-only AND folds the platform atom.
    #[test]
    fn mod_row_test_gate_marks_test_only() {
        let report = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                "#[cfg(all(test, target_arch = \"aarch64\"))] pub mod foo;".to_string(),
            ),
            (
                "src/foo.rs".to_string(),
                "pub fn helper() -> u8 { crate::BASE }".to_string(),
            ),
            (
                "src/other.rs".to_string(),
                "pub const BASE: u8 = 4;".to_string(),
            ),
        ]);
        assert_eq!(
            missing_of(&report, "BASE"),
            one_atom("target_arch", "aarch64"),
            "the mod row's platform atom folds into the use context"
        );
        let row = report
            .findings
            .iter()
            .find(|f| f.name == "BASE")
            .expect("row present");
        assert!(
            row.test_only,
            "#[cfg(test)] on the mod row marks the child's uses test-only"
        );
    }

    /// The bin-root shape: `src/bin/tool.rs` is its own crate root (a
    /// direct `.rs` child of a `bin` dir), so its children resolve in
    /// `src/bin/` — not `src/bin/tool/`.
    #[test]
    fn bin_root_children_resolve_in_bin_dir() {
        let report = audit_sources(&[
            (
                "src/bin/tool.rs".to_string(),
                "#[cfg(unix)] pub mod util;".to_string(),
            ),
            (
                "src/bin/util.rs".to_string(),
                "pub fn u2() -> u8 { 9 }".to_string(),
            ),
        ]);
        assert_eq!(
            report.files_modrow_gated, 1,
            "the bin root's mod row folds into src/bin/util.rs"
        );
    }

    /// P24 (d) (the `ane_npc_goat` `SWEEP_COUNTS` specimen): the only
    /// ungated use is an inline `{X:?}` capture inside a format-family
    /// macro — counted, so the gated uses cannot condemn the decl alone.
    #[test]
    fn inline_format_capture_counts_as_ungated_use() {
        let report = one(r#"
const SWEEP_COUNTS: [usize; 3] = [10, 100, 1000];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn sweep() -> usize { SWEEP_COUNTS[0] }

fn main() {
    println!("Sweep sizes: {SWEEP_COUNTS:?}");
}
"#);
        assert!(
            report.findings.is_empty(),
            "the inline capture is an ungated use: {:?}",
            report.findings
        );
    }

    /// P24 (d) arg-scan leg (the chaind FPs): a native test use inside
    /// `assert_eq!` args is a real use — it must break an all-wasm32
    /// intersection, not hide behind the macro.
    #[test]
    fn assert_family_args_count_as_uses() {
        let report = one(r#"
const SIGNALING: u8 = 0x20;

#[cfg(target_arch = "wasm32")]
fn wasm_arm() -> u8 { SIGNALING }

#[cfg(test)]
mod tests {
    #[test]
    fn constants() { assert_eq!(super::SIGNALING, 0x20); }
}
"#);
        assert!(
            report.findings.is_empty(),
            "the assert_eq! arg use breaks the all-wasm32 intersection: {:?}",
            report.findings
        );
    }

    /// P24 (d) negative controls, same shape as the specimen but with
    /// NON-capture braces: doubled `{{X}}` (literal braces), positional
    /// `{0}`, and a braced word in a NON-format macro — none of them
    /// count, so the row still flags.
    #[test]
    fn non_capture_braces_do_not_count() {
        let base = r#"
const X: usize = 1;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn arm() -> usize { X }
"#;
        for (label, tail) in [
            ("doubled brace", "fn m() { println!(\"{{X}} literal\"); }"),
            ("positional", "fn m() { println!(\"{0}\", 7); }"),
            ("non-format macro", "fn m() { shader!(\"{X} is data\"); }"),
        ] {
            let report = one(&format!("{base}{tail}"));
            assert!(
                !missing_of(&report, "X").is_empty(),
                "{label}: no capture counted — the row must still flag"
            );
        }
    }
}
