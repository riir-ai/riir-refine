//! Issue 135 — the effective-lint-level gate for `cargo heal --fix`.
//!
//! The healer's proposers fire on source SHAPE, not on the target crate's
//! lint configuration. For a default-on lint that is the same thing; for an
//! allow-by-default one (pedantic / nursery / restriction / cargo) it is
//! not: `doc_markdown` is PEDANTIC, a crate that never enables pedantic has
//! no `doc_markdown` finding at all, and "fixing" it rewrites code the
//! crate's own clippy says is clean (the reflex-site `466e331` artifact —
//! a doc-comment shell command gained a backticked argv token). The old
//! justification was riir-clippy's own `#![warn(clippy::pedantic)]` root —
//! a property of THIS crate, applied to every crate the global binary is
//! pointed at.
//!
//! Two halves, both pure over their inputs:
//! - [`LINT_GROUPS`] — the clippy group of every `FIXABLE_LINTS ∪
//!   VERIFY_LINTS` lint, derived from the PINNED driver's `-W help`
//!   (clippy 0.1.98 / toolchain 1.98.1), never from the corpus: the corpus
//!   `lint_group` column had 22 stale labels on the day this landed
//!   (`unnecessary_map_or` read pedantic, is style; `collapsible_else_if` /
//!   `range_plus_one` read style, are pedantic; `use_self` read style, is
//!   nursery). A gate keyed on a stale label drops default-on fixes or
//!   applies allow-by-default ones. Tests pin coverage (every fix-set lint
//!   has a row), the corpus agreeing with the table, and — when the driver
//!   is installed — the table agreeing with the driver.
//! - [`LintLevels`] — the target crate's effective level for a lint, read
//!   from the places a crate can raise one: `[lints.clippy]` in the
//!   package manifest (or `[workspace.lints.clippy]` when the package says
//!   `[lints] workspace = true`), then the crate root's inner
//!   `#![warn|deny|forbid|expect|allow(clippy::…)]`, then the edited file's
//!   own inner attributes (a module scope). Later sources override earlier
//!   ones, the rustc precedence (source attributes beat command-line lint
//!   flags; `forbid` is sticky).
//!
//! ## Documented divergences (all skip-direction — loud, never silent)
//!
//! - `RUSTFLAGS` / `.cargo/config.toml` `rustflags = ["-W", "clippy::…"]`
//!   are not read, nor are `cfg_attr(…, warn(…))` attributes or the inner
//!   attributes of intermediate parent modules. A crate enabling pedantic
//!   only that way reads as not enabling it: the edits are SKIPPED and
//!   reported, and `--groups pedantic` restores them in one flag.
//! - A module under `src/` is attributed to `src/lib.rs` when it exists,
//!   else `src/main.rs` (a package carrying both can compile a module into
//!   either; the lib root is the common case).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The toolchain [`LINT_GROUPS`] was derived from (`rust-toolchain.toml`).
pub const PINNED_TOOLCHAIN: &str = "1.98.1";

/// The clippy groups whose lints are allow-by-default.
pub const ALLOW_BY_DEFAULT_GROUPS: &[&str] = &["pedantic", "nursery", "restriction", "cargo"];

