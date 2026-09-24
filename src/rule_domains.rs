//! The compiled rule→domain table (v3 — Issue 103; moved UNGATED from
//! `kat_sync_payload` by Issue 134 T1 so the score-bench/demonstrability
//! lanes can project through the SAME table without the `kat_service` dep
//! tree — one source of truth, re-exported at every historical path).
//!
//! Nothing here reads env, network, or cargo: the table is a pure read of
//! the in-memory corpus tables compiled into the binary.

/// The domain id a `rustc_errors`-attributed row carries — the receiver's
/// registry appends it as its own slot (the corpus-view slot law).
pub const RUSTC_ERRORS_DOMAIN: &str = "rustc_errors";

/// The compiled rule→domain table: corpus membership per retrieval domain
/// plus the `rustc_errors` playbook-code set. One source of truth for every
/// payload-domain attribution (the sync builder AND the pending estimator
/// project through the same instance, so estimator/payload agreement stays
/// by construction).
///
/// `default()` is the COMPILED table, process-cached ([`Self::compiled`]) —
/// deterministic per binary (a pure read of in-memory corpus tables; no
/// cargo, no network, no clock). A domain whose feature is not compiled in
/// contributes nothing: its rows cross as `""` → the receiver's `other`,
/// the honest unknown, never a guess.
///
/// `shader_perf` is deliberately ABSENT (Issue 103's domain list predates
/// it, and the receiver's 8-slot registry has exactly one free slot after
/// `rustc_errors`): shader rows cross as `""` → `other` until a future
/// change widens the registry. Honest, visible, never mislabeled.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleDomains {
    /// `rule_id → domain id` for the retrieval-domain corpora.
    pub rules: std::collections::BTreeMap<String, String>,
    /// The compiled `rustc_errors` playbook codes (`E0502`, …). A row whose
    /// `lint_key` is `"{code}:{strategy}"` ([`crate::domains::rustc_errors
    /// ::evolve::traj_key`]) attributes to `rustc_errors` when its code half
    /// is a member.
    pub rustc_codes: std::collections::BTreeSet<String>,
}

impl RuleDomains {
    /// The empty table — every row `""`. The golden fixture's explicit table
    /// (and tests that need feature-independent bytes) use this or a
    /// hand-built instance; production builds default to [`Self::compiled`].
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// The COMPILED table, process-cached. Same binary → same table, so the
    /// sync builder and the pending estimator never disagree about a row's
    /// domain within one install.
    #[must_use]
    pub fn compiled() -> Self {
        static TABLE: std::sync::OnceLock<RuleDomains> = std::sync::OnceLock::new();
        TABLE.get_or_init(Self::build).clone()
    }

    fn build() -> Self {
        use crate::domain::HealEntry as _;
        let mut out = Self::empty();
        macro_rules! corpus_domain {
            ($feat:literal, $name:literal, $build:path) => {
                #[cfg(feature = $feat)]
                for e in $build().entries {
                    out.rules
                        .entry(e.rule_id().to_string())
                        .or_insert_with(|| $name.to_string());
                }
            };
        }
        corpus_domain!("clippy_lints", "clippy", crate::build_seed_corpus);
        corpus_domain!("rust_perf", "rust_perf", crate::build_perf_seed_corpus);
        corpus_domain!("kernel_heal", "kernel_opt", crate::build_kernel_seed_corpus);
        corpus_domain!("sec", "sec", crate::build_sec_seed_corpus);
        corpus_domain!("docker", "docker", crate::build_docker_seed_corpus);
        corpus_domain!("dist", "dist", crate::build_dist_seed_corpus);
        #[cfg(feature = "rustc_errors")]
        for (code, _) in crate::domains::rustc_errors::evolve::playbook_inventory() {
            out.rustc_codes.insert(code);
        }
        out
    }

    /// The domain id for one store row's `lint_key`: exact corpus hit → that
    /// domain; `{code}:{strategy}` with a compiled playbook code →
    /// [`RUSTC_ERRORS_DOMAIN`]; otherwise `""` (the receiver's `other`).
    #[must_use]
    pub fn domain_for(&self, lint_key: &str) -> &str {
        if let Some(d) = self.rules.get(lint_key) {
            return d;
        }
        if !self.rustc_codes.is_empty() {
            if let Some(code) = lint_key.split(':').next() {
                if self.rustc_codes.contains(code) {
                    return RUSTC_ERRORS_DOMAIN;
                }
            }
        }
        ""
    }

