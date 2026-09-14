//! `cargo heal --platform-audit` — post-mining intake class P22: the
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
//!   sibling audit (config-audit / orphan-report).
//! * **Module-DECLARATION gates (`#[cfg] mod x;` in the parent) are
//!   invisible** — the child file is scanned as its own root. A gate on
//!   the USE side then under-approximates (misses); on the DECL side it
//!   can suppress a flag. **File-level `#![cfg]` INNER attrs ARE folded**
//!   (P24 (a), 2026-09-15): the file's atoms seed the root context for
//!   both decls and uses — the first workspace sweep measured 5 FP rows
//!   (decls in `#![cfg(all(macos, aarch64))]` files read ungated) and 1
//!   rustc-proven miss (file-gated uses read UNGATED) from this one
//!   blindness, in both directions. The parent-side `mod x;` row remains
//!   open (P24 (b)).
//! * **Macro token streams are not descended** (the P13 lesson) — a use
//!   inside `assert!(..)`, `println!` or any macro body is invisible.
//!   The dangerous direction: a macro-only UNGATED use breaks the
//!   intersection silently and suppresses a flag (a miss, not a false
//!   positive). A macro-only GATED use simply reads as fewer uses.
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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::config_audit::{attrs_cfg_test, attrs_test_fn, read_sources};

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
    /// Refusal reason (the frontier-report law): empty scan, never a
    /// confident green zero.
    pub blind: Option<String>,
}

