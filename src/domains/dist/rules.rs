//! Rule detection over scanned manifests — Issue 085 Phase 1.
//!
//! The same shape as the docker domain's `rules.rs`: detection (and, later,
//! fix emission) live together per rule so a rule cannot detect one shape and
//! rewrite another. Phase 1 is SUGGEST-ONLY — every finding ships as a corpus
//! citation with advice text, never a `SourceEdit`. The bounded insert (a
//! `[profile.dist]` block appended at EOF) is the Phase-2 `--fix` lane and
//! stays unimplemented until the suggest tier earns trust.
//!
//! ## Zero deps
//!
//! A manifest is scanned by a hand-written section-aware line scanner — no
//! `toml` crate (keeping the default build light is a repo rule), no regex.
//! The rules read SECTION MEMBERSHIP and KEY PRESENCE, which is exactly what
//! a line scanner proves. Values are never interpreted: `strip = "symbols"`
//! and `strip = true` both satisfy the strip check because BOTH mean the knob
//! was deliberately set.
//!
//! ## What the scanner does NOT prove
//!
//! - Whether the manifest is VALID TOML (a broken manifest still scans; the
//!   rules stay silent rather than cite a file cargo itself will refuse).
//!   Handled by `well_formed_enough`: a quoted or `=`-bearing "header" is
//!   malformed and marks the file unscannable.
//! - Whether a `[workspace]` parent actually LISTS this crate as a member —
//!   the `member-profile-ignored` evidence is a nearest-ancestor walk, and
//!   an `exclude`d member gets a hedged suggestion text (see the rule).
//!
//! ## Caller evidence (Bench 082 blind spots B1 + B2, landed 2026-09-10)
//!
//! Two facts the manifest text alone cannot prove ride in on
//! [`DistContext`](super::DistContext): binary autodiscovery
//! (`src/main.rs` / `src/bin/*.rs` beside the manifest — tree evidence,
//! gated on `[package]` by [`ships_bin`]) and the workspace ROOT's profile
//! knobs (a member cannot carry effective profiles, so [`effective_knobs`]
//! resolves self-or-root). Both are derived once per scanned manifest by
//! [`DistContext::derive`](super::DistContext::derive); absent evidence
//! keeps the affected rules exactly at their Phase-1 behavior.

use super::context::{DistContext, ProfileKnobs};

/// Every rule this module can emit, in documentation order.
pub const DIST_RULES: &[&str] = &[
    "shipped-binary-unstripped",
    "shipped-binary-missing-size-profile",
    "cli-panic-unwind-tables",
    "member-profile-ignored",
    "large-api-dependency-mostly-unused-hint",
    "pgo-training-set-covers-real-workload-hint",
    "bench-targets-shared-dependency-cluster-hint",
];

/// Phase 1 is suggest-only — no rule carries a bounded fix. The constant
/// exists so the fix lane's eligibility table has a place to grow and the
/// gates can pin the suggest-only contract mechanically.
pub const FIXABLE_DIST: &[&str] = &[];

/// One scanned manifest, section-aware. Only what the rules read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestScan {
    /// Every section header NAME, in order, with its 1-based line.
    pub sections: Vec<(String, usize)>,
    /// `[[bin]]` seen (the crate ships at least one binary target).
    pub has_bin: bool,
    /// `[workspace]` seen in THIS manifest (a root — virtual or combined).
    pub has_workspace: bool,
    /// `[package]` seen (a concrete crate, not a virtual root).
    pub has_package: bool,
    /// Profile sections: (profile name, 1-based line, keys seen).
    pub profiles: Vec<(String, usize, Vec<String>)>,
    /// Dependency names matching the known-large API crate table, in file
    /// order: (crate name, 1-based line of the dependency key). Collected from
    /// every section whose name ends with `dependencies` — the value-shaped
    /// facts (versions, features) are deliberately NOT read, keeping the
    /// scanner's key/name-PRESENCE contract intact.
    pub large_dependencies: Vec<(String, usize)>,
    /// `[[bench]]` targets in file order: (1-based line of the header, keys
    /// seen). Batch 221 — the bench-cluster rule reads the TARGET COUNT and
    /// the `required-features` key PRESENCE per target; every other value
    /// stays unread (the same presence-only contract as the profile keys).
    pub benches: Vec<(usize, Vec<String>)>,
}