    /// Aggregate `(rule_id, count)` pairs into per-domain totals for the
    /// fix-stats push (Plan 038 Phase 2 — the fixstats lane's `by_domain`
    /// had stayed honest-empty because the push tail could not attribute
    /// a rule to a domain; this is the SAME compiled attribution the
    /// mining v3 sync rows carry, so the two lanes cannot disagree).
    /// Unattributed rules fold into the reserved `"other"` bucket — the
    /// corpus view's own catch-all — never dropped, never guessed at a
    /// domain. Pure over its input; the empty run folds to empty.
    #[must_use]
    pub fn fold_rule_domains(&self, rules: &[(String, u32)]) -> Vec<(String, u32)> {
        let mut by_domain: std::collections::BTreeMap<String, u32> =
            std::collections::BTreeMap::new();
        for (rule, count) in rules {
            let domain = self.domain_for(rule);
            let key = if domain.is_empty() { "other" } else { domain };
            *by_domain.entry(key.to_string()).or_insert(0) += *count;
        }
        by_domain.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{RuleDomains, RUSTC_ERRORS_DOMAIN};

    #[test]
    fn domain_attribution_is_table_membership_then_rustc_code() {
        let d = RuleDomains {
            rules: [("needless_return".to_string(), "clippy".to_string())]
                .into_iter()
                .collect(),
            rustc_codes: ["E0502".to_string(), "E0597".to_string()]
                .into_iter()
                .collect(),
        };
        assert_eq!(d.domain_for("needless_return"), "clippy");
        assert_eq!(d.domain_for("totally_unknown_rule"), "");
        assert_eq!(d.domain_for("E0502:hoist-to-first-use"), RUSTC_ERRORS_DOMAIN);
        // A code NOT in the compiled playbook inventory stays unknown —
        // attribution is membership, never the `E…` SHAPE.
        assert_eq!(d.domain_for("E9999:not_a_playbook"), "");
        // A BARE compiled code (no `:strategy` half) attributes too —
        // membership is on the code half, whatever the key's shape.
        assert_eq!(d.domain_for("E0502"), RUSTC_ERRORS_DOMAIN);
    }

    #[cfg(feature = "clippy_lints")]
    #[test]
    fn compiled_table_attributes_corpus_rules() {
        let d = RuleDomains::compiled();
        assert_eq!(d.domain_for("needless_return"), "clippy");
        // A composite rustc key is NOT a corpus rule id.
        assert_ne!(d.domain_for("E0502:hoist-to-first-use"), "clippy");
    }

    #[cfg(feature = "rustc_errors")]
    #[test]
    fn compiled_table_attributes_rustc_codes() {
        let d = RuleDomains::compiled();
        assert_eq!(d.domain_for("E0502:hoist-to-first-use"), RUSTC_ERRORS_DOMAIN);
        assert_eq!(d.domain_for("E0614:deref-at-use"), RUSTC_ERRORS_DOMAIN);
    }

    #[test]
    fn fold_rule_domains_aggregates_and_folds_unknown_into_other() {
        let d = RuleDomains {
            rules: [
                ("needless_return".to_string(), "clippy".to_string()),
                ("shrink-to-fit-after-build".to_string(), "rust_perf".to_string()),
            ]
            .into_iter()
            .collect(),
            rustc_codes: ["E0502".to_string()].into_iter().collect(),
        };
        let folded = d.fold_rule_domains(&[
            ("needless_return".to_string(), 3),
            ("E0502:hoist".to_string(), 2),
            ("needless_return".to_string(), 4),
            ("totally_unknown".to_string(), 5),
            ("shrink-to-fit-after-build".to_string(), 1),
        ]);
        // BTreeMap order (alphabetical), repeated rules sum, unknown →
        // the reserved bucket, never dropped.
        assert_eq!(
            folded,
            vec![
                ("clippy".to_string(), 7),
                ("other".to_string(), 5),
                ("rust_perf".to_string(), 1),
                (RUSTC_ERRORS_DOMAIN.to_string(), 2),
            ]
        );
        assert!(d.fold_rule_domains(&[]).is_empty(), "the empty run folds empty");
    }
}
