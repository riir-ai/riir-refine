//! Entries slice — Batch 214 (Plan 214) — the conversion set's TAIL, part 1
//! (the own-rule member): one `ggml-org/llama.cpp` commit (MIT; the full
//! `.patch` fetched at
//! `https://github.com/ggml-org/llama.cpp/commit/1167d3f4….patch` — the
//! patch-at-pin read is the verification source; quotes verified VERBATIM).
//!
//! Pack composition (category split D +1; corpus 777 → 778):
//! - D +1 `componentwise-version-guard-rollover-silences-fast-path`
//!   (1167d3f4 #29453 — a feature guard comparing version components
//!   INDEPENDENTLY (`MAJOR >= N && MINOR >= M`) evaluates false on every
//!   future major rollover and silently un-defines the fast path it
//!   guards — a quiet performance loss, never a compile error)
//!
//! The same batch's FOUR count-neutral riders live on their hosts:
//! `profile-window-drains-queued-work-before-open` (`entries_gpu_77`, the
//! `cubecl` #1779 lost-device election stop), `fewrow-tensor-tile-decode-
//! into-rhs-operand` (`entries_gpu_66`, `PrismML` #225 the per-dispatch-path
//! row-tile axis), `one-shot-staging-release-to-system` (`rust_perf`
//! `entries_15`, `PrismML` #312 the evict-to-reuse pole), and
//! `argmax-epilogue-replaces-output-materialization` (`entries_gpu_52`,
//! `PrismML` #313 the host-chain k>1 top-k arm).
//!
//! STAGING DRIFT carried (the TIP-wins law, B200 precedent): the staging
//! row named "the fix = the PACKED compare"; the LANDED diff carries the
//! ORDERED component guard (`CCCL_MAJOR_VERSION > 3 || (CCCL_MAJOR_VERSION
//! == 3 && CCCL_MINOR_VERSION >= 1)`, the `ORippler` review suggestion) —
//! the packed-integer form is the commit message's RATIONALE, not the
//! landed spelling. Both are carried in-rule, the landed form first.
//!
//! DEDUP (run at authoring, both corpora):
//! - `MAJOR_VERSION / major..minor / version gate|guard|compare|rollover /
//!   CCCL` — one hit, `functional-cache-versioning-loses-donation-under-
//!   kernel-read` (B100) — a cache-versioning copy-amplification hazard,
//!   different mechanism. No owner for the component-compare rollover.
//! - `measured on nothing / elect a winner / lost device / device
//!   poisoned` — zero hits (the Lead 5 rider's election axis is unowned;
//!   the family host is the B207 measurement-honesty rule).
//! - `MUL_MAT_ID / dispatch path / per-path width` — the adjacent rules
//!   (`ids-path-rebinds-stride-meaning` B198, `tile-picker-input-from-
//!   dispatch-grid-partition` B192, `dispatcher-eligibility-predicate-
//!   lags-kernel-capability` B103) own stride-meaning rebinds, selector
//!   inputs, and eligibility lags — none owns a row-tile WIDTH validated
//!   per (type, dispatch-path).
//! - `evict..reuse / spare / zero-filled by resize / page-faulted again`
//!   — zero owners for the eviction-handoff face.
//! - `partial_sort / chain prefix / sampler / threshold prefilter` — the
//!   adjacent rules (monotone-transform-elision, refine-by-rescan,
//!   selector-lattice-beam) own transform elision, tie collection, and
//!   draft walks — none owns the chain-prefix no-op admission + top-k
//!   from the source.

use crate::domains::kernel_opt::KernelRule;

