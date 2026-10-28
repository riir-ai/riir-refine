//! The shared manifest-gate parser substrate — UNGATED by design.
//!
//! Two consumers must never disagree about which targets a cargo invocation
//! compiles: the auto-oracle's gap-arming census ([`crate::clippy_oracle`],
//! always compiled — the module-gating invariant is that the oracle works at
//! every feature posture) and the post-verify required-features audit
//! ([`crate::fix_verify`] P43, behind `fix_verify`). Both read the SAME
//! parser here — one home, the two surfaces agree by construction. The
//! extraction precedent is the repo's own standing rule: before fixing a
//! rule-in-one-instrument class, land the repair as one shared mechanism.
//!
//! `fix_verify` re-exports these items so every existing call site (the
//! bin's P43 arm, this crate's tests, downstream paths) keeps compiling
//! unchanged; new callers should prefer `crate::manifest_gate::` directly.
//!
//! Line-based manifest reading throughout — no toml dep (the module's other
//! manifest readers set that precedent). Inline `#` comments are stripped
//! quote-aware by [`strip_inline_comment`] BEFORE any value parse — the
//! founding casualty is real: ndb's default table carries a trailing comment
//! ON the `default = [ … ]` row, and the naive read glued the comment into
//! the last feature name (`bm25_facet\"] # …`) plus every continuation line
//! of the comment, so the default set parsed corrupted while cargo resolved
//! it fine — a false P43 mark and a false census gap on a DEFAULT-ON
//! feature. Tests live in `fix_verify`'s suite (via the re-export) plus the
//! census arms in [`crate::clippy_oracle`] and the comment arms here.

/// Strip a `#` comment from a manifest line, respecting quoted strings
/// (basic `"…"` with `\\` escapes and literal `'…'`): a `#` inside quotes
/// is data, anywhere else it starts a comment that runs to end of line.
#[must_use]
fn strip_inline_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, ch) {
            (Some(q), ch) if ch == q => quote = None,
            (Some('"'), '\\') => escaped = true,
            (None, '#') => return &line[..i],
            (None, q @ ('"' | '\'')) => quote = Some(q),
            _ => {}
        }
    }
    line
}

/// The cfg-audit half of the manifest scan: the package's `default` feature
/// list + every declared feature key (the feature-set constructor below
/// consumes both; `manifest_nondefault_feature_count` is the count-shaped
/// consumer of the same parse).
#[must_use]
pub fn manifest_feature_names(manifest_text: &str) -> (Vec<String>, Vec<String>) {
    let mut in_features = false;
    let mut default_deps: Vec<String> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    // `default = [\n "a",\n "b",\n]` spans lines — accumulate the value
    // until the next key or section.
    let mut pending_default: Option<String> = None;
    let finalize_default = |pending: &mut Option<String>, out: &mut Vec<String>| {
        if let Some(value) = pending.take() {
            for part in value.trim_matches(['[', ']']).split(',') {
                let p = part.trim().trim_matches('"');
                if !p.is_empty() {
                    out.push(p.to_string());
                }
            }
        }
    };
    for line in manifest_text.lines() {
        let t = strip_inline_comment(line.trim());
        if t.starts_with('[') {
            in_features = t == "[features]";
            finalize_default(&mut pending_default, &mut default_deps);
            continue;
        }
        if !in_features {
            continue;
        }
        if let Some(eq) = t.find('=') {
            finalize_default(&mut pending_default, &mut default_deps);
            let key = t[..eq].trim().trim_matches('"');
            let value = t[eq + 1..].trim();
            if key == "default" {
                pending_default = Some(value.to_string());
                // A single-line default (the common shape, and the
                // comment-stripped shape) is COMPLETE at its `]` — finalize
                // now, so prose that FOLLOWS the array (a wrapped comment's
                // continuation lines carry no `=` and no leading `#`) is
                // never glued into the parsed set. The ndb casualty: the
                // last default read as `bm25_facet\"] # …` plus four prose
                // lines, while cargo resolved the same table to a clean 17.
                if value.contains(']') {
                    finalize_default(&mut pending_default, &mut default_deps);
                }
            } else {
                keys.push(key.to_string());
            }
        } else if let Some(mut pd) = pending_default.take() {
            pd.push(' ');
            pd.push_str(t);
            if pd.contains(']') {
                let mut holder = Some(pd);
                finalize_default(&mut holder, &mut default_deps);
            } else {
                pending_default = Some(pd);
            }
        }
    }
    finalize_default(&mut pending_default, &mut default_deps);
    (default_deps, keys)
}

/// The enabled-feature set a cargo invocation compiled with.
#[derive(Debug, Clone, Default)]
pub struct FeatureSet {
    /// `--all-features` was passed — every declared feature is on.
    pub all_on: bool,
    /// Explicitly enabled feature names (crate defaults + parsed
    /// `--features` entries that name THIS crate's features).
    pub names: std::collections::HashSet<String>,
}