/// Scan `paths` (dirs recursively / files) and classify.
#[must_use]
pub fn audit_paths(paths: &[PathBuf]) -> PlatformAuditReport {
    let sources = read_sources(paths);
    audit_sources(&sources)
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

    // name → decl records; name → use evidence, split by REFERENCE KIND:
    // code mentions (path occurrences) vs import references. A `use a::C`
    // resolves on every platform the IMPORT compiles, so the import's own
    // path is a real use of the SOURCE const — but it says nothing about
    // the LOCAL binding (an ungated import whose only code mentions are
    // gated is exactly the ProcessStatus specimen). use-decls therefore
    // read only code evidence; value decls read the union.
    let mut decls: BTreeMap<String, Vec<DeclRec>> = BTreeMap::new();
    let mut code_uses: BTreeMap<String, UseEvidence> = BTreeMap::new();
    let mut src_uses: BTreeMap<String, UseEvidence> = BTreeMap::new();

    for (file, src) in sources {
        let Ok(parsed) = syn::parse_file(src) else {
            report.files_unparsed += 1;
            continue;
        };
        // File-level `#![cfg]` INNER attrs gate the whole file (the
        // ane_prefill bank/exec/exec_zc/headroom shape — the P24 (a) fold).
        // Seeding the root context fixes BOTH directions at once: decls
        // recorded under it inherit the gate into their atoms (a gated
        // file's items are not platform-dead — they do not even exist
        // elsewhere), and uses inside it carry the gate into their evidence
        // (a file-gated use is NOT an ungated use — the note_ane_dispatch
        // miss). `#![cfg(test)]` at file level is test context, same as an
        // item-level one.
        let ctx = Ctx {
            atoms: guaranteed_from_attrs(&parsed.attrs),
            in_test: attrs_cfg_test(&parsed.attrs),
        };
        walk_items(
            &parsed.items,
            &ctx,
            &mut decls,
            &mut code_uses,
            &mut src_uses,
            file,
        );
    }

    if report.files_unparsed == sources.len() {
        report.blind = Some("every scanned file failed to parse".into());
        return report;
    }

    report.decls_tracked = decls.values().map(Vec::len).sum();
    for (name, ds) in &decls {
        if ds.len() != 1 {
            report.ambiguous_names += 1;
            continue;
        }
        let d = &ds[0];
        let merged: Vec<&UseEvidence> = if d.kind == "use" {
            code_uses
                .get(name)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            let mut v: Vec<&UseEvidence> = Vec::new();
            if let Some(e) = code_uses.get(name) {
                v.push(e);
            }
            if let Some(e) = src_uses.get(name) {
                v.push(e);
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
    kind: &'static str,
    origin: String,
    atoms: AtomSet,
    is_pub: bool,
}

#[derive(Default)]
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

/// Recursively walk module-level items: record tracked declarations,
/// recurse inline mods, and collect path uses from everything else.
fn walk_items(
    items: &[syn::Item],
    ctx: &Ctx,
    decls: &mut BTreeMap<String, Vec<DeclRec>>,
    code_uses: &mut BTreeMap<String, UseEvidence>,
    src_uses: &mut BTreeMap<String, UseEvidence>,
    file: &str,
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
                    walk_items(inner, &ctx2, decls, code_uses, src_uses, file);
                }
                // External `mod x;` — the child file is scanned as its own
                // root; its gate is invisible there (documented).
            }
            syn::Item::Const(c) => {
                record_decl(
                    decls,
                    &c.ident.to_string(),
                    "const",
                    format!("{}:{}", file, c.ident.span().start().line),
                    ctx2.atoms.clone(),
                    !matches!(c.vis, syn::Visibility::Inherited),
                );
                collect_uses(item, &ctx2, code_uses, &BTreeSet::new());
            }
            syn::Item::Static(s) => {
                record_decl(
                    decls,
                    &s.ident.to_string(),
                    "static",
                    format!("{}:{}", file, s.ident.span().start().line),
                    ctx2.atoms.clone(),
                    !matches!(s.vis, syn::Visibility::Inherited),
                );
                collect_uses(item, &ctx2, code_uses, &BTreeSet::new());
            }
            syn::Item::Fn(f) => {
                let name = f.sig.ident.to_string();
                record_decl(
                    decls,
                    &name,
                    "fn",
                    format!("{}:{}", file, f.sig.ident.span().start().line),
                    ctx2.atoms.clone(),
                    !matches!(f.vis, syn::Visibility::Inherited),
                );
                // Recursion inside the fn's own body is not a use.
                let mut skip = BTreeSet::new();
                skip.insert(name);
                collect_uses(item, &ctx2, code_uses, &skip);
            }
            syn::Item::Use(u) => {
                for leaf in use_leaves(&u.tree) {
                    record_decl(
                        decls,
                        &leaf.local,
                        "use",
                        format!("{}:{}", file, u.span().start().line),
                        ctx2.atoms.clone(),
                        !matches!(u.vis, syn::Visibility::Inherited),
                    );
                    // The import IS a reference to its source name, in
                    // the import's cfg context — a gated import is the
                    // source's only-gated-reference shape. It does NOT
                    // count for the local binding (an ungated import
                    // whose only code mentions are gated is the specimen
                    // class and must stay flaggable).
                    tally_use(src_uses, &leaf.source, &ctx2, 1);
                }
            }
            _ => collect_uses(item, &ctx2, code_uses, &BTreeSet::new()),
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

fn record_decl(
    decls: &mut BTreeMap<String, Vec<DeclRec>>,
    name: &str,
    kind: &'static str,
    origin: String,
    atoms: AtomSet,
    is_pub: bool,
) {
    decls.entry(name.to_string()).or_default().push(DeclRec {
        kind,
        origin,
        atoms,
        is_pub,
    });
}

fn tally_use(uses: &mut BTreeMap<String, UseEvidence>, name: &str, ctx: &Ctx, n: usize) {
    let e = uses.entry(name.to_string()).or_default();
    e.contexts.insert((ctx.atoms.clone(), ctx.in_test));
    e.total += n;
}

/// Collect every path occurrence inside an item with one cfg context.
/// Nested fn-local items are not decl-tracked (documented); their contents
/// attribute to this context — under-approximation, the safe direction.
fn collect_uses(
    item: &syn::Item,
    ctx: &Ctx,
    uses: &mut BTreeMap<String, UseEvidence>,
    skip: &BTreeSet<String>,
) {
    let mut visitor = UseVisitor {
        found: BTreeMap::new(),
        skip,
    };
    visitor.visit_item(item);
    for (name, n) in visitor.found {
        tally_use(uses, &name, ctx, n);
    }
}

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
            "scanned {} file(s) ({} unparsed), {} tracked decl(s) (const/static/fn/use)",
            self.files_scanned, self.files_unparsed, self.decls_tracked
        );
        if let Some(reason) = &self.blind {
            let _ = writeln!(out, "\n⛔ REFUSING: {reason} — never a confident green zero.");
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
             rows against the listed atoms; file-level `#![cfg]` inner attrs folded \
             into the root context (P24 (a)) but parent-side `#[cfg] mod x;` gates \
             remain invisible (both sides under-approximate); macro token streams not \
             descended (a macro-only ungated use suppresses a flag — a miss, not \
             an FP; inline `{{name}}` format captures are that shape too); fn-local items walk under the outer context; not() proves \
             no platform; any() arms intersect; feature atoms dropped (the \
             cfg-audit family owns that class); types (struct/enum/trait) not \
             tracked; [pub] rows may serve crate-external consumers the scan \
             cannot see — read the downstream hint."
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
        let report = one(
            r#"
const NEON_U8: usize = 16;
#[cfg(target_arch = "x86_64")]
const AVX2_U8: usize = 32;

#[cfg(target_arch = "aarch64")]
#[cfg(feature = "interval_pruner")]
unsafe fn neon_is_interval_closed(mask: &[bool]) -> bool {
    word_is_interval_closed(mask, NEON_U8)
}

fn word_is_interval_closed(mask: &[bool], width: usize) -> bool { mask.len() == width }
"#,
        );
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
        let report = one(
            r#"
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
"#,
        );
        assert!(report.findings.is_empty());
    }

    /// Uses on two different platforms intersect to nothing — silent.
    #[test]
    fn two_platform_uses_silent() {
        let report = one(
            r#"
const KEY: u32 = 7;
#[cfg(target_os = "macos")]
fn a() -> u32 { KEY }
#[cfg(target_os = "linux")]
fn b() -> u32 { KEY }
"#,
        );
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// One ungated use saves the item even alongside gated uses.
    #[test]
    fn ungated_use_saves() {
        let report = one(
            r#"
const KEY: u32 = 7;
#[cfg(target_os = "macos")]
fn a() -> u32 { KEY }
fn b() -> u32 { KEY }
"#,
        );
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// `any(macos, feature)` guarantees no platform — an any-arm use can
    /// only save, never condemn.
    #[test]
    fn any_arm_gives_no_guarantee() {
        let report = one(
            r#"
const KEY: u32 = 7;
#[cfg(any(target_os = "macos", feature = "gpu"))]
fn a() -> u32 { KEY }
"#,
        );
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// `not(windows)` proves no positive atom — silent, and it breaks the
    /// intersection for other uses too (conservative).
    #[test]
    fn not_contributes_nothing() {
        let report = one(
            r#"
const KEY: u32 = 7;
#[cfg(not(target_os = "windows"))]
fn a() -> u32 { KEY }
"#,
        );
        assert!(missing_of(&report, "KEY").is_empty());
    }

    /// Feature atoms drop; a surviving platform atom inside `all(...)`
    /// still condemns.
    #[test]
    fn feature_atoms_drop_but_all_keeps_platform() {
        let report = one(
            r#"
const KEY: u32 = 7;
#[cfg(all(unix, feature = "z"))]
fn a() -> u32 { KEY }
"#,
        );
        assert_eq!(
            missing_of(&report, "KEY"),
            one_atom("target_family", "unix")
        );
    }

    /// Mod-stack cfgs attribute to decls and uses inside the mod.
    #[test]
    fn mod_cfg_stack() {
        let report = one(
            r#"
#[cfg(unix)]
mod m {
    pub const X: u32 = 3;
}
#[cfg(target_os = "macos")]
fn user() -> u32 { m::X }
"#,
        );
        // Decl D = {family unix}; the only use sits under macos — the
        // item is dead on the non-macos unix hosts (linux).
        assert_eq!(
            missing_of(&report, "X"),
            one_atom("target_os", "macos")
        );
    }

    /// Recursion inside the fn's own body is not a use; one gated external
    /// call still condemns.
    #[test]
    fn fn_recursion_not_a_use() {
        let report = one(
            r#"
fn walk(n: u32) -> u32 {
    if n == 0 { 0 } else { walk(n - 1) }
}
#[cfg(target_arch = "wasm32")]
fn caller() -> u32 { walk(3) }
"#,
        );
        assert_eq!(
            missing_of(&report, "walk"),
            one_atom("target_arch", "wasm32")
        );
    }

    /// Same name declared twice — ambiguous, skipped, never guessed.
    #[test]
    fn ambiguous_name_skipped() {
        let report = one(
            r#"
mod a { const MASK: u32 = 1; }
mod b { const MASK: u32 = 2; }
#[cfg(target_os = "macos")]
fn user() -> u32 { a::MASK + b::MASK }
"#,
        );
        assert!(report.findings.is_empty());
        assert_eq!(report.ambiguous_names, 1);
    }

    /// The `ProcessStatus` specimen shape: an UNGATED import whose only
    /// code mentions sit in gated context — the binding is dead weight on
    /// every other platform. The import's own path must NOT count as a use
    /// of the local binding (it counts only for the source, below).
    #[test]
    fn ungated_import_with_gated_code_uses_flags() {
        let report = one(
            r#"
use std::sync::Mutex;

#[cfg(target_os = "macos")]
fn a(m: Mutex<()>) {}
"#,
        );
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
        assert!(report
            .findings
            .iter()
            .all(|f| f.name == "SPEED"),
            "the import decl itself stays silent");
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
        assert!(report.findings.is_empty());
        assert_eq!(report.ambiguous_names, 1);
    }

    /// A rename binds a different local name than the source it uses:
    /// the LOCAL binding flags when its code mentions are gated (the
    /// ungated-import shape under a rename).
    #[test]
    fn rename_import_tracks_both_names() {
        let report = one(
            r#"
use std::sync::Mutex as Locked;

#[cfg(target_os = "macos")]
fn a(m: Locked) {}
"#,
        );
        assert_eq!(
            missing_of(&report, "Locked"),
            one_atom("target_os", "macos")
        );
    }

    /// All uses in test context still flag (platform semantics), with the
    /// test-only marker for adjudication.
    #[test]
    fn test_only_marker() {
        let report = one(
            r#"
const KEY: u32 = 7;
#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    fn t() -> u32 { super::KEY }
}
"#,
        );
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
        let report = one(
            r#"
pub const KEY: u32 = 7;
#[cfg(target_os = "macos")]
fn a() -> u32 { KEY }
"#,
        );
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
        let report = one(
            r#"
const UNUSED_HERE: u32 = 1;
#[cfg(target_os = "macos")]
fn a() -> u32 { 0 }
"#,
        );
        assert!(report.findings.is_empty());
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
        let report = one(
            r#"
mod constants { pub static POOL: u32 = 4; }
#[cfg(target_os = "windows")]
fn a() -> u32 { constants::POOL }
"#,
        );
        assert_eq!(
            missing_of(&report, "POOL"),
            one_atom("target_os", "windows")
        );
    }

    /// P24 (a), DECL side: a `#![cfg]`-gated file's items are NOT
    /// platform-dead — the whole file compiles away elsewhere, so the
    /// first workspace sweep's 5 FP rows (the ane_prefill bank/exec/
    /// exec_zc/headroom shape) must go silent once the file gate folds
    /// into the decl's atoms.
    #[test]
    fn file_gate_shelters_whole_file_decls() {
        let report = one(
            r#"
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

pub const DEFAULT_MAX_ANE_BYTES: u64 = 13_000_000_000;

pub fn ane_stage_dump() -> String {
    format!("{}", DEFAULT_MAX_ANE_BYTES)
}
"#,
        );
        assert!(
            report.findings.is_empty(),
            "a whole-file gate covers the decl AND its uses: {:?}",
            report.findings
        );
    }

    /// P24 (a), USE side (the rustc-proven miss): uses inside a
    /// `#![cfg]`-gated file are NOT ungated — an ungated decl in another
    /// file whose every use lives in file-gated modules must flag with
    /// the FILE's atoms (the note_ane_dispatch specimen).
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
}
