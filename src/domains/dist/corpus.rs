//! The dist seed corpus — Phase 1 (Issue 085).
//!
//! Seven rules, all suggest-only, every one with measured provenance — the
//! first four from the session that found the class (the owner had to ASK why
//! the binary was 2x — that is the failure this domain exists to prevent),
//! the fifth from upstream's own measured compile-time series (Batch 143 —
//! nnethercote Dec-2025: `windows`/`rustix`/`aws-sdk-ec2` named with
//! `-Zhint-mostly-unused` wins for small-fraction consumers), the sixth
//! from the July-2026 series (Batch 146 — rustc #159091: PGO training-set
//! coverage, the out-of-manifest next lever for the fully-tuned audience),
//! the seventh from bevy's own benches split (Batch 221 — #25321: per-crate
//! bench features, optional bench deps, required-features per [[bench]]).
//! The dedup verdict rides each rule's `incumbent` field: `cargo` itself
//! warns about NOTHING here (checked live 2026-09-09 — a bare release build
//! of a bin crate with no profile section compiles silent), so all seven are
//! Class B (novel detection) with no incumbent to overlay.
//!
//! | Rule | Family | Fix | Incumbent |
//! |---|---|---|---|
//! | `shipped-binary-unstripped` | `ShipSize` | suggest | none |
//! | `shipped-binary-missing-size-profile` | `ShipSize` | suggest | none |
//! | `cli-panic-unwind-tables` | `ShipSize` | suggest | none |
//! | `member-profile-ignored` | `ProfilePlacement` | suggest (gated) | none |
//! | `large-api-dependency-mostly-unused-hint` | `BuildTime` | suggest | none |
//! | `pgo-training-set-covers-real-workload-hint` | `BuildTime` | suggest | none |
//! | `bench-targets-shared-dependency-cluster-hint` | `BuildTime` | suggest | none |

use super::{DistDomain, DistFamily, DistRule};
use crate::corpus::Corpus;

/// Build the dist seed corpus (BLAKE3-committed via `Corpus<D>`).
#[must_use]
pub fn build_dist_seed_corpus() -> Corpus<DistDomain> {
    Corpus::new(entries())
}

/// Shorthand constructor — the corpus is data (the docker `rule()` precedent).
fn rule(
    rule_id: &str,
    dist_concept: &str,
    trigger: &str,
    buggy_code: &str,
    fixed_code: &str,
    family: DistFamily,
    notes: Option<&str>,
) -> DistRule {
    DistRule {
        rule_id: rule_id.to_string(),
        dist_concept: dist_concept.to_string(),
        trigger: trigger.to_string(),
        buggy_code: buggy_code.to_string(),
        fixed_code: fixed_code.to_string(),
        family,
        notes: notes.map(str::to_string),
    }
}

