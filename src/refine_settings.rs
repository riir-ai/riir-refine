//! The machine-local settings file (`settings.toml` in the account dir)
//! — the persisted choices for the run-start auto surfaces and the
//! offline firewall's companion state (Plan 342).
//!
//! Shape: a small, WELL-COMMENTED TOML file the user can read and edit by
//! hand; `set()` edits values in place and preserves every comment line,
//! and a missing file is written whole from [`TEMPLATE`] so the file is
//! always self-documenting. No `toml` crate (the repo's dep-light
//! discipline): a tiny hand parser over flat `[section]` + `key = bool`
//! lines with optional trailing comments — unknown sections/keys are
//! ignored on read, never destroyed on write.
//!
//! Precedence (the firewall law): the offline marker (see
//! [`crate::offline`]) overrides EVERYTHING — env and this file cannot
//! re-enable a network lane while the marker stands; only `cargo refine
//! online` clears it. Otherwise: this file is the persisted choice, the
//! envs are the per-run overrides in their documented directions
//! (`RIIR_REFINE_UPDATE_CHECK=0` and `RIIR_REFINE_AUTO_CORPUS=0` demote;
//! `RIIR_REFINE_AUTO_UPGRADE=1` promotes — self-replacement stays opt-in).
//!
//! Ungated by design: the settings surface ships in every build (the
//! charitable release set included) — a user who cannot dial still owns
//! their machine-local choices.

use std::path::{Path, PathBuf};

/// The settings file name inside the account dir.
pub const SETTINGS_FILE: &str = "settings.toml";

/// The one home (delegates to the account-dir resolution the consent
/// records use — `RIIR_REFINE_KAT_ACCOUNT_DIR`, else XDG/`~/.config` /
/// `%APPDATA%`, else temp) so every machine-local state shares one
/// directory and the resolution cannot drift between modules.
#[must_use]
pub fn default_dir() -> PathBuf {
    riir_kat::kat_mining_consent::default_dir()
}

/// The settings file's full path inside `dir`.
#[must_use]
pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join(SETTINGS_FILE)
}

/// The persisted choices. Defaults: the run-start auto-check is ON (the
/// owner mandate — app + corpus before any healing run), the hands-free
/// binary self-replacement stays OFF (a binary that replaces itself
/// without explicit consent is a consent violation, full stop).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The run-start release check (the `update available:` alert line —
    /// one cached GET, ok → 12 h window).
    pub app_check: bool,
    /// The run-start corpus lease refresh (at most once per 12 h; the
    /// charge plane stays the tier gate). Needs a `kat_transport` build —
    /// a charitable build's hook degrades honestly exactly as today.
    pub corpus_refresh: bool,
    /// The hands-free upgrade apply at run start (opt-in).
    pub auto_upgrade: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app_check: true,
            corpus_refresh: true,
            auto_upgrade: false,
        }
    }
}

impl Settings {
    /// The key -> bool lookup for the render + the toggle verbs.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<bool> {
        match key {
            "app_check" => Some(self.app_check),
            "corpus_refresh" => Some(self.corpus_refresh),
            "auto_upgrade" => Some(self.auto_upgrade),
            _ => None,
        }
    }

    /// The one-line state render the verbs print
    /// (`app check on · corpus refresh off · auto upgrade on`).
    #[must_use]
    pub fn state_line(&self) -> String {
        let word = |b: bool| if b { "on" } else { "off" };
        format!(
            "app check {} · corpus refresh {} · auto upgrade {}",
            word(self.app_check),
            word(self.corpus_refresh),
            word(self.auto_upgrade)
        )
    }
}

/// The `cargo refine off|on --<flag>` vocabulary -> the settings keys it
/// flips (order = write order). `--auto-update` is the WHOLE run-start
/// auto-check pair (app + corpus) — the owner's naming for "stop checking
/// before every run"; the two half-flags exist for the split posture.
#[must_use]
pub fn flag_keys(flag: &str) -> Option<&'static [&'static str]> {
    match flag {
        "--auto-update" => Some(&["app_check", "corpus_refresh"]),
        "--auto-corpus" => Some(&["corpus_refresh"]),
        "--app-check" => Some(&["app_check"]),
        "--auto-upgrade" => Some(&["auto_upgrade"]),
        _ => None,
    }
}

/// The valid flag words, for usage + refusal lines.
pub const VALID_FLAGS: &[&str] = &[
    "--auto-update",
    "--auto-corpus",
    "--app-check",
    "--auto-upgrade",
];