impl FeatureSet {
    #[must_use]
    pub fn contains(&self, feature: &str) -> bool {
        self.all_on || self.names.contains(feature)
    }
}

/// Assemble the enabled set the gate compiled with: the manifest's default
/// features (unless `--no-default-features`) + every `--features` entry
/// naming one of the manifest's own features (dep-prefixed `a/b` and
/// `a?/b` forms name OTHER crates' features — ignored). `--all-features`
/// switches `all_on`, which short-circuits every feature atom to enabled.
#[must_use]
pub fn feature_set_from_args(manifest_text: &str, verify_args: Option<&str>) -> FeatureSet {
    let (defaults, keys) = manifest_feature_names(manifest_text);
    let mut set = FeatureSet::default();
    let Some(args) = verify_args else {
        set.names.extend(defaults);
        return set;
    };
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if tokens.contains(&"--all-features") {
        set.all_on = true;
        set.names.extend(defaults);
        set.names.extend(keys);
        return set;
    }
    if !tokens.contains(&"--no-default-features") {
        set.names.extend(defaults);
    }
    let mut requested: Vec<String> = Vec::new();
    let mut iter = tokens.iter().peekable();
    while let Some(t) = iter.next() {
        let value = if let Some(rest) = t.strip_prefix("--features=") {
            Some(rest.to_string())
        } else if *t == "--features" {
            iter.next().map(|v| (*v).to_string())
        } else {
            None
        };
        if let Some(v) = value {
            requested.extend(
                v.split([',', ' ', '\t'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }
    }
    for r in requested {
        if keys.contains(&r) {
            set.names.insert(r);
        }
    }
    set
}

// Divergences (documented, conservative direction — a mark never claims a
// compile that did not happen, an unmarked file was compiled by at least one
// satisfied covering row):
// - required-features naming an OPTIONAL DEP (`dep:x` / bare non-feature)
//   never satisfy → the file stays marked even under --all-features; the
//   suggestion then over-shoots (honest — the row genuinely is not compiled
//   by any feature-only posture);
// - a [lib] / [[bin]] row with an explicit `path` pointing OUTSIDE src/ is
//   matched only by that exact path; src/ module-tree files always count as
//   lib-covered (they ride --lib, which has no required-features gate in
//   practice — a lib row WITH unsatisfied required-features marks the whole
//   tree, correctly);
// - glob-free: `path` match is exact, `./` prefixes normalized on both sides.

/// One cargo target row, restricted to what the gate reads.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetRow {
    pub kind: &'static str,
    pub name: Option<String>,
    /// The package-relative path cargo compiles this row from: the explicit
    /// `path` (`/`-separated, `./` stripped), else the kind/name default
    /// (`src/lib.rs`, `src/main.rs` for the main bin, `src/bin/<name>.rs`,
    /// `tests/<name>.rs`, `benches/<name>.rs`, `examples/<name>.rs`) —
    /// resolved once at the end of [`manifest_targets`]. `None` = the row
    /// covers nothing matchable (no path and no name).
    pub path: Option<String>,
    pub required_features: Vec<String>,
}

/// The `[package]` auto-discovery switches (cargo defaults: all true).
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // mirrors cargo's four independent [package] switches
pub struct AutoFlags {
    pub bins: bool,
    pub tests: bool,
    pub benches: bool,
    pub examples: bool,
}

impl Default for AutoFlags {
    fn default() -> Self {
        Self {
            bins: true,
            tests: true,
            benches: true,
            examples: true,
        }
    }
}

/// Parse the manifest's target rows + auto-discovery flags. Line-based, the
/// `manifest_feature_names` style — no toml dep; multiline
/// `required-features = [ … ]` arrays accumulate until the closing bracket,
/// like the `default = [ … ]` reader.
#[must_use]
pub fn manifest_targets(manifest_text: &str) -> (Vec<TargetRow>, AutoFlags) {
    let mut rows: Vec<TargetRow> = Vec::new();
    let mut auto = AutoFlags::default();
    let mut pkg_name: Option<String> = None;
    // `Some(kind)` while inside a `[[kind]]` row; `[lib]` maps to the same
    // shape. Any other section header closes the current row.
    let mut current: Option<(&'static str, TargetRow)> = None;
    let mut pending_rf: Option<String> = None;
    let finalize_rf = |pending: &mut Option<String>, row: &mut TargetRow| {
        if let Some(value) = pending.take() {
            for part in value.trim_matches(['[', ']']).split(',') {
                let p = part.trim().trim_matches('"');
                if !p.is_empty() {
                    row.required_features.push(p.to_string());
                }
            }
        }
    };
    let mut close_row = |current: &mut Option<(&'static str, TargetRow)>,
                         pending: &mut Option<String>| {
        if let Some((_, mut row)) = current.take() {
            finalize_rf(pending, &mut row);
            rows.push(row);
        }
    };
    for line in manifest_text.lines() {
        let t = strip_inline_comment(line.trim());
        if t.starts_with('[') {
            close_row(&mut current, &mut pending_rf);
            if let Some(kind) = target_kind_from_header(t) {
                current = Some((
                    kind,
                    TargetRow {
                        kind,
                        name: None,
                        path: None,
                        required_features: Vec::new(),
                    },
                ));
            }
            continue;
        }
        if let Some((kind, row)) = current.as_mut() {
            if let Some(pending) = pending_rf.as_mut() {
                pending.push(' ');
                pending.push_str(t);
                if pending.contains(']') {
                    finalize_rf(&mut pending_rf, row);
                }
                continue;
            }
            let Some(eq) = t.find('=') else {
                continue;
            };
            let key = t[..eq].trim();
            let value = t[eq + 1..].trim();
            match key {
                "name" => {
                    let n = value.trim().trim_matches('"');
                    if !n.is_empty() {
                        row.name = Some(n.to_string());
                    }
                }
                "path" => {
                    let p = value.trim().trim_matches('"').trim_start_matches("./");
                    if !p.is_empty() {
                        row.path = Some(p.to_string());
                    }
                }
                "required-features" => {
                    if value.contains(']') {
                        let mut holder = Some(value.to_string());
                        finalize_rf(&mut holder, row);
                    } else if value.starts_with('[') {
                        pending_rf = Some(value.to_string());
                    }
                }
                _ => {
                    let _ = kind; // harness/test flags: not gate inputs
                }
            }
        } else {
            apply_package_line(t, &mut auto, &mut pkg_name);
        }
    }
    close_row(&mut current, &mut pending_rf);
    resolve_default_target_paths(&mut rows, pkg_name.as_deref());
    (rows, auto)
}

/// The target-row header a section line names, if any.
fn target_kind_from_header(t: &str) -> Option<&'static str> {
    match t {
        "[lib]" => Some("lib"),
        "[[bin]]" => Some("bin"),
        "[[test]]" => Some("test"),
        "[[bench]]" => Some("bench"),
        "[[example]]" => Some("example"),
        _ => None,
    }
}

/// The `[package]`-section lines the gate reads: the four auto-discovery
/// switches and the crate `name` (the main bin's default-path input).
fn apply_package_line(t: &str, auto: &mut AutoFlags, pkg_name: &mut Option<String>) {
    if let Some((key, value)) = t.split_once('=') {
        let on = value.trim() != "false";
        match key.trim() {
            "autobins" => auto.bins = on,
            "autotests" => auto.tests = on,
            "autobenches" => auto.benches = on,
            "autoexamples" => auto.examples = on,
            "name" => *pkg_name = Some(value.trim().trim_matches('"').to_string()),
            _ => {}
        }
    }
}

/// Fill each row's `path` from the kind/name default when the manifest gave
/// no explicit one (`src/lib.rs`, `src/main.rs` for the main bin,
/// `src/bin/<n>.rs`, `tests/<n>.rs`, `benches/<n>.rs`, `examples/<n>.rs`); a
/// row with neither path nor name covers nothing matchable.
fn resolve_default_target_paths(rows: &mut [TargetRow], pkg_name: Option<&str>) {
    for row in rows.iter_mut() {
        if row.path.is_none() {
            row.path = row.name.as_deref().and_then(|name| match row.kind {
                "lib" => Some("src/lib.rs".to_string()),
                "bin" if Some(name) == pkg_name => Some("src/main.rs".to_string()),
                "bin" => Some(format!("src/bin/{name}.rs")),
                "test" => Some(format!("tests/{name}.rs")),
                "bench" => Some(format!("benches/{name}.rs")),
                "example" => Some(format!("examples/{name}.rs")),
                _ => None,
            });
        }
    }
}

/// The targets covering `rel` (package-relative, `/`-separated): resolved
/// rows matched by path first, then the auto-discovery conventions. An empty
/// result means no target compiles the file at any posture (orphan).
#[must_use]
pub fn covering_targets(rel: &str, rows: &[TargetRow], auto: AutoFlags) -> Vec<TargetRow> {
    let rel = rel.trim_start_matches("./");
    let explicit: Vec<TargetRow> = rows
        .iter()
        .filter(|r| r.path.as_deref() == Some(rel))
        .cloned()
        .collect();
    if !explicit.is_empty() {
        return explicit;
    }
    let implicit = |kind: &'static str| TargetRow {
        kind,
        name: None,
        path: Some(rel.to_string()),
        required_features: Vec::new(),
    };
    if rel == "src/lib.rs" || (rel.starts_with("src/") && !rel.starts_with("src/bin/")) {
        // The lib module tree — [lib] has no required-features gate in
        // practice, and --lib compiles it unconditionally.
        return vec![implicit("lib")];
    }
    if rel == "src/main.rs" || rel.starts_with("src/bin/") {
        return auto.bins.then(|| implicit("bin")).into_iter().collect();
    }
    if rel.starts_with("tests/") {
        return auto.tests.then(|| implicit("test")).into_iter().collect();
    }
    if rel.starts_with("benches/") {
        return auto
            .benches
            .then(|| implicit("bench"))
            .into_iter()
            .collect();
    }
    if rel.starts_with("examples/") {
        return auto
            .examples
            .then(|| implicit("example"))
            .into_iter()
            .collect();
    }
    Vec::new()
}

/// The finding-side twin of the P43 audit (the auto-oracle's census — same
/// parser, the two surfaces cannot disagree): every EXPLICIT target row
/// whose `required-features` are not all enabled by `enabled`. Such a row is
/// skipped SILENTLY by `--all-targets` (it compiles to nothing — the green-
/// zero class), so every clippy finding inside it is invisible to whatever
/// ran at that posture. Returns `(target name, kind, missing features in
/// manifest order)`; rows the posture enables never appear.
#[must_use]
pub fn required_features_skipped_targets(
    manifest_text: &str,
    enabled: &FeatureSet,
) -> Vec<(String, String, Vec<String>)> {
    let (rows, _auto) = manifest_targets(manifest_text);
    let mut out: Vec<(String, String, Vec<String>)> = Vec::new();
    for row in rows {
        if row.required_features.is_empty() {
            continue;
        }
        let missing: Vec<String> = row
            .required_features
            .iter()
            .filter(|feat| !enabled.contains(feat))
            .cloned()
            .collect();
        if missing.is_empty() {
            continue;
        }
        let name = row
            .name
            .clone()
            .or_else(|| row.path.clone())
            .unwrap_or_default();
        out.push((name, row.kind.to_string(), missing));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_keeps_hash_inside_quotes_and_cuts_outside() {
        assert_eq!(strip_inline_comment("name = \"a#b\""), "name = \"a#b\"");
        assert_eq!(strip_inline_comment("a = 'lit # str'"), "a = 'lit # str'");
        assert_eq!(strip_inline_comment("a = 1 # tail"), "a = 1 ");
        assert_eq!(strip_inline_comment("# whole line"), "");
        let escaped = "a = \"esc \\\" # still data\" # tail";
        assert_eq!(
            strip_inline_comment(escaped),
            "a = \"esc \\\" # still data\" "
        );
    }

    #[test]
    fn default_row_with_trailing_comment_parses_clean() {
        // The founding casualty's shape: the trailing comment continues on
        // the next lines; every line of it must stay out of the parsed set.
        let manifest = concat!(
            "[features]\n",
            "default = [\"a\", \"bm25_facet\"] # why DEFAULT-ON (Plan N\n",
            "2026-01-02): the promotion bar # was met; G1 11/11 incl. the pin\n",
            "G3 clean). Repo gates: ALL PASS.\n",
            "bm25_facet = []\n",
            "other = []\n",
        );
        let (defaults, keys) = manifest_feature_names(manifest);
        assert_eq!(defaults, vec!["a".to_string(), "bm25_facet".to_string()]);
        assert!(keys.contains(&"other".to_string()));
        let set = feature_set_from_args(manifest, None);
        assert!(
            set.contains("bm25_facet"),
            "default-on feature must resolve"
        );
    }

    #[test]
    fn target_row_values_with_trailing_comments_parse_clean() {
        let manifest = concat!(
            "[features]\ndefault = []\ngate = [] # note\n",
            "[[bench]]\nname = \"x\" # the goat bench\n",
            "required-features = [\"gate\"] # why\n",
        );
        let (rows, _auto) = manifest_targets(manifest);
        let row = rows.iter().find(|r| r.kind == "bench").unwrap();
        assert_eq!(
            row.name.as_deref(),
            Some("x"),
            "comment must not glue into the name"
        );
        assert_eq!(row.required_features, vec!["gate".to_string()]);
        // gate is NOT default-on in this fixture: the census must report the
        // bench missing EXACTLY [gate] — clean values in, clean missing out.
        let set = feature_set_from_args(manifest, None);
        let skipped = required_features_skipped_targets(manifest, &set);
        assert_eq!(
            skipped,
            vec![(
                "x".to_string(),
                "bench".to_string(),
                vec!["gate".to_string()]
            )]
        );
    }
}