fn entries() -> Vec<DistRule> {
    use DistFamily as F;
    vec![
        rule(
            "shipped-binary-unstripped",
            "a shipped binary's default symbol table IS the internal architecture — every function name, every module boundary, readable by anyone who downloads it",
            "a manifest with [[bin]] and no strip key in any [profile.*] section",
            "[[bin]]\nname = \"mytool\"\n\n[dependencies]\nserde = \"1\"\n",
            "[[bin]]\nname = \"mytool\"\n\n[profile.release]\nstrip = true\n\n[dependencies]\nserde = \"1\"\n",
            F::ShipSize,
            Some(
                "Measured basis: the unstripped shipping binary of this very tool exposed 14,485 symbols (Proposal 003 G2, 2026-09-06). strip = \"symbols\" and strip = true are equivalent here; any explicit strip key satisfies the rule.",
            ),
        ),
        rule(
            "shipped-binary-missing-size-profile",
            "the default release profile optimizes for SPEED, not size — for a shipped CLI the knob set (lto, codegen-units, opt-level for size, abort, strip) is commonly a 40-60% byte reduction on the same behavior",
            "a manifest with [[bin]] and no lto key in any [profile.*] section",
            "[[bin]]\nname = \"mytool\"\n",
            "[[bin]]\nname = \"mytool\"\n\n# Size-optimized DIST profile — release via `cargo build --profile dist`.\n# NOT [profile.release]: [profile.bench] inherits release, and opt-level/\n# codegen-units there poison criterion benches.\n[profile.dist]\ninherits = \"release\"\nopt-level = \"s\"\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\nstrip = true\n",
            F::ShipSize,
            Some(
                "Measured basis: 11,903,168 -> 4,546,448 bytes (-61%) on this tool's own shipping feature set, arm64 (Issue 085). cargo bloat first: 70% of .text was own-code tables, so codegen knobs win, dep removal does not. The suggestion is a SEPARATE profile on purpose — see the domain's refuses() table.",
            ),
        ),
        rule(
            "cli-panic-unwind-tables",
            "panic = unwind bakes landing pads and unwind tables into every function so panics can walk the stack — bytes a short-lived CLI tool never needs",
            "a manifest with [[bin]] and no panic key in any [profile.*] section",
            "[[bin]]\nname = \"mytool\"\n",
            "[[bin]]\nname = \"mytool\"\n\n[profile.dist]\ninherits = \"release\"\npanic = \"abort\"\n",
            F::ShipSize,
            Some(
                "Context-dependent ON PURPOSE: a server that isolates panics per task or connection (a panicked worker must not take down the process) RELIES on unwinding — under abort, one panic kills the whole server. The rule fires only on manifests; deciding CLI-vs-server is the reader's call, which is why this is suggest-only with this note.",
            ),
        ),
        rule(
            "member-profile-ignored",
            "cargo applies profile settings from the WORKSPACE ROOT manifest only — a [profile.*] block in a member crate is silently ignored when the crate is built through the workspace, and cargo only warns in newer toolchains under some invocations",
            "a member manifest (no [workspace] of its own) carrying [profile.*] sections, when a parent manifest declares [workspace] — caller evidence, silent without it",
            "[package]\nname = \"member-tool\"\n\n[[bin]]\nname = \"member-tool\"\n\n[profile.release]\nstrip = true\n",
            "# The block belongs in the workspace ROOT manifest; see that file.\n[package]\nname = \"member-tool\"\n\n[[bin]]\nname = \"member-tool\"\n",
            F::ProfilePlacement,
            Some(
                "Precision caveat: the evidence is a nearest-ancestor [workspace] walk. An `exclude`d member (built standalone) legitimately keeps its profiles — the advice text hedges accordingly. Silent without the walk evidence, never guessed (the docker ScanContext pattern).",
            ),
        ),
        rule(
            "large-api-dependency-mostly-unused-hint",
            "a small-fraction consumer of a very large API crate pays the crate's whole compile-time cost — rustc's -Zhint-mostly-unused tells the compiler only a tiny fraction is used, with big compile-time wins reported for windows/rustix/aws-sdk-* consumers",
            "a manifest whose [dependencies] (or any *dependencies section) names a known-large API crate: windows, windows-sys, rustix, or aws-sdk-*",
            "[package]\nname = \"widget\"\n\n[dependencies]\nserde = \"1\"\nwindows = { version = \"0.58\", features = [\"Win32_Foundation\", \"Win32_Storage\"] }\n",
            "# The fix is NOT a manifest edit — the hint is a rustc flag, set once\n# for the workspace. This IS the fixed artifact: .cargo/config.toml\n# (nightly-only; MEASURE per crate before adopting).\n[build]\nrustflags = [\"-Z\", \"hint-mostly-unused\"]\n",
            F::BuildTime,
            Some(
                "Source: nnethercote, \"How to speed up the Rust compiler in December 2025\" — 'Josh Triplett introduced a new experimental flag, -Zhint-mostly-unused, which can give big compile time wins for people using small fractions of very large crates ... such as windows, rustix, and aws-sdk-ec2.' Caveats the advice text carries: nightly-only -Z; the flag lives in .cargo/config.toml rustflags — OUTSIDE the manifest, which is why the fixed fixture IS the config file (the scanner is manifest-scoped and cannot see adoption, so the suggestion repeats until the dep is dropped or the hint is measured in); per-crate measurement required before adopting. The crate table (windows, windows-sys, rustix, aws-sdk- prefix) is deliberately small — a miss is a silent no-advice, an over-broad table is noise.",
            ),
        ),
        rule(
            "pgo-training-set-covers-real-workload-hint",
            "a PGO-trained release binary only claims the win its TRAINING SET covers — a narrow or unrepresentative workload mix leaves the majority of the PGO win unclaimed even when the profile machinery is perfectly configured",
            "a manifest with [[bin]] whose effective [profile.*] knobs (strip, lto, panic) are ALL set — the manifest-level levers exhausted, the audience whose next lever is out-of-manifest",
            "[[bin]]\nname = \"mytool\"\n\n[profile.dist]\ninherits = \"release\"\nstrip = true\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\n",
            "# The fix is NOT a manifest edit — PGO training-set coverage is a\n# build-pipeline property. This IS the fixed artifact: the training\n# workload must be the SHIPPED workload mix (rustc #159091 added\n# rustdoc benches to rustc's PGO training set: +2.85% mean wall-time\n# across rustdoc benchmarks, >6% best — 'the training set matters').\n# Train on your real corpus; measure before/after.\n# (pgo training pipeline snippet — out-of-manifest by construction)",
            F::BuildTime,
            Some(
                "Source: nnethercote, \"How to speed up the Rust compiler in July 2026\" (2026-07-31), #159091 (Jakub Beránek), verbatim: 'added rustdoc benchmarks to the PGO training set, which lets rustdoc benefit more from PGO. This gave a mean 2.85% reduction in wall-time across all the rustdoc benchmarks, with the largest reduction exceeding 6%. PGO is fiddly to set up but it can really make a difference, and the training set matters.' PROXY-TRIGGERED BY DESIGN (the large-api-dependency precedent): the scanner cannot see whether PGO exists at all — the trigger is the fully-tuned effective profile (strip+lto+panic all set), i.e. the audience that has exhausted the manifest-level levers this domain advises; the -hint suffix + the conditional advice ('IF you use or add PGO') + the measure-first caveat are load-bearing. Strictly additive by construction: silent for every manifest with a missing knob (the three ship-size fixtures are unaffected), and on a fully-tuned shipped bin it fires alone — no sibling rule co-fires.",
            ),
        ),
        rule(
            "bench-targets-shared-dependency-cluster-hint",
            "a bench package whose [[bench]] targets share ONE dependency set compiles the UNION of every target's heavy deps to run ANY one of them — per-crate bench features (ALL default-on) with optional bench deps + a required-features row per [[bench]] let a single-crate bench loop skip the heavy siblings",
            "a manifest with 3 or more [[bench]] targets, none carrying a required-features key",
            "# One bench package, many targets, ONE shared dependency set: running\n# the ecs bench compiles the render stack too (bevy #25321's own story).\n[package]\nname = \"benches\"\nautobenches = false\n\n[dependencies]\ncriterion = \"0.8\"\necs-core = { path = \"../crates/ecs-core\" }\nrender-stack = { path = \"../crates/render-stack\" }\n\n[[bench]]\nname = \"ecs\"\nharness = false\n\n[[bench]]\nname = \"render\"\nharness = false\n\n[[bench]]\nname = \"scene\"\nharness = false\n",
            "# The split: bench deps become OPTIONAL normal deps; one default-on\n# feature per measured crate; every [[bench]] names its feature. Bare\n# `cargo bench` stays behavior-identical (default = all); a single-crate\n# loop runs --no-default-features --features bench_ecs and skips the\n# render build entirely. The required-features rows are LOAD-BEARING.\n[package]\nname = \"benches\"\nautobenches = false\n\n[dependencies]\ncriterion = \"0.8\"\necs-core = { path = \"../crates/ecs-core\", optional = true }\nrender-stack = { path = \"../crates/render-stack\", optional = true }\n\n[features]\ndefault = [\"bench_ecs\", \"bench_render\", \"bench_scene\"]\nbench_ecs = [\"dep:ecs-core\"]\nbench_render = [\"dep:render-stack\"]\nbench_scene = [\"dep:ecs-core\"]\n\n[[bench]]\nname = \"ecs\"\nharness = false\nrequired-features = [\"bench_ecs\"]\n\n[[bench]]\nname = \"render\"\nharness = false\nrequired-features = [\"bench_render\"]\n\n[[bench]]\nname = \"scene\"\nharness = false\nrequired-features = [\"bench_scene\"]\n",
            F::BuildTime,
            Some(
                "Source: bevyengine/bevy #25321, 'Split benchmarks with features to allow avoiding unnecessary compilation' (merged 2026-10-05 @ 430c448d4b0878741fa025836ffc26b852438cf2, +122/-20 — benches/Cargo.toml only), PR body verbatim: 'Since benchmarks share the same dependencies, that includes `bevy_render`, even though I'm only trying to run `bevy_ecs` benchmarks. `bevy_render` also happens to take way longer to compile than anything else.' + 'Add features to `benches` to enable specific crates' benchmarks: feature `bench_ecs` for `bevy_ecs`, `bench_render` for `bevy_render`, etc.' + 'All features are enabled by default, so default behavior should be the same.' + 'The dependencies are normal dependencies instead of dev-dependencies now, but they're also now `optional = true`.' — bevy's landed split: 10 bench_* features, deps optional-normal, required-features per [[bench]]; the merged resolution to the review's automated-benchmarking question IS the default-on half (bare `cargo bench` compiles everything, unchanged; the win is realized by the targeted --no-default-features --features bench_ecs invocation; the author ran both postures). ⛔ OUR OWN lore rides the advice: a feature-gated bench target WITHOUT its required-features row is skipped SILENTLY by --all-targets, and a whole-file-gated one reports ok. 0 passed (the cfg-gated green-zero class, katgpt-rs bench_694) — the rows are load-bearing, never decorative; this advice is for humans cutting bench compile time, never for gate-asserted suites. Detector precision: fires on >= 3 [[bench]] with NO required-features key on ANY of them (2 targets have nothing to split); the gated form is the fixed artifact and stays silent (strictly additive — every pre-existing fixture is benchless). RIDER (Batch 224 / Plan 226 — the C++ twin, ggml-org/llama.cpp @ ccdbe290533f68154ede46e0618622eebcb6c070 #30299, MIT, merged 2026-10-12, the commit immediately after 073b74034): 'tests : link each test against only the libraries it uses' — the SAME law in CMake spelling: the test-helper functions gain a LINK keyword (default llama-common) so CALL SITES DECLARE the minimal link set — 'ggml-only tests now link just ggml: test-opt, test-metal-graph-optimize, test-backend-ops, test-tiled-mulmat, test-barrier, test-quantize-fns, test-quantize-perf, test-rope, test-col2im-1d, test-gguf, test-alloc' + 'llama-only tests drop llama-common' + 'both mtmd tests link mtmd only, dropping the LLAMA_TEST_NAME / target_link_libraries workarounds'. TWO CARRY-OVER ATOMS for the Rust twin: (1) the INTERFACE-LEAK half — 'Both helpers now set cxx_std_17 explicitly, since the tests no longer inherit it from llama-common's PUBLIC interface.' — a shared helper that propagates a language standard PUBLICly makes every consumer depend on it transitively; the moment the fat dep leaves, the implicit std goes with it (the CMake face of the leaky-interface class — split the link set AND the helper must OWN what it used to propagate); (2) the WORKAROUND-COLLAPSE half — per-target post-hoc target_link_libraries patches (the LLAMA_TEST_NAME set/use/unset dance) exist exactly because the helper could not express per-test link sets; give the helper the parameter and the patches die ('tests/testing.h no longer includes common.h; the only symbol it used from there (string_join) is inlined' — the same collapse at the include seam). Cross-language confirmation of the Rust rule's mechanism-naming: the cost is at LINK/compile time, the unit of minimization is the PER-TARGET declared set, and the default stays the union (llama-common) so bare builds are unchanged — opt-in narrowing, never a default flip.",
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::dist::DistContext;
    use crate::domains::dist::rules::{DIST_RULES, scan_source_with};

    #[test]
    fn rule_ids_unique_and_kebab() {
        let mut seen = std::collections::BTreeSet::new();
        for r in entries() {
            assert!(
                seen.insert(r.rule_id.clone()),
                "duplicate rule id {}",
                r.rule_id
            );
            assert!(
                r.rule_id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} is not kebab-case",
                r.rule_id
            );
        }
    }

    #[test]
    fn corpus_and_detector_rule_sets_agree() {
        let ids: std::collections::BTreeSet<String> =
            entries().into_iter().map(|r| r.rule_id).collect();
        let detectors: std::collections::BTreeSet<String> =
            DIST_RULES.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            ids, detectors,
            "the corpus and the detector table have drifted apart"
        );
    }

    /// The Issue 085 precision contract, mechanically: every buggy fixture
    /// produces EXACTLY its own rule's citation (no cross-fire), and every
    /// fixed fixture produces ZERO.
    #[test]
    fn fixtures_fire_exactly_their_own_rule() {
        for r in entries() {
            let buggy = scan_source_with(&r.buggy_code, DistContext::default());
            let fixed = scan_source_with(&r.fixed_code, DistContext::default());
            // Context-gated rule: without evidence the member rule itself is
            // silent (its fire-half is tested with evidence below). The other
            // ship-size rules MAY cross-fire its fixtures — they share a
            // trigger shape — so assert only the member rule's absence.
            if r.rule_id == "member-profile-ignored" {
                assert!(
                    buggy.iter().all(|f| f.rule_id != r.rule_id),
                    "{}: must be silent without caller evidence",
                    r.rule_id
                );
                continue;
            }
            assert!(
                buggy.iter().any(|f| f.rule_id == r.rule_id),
                "{}: buggy fixture does not fire its own rule (got {:?})",
                r.rule_id,
                buggy.iter().map(|f| f.rule_id).collect::<Vec<_>>()
            );
            assert!(
                fixed.iter().all(|f| f.rule_id != r.rule_id),
                "{}: fixed fixture still fires its own rule",
                r.rule_id
            );
        }
    }

    /// The member rule WITH evidence: fires on the buggy side, and the
    /// fixed side (block removed) is silent.
    #[test]
    fn member_rule_fires_with_workspace_evidence() {
        let ctx = DistContext {
            under_workspace_root: true,
            ..DistContext::default()
        };
        let buggy =
            "[package]\nname = \"m\"\n\n[[bin]]\nname = \"m\"\n\n[profile.release]\nstrip = true\n";
        let findings = scan_source_with(buggy, ctx);
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "member-profile-ignored")
        );
        // A virtual root (workspace, no package/bin) is silent even with the
        // evidence flag — nothing ships from it.
        let root = "[workspace]\nmembers = [\"m\"]\n";
        assert_eq!(scan_source_with(root, ctx), []);
    }

    /// Batch 143 — the large-API rule: fires ONCE per manifest at the first
    /// hit across every *dependencies section; the config.toml fix is silent
    /// (no dependency sections — the out-of-manifest honesty the notes
    /// carry); small/commented deps never fire.
    #[test]
    fn large_api_dep_fires_once_and_config_silences() {
        let multi = "[dependencies]\nserde = \"1\"\nwindows = \"0.58\"\nrustix = \"1\"\n\n[dev-dependencies]\naws-sdk-ec2 = \"1\"\n";
        let findings = scan_source_with(multi, DistContext::default());
        let r3: Vec<_> = findings
            .iter()
            .filter(|f| f.rule_id == "large-api-dependency-mostly-unused-hint")
            .collect();
        assert_eq!(
            r3.len(),
            1,
            "one advice per manifest, anchored at the FIRST large dep"
        );
        assert_eq!(r3[0].line, 3, "windows is the first large dependency");
        assert!(
            r3[0].excerpt.starts_with("windows"),
            "the excerpt names the triggering crate"
        );

        // The fixed artifact — the .cargo/config.toml — is silent.
        let fixed = entries()
            .into_iter()
            .find(|r| r.rule_id == "large-api-dependency-mostly-unused-hint")
            .unwrap();
        assert_eq!(
            scan_source_with(&fixed.fixed_code, DistContext::default()),
            []
        );

        // A manifest without large deps never fires.
        let small = "[dependencies]\nserde = \"1\"\ntokio.workspace = true\n";
        assert_eq!(scan_source_with(small, DistContext::default()), []);
    }

    /// The negative controls: lib-only crates never get ship-size advice,
    /// and a complete profile silences every rule.
    #[test]
    fn lib_only_and_complete_profiles_stay_silent() {
        let lib_only = "[package]\nname = \"lib\"\n\n[lib]\ncrate-type = [\"rlib\"]\n";
        assert_eq!(scan_source_with(lib_only, DistContext::default()), []);
        // Batch 146: a COMPLETE profile no longer scans empty — it earns
        // exactly ONE finding, the PGO next-lever hint (the manifest-level
        // levers are exhausted; the ship-size rules stay silent). The
        // original intent — no SHIP-SIZE noise on a tuned manifest — is
        // preserved and pinned below.
        let complete = "[[bin]]\nname = \"t\"\n\n[profile.dist]\ninherits = \"release\"\nopt-level = \"s\"\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\nstrip = true\n";
        let findings = scan_source_with(complete, DistContext::default());
        let ids: Vec<&str> = findings.iter().map(|f| f.rule_id).collect();
        assert_eq!(
            ids,
            vec!["pgo-training-set-covers-real-workload-hint"],
            "a fully-tuned bin earns exactly the PGO hint — no ship-size noise"
        );
    }

    /// The suggest-only contract: no dist rule is fixable in Phase 1.
    #[test]
    fn phase1_is_suggest_only() {
        assert_eq!(crate::domains::dist::rules::FIXABLE_DIST, &[] as &[&str]);
    }

    /// Batch 221 — the bench-cluster rule: fires at >= 3 ungated
    /// [[bench]] targets, silent at 2, and the gated form (the fixed
    /// artifact, required-features rows present) is silent.
    #[test]
    fn bench_cluster_rule_boundary_and_gated_silence() {
        let two = "[[bench]]\nname = \"a\"\n\n[[bench]]\nname = \"b\"\n";
        assert_eq!(scan_source_with(two, DistContext::default()), []);

        let three =
            "[[bench]]\nname = \"a\"\n\n[[bench]]\nname = \"b\"\n\n[[bench]]\nname = \"c\"\n";
        let findings = scan_source_with(three, DistContext::default());
        let hits: Vec<_> = findings
            .iter()
            .filter(|f| f.rule_id == "bench-targets-shared-dependency-cluster-hint")
            .collect();
        assert_eq!(hits.len(), 1, "one advice per manifest");
        assert_eq!(hits[0].line, 1, "anchored at the first bench header");
        assert!(hits[0].excerpt.contains('3'), "the excerpt names the count");

        // One gated target is enough to silence (the split landed): the
        // detector's ALL-gated clause reads the fixed form as done.
        let half_gated = "[[bench]]\nname = \"a\"\n\n[[bench]]\nname = \"b\"\n\n[[bench]]\nname = \"c\"\nrequired-features = [\"bench_c\"]\n";
        assert_eq!(scan_source_with(half_gated, DistContext::default()), []);

        let fixed = entries()
            .into_iter()
            .find(|r| r.rule_id == "bench-targets-shared-dependency-cluster-hint")
            .unwrap();
        assert_eq!(
            scan_source_with(&fixed.fixed_code, DistContext::default()),
            []
        );

        // Batch 224 rider (Plan 226): the C++ twin's needles — the
        // #30299 LINK-keyword law landed on this rule's notes
        // (count-neutral; every needle verbatim at the pin).
        let notes = fixed.notes.as_ref().unwrap();
        for needle in [
            "RIDER (Batch 224 / Plan 226",
            "ccdbe290533f68154ede46e0618622eebcb6c070",
            "link each test against only the libraries it uses",
            "ggml-only tests now link just ggml",
            "Both helpers now set cxx_std_17 explicitly, since the tests no longer inherit it from llama-common's PUBLIC interface.",
            "dropping the LLAMA_TEST_NAME / target_link_libraries workarounds",
        ] {
            assert!(
                notes.contains(needle),
                "batch-224 dist rider missing needle: {needle}"
            );
        }
    }
}