/// `(lint, group)` for every lint in `FIXABLE_LINTS ∪ VERIFY_LINTS`, sorted
/// by lint name. Derived from `clippy-driver -W help` at [`PINNED_TOOLCHAIN`]
/// — re-derive on a toolchain bump (the `table_matches_installed_driver`
/// test names every disagreement).
pub const LINT_GROUPS: &[(&str, &str)] = &[
    ("assign_op_pattern", "style"),
    ("bool_comparison", "complexity"),
    ("byte_char_slices", "style"),
    ("bytes_count_to_len", "complexity"),
    ("cast_abs_to_unsigned", "suspicious"),
    ("cast_lossless", "pedantic"),
    ("chunks_exact_to_as_chunks", "style"),
    ("clone_on_copy", "complexity"),
    ("collapsible_else_if", "pedantic"),
    ("collapsible_if", "style"),
    ("comparison_to_empty", "style"),
    ("deref_addrof", "complexity"),
    ("doc_lazy_continuation", "style"),
    ("doc_markdown", "pedantic"),
    ("doc_overindented_list_items", "style"),
    ("double_parens", "complexity"),
    ("empty_line_after_doc_comments", "suspicious"),
    ("excessive_precision", "style"),
    ("filter_map_bool_then", "style"),
    ("filter_next", "complexity"),
    ("format_collect", "pedantic"),
    ("from_str_radix_10", "style"),
    ("identity_op", "complexity"),
    ("into_iter_on_ref", "style"),
    ("io_other_error", "style"),
    ("items_after_statements", "pedantic"),
    ("iter_next_loop", "correctness"),
    ("iter_nth_zero", "style"),
    ("iter_skip_next", "style"),
    ("legacy_numeric_constants", "style"),
    ("len_without_is_empty", "style"),
    ("let_and_return", "style"),
    ("manual_checked_ops", "complexity"),
    ("manual_clamp", "complexity"),
    ("manual_is_multiple_of", "complexity"),
    ("manual_let_else", "pedantic"),
    ("manual_map", "style"),
    ("manual_range_contains", "style"),
    ("manual_range_patterns", "complexity"),
    ("manual_slice_fill", "style"),
    ("manual_string_new", "pedantic"),
    ("manual_unwrap_or", "complexity"),
    ("map_clone", "style"),
    ("map_flatten", "complexity"),
    ("map_unwrap_or", "pedantic"),
    ("match_bool", "pedantic"),
    ("match_like_matches_macro", "style"),
    ("match_ref_pats", "style"),
    ("needless_bool", "complexity"),
    ("needless_bool_assign", "complexity"),
    ("needless_late_init", "style"),
    ("needless_lifetimes", "complexity"),
    ("needless_match", "complexity"),
    ("needless_pass_by_ref_mut", "nursery"),
    ("needless_range_loop", "style"),
    ("needless_return", "style"),
    ("new_without_default", "style"),
    ("partialeq_to_none", "style"),
    ("print_with_newline", "style"),
    ("ptr_arg", "style"),
    ("question_mark", "style"),
    ("range_plus_one", "pedantic"),
    ("redundant_closure", "style"),
    ("redundant_closure_for_method_calls", "pedantic"),
    ("redundant_field_names", "style"),
    ("redundant_pattern_matching", "style"),
    ("redundant_slicing", "complexity"),
    ("redundant_static_lifetimes", "style"),
    ("semicolon_if_nothing_returned", "pedantic"),
    ("single_char_add_str", "style"),
    ("single_char_pattern", "pedantic"),
    ("single_match", "style"),
    ("single_match_else", "pedantic"),
    ("string_lit_as_bytes", "nursery"),
    ("suboptimal_flops", "nursery"),
    ("tabs_in_doc_comments", "style"),
    ("to_string_in_format_args", "perf"),
    ("uninlined_format_args", "pedantic"),
    ("unit_arg", "complexity"),
    ("unnecessary_cast", "complexity"),
    ("unnecessary_lazy_evaluations", "style"),
    ("unnecessary_map_or", "style"),
    ("unnecessary_sort_by", "complexity"),
    ("unnecessary_unwrap", "complexity"),
    ("unreadable_literal", "pedantic"),
    ("unused_unit", "style"),
    ("use_self", "nursery"),
    ("useless_format", "complexity"),
    ("vec_init_then_push", "perf"),
    ("while_let_on_iterator", "style"),
    ("wildcard_in_or_patterns", "complexity"),
    ("write_with_newline", "style"),
    ("wrong_self_convention", "style"),
];

/// The pinned clippy group of `lint`, or `None` when the lint is not in the
/// fix set's table.
#[must_use]
pub fn default_group(lint: &str) -> Option<&'static str> {
    LINT_GROUPS
        .binary_search_by(|(name, _)| (*name).cmp(lint))
        .ok()
        .map(|i| LINT_GROUPS[i].1)
}

/// Whether `lint` is allow-by-default at the pinned toolchain. A lint with
/// no table row is treated as default-on (the coverage test makes that
/// state unreachable for the fix set).
#[must_use]
pub fn is_allow_by_default(lint: &str) -> bool {
    default_group(lint).is_some_and(|g| ALLOW_BY_DEFAULT_GROUPS.contains(&g))
}

/// The fix-set lints whose edits a file may NOT take (Issue 135 T2):
/// allow-by-default at the pinned toolchain, not enabled by the target
/// crate (`levels`), and not opted in by `--groups` (`groups`; `all` opts
/// into every allow-by-default group).
#[must_use]
pub fn gated_lints(levels: &LintLevels, groups: &[String]) -> Vec<&'static str> {
    LINT_GROUPS
        .iter()
        .filter(|(lint, group)| {
            ALLOW_BY_DEFAULT_GROUPS.contains(group)
                && !groups.iter().any(|g| g == "all" || g == group)
                && !levels.enables(lint)
        })
        .map(|(lint, _)| *lint)
        .collect()
}

/// A lint level as written in a manifest or attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Allow,
    Warn,
    Deny,
    Forbid,
    /// `#[expect]` — the lint RUNS (an unfulfilled expectation warns).
    Expect,
}