pub(super) fn entries() -> Vec<KernelRule> {
    vec![
        // ── D: guard arithmetic (#29453) — the version guard whose
        // component-wise compare breaks at major rollover; the fast
        // path silently stops being defined.
        KernelRule {
            rule_id: "componentwise-version-guard-rollover-silences-fast-path".to_string(),
            gpu_concept: "MECHANISM (a version guard's component-wise compare breaks at major rollover; category D guard arithmetic): a feature guard spelled as two INDEPENDENT component compares — 'CCCL_MAJOR_VERSION >= 3 && CCCL_MINOR_VERSION >= 1' — evaluates FALSE on every future major rollover, because the minor resets to 0 when a new major series is cut: on 4.x the guard reads '4 >= 3 && 0 >= 1, i.e. false' (the source's own arithmetic, verbatim). The consequence is SILENT: 'STRIDED_ITERATOR_AVAILABLE stops being defined and argsort silently falls back to the init_offsets path. Nothing warns and the build still succeeds, so the regression is a quiet performance loss rather than a compile error.' THE FAILURE-MODE LAW: a guard arithmetic bug is a TIME BOMB keyed to the upstream version calendar — it passes every build and every test TODAY, and detonates as a fast-path loss at the next major bump, with no error anywhere. THE REPAIRS (two spellings, both carried): (1) the LANDED form — the ordered component guard, 'CCCL_MAJOR_VERSION > 3 || (CCCL_MAJOR_VERSION == 3 && CCCL_MINOR_VERSION >= 1)' with the intent comment '// strided_iterator was added in CCCL 3.1' — the >major||(==major&&>=minor) ordering survives every rollover by construction; (2) the PACKED form — the commit message's rationale: 'CCCL already exposes the version as a single packed integer in MMMmmmpp form ... CCCL_VERSION = MAJOR * 1000000 + MINOR * 1000 + PATCH so 3.4.3 is 3004003 and \">= 3.1\" is a plain \">= 3001000\". One comparison, with no component arithmetic left to get wrong.' — when the library offers a packed version integer, one compare replaces the component arithmetic entirely. THE VALIDATION TABLE (the hand-checked reference): 'Checked against a hand-written \"version >= 3.1\" reference over 2.9.9, 3.0.0, 3.1.0, 3.1.99, 3.2.0, 3.4.3, 3.9.9, 3.99.99, 4.0.0, 4.2.7 and 5.0.0: no divergences. The old guard disagreed at 4.0.0 and 5.0.0.' — the disagreement points ARE the rollover points, exactly the class the table exists to catch. THE VERIFICATION METHODOLOGY (load-bearing beyond this instance): 'a passing regression test does not on its own prove the guard is still taken, since the fallback path passes too. Preprocessing the real translation unit confirms the strided-iterator branch is the one compiled in: counting_iterator is present, init_offsets is not.' — a GUARD-TAKEN proof (preprocess the unit, grep the fast-path symbol, grep the fallback symbol's ABSENCE) is the only test that separates 'the fast path compiled in' from 'the fallback still passes'. RUST SPELLING: the same hazard in Rust version checks — 'semver.major >= 3 && semver.minor >= 1' rolls over identically; the repairs translate one-to-one ('(major, minor) >= (3, 1)' tuple compare, or 'major > 3 || (major == 3 && minor >= 1)', or a packed u32 'major * 1_000_000 + minor * 1_000 + patch'). REGIME: any compile-time or runtime feature guard keyed to a versioned dependency's components (CCCL/CUDA toolkit versions, backend API versions, semver feature detection); guards whose failure mutes a fast path rather than breaking the build. ANTI-REGIME: guards against a version the project PINNED (the calendar cannot roll while the pin holds — but say the pin out loud, pins lift); equality-only guards ('== 3.1') which fail differently; capability probes that query the FEATURE directly instead of the version (always prefer those when the library offers one). NotBenchable (guard arithmetic — the B186 unified-admission-dead-tail precedent for landing upstream defects with transferable decision content; the evidence class is the rollover table + the guard-taken proof, never a timing figure). KINSHIP (do not double-count): dispatcher-eligibility-predicate-lags-kernel-capability (B103) is the nearest neighbor — that predicate is narrower than the kernel's capability FROM DAY ONE (written in the same commit); THIS guard is CORRECT at authoring time and breaks at a FUTURE version — the time axis is the distinguishing atom; unified-admission-dead-tail (B186) is structurally dead code after a switch (unreachable in every configuration); THIS is live code that silently stops being live — audit any version-keyed guard for both; the B211 superset-admission fall-through law (fwht #29100) composes — a deliberate superset admission is safe when the refusal path falls through to a correct generic kernel, and THIS rule names the OTHER half: when the fallback is slower, the fall-through is still a (quiet) performance loss, so the guard-taken proof is owed regardless. CITE: ggml-org/llama.cpp @ 1167d3f42c2b4dc5469bf4e696cb8c678ca79110 (#29453, MIT, merged 2026-10-08) — ggml/src/ggml-cuda/argsort.cu (the 2-line guard repair + the intent comment; the commit message is the quote authority for the packed form, the validation table, and the guard-taken methodology). Batch 214 / Plan 214.".to_string(),
            ref_shape: "#![allow(dead_code)]\n// naive: two independent component compares -- correct on 3.x, silently\n// false on every future major rollover: on 4.x the guard reads\n// 4 >= 3 && 0 >= 1, i.e. false (the source's own arithmetic). The guarded\n// fast path stops being defined; the build succeeds; argsort runs the slow\n// fallback with no warning anywhere.\n\nfn strided_iterators_available_naive(major: u32, minor: u32) -> bool {\n    major >= 3 && minor >= 1 // breaks at 4.0, 5.0, ...\n}\n\nfn sort_path_naive(major: u32, minor: u32) -> &'static str {\n    if strided_iterators_available_naive(major, minor) {\n        return \"strided_iterator_fast\";\n    }\n    \"init_offsets_slow\" // the quiet perf loss, chosen by accident\n}\n".to_string(),
            opt_pattern: "#![allow(dead_code)]\n// repaired (1): the ORDERED component guard -- survives every rollover by\n// construction. repaired (2): the PACKED single compare. The source's\n// hand-checked validation table says both repairs match a plain\n// version >= 3.1 reference everywhere, and the old guard's disagreement\n// points are exactly the rollover points 4.0.0 and 5.0.0 (it agrees at\n// 4.2.7 only by accident -- minor >= 1 there). And a true predicate is not\n// enough: the fast path must be the one TAKEN (the guard-taken proof -- the\n// fallback path passes too, so only the taken branch separates them).\n\nfn strided_iterators_available(major: u32, minor: u32) -> bool {\n    // strided_iterator was added in 3.1: > major || (== major && >= minor)\n    major > 3 || (major == 3 && minor >= 1)\n}\n\nfn version_packed(major: u32, minor: u32, patch: u32) -> u32 {\n    // CCCL_VERSION = MAJOR * 1000000 + MINOR * 1000 + PATCH: 3.4.3 is\n    // 3004003, so >= 3.1 is a plain >= 3001000 -- one comparison, no\n    // component arithmetic left to get wrong.\n    major * 1_000_000 + minor * 1_000 + patch\n}\n\nfn strided_iterators_available_packed(major: u32, minor: u32, patch: u32) -> bool {\n    version_packed(major, minor, patch) >= 3_001_000\n}\n\nfn strided_iterators_available_naive(major: u32, minor: u32) -> bool {\n    major >= 3 && minor >= 1 // the defect, carried for the disagreement pin\n}\n\nfn sort_path_fast(major: u32, minor: u32) -> &'static str {\n    if strided_iterators_available(major, minor) {\n        return \"strided_iterator_fast\";\n    }\n    \"init_offsets_slow\"\n}\n\nfn sort_path_naive(major: u32, minor: u32) -> &'static str {\n    if strided_iterators_available_naive(major, minor) {\n        return \"strided_iterator_fast\";\n    }\n    \"init_offsets_slow\" // at 4.0.0 the fast path silently vanished\n}\n\n#[test]\nfn the_rollover_table_agrees_with_the_reference_on_both_repairs() {\n    // the source's hand-checked table (the reference: version >= 3.1).\n    let table = [\n        (2, 9, 9, false),\n        (3, 0, 0, false),\n        (3, 1, 0, true),\n        (3, 1, 99, true),\n        (3, 2, 0, true),\n        (3, 4, 3, true),\n        (3, 9, 9, true),\n        (3, 99, 99, true),\n        (4, 0, 0, true),\n        (4, 2, 7, true),\n        (5, 0, 0, true),\n    ];\n    for &(major, minor, patch, want) in &table {\n        // both repairs agree with the reference on EVERY row:\n        assert_eq!(strided_iterators_available(major, minor), want, \"ordered\");\n        assert_eq!(\n            strided_iterators_available_packed(major, minor, patch),\n            want,\n            \"packed\"\n        );\n        // the guard-taken proof: where the reference says available, the\n        // repaired sort path IS the fast one -- never the fallback.\n        assert_eq!(\n            sort_path_fast(major, minor),\n            if want {\n                \"strided_iterator_fast\"\n            } else {\n                \"init_offsets_slow\"\n            }\n        );\n\n        // the naive form: its disagreement set is EXACTLY the rollover rows\n        // (a future major with the minor reset to 0) -- nowhere else.\n        let naive = strided_iterators_available_naive(major, minor);\n        if major > 3 && minor < 1 {\n            assert_ne!(naive, want, \"the naive guard must disagree at rollover\");\n            assert_eq!(\n                sort_path_naive(major, minor),\n                \"init_offsets_slow\",\n                \"the rollover silenced the fast path\"\n            );\n        } else {\n            assert_eq!(naive, want, \"the naive form agrees outside rollovers\");\n        }\n    }\n}\n".to_string(),
        },
    ]
}