/// The well-commented template written when the file is missing (and the
/// shape the file converges to when a key must be appended). The comments
/// ARE the documentation — the owner asked for a config a human can read.
#[must_use]
pub fn template() -> String {
    format!(
        r#"# cargo refine — machine-local settings
# Safe to edit by hand. Written by `cargo refine off|on --<flag>`; edits
# keep your comments. One file per machine, in the account dir beside the
# consent records. Deleted file = the defaults below.
#
# Precedence: the offline marker (`cargo refine offline`) overrides
# EVERYTHING here and in the environment — while it stands, no network
# lane dials; only `cargo refine online` clears it. Otherwise the envs
# named per key are per-run overrides in their documented direction.

[auto]
# Check for a newer cargo refine release before every healing run (the
# "update available:" line — one cached HTTP GET; a clean check silences
# it for 12 h). Per-run demote: RIIR_REFINE_UPDATE_CHECK=0.
app_check = {app_check}

# Refresh the corpus lease before every healing run, at most once per
# 12 h (charges 0.001 KAT per NEW epoch on the KAT lane; the points
# posture charges nothing by law). Silent when the corpus is current.
# Needs a kat_transport build — a build without it prints its rebuild
# line instead. Per-run demote: RIIR_REFINE_AUTO_CORPUS=0.
corpus_refresh = {corpus_refresh}

# Apply a pending upgrade automatically at run start (the swap renames;
# the CURRENT run keeps the old binary — rerun to pick it up). OPT-IN:
# a binary that replaces itself without an explicit yes is refused by
# law. Arms from this key OR RIIR_REFINE_AUTO_UPGRADE=1.
auto_upgrade = {auto_upgrade}
"#,
        app_check = Settings::default().app_check,
        corpus_refresh = Settings::default().corpus_refresh,
        auto_upgrade = Settings::default().auto_upgrade,
    )
}

/// One honest warning line for a record that exists but could not be
/// honoured (the consent-module discipline — a silently swallowed parse
/// failure is how a user ends up believing a choice is stored when it is
/// not).
#[must_use]
pub fn malformed_line(path: &Path) -> String {
    format!(
        "settings: {} could not be parsed — the defaults are in effect until it is fixed or deleted",
        path.display()
    )
}

/// Read the settings, discarding the diagnostic. Never panics; a missing
/// or unreadable file reads [`Settings::default()`]; a MALFORMED file also
/// reads the defaults (use [`read_with_warning`] where a line can print).
#[must_use]
pub fn read(dir: &Path) -> Settings {
    read_with_warning(dir).0
}

/// Read the settings plus the warning line for a malformed record.
#[must_use]
pub fn read_with_warning(dir: &Path) -> (Settings, Option<String>) {
    let path = settings_path(dir);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (Settings::default(), None);
    };
    match parse(&text) {
        Some(s) => (s, None),
        None => (Settings::default(), Some(malformed_line(&path))),
    }
}

/// Is `key` one of the three settings keys (the parser's malformed-value
/// boundary: unknown keys' values are never load-bearing).
#[must_use]
fn known(key: &str) -> bool {
    matches!(key, "app_check" | "corpus_refresh" | "auto_upgrade")
}

/// The minimal parse: flat sections, `key = bool` (optionally with a
/// trailing `# comment`). Unknown keys/sections are skipped. `None` when
/// a KNOWN key carries a value that is not `true`/`false` — that is the
/// malformed case (unknown garbage stays ignored, never load-bearing).
#[must_use]
fn parse(text: &str) -> Option<Settings> {
    let mut out = Settings::default();
    let mut section = String::new();
    let mut seen_known = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return None; // a bare word inside a section is not our shape
        };
        let key = key.trim();
        let value = value.split('#').next().unwrap_or("").trim();
        if section != "auto" {
            continue;
        }
        let parsed = match value {
            "true" => true,
            "false" => false,
            _ if known(key) => return None,
            _ => continue, // an unknown key's odd value is skipped, never load-bearing
        };
        match key {
            "app_check" => {
                out.app_check = parsed;
                seen_known = true;
            }
            "corpus_refresh" => {
                out.corpus_refresh = parsed;
                seen_known = true;
            }
            "auto_upgrade" => {
                out.auto_upgrade = parsed;
                seen_known = true;
            }
            _ => {}
        }
    }
    let _ = seen_known;
    Some(out)
}