impl Level {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(Self::Allow),
            "warn" => Some(Self::Warn),
            "deny" => Some(Self::Deny),
            "forbid" => Some(Self::Forbid),
            "expect" => Some(Self::Expect),
            _ => None,
        }
    }
}

/// One level directive: a clippy lint OR group name (no `clippy::` prefix,
/// hyphens normalised to underscores) and its level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    pub name: String,
    pub level: Level,
}

fn directive(name: &str, level: Level) -> Directive {
    Directive { name: name.trim().replace('-', "_"), level }
}

/// The directives in effect for one file, in precedence order (later wins).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LintLevels {
    pub directives: Vec<Directive>,
}

impl LintLevels {
    /// Whether `lint` runs in this crate: its pinned default, overridden by
    /// every directive naming the lint or its group, in order. `all` names
    /// only the default-on groups, so it never raises an allow-by-default
    /// lint. `forbid` is sticky.
    #[must_use]
    pub fn enables(&self, lint: &str) -> bool {
        let group = default_group(lint);
        let allow_default = group.is_some_and(|g| ALLOW_BY_DEFAULT_GROUPS.contains(&g));
        let mut on = !allow_default;
        let mut forbidden = false;
        for d in &self.directives {
            let hits = d.name == lint
                || group.is_some_and(|g| d.name == g)
                || (d.name == "all" && !allow_default);
            if !hits || forbidden {
                continue;
            }
            match d.level {
                Level::Allow => on = false,
                Level::Forbid => {
                    on = true;
                    forbidden = true;
                }
                Level::Warn | Level::Deny | Level::Expect => on = true,
            }
        }
        on
    }
}

/// The `[lints.clippy]` directives of a manifest (or `[workspace.lints.clippy]`
/// when `workspace` is set), sorted by cargo `priority` ascending with
/// groups before lints at equal priority, plus whether the package opts into
/// workspace inheritance (`[lints] workspace = true`).
///
/// A hand scan, not a TOML parse (the default build carries no `toml` dep;
/// the `manifest_has_dep` precedent). Handled shapes: the section form
/// `name = "warn"` / `name = { level = "warn", priority = -1 }`, and dotted
/// keys `clippy.name = …` inside `[lints]` / `[workspace.lints]`. Anything
/// else reads as absent — the skip direction.
#[must_use]
pub fn parse_manifest_lints(text: &str, workspace: bool) -> (Vec<Directive>, bool) {
    let (clippy_hdr, lints_hdr) = if workspace {
        ("[workspace.lints.clippy]", "[workspace.lints]")
    } else {
        ("[lints.clippy]", "[lints]")
    };
    let mut rows: Vec<(i64, bool, usize, Directive)> = Vec::new();
    let mut inherits = false;
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = line.replace(' ', "");
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        let value = value.trim();
        let name = if section == clippy_hdr {
            key
        } else if section == lints_hdr {
            if !workspace && key == "workspace" && value == "true" {
                inherits = true;
                continue;
            }
            match key.strip_prefix("clippy.") {
                Some(n) => n.trim_matches('"'),
                None => continue,
            }
        } else {
            continue;
        };
        let Some((level, priority)) = parse_level_value(value) else {
            continue;
        };
        let d = directive(name, level);
        let is_lint = default_group(&d.name).is_some();
        rows.push((priority, is_lint, rows.len(), d));
    }
    rows.sort_by_key(|(p, is_lint, i, _)| (*p, *is_lint, *i));
    (rows.into_iter().map(|(_, _, _, d)| d).collect(), inherits)
}

/// `"warn"` or `{ level = "warn", priority = -1 }` → (level, priority).
fn parse_level_value(value: &str) -> Option<(Level, i64)> {
    if let Some(s) = value.strip_prefix('"') {
        return Level::parse(s.trim_end_matches('"')).map(|l| (l, 0));
    }
    let body = value.strip_prefix('{')?.strip_suffix('}')?;
    let mut level = None;
    let mut priority = 0i64;
    for part in body.split(',') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        match k.trim() {
            "level" => level = Level::parse(v.trim().trim_matches('"')),
            "priority" => priority = v.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    level.map(|l| (l, priority))
}

/// The inner `#![level(clippy::…)]` directives of a parsed file, in source
/// order. Non-clippy lints, `reason = …` entries and `cfg_attr` wrappers
/// are ignored.
#[must_use]
pub fn parse_inner_attrs(file: &syn::File) -> Vec<Directive> {
    let mut out = Vec::new();
    for attr in &file.attrs {
        if !matches!(attr.style, syn::AttrStyle::Inner(_)) {
            continue;
        }
        let Some(ident) = attr.path().get_ident() else {
            continue;
        };
        let Some(level) = Level::parse(&ident.to_string()) else {
            continue;
        };
        let Ok(metas) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };
        for meta in metas {
            let syn::Meta::Path(path) = meta else {
                continue;
            };
            let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            if let [tool, name] = segs.as_slice() {
                if tool == "clippy" {
                    out.push(directive(name, level));
                }
            }
        }
    }
    out
}