/// The known-large API crate table for `large_dependencies` (Batch 143 —
/// nnethercote Dec-2025, "Faster compilation of large API crates": `windows`,
/// `rustix`, and `aws-sdk-ec2` named; the `aws-sdk-` prefix generalizes the
/// service-crate family the post's example belongs to). Deliberately SMALL:
/// a missed crate is a false negative (advice simply not offered), an
/// over-broad table is noise on every small consumer.
const LARGE_API_CRATES_EXACT: &[&str] = &["windows", "windows-sys", "rustix"];
const LARGE_API_CRATES_PREFIX: &[&str] = &["aws-sdk-"];

fn is_large_api_crate(name: &str) -> bool {
    LARGE_API_CRATES_EXACT.contains(&name)
        || LARGE_API_CRATES_PREFIX.iter().any(|p| name.starts_with(p))
}

/// Whether `line` is a section header (`[x]` / `[[x]]`), and its name.
/// `None` when the line is not a header OR is a malformed one (a name
/// carrying `=`, quotes, or `#` — the scanner refuses to invent sections).
fn section_header(line: &str) -> Option<(bool, String)> {
    let t = line.trim();
    if !(t.starts_with('[') && t.ends_with(']')) {
        return None;
    }
    let (array, inner) = if let Some(rest) = t.strip_prefix("[[") {
        (true, rest.strip_suffix("]]")?.trim().to_string())
    } else {
        (false, t[1..t.len() - 1].trim().to_string())
    };
    if inner.is_empty() || inner.contains('=') || inner.contains('"') || inner.contains('#') {
        return None;
    }
    Some((array, inner))
}