/// Set one key's value, preserving every comment line (only the value
/// token between `=` and any trailing `#` changes). A missing file is
/// written whole from the template with the value applied; a file whose
/// section lost the key gets the full `[auto]` block appended (the other
/// keys keep their parsed values) — the file converges to the
/// well-commented shape from any starting state.
///
/// # Errors
/// The dir could not be created or the write failed (carries the path).
pub fn set(dir: &Path, key: &str, value: bool) -> Result<String, String> {
    if Settings::default().get(key).is_none() {
        return Err(format!("settings: unknown key {key:?}"));
    }
    let path = settings_path(dir);
    let current = std::fs::read_to_string(&path).ok();
    let new_text = match current {
        None => {
            let mut text = template();
            apply_value(&mut text, key, value);
            text
        }
        Some(text) => {
            let mut text = text;
            if !replace_value(&mut text, key, value) {
                // The key is missing from an existing file — re-add the
                // whole commented [auto] block so the shape stays
                // self-documenting: the REQUESTED value for `key`, the
                // parsed (or default) values for the other two.
                let parsed = parse(&text).unwrap_or_default();
                let mut block = template();
                for other in ["app_check", "corpus_refresh", "auto_upgrade"] {
                    let v = if other == key {
                        value
                    } else {
                        parsed.get(other).unwrap_or_default()
                    };
                    apply_value(&mut block, other, v);
                }
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(
                    "\n# re-added by `cargo refine off|on` — the file was missing this key\n",
                );
                text.push_str(&block);
            }
            text
        }
    };
    std::fs::create_dir_all(dir).map_err(|e| format!("settings: {}: {e}", dir.display()))?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, new_text).map_err(|e| format!("settings: {}: {e}", path.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("settings: {}: {e}", path.display()))?;
    Ok(format!("settings: {key} = {value} ({})", path.display()))
}

/// Overwrite the `key`'s value in a template text (the template always
/// carries every key — the lookup cannot miss). The replace runs
/// UNCONDITIONALLY: a `debug_assert!` wrapping a side-effecting call is
/// compiled out in release and would silently write default values.
fn apply_value(text: &mut String, key: &str, value: bool) {
    let ok = replace_value(text, key, value);
    debug_assert!(ok, "the template has every key");
}