/// The crate root a file compiles under, by cargo's target auto-discovery
/// layout. `None` when the layout does not say (custom `[lib] path`, a file
/// outside the conventional directories).
#[must_use]
pub fn crate_root_for(pkg_root: &Path, file: &Path) -> Option<PathBuf> {
    let rel = file.strip_prefix(pkg_root).ok()?;
    let parts: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
    let target_dir_root = |dir: &str, rest: &[&str]| -> Option<PathBuf> {
        match rest {
            [single] => Some(pkg_root.join(dir).join(single)),
            [sub, ..] => Some(pkg_root.join(dir).join(sub).join("main.rs")),
            [] => None,
        }
    };
    match parts.as_slice() {
        ["build.rs"] => Some(pkg_root.join("build.rs")),
        ["src", "bin", rest @ ..] => target_dir_root("src/bin", rest),
        ["src", ..] => {
            let lib = pkg_root.join("src/lib.rs");
            let main = pkg_root.join("src/main.rs");
            if lib.is_file() {
                Some(lib)
            } else if main.is_file() {
                Some(main)
            } else {
                None
            }
        }
        [dir @ ("tests" | "examples" | "benches"), rest @ ..] => target_dir_root(dir, rest),
        _ => None,
    }
}

/// Nearest ancestor directory holding a `Cargo.toml` with a `[package]`
/// table (a virtual workspace manifest is not a package).
fn package_root(file: &Path) -> Option<PathBuf> {
    let file = std::path::absolute(file).ok()?;
    let mut dir = file.parent()?.to_path_buf();
    loop {
        if let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) {
            if text.lines().any(|l| l.trim() == "[package]") {
                return Some(dir);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Nearest ancestor (from `pkg_root`'s parent) whose manifest declares
/// `[workspace]`, or the package itself when it does.
fn workspace_manifest(pkg_root: &Path) -> Option<String> {
    let mut dir = Some(pkg_root.to_path_buf());
    while let Some(d) = dir {
        if let Ok(text) = std::fs::read_to_string(d.join("Cargo.toml")) {
            if text.lines().any(|l| l.trim() == "[workspace]") {
                return Some(text);
            }
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    None
}

/// Per-run resolver: caches the manifest + crate-root directives per crate
/// root, so a sweep over hundreds of files reads each manifest and root
/// once.
#[derive(Debug, Default)]
pub struct LevelResolver {
    cache: HashMap<(PathBuf, Option<PathBuf>), Vec<Directive>>,
}

impl LevelResolver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The effective directives for `file` (already parsed as `parsed`):
    /// manifest, then crate root, then the file's own inner attributes.
    pub fn levels_for(&mut self, file: &Path, parsed: &syn::File) -> LintLevels {
        let abs = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
        let (mut directives, is_root) = match package_root(&abs) {
            None => (Vec::new(), false),
            Some(pkg) => {
                let root = crate_root_for(&pkg, &abs);
                let is_root = root.as_deref() == Some(abs.as_path());
                let cached = self
                    .cache
                    .entry((pkg.clone(), root.clone()))
                    .or_insert_with(|| crate_directives(&pkg, root.as_deref()))
                    .clone();
                (cached, is_root)
            }
        };
        // The crate root's attributes are already in the cached set (read
        // from disk); a module file adds its own scope on top.
        if !is_root {
            directives.extend(parse_inner_attrs(parsed));
        }
        LintLevels { directives }
    }
}

/// Manifest + crate-root directives (the root read from disk once per run).
fn crate_directives(pkg: &Path, root: Option<&Path>) -> Vec<Directive> {
    let mut out = Vec::new();
    if let Ok(text) = std::fs::read_to_string(pkg.join("Cargo.toml")) {
        let (own, inherits) = parse_manifest_lints(&text, false);
        if inherits {
            if let Some(ws) = workspace_manifest(pkg) {
                out.extend(parse_manifest_lints(&ws, true).0);
            }
        }
        out.extend(own);
    }
    if let Some(root) = root {
        if let Some(parsed) = std::fs::read_to_string(root)
            .ok()
            .and_then(|s| syn::parse_file(&s).ok())
        {
            out.extend(parse_inner_attrs(&parsed));
        }
    }
    out
}

#[cfg(test)]
mod tests;