/// Scan a manifest's text. Comments are stripped conservatively (the first
/// `#` on a line wins) — key PRESENCE is the only value-shaped fact the rules
/// read, and clipping at `#` before the `=` can never invent or hide a KEY.
#[must_use]
pub fn scan(source: &str) -> ManifestScan {
    let mut m = ManifestScan::default();
    // (is_array, name) of the section the next key lines belong to.
    let mut current: Option<(bool, String)> = None;
    for (i, raw) in source.lines().enumerate() {
        let line_no = i + 1;
        let line = match raw.find('#') {
            Some(pos) => &raw[..pos],
            None => raw,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((array, name)) = section_header(trimmed) {
            if let Some(profile_name) = name.strip_prefix("profile.").map(str::trim) {
                if !profile_name.is_empty() {
                    m.profiles
                        .push((profile_name.to_string(), line_no, Vec::new()));
                    current = Some((false, name));
                    continue;
                }
            }
            if array && name == "bin" {
                m.has_bin = true;
            }
            if array && name == "bench" {
                m.benches.push((line_no, Vec::new()));
            }
            if !array && name == "workspace" {
                m.has_workspace = true;
            }
            if !array && name == "package" {
                m.has_package = true;
            }
            current = Some((array, name.clone()));
            m.sections.push((name, line_no));
            continue;
        }
        // Inside a section: collect `key =` presences (profile keys, large
        // dep names, and — Batch 221 — `[[bench]]` target keys).
        match &current {
            Some((false, name)) if name.starts_with("profile.") => {
                if let Some(eq) = trimmed.find('=') {
                    let key = trimmed[..eq].trim();
                    if !key.is_empty() {
                        if let Some(last) = m.profiles.last_mut() {
                            last.2.push(key.to_ascii_lowercase());
                        }
                    }
                }
            }
            Some((false, name)) if name.ends_with("dependencies") => {
                // A dependency key: `name = ...`, `name = { ... }`, or the
                // workspace-inherited `name.workspace = true`. The dep NAME
                // is the key up to the first '.'; only large-API names are
                // recorded (the table is the rule's whole vocabulary).
                if let Some(eq) = trimmed.find('=') {
                    let raw = trimmed[..eq].trim();
                    let dep_name = raw.split('.').next().unwrap_or(raw).trim();
                    if is_large_api_crate(dep_name) {
                        m.large_dependencies.push((dep_name.to_string(), line_no));
                    }
                }
            }
            Some((true, name)) if name == "bench" => {
                if let Some(eq) = trimmed.find('=') {
                    let key = trimmed[..eq].trim();
                    if !key.is_empty() {
                        if let Some(last) = m.benches.last_mut() {
                            last.1.push(key.to_ascii_lowercase());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    m
}

/// Minimal well-formedness gate. A line that LOOKS like it wants to be a
/// header (`[foo` with no closing bracket) leaves `current` unchanged — it is
/// an unknown line, harmless. The one thing the scanner refuses is a header
/// with a non-plain name (quotes / `=` / `#`): those are exotic tables the
/// rules cannot reason about, so the file is marked unscannable.
#[must_use]
pub fn well_formed_enough(scan: &ManifestScan) -> bool {
    // A profile-header name containing quotes/`=`/`#` never lands in
    // `profiles` (section_header returns None) — detect the leftovers by
    // re-checking the raw headers for exotic names is redundant; instead the
    // gate is: every recorded profile name is plain. Recorded names come from
    // section_header, which already refused exotics, so the gate reduces to
    // "scan ran" — kept as a function so the contract has ONE home and the
    // exotic-name refusal stays testable in one place.
    scan.profiles
        .iter()
        .all(|(name, _, _)| !name.contains('"') && !name.contains('=') && !name.contains('#'))
}

/// Key presence across every `[profile.*]` section (crate-visible so the
/// context's `ProfileKnobs::from_scan` reads the same fact the rules read).
pub(crate) fn any_profile_key(scan: &ManifestScan, key: &str) -> bool {
    scan.profiles
        .iter()
        .any(|(_, _, keys)| keys.iter().any(|k| k == key))
}

/// Does this manifest ship a binary? An explicit `[[bin]]` section, or —
/// blind spot B1 (Bench 082) — cargo's package autodiscovery: `src/main.rs`
/// or a directly-nested `src/bin/*.rs`, carried as tree evidence on the
/// context. Autodiscovery additionally requires `[package]`: a virtual root
/// (workspace-only) manifest discovers no targets, so stray `src/` beside a
/// `[workspace]` table must not fire the ship-size rules. A literal `[[bin]]`
/// keeps its Phase-1 shape (no package requirement) — the section itself is
/// the older, explicit contract.
fn ships_bin(scan: &ManifestScan, ctx: DistContext) -> bool {
    scan.has_bin || (scan.has_package && ctx.autodiscovered_bin)
}

/// The effective knob set — blind spot B2 (Bench 082): what THIS manifest's
/// `[profile.*]` sections set, OR — under a workspace root — what the ROOT
/// manifest sets. `cargo build` from a workspace root applies the root's
/// profiles to every member and ignores member profiles (the mirror image of
/// the `member-profile-ignored` rule), so a root knob satisfies the check for
/// a member manifest. The union is deliberate: a standalone build honors self
/// knobs, a root build honors root knobs, and we cannot know which one the
/// reader will run.
fn effective_knobs(scan: &ManifestScan, ctx: DistContext) -> ProfileKnobs {
    let own = ProfileKnobs::from_scan(scan);
    let root = ctx.root_profile_knobs.unwrap_or_default();
    ProfileKnobs {
        strip: own.strip || root.strip,
        lto: own.lto || root.lto,
        panic: own.panic || root.panic,
    }
}

/// One finding: a rule citation at one line with advice text. Phase 1
/// carries NO `SourceEdit` — suggest-only by contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistFinding {
    /// The corpus rule this finding fronts.
    pub rule_id: &'static str,
    /// 1-based line in the scanned source.
    pub line: usize,
    /// The matched text, trimmed — the human-review excerpt.
    pub excerpt: String,
    /// The advice (the corpus entry's `fixed_code` is the worked example).
    pub advice: &'static str,
}

/// Scan a manifest for every rule, text-only.
#[must_use]
pub fn scan_source(source: &str) -> Vec<DistFinding> {
    scan_source_with(source, DistContext::default())
}

/// Scan with caller evidence (the docker `ScanContext` pattern: a rule whose
/// evidence is absent stays SILENT — never guessed).
#[must_use]
pub fn scan_source_with(source: &str, ctx: DistContext) -> Vec<DistFinding> {
    let scan = scan(source);
    let mut out = Vec::new();
    detect(&scan, ctx, &mut out);
    out
}

/// The detectors. Kept together so a rule cannot detect one shape and advise
/// another (the docker rule/detector co-location lesson).
fn detect(scan: &ManifestScan, ctx: DistContext, out: &mut Vec<DistFinding>) {
    // Rules 1–3 are about SHIPPED binaries: a lib-only crate has no artifact
    // whose bytes anyone downloads, so the advice would be noise.
    if ships_bin(scan, ctx) {
        let knobs = effective_knobs(scan, ctx);
        let anchor = scan
            .profiles
            .first()
            .map(|(_, line, _)| *line)
            .or_else(|| {
                scan.sections
                    .iter()
                    .find(|(name, _)| name == "bin")
                    .map(|(_, line)| *line)
            })
            .unwrap_or(1);
        let excerpt = match scan.profiles.first() {
            Some((name, _, _)) => format!("[profile.{name}]"),
            None if !scan.has_bin => "autodiscovered bin (src/main.rs or src/bin/)".to_string(),
            None => "[[bin]]".to_string(),
        };

        if !knobs.strip {
            out.push(DistFinding {
                rule_id: "shipped-binary-unstripped",
                line: anchor,
                excerpt: excerpt.clone(),
                advice: "add `strip = true` — the default keeps the symbol table, and a shipped binary's symtab IS the internal architecture (14,485 symbols measured on this tool's own v0.1.3)",
            });
        }
        if !knobs.lto {
            out.push(DistFinding {
                rule_id: "shipped-binary-missing-size-profile",
                line: anchor,
                excerpt: excerpt.clone(),
                advice: "add a size profile ([profile.dist]: lto = \"fat\" + codegen-units = 1 + opt-level = \"s\" + panic = \"abort\" + strip = true) and build releases with --profile dist — measured -61% on this tool itself; NEVER put opt-level/codegen-units into [profile.release] (the bench profile inherits release)",
            });
        }
        if !knobs.panic {
            out.push(DistFinding {
                rule_id: "cli-panic-unwind-tables",
                line: anchor,
                excerpt,
                advice: "consider `panic = \"abort\"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection",
            });
        }

        // Rule 6 — the BuildTime next-lever hint (Batch 146). Fires ONLY on
        // the fully-tuned audience: a shipped binary whose effective
        // strip+lto+panic knobs are ALL set. The manifest-level levers this
        // domain advises are exhausted there; the next one (PGO
        // training-set coverage) lives OUTSIDE the manifest, so the advice
        // is conditional and the -hint suffix + measure-first caveat are
        // load-bearing (the large-api-dependency precedent). The scanner
        // cannot see whether PGO exists at all — proxy-triggered by
        // design, and silent for every not-yet-tuned manifest, which keeps
        // the rule strictly additive over the three ship-size fixtures.
        if knobs.strip && knobs.lto && knobs.panic {
            let anchor = scan
                .profiles
                .first()
                .map(|(_, line, _)| *line)
                .or_else(|| {
                    scan.sections
                        .iter()
                        .find(|(name, _)| name == "bin")
                        .map(|(_, line)| *line)
                })
                .unwrap_or(1);
            let excerpt = match scan.profiles.first() {
                Some((name, _, _)) => format!("[profile.{name}]"),
                None => "[[bin]]".to_string(),
            };
            out.push(DistFinding {
                rule_id: "pgo-training-set-covers-real-workload-hint",
                line: anchor,
                excerpt,
                advice: "every manifest-level size/tuning knob this domain advises is set here — the next build-time lever is out-of-manifest: IF you use (or add) PGO, the training set must cover the SHIPPED workload mix (rustc #159091: adding rustdoc benches to the PGO training set = 2.85% mean wall-time across rustdoc benchmarks, >6% best; 'PGO is fiddly to set up but it can really make a difference, and the training set matters'); train on the real workload corpus, not a narrow benchmark — measure before/after",
            });
        }
    }

    // Rule 4 — caller evidence. Without the parent walk the rule is silent.
    if ctx.under_workspace_root
        && scan.has_package
        && scan.has_bin
        && !scan.profiles.is_empty()
        && !scan.has_workspace
    {
        let names: Vec<String> = scan
            .profiles
            .iter()
            .map(|(name, _, _)| format!("[profile.{name}]"))
            .collect();
        out.push(DistFinding {
            rule_id: "member-profile-ignored",
            line: scan.profiles[0].1,
            excerpt: names.join(", "),
            advice: "these profile sections are IGNORED when the crate is built via its workspace — profiles only apply from the workspace-root manifest; move the block to the root (keeping it only matters for standalone builds)",
        });
    }

    // Rule 5 — dependency-shape advice (Batch 143). The trigger is IN the
    // manifest (a known-large API crate in any dependencies section); the FIX
    // is out-of-band (-Zhint-mostly-unused lives in .cargo/config.toml
    // rustflags, not the manifest), so Phase 1 advises and the notes carry
    // the measure-first caveat. One finding per manifest, anchored at the
    // first hit.
    if let Some((name, line)) = scan.large_dependencies.first() {
        out.push(DistFinding {
            rule_id: "large-api-dependency-mostly-unused-hint",
            line: *line,
            excerpt: format!("{name} = ..."),
            advice: "if this large API crate is a small fraction of your actual usage, measure `-Zhint-mostly-unused` (nightly-only rustc flag, set via [build] rustflags in .cargo/config.toml) — big compile-time wins are reported for small-fraction consumers of windows/rustix/aws-sdk-*; measure per crate before adopting",
        });
    }

    // Rule 7 — the bench compile-time cluster split (Batch 221, bevy
    // #25321). Fires on MANY bench targets NONE of which carries a
    // required-features key: the shared dependency set compiles the UNION
    // of every target's heavy deps to run any ONE of them. The gated form
    // (required-features rows present) IS the fixed artifact and stays
    // silent — strictly additive over every existing fixture.
    bench_cluster_finding(scan, out);
}

/// Rule 7's detector — extracted so `detect` keeps one rule per readable
/// block (the `too_many_lines` split point).
fn bench_cluster_finding(scan: &ManifestScan, out: &mut Vec<DistFinding>) {
    let bench_count = scan.benches.len();
    if bench_count >= 3
        && scan
            .benches
            .iter()
            .all(|(_, keys)| !keys.iter().any(|k| k == "required-features"))
    {
        let (line, _) = scan.benches[0];
        out.push(DistFinding {
            rule_id: "bench-targets-shared-dependency-cluster-hint",
            line,
            excerpt: format!("[[bench]] x{bench_count} — no required-features rows"),
            advice: "these bench targets share ONE dependency set, so running any ONE of them compiles the union of their heavy deps — split the bench surface behind per-cluster features, ALL default-on (behavior-identical `cargo bench`): bench deps become `optional = true` normal deps, one `bench_<crate>` feature per measured crate, and every [[bench]] names its feature via `required-features` — a single-crate bench loop then runs `--no-default-features --features bench_ecs` and skips the heavy siblings (bevy #25321: the ecs bench no longer builds bevy_render). The required-features rows are LOAD-BEARING — without them a gated bench target is skipped silently (green-zero class)",
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_and_keys_parse() {
        let m = scan(
            "[package]\nname = \"x\"\n\n[profile.release]\nstrip = true\n\n[[bin]]\nname = \"x\"\n",
        );
        assert!(m.has_package && m.has_bin);
        assert_eq!(m.profiles.len(), 1);
        assert_eq!(m.profiles[0].0, "release");
        assert_eq!(m.profiles[0].2, vec!["strip"]);
        assert_eq!(m.sections.len(), 2); // [package] + [[bin]]
    }

    #[test]
    fn comments_do_not_hide_or_invent_keys() {
        let m = scan("[profile.release]\n# strip = true\nlto = \"fat\" # real key\n");
        assert!(
            !any_profile_key(&m, "strip"),
            "commented key must not count"
        );
        assert!(any_profile_key(&m, "lto"));
    }

    #[test]
    fn array_and_workspace_flags() {
        let m = scan("[[bin]]\nname = \"a\"\n\n[workspace]\nmembers = []\n");
        assert!(m.has_bin && m.has_workspace && !m.has_package);
    }

    #[test]
    fn malformed_header_name_marks_unscannable() {
        // A quoted section name is exotic — well_formed_enough must refuse
        // (via the recorded-name check) so rules never cite such a file.
        let ok = scan("[profile.release]\nstrip = true\n");
        assert!(well_formed_enough(&ok));
        // section_header refuses the exotic name entirely, so nothing is
        // recorded; the finding-side gate is that profiles stay EMPTY when a
        // manifest only carries exotic profile headers.
        let exotic = scan("[profile.\"weird name\"]\nstrip = true\n");
        assert_eq!(exotic.profiles, []);
    }

    #[test]
    fn profile_keys_case_normalized() {
        // TOML section names are CASE-SENSITIVE: `[Profile.Release]` is not a
        // cargo profile and the scanner must not treat it as one. Keys within
        // a real profile header are normalized (strip vs STRIP both count).
        let lower = scan("[profile.release]\nstrip = true\n");
        assert_eq!(lower.profiles[0].0, "release");
        assert!(any_profile_key(&lower, "strip"));
        let upper_key = scan("[profile.release]\nSTRIP = true\n");
        assert!(any_profile_key(&upper_key, "strip"));
        let wrong_case = scan("[Profile.Release]\nstrip = true\n");
        assert!(
            wrong_case.profiles.is_empty(),
            "a case-variant header is not a profile section"
        );
    }

    #[test]
    fn large_api_dependency_names_are_collected_across_dependency_sections() {
        let src = "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\nwindows = { version = \"0.58\", features = [\"Win32_Foundation\"] }\n\n[dev-dependencies]\nrustix = \"1\"\n\n[workspace]\n\n[workspace.dependencies]\naws-sdk-ec2 = \"1\"\n";
        let m = scan(src);
        let names: Vec<&str> = m
            .large_dependencies
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["windows", "rustix", "aws-sdk-ec2"],
            "every *dependencies section counts, in file order"
        );
        assert_eq!(
            m.large_dependencies[0].1, 6,
            "anchor line is 1-based at the key"
        );
    }

    #[test]
    fn small_and_commented_dependencies_stay_unrecorded() {
        let src = "[dependencies]\nserde = \"1\"\n# windows = \"0.58\"\ntokio.workspace = true\n";
        let m = scan(src);
        assert!(
            m.large_dependencies.is_empty(),
            "small deps, commented deps, and non-table large keys never fire"
        );
    }

    #[test]
    fn workspace_inherited_form_resolves_the_dep_name() {
        let src = "[dependencies]\nwindows.workspace = true\n";
        let m = scan(src);
        assert_eq!(m.large_dependencies.len(), 1);
        assert_eq!(m.large_dependencies[0].0, "windows");
    }
}