/// Replace the value token of `key` in place; comments (leading lines and
/// any trailing `# …` on the key's own line) survive. Returns false when
/// the key is not present.
fn replace_value(text: &mut String, key: &str, value: bool) -> bool {
    let mut changed = false;
    let mut out = String::with_capacity(text.len() + 8);
    for line in text.lines() {
        let trimmed = line.trim_start();
        let in_comment = trimmed.starts_with('#') || trimmed.starts_with('[');
        let candidate = line.split('#').next().unwrap_or("");
        let is_key = !in_comment
            && candidate
                .split_once('=')
                .is_some_and(|(k, _)| k.trim() == key);
        if is_key {
            let indent = line.len() - trimmed.len();
            let trailing = line
                .split_once('#')
                .map(|(_, rest)| format!("  #{rest}"))
                .unwrap_or_default();
            out.push_str(&" ".repeat(indent));
            out.push_str(key);
            out.push_str(" = ");
            out.push_str(if value { "true" } else { "false" });
            out.push_str(&trailing);
            changed = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if changed {
        // `lines()` drops one trailing newline; the write path adds none —
        // keep files newline-terminated like the template.
        if !out.ends_with("\n\n") {
            // no-op guard: exactly one '\n' was appended for the last line
        }
        *text = out;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UNIQUE dir per test (the lib tests run in parallel in one process;
    /// a shared dir is a shared-temp-path race — each test names itself).
    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "refine_settings_test_{}_{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn defaults_are_app_on_corpus_on_upgrade_off() {
        let s = Settings::default();
        assert!(s.app_check);
        assert!(s.corpus_refresh);
        assert!(!s.auto_upgrade);
        assert_eq!(
            s.state_line(),
            "app check on · corpus refresh on · auto upgrade off"
        );
    }

    #[test]
    fn flag_mapping_names_every_flag_and_the_pair() {
        assert_eq!(
            flag_keys("--auto-update").unwrap(),
            &["app_check", "corpus_refresh"]
        );
        assert_eq!(flag_keys("--auto-corpus").unwrap(), &["corpus_refresh"]);
        assert_eq!(flag_keys("--app-check").unwrap(), &["app_check"]);
        assert_eq!(flag_keys("--auto-upgrade").unwrap(), &["auto_upgrade"]);
        for flag in VALID_FLAGS {
            assert!(flag_keys(flag).is_some(), "{flag} must map");
        }
        assert!(flag_keys("--auto").is_none());
        assert!(
            flag_keys("auto-update").is_none(),
            "the dashes are the vocabulary"
        );
    }

    #[test]
    fn template_is_well_commented_and_carries_every_key() {
        let t = template();
        for key in ["app_check", "corpus_refresh", "auto_upgrade"] {
            assert!(t.contains(&format!("{key} = ")), "template carries {key}");
        }
        // The owner's ask: the saved config is WELL COMMENTED — each key's
        // block carries its prose line above it.
        for phrase in [
            "Check for a newer cargo refine release",
            "Refresh the corpus lease",
            "Apply a pending upgrade automatically",
            "Precedence: the offline marker",
        ] {
            assert!(t.contains(phrase), "template explains {phrase:?}");
        }
    }

    #[test]
    fn set_on_missing_file_writes_the_commented_template() {
        let d = dir("missing_template");
        let p = settings_path(&d);
        let _ = std::fs::remove_file(&p);
        let line = set(&d, "auto_upgrade", true).unwrap();
        assert!(line.contains("auto_upgrade = true"));
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("auto_upgrade = true"));
        assert!(
            text.contains("# Refresh the corpus lease"),
            "comments survive"
        );
        assert!(text.contains("app_check = true"), "siblings keep defaults");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn every_key_flips_on_a_missing_file_the_first_set_must_stick() {
        // Regression guard for the debug-only-side-effect class: the
        // template write path must apply the REQUESTED value for EVERY
        // key (a debug_assert-wrapped replace compiles out in release and
        // silently wrote defaults — caught by the rig smoke, not by a
        // debug test). Assert the end state, all three keys.
        for (tag, key) in [
            ("flip_app", "app_check"),
            ("flip_corpus", "corpus_refresh"),
            ("flip_upgrade", "auto_upgrade"),
        ] {
            let d = dir(tag);
            let p = settings_path(&d);
            let _ = std::fs::remove_file(&p);
            set(&d, key, false).unwrap();
            let (s, warn) = read_with_warning(&d);
            assert!(warn.is_none());
            assert!(!s.get(key).unwrap(), "{key} must flip on the first set");
            let _ = std::fs::remove_file(&p);
        }
    }

    #[test]
    fn set_preserves_comments_and_sibling_values() {
        let d = dir("preserve_comments");
        let p = settings_path(&d);
        let _ = std::fs::remove_file(&p);
        set(&d, "corpus_refresh", true).unwrap();
        // The user hand-edits: a comment with their words + a flipped value.
        let text = std::fs::read_to_string(&p).unwrap();
        let edited = text.replace(
            "corpus_refresh = true",
            "corpus_refresh = false  # my laptop, metered wifi",
        );
        std::fs::write(&p, edited).unwrap();
        let (s, warn) = read_with_warning(&d);
        assert!(warn.is_none());
        assert!(!s.corpus_refresh, "the hand edit reads");
        assert!(s.app_check);
        // Flip it back — the user's trailing comment must survive.
        set(&d, "corpus_refresh", true).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(
            text.contains("corpus_refresh = true  # my laptop, metered wifi"),
            "the trailing comment survives: {text}"
        );
        assert!(text.contains("# my laptop"), "comment intact");
        let (s, warn) = read_with_warning(&d);
        assert!(warn.is_none());
        assert!(s.corpus_refresh);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn malformed_value_reads_defaults_with_a_warning() {
        let d = dir("malformed_warn");
        let p = settings_path(&d);
        std::fs::write(&p, "[auto]\napp_check = yes-please\n").unwrap();
        let (s, warn) = read_with_warning(&d);
        assert_eq!(s, Settings::default(), "malformed reads defaults");
        let w = warn.expect("a malformed record warns");
        assert!(w.contains("could not be parsed"));
        assert!(w.contains(SETTINGS_FILE));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn unknown_sections_and_keys_are_ignored_not_destroyed() {
        let d = dir("unknown_sections");
        let p = settings_path(&d);
        std::fs::write(
            &p,
            "[other]\nfancy = true\n\n[auto]\napp_check = false\nmystery = 7\n",
        )
        .unwrap();
        let (s, warn) = read_with_warning(&d);
        assert!(
            warn.is_none(),
            "unknown shapes are ignored, never malformed"
        );
        assert!(!s.app_check);
        assert!(s.corpus_refresh);
        set(&d, "app_check", true).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(
            text.contains("[other]"),
            "the user's other section survives"
        );
        assert!(text.contains("fancy = true"));
        assert!(text.contains("mystery = 7"));
        assert!(text.contains("app_check = true"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn missing_key_in_existing_file_converges_to_the_commented_shape() {
        let d = dir("converge");
        let p = settings_path(&d);
        std::fs::write(&p, "[auto]\napp_check = false\n").unwrap();
        set(&d, "auto_upgrade", true).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("auto_upgrade = true"));
        assert!(text.contains("# re-added by `cargo refine off|on`"));
        let (s, warn) = read_with_warning(&d);
        assert!(warn.is_none());
        assert!(s.auto_upgrade);
        assert!(!s.app_check, "the pre-existing value survives the append");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn set_refuses_an_unknown_key() {
        assert!(set(&dir("unknown_key"), "not_a_key", true).is_err());
    }
}
