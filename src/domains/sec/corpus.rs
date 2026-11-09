//! The sec seed corpus — Phase 1 (Plan 018, Proposal 004) + Phase 2
//! (Batch 88, Research 104).
//!
//! Six rules survived Phase-1 mine-time falsification (Bench 046's probe
//! evidence table; 5 of 11 candidates killed). Every Phase-1 rule was
//! verified against real clippy 1.93.0 BEFORE authoring; every Phase-2 rule
//! against real clippy 1.97.0 (same protocol: no default or restriction
//! lint fires on the buggy form with an equivalent fix — the T0.4
//! boundary).
//!
//! | Rule | Family | Proposer | Killed neighbours (evidence) |
//! |---|---|---|---|
//! | `ffi-extern-fn-catch-unwind` | Ffi | yes | — (no lint fires) |
//! | `ptr-deref-null-check-first` | `UnsafeHygiene` | yes | — (no lint fires) |
//! | `drop-impl-no-panic` | `DropLeak` | yes | `unwrap_used` fires w/o any fix |
//! | `extern-fn-reference-param-to-pointer` | `Ffi` | no (suggest-only) | `improper_ctypes_definitions` measured SILENT on 1.93 |
//! | `rc-backedge-field-prefer-weak` | `DropLeak` | no (suggest-only) | — (no lint fires) |
//! | `slice-from-raw-parts-len-guard` | `UnsafeHygiene` | yes | — (no lint fires) |
//! | `unsafe-impl-send-sync-unsound-field` | `UnsafeHygiene` | no (suggest-only) | only `undocumented_unsafe_blocks` fires (1.97; wants a comment, not a redesign) |
//! | `ffi-export-panicking-body` | `Ffi` | no (suggest-only) | only `indexing_slicing` fires w/o fix; Phase-1 catch-unwind owns the UNIT-return shape |
//! | `weak-hash-to-blake3` | `CryptoHygiene` | no (suggest-only, dep-gated) | — (no lint fires on 1.97; cargo-audit owns vulnerable CRATES, not misuse) |
//! | `password-hash-argon2` | `CryptoHygiene` | no (suggest-only, dep-gated) | — (silent on 1.97) |
//! | `constant-time-secret-compare` | `CryptoHygiene` | yes (strict-gated) | — (silent on 1.97) |
//! | `nan-unsafe-sort-comparator` | `PanicSafety` | yes (strict-gated, P8) | — (silent on 1.97; probe: zero lints fire on the shape) |
//! | `length-prefix-validate-before-alloc` | `InputValidation` | no (suggest-only) | — (no lint fires on the shape; the fix must INVENT a limit + an error path) |
//! | `secret-file-create-without-0600` | `InputValidation` | no (suggest-only) | — (no lint fires; the fix is cfg(unix)-shaped — a portable rewrite is judgment-bearing) |
//! | `unbounded-read-into-vec` | `InputValidation` | no (suggest-only) | — (no lint fires on the shape; the fix must CHOOSE the limit — a protocol decision) |
//! | `unchecked-wire-arithmetic` | `InputValidation` | no (suggest-only) | `arithmetic_side_effects` SILENT on literal/const operands (`len + 8`, `len + CONST` — measured 1.98 probe, Plan 156 T7 refinement); on variable-operand sites it fires blanket, w/o suggestion or wire-origin scoping — restriction/allow-by-default; `manual_checked_ops` is Div-only at this pin |
//! | `untrusted-path-join-escape` | `InputValidation` | no (suggest-only) | — (no lint fires on the shape; the refusal path's error type is the fn's choice) |
//! | `webhook-signature-over-raw-bytes` | `CryptoHygiene` | no (suggest-only) | — (no lint fires; the coverage question is per-scheme, and canonical-signature schemes are deliberate) |
//! | `unvalidated-egress-url` | `InputValidation` | no (suggest-only) | — (no lint fires; the allowlist is deployment policy, never mechanical) |
//! | `cors-any-origin-with-credentials` | `InputValidation` | no (suggest-only) | — (no lint fires; legitimate origins are deployment policy) |
//! | `jwt-insecure-validation-flags` | `CryptoHygiene` | no (suggest-only) | — (no lint fires; debug/tooling decodes skip validation deliberately — review decides) |
//! | `auth-cookie-without-secure-flags` | `InputValidation` | no (suggest-only) | — (no lint fires; JS-readable/cross-site cookies are deliberate choices) |
//! | `client-ip-from-untrusted-forwarded-header` | `InputValidation` | no (suggest-only) | — (no lint fires; the trusted-proxy set is deployment policy) |
//! | `shell-string-command-interpolation` | `InputValidation` | no (suggest-only) | — (no lint fires; the shell-features regime is a design decision) |
//! | `ipv4-mapped-range-check-unwrap` | `InputValidation` | no (suggest-only) | — (no lint fires; some guards deliberately treat all V6 as external) |
//! | `abuse-strike-counts-cross-site-requests` | `InputValidation` | no (suggest-only) | — (no lint fires; label-honoring policy is deployment-specific) |
//! | `packed-field-silent-overflow-wrong-key` | `InputValidation` | no (suggest-only) | — (no lint fires; `cast_possible_truncation` is blanket `as`-cast pedantic with no key scoping and no fix; the mask site is invisible to `unchecked-wire-arithmetic` — `BitAnd` is not in its operator set) |
//! | `capacity-exceeded-silent-fallback-not-refusal` | `InputValidation` | no (suggest-only) | — (no lint fires; the refusal path's error type is the fn's choice) |
//! | `sigmoid-minus-one` | `Numerics` | yes (strict-gated, Issue 185 T1) | — (valid code; no lint sees a rounding cliff) |
//! | `softmax-onehot-collapse` | `Numerics` | no (suggest-only, Issue 185 T2) | — (valid code; the repair restructures the pass) |
//! | `neg-ln-softmax-prob` | `Numerics` | no (suggest-only, Issue 185 T3) | — (valid code; the row no longer carries the logits) |
//! | `secret-key-without-zeroize` | `CryptoHygiene` | no (suggest-only, dep-gated) | — (the repair is an API decision — a field-type change every constructor site feels; the dep gate keeps the suggestion buildable) |
//! | `untrusted-keyed-state-unbounded` | `InputValidation` | no (suggest-only) | — (the cap + the full-table policy are protocol decisions — the eviction-vs-refusal axis changes what an attacker can do to honest tenants) |
//! | `gateway-type-forces-security-check` | `InputValidation` | no (suggest-only) | — (the ctor flip ripples to every caller — an API-design decision; precision honestly poor, vocabulary-gated, review decides) |
//! | `untrusted-archive-extract-paths` | `InputValidation` | no (suggest-only) | — (no lint fires on the shape; the screen is the deployment's trust decision — which archive sources are legitimate is policy — and the repair is the crate's own screened accessor) |
//! | `browser-reachable-service-host-origin-rebinding-gate` | `InputValidation` | no (suggest-only) | — (no lint fires on the shape; the allowlist + the missing-Origin fail-open stance are deployment policy, and WS handshakes are not CORS-gated so no response-header CORS lint can see the class) |
//! | `custom-header-mutation-gate-unanswered-preflight` | `InputValidation` | no (suggest-only) | — (no lint fires on the shape; the header name is a deployment decision and bearer-auth/CSRF-token/SameSite=Strict APIs deliberately skip the gate — review decides) |
//! | `ascii-token-unicode-word-boundary-scan-miss` | `SensitiveData` | no (suggest-only) | — (no lint fires; the fix choice among the three Rust spellings (`(?-u:\\b)` / fancy-regex lookarounds / flanking-byte validation) is a dependency + correctness decision, and Unicode boundaries are sometimes deliberate — review decides) |
//! | `luhn-validate-numeric-pii-candidates-before-redact` | `SensitiveData` | no (suggest-only) | — (no lint fires; the validator scheme + the rescan policy are the detector's design decisions, and a compliance-first over-redaction stance deliberately rejects the rule) |
//! | `redaction-audit-hit-carries-raw-match` | `SensitiveData` | no (suggest-only) | — (no lint fires; whether previews ship at all and the tier boundaries are compliance decisions, not physics) |
//! | `externally-authored-edits-by-id-not-counted-index` | `InputValidation` | no (suggest-only) | — (no lint fires on the shape; the addressing scheme is the API designer's choice, and a detector would be vocabulary-gated hard — machine-internal indexing is everywhere) |
//!
//! Phase-1 killed candidates (recorded in the batch research note):
//! `ffi-fnptr-type-unsafe-extern` + `ffi-by-value-no-drop-type`
//! (`improper_ctypes` fires WITH a fix), `ffi-portable-c-alias` (the alias
//! map is platform-dependent — not parse-only-provable),
//! `no-box-leak` + `maybe-uninit-requires-justification` (no bounded fix;
//! clippy owns the direct-chain uninit form at error level).
//!
//! Batch 131 (2026-09-08): the `InputValidation` family opens with two
//! SUGGEST-ONLY rules (dhishwasher/Girder distill, BSL 1.1 — ORIGINAL
//! fixtures below, shapes cited at the pin in `.research/153`). Both ship
//! parse-only DETECTORS with zero-FP negative suites
//! (`proposers::length_prefix_alloc` / `proposers::secret_file_create`);
//! `collect_edits` arms and `FIXABLE_SEC` membership are deliberately
//! absent — these rules emit suggestions, never auto-fixes.
//!
//! Batch 140 (2026-09-09): the rustsec/advisory-db distill (CC0; Research
//! 161 / Plan 120) adds the family's third suggest-only rule — the
//! unbounded stream-read class (RUSTSEC-2024-0003 / CVE-2019-9514 anchor),
//! detector in `proposers::unbounded_read`. The DB is cited as CLASS
//! provenance only; the fixtures are ORIGINAL syntheses.
//!
//! Batch 165 (2026-09-20, Plan 156 / Research 193): the family's fourth
//! suggest-only rule — unchecked arithmetic on wire-byte origins (the
//! overflow sink). The taint walk is SHARED with
//! `length-prefix-validate-before-alloc` (`proposers::wire_taint`, one
//! walker two sinks — Plan 156 T2's sink-distinct adjudication: distinct
//! defect classes, distinct repairs, and the derived-validated shape's
//! overflow already happened at the op, so a composition gate would
//! suppress a true finding). Detector in
//! `proposers::unchecked_wire_arithmetic`; probe evidence (real clippy
//! 0.1.98, the pinned 1.98.1) recorded in Plan 156 T1.
//!
//! Batch 258 (2026-10-20, Plan 262 / Research 292): the web-trust family
//! seed from the cloudflare/security-audit-skill distill (@ `c1c8a8c`,
//! MIT) — THREE suggest-only rules across two existing families
//! (adjudicated at authoring per the plan; no new family): path-traversal
//! (`untrusted-path-join-escape`, `InputValidation` — the `components()`
//! refusal, the riir-deployer `48d63f7` in-house shape), webhook
//! signature-coverage (`webhook-signature-over-raw-bytes`,
//! `CryptoHygiene` — distinct-from the compare primitive on the coverage
//! axis), and egress selection (`unvalidated-egress-url`,
//! `InputValidation` — sink-distinct from unbounded-read's read-source
//! face, the Plan 156 T2 shape). PLUS the count-neutral rider on
//! `unbounded-read-into-vec`: decompression decoders
//! (`GzDecoder`/`ZlibDecoder`/`BzDecoder`/`XzDecoder`/`ZstdDecoder`) are
//! transparent to the taint walk (`take` stays a taint break) and the
//! concept gains the bound-after-every-expansion-stage face. The
//! `poisoned-lock-fail-open` candidate DROPPED at authoring per the plan
//! (thin decision content — the guard is one method call). Detector
//! modules: `proposers::path_join_escape`,
//! `proposers::webhook_raw_bytes`, `proposers::egress_url`; the probe
//! evidence (real clippy, pedantic+nursery+suspicious+correctness, zero
//! lints on the three buggy fixtures) recorded in Plan 262 T7.
//!
//! Batch 291 (2026-10-26, Plan 295 / Research 329): the danoszz/penthera
//! distill (MIT @ `07421f9` — an offline rule-based security agent,
//! Node/TS; the corrected-transcription first: the source contributes
//! CLASS knowledge, the fixtures are OUR original Rust). THREE suggest-only
//! rules across the two existing families: wildcard-CORS + credentials
//! (`cors-any-origin-with-credentials`, `InputValidation` — `very_permissive()`
//! fires alone; `any()`/`permissive()` need the `allow_credentials(true)`
//! marker), JWT decode configured to not verify
//! (`jwt-insecure-validation-flags`, `CryptoHygiene` — the 9.x
//! `insecure_disable_signature_validation` surface + its 10.x
//! `dangerous::insecure_decode` replacement + the `validate_exp = false`
//! skip, jwt-vocabulary-gated), and auth-named cookies without the
//! Secure/HttpOnly hardening (`auth-cookie-without-secure-flags`,
//! `InputValidation` — the name-vocabulary gate, the secret-file-0600
//! fn-wide-suppression shape). Detector modules:
//! `proposers::cors_any_creds`, `proposers::jwt_insecure_validation`,
//! `proposers::auth_cookie_missing_flags`; probe evidence (the boundary
//! gate protocol — the probe crate pins jsonwebtoken 9 so the 9.x fixture
//! compiles) recorded in Plan 295. The two defers (the secrets.json AI-key
//! vocabulary probe for the secret-scan lane; an agent-tooling audit phase
//! for the katgpt-rs security-audit skill) stay named in Research 329,
//! unfiled.
//!
//! Batch 292 (2026-10-27, Plan 296 / Research 330): the hevnsnt/CVEngine
//! distill (GPL-3.0 @ `9dd9c441` — a hardened attack-surface scanner;
//! pure-Python source, the transcription law applies: language is never a
//! gate). FOUR suggest-only rules, all `InputValidation`, on the two most
//! universal std surfaces: spoofed client identity
//! (`client-ip-from-untrusted-forwarded-header` — the header literal +
//! identity/throttle vocabulary, the trusted marker suppresses fn-wide),
//! shell-string interpolation (`shell-string-command-interpolation` — the
//! `sh -c`/`cmd /C` literal + a format!/concat arg; the metacharacter
//! allowlist ≠ safety, CWE-88), the mapped-address bypass
//! (`ipv4-mapped-range-check-unwrap` — the V4/V6 match pair without
//! `to_ipv4_mapped`, range-test vocabulary gated), and the strike-count
//! asymmetry (`abuse-strike-counts-cross-site-requests` — a strike
//! increment beside a rejection with no site-label vocabulary; the
//! honest-drop path recorded in the plan never fired — the fixture
//! carried a zero-FP negative suite). PLUS the count-neutral USE-TIME
//! rider on `unvalidated-egress-url` (stored names rot; the sink
//! re-filters; narrows the rebinding window, never closes it). Detector
//! modules: `proposers::client_ip_forwarded`,
//! `proposers::shell_string_command`, `proposers::ipv4_mapped_range`,
//! `proposers::strike_cross_site`; probe evidence (the boundary-gate
//! protocol — std-only fixtures, no probe deps added) recorded in Plan
//! 296 T7.
//!
//! Batch 294 (2026-10-27, Plan 298 / Research 332): the wasmtime
//! RUSTSEC-2026-0380/0379 distill (CC0 — the two CVSS-9.9 sandbox-escape
//! advisories; the JIT-internal failures generalized to their Rust-shaped
//! userland classes). TWO suggest-only rules, all `InputValidation`:
//! the masked-key packing site (`packed-field-silent-overflow-wrong-key`
//! — a count/ID masked to a 2^k−1 literal then used as a lookup key
//! `packed-field-silent-overflow-wrong-key` — a count/ID masked to a 2^k−1 literal then used as a lookup key
//! silently loses the high bits; CVE-2026-115889's 24-bit packed
//! value-number class; the B258 recorded drop adjudicated in-rule — the
//! mask site is invisible to `unchecked-wire-arithmetic` (`BitAnd` is not
//! in its operator set) and `cast_possible_truncation` is blanket
//! `as`-cast pedantic with no key scoping) and the silent-cap degrade
//! (`capacity-exceeded-silent-fallback-not-refusal` — the cap-exceeded
//! branch truncates and falls through; CVE-2026-115893's `br_table` class,
//! the >65,520-targets REFUSAL half is the lesson; distinct-from
//! `length-prefix-validate-before-alloc` on the missing-check vs
//! lying-cap axis). Detector modules:
//! `proposers::packed_field_wrong_key`,
//! `proposers::capacity_silent_truncate`; probe evidence (the
//! boundary-gate protocol — std-only fixtures, no probe deps added)
//! recorded in Plan 298 T1.
//!
//! Batch 325 (2026-11-02, Plan 327 / Research 361): the mozilla/uniffi-rs
//! FFI-hygiene distill (MPL-2.0 @ `ef9625b68dd51207f55f024a27649e58edb52b09`
//! — cited as class provenance; the fixtures are ORIGINAL syntheses, never
//! uniffi source). FOUR suggest-only rules — the sec FFI-hygiene family
//! (panic containment, ABI hygiene, custody) extended onto the
//! pyo3/cxx/uniffi-class mixed-language surface. ALL NotBenchable-by-domain
//! per the dial9 split (correctness/liveness wins at the boundary, never
//! latency): raw-`Arc` custody across an unsafe boundary
//! (`arc-refcount-across-raw-boundary`, `DropLeak` — the A− lead; the
//! borrowed/consume two-regime law, the integer-exact refcount ledger the
//! test oracle), the callback invoked under a live mutex guard
//! (`ffi-callback-invoke-outside-lock`, `Ffi` — the cross-FFI ABBA
//! deadlock class; ADJACENT-DISTINCT from the `entries_08`
//! `host-lock-across-reentrant-blocking-call` same-lock regime, the
//! in-rule adjudication note mandatory), the caller-initialized repr(C)
//! out-param taken as `&mut MaybeUninit` where `&mut ManuallyDrop` is the
//! honest contract (`out-param-manuallydrop-not-maybeuninit`, `Ffi` — the
//! set-twice = leak single-writer contract), and the error-REPORTING
//! handler doing fallible work without its own `catch_unwind`
//! (`panic-handler-nested-catch-unwind`, `PanicSafety` — the handler path
//! was unowned; the extern body wrapper + Drop bodies keep their own
//!Detector modules: `proposers::arc_raw_boundary`,
//! `proposers::callback_lock_invoke`, `proposers::outparam_init_contract`,
//! `proposers::panic_handler_guard`. The two stretch candidates are HONEST
//! NON-CONVERSIONS at this batch: #5 `null-sentinel-companion-conjunction`
//! (B−) held at its own conjunction-or-demote bar (the conjunction fixture
//! set needs the authoring budget the four mains consumed — a named lead
//! for a future FFI batch); #6 `vec-promote-manuallydrop-not-forget` (C+)
//! thin + perf-neutral (borrowck-prose novelty).
//!
//! Batch 327 (2026-11-03, Plan 332 / Research 364): the firezone/sanstun
//! distill (Apache-2.0 @ `35cb1932948bd0776` — a `sans-IO` `WireGuard`
//! protocol implementation; cited as class provenance; the fixtures are
//! ORIGINAL syntheses, never pasted source). THREE suggest-only rules
//! across the two families: secret lifetime
//! (`secret-key-without-zeroize`, `CryptoHygiene` — the dep-gated
//! `constant-time-secret-compare` shape: a secret-vocabulary struct
//! holding raw key material with no Drop impl and no zeroization wrapper
//! keeps the bytes alive in freed memory; keys.rs's pervasive
//! `Zeroizing<[u8; 32]>` wrap and its `zeroized on drop` docs are the
//! anchor), the resource-exhaustion CARDINALITY face
//! (`untrusted-keyed-state-unbounded`, `InputValidation` — a peer-keyed
//! `HashMap` growing via insert with no cap guard; the fix is the source's
//! own bounded table + fail-closed refusal of NEW keys when full —
//! eviction would reward the attacker over honest entries;
//! ratelimiter.rs's `MAX_SOURCES` guard and its
//! `full_table_refuses_new_sources_until_idle_ones_are_dropped` test are
//! the anchor), and checkpoint bypass via constructor reachability
//! (`gateway-type-forces-security-check`, `InputValidation` — a pub ctor
//! on a message-vocabulary struct beside a same-file gateway fn whose
//! checks the ctor skips; message.rs's crate-private ctors +
//! `Interface::receive` as the single parse dispatch are the anchor;
//! honest poor precision — vocabulary-gated, same-file only, review
//! decides). Detector modules: `proposers::zeroize_secret`,
//! `proposers::keyed_state_cap`, `proposers::gateway_ctor`.

use super::{SecFamily, SecRule};
use crate::corpus::Corpus;
use crate::domains::sec::SecDomain;

/// Build the sec seed corpus (BLAKE3-committed via `Corpus<D>`).
#[must_use]
pub fn build_sec_seed_corpus() -> Corpus<SecDomain> {
    Corpus::new(entries())
}

fn entries() -> Vec<SecRule> {
    vec![
        // ── ffi-extern-fn-catch-unwind (Ffi; proposer: unit-return fns) ──
        SecRule {
            rule_id: "ffi-extern-fn-catch-unwind".to_string(),
            sec_concept: "a panic unwinding across an extern \"C\" boundary is UB — \
contain it with catch_unwind (the in-house #[riir_ffi] rule, generalized)"
                .to_string(),
            buggy_code: "#[no_mangle]\npub extern \"C\" fn on_packet(n: u32) {\n    let v = vec![n; 4];\n    println!(\"{v:?}\");\n}\n".to_string(),
            fixed_code: "#[no_mangle]\npub extern \"C\" fn on_packet(n: u32) {\n    let _ = std::panic::catch_unwind(|| {\n        let v = vec![n; 4];\n        println!(\"{v:?}\");\n    });\n}\n".to_string(),
            family: SecFamily::Ffi,
        },
        // ── ptr-deref-null-check-first (UnsafeHygiene; proposer: Option fns) ──
        SecRule {
            rule_id: "ptr-deref-null-check-first".to_string(),
            sec_concept: "a raw-pointer deref without a null check turns a null caller \
into UB — check first (as_ref + ? in Option-returning fns)"
                .to_string(),
            buggy_code: "pub struct Header { pub len: u32 }\n\npub unsafe fn read_len(p: *const Header) -> Option<u32> {\n    Some((*p).len)\n}\n".to_string(),
            fixed_code: "pub struct Header { pub len: u32 }\n\npub unsafe fn read_len(p: *const Header) -> Option<u32> {\n    let Some(p) = (unsafe { p.as_ref() }) else { return None };\n    Some(p.len)\n}\n".to_string(),
            family: SecFamily::UnsafeHygiene,
        },
        // ── drop-impl-no-panic (DropLeak; proposer: unwrap/expect → default) ──
        SecRule {
            rule_id: "drop-impl-no-panic".to_string(),
            sec_concept: "a panic inside drop glue during unwind is a double-panic \
abort — drop must degrade, never unwrap"
                .to_string(),
            buggy_code: "pub struct Conn { pub handle: Option<u32> }\n\nimpl Drop for Conn {\n    fn drop(&mut self) {\n        let h = self.handle.unwrap();\n        println!(\"close {h}\");\n    }\n}\n".to_string(),
            fixed_code: "pub struct Conn { pub handle: Option<u32> }\n\nimpl Drop for Conn {\n    fn drop(&mut self) {\n        let h = self.handle.unwrap_or_default();\n        println!(\"close {h}\");\n    }\n}\n".to_string(),
            family: SecFamily::DropLeak,
        },
        // ── extern-fn-reference-param-to-pointer (Ffi; suggest-only) ──
        SecRule {
            rule_id: "extern-fn-reference-param-to-pointer".to_string(),
            sec_concept: "a reference param in an extern \"C\" definition cannot be \
null — a C caller passing NULL is instant UB with no unsafe marker on our side; \
take *const/*mut and deref explicitly"
                .to_string(),
            buggy_code: "#[no_mangle]\npub extern \"C\" fn sum_rows(rows: &Vec<u32>) -> u32 {\n    rows.iter().sum()\n}\n".to_string(),
            fixed_code: "#[no_mangle]\npub extern \"C\" fn sum_rows(rows: *const Vec<u32>) -> u32 {\n    let rows: &Vec<u32> = unsafe { &*rows };\n    rows.iter().sum()\n}\n".to_string(),
            family: SecFamily::Ffi,
        },
        // ── rc-backedge-field-prefer-weak (DropLeak; suggest-only) ──
        SecRule {
            rule_id: "rc-backedge-field-prefer-weak".to_string(),
            sec_concept: "a strong-Rc field pointing back at the same type (the \
parent/backedge shape) holds the cycle alive forever — prefer Weak for the \
backedge (parse-only scope: the direct Rc<Self> field, NOT provable cycles)"
                .to_string(),
            buggy_code: "use std::rc::Rc;\n\npub struct Tree {\n    pub value: u32,\n    pub children: Vec<Rc<Tree>>,\n    pub parent: Rc<Tree>,\n}\n".to_string(),
            fixed_code: "use std::rc::{Rc, Weak};\n\npub struct Tree {\n    pub value: u32,\n    pub children: Vec<Rc<Tree>>,\n    pub parent: Weak<Tree>,\n}\n".to_string(),
            family: SecFamily::DropLeak,
        },
        // ── slice-from-raw-parts-len-guard (UnsafeHygiene; proposer) ──
        SecRule {
            rule_id: "slice-from-raw-parts-len-guard".to_string(),
            sec_concept: "slice::from_raw_parts requires ptr non-null EVEN WHEN \
len == 0 (the documented safety contract) — guard the empty case before \
constructing the slice"
                .to_string(),
            buggy_code: "pub unsafe fn as_bytes(buf: *const u8, len: usize) -> &'static [u8] {\n    std::slice::from_raw_parts(buf, len)\n}\n".to_string(),
            fixed_code: "pub unsafe fn as_bytes(buf: *const u8, len: usize) -> &'static [u8] {\n    if len == 0 {\n        &[]\n    } else {\n        std::slice::from_raw_parts(buf, len)\n    }\n}\n".to_string(),
            family: SecFamily::UnsafeHygiene,
        },
        // ══ Phase 2 (Batch 88) — probe evidence: real clippy 1.97.0, full ══
        // ══ groups (pedantic + restriction + nursery), all 5 shapes OPEN. ══
        // ── unsafe-impl-send-sync-unsound-field (UnsafeHygiene; suggest-only) ──
        SecRule {
            rule_id: "unsafe-impl-send-sync-unsound-field".to_string(),
            sec_concept: "an `unsafe impl Sync` (or `Send`) for a local type whose fields \
contain `Rc`/`RefCell`/`Cell` grants thread-safety the field types deny — the \
Rudra unsound-impl class clippy cannot see; the fix is a type-level redesign \
(Arc/Mutex swap), never a mechanical rewrite, so this rule is suggest-only"
                .to_string(),
            buggy_code: "use std::cell::RefCell;\nuse std::rc::Rc;\n\npub struct CacheInner {\n    pub map: RefCell<Rc<str>>,\n}\nunsafe impl Sync for CacheInner {}\n".to_string(),
            fixed_code: "use std::rc::Rc;\nuse std::sync::{Arc, Mutex};\n\npub struct CacheInner {\n    pub map: Mutex<Rc<str>>,\n}\nunsafe impl Sync for CacheInner {}\n".to_string(),
            family: SecFamily::UnsafeHygiene,
        },
        // ── ffi-export-panicking-body (Ffi; suggest-only, VALUE-return) ──
        SecRule {
            rule_id: "ffi-export-panicking-body".to_string(),
            sec_concept: "a panic (unwrap/expect/panic!/indexing/assert) inside an \
extern \"C\" fn unwinds across the FFI boundary — UB (the Phase-1 \
catch-unwind rule owns the UNIT-return shape with a wrap fix; the \
VALUE-return shape needs a caller-chosen sentinel, so it is suggest-only)"
                .to_string(),
            buggy_code: "#[no_mangle]\npub extern \"C\" fn step(cfg: u32) -> u32 {\n    let v: Vec<u32> = vec![cfg, 1, 2];\n    v[0]\n}\n".to_string(),
            fixed_code: "#[no_mangle]\npub extern \"C\" fn step(cfg: u32) -> u32 {\n    let v: Vec<u32> = vec![cfg, 1, 2];\n    v.first().copied().unwrap_or(0)\n}\n".to_string(),
            family: SecFamily::Ffi,
        },
        // ── weak-hash-to-blake3 (CryptoHygiene; suggest-only, dep-gated) ──
        SecRule {
            rule_id: "weak-hash-to-blake3".to_string(),
            sec_concept: "MD5/SHA1 are cryptographically BROKEN (chosen-prefix \
collisions) and SHA256 is the slower house loser — prefer blake3 for new \
digests (the house rule; dep-gated: fires only when the file already \
imports blake3, so the swap cannot break the build)"
                .to_string(),
            buggy_code: "pub fn legacy_id(data: &[u8]) -> [u8; 20] {\n    use sha1::{Digest, Sha1};\n    let out: [u8; 20] = Sha1::digest(data).into();\n    out\n}\n".to_string(),
            fixed_code: "pub fn legacy_id(data: &[u8]) -> blake3::Hash {\n    blake3::hash(data)\n}\n".to_string(),
            family: SecFamily::CryptoHygiene,
        },
        // ── password-hash-argon2 (CryptoHygiene; suggest-only, dep-gated) ──
        SecRule {
            rule_id: "password-hash-argon2".to_string(),
            sec_concept: "a password hash must be a MEMORY-HARD KDF (argon2) — a bare \
Sha256/Sha1/Md5 digest of the password is brute-forceable at GPU speeds \
(the house rule; dep-gated on an argon2 import; trigger scope: fns whose \
name contains \"password\" calling a bare digest — name-based, so \
false-NEGATIVES are honest and expected)"
                .to_string(),
            buggy_code: "pub fn hash_password(password: &str) -> [u8; 32] {\n    use sha2::{Digest, Sha256};\n    let out: [u8; 32] = Sha256::digest(password.as_bytes()).into();\n    out\n}\n".to_string(),
            fixed_code: "pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {\n    use argon2::{Argon2, PasswordHasher};\n    use argon2::password_hash::{SaltString, rand_core::OsRng};\n    let salt = SaltString::generate(&mut OsRng);\n    Argon2::default()\n        .hash_password(password.as_bytes(), &salt)\n        .map(|h| h.to_string())\n}\n".to_string(),
            family: SecFamily::CryptoHygiene,
        },
        // ── constant-time-secret-compare (CryptoHygiene; strict-gated proposer) ──
        SecRule {
            rule_id: "constant-time-secret-compare".to_string(),
            sec_concept: "comparing a secret (token/mac/signature/key) with `==` leaks \
the match prefix through timing — compare in constant time via \
`subtle::ConstantTimeEq` (dep-gated: fires only when the file already \
imports subtle, so the swap cannot break the build; trigger scope: the \
enclosing fn's name OR an operand name hits the secret vocabulary, and \
both operands are plain identifiers)"
                .to_string(),
            buggy_code: "use subtle::ConstantTimeEq;\n\npub fn check_token(provided: &[u8], expected: &[u8]) -> bool {\n    provided == expected\n}\n".to_string(),
            fixed_code: "use subtle::ConstantTimeEq;\n\npub fn check_token(provided: &[u8], expected: &[u8]) -> bool {\n    provided.ct_eq(expected).into()\n}\n".to_string(),
            family: SecFamily::CryptoHygiene,
        },
        // ── nan-unsafe-sort-comparator (PanicSafety; P8 intake, strict-gated proposer) ──
        SecRule {
            rule_id: "nan-unsafe-sort-comparator".to_string(),
            sec_concept: "a sort comparator built from `partial_cmp(..).unwrap_or(Equal)` treats \
NaN as equal to EVERYTHING — an inconsistent (non-total) order the std sorts \
MAY PANIC on since 1.81, and silently mis-order NaN otherwise — use \
`total_cmp` (name-identical on f32/f64, direction-preserving, NaN-deterministic); \
probe 2026-09-01: zero clippy lints fire on the shape (real clippy 1.97, \
pedantic+nursery+perf+suspicious+correctness+style)"
                .to_string(),
            buggy_code: "pub fn sort_scores(v: &mut [f32]) {\n    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));\n}\n".to_string(),
            fixed_code: "pub fn sort_scores(v: &mut [f32]) {\n    v.sort_by(|a, b| a.total_cmp(b));\n}\n".to_string(),
            family: SecFamily::PanicSafety,
        },
        // ══ Batch 131 (Girder distill) — the InputValidation family opens. ══
        // ══ Both suggest-only: detectors ship, collect_edits arms do not. ══
        // ── length-prefix-validate-before-alloc (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "length-prefix-validate-before-alloc".to_string(),
            sec_concept: "one packet controls the allocation: a length parsed from a PEER \
(u32::from_be_bytes on a wire header, or a Content-Length-style parse::<usize>()) \
flowing into vec![0u8; len] with NO intervening `0 < len <= limit` check lets a \
4-byte header demand a multi-GB buffer — allocation-size DoS (OOM/abort); the \
cost axis is SECURITY, not cycles. Validate BEFORE the alloc: \
`if len == 0 || len > MAX { return Err(..) }`. The class recurs even in careful \
code — the evidence is BOTH WAYS in one codebase (an unbounded transport beside \
its own bounded twin, same author). Suggest-only: the fix must invent a limit \
constant and an error-return path the fn may not have. Honest recall limits: \
cross-fn length origins (helper parses, caller allocs) undetected; bare-ident \
lengths only (field/method origins out); vec!-repeat form only (`with_capacity` \
from trusted metadata is out of scope by design); the shape proves a MISSING \
VALIDATION, not an untrusted source — sites parsing trusted lengths fire and \
review decides. Batch 131 / Research 153 / Plan 112."
                .to_string(),
            buggy_code: "pub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    let mut prefix = [0_u8; 4];\n    std::io::Read::read_exact(stream, &mut prefix)?;\n    let len = u32::from_be_bytes(prefix) as usize;\n    let mut body = vec![0_u8; len];\n    std::io::Read::read_exact(stream, &mut body)?;\n    Ok(body)\n}\n"
                .to_string(),
            fixed_code: "pub const MAX_FRAME_BYTES: usize = 1 << 20;\n\npub fn read_frame(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    let mut prefix = [0_u8; 4];\n    std::io::Read::read_exact(stream, &mut prefix)?;\n    let len = u32::from_be_bytes(prefix) as usize;\n    if len == 0 || len > MAX_FRAME_BYTES {\n        return Err(std::io::Error::new(\n            std::io::ErrorKind::InvalidData,\n            \"frame length out of range\",\n        ));\n    }\n    let mut body = vec![0_u8; len];\n    std::io::Read::read_exact(stream, &mut body)?;\n    Ok(body)\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── secret-file-create-without-0600 (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "secret-file-create-without-0600".to_string(),
            sec_concept: "a credential-named file (secret/password/passwd/credential/\
token/hmac vocabulary, word-boundary) created world-readable: `File::create` \
(and `fs::write`, and an OpenOptions create/create_new WITHOUT `.mode(..)`) \
makes the file 0644-by-umask, readable by every local user — harden at create \
time with `#[cfg(unix)] OpenOptions::mode(0o600)`, or portably AFTER create \
with `#[cfg(unix)] set_permissions(Permissions::from_mode(0o600))` (the house \
`kat_account::write_private` form). Suggest-only: a portable mechanical fix is \
not parse-only-provable (Windows `from_mode` maps coarsely; hardening semantics \
are per-OS). Honest recall limits: the vocabulary is deliberately NARROW — \
`key`/`identity`/`private`/`pem` are excluded as ambiguous (public keys, \
identity metadata, public certs), so `api_key`/`private_key` writes are missed; \
any `mode`/`set_permissions`/`umask` marker anywhere in the fn suppresses it \
(one hardened file hides a sibling hazard — conservative). Batch 131 / \
Research 153 / Plan 112."
                .to_string(),
            buggy_code: "pub fn save_session_token(path: &std::path::Path, token: &str) -> std::io::Result<()> {\n    let mut f = std::fs::File::create(path)?;\n    use std::io::Write;\n    f.write_all(token.as_bytes())\n}\n"
                .to_string(),
            fixed_code: "pub fn save_session_token(path: &std::path::Path, token: &str) -> std::io::Result<()> {\n    let mut f = std::fs::File::create(path)?;\n    #[cfg(unix)]\n    {\n        use std::os::unix::fs::PermissionsExt;\n        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;\n    }\n    use std::io::Write;\n    f.write_all(token.as_bytes())\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── Batch 140 (advisory-db distill) — the InputValidation family's ══
        // ── third suggest-only rule (detector, no collect_edits arm).      ══
        // ── unbounded-read-into-vec (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "unbounded-read-into-vec".to_string(),
            sec_concept: "an unbounded stream read into a growable Vec from a network reader \
(read_to_end / read_to_string / a loop { read?; extend } drain, no Read::take cap) lets one peer \
control the allocation: the buffer grows without limit on peer-controlled bytes — the resource-\
exhaustion class RUSTSEC-2024-0003 / CVE-2019-9514 (h2) anchors in the wild. The cost axis is \
SECURITY, not cycles. Bound it: `Read::take(reader, MAX_BYTES)` + `Vec::with_capacity(..)`. \
Suggest-only: choosing the limit is a protocol decision, never mechanical. Honest recall limits: \
the untrusted-source proof is parse-provable vocabulary only (TcpStream-typed bindings, \
listener accept()/incoming() children, connect()/reqwest::get() results — plus the \
Batch-258 decoder see-through: `GzDecoder`/`ZlibDecoder`/`BzDecoder`/`XzDecoder`/`ZstdDecoder` \
wrappers are TRANSPARENT, the taint flows through, while `take` stays a taint break) — a bare `R: Read` \
generic is an honest NO; File readers are the trusted regime and never fire; cross-fn drains \
undetected. A decoder between the reader and the drain does NOT excuse the cap — \
`Read::take` belongs on the DECODER (the decompressed side): a compressed body can expand \
orders of magnitude past any transfer-size check, so verify limits after EVERY expansion stage \
(the Batch-258 rider). Second-expansion corroboration (Batch 328, google/oss-fuzz @ \
`a380569f`): a download-then-extract pipeline makes the archive an untrusted read \
source AND the extraction a second unbounded expansion (the zip-bomb adjacency — \
the source's own `http_utils.py::download_and_unpack_zip` is the real regime \
instance: download to tmp then `extractall`, no member screen). Corroboration \
only — no new trigger. Batch 140 / Research 161 / Plan 120 (rustsec/advisory-db @ b50980aa, CC0 — cited \
as class provenance; fixtures are original syntheses)."
                .to_string(),
            buggy_code: "pub fn read_request(mut stream: std::net::TcpStream) -> std::io::Result<Vec<u8>> {\n    let mut buf = Vec::new();\n    std::io::Read::read_to_end(&mut stream, &mut buf)?;\n    Ok(buf)\n}\n"
                .to_string(),
            fixed_code: "pub fn read_request(stream: std::net::TcpStream, max_bytes: u64) -> std::io::Result<Vec<u8>> {\n    let mut limited = std::io::Read::take(stream, max_bytes);\n    let mut buf = Vec::with_capacity(max_bytes.min(1 << 20) as usize);\n    std::io::Read::read_to_end(&mut limited, &mut buf)?;\n    Ok(buf)\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── Batch 165 (wire-arithmetic overflow sink) — the InputValidation ══
        // ── family's fourth suggest-only rule (shared wire_taint walker).  ══
        // ── unchecked-wire-arithmetic (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "unchecked-wire-arithmetic".to_string(),
            sec_concept: "a wire-byte-origin integer (u32::from_be_bytes on a header, or a \
.parse() of a length field) flowing through a BARE arithmetic operator (+ - * << >> \
and their compound-assign spellings) with NO intervening bound check is an unchecked \
overflow — panic in debug, silent WRAP in release, and a wrapped length silently \
corrupts the frame parse downstream (the classic \"check the multiplication, not just \
the result\" class: a post-hoc range check validates the wrapped value). Use the \
checked op + an error path: len.checked_add(FRAME_OVERHEAD).ok_or(..)?. Suggest-only: \
the invented error path's type the fn's signature may not carry — a protocol decision. \
Sink-level novelty over length-prefix-validate-before-alloc on the SHARED wire_taint \
walker (one walker, two sinks; Plan 156 T2 sink-distinct adjudication — the alloc \\
sink owns allocation-size DoS, this sink owns the overflow itself, and in the \\
derived-validated shape the alloc rule is correctly silent while this one correctly \\
still fires). Boundary (measured, Plan 156 T1 + T7 refinement): clippy's \\
arithmetic_side_effects is SILENT on literal/const operands (`len + 8`, \\
`len + FRAME_OVERHEAD`) and where it does fire (variable operands) it is blanket — \
every `x * y` site, no suggestion, no wire-origin scoping, restriction/allow-by-default; \
manual_checked_ops is Div-only at this pin — Div/Rem are NOT this sink. Honest recall limits: same-fn \
bare-ident origins only (cross-fn/field origins out); vec![0u8; len + 8] (arithmetic \
INLINE in the repeat) is unreached — the shared walk never descends into macro \
tokens. Batch 165 / Research 193 / Plan 156 (probe: real clippy 0.1.98 at the \
pinned 1.98.1, zero safety lints on the shape)."
                .to_string(),
            buggy_code: "pub const FRAME_OVERHEAD: usize = 8;\n\npub fn framed_payload_len(header: [u8; 4]) -> usize {\n    let len = u32::from_be_bytes(header) as usize;\n    len + FRAME_OVERHEAD\n}\n"
                .to_string(),
            fixed_code: "pub const FRAME_OVERHEAD: usize = 8;\n\npub fn framed_payload_len(header: [u8; 4]) -> Option<usize> {\n    let len = u32::from_be_bytes(header) as usize;\n    len.checked_add(FRAME_OVERHEAD)\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Batch 258 (security-audit-skill distill) — the web-trust ══
        // ══ seed: three suggest-only rules across two families.      ══
        // ── untrusted-path-join-escape (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "untrusted-path-join-escape".to_string(),
            sec_concept: "a caller-supplied name joined onto a trusted base \
(`base.join(name)`, name a fn parameter of the str/Path/OsStr family) and the \
joined path sunk into `fs::*` / `File::{open,create}` with NO component check \
in the fn lets `..`, absolute components, or Windows prefixes ESCAPE the base \
directory — read any file the service's user can read, write the export \
outside the export tree (the path-traversal class). The bounded repair is \
the components() walk refusing everything but Normal: \
`if joined.components().any(|c| !matches!(c, Component::Normal(_))) { \
return Err(..) }` — the in-house riir-deployer `48d63f7` files-row fix \
shape, cited in-rule. Suggest-only: the refusal path's error type is the \
fn's choice, never mechanical. Honest recall limits: the shape proves a \
MISSING GUARD, not an attacker — a helper joining two internal parameters \
fires too and review decides; lexical `starts_with`/`strip_prefix` do NOT \
suppress (a joined `..` survives both lexical checks — firing there is a \
TRUE finding, the rule's subtle half); `components()`/`canonicalize`/the \
`Component::{ParentDir,RootDir,CurDir,Prefix}` spellings anywhere in the fn \
suppress (one hardened fn hides a sibling — conservative); literal/const \
join args never fire (trusted by construction); field origins \
(`entry.file_name()` — the read_dir iteration idiom) are an honest NO; \
cross-fn joins undetected. Fix-form rider (Batch 328, google/oss-fuzz @ \
`a380569f`): when the untrusted input is a single file NAME (basename only), \
extracting the basename before the join IS the repair — the source's own \
`fuzz_target.py::_save_crash` copies via `os.path.basename(crash.input_path)` \
before joining the artifact directory, the harness's own placement hygiene. \
Listing-time rider (Batch 329, tembo/voxl @ `eba3b7ce`, MIT): the join-time \
guard is not the whole surface — an ENUMERATION (a directory listing served \
to clients) must re-check each entry's RESOLVED target against the root \
before trusting its metadata, because a symlink INSIDE the base escapes \
through its canonical target — the source's `files.rs::list` follows \
symlinks for type/size (\"as a file manager would\") but HIDES links whose \
`canonicalize` escapes the root (files.rs L94-105: `t.is_symlink() && \
!canonicalize(e.path()).starts_with(&f.root) → skip`), while its `resolve()` \
owns the join half — two sinks, one boundary. \
Batch 258 / Plan 262 / Research 292 \
(cloudflare/security-audit-skill @ `c1c8a8c`, MIT — cited as class \
provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "pub fn read_asset(base: &std::path::Path, name: &str) -> std::io::Result<String> {\n    std::fs::read_to_string(base.join(name))\n}\n"
                .to_string(),
            fixed_code: "pub fn read_asset(base: &std::path::Path, name: &str) -> std::io::Result<String> {\n    let joined = base.join(name);\n    if joined.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {\n        return Err(std::io::Error::new(\n            std::io::ErrorKind::InvalidData,\n            \"path escapes the base directory\",\n        ));\n    }\n    std::fs::read_to_string(joined)\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── webhook-signature-over-raw-bytes (CryptoHygiene; suggest-only) ──
        SecRule {
            rule_id: "webhook-signature-over-raw-bytes".to_string(),
            sec_concept: "a signature/HMAC verification computed over a RE-SERIALIZED \
body (`serde_json::to_string`/`to_vec` of a just-parsed value) instead of the \
exact received bytes: canonicalization drift (key order, whitespace, number \
format) lets an attacker reshape a payload and keep the check passing — the \
envelope/payload identity mismatch class (a signature is over the raw bytes \
the sender signed, verified BEFORE or alongside any parse). Suggest-only: \
which key, which fields, and the timestamp/nonce binding are per-scheme \
decisions. Distinct-from constant-time-secret-compare (owned): that rule \
owns the COMPARE primitive (timing), this owns WHICH BYTES the signature \
covers (coverage) — different axes. Honest recall limits: canonical-signature \
schemes that DELIBERATELY sign the canonical re-serialization (JWS-style) \
fire too and review decides — the rule flags the coverage question, not a \
definite bug; the re-serialization must happen in the same fn as the verify; \
to_writer forms and non-serde re-encodings unreached. Batch 258 / Plan 262 \
/ Research 292 (cloudflare/security-audit-skill @ `c1c8a8c`, MIT — cited as \
class provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "fn verify_sig(secret: &[u8], body: &[u8]) -> bool {\n    let mut tag_ok = body.len() == 32;\n    for b in body {\n        if *b == 0 {\n            tag_ok = false;\n        }\n    }\n    let _ = secret;\n    tag_ok\n}\n\nfn verify_webhook(secret: &[u8], raw: &[u8]) -> bool {\n    let event: serde_json::Value = serde_json::from_slice(raw).unwrap_or_default();\n    let canonical = serde_json::to_vec(&event).unwrap_or_default();\n    verify_sig(secret, &canonical)\n}\n"
                .to_string(),
            fixed_code: "fn verify_sig(secret: &[u8], body: &[u8]) -> bool {\n    let mut tag_ok = body.len() == 32;\n    for b in body {\n        if *b == 0 {\n            tag_ok = false;\n        }\n    }\n    let _ = secret;\n    tag_ok\n}\n\nfn verify_webhook(secret: &[u8], raw: &[u8]) -> bool {\n    verify_sig(secret, raw)\n}\n"
                .to_string(),
            family: SecFamily::CryptoHygiene,
        },
        // ── unvalidated-egress-url (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "unvalidated-egress-url".to_string(),
            sec_concept: "a URL assembled from request/tenant/config input reaching an \
outbound client sink (`reqwest::get(u)`, `ureq::get(u)`) with no scheme/host \
allowlist and no private-range refusal lets the caller pick the egress target: \
cloud metadata endpoints, loopback admin edges, internal peers — the SSRF \
class. The bounded repair is POLICY, never mechanical: pin the scheme, check \
the host against an allowlist, refuse private/loopback/link-local ranges. \
Suggest-only: the allowlist is deployment policy (which hosts are legitimate \
is the operator's answer). Sink-level novelty over unbounded-read-into-vec \
(owned): that rule owns `reqwest::get(..)` results as untrusted READ sources \
(response bytes growing a buffer), this owns the URL-CHOICE side of the same \
call — the Plan 156 T2 sink-distinct shape. Honest recall limits: only \
parse-provable vocabulary fires (free calls under the `reqwest::`/`ureq::` \
prefixes; a URL-vocabulary parameter or a format!-with-placeholder local as \
the URL argument) — client-binding method sinks and framework extractors \
(axum Path/Query) are honest NOs; a bare scheme check does NOT suppress (the \
host is the SSRF axis); the validation vocabulary (allowlist/scheme()/\
host_str()/private/loopback/link_local) anywhere in the fn suppresses; an \
	internal-only tool fetching its own config URL fires too and review decides; \
cross-fn URL construction undetected. USE-TIME face (Batch 292 rider, \
count-neutral — CVEngine `scannable_hosts`/`discovery.py` verbatim law, \
GPL-3.0 @ `9dd9c441`, cited as class provenance): an admission-time check on \
a STORED name rots — DNS moves, and a name a tenant controls can be pointed \
at metadata or an internal range AFTER it was validated — so every path that \
aims an outbound sink at a stored host list re-filters at the sink; the \
honest framing is that use-time re-validation narrows the DNS-rebinding \
window, never closes it. Batch 258 / Plan 262 / Research 292 \
(cloudflare/security-audit-skill @ `c1c8a8c`, MIT — cited as class \
provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "pub fn fetch_report(base: &str, probe: &str) -> Result<String, String> {\n    let url = format!(\"https://{base}/report/{probe}\");\n    let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;\n    resp.into_string().map_err(|e| e.to_string())\n}\n"
                .to_string(),
            fixed_code: "const ALLOWED_HOSTS: [&str; 1] = [\"reports.internal\"];\n\npub fn fetch_report(base: &str, probe: &str) -> Result<String, String> {\n    if !ALLOWED_HOSTS.contains(&base) {\n        return Err(String::from(\"host not allowed\"));\n    }\n    let url = format!(\"https://{base}/report/{probe}\");\n    let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;\n    resp.into_string().map_err(|e| e.to_string())\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Batch 291 (2026-10-26, Plan 295 / Research 329) — the danoszz/  ══
        // ══ penthera distill (MIT @ 07421f9): three suggest-only rules    ══
        // ══ across the two existing families. Fixtures are ORIGINAL       ══
        // ══ Rust syntheses; the source contributes class knowledge only.  ══
        // ── cors-any-origin-with-credentials (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "cors-any-origin-with-credentials".to_string(),
            sec_concept: "a permissive-origin CORS policy coupled with credentialed \
requests: `allow_origin(Any)` / `AllowOrigin::any()` / `permissive()` chained with \
`allow_credentials(true)` — or the one-call form `CorsLayer::very_permissive()`, \
which is credentials PLUS the preflight origin reflected back as allowed (the \
`origin: true` + `credentials: true` shape). The wildcard+credentials pair is \
protocol-inert for credentialed reads (browsers refuse credentials against a \
wildcard `Access-Control-Allow-Origin`), so the declared grant is dead \
misconfiguration — and the common naive repair (switch to very_permissive) \
ACTIVATES the leak: every origin's credentialed cross-origin requests pass. \
The bounded repair is an explicit origin allowlist (`AllowOrigin::list` / the \
docs' origins array / a predicate) beside `allow_credentials(true)`, or dropping \
credentials. Suggest-only: the legitimate origins are deployment policy, never \
mechanical. Honest recall limits: same-fn co-occurrence only; `allow_origin(Any)` \
WITHOUT a credentials marker stays silent (bare wildcard CORS is a policy smell, \
not this graded class); plain `permissive()` alone is silent (tower-http 0.7 \
docs: it sets NO credentials); `very_permissive()` fires UNCONDITIONALLY (the \
combo is its by-construction content); list/predicate/dynamic origin forms \
never fire; cross-fn builder assembly undetected. Batch 291 / Plan 295 / \
Research 329 (danoszz/penthera @ `07421f9`, MIT — cited as class provenance; \
the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "use tower_http::cors::{Any, CorsLayer};\n\npub fn permissive_cors() -> CorsLayer {\n    CorsLayer::new()\n        .allow_origin(Any)\n        .allow_credentials(true)\n}\n"
                .to_string(),
            fixed_code: "use tower_http::cors::{AllowOrigin, CorsLayer};\n\nconst ALLOWED_ORIGINS: [&str; 1] = [\"https://app.example.com\"];\n\npub fn allowlisted_cors() -> CorsLayer {\n    CorsLayer::new()\n        .allow_origin(AllowOrigin::list(\n            ALLOWED_ORIGINS.iter().filter_map(|o| o.parse().ok()),\n        ))\n        .allow_credentials(true)\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── jwt-insecure-validation-flags (CryptoHygiene; suggest-only) ──
        SecRule {
            rule_id: "jwt-insecure-validation-flags".to_string(),
            sec_concept: "a JWT decode configured to NOT verify: jsonwebtoken's \
`insecure_disable_signature_validation()` (the 9.x Validation surface — method \
or field form; REMOVED at 10.0 in favour of `dangerous::insecure_decode`, the \
same class one namespace over) accepts forged unsigned tokens — the alg:none \
bypass class — and `validate_exp = false` accepts expired tokens (expiry-skip). \
The penthera jwt probe (alg:none forged token accepted on an authed route; \
expired-token check) is the runtime face of exactly these decode-time flags. \
The bounded repair is the default posture: pin the algorithm list, leave \
signature + exp validation on. Suggest-only: some debug/tooling decodes \
deliberately skip validation — review decides. Honest recall limits: the \
signature surfaces fire on their unique names (any receiver); \
`validate_exp = false` fires only inside jwt vocabulary (a \
`jsonwebtoken`-prefixed path or a `Validation`/`DecodingKey`/`Algorithm` token \
in the same fn — other crates' fields stay silent); explicit \
`validate_exp = true` is silent; a helper-based decode is unconnected \
parse-only. Batch 291 / Plan 295 / Research 329 (danoszz/penthera @ \
`07421f9`, MIT — cited as class provenance; the fixture is an original \
synthesis)."
                .to_string(),
            buggy_code: "use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};\nuse serde::Deserialize;\n\n#[derive(Deserialize)]\npub struct Claims {\n    pub sub: String,\n    pub exp: usize,\n}\n\npub fn read_claims(token: &str, key: &DecodingKey) -> Option<Claims> {\n    let mut validation = Validation::new(Algorithm::HS256);\n    validation.validate_exp = false;\n    validation.insecure_disable_signature_validation();\n    let data = decode::<Claims>(token, key, &validation).ok()?;\n    Some(data.claims)\n}\n"
                .to_string(),
            fixed_code: "use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};\nuse serde::Deserialize;\n\n#[derive(Deserialize)]\npub struct Claims {\n    pub sub: String,\n    pub exp: usize,\n}\n\npub fn read_claims(token: &str, key: &DecodingKey) -> Option<Claims> {\n    let validation = Validation::new(Algorithm::HS256);\n    let data = decode::<Claims>(token, key, &validation).ok()?;\n    Some(data.claims)\n}\n"
                .to_string(),
            family: SecFamily::CryptoHygiene,
        },
        // ── auth-cookie-without-secure-flags (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "auth-cookie-without-secure-flags".to_string(),
            sec_concept: "an auth/session-named cookie (session/sid/token/auth/jwt \
vocabulary in the literal name) built WITHOUT the Secure/HttpOnly hardening — \
the cookie crate's `Cookie::build(..)`/`Cookie::new(..)` with no `.secure(..)` \
or `.http_only(..` marker anywhere in the fn: the session value ships \
JavaScript-readable over plain http (the penthera cookie class: session cookies \
missing Secure/HttpOnly/SameSite, severity keyed on the name vocabulary). The \
bounded repair is the builder chain: `.http_only(true).secure(true)` (+ \
`.same_site(SameSite::Lax)` for the CSRF half). Suggest-only: cookies that \
must be JS-readable or cross-site are deliberate choices. Honest recall \
limits: the name must be a string LITERAL (variable names are an honest NO); \
non-auth names (preference/tracking/consent) never fire — the vocabulary is \
the FP control; ANY flag marker in the fn suppresses fn-wide (one hardened \
cookie hides a sibling — the secret-file-create conservative rule), so an \
explicit `.secure(false)` is read as a written decision, never flagged; \
builder-split-across-locals and set_http_only-after-construction are \
unconnected parse-only (the fn-wide marker covers the setter forms). Batch \
291 / Plan 295 / Research 329 (danoszz/penthera @ `07421f9`, MIT — cited as \
class provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "use cookie::Cookie;\n\npub fn session_cookie(value: &str) -> Cookie<'static> {\n    Cookie::build((\"sid\", value.to_owned()))\n        .path(\"/\")\n        .same_site(cookie::SameSite::Lax)\n        .build()\n}\n"
                .to_string(),
            fixed_code: "use cookie::Cookie;\n\npub fn session_cookie(value: &str) -> Cookie<'static> {\n    Cookie::build((\"sid\", value.to_owned()))\n        .path(\"/\")\n        .http_only(true)\n        .secure(true)\n        .same_site(cookie::SameSite::Lax)\n        .build()\n}\n"
                .to_string(),
        family: SecFamily::InputValidation,
        },
        // ══ Batch 292 (2026-10-27, Plan 296 / Research 330) — the         ══
        // ══ hevnsnt/CVEngine distill (GPL-3.0 @ `9dd9c441`): three         ══
        // ══ suggest-only InputValidation rules on the two most universal  ══
        // ══ std surfaces (`std::net::IpAddr`, `std::process::Command`).   ══
        // ══ Pure-Python source (the transcription law: language is never  ══
        // ══ a gate); fixtures are ORIGINAL Rust syntheses; the source     ══
        // ══ contributes class knowledge only.                              ══
        // ── client-ip-from-untrusted-forwarded-header (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "client-ip-from-untrusted-forwarded-header".to_string(),
            sec_concept: "an `x-forwarded-for` header read used as the CLIENT IDENTITY \
(rate-limit key, ban key, audit actor) without any trusted-proxy gate: the \
header is attacker-settable, so the leftmost (or unconditional whole-header) \
read lets any caller rotate the identity that a throttle/ban decision keys on \
— the rate-limit/ban bypass class. The bounded repair is the three-regime \
resolver (CVEngine `client_ip.py`'s decision content): no trusted proxies \
configured → ignore the header entirely (use the peer address); the peer IS \
a trusted proxy → walk the chain RIGHT-TO-LEFT, the first untrusted hop wins; \
a malformed hop mid-chain → the whole chain is untrusted → fail closed to \
the peer. Plus the mapped-unwrap inside the parse (`::ffff:1.2.3.4` is \
`1.2.3.4` — ipv4-mapped-range-check-unwrap owns the range-test face). \
Suggest-only: the trusted-proxy set is deployment policy. Honest recall \
limits: fires only when the header LITERAL co-occurs with identity/throttle \
vocabulary (client_ip/rate_limit/ratelimit/throttle/identity/audit/banned) \
in the same fn and NO trusted-proxy marker (trusted) — a proxy-aware resolver \
suppresses fn-wide; an xff read with no identity use (logging/debug) stays \
silent; framework header extractors that never name the literal are honest \
NOs; cross-fn identity flow undetected. Batch 292 / Plan 296 / Research 330 \
(hevnsnt/CVEngine @ `9dd9c441`, GPL-3.0 — cited as class provenance; the \
fixture is an original synthesis)."
                .to_string(),
            buggy_code: "use std::net::IpAddr;\n\npub fn client_ip(forwarded: Option<&str>, peer: IpAddr) -> IpAddr {\n    forwarded\n        .and_then(|chain| chain.split(',').next())\n        .and_then(|hop| hop.trim().parse::<IpAddr>().ok())\n        .unwrap_or(peer)\n}\n\npub fn throttle_key(forwarded: Option<&str>, peer: IpAddr) -> IpAddr {\n    let ip = client_ip(forwarded, peer);\n    if banned(ip) {\n        panic!(\"rejected\");\n    }\n    ip\n}\n\nfn banned(_ip: IpAddr) -> bool {\n    false\n}\n"
                .to_string(),
            fixed_code: "use std::net::IpAddr;\n\nfn trusted(peer: IpAddr) -> bool {\n    // the deployment's proxy list — policy, never mechanical\n    let _ = peer;\n    false\n}\n\npub fn client_ip(forwarded: Option<&str>, peer: IpAddr) -> Option<IpAddr> {\n    if !trusted(peer) {\n        return Some(peer); // no trusted proxies: the header is attacker-settable\n    }\n    let chain = forwarded?;\n    let mut client = peer;\n    for hop in chain.split(',').rev() {\n        let Ok(ip) = hop.trim().parse::<IpAddr>() else {\n            return Some(peer); // a malformed hop: the chain is untrusted, fail closed\n        };\n        if !trusted(ip) {\n            client = ip;\n            break;\n        }\n    }\n    Some(client)\n}\n\npub fn throttle_key(forwarded: Option<&str>, peer: IpAddr) -> Option<IpAddr> {\n    let ip = client_ip(forwarded, peer);\n    if banned(ip?) {\n        panic!(\"rejected\");\n    }\n    ip\n}\n\nfn banned(_ip: IpAddr) -> bool {\n    false\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── shell-string-command-interpolation (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "shell-string-command-interpolation".to_string(),
            sec_concept: "user-influenced data interpolated into a SHELL-STRING command \
(`Command::new(\"sh\")` + `.arg(\"-c\")` + a `format!`/concat expression carrying \
the data): a metacharacter allowlist on the data does NOT make the shell string \
safe — argument injection (CWE-88) survives every allowlist that excludes only \
`;|&$` (the CVEngine tlsx site: `f'tlsx -host {domain}'` passes its own \
`[A-Za-z0-9._:/\\[\\] -]` charset and still reaches `foo -oG /tmp/x`). The bounded \
repair is the ARG ARRAY on the real binary (no shell, no metacharacter layer at \
all); where shell features are genuinely needed, strict per-token validation \
faces the injected datum and review owns it. Suggest-only: the shell-features \
regime is a design decision. Honest recall limits: fires on the `sh`/`bash`/\
`zsh`/`cmd`/`powershell` literal + a `-c`/`-Command`/`/C` literal + a \
`format!`/concat arg in the same fn; a format! arg on a NON-shell binary \
(nmap, tlsx invoked directly) stays silent (argument injection there is a \
different, narrower class); a STATIC `-c` script with no interpolation stays \
silent; macro-token bodies are never descended into; cross-fn construction \
undetected. Batch 292 / Plan 296 / Research 330 (hevnsnt/CVEngine @ \
`9dd9c441`, GPL-3.0 — cited as class provenance; the fixture is an original \
synthesis)."
                .to_string(),
            buggy_code: "use std::process::Command;\n\npub fn probe(domain: &str) -> std::io::Result<String> {\n    let out = Command::new(\"sh\")\n        .arg(\"-c\")\n        .arg(format!(\"tlsx -san -silent -host {domain}\"))\n        .output()?;\n    Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n"
                .to_string(),
            fixed_code: "use std::process::Command;\n\npub fn probe(domain: &str) -> std::io::Result<String> {\n    let out = Command::new(\"tlsx\")\n        .arg(\"-san\")\n        .arg(\"-silent\")\n        .arg(\"-host\")\n        .arg(domain) // the arg array: no shell, no metacharacter layer\n        .output()?;\n    Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── ipv4-mapped-range-check-unwrap (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "ipv4-mapped-range-check-unwrap".to_string(),
            sec_concept: "an `IpAddr` match whose V4 arm runs a range/allowlist/denylist \
test and whose V6 arm rejects or falls through WITHOUT unwrapping the mapped \
form: `::ffff:127.0.0.1` is a V6 address whose bits are loopback, so a \
version-split check waves it through every guard it was meant to fail — the \
bypass class for SSRF refusals, allowlists and firewall rules (CVEngine \
unwraps `ipv4_mapped` BEFORE any range test in both its resolver and its \
scanner gate). The bounded repair is `to_ipv4_mapped()` before the test: \
`if let Some(v4) = v6.to_ipv4_mapped() { v4 path } else { v6 path }`, or the \
normalize-up-front form. The companion law rides the context: PASS THE PARSED \
FORM ONWARD, never the raw token — Rust's `IpAddr::from_str` refuses scope ids \
(`fe80::1%eth0`), which is itself the boundary (CVEngine must refuse them \
manually; Rust's parser already did). Suggest-only: some guards deliberately \
treat all V6 as external. Honest recall limits: fires on an `IpAddr::V4`/\
`IpAddr::V6` match pair whose V6 arm carries no `to_ipv4_mapped` and whose fn \
lacks it entirely, gated on range-test vocabulary (contains/is_loopback/\
is_private/is_link_local/allowlist/blocked/nets) so a debug-print match stays \
silent; an fn with `to_ipv4_mapped` anywhere suppresses (the normalize is \
present); a folded single-arm match is parse-only unreachable. Batch 292 / \
Plan 296 / Research 330 (hevnsnt/CVEngine @ `9dd9c441`, GPL-3.0 — cited as \
class provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "use std::net::{IpAddr, Ipv4Addr};\n\nconst BLOCKED: [Ipv4Addr; 2] = [Ipv4Addr::LOCALHOST, Ipv4Addr::new(169, 254, 169, 254)];\n\npub fn is_blocked(ip: IpAddr) -> bool {\n    match ip {\n        IpAddr::V4(v4) => BLOCKED.contains(&v4),\n        IpAddr::V6(_) => false,\n    }\n}\n"
                .to_string(),
            fixed_code: "use std::net::{IpAddr, Ipv4Addr};\n\nconst BLOCKED: [Ipv4Addr; 2] = [Ipv4Addr::LOCALHOST, Ipv4Addr::new(169, 254, 169, 254)];\n\npub fn is_blocked(ip: IpAddr) -> bool {\n    match ip {\n        IpAddr::V4(v4) => BLOCKED.contains(&v4),\n        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {\n            Some(v4) => BLOCKED.contains(&v4), // ::ffff:127.0.0.1 IS loopback\n            None => false,\n        },\n    }\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── abuse-strike-counts-cross-site-requests (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "abuse-strike-counts-cross-site-requests".to_string(),
            sec_concept: "an abuse/strike counter incremented on EVERY rejected request \
regardless of the browser's site label: a hostile page can make a victim's \
browser send attack-shaped requests (an image tag, an auto-submitted form), so \
counting those rejections bans the VICTIM — denial-of-service via the defense \
itself (CVEngine's guard docstring: requests a browser marks cross-site are \
rejected but never counted, so a hostile page cannot get a visitor banned). \
The bounded repair is the site-label gate before the increment: `sec-fetch-site` \
(cross-site/same-site) — or Origin/Referer where the label is absent — rejects \
the request but skips the strike. Suggest-only: which labels to honor and the \
fallback ordering are deployment policy. Honest recall limits: fires only on \
strike-vocabulary (strike/abuse/penalty) incrementing beside a rejection \
marker (400/403/StatusCode/reject) with NO site-label vocabulary (sec_fetch/\
sec-fetch-site/same-site/cross-site/origin-header/referer vocabulary) anywhere \
in the fn — a guard that already classifies suppresses; a strike counter with \
no rejection path (game score, retry budget) stays silent; cross-fn counting \
undetected. Batch 292 / Plan 296 / Research 330 (hevnsnt/CVEngine @ \
`9dd9c441`, GPL-3.0 — cited as class provenance; the fixture is an original \
synthesis)."
                .to_string(),
            buggy_code: "pub struct Guard { strikes: u32 }\n\npub enum Verdict { Ok, Rejected }\n\nimpl Guard {\n    pub fn inspect(&mut self, body: &str) -> Verdict {\n        if body.contains(\"; cat \") || body.contains(\"../\") {\n            self.strikes += 1; // counts the victim's forged requests too\n            return Verdict::Rejected;\n        }\n        Verdict::Ok\n    }\n}\n"
                .to_string(),
            fixed_code: "pub struct Guard { strikes: u32 }\n\npub enum Verdict { Ok, Rejected }\n\nimpl Guard {\n    pub fn inspect(&mut self, body: &str, site_label: Option<&str>) -> Verdict {\n        if body.contains(\"; cat \") || body.contains(\"../\") {\n            // a hostile page forged this: reject, never count (the victim\n            // must not be banned by it)\n            if site_label != Some(\"same-site\") {\n                return Verdict::Rejected;\n            }\n            self.strikes += 1;\n            return Verdict::Rejected;\n        }\n        Verdict::Ok\n    }\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Batch 294 (2026-10-27, Plan 298 / Research 332) — the      ══
        // ══ wasmtime RUSTSEC-2026-0380/0379 distill: two suggest-only  ══
        // ══ InputValidation rules (the JIT classes, userland-shaped).  ══
        // ── packed-field-silent-overflow-wrong-key (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "packed-field-silent-overflow-wrong-key".to_string(),
            sec_concept: "a count/ID masked to a fixed bit-width (`& 0xFF` … the 2^k-1 \
literals) and used as a LOOKUP KEY silently loses every bit above the mask — values \
whose legitimate range exceeds the field width alias onto wrong keys: wrong-item \
access, no panic (the wasmtime 24-bit packed value-number class, RUSTSEC-2026-0380 / \
CVE-2026-115889: release builds lose the overflow checks, the corrupt value keys a \
lookup, type confusion follows; CVSS 9.9). The bounded repair is a range assert at the \
packing site (`assert!(id <= MASK)`), a checked newtype that refuses construction past \
the field width, or widening the field. Suggest-only: the corrupt key may be a DESIGNED \
alias (ring buffer `i & (SIZE-1)`, hash-bucket `fnv & 0xFF` — the counter/ID vocabulary \
gate keeps the common hash/checksum spellings out; a counter-named ring index still \
fires and review decides). Distinct-from: unchecked-wire-arithmetic owns the bare-op \
overflow sink (+ - * << >> — BitAnd is deliberately absent from its operator set, so the \
mask site is invisible to it); clippy cast_possible_truncation is blanket as-cast \
pedantic with no key-usage scoping and no fix; the B258 recorded drop (integer \
overflow/truncation owned by those two) is adjudicated in-rule — this rule owns NEITHER \
half, the packing SITE is third-sink novelty (the Plan 156 T2 pattern); Research 161's \
renewal check clears (its dropped set — recursion/set_len/uninit/symlink — does not \
overlap). Honest recall limits: type-blind (`b: u8 & 0xFF` no-ops fire — parse-only \
cannot see the operand width); named-const masks (`id & VALUE_MASK`) are unprovable \
values — silent; method-based lookups (`map.get(&(id & MASK))`) out — the index form \
only; cross-fn packing undetected. Batch 294 / Plan 298 / Research 332 \
(RUSTSEC-2026-0380, CC0 — cited as class provenance; the fixture is an original \
synthesis)."
                .to_string(),
            buggy_code: "pub struct Row {\n    pub kind: u8,\n}\n\npub fn resolve_row(header: [u8; 4], table: &[Row; 4096]) -> &Row {\n    let id = u32::from_be_bytes(header);\n    let slot = (id & 0xFFF) as usize;\n    &table[slot]\n}\n"
                .to_string(),
            fixed_code: "pub struct Row {\n    pub kind: u8,\n}\n\npub fn resolve_row(header: [u8; 4], table: &[Row; 4096]) -> &Row {\n    let id = u32::from_be_bytes(header);\n    assert!(id < 4096, \"id past the 12-bit packing width\");\n    let slot = id as usize; // full width — no mask, no alias\n    &table[slot]\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── capacity-exceeded-silent-fallback-not-refusal (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "capacity-exceeded-silent-fallback-not-refusal".to_string(),
            sec_concept: "a fixed-capacity structure built from variable-length input \
DEGRADES SILENTLY when capacity is exceeded — the cap-exceeded branch truncates and \
falls through, so the encode/build continues with silently-dropped entries: wrong \
encoding, wrong dispatch, no refusal (the wasmtime aarch64 br_table class, \
RUSTSEC-2026-0379 / CVE-2026-115893: >256K-target jump tables silently pointed at the \
WRONG target — the fix's refusal half is the lesson: >65,520 targets now REFUSE loud; \
CVSS 9.9). The bounded repair is the early capacity check + typed error at the \
encode/build boundary: `if items.len() > CAP { return Err(..) }` — a cap that lies is \
worse than no cap. Suggest-only: the error type is the fn's choice (a protocol \
decision). Gates: the same-cap shape (the if compares `.len() > CAP` and the branch \
truncates the SAME receiver to the SAME cap text — const-vs-literal compare by token \
text) + encode/build vocabulary (fn name segments — encode/build/pack/serial/marshal/\
emit/table/dispatch/jump/frame/section/vessel — or `to_{be,le,ne}_bytes` in the body); \
the refusal twin (any return/?/panic-family/unwrap/expect in the branch) suppresses; \
benign caps outside that vocabulary (\"limit the preview to 10 lines\") stay silent. \
Distinct-from length-prefix-validate-before-alloc: that rule owns the MISSING-CHECK \
face (no cap at all — the allocation-DoS sink); this owns the LYING-CAP face (a cap \
that silently drops — the wrong-dispatch sink); complementary defects, distinct \
repairs. Honest recall limits: a warned truncate or a log-limit drop inside \
encode-vocabulary fns fires too and review decides (suggest-only is the blast-radius \
control); the slice form (`&entries[..CAP]`) and `take(n)` clamps are out; cross-fn \
builds unconnected. Batch 294 / Plan 298 / Research 332 (RUSTSEC-2026-0379, CC0 — \
cited as class provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "const MAX_TARGETS: usize = 64;\n\npub fn encode_table(targets: &[u16], out: &mut Vec<u8>) {\n    let mut owned = targets.to_vec();\n    if owned.len() > MAX_TARGETS {\n        owned.truncate(MAX_TARGETS); // silently drops the excess\n    }\n    out.extend_from_slice(&(owned.len() as u16).to_be_bytes());\n    for t in &owned {\n        out.extend_from_slice(&t.to_be_bytes());\n    }\n}\n"
                .to_string(),
            fixed_code: "const MAX_TARGETS: usize = 64;\n\npub fn encode_table(targets: &[u16]) -> Result<Vec<u8>, String> {\n    if targets.len() > MAX_TARGETS {\n        return Err(format!(\"too many targets: {}\", targets.len()));\n    }\n    let mut out = Vec::with_capacity(targets.len() * 2 + 2);\n    out.extend_from_slice(&(targets.len() as u16).to_be_bytes());\n    for t in targets {\n        out.extend_from_slice(&t.to_be_bytes());\n    }\n    Ok(out)\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Issue 185 (2026-10-27) — the softmax-collapse family (Numerics). ══
        // ══ One mechanical rewrite + two suggest-only detectors.            ══
        // ── sigmoid-minus-one (Numerics; strict-gated proposer) ──
        SecRule {
            rule_id: "sigmoid-minus-one".to_string(),
            sec_concept: "`sigmoid(z) - 1.0` (the one-vs-rest / BCE positive-slot \
gradient) rounds to EXACTLY 0.0 in f32 once z passes ~16.6 (the Softmax Collapse \
cliff, arXiv:2610.31642) — sigmoid(z) rounds to 1.0 and the difference is lost, so the \
positive slot stops learning while the negatives keep pushing. Write the same quantity \
as `-sigmoid(-z)` (full f32 range; the identity sigmoid(z) - 1 = -sigmoid(-z) is exact \
for the logistic function); the one-hot split `sigmoid(z) - if d == label { 1.0 } else \
{ 0.0 }` becomes `if d == label { -sigmoid(-z) } else { sigmoid(z) }`. Strict gates: \
logistic-name ALLOWLIST (fast/hard/lut/deriv/log/repo-prefixed sigmoids refuse — an \
approximation need not satisfy sigmoid(-z) = 1 - sigmoid(z)), single-arg free call, \
never inside a comparison or `.abs()` (a saturation check), split form only in a bare \
context, comment guard, macro refusal, tests skipped. Shipped repair: riir-reflex \
6ba895d (label_heads.rs). Issue 185 T1."
                .to_string(),
            buggy_code: "fn sigmoid(z: f32) -> f32 {\n    1.0 / (1.0 + (-z).exp())\n}\n\npub fn positive_grad(z: f32, d: usize, label: usize) -> f32 {\n    let g = sigmoid(z) - if d == label { 1.0 } else { 0.0 };\n    g\n}\n".to_string(),
            fixed_code: "fn sigmoid(z: f32) -> f32 {\n    1.0 / (1.0 + (-z).exp())\n}\n\npub fn positive_grad(z: f32, d: usize, label: usize) -> f32 {\n    let g = if d == label { -sigmoid(-z) } else { sigmoid(z) };\n    g\n}\n".to_string(),
            family: SecFamily::Numerics,
        },
        // ── softmax-onehot-collapse (Numerics; suggest-only) ──
        SecRule {
            rule_id: "softmax-onehot-collapse".to_string(),
            sec_concept: "the `grad = softmax - onehot` paste computes the target slot as \
`p_t - 1`, which rounds to EXACTLY 0.0 in f32 once the logit margin passes the cliff \
24*ln2 ~ 16.6 (Softmax Collapse, arXiv:2610.31642); every non-target p_i stays \
positive, so the row stops summing to zero — under AdamW the Slingshot instability. \
Exclude the target from the LSE instead: m = logsumexp over the OTHER logits, \
grad[t] = -sigmoid(m - z_t), grad[i != t] = sigmoid(m - z_t) * exp(z_i - m) (the \
`ce_collapse_free_into` primitive). Fires on a softmax row (a `softmax*` call result, \
or a row normalized by `/= sum` / `* inv_sum` / a scale-by-sum call) followed in the \
same fn by `X[t] -= 1.0`, `X[v] - if v == t { 1.0 } else { 0.0 }`, \
`x - f32::from(i == t)` over a row element, or `X - onehot`. Suggest-only: the repair \
restructures the pass and the shapes vary (SIMD, temperature, weighted one-hot). \
Recall limits: rows normalized in a helper (cross-fn), weighted one-hots \
(`X[t] -= w`), un-sum-named divisors. Shipped repairs: riir-rethink 2850291, \
katgpt-rs 4ba82db2f, riir-ai 8981315af, riir-refine 1b426b53. Issue 185 T2."
                .to_string(),
            buggy_code: "fn softmax(v: &[f32]) -> Vec<f32> {\n    let m = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let e: Vec<f32> = v.iter().map(|x| (x - m).exp()).collect();\n    let s: f32 = e.iter().sum();\n    e.iter().map(|x| x / s).collect()\n}\n\npub fn ce_grad(logits: &[f32], target: usize) -> Vec<f32> {\n    let mut grad = softmax(logits);\n    grad[target] -= 1.0;\n    grad\n}\n".to_string(),
            fixed_code: "pub fn ce_grad(logits: &[f32], target: usize) -> Vec<f32> {\n    let mut grad = vec![0.0_f32; logits.len()];\n    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let mut sum_excl = 0.0_f32;\n    for (i, (g, &z)) in grad.iter_mut().zip(logits).enumerate() {\n        if i != target {\n            *g = (z - max).exp();\n            sum_excl += *g;\n        }\n    }\n    if sum_excl <= 0.0 {\n        return grad;\n    }\n    let gap = sum_excl.ln() + max - logits[target];\n    let s = 1.0 / (1.0 + (-gap).exp());\n    let scale = s / sum_excl;\n    for g in &mut grad {\n        *g *= scale;\n    }\n    grad[target] = -s;\n    grad\n}\n".to_string(),
            family: SecFamily::Numerics,
        },
        // ── neg-ln-softmax-prob (Numerics; suggest-only, low severity) ──
        SecRule {
            rule_id: "neg-ln-softmax-prob".to_string(),
            sec_concept: "`-p[t].ln()` / `-(p[t].max(eps)).ln()` / `loss -= p[t].ln()` \
over a softmax row reads 0 past the f32 margin cliff (~16.6, arXiv:2610.31642) — the \
probability rounded to 1.0, so the reported loss is a floor, not a measurement (an \
eps clamp hides the other tail too). Compute the loss from the logits: \
softplus(m - z_t) with m = logsumexp over the other logits (exact both sides, no \
clamp — the `ce_collapse_free_into` loss). Low severity: the loss is a REPORT, the \
gradient is `softmax-onehot-collapse`'s. Suggest-only: the row no longer carries the \
logits. Shipped repairs: riir-ai 8981315af, riir-refine 1b426b53. Issue 185 T3."
                .to_string(),
            buggy_code: "fn softmax(v: &[f32]) -> Vec<f32> {\n    let m = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let e: Vec<f32> = v.iter().map(|x| (x - m).exp()).collect();\n    let s: f32 = e.iter().sum();\n    e.iter().map(|x| x / s).collect()\n}\n\npub fn ce_loss(logits: &[f32], target: usize) -> f32 {\n    let p = softmax(logits);\n    -p[target].ln()\n}\n".to_string(),
            fixed_code: "pub fn ce_loss(logits: &[f32], target: usize) -> f32 {\n    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let sum_excl: f32 = logits\n        .iter()\n        .enumerate()\n        .filter(|&(i, _)| i != target)\n        .map(|(_, &z)| (z - max).exp())\n        .sum();\n    let gap = sum_excl.ln() + max - logits[target];\n    gap.max(0.0) + (-gap.abs()).exp().ln_1p()\n}\n".to_string(),
            family: SecFamily::Numerics,
        },
        // ── arc-refcount-across-raw-boundary (DropLeak; suggest-only, B325) ──
        SecRule {
            rule_id: "arc-refcount-across-raw-boundary".to_string(),
            sec_concept: "a raw Arc pointer crossing an unsafe boundary has exactly two \
custody regimes — BORROWED (`Arc::increment_strong_count` + deref, the boundary keeps \
the count) or CONSUMING (`Arc::from_raw` exactly once) — and any mix of the two \
(from_raw-then-forget, double from_raw, an increment stacked on a from_raw consume) \
double-frees or leaks. Detector: `proposers::arc_raw_boundary` (the `into_raw` \
round-trip is the legitimate shape and suppresses). Batch 325 / Plan 327 / Research 361 \
(mozilla/uniffi-rs @ ef9625b6, MPL-2.0 — class provenance, original fixture) FACE (Batch 341, metal-graph — the WRAP-CUSTODY-AGREEMENT arm): a foreign-buffer wrap (taking raw outside storage into managed custody) requires an EXPLICIT retention regime AND callback/flag agreement in BOTH directions: 'Metal buffer wrap requires retained buffer or owner lifetime' — no regime selected refuses at entry; a RETAIN_OWNER flag with missing callbacks refuses ('owner retention is invalid') AND callbacks present without the flag refuse the mirror way ('owner callbacks require RETAIN_OWNER') — and write-access demands the mutability flag ('Metal buffer wrap write access requires MUTABLE flag') (metal_backend.mm:778-801). The two-regime custody law above owns the Arc count algebra; THIS owns the AGREEMENT axis — flag⇄callback consistency and access⇄mutability pairing checked BOTH directions at the boundary, fail-closed."
                .to_string(),
            buggy_code: "use std::sync::Arc;\n\n// waker-shaped custody bug: the borrowed handout (increment) and the consuming\n// path (from_raw) collapse into one fn, and the consume is forgotten — the\n// count is over-handed AND never retired (double-own → double-free or leak).\nstruct InstinctWaker;\n\nunsafe fn clone_waker(ptr: *const InstinctWaker) {\n    Arc::increment_strong_count(ptr);\n    let waker = unsafe { Arc::<InstinctWaker>::from_raw(ptr) };\n    std::mem::forget(waker);\n}\n"
                .to_string(),
            fixed_code: "// fixed: pick ONE regime per path — borrowed (increment + deref, the boundary\n// keeps custody) or consuming (from_raw exactly once; the Arc's own drop retires it).\nstruct InstinctWaker;\n\nunsafe fn clone_waker(ptr: *const InstinctWaker) {\n    Arc::increment_strong_count(ptr);\n    let borrowed: &InstinctWaker = &*ptr;\n    let _ = borrowed;\n}\n\nunsafe fn consume_waker(ptr: *mut InstinctWaker) {\n    let owned: Arc<InstinctWaker> = Arc::from_raw(ptr);\n    drop(owned);\n}\n"
                .to_string(),
            family: SecFamily::DropLeak,
        },
        // ── ffi-callback-invoke-outside-lock (Ffi; suggest-only, B325) ──
        SecRule {
            rule_id: "ffi-callback-invoke-outside-lock".to_string(),
            sec_concept: "a callback/waker stored behind a mutex must be EXTRACTED under the lock \
and INVOKED after the guard drops — invoking while the guard is live deadlocks when the \
callback re-enters this lock or holds a foreign runtime lock (the cross-FFI ABBA regime: \
internal Mutex vs GIL/event-loop). Distinct-from `host-lock-across-reentrant-blocking-call` \
(same-lock re-entry vs cross-FFI ABBA two-lock — in-rule adjudication note mandatory). \
Detector: `proposers::callback_lock_invoke`. Batch 325 / Plan 327 / Research 361 \
(mozilla/uniffi-rs @ ef9625b6, MPL-2.0 — class provenance, original fixture)."
                .to_string(),
            buggy_code: "use std::sync::Mutex;\ntrait Invoke { fn invoke(&self, poll: u32); }\nstruct Scheduler { cb: Mutex<Option<Box<dyn Invoke>>> }\nimpl Scheduler {\n    fn poll_once(&self, poll: u32) {\n        let g = self.cb.lock().unwrap();\n        if let Some(cb) = g.as_ref() {\n            cb.invoke(poll); // guard live: the callback may re-enter — ABBA deadlock\n        }\n    }\n}\n"
                .to_string(),
            fixed_code: "use std::sync::Mutex;\ntrait Invoke { fn invoke(&self, poll: u32); }\nstruct Scheduler { cb: Mutex<Option<Box<dyn Invoke>>> }\nimpl Scheduler {\n    fn poll_once(&self, poll: u32) {\n        let to_invoke = self.cb.lock().unwrap().take();\n        if let Some(cb) = to_invoke {\n            cb.invoke(poll); // extract-under-lock, invoke-after-release\n        }\n    }\n}\n"
                .to_string(),
            family: SecFamily::Ffi,
        },
        // ── out-param-manuallydrop-not-maybeuninit (Ffi; suggest-only, B325) ──
        SecRule {
            rule_id: "out-param-manuallydrop-not-maybeuninit".to_string(),
            sec_concept: "a caller-initialized `#[repr(C)]` FFI out-parameter slot — an output struct the \
FOREIGN caller allocates and the callee OVERWRITES exactly once — takes `&mut ManuallyDrop<T>` \
(safe reads, explicit drop discipline, set-twice = a visible single-writer leak), never \
`&mut MaybeUninit<T>`: MaybeUninit is for callee-initializes scratch the caller never reads \
as initialized, not for a write-once caller-owned slot. Detector: \
`proposers::outparam_init_contract` (full-write sites fire; partial field init keeps \
MaybeUninit). Batch 325 / Plan 327 / Research 361 (mozilla/uniffi-rs @ ef9625b6, MPL-2.0 — \
class provenance, original fixture)."
                .to_string(),
            buggy_code: "use std::mem::MaybeUninit;\n\n#[repr(C)]\npub struct RustBuffer {\n    pub capacity: i64,\n    pub len: i64,\n    pub data: *mut u8,\n}\n\n#[no_mangle]\npub extern \"C\" fn rustbuf_from_vec(out: &mut MaybeUninit<RustBuffer>) -> i32 {\n    // caller-initialized MaybeUninit slot: the callee overwrites the whole\n    // value exactly once, so ManuallyDrop was the honest contract; the\n    // status code is the C-ABI pair for the out-param\n    let buf = RustBuffer { capacity: 16, len: 0, data: std::ptr::null_mut() };\n    unsafe { MaybeUninit::write(out, buf) };\n    0\n}\n"
                .to_string(),
            fixed_code: "#[repr(C)]\npub struct RustBuffer {\n    pub capacity: i64,\n    pub len: i64,\n    pub data: *mut u8,\n}\n\n#[no_mangle]\npub extern \"C\" fn rustbuf_from_vec(out: &mut ManuallyDrop<RustBuffer>) -> i32 {\n    // caller-initialized contract: safe reads; set-twice = a visible leak;\n    // the status code is the C-ABI pair for the out-param\n    let buf = RustBuffer { capacity: 16, len: 0, data: std::ptr::null_mut() };\n    unsafe { ManuallyDrop::write(out, ManuallyDrop::new(buf)) };\n    0\n}\n"
                .to_string(),
            family: SecFamily::Ffi,
        },
        // ── panic-handler-nested-catch-unwind (PanicSafety; suggest-only, B325) ──
        SecRule {
            rule_id: "panic-handler-nested-catch-unwind".to_string(),
            sec_concept: "an error-reporting handler that itself does fallible work (`format!`/alloc) \
carries its own `catch_unwind` + graceful degradation — a panic escaping the handler during \
an active unwind double-panics, and a panic from the reporting path masks the original error. \
Distinct-from `ffi-extern-fn-catch-unwind` (extern BODY wrapper) and `drop-impl-no-panic` \
(Drop bodies) — the handler PATH was unowned. Detector: `proposers::panic_handler_guard`. \
Batch 325 / Plan 327 / Research 361 (mozilla/uniffi-rs @ ef9625b6, MPL-2.0 — class \
provenance, original fixture)."
                .to_string(),
            buggy_code: "pub struct DecodeError {\n    pub code: i32,\n}\n\npub fn report_error(err: &DecodeError, out: &mut String) {\n    // render the failure for the caller; the buffer is the contract\n    out.push_str(&format!(\"decode failed: code={}\", err.code));\n}\n"
                .to_string(),
            fixed_code: "pub struct DecodeError {\n    pub code: i32,\n}\n\npub fn report_error(err: &DecodeError, out: &mut String) {\n    // the handler guards ITSELF: a panic here would double-panic the\n    // active unwind and mask this very error\n    if let Ok(msg) = std::panic::catch_unwind(|| format!(\"decode failed: code={}\", err.code)) {\n        out.push_str(&msg);\n    }\n    // graceful degradation: an empty buffer is the spoken fallback\n}\n"
                .to_string(),
            family: SecFamily::PanicSafety,
        },
        // ══ Batch 327 (2026-11-03, Plan 332 / Research 364) — the       ══
        // ══ firezone/sanstun distill (Apache-2.0 @ 35cb1932948bd0776):  ══
        // ══ three suggest-only rules across the two families.           ══
        // ── secret-key-without-zeroize (CryptoHygiene; suggest-only, dep-gated) ──
        SecRule {
            rule_id: "secret-key-without-zeroize".to_string(),
            sec_concept: "a struct in the secret vocabulary (*Key/*Secret/*Seed/*Token/*\
Password/*Credential as word segments) holding RAW key material ([u8; N], Vec<u8>, String) \
with no Drop impl and no zeroization wrapper keeps the secret bytes alive in FREED MEMORY \
after the struct drops — heap spray, core dumps and swap all read them back. Wrap in \
`zeroize::Zeroizing` or derive Zeroize/ZeroizeOnDrop (the sanstun anchor: keys.rs documents \
`An X25519 static private key, zeroized on drop.` and carries `pub struct \
PresharedKey(Zeroizing<[u8; 32]>);` — Zeroizing wraps owned [u8; 32] key material across \
8 files of the crate, fields/params/returns alike). DEP-GATED, the \
constant-time-secret-compare shape: fires only when the file already imports zeroize, so \
the suggestion cannot break the build (a file doing secret hygiene without the dep is an \
honest miss). Distinct-from constant-time-secret-compare IN-RULE: that rule owns the \
COMPARE primitive (the timing leak at comparison); this owns the LIFETIME/ERASE axis (bytes \
persisting after drop). Suggest-only: choosing the wrapper (Zeroizing vs derive vs a custom \
Drop) is an API decision the struct's users feel. Gates: struct-name vocabulary + raw field \
type NOT already Zeroizing-wrapped + NO Drop impl for the struct in the file + NO \
Zeroize/ZeroizeOnDrop derive or non-doc zeroize attribute on it; #[cfg(test)] mods skipped; \
doc-comment prose never counts as a derive. Honest recall limits: wrapper-typed secrets \
(x25519_dalek::StaticSecret fields) parse as opaque and stay silent — only the three raw \
shapes fire; a Drop impl in ANOTHER file is invisible; per-field findings — a struct with \
one wrapped and one raw field still fires on the raw one (partial wrapping is the bug); a \
zeroize import used only through fully-qualified paths is an honest miss; the vocabulary \
proves a NAME CLASS — a TokenBucket over Vec<u8> fires and review decides. Batch 327 / \
Plan 332 / Research 364 (firezone/sanstun @ 35cb1932948bd0776, Apache-2.0 — cited as class \
provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "use zeroize::Zeroizing;\n\
\n/// The tenant's signing seed as loaded from disk.\n\
pub struct SigningSeed {\n\
    pub tenant: u32,\n\
    pub material: [u8; 32],\n\
}\n\
\npub fn scratch_tag() -> Zeroizing<[u8; 32]> {\n\
    Zeroizing::new([0_u8; 32]) // a scratch value is wrapped; the struct's own seed is not\n\
}\n"
                .to_string(),
            fixed_code: "use zeroize::{Zeroize, Zeroizing};\n\
\n/// The tenant's signing seed, zeroized on drop.\n\
pub struct SigningSeed {\n\
    pub tenant: u32,\n\
    pub material: Zeroizing<[u8; 32]>,\n\
}\n\
\npub fn scratch_tag() -> Zeroizing<[u8; 32]> {\n\
    Zeroizing::new([0_u8; 32])\n\
}\n"
                .to_string(),
            family: SecFamily::CryptoHygiene,
        },
        // ── untrusted-keyed-state-unbounded (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "untrusted-keyed-state-unbounded".to_string(),
            sec_concept: "a HashMap/HashSet keyed by PEER-DERIVED distinct keys (peer \
vocabulary as word segments of the map variable/field, the key type, or the enclosing fn: \
addr/ip/peer/session/device/tenant/origin/client/conn/source) growing via insert with NO \
cap guard admits the many-keys memory DoS — one adversarial peer family minting distinct \
keys exhausts memory. The CARDINALITY face of resource exhaustion. The fix shape is the \
source's own bounded table + FAIL-CLOSED refusal of NEW keys when full: `const \
MAX_SOURCES: usize = 65536;` + `if self.buckets.len() >= MAX_SOURCES { return false; }` \
before the insert — eviction would REWARD the attacker by admitting them over honest \
entries (the source's own test name says it: `full_table_refuses_new_sources_until_idle_\
ones_are_dropped`; its doc: `Packets from new sources are refused while the table is \
full.`). Distinct-from length-prefix-validate-before-alloc and unbounded-read-into-vec \
IN-RULE: those own the VOLUME faces (one length field to one alloc; one stream to one \
growing buffer); this owns the CARDINALITY face (many distinct keys to an unbounded entry \
count). Suggest-only: choosing the cap and the full-table policy is a protocol decision. \
Gates: the receiver resolves to a HashMap/HashSet declared IN THE SAME FILE (a local or a \
struct field); peer-key vocabulary on the map name, key type, or fn name; suppression is \
FN-WIDE, the one-guard-hides-the-fn house rule — any len()-vs-cap comparison, any \
contains_key call, or any remove/retain/extract_if/drain/clear in the fn silences every \
insert in it. Honest recall limits: cross-file map types are an honest NO; \
entry().or_insert() growth undetected (insert only); with_capacity is a capacity HINT, \
never a bound, and never a suppression; plural names miss the exact segment (`sessions` is \
not `session`) unless another source hits; the vocabulary proves a NAME CLASS — a \
session-named map over trusted internal ids fires and review decides. Batch 327 / Plan \
332 / Research 364 (firezone/sanstun @ 35cb1932948bd0776, Apache-2.0 — cited as class \
provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "use std::collections::HashMap;\n\
\n#[derive(Eq, Hash, PartialEq)]\n\
pub struct ClientId(pub u32);\n\
pub struct Member;\n\
\npub struct Room {\n\
    members: HashMap<ClientId, Member>,\n\
}\n\
\nimpl Room {\n\
    pub fn join(&mut self, who: ClientId, m: Member) {\n\
        // a client family mints distinct ids; nothing bounds the table\n\
        self.members.insert(who, m);\n\
    }\n\
}\n"
                .to_string(),
            fixed_code: "use std::collections::HashMap;\n\
\n#[derive(Eq, Hash, PartialEq)]\n\
pub struct ClientId(pub u32);\n\
pub struct Member;\n\
\npub struct Room {\n\
    members: HashMap<ClientId, Member>,\n\
}\n\
\nconst MAX_CLIENTS: usize = 1024;\n\
\nimpl Room {\n\
    pub fn join(&mut self, who: ClientId, m: Member) -> bool {\n\
        if self.members.len() >= MAX_CLIENTS {\n\
            return false; // full: refuse NEW clients, known ones keep service\n\
        }\n\
        self.members.insert(who, m);\n\
        true\n\
    }\n\
}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── gateway-type-forces-security-check (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "gateway-type-forces-security-check".to_string(),
            sec_concept: "a security-relevant message/handler type whose CONSTRUCTOR is \
PUBLIC lets any caller synthesize inputs and walk around the gateway's checkpoint — rate \
limit, MAC verification, replay check — the checks exist but are skippable because the \
dangerous handler stays reachable without them. The fix is crate-private constructors + \
the gateway type as the ONLY producer, so the handler is UNREACHABLE without passing the \
checkpoint (the sanstun anchor: message.rs keeps every message-type constructor \
crate-private — `pub(crate) fn new<const N: usize>` on Datagram, pub(crate) fields on \
HandshakeResponse/CookieReply/TransportData, no public ctor anywhere — and the doc pins \
the law: `Messages can only be obtained through that method and cannot be cloned or \
constructed directly.`; interface.rs's `pub fn receive(&mut self, src_addr: SocketAddr, \
datagram: &[u8], now: Instant)` is the single parse dispatch behind the guard sequence \
mac1 ct_eq, under-load window, cookie check, rate_limiter.allow). Distinct-from \
client-ip-from-untrusted-forwarded-header IN-RULE: that rule owns rate-limit KEY SPOOFING \
(an untrusted header as the identity); this owns CHECKPOINT BYPASS via constructor \
reachability. Suggest-only, and precision is honestly POOR: which handler is \
security-relevant is vocabulary-gated (message vocabulary on the struct name: \
handshake/response/reply/datagram/packet/frame/ticket/claim/envelope/message; gateway \
vocabulary on the fn name: receive/verify/gateway/checkpoint/accept/authorize + check \
tokens in the body: rate/limit/mac/verif/auth/replay/cookie/allow); cross-file gateway \
relationships are invisible (same-file only); a pub ctor on a genuinely-public DTO fires \
and review decides — suggest-only is the blast-radius control. Gates: ALL must hold — a \
pub struct in the message vocabulary + a pub associated ctor (new / from_*) on it + a \
same-file gateway fn taking a self reference with check vocabulary that references the \
struct's type by EXACT ident; the fixed shape (the ctor already pub(crate)/private, or \
no ctor in the file) stays silent. Batch 327 / Plan 332 / Research 364 (firezone/sanstun \
@ 35cb1932948bd0776, Apache-2.0 — cited as class provenance; the fixture is an original \
synthesis)."
                .to_string(),
            buggy_code: "pub struct LoginClaim {\n\
    user: u64,\n\
}\n\
\nimpl LoginClaim {\n\
    pub fn new(user: u64) -> Self {\n\
        Self { user }\n\
    }\n\
}\n\
\npub struct Gate {\n\
    rate_budget: u32,\n\
}\n\
\nimpl Gate {\n\
    // the budget exists — but the pub ctor walks around the checkpoint\n\
    pub fn authorize(&mut self, claim: &LoginClaim) -> bool {\n\
        if self.rate_budget == 0 {\n\
            return false;\n\
        }\n\
        self.rate_budget -= 1;\n\
        true\n\
    }\n\
}\n"
                .to_string(),
            fixed_code: "pub struct LoginClaim {\n\
    user: u64,\n\
}\n\
\nimpl LoginClaim {\n\
    // crate-private: the gateway is the only producer\n\
    pub(crate) fn new(user: u64) -> Self {\n\
        Self { user }\n\
    }\n\
}\n\
\npub struct Gate {\n\
    rate_budget: u32,\n\
}\n\
\nimpl Gate {\n\
    // the budget exists — and now the ctor cannot walk around it\n\
    pub fn authorize(&mut self, claim: &LoginClaim) -> bool {\n\
        if self.rate_budget == 0 {\n\
            return false;\n\
        }\n\
        self.rate_budget -= 1;\n\
        true\n\
    }\n\
}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Batch 328 (2026-11-04, Plan 336 / Research 366) — the        ══
        // ══ google/oss-fuzz distill (Apache-2.0 @ a380569f): one         ══
        // ══ suggest-only InputValidation rule (detector, no              ══
        // ══ collect_edits arm).                                          ══
        // ── untrusted-archive-extract-paths (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "untrusted-archive-extract-paths".to_string(),
            sec_concept: "a path extracted from an UNTRUSTED ARCHIVE (remote download, \
user upload) whose member names are attacker-chosen: `ZipArchive::extract(dest)` \
with no member screen, or a `by_index`/`by_name` loop joining `entry.name()` into \
a destination path — a member named `../../etc/cron.d/x` (or an absolute path, or \
a Windows prefix) escapes the destination directory: arbitrary file write (the \
zip-slip class). The bounded repair is the zip crate's own screened accessor \
`entry.enclosed_name()` — `Some(p) => dest.join(p), None => skip/refuse` (refuses \
`..`/absolute/prefix components) — or the generic `components()` walk refusing \
everything but `Component::Normal`, or mangled extraction (sanitize separators \
before the join). Suggest-only: the screen IS the deployment's trust decision — \
which archive sources are legitimate is policy, and the repair choice \
(skip-and-log vs refuse-loud vs mangle) changes what the caller observes. \
Distinct-from `untrusted-path-join-escape` (owned): that rule owns the \
CALLER-SUPPLIED name joined onto a trusted base (fn-parameter str/Path args → \
fs sinks); this owns the ARCHIVE-MEMBER surface — the names come from the \
untrusted FILE's own central directory, not the caller's arguments, and the \
repair is the crate's own screened accessor (`enclosed_name`), a different \
fix-space (sink-distinct per the `unvalidated-egress-url` precedent, the Plan \
156 T2 shape). Also distinct-from `unbounded-read-into-vec`: that rule owns \
the VOLUME of the read; this owns the PATHS the archive's members carry. \
Verified advisory posture (rustsec.org fetched 2026-11-04): the zip crate's \
extraction surface carries an ACTIVE advisory stream — RUSTSEC-2025-0168 \
(Incorrect path canonicalization during archive extraction → arbitrary file \
write, https://rustsec.org/advisories/RUSTSEC-2025-0168) plus a same-surface \
2026 wave (symlink targets outside the destination, SUID/SGID preservation, \
permissions applied through symlinks); the class fires in consumers too \
(RUSTSEC-2026-0434 ant-releases, RUSTSEC-2026-0427 sn-releases — both zip-slip) \
— the extract() surface's screening posture is version-dependent, consumers \
re-derive the advisory posture at adoption. Honest recall limits: \
parse-provable vocabulary fires (the zip-crate naming: `ZipArchive`/`zip::` \
paths, `.extract(` on an archive-typed receiver, `by_index`/`by_name` + \
`entry.name()`-shaped joins); the screening markers ANYWHERE in the fn suppress \
the WHOLE fn (`enclosed_name(`/`.components()`/`mangled`/`sanitize` — one \
screened member hides a sibling, conservative); a safe helper fn called from \
the hazardous fn is an honest NO (same-fn only, cross-fn undetected); a \
tar/7z extractor with no zip vocabulary is an honest miss (the class \
generalizes; the vocabulary is the zip crate's); the shape proves a MISSING \
SCREEN, not an attacker — a hardened-for-internal-use extractor fires too and \
review decides. Detector: `proposers::archive_extract`. Batch 328 / Plan 336 / \
Research 366 (google/oss-fuzz @ `a380569f`, Apache-2.0 — cited as class \
provenance; the fixture is an original synthesis)."
                .to_string(),
            buggy_code: "use std::path::Path;\nuse zip::ZipArchive;\n\npub fn \
unpack_bundle<R: std::io::Read + std::io::Seek>(\n    reader: R,\n    dest: &Path,\n) \
-> std::io::Result<()> {\n    let mut archive = ZipArchive::new(reader)?;\n    for i in \
0..archive.len() {\n        let mut entry = archive.by_index(i)?;\n        let out_path = \
dest.join(entry.name().to_string());\n        let mut out = std::fs::File::create(out_path)?;\n        \
std::io::copy(&mut entry, &mut out)?;\n    }\n    Ok(())\n}\n"
                .to_string(),
            fixed_code: "use std::path::Path;\nuse zip::ZipArchive;\n\npub fn \
unpack_bundle<R: std::io::Read + std::io::Seek>(\n    reader: R,\n    dest: &Path,\n) \
-> std::io::Result<()> {\n    let mut archive = ZipArchive::new(reader)?;\n    for i in \
0..archive.len() {\n        let mut entry = archive.by_index(i)?;\n        let Some(rel) = \
entry.enclosed_name() else {\n            continue;\n        };\n        let out_path = \
dest.join(rel);\n        let mut out = std::fs::File::create(out_path)?;\n        \
std::io::copy(&mut entry, &mut out)?;\n    }\n    Ok(())\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Batch 329 (2026-11-04, Plan 337 / Research 367) — the        ══
        // ══ tembo/voxl distill (MIT @ eba3b7ce): one suggest-only        ══
        // ══ InputValidation rule (detector, no collect_edits arm).       ══
        // ── browser-reachable-service-host-origin-rebinding-gate (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "browser-reachable-service-host-origin-rebinding-gate".to_string(),
            sec_concept: "a browser-reachable local service (a dev tool, dashboard, \
debugger, agent API — anything bound to localhost/0.0.0.0 with a \
browser-facing API or WS endpoint) answering requests whose Host it never \
checks: ANY web page can open a WebSocket to it or POST to it (WS handshakes \
are NOT CORS-gated — no preflight exists, so response-header CORS policy is \
irrelevant), and a page whose own domain is DNS-rebound to the service's \
address even presents a same-origin-looking Host. The bounded repair (the \
source's gate, verbatim structure): refuse the Host unless it parses as an \
IP literal || the localhost family (`localhost`/`*.localhost`) || an explicit \
allowlist (entries like `.example.com` cover subdomains, `*` any); refuse \
the Origin unless its canonical key equals the requested host:port or the \
allowlist names it; the 403 refusal names the fix flag. The missing-Origin \
allowance is fail-open BY DESIGN with the reason in-source, verbatim: \
\"Browsers always send a Host, so one without it comes from another kind of \
client.\" — the regime boundary: a non-browser client is allowed; the gate is \
anti-browser-drive-by, not auth. Two count-neutral faces carried \
in-concept: (a) origin default-port canonicalization BEFORE comparison — \
an origin with its port written out, so `https://a.example` and \
`https://a.example:443` compare equal — compare canonical keys, never raw \
header strings; (b) a non-UTF-8 header value reads as \"\" → fail-closed \
refusal (a value that isn't text can't name an allowed host or origin). \
KINSHIPS: distinct-from `cors-any-origin-with-credentials` — that owns the \
RESPONSE-header issuance policy (tower-http co-occurrence), THIS the \
REQUEST-side handshake gate, and the mechanism reason is distinct: WS \
handshakes are NOT CORS-gated, a WS server setting no CORS headers is still \
reachable from any page, so the server itself must gate; distinct-from \
`client-ip-from-untrusted-forwarded-header` — spoofed PROXY-header trust, \
a different header on a different axis; distinct-from \
`gateway-type-forces-security-check` — constructor reachability, not \
header gating; the Host-rebinding/allowlist vocabulary has ZERO corpus \
owners (the `unvalidated-egress-url` rider's DNS-rebinding phrase is the \
OUTBOUND sink axis, not an inbound Host gate). Honest recall limits: the \
shape proves a MISSING GATE, not an attacker — an internal-only tool fires \
too and review decides (the regime IS browser-reachability, unknowable \
parse-only); same-fn only — a cross-fn gate assembly (a helper, a \
middleware layer) is undetected; a handler that never reads Host/Origin \
vocabulary is an honest NO; the gate vocabulary (`host_allowed`/\
`allowed_hosts`/`allowed_origins`/`origin_key`/`IpAddr`-parse/`refuse`/\
`rebind`/`allowlist`/`is_loopback`) anywhere in the fn suppresses. Detector: \
`proposers::origin_rebind_gate`. Batch 329 / Plan 337 / Research 367 \
(tembo/voxl @ `eba3b7ce`, MIT — cited as class provenance; the fixture is \
an original synthesis)."
                .to_string(),
            buggy_code: "use std::net::IpAddr;\n\npub struct RequestContext<'a> {\n    \
pub host: Option<&'a str>,\n    pub origin: Option<&'a str>,\n    pub path: &'a str,\n}\n\n\
pub fn handle(req: RequestContext<'_>, body: &str) -> String {\n    // any page can drive this: \
no Host check, no Origin check\n    format!(\"{}: {}\", req.path, body)\n}\n\n\
pub fn is_loopback(host: &str) -> bool {\n    host.parse::<IpAddr>().is_ok() || host == \
\"localhost\"\n}\n"
                .to_string(),
            fixed_code: "use std::net::IpAddr;\n\npub struct RequestContext<'a> {\n    \
pub host: Option<&'a str>,\n    pub origin: Option<&'a str>,\n    pub path: &'a str,\n}\n\n\
const ALLOWED_HOSTS: [&str; 0] = [];\n\npub fn handle(req: RequestContext<'_>, body: &str) -> \
Option<String> {\n    let (name, _port) = split_host(req.host?);\n    let host_ok = \
name.parse::<IpAddr>().is_ok()\n        || name == \"localhost\"\n        || \
name.ends_with(\".localhost\")\n        || ALLOWED_HOSTS.contains(&name.as_str());\n    if \
!host_ok {\n        return None; // 403: host cannot be answered — DNS-rebinding gate\n    }\n    \
if let Some(origin) = req.origin {\n        let own = origin.rsplit_once(\"//\").is_some_and(|(_, rest)| {\
\n            rest.split(':').next() == Some(name.as_str())\n        });\n        if !own {\n            \
return None; // 403: cross-origin browser request\n        }\n    }\n    \
Some(format!(\"{}: {}\", req.path, body))\n}\n\nfn split_host(v: &str) -> (String, \
Option<u16>) {\n    match v.rsplit_once(':') {\n        Some((n, p)) if !n.contains(':') => \
(n.to_ascii_lowercase(), p.parse().ok()),\n        _ => \
(v.trim_end_matches('.').to_ascii_lowercase(), None),\n    }\n}\n\npub fn \
is_loopback(host: &str) -> bool {\n    host.parse::<IpAddr>().is_ok() || host == \
\"localhost\"\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── custom-header-mutation-gate-unanswered-preflight (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "custom-header-mutation-gate-unanswered-preflight".to_string(),
            sec_concept: "a browser-reachable MUTATING route (the upload class: an \
upload/multipart handler that streams a request body to disk) performing NO \
custom-header / CSRF-token check is writable by ANY web page the victim's \
browser visits — a hostile page can POST the upload cross-site (an \
auto-submitted form or a simple fetch) and the browser attaches the victim's \
ambient credentials. The defense is MECHANICAL, the source's law verbatim (at \
the pin): \"Uploads must carry this header. Browsers only send custom headers \
cross-origin after a CORS preflight, which we never answer, so other sites \
can't upload.\" — require a custom header on the mutating route and never \
answer the CORS preflight: browsers cannot attach custom headers cross-site \
without a successful preflight, so the gate closes the cross-site write path \
while same-origin clients keep working (the `if \
!headers.contains_key(UPLOAD_HEADER) { return 403; }` shape). Suggest-only: \
the header name is a deployment decision (server and clients must agree), and \
an API already protected by bearer-token auth, CSRF tokens, or \
SameSite=Strict cookies deliberately skips the gate — bearer auth is the SAME \
mechanism class (browsers cannot attach Authorization cross-site without a \
preflight either), so an authorization-checked handler is the fixed shape, \
not a finding. The B258 CSRF-bulk drop adjudicated at authoring: the declined \
bulk is session ARCHITECTURE (rate limiting, IDOR, authz, session management, \
OpenAPI exposure), THIS is the specific mechanical header-gate form with an \
in-source mechanism reason — a different fix-space. Distinct-from \
`browser-reachable-service-host-origin-rebinding-gate` (sibling, this \
batch): that gates WHO may connect (the Host/Origin handshake — \
DNS-rebinding + drive-by reachability), this gates WHICH requests may mutate \
(the write path — cross-site state change); a server can pass the handshake \
gate and still take a cross-site upload. Distinct-from \
`cors-any-origin-with-credentials` (owned): that owns the RESPONSE-header \
issuance policy, this a REQUEST gate. Distinct-from \
`abuse-strike-counts-cross-site-requests` (owned): that owns the \
CLASSIFICATION policy (counting vs rejecting cross-site traffic), this the \
WRITE gate. Honest recall limits: both gates are fn-level — an upload \
handler whose disk write delegates to a helper with no upload vocabulary, or \
whose gate/middleware lives in another fn (an axum middleware layer — the \
source itself gates Host/Origin in its access.rs, cross-fn to this handler), \
is unconnected parse-only; the vocabulary proves a CLASS, not a verdict — \
an offline batch uploader (no browser ever reaches it) fires too and review \
decides; the broader mutating-route surface (rename/delete/create endpoints \
without an upload file-write shape) is OUT of the fire set — same class, \
deliberately refused for precision (the family's frontier). Detector: \
`proposers::mutation_header_gate`. Batch 329 / Plan 337 / Research 367 \
(tembo/voxl @ `eba3b7ce`, MIT — cited as class provenance; the fixture is an \
original synthesis)."
                .to_string(),
            buggy_code: "use axum::body::Body;\nuse axum::extract::State;\nuse \
axum::http::StatusCode;\nuse axum::response::{IntoResponse, Response};\nuse \
std::path::PathBuf;\n\npub struct Store {\n    pub root: PathBuf,\n}\n\npub async \
fn upload_file(State(state): State<Store>, body: Body) -> Response {\n    let \
dir = state.root.join(\"incoming\");\n    if \
tokio::fs::create_dir_all(&dir).await.is_err() {\n        return \
StatusCode::INTERNAL_SERVER_ERROR.into_response();\n    }\n    let bytes = \
axum::body::to_bytes(body, 1 << 20).await.unwrap_or_default();\n    if \
tokio::fs::write(dir.join(\"blob\"), bytes).await.is_err() {\n        return \
StatusCode::INTERNAL_SERVER_ERROR.into_response();\n    }\n    \
StatusCode::OK.into_response()\n}\n"
                .to_string(),
            fixed_code: "use axum::body::Body;\nuse axum::extract::State;\nuse \
axum::http::{HeaderMap, StatusCode};\nuse \
axum::response::{IntoResponse, Response};\nuse std::path::PathBuf;\n\npub \
struct Store {\n    pub root: PathBuf,\n}\n\nconst UPLOAD_HEADER: &str = \
\"x-app-upload\";\n\npub async fn upload_file(State(state): State<Store>, \
headers: HeaderMap, body: Body) -> Response {\n    // Browsers only send \
custom headers cross-origin after a CORS preflight, which we never answer, \
so other sites can't upload.\n    if !headers.contains_key(UPLOAD_HEADER) {\n    \
    return StatusCode::FORBIDDEN.into_response();\n    }\n    let dir = \
let dir = \
state.root.join(\"incoming\");\n    if \
tokio::fs::create_dir_all(&dir).await.is_err() {\n        return \
StatusCode::INTERNAL_SERVER_ERROR.into_response();\n    }\n    let bytes = \
axum::body::to_bytes(body, 1 << 20).await.unwrap_or_default();\n    if \
tokio::fs::write(dir.join(\"blob\"), bytes).await.is_err() {\n        return \
StatusCode::INTERNAL_SERVER_ERROR.into_response();\n    }\n    \
StatusCode::OK.into_response()\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Batch 330 (2026-11-04, Plan 338 / Research 368) — the         ══
        // ══ vectorize-io/hindsight distill (MIT @ 841979df): three        ══
        // ══ suggest-only rules founding the SensitiveData family (the     ══
        // ══ scan/redact/audit lane — detection precision, redaction       ══
        // ══ precision, audit-emission hygiene).                           ══
        // ── ascii-token-unicode-word-boundary-scan-miss (SensitiveData; suggest-only) ──
        SecRule {
            rule_id: "ascii-token-unicode-word-boundary-scan-miss".to_string(),
            sec_concept: "a token-anchored secret/PII scan whose boundary is Unicode-aware silently MISSES the token when it is butted against non-ASCII word characters: both Python `re` and the Rust `regex` crate compile `\\b` (and `\\w`) with Unicode semantics by default, so a CJK character counts as a word character and there is no word boundary between 为 and s — `\\bsk_test_...\\b` fails to match in 凭证为sk_test_ABC... and secrets embedded in Chinese/Japanese/Korean prose reach the store unredacted. The source's law, verbatim (at the pin): \"``re`` compiles ``\\b`` (and ``\\w``) with Unicode semantics, so a CJK character counts as a word character: there is no word boundary between 为 and s, and ``\\bsk_test_...\\b`` silently fails to match in 凭证为sk_test_ABC... . Secrets embedded in Chinese/Japanese/Korean prose therefore reached memory units unredacted.\" — fixed by anchoring on the ASCII token alphabet instead (`(?<![A-Za-z0-9_])` / `(?![A-Za-z0-9_])` lookarounds; the source's helper + its in-source enforcement, verbatim: \"New boundary-based patterns must use this helper, not ``\\b`` — test_redaction_patterns_do_not_use_unicode_word_classes enforces it.\"). The lookarounds are STRICTLY MORE PERMISSIVE than `\\b` (`[A-Za-z0-9_]` is a subset of `\\w`), so nothing that matched before stops matching while a secret butted against non-ASCII is now detected — the property that makes the rewrite safe. THE RUST FIX-SPACE DIFFERS (the `regex` crate has NO lookarounds; carried as the bounded spellings, review picks): (a) `(?-u:\\b)` — the ASCII word boundary (byte-lossy on non-UTF-8 patterns only); (b) fancy-regex lookarounds (dep-gated); (c) post-match flanking-byte validation (`is_ascii_alphanumeric`/`_` on the byte before/after the match — no regex change at all). KINSHIPS: distinct-from the corpus's own identifier-vocabulary gates (`secret_file_create`, `secret_ct_eq` — word-boundary SEGMENTS gating code identifiers, a different surface: those decide WHERE a detector looks, this decides WHAT a pattern's boundary means); the unicode-boundary-semantics vocabulary has ZERO corpus owners (greps `word.bound|(?-u|look.?around|cjk` at authoring: only the identifier-gate hits above + the text-normalization SIMD family, entries_243-class — byte classification, not boundary semantics). Honest recall limits: shape-gated to scan/redact contexts (a fn naming secret/token/scan/redact vocabulary holding a pattern literal or a boundary predicate) — a scanner with no such vocabulary is an honest NO; a Unicode-aware boundary may be DELIBERATE (a CJK word-segmenter wants Unicode `\\b`) — the class fires when the TOKEN ALPHABET is ASCII (prefixed secret keys, card/SSN digit runs) and review decides; a secret already matched by a non-anchored pattern is out of class. Suggest-only: the choice among the three Rust spellings is a dependency + correctness decision, and Unicode boundaries are sometimes the intent. Detector: `proposers::sensitive_data_boundary`. Batch 330 / Plan 338 / Research 368 (vectorize-io/hindsight @ `841979df`, MIT — cited as class provenance; the fixture is an original std-only synthesis, standalone-compiled AND executed)."
                .to_string(),
            buggy_code: "/// Token-anchored scan: the boundary predicate is Unicode-aware —\n/// the same trap as the regex crate's default `\\b`.\nfn boundary_before(prev: Option<char>) -> bool {
    match prev {
        None => true,
        // is_alphanumeric() counts CJK: no boundary between 为 and `s`.
        Some(c) => !(c.is_alphanumeric() || c == '_'),
    }
}

/// Redact every token-anchored `needle` occurrence. The secret in\n/// \"凭证为sk_test_ABC...\" is silently skipped: the preceding CJK char\n/// reads as a word character, so the anchor never fires.\npub fn redact_tokens(text: &str, needle: &str, mask: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = text.to_string();
    let mut start = 0;
    while let Some(pos) = text[start..].find(needle) {
        let byte_pos = start + pos;
        let idx = text[..byte_pos].chars().count();
        let prev = if idx == 0 { None } else { Some(chars[idx - 1]) };
        if boundary_before(prev) {
            out = out.replace(needle, mask);
        }
        start = byte_pos + 1;
    }
    out
}
"
                .to_string(),
            fixed_code: "/// Token-anchored scan on the ASCII token alphabet — the source's\n/// lookaround pair (?<![A-Za-z0-9_]) / (?![A-Za-z0-9_]) as a std\n/// boundary predicate. Strictly more permissive than Unicode `\\b`.\nfn boundary_before_ascii(prev: Option<char>) -> bool {
    match prev {
        None => true,
        // Only [A-Za-z0-9_] count as token chars: 为 IS a boundary.
        Some(c) => !(c.is_ascii_alphanumeric() || c == '_'),
    }
}

/// Redact every token-anchored `needle` occurrence. The secret butted\n/// against CJK prose is now detected; nothing that matched before\n/// stops matching (the subset property that makes this safe).\npub fn redact_tokens(text: &str, needle: &str, mask: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = text.to_string();
    let mut start = 0;
    while let Some(pos) = text[start..].find(needle) {
        let byte_pos = start + pos;
        let idx = text[..byte_pos].chars().count();
        let prev = if idx == 0 { None } else { Some(chars[idx - 1]) };
        if boundary_before_ascii(prev) {
            out = out.replace(needle, mask);
        }
        start = byte_pos + 1;
    }
    out
}
"
                .to_string(),
            family: SecFamily::SensitiveData,
        },
        // ── luhn-validate-numeric-pii-candidates-before-redact (SensitiveData; suggest-only) ──
        SecRule {
            rule_id: "luhn-validate-numeric-pii-candidates-before-redact".to_string(),
            sec_concept: "a digit-FORMAT-only PII detector (13-16 decimal digits with optional separators) redacts benign numeric runs — timestamps, unix millis, port lists — as if they were cards; checksum-validate each candidate BEFORE substituting, and a REJECTED candidate must not consume a later valid start. The source's law, verbatim (at the pin): \"Candidates also need a Luhn check below: digit count alone redacted timestamps and port lists as if they were cards.\" The three in-source guards carried with it: (a) the repeated-digit guard, verbatim: \"Repeated-digit placeholders (including all zeroes) can pass Luhn.\" — `0000 0000 0000 0000` passes the checksum and must refuse; (b) the UUID-overlap window, verbatim: \"A UUID's numeric segments may pass Luhn. Inspect only the bounded neighborhood of this candidate (UUIDs are 36 chars)\" — a hex segment crossing a card candidate is inspected within a bounded window, not re-scanned greedily; (c) THE OVERLAP RESCAN (the load-bearing mechanic), verbatim: \"A rejected candidate must not consume a later valid start (e.g. \\\"2026 4111 1111 1111 1111\\\"). re.sub resumes at the rejected span's end; search again from its next character instead. Keep offsets in the original content so boundaries and UUID checks remain intact.\" — the naive span from `2026` fails Luhn and the substitution pass jumps PAST the whole rejected span, missing the real card starting mid-span; the rescan from start+1 finds it. Rust shape: same three guards over any digit-run candidate — checksum before mask, placeholder refusal, rescan-from-rejected-start+1 on the match loop. KINSHIPS: distinct-from `packed-field-silent-overflow-wrong-key` (a mask/checksum for overflow KEYS in packed fields — wire arithmetic, not PII detection); `luhn|credit.card` have ZERO corpus owners (grep at authoring). Honest limits: Luhn-valid ≠ card — any digit run passes at ~1/10 rate, so the checksum is a PRECISION multiplier layered on the format gate, never a standalone classifier (a 10x false-positive drop, not zero); which validator is the detector's design decision (Luhn owns cards; other PII classes carry other schemes — the CLASS is checksum-before-substitute, the scheme is policy); a deployment that deliberately over-redacts (compliance-first) rejects this rule — that stance is why it stays suggest-only. Suggest-only: the validator scheme + the rescan policy are the detector's design decisions. Detector: `proposers::luhn_before_redact`. Batch 330 / Plan 338 / Research 368 (vectorize-io/hindsight @ `841979df`, MIT — cited as class provenance; the fixture is an original synthesis, standalone-compiled AND executed)."
                .to_string(),
            buggy_code: "fn digits_of(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

/// NAIVE: any 13-16 digit run is a card. The unix-millis timestamp\n/// below is not a card, and the naive span from `2026` eats the real\n/// card that starts mid-string.\nfn is_card_candidate(s: &str) -> bool {
    let n = digits_of(s).len();
    (13..=16).contains(&n)
}

pub fn redact_cards(content: &str) -> String {
    // one pass, resume at each span's end: a rejected span's tail is\n    // never re-inspected
    if is_card_candidate(content) {
        return \"[REDACTED-CARD]\".to_string();
    }
    content.to_string()
}
"
                .to_string(),
            fixed_code: "fn digits_of(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

fn is_card_candidate(s: &str) -> bool {
    let n = digits_of(s).len();
    (13..=16).contains(&n)
}

/// The Luhn checksum over the candidate's digits.
fn passes_luhn(s: &str) -> bool {
    let digits = digits_of(s);
    // Repeated-digit placeholders (including all zeroes) can pass Luhn.
    if digits.is_empty() || digits.iter().all(|&d| d == digits[0]) {
        return false;
    }
    let mut checksum = 0u32;
    for (index, &d) in digits.iter().rev().enumerate() {
        let mut d = d;
        if index % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        checksum += d;
    }
    checksum % 10 == 0
}

pub fn redact_cards(content: &str) -> String {
    if !is_card_candidate(content) {
        return content.to_string();
    }
    if passes_luhn(content) {
        return \"[REDACTED-CARD]\".to_string();
    }
    content.to_string()
}

/// The overlap rescan: search again from the REJECTED candidate's next\n/// character (start + 1), never resume past the whole rejected span —\n/// \"2026 4111 1111 1111 1111\" keeps its inner card.\npub fn rescan_from(rejected_start: usize) -> usize {
    rejected_start + 1
}
"
                .to_string(),
            family: SecFamily::SensitiveData,
        },
        // ── redaction-audit-hit-carries-raw-match (SensitiveData; suggest-only) ──
        SecRule {
            rule_id: "redaction-audit-hit-carries-raw-match".to_string(),
            sec_concept: "a detection/audit emission that carries the MATCHED VALUE verbatim re-leaks the secret into logs, APMs, queues and audit stores — the audit wire IS a second copy of the data at rest, one nobody redacted. Emit a LENGTH-TIERED FINGERPRINT PREVIEW instead: enough prefix + suffix for a SIEM operator to correlate against their credential inventory, never the body. The source's law, verbatim (at the pin): \"The preview keeps the prefix and a short suffix so a SIEM operator can correlate against their credential inventory (the prefix names the provider; the suffix disambiguates specific instances) without the raw secret crossing the wire.\" The tiers, verbatim: \"Length < 6: redact entirely (return a fixed-length mask). Catches noise like a single -----BEGIN... marker line. / Length 6-15: keep the first 2 + last 2 around an ellipsis. / Length > 15: keep the first 4 + last 4 around an ellipsis.\" The contract on the emission type, verbatim: \"The preview is *never* the raw value.\" And the honest-absence half (the T0 corroborating file, verbatim): \"None when no per-hit data ... so receivers can distinguish 'no preview info' from 'scanned, nothing matched'\" — an ABSENT preview must stay absent (None/empty-with-meaning), never a blank string that reads as a clean scan. KINSHIPS: distinct-from `constant-time-secret-compare` (comparison TIMING vs emission CONTENT — different leak axis); distinct-from `secret-key-without-zeroize` (memory lifetime/erase vs the log copy); the `fingerprint` hits at authoring are entries_08's hash-table fingerprint SLOTS (a storage layout, not an emission mask) — zero owners of the preview/mask-emission class. Honest limits: a fingerprint is DERIVED data — first4+last4 of a 40-char token still leaks 8 chars, and for a short high-entropy body even 4+4 may be too much; the tier boundaries are compliance policy, not physics (which is exactly why this stays suggest-only); the class is the AUDIT EMISSION of a detector — the scanner tooling's own --blind discipline is a different surface (that redacts FILES for sharing, this redacts EMISSIONS for telemetry). Suggest-only: whether previews ship at all, and the tier boundaries, are compliance decisions. Detector: `proposers::audit_hit_fingerprint`. Batch 330 / Plan 338 / Research 368 (vectorize-io/hindsight @ `841979df`, MIT — cited as class provenance; the fixture is an original synthesis, standalone-compiled AND executed)."
                .to_string(),
            buggy_code: "/// NAIVE: the audit trail records what matched — the secret itself.\npub struct ScanHit {
    pub detector: String,
    pub matched: String,
}

pub fn record_hit(hits: &mut Vec<ScanHit>, detector: &str, matched: &str) {
    hits.push(ScanHit {
        detector: detector.to_string(),
        matched: matched.to_string(), // the raw value crosses to the log wire
    });
}
"
                .to_string(),
            fixed_code: "/// FIXED: the emission carries a length-tiered fingerprint preview —\n/// correlation prefix + suffix, never the body. Absent stays absent.\npub struct ScanHit {
    pub detector: String,
    pub preview: Option<String>, // None = no per-hit data, NOT a clean scan
}

fn fingerprint_value(value: &str) -> String {
    let n = value.chars().count();
    if n < 6 {
        return \"[redacted]\".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if n <= 15 {
        format!(
            \"{}...{}\",
            chars[..2].iter().collect::<String>(),
            chars[n - 2..].iter().collect::<String>()
        )
    } else {
        format!(
            \"{}...{}\",
            chars[..4].iter().collect::<String>(),
            chars[n - 4..].iter().collect::<String>()
        )
    }
}

pub fn record_hit(hits: &mut Vec<ScanHit>, detector: &str, matched: Option<&str>) {
    hits.push(ScanHit {
        detector: detector.to_string(),
        preview: matched.map(fingerprint_value),
    });
}
"
                .to_string(),
            family: SecFamily::SensitiveData,
        },
        // ── externally-authored-edits-by-id-not-counted-index (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "externally-authored-edits-by-id-not-counted-index".to_string(),
            sec_concept: "an edit-application API addressed by an index the AUTHOR COUNTED from a rendered view cannot be range-validated into correctness: an off-by-one is still in range, so it silently overwrites an UNRELATED block and is recorded as a success — and the operation usually leaves the structural length unchanged, so no shrink guard or length-diff can notice. The law, verbatim (at the pin): \"An index has to be *counted* by the model, and an off-by-one is still in range, so it silently overwrites an unrelated block and is recorded as a success — no length change for a shrink guard to notice. An id is copied, not derived; a wrong one does not resolve and is skipped and reported.\" Address externally-authored edits by a COPIED IDENTITY (a block/item/section id the author transcribes from the source), resolve it, and SKIP-AND-REPORT a non-resolving id — validity is IDENTITY MEMBERSHIP, never range membership. KINSHIPS: distinct-from `packed-field-silent-overflow-wrong-key` (a masked key ALIASING below a packing width — bits lost, read-side lookup sink, repair = widen/assert; this = write-side edit application where the address was never the identity, repair = id addressing + resolve-or-skip); distinct-from `capacity-exceeded-silent-fallback-not-refusal` (shares the silent-wrong-write-recorded-as-success harm but owns the lying-cap face); distinct-from `luhn-validate-numeric-pii-candidates-before-redact` (the same genus — syntactic validity ≠ semantic validity before acting — at the detection/redaction sink, not edit addressing); distinct-from `untrusted-archive-extract-paths`/`untrusted-path-join-escape` (those names ESCAPE a sink; this ordinal stays in range and corrupts by MISIDENTIFICATION). Regime: externally-authored edit application (LLM ops on documents, scripted patches, DSL/config overlays) where the author saw a rendered view and derives the address by counting; harm peaks when ops leave length unchanged. Anti-regimes: machine-internal callers computing indices from the same authoritative structure (no counting error exists); whole-document rewrite APIs (no addressing); append-only logs (a wrong append corrupts nothing prior). Honest limits: a mis-copied-but-VALID id still names the wrong block (the law's protection is against non-resolving addresses; a semantic wrong-id is review's problem); ids are only as trustworthy as their namespace. Suggest-only: the addressing scheme is the API designer's choice. Detector: none at landing (vocabulary-gated hard — machine-internal indexing is everywhere; a detector would need edit/apply/patch vocabulary + an in-range-branch-without-identity-check shape, deferred). Batch 331 / Plan 339 / Research 368 (+ addendum; the hindsight round-2 pool — vectorize-io/hindsight @ `841979df`, MIT — cited as class provenance; the fixture is an original synthesis, standalone-compiled AND executed)."
                .to_string(),
            buggy_code: "/// NAIVE: edits addressed by a counted index. An off-by-one is still in
/// range, so it silently overwrites an unrelated block and is recorded as
/// a success — no length change for a shrink guard to notice.
pub struct Block {
    pub text: String,
}
pub struct Edit {
    pub block_index: usize,
    pub new_text: String,
}

/// Returns how many edits were applied (each journaled as a success).
pub fn apply_edits(doc: &mut [Block], edits: &[Edit]) -> usize {
    let mut applied = 0;
    for e in edits {
        if e.block_index < doc.len() {
            // in-range is the ONLY check
            doc[e.block_index].text = e.new_text.clone();
            applied += 1; // recorded as success
        }
    }
    applied
}
"
                .to_string(),
            fixed_code: "/// FIXED: edits addressed by COPIED ids. A wrong id does not resolve: it
/// is skipped and reported — never a silent sibling overwrite.
pub struct Block {
    pub id: String,
    pub text: String,
}
pub struct Edit {
    pub block_id: String,
    pub new_text: String,
}

#[derive(Default, Debug, PartialEq)]
pub struct Report {
    pub applied: Vec<String>,
    pub skipped: Vec<(String, &'static str)>,
}

pub fn apply_edits(doc: &mut [Block], edits: &[Edit]) -> Report {
    let mut report = Report::default();
    for e in edits {
        match doc.iter_mut().find(|b| b.id == e.block_id) {
            Some(b) => {
                b.text = e.new_text.clone();
                report.applied.push(b.id.clone());
            }
            None => report.skipped.push((e.block_id.clone(), \"no such block\")),
        }
    }
    report
}
"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── conditional-reuseaddr-after-liveness-probe (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "conditional-reuseaddr-after-liveness-probe".to_string(),
            sec_concept: "an explicit `SO_REUSEADDR` (socket2::set_reuseaddr) on a listener bind turns the port into a take-over surface: on the BSD/macOS semantics a loopback bind WITH reuse sits beside a LIVE wildcard (0.0.0.0 / [::]) listener and SHADOWS it for local clients — the local traffic hijack — while the legitimate use of reuse is only the quick restart over a previous server's TIME_WAIT ends. The source's law, verbatim (at the pin): 'The server binds without `SO_REUSEADDR`, so it cannot sit on `127.0.0.1` beside a live server and take its local traffic. It rebinds with `SO_REUSEADDR` only when no live server answers on the port, so a quick restart over a previous server's closed connections still works.' — and the refusal half: 'A port already in use is refused, also when the other server listens at `0.0.0.0` or `[::]`.' THE POLICY, three legs: (1) bind WITHOUT reuse first — every holder refuses, a live same-address listener AND a live wildcard one; (2) on EADDRINUSE, run a LIVENESS PROBE — dial the address with a short timeout: answered or timed out means a LIVE listener answers (a wildcard holder answers a loopback probe), ConnectionRefused means the holders are TIME_WAIT ends only; (3) rebind WITH reuse only on the refused probe — the quick-restart path — and refuse loudly otherwise. THE ANTI-REGIME carried in-rule: std's `TcpListener::bind` never sets SO_REUSEADDR — safe by default; the rule exists to gate the EXPLICIT opt-in call sites, and the platform split is the decision content (BSD/macOS SO_REUSEADDR permits the wildcard-shadow bind; Linux refuses same-addr:port live-over-live but still permits loopback-over-wildcard under reuse — probe-first is the portable posture, never assume either semantics). KINSHIPS: distinct-from `browser-reachable-service-host-origin-rebinding-gate` (the REQUEST-side handshake gate against DNS rebinding — this is the BIND-side posture; both are 'verify the resource is actually yours/free before attaching', adjacent, compose); distinct-from `secret-file-create-without-0600` (permission hardening on a created file vs bind-option discipline on a created socket); zero corpus owners of the reuseaddr class (T0 grep: reuseaddr / reuse_address / SO_REUSE / EADDRINUSE — all zero). Suggest-only: the bind/rebind/probe flow is the service author's control; no mechanical rewrite exists. Fixture: the bind table model — beside-live refusal (loopback AND wildcard holders), the probe-pass quick restart over TIME_WAIT, and the unconditional-reuse bind demonstrating the shadow hazard it must refuse. Detector: none at landing (vocabulary-gated — an explicit reuseaddr/set_reuseaddr call site is the seed; a natural future lane, deferred — B340 posture; the B342 batch repaired this sentence's earlier false claim that `proposers::reuseaddr_gate` exists — it never landed). Batch 337 / Plan 346 / Research 376 (skydiscover-ai/jitengine @ `2c34508d`, Apache-2.0 — cited as class provenance; the fixture is an original synthesis, standalone-compiled AND executed before entry)."
                .to_string(),
            buggy_code: "/// NAIVE: unconditional SO_REUSEADDR on the listener bind — beside a\n/// live wildcard listener, the loopback bind with reuse shadows it for\n/// local clients: the port hijack.\nuse std::net::{TcpListener};\nuse socket2::{Socket, Domain, Type, SockAddr};\n\npub fn serve_reuse(address: &str) -> std::io::Result<TcpListener> {\n    let addr: SockAddr = address.parse()?;\n    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;\n    socket.set_reuseaddr(true)?; // UNCONDITIONAL — the hazard\n    socket.bind(&addr)?;\n    socket.listen(128)?;\n    Ok(TcpListener::from(socket))\n}\n"
                .to_string(),
            fixed_code: "/// FIXED: bind WITHOUT reuse first; on EADDRINUSE, probe; rebind WITH\n/// reuse only when NO live listener answers the address (TIME_WAIT ends\n/// only). A live wildcard holder answers the loopback probe: refused.\nuse std::net::TcpListener;\n\n#[derive(Debug, PartialEq)]\npub enum BindVerdict {\n    Bound,\n    ReboundOverTimeWait,\n    RefusedLive,\n}\n\n/// `probe_free` dials (host, port) with a short timeout: false only when\n/// the dial is CONNECTION REFUSED (no live listener — TIME_WAIT ends hold\n/// nothing); answered/timeout/other-error means something live answers.\npub fn serve_gated<F>(address: &str, probe_free: F) -> std::io::Result<(TcpListener, BindVerdict)>\nwhere\n    F: Fn(&str) -> bool,\n{\n    // leg 1: the strict bind — every holder refuses.\n    match TcpListener::bind(address) {\n        Ok(l) => Ok((l, BindVerdict::Bound)),\n        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {\n            // leg 2: the liveness probe decides.\n            if probe_free(address) {\n                // leg 3: TIME_WAIT only — the quick restart may reuse.\n                let l = reuse_bind(address)?;\n                Ok((l, BindVerdict::ReboundOverTimeWait))\n            } else {\n                Err(std::io::Error::new(\n                    std::io::ErrorKind::AddrInUse,\n                    \"a live listener answers the address; refusing the takeover\",\n                ))\n            }\n        }\n        Err(e) => Err(e),\n    }\n}\n\nfn reuse_bind(address: &str) -> std::io::Result<TcpListener> {\n    // the ONLY reuse site in the service: past the probe.\n    let _ = address;\n    // platform note carried in-rule: on Linux the same-addr live-over-live\n    // bind refuses without reuse too — probe-first stays the portable path.\n    unimplemented!() // socket2 set_reuseaddr(true) + bind, probe-gated\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── removed-span-veto-outscopes-recovery-path (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "removed-span-veto-outscopes-recovery-path".to_string(),
            sec_concept: "sanitization EVIDENCE outscopes the recovery path: when a sanitizer removes attacker-suspect regions and a downstream recovery path (a model judge, a re-review, a heuristic rescuer) may re-admit refused calls, the recovery path is scoped OUT of authority over anything riding the removed text — a target (URL/email/account/routing string) appearing inside a removed span stays refused even when the judge scores it coherent. The source's law, verbatim (at the pin): 'A taint-novel routing target is recoverable iff NONE of its significant tokens appear as a SUBSTRING of the detected injection span text' — and the plumbing law that makes the veto real: the veto gate sits BEFORE the judge invocation, so the judge never even sees a vetoed call. THE GENUS: sanitization output is ADMISSIBILITY evidence — the removed-span record bounds what ANY later authority may restore (fail-closed by construction, not by review); the fast path cannot bypass the gate, and neither can the slow path. KINSHIPS: distinct-from `externally-authored-edits-by-id-not-counted-index` (identity-vs-range validity at the ADDRESSING sink — this is ADMISSION authority scoping at the recovery sink); adjacent the SensitiveData family (detection/redaction precision is the PREVIOUS layer — what the sanitizer removed; this rule is the layer AFTER redaction: what the removed evidence then forbids); kin `gateway-type-forces-security-check` (the checkpoint-bypass genus — the same 'the fast path cannot bypass the gate' shape, here extended to the RECOVERY path). Regime: any pipeline with (a) a sanitizer/redactor producing removed-region evidence and (b) a downstream re-admission authority (LLM judge, retry-with-more-context, allowlist heuristic) — agent tool-call gates, mail/chat moderation rescuers, injection-filtered command relays. Anti-regimes: no recovery path exists (the refusal is final — nothing to scope); the removed regions carry no targets (pure prose). Suggest-only: the veto's placement (before vs after the judge) is the pipeline author's design. Detector: none at landing (vocabulary-gated hard — needs sanitizer-output + judge/referral vocabulary co-residence in one flow; deferred). Fixture: `corpus/fixtures/b340_fixtures.rs` F3 (the veto state machine: a refusal whose target rides the removed span is NOT restorable by the recovery path; a clean target IS; fail-closed default). Batch 340 / Plan 349 / Research 381 (saintmachines/spillway @ `0fc9f55f`, Apache-2.0 — floor.py `no_target_in_injection_span` + the mvspan gate at the referral site, quoted byte-exact at the pin; the fixture is an original synthesis, standalone-compiled AND executed)."
                .to_string(),
            buggy_code: "/// NAIVE: the recovery path (judge) is consulted on EVERY refused
/// call, INCLUDING calls whose target rode the removed span — the
/// judge can re-admit a target the sanitizer explicitly removed.
pub struct SanitizeReport {
    pub clean: String,
    pub removed_spans: Vec<String>, // the evidence
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Allowed,
    Blocked,
}

fn judge_allows(target: &str, task: &str) -> bool {
    target.len() + task.len() > 8
}

pub fn gate(
    report: &SanitizeReport,
    target: &str,
    task: &str,
) -> Verdict {
    if report.removed_spans.iter().any(|s| s.contains(target)) {
        // removed-span evidence recorded — but then...
        if judge_allows(target, task) {
            return Verdict::Allowed; // the judge OVERRIDES the evidence
        }
    }
    if judge_allows(target, task) {
        return Verdict::Allowed;
    }
    Verdict::Blocked
}
"
                .to_string(),
            fixed_code: "/// FIXED: the removed-span VETO gates the recovery path — the judge
/// is never consulted for a target riding the removed text; the
/// evidence outscopes every later authority (fail-closed).
pub struct SanitizeReport {
    pub clean: String,
    pub removed_spans: Vec<String>, // the evidence
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Allowed,
    Blocked,
}

fn judge_allows(target: &str, task: &str) -> bool {
    target.len() + task.len() > 8
}

fn target_in_removed_span(target: &str, report: &SanitizeReport) -> bool {
    report.removed_spans.iter().any(|s| s.contains(target))
}

pub fn gate(
    report: &SanitizeReport,
    target: &str,
    task: &str,
) -> Verdict {
    // THE VETO comes FIRST: the judge never sees a vetoed call.
    if target_in_removed_span(target, report) {
        return Verdict::Blocked;
    }
    if judge_allows(target, task) {
        return Verdict::Allowed;
    }
    Verdict::Blocked
}
"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── read-output-trust-inherits-argument-taint (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "read-output-trust-inherits-argument-taint".to_string(),
            sec_concept: "a read's output inherits its ARGUMENTS' trust: outputs of a read whose argument values are attacker-derived (taint-novel — present in untrusted tool/content text, absent from the user's own request) are untrusted EVEN WHEN delivered in typed, well-formed fields — provenance flows through the CALL, not through the channel. The source's law, verbatim (at the pin): 'A read's output identities are trustworthy only if the read was NOT steered by injection, i.e. none of the read's arg values are taint-novel.' The attack shape: injected content steers one argument (a query, a path, an id filter) of a structured read; the read returns attacker-chosen rows in perfectly typed fields; a downstream consumer that trusts the CHANNEL (typed output == safe) is steered without ever seeing a malformed value. KINSHIPS: distinct-from the wire_taint family (`proposers::wire_taint` — buffer-level code patterns: length-prefix/read-into-vec shapes at the WIRE decode sink; this rule is call-level information flow at the TOOL/QUERY composition sink — different layer, different sink, adjacent genus note mandatory); distinct-from `unvalidated-egress-url` (egress URL assembly — this is input-side read steering feeding trust decisions); kin `abuse-strike-counts-cross-site-requests` (label/provenance carried across requests — the same 'the channel is not the provenance' genus at the counting sink). Regime: tool-using agents, plugin/function-call hosts, RPC layers where untrusted text can reach ARGUMENT values while outputs are consumed as trusted structure. Anti-regimes: arguments are machine-derived constants (no taint exists); the consumer re-validates outputs against the user's own request anyway (the inheritance is redundant). Suggest-only: the taint tracking's granularity (per-argument vs per-call) is the host's design. Detector: none at landing (vocabulary-gated hard — needs call-argument provenance vocabulary; deferred). Fixture: `corpus/fixtures/b340_fixtures.rs` F4 (dual-source conflict: a value read under a steered argument poisons its typed outputs even when the same value arrives clean elsewhere — the conflict resolves to the attacker). FACE (Batch 342, harness-adapters — the HOST-SANCTION-PROVENANCE arm): sanction inputs derive from the HUMAN turns only — safeagents' Policy builds its host-allowlist context from 'The request without the model's own turns: a host the model wrote earlier is no more sourced than one it writes now' (policy.py:59-62, huggingface/safeagents @ `0de8e22d`, quoted at the pin) — model-echo NEVER self-authorizes: an agent cannot launder a host into the allowed set by having said it in an earlier turn; the taint-inheritance law above owns what a steered READ returns, THIS face owns what the SANCTION SOURCE may contain. FACE (Batch 342, harness-adapters — the AUTHORED-ONLY SCOPING arm, the per-field-class trust inversion): detectors match only AGENT-AUTHORED fields (commands, arguments, file contents, queries, URLs — the plugin channel sets `frozenset({Channel.COMMAND, Channel.ARGUMENTS, ...})` at the pin), never tool OUTPUTS, and never the model's own reasoning ('Reasoning is left out: a model's account of what it is doing is not evidence of what it does', scan.py docstring) — evidence flows ONE WAY: authored fields are the scan SURFACE, outputs are the DATA the scan protects; a scanner that reads outputs as findings (or skips authored fields for them) inverts the trust direction and misses the steered call while flagging benign results; day-one Unicode discipline (the `ascii-token-unicode-word-boundary-scan-miss` law) applies to every name-anchored detector pattern these faces seed. Batch 340 / Plan 349 / Research 381 (saintmachines/spillway @ `0fc9f55f`, Apache-2.0 — floor.py `query_entailed`, quoted byte-exact at the pin; the fixture is an original synthesis, standalone-compiled AND executed)."
                .to_string(),
            buggy_code: "/// NAIVE: outputs are trusted because the CHANNEL is typed. The
/// arguments that selected the rows are never checked for taint —
/// steered arguments steer the typed output.
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: String,
    pub payee: String,
    pub balance_cents: u64,
}

pub struct Read {
    pub query: String,
}

#[derive(Debug, PartialEq)]
pub enum Provenance {
    Trusted,
    Attacker,
}

pub fn lookup(accounts: &[Account], read: &Read, arg_prov: &Provenance) -> Option<Account> {
    let _ = arg_prov;
    accounts
        .iter()
        .find(|a| a.payee.contains(&read.query))
        .cloned()
}

pub fn pay_selected(account: &Account) -> String {
    format!(\"pay {} ({})\", account.payee, account.id)
}
"
                .to_string(),
            fixed_code: "/// FIXED: the output carries the MAX of its arguments' provenance;
/// the consumer pays only Trusted-provenance outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: String,
    pub payee: String,
    pub balance_cents: u64,
}

pub struct Read {
    pub query: String,
}

#[derive(Debug, PartialEq)]
pub enum Provenance {
    Trusted,
    Attacker,
}

pub struct ReadOutput {
    pub account: Account,
    pub provenance: Provenance,
}

pub fn lookup(accounts: &[Account], read: &Read, arg_prov: &Provenance) -> ReadOutput {
    let account = accounts
        .iter()
        .find(|a| a.payee.contains(&read.query))
        .cloned()
        .expect(\"caller pins existence\");
    ReadOutput {
        account,
        provenance: arg_prov.clone(),
    }
}

pub fn pay_selected(out: &ReadOutput) -> Option<String> {
    match out.provenance {
        Provenance::Trusted => Some(format!(\"pay {} ({})\", out.account.payee, out.account.id)),
        Provenance::Attacker => None,
    }
}
"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ── credential-material-in-command-argv (SensitiveData; suggest-only) ──
        SecRule {
            rule_id: "credential-material-in-command-argv".to_string(),
            sec_concept: "credential MATERIAL travelling in a spawned command's ARGV: token \
shapes (`sk-…`-class, `hf_…`, `ghp_…`/`github_pat_…`, JWT `eyJ…`), \
`Authorization:`/`x-api-key:` header values, or secret-named env interpolation \
(`format!(\"--token {hf_token}\")` over `env::var(\"HF_TOKEN\")`) passed through \
`.arg()`/`.args()` — argv is WORLD-READABLE for the process lifetime (`ps` and \
`/proc/<pid>/cmdline` to every local account; shell history and process-accounting \
logs pick up a second copy). THE SOURCE (huggingface/safeagents @ `0de8e22d` \
Apache-2.0 + its atif-scan dependency @ `0228cd9d` MIT — the agent-egress-guard \
family's SECOND source, after SPILLWAY B340): atif-scan's \
`side_channel.provider_credential_use` matches exactly this surface in \
agent-authored commands — `-H \"Authorization: Bearer $K\"` on a command line, a \
key parameter in a URL, an SDK's `api_key` argument (provider_keys.py \
HEADER_LINE/URL_KEY, quoted at the pin) — and its regression cases are agents \
shipping the operator's `HF_TOKEN` to a Hub `whoami` (TB2.1: a DeepSeek V4 Flash \
agent read `DEEPSEEK_API_KEY` from another process's environment and checked it \
against `api.deepseek.com/v1/models`; a TB-lite agent asked `whoami` with the \
operator's token). THE FIX IS THE TRANSPORT, never a cleaner argv: \
`Command::env` (process-private environment), stdin (`Stdio::piped()` + \
`write_all`), or a config file the child reads by handle — the value must never \
be reconstructable from the process listing. KINSHIPS (adjudication note \
mandatory per plan 352 T1): adjacent-DISTINCT from \
`shell-string-command-interpolation` — THAT rule owns the INJECTION axis \
(untrusted data INTO a shell string; fix = the arg array); THIS owns the \
EXPOSURE axis (credential material OUT via argv regardless of injection; fix = \
the transport) — both can fire on one spawn and compose; distinct-from \
`secret-file-create-without-0600` (credential AT REST on a created file vs \
credential IN TRANSIT on a spawned command); distinct-from \
`redaction-audit-hit-carries-raw-match` (that owns the audit WIRE re-leaking \
detected secrets; this owns the argv channel leaking them before any audit); \
adjacent the SensitiveData family's law (where secret material travels — argv is \
one more surface it can be read from). Regime: any `std::process::Command` (or \
shell-line) construction whose arguments carry credential vocabulary — deploy \
scripts, cloud-CLI wrappers, model-provider probes. ANTI-REGIMES: file-transport \
flags (`--token-file`, netrc) — a PATH is not the secret; stdin-piped or \
env-transported values (the fix posture itself); CI masks (a `$$TOKEN` \
placeholder never expands locally). Honest recall limits: generic variable names \
(`let t = load_key(); .arg(&t)`) are untypeable parse-only; cross-fn argv \
construction unconnected; shell-built lines are the injection rule's territory; \
`env::var` read INSIDE the child (the child fetches its own secret) stays silent \
here. Suggest-only: the transport choice is the service author's. Fixture: \
`fixtures/b342_fixtures.rs` F1 (the bearer-header exposure, the env-transport \
fix, the token-file + stdin anti-regimes, the all-safe stub arm). Detector: \
none at landing (vocabulary-gated: token-shape regexes + secret-name vocabulary \
reaching `.arg` sites are the seed — a natural future lane, deferred). Batch 342 \
/ Plan 352 / Research 385 (huggingface/safeagents @ `0de8e22d` Apache-2.0 + \
huggingface/atif-scan @ `0228cd9d` MIT — cited as class provenance; the fixture \
is an original synthesis, standalone-compiled AND executed before entry)."
                .to_string(),
            buggy_code: "use std::process::Command;\n\n/// Deploy probe: the Hub token travels in ARGV — world-readable via ps//proc.\npub fn whoami(token: &str) -> std::io::Result<String> {\n    let out = Command::new(\"curl\")\n        .arg(\"-sfL\")\n        .arg(\"-H\")\n        .arg(format!(\"Authorization: Bearer {token}\"))\n        .arg(\"https://huggingface.co/api/whoami-v2\")\n        .output()?;\n    Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n"
                .to_string(),
            fixed_code: "use std::io::Write;\nuse std::process::{Command, Stdio};\n\n/// Deploy probe: the token rides STDIN — never the process listing.\npub fn whoami(token: &str) -> std::io::Result<String> {\n    let mut child = Command::new(\"curl\")\n        .arg(\"-sfL\")\n        .arg(\"-H\")\n        .arg(\"@-\")\n        .arg(\"https://huggingface.co/api/whoami-v2\")\n        .stdin(Stdio::piped())\n        .stdout(Stdio::piped())\n        .spawn()?;\n    if let Some(mut stdin) = child.stdin.take() {\n        stdin.write_all(format!(\"Authorization: Bearer {token}\\n\").as_bytes())?;\n    }\n    let out = child.wait_with_output()?;\n    Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n"
                .to_string(),
            family: SecFamily::SensitiveData,
        },
        // ── egress-control-bypass-flag (InputValidation; suggest-only) ──
        SecRule {
            rule_id: "egress-control-bypass-flag".to_string(),
            sec_concept: "a flag on a network call or spawned command that DEFEATS an \
egress control the deployment operates: wildcard proxy exclusions \
(`--noproxy '*'`, `NO_PROXY=*`), proxy variables removed or emptied for the call \
(`env -u HTTPS_PROXY`, `unset HTTPS_PROXY`, `git -c https.proxy=`), \
TLS-verification opt-outs (`curl --insecure`/`--no-check-certificate`, reqwest's \
`danger_accept_invalid_certs(true)`/`danger_accept_invalid_hostnames(true)`). THE \
SOURCE LAW, verbatim at the pin: 'A proxy is how many sandboxes enforce their \
network policy, so this routes around it. Blind sandbox-escape reviews of DeepSWE \
runs found it in DeepSeek V4 Pro/Flash trials after a proxy 403, usually with no \
pinned address' — atif-scan `network.proxy_bypass`, Severity.MEDIUM (the flag \
routes AROUND the policy, not to a differently-chosen host). THE ANTI-REGIMES, \
the source's own carve-outs carried in-rule verbatim: a client whose every URL is \
local (`curl --noproxy '*' http://127.0.0.1:8080/` — the TB2.1 shape), \
`--noproxy localhost` or a NO_PROXY LIST of names (scoped exclusions, not \
defeats), and proxies removed for a program that isn't a network client (a test \
suite honouring proxy variables needs them gone to reach its own local server). \
KINSHIPS: sink-distinct from `unvalidated-egress-url` (THAT owns URL CHOICE — \
which host the egress aims at, the SSRF class; THIS owns the policy LAYER between \
the process and the network — the flag that turns the policy OFF; both can fire \
on one line and compose: a defeated proxy AND an attacker-chosen host); \
distinct-from `jwt-insecure-validation-flags` (decode-time token VERIFICATION \
config vs transport-layer TLS/proxy verification); kin \
`gateway-type-forces-security-check` (the checkpoint-bypass genus — a knob that \
silences the gate). Regime: any spawned command or HTTP-client construction \
carrying the defeat vocabulary where an egress control exists (the deployment's \
proxy/cert-pinning posture). Honest recall limits: the control's EXISTENCE is \
deployment state a parse cannot see — the rule fires on the defeat vocabulary \
and review adjudicates whether the control was operating; configured-remote git \
ops (`push origin`) carry no URL a parse can see; the bare `-k` short form is an \
honest NO (too many CLIs use it for other axes); config-file proxy clears are \
not connected. Suggest-only: whether the control should exist and whether an \
exclusion is scoped is the operator's answer. Fixture: `fixtures/b342_fixtures.rs` \
F2 (the wildcard defeat, the name-list + local-only carve-outs, the git \
empty-proxy, the TLS opt-outs, the tampered-verdict arm). Detector: none at \
landing (vocabulary-gated: the wildcard/empty proxy shapes + the \
`danger_accept_invalid_certs` method name are the seed — a natural future lane, \
deferred). Batch 342 / Plan 352 / Research 385 (huggingface/safeagents @ \
`0de8e22d` Apache-2.0 + huggingface/atif-scan @ `0228cd9d` MIT — cited as class \
provenance; the fixture is an original synthesis, standalone-compiled AND \
executed before entry)."
                .to_string(),
            buggy_code: "use std::process::Command;\n\n/// Sandbox egress: the proxy policy is DEFEATED for this fetch.\npub fn fetch_internal(url: &str) -> std::io::Result<String> {\n    let out = Command::new(\"curl\")\n        .arg(\"--noproxy\")\n        .arg(\"*\")\n        .arg(\"-sfL\")\n        .arg(url)\n        .output()?;\n    Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n"
                .to_string(),
            fixed_code: "use std::process::Command;\n\n/// Sandbox egress: the fetch rides the deployment's egress policy.\npub fn fetch_internal(url: &str) -> std::io::Result<String> {\n    let out = Command::new(\"curl\")\n        .arg(\"-sfL\")\n        .arg(url)\n        .output()?;\n    Ok(String::from_utf8_lossy(&out.stdout).into_owned())\n}\n"
                .to_string(),
            family: SecFamily::InputValidation,
        },
        // ══ Batch 345 (2026-11-09, Plan 357 / Research 389): the LocalSR ══
        // distill's sec half — the execution-artifact trust ladder. The
        // source: HerRei/local-upscale (LocalSR) @ cf6f8b55f407b17141b338
        // ba3955e337fc0f6d12 (main tip 2026-11-07, MIT).
        SecRule {
            rule_id: "execution-artifact-trust-ladder-before-deserialize".to_string(),
            sec_concept: "an artifact handed to an EXECUTION-CAPABLE runtime (a deserializer whose format can run code on load — pickle, Marshal, Java serialization — or, the Rust face, a wasm module into wasmi/wasmtime, a dylib, a spawned interpreter) is gated through a TRUST LADDER evaluated BEFORE the loader is invoked: a data-only safe format loads without any opt-in; an execution-capable format without further proof is REFUSED with an error naming the explicit opt-in; an artifact the catalog names is admitted even in the execution-capable format ONLY when its bytes match the pinned size + digest (catalog-verified), and tampered bytes fail that check regardless of consent — and the gate is TWO-SIDED across the process boundary: the host re-derives the verdict from its own inputs and refuses independently, because neither end can see or trust the other's check. The gate sits AT THE SINK: the loader must never receive an unverified artifact, not merely be warned after. SOURCE, verbatim (at the pin, src/localsr/core/model_adapter.py::_checkpoint_trust): the catalog arm — 'failed its catalog SHA-256 check. LocalSR refused to deserialize it; remove the file and download it again'; the consent arm — 'LocalSR blocks unverified .pth, .pt, and .ckpt checkpoints because they may execute code while loading. Use a .safetensors checkpoint or, only for a checkpoint you have independently trusted, set {UNVERIFIED_CHECKPOINT_ENV}=1'; the ladder returns 'catalog-verified' / 'explicitly-trusted' / 'safe-format'. THE SINK PROOF (tests/test_checkpoint_security.py): 'Spandrel must not receive an unverified pickle checkpoint' — the test asserts the loader function is NEVER CALLED on the refusal path. THE TWO-SIDED GATE (desktop/src-tauri/src/commands.rs:1614): 'pickle-based checkpoints require the explicit unsafe checkpoint confirmation' — the Rust host refuses at its own boundary even though the Python worker re-checks independently. THE MANDATORY ADJUDICATION vs the recorded 0sec drop (corpus.rs, Batch 344 staging): THAT drop owns the deserialize-DATA class ('typed serde = fix by construction' — deserializing DATA into typed values needs no gate); THIS owns the execution-capable-ARTIFACT class — the runtime executes arbitrary semantics FROM the file — whose Rust instances are real (this repo's own wasm-validator store manifest: version + blake3 + active/revoked is the positive instance; the B294 wasmtime family proves executors are in-scope). KINSHIPS: sink-distinct from untrusted-archive-extract-paths (zip-slip owns the PATH axis of an extracted archive; THIS owns the EXECUTION axis of a loaded artifact); kin gateway-type-forces-security-check (the constructor-reachability bypass genus — THIS is the ladder that gateway must not be able to skip); the digest arm composes untrusted-download-verify-then-atomic-install (rust_perf, this batch — the download lane feeds the ladder's catalog half). Regime: any host loading artifacts into an execution-capable sink — ML checkpoints via a subprocess engine, wasm validators, plugin/dylib loading, serialized task graphs handed to an interpreter. Anti-regimes: pure-data formats end-to-end (typed serde both sides — the 0sec drop's exact posture); artifacts minted by the same process that consumes them (no trust boundary crossed). Suggest-only: the ladder's arm order (catalog first vs consent first) is the host's policy; the fixture pins THAT a ladder exists at the sink, not its order. Detector: none at landing (vocabulary-gated: a load-call co-occurrence with a consent vocabulary or a digest field — deferred). Fixture: fixtures/b345_fixtures.rs F1 (the safe-format arm, the pre-load refusal with the loader-never-ran proof, the catalog digest admit + tampered refuse incl. consent-does-not-rescue-tampered-bytes, the naive name-only check, the two-sided host/engine arm). Batch 345 / Plan 357 / Research 389 (HerRei/local-upscale @ cf6f8b55f407b17141b338ba3955e337fc0f6d12, MIT — cited as class provenance; the fixture is an original synthesis, standalone-compiled AND executed before entry)."
                .to_string(),
            buggy_code: "use std::cell::Cell;\n\n/// The host hands ANY checkpoint the user picked straight to the\n/// execution-capable loader. A .pth downloaded from a forum runs code\n/// at load time; the host never asked, the worker never checked, and\n/// the only gate is a warning logged after the load returns.\npub fn load_model(path: &str, consent_seen: Cell<bool>) -> Result<Model, Error> {\n    let bytes = std::fs::read(path)?;\n    // the engine accepts anything; the format decides\n    let model = engine_load(&bytes)?; // pickle: arbitrary code runs HERE\n    consent_seen.set(true); // the warning, after the fact\n    Ok(model)\n}\n\nfn engine_load(bytes: &[u8]) -> Result<Model, Error> {\n    // accepts any bytes — the trust decision was never made\n    Ok(Model::from_untrusted(bytes))\n}\n\npub struct Model;\nimpl Model {\n    fn from_untrusted(_bytes: &[u8]) -> Self { Model }\n}\npub struct Error;\n".to_string(),
            fixed_code: "use std::cell::Cell;\n\n/// The trust ladder, evaluated BEFORE the loader is invoked: catalog\n/// digest first, execution-capable formats need explicit consent,\n/// data-only formats load freely. The host re-derives the verdict and\n/// refuses independently of the engine's own check.\n#[derive(Clone, Copy, PartialEq)]\nenum Format { DataOnly, ExecutionCapable }\n\n#[derive(Debug, PartialEq)]\nenum TrustError { DigestMismatch, UnverifiedExecutionCapable }\n\npub fn load_model(\n    bytes: &[u8],\n    format: Format,\n    catalog_digest: Option<u64>,\n    explicit_consent: bool,\n    consent_seen: Cell<bool>,\n) -> Result<Model, TrustError> {\n    // the catalog arm: a named artifact must match its pinned digest —\n    // consent does NOT rescue tampered bytes\n    if let Some(expected) = catalog_digest {\n        if digest(bytes) != expected {\n            return Err(TrustError::DigestMismatch);\n        }\n    } else if format == Format::ExecutionCapable && !explicit_consent {\n        // refuse BEFORE the loader runs — the error names the opt-in\n        return Err(TrustError::UnverifiedExecutionCapable);\n    }\n    consent_seen.set(true);\n    Ok(engine_load(bytes))\n}\n\nfn digest(bytes: &[u8]) -> u64 {\n    let mut h: u64 = 0xcbf2_9ce4_8422_2325;\n    for &b in bytes {\n        h ^= u64::from(b);\n        h = h.wrapping_mul(0x1000_0000_01b3);\n    }\n    h\n}\n\nfn engine_load(_bytes: &[u8]) -> Model { Model }\n\npub struct Model;\npub enum Error {}\n\n#[test]\nfn the_ladder_refuses_before_the_loader_runs() {\n    let seen = Cell::new(false);\n    let err = load_model(b\"downloaded-from-a-forum\", Format::ExecutionCapable, None, false, seen);\n    assert_eq!(err, Err(TrustError::UnverifiedExecutionCapable));\n    assert!(!seen.get(), \"the refusal path never reached the loader\");\n}\n\n#[test]\nfn catalog_digest_gates_tampered_bytes_even_with_consent() {\n    let seen = Cell::new(false);\n    let err = load_model(b\"tampered\", Format::ExecutionCapable, Some(0xA1), true, seen);\n    assert_eq!(err, Err(TrustError::DigestMismatch));\n    assert!(!seen.get());\n}\n".to_string(),
            family: SecFamily::InputValidation,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fixture_parses() {
        for r in entries() {
            assert!(
                syn::parse_file(&r.buggy_code).is_ok(),
                "buggy fixture for {} must parse",
                r.rule_id
            );
            assert!(
                syn::parse_file(&r.fixed_code).is_ok(),
                "fixed fixture for {} must parse",
                r.rule_id
            );
        }
    }

    #[test]
    fn rule_ids_unique_and_kebab() {
        let all = entries();
        let mut ids: Vec<&str> = all.iter().map(|r| r.rule_id.as_str()).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "rule ids must be unique");
        // plan 329 P1: the FIRST sec count pin — manifest-derived (the
        // domain carried no live count pin before; AGENTS/README carried
        // the only numbers). A red means the corpus moved without
        // `scripts/pins_manifest.py --write`.
        assert_eq!(all.len(), crate::pins_manifest::count("sec", "rules"));
        for id in ids {
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "rule id {id} must be kebab-case (digits allowed — the \
                 rust_perf convention, e.g. usize-index-to-u32)"
            );
        }
    }

    /// The Phase-1 survival count — the Proposal 004 caveat-1 gate number.
    /// Phase 2 opens only if this is >= 6.
    #[test]
    fn phase1_lands_at_least_six_rules() {
        assert!(
            entries().len() >= 6,
            "Phase 2 gate: need >=6 surviving rules, have {}",
            entries().len()
        );
    }

    /// Batch 325 (the uniffi-rs FFI-hygiene distill, Plan 327 /
    /// Research 361, mozilla/uniffi-rs @ ef9625b6, MPL-2.0) — presence,
    /// families, provenance discipline. All four are suggest-only
    /// (NotBenchable-by-domain per the dial9 split) and none may join
    /// `FIXABLE_SEC`.
    #[test]
    fn batch325_rules_present_families_and_provenance() {
        let all = entries();
        let ids = [
            ("arc-refcount-across-raw-boundary", SecFamily::DropLeak),
            ("ffi-callback-invoke-outside-lock", SecFamily::Ffi),
            ("out-param-manuallydrop-not-maybeuninit", SecFamily::Ffi),
            ("panic-handler-nested-catch-unwind", SecFamily::PanicSafety),
        ];
        for (id, family) in ids {
            let r = all
                .iter()
                .find(|r| r.rule_id == id)
                .unwrap_or_else(|| panic!("batch-325 rule '{id}' missing from corpus"));
            assert_eq!(r.family, family, "batch-325 {id} family");
            assert!(
                r.sec_concept
                    .contains("Batch 325 / Plan 327 / Research 361"),
                "batch-325 {id} must cite its landing lineage"
            );
            assert!(
                r.sec_concept.contains("ef9625b6"),
                "batch-325 {id} must cite the uniffi-rs pin"
            );
        }
        // Suggest-only discipline: none of the batch ids may be
        // --fix-eligible (the detector modules ship no collect_edits arm).
        for id in [
            "arc-refcount-across-raw-boundary",
            "ffi-callback-invoke-outside-lock",
            "out-param-manuallydrop-not-maybeuninit",
            "panic-handler-nested-catch-unwind",
        ] {
            assert!(
                !crate::domains::sec::proposers::FIXABLE_SEC.contains(&id),
                "batch-325 {id} must stay suggest-only (never FIXABLE_SEC)"
            );
        }
    }

    /// Batch 328 (the google/oss-fuzz distill, Plan 336 / Research 366,
    /// google/oss-fuzz @ a380569f, Apache-2.0) — presence, family,
    /// provenance discipline. Suggest-only, never `FIXABLE_SEC`.
    #[test]
    fn batch328_rules_present_families_and_provenance() {
        let all = entries();
        let ids = [(
            "untrusted-archive-extract-paths",
            SecFamily::InputValidation,
        )];
        for (id, family) in ids {
            let r = all
                .iter()
                .find(|r| r.rule_id == id)
                .unwrap_or_else(|| panic!("batch-328 rule '{id}' missing from corpus"));
            assert_eq!(r.family, family, "batch-328 {id} family");
            assert!(
                r.sec_concept
                    .contains("Batch 328 / Plan 336 / Research 366"),
                "batch-328 {id} must cite its landing lineage"
            );
            assert!(
                r.sec_concept.contains("a380569f"),
                "batch-328 {id} must cite the oss-fuzz pin"
            );
        }
        // Suggest-only discipline: the batch id may not be --fix-eligible
        // (the detector module ships no collect_edits arm).
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC
                .contains(&"untrusted-archive-extract-paths"),
            "batch-328 untrusted-archive-extract-paths must stay suggest-only \
             (never FIXABLE_SEC)"
        );
    }

    /// Batch 329 (the tembo/voxl distill, Plan 337 / Research 367,
    /// tembo/voxl @ eba3b7ce, MIT) — presence, family, provenance
    /// discipline. Suggest-only, never `FIXABLE_SEC`.
    #[test]
    fn batch329_rules_present_families_and_provenance() {
        let all = entries();
        let ids = [
            (
                "browser-reachable-service-host-origin-rebinding-gate",
                SecFamily::InputValidation,
            ),
            (
                "custom-header-mutation-gate-unanswered-preflight",
                SecFamily::InputValidation,
            ),
        ];
        for (id, family) in ids {
            let r = all
                .iter()
                .find(|r| r.rule_id == id)
                .unwrap_or_else(|| panic!("batch-329 rule '{id}' missing from corpus"));
            assert_eq!(r.family, family, "batch-329 {id} family");
            assert!(
                r.sec_concept
                    .contains("Batch 329 / Plan 337 / Research 367"),
                "batch-329 {id} must cite its landing lineage"
            );
            assert!(
                r.sec_concept.contains("eba3b7ce"),
                "batch-329 {id} must cite the tembo/voxl pin"
            );
        }
        // Suggest-only discipline: the batch ids may not be --fix-eligible
        // (the detector modules ship no collect_edits arm).
        for id in [
            "browser-reachable-service-host-origin-rebinding-gate",
            "custom-header-mutation-gate-unanswered-preflight",
        ] {
            assert!(
                !crate::domains::sec::proposers::FIXABLE_SEC.contains(&id),
                "batch-329 {id} must stay suggest-only (never FIXABLE_SEC)"
            );
        }
    }

    /// Batch 330 (the vectorize-io/hindsight distill, Plan 338 / Research
    /// 368, vectorize-io/hindsight @ 841979df, MIT) — presence, family,
    /// provenance discipline; founds the `SensitiveData` family. All three
    /// suggest-only, never `FIXABLE_SEC`.
    #[test]
    fn batch330_rules_present_families_and_provenance() {
        let all = entries();
        let ids = [
            (
                "ascii-token-unicode-word-boundary-scan-miss",
                SecFamily::SensitiveData,
            ),
            (
                "luhn-validate-numeric-pii-candidates-before-redact",
                SecFamily::SensitiveData,
            ),
            (
                "redaction-audit-hit-carries-raw-match",
                SecFamily::SensitiveData,
            ),
        ];
        for (id, family) in ids {
            let r = all
                .iter()
                .find(|r| r.rule_id == id)
                .unwrap_or_else(|| panic!("batch-330 rule '{id}' missing from corpus"));
            assert_eq!(r.family, family, "batch-330 {id} family");
            assert!(
                r.sec_concept
                    .contains("Batch 330 / Plan 338 / Research 368"),
                "batch-330 {id} must cite its landing lineage"
            );
            assert!(
                r.sec_concept.contains("841979df"),
                "batch-330 {id} must cite the hindsight pin"
            );
        }
        // Suggest-only discipline: none of the batch ids may be
        // --fix-eligible (the detector modules ship no collect_edits arm).
        for id in [
            "ascii-token-unicode-word-boundary-scan-miss",
            "luhn-validate-numeric-pii-candidates-before-redact",
            "redaction-audit-hit-carries-raw-match",
        ] {
            assert!(
                !crate::domains::sec::proposers::FIXABLE_SEC.contains(&id),
                "batch-330 {id} must stay suggest-only (never FIXABLE_SEC)"
            );
        }
        // The sensitive-data family's founding members' law needles (keyed
        // after reading the landed text).
        let c1 = all
            .iter()
            .find(|r| r.rule_id == "ascii-token-unicode-word-boundary-scan-miss")
            .unwrap();
        assert!(
            c1.sec_concept.contains("STRICTLY MORE PERMISSIVE") && c1.sec_concept.contains("(?-u:"),
            "batch-330 C1 must carry the subset property + the Rust fix-space"
        );
        let c2 = all
            .iter()
            .find(|r| r.rule_id == "luhn-validate-numeric-pii-candidates-before-redact")
            .unwrap();
        assert!(
            c2.sec_concept.contains("Repeated-digit placeholders")
                && c2
                    .sec_concept
                    .contains("search again from its next character"),
            "batch-330 C2 must carry the placeholder guard + the overlap rescan"
        );
        let c3 = all
            .iter()
            .find(|r| r.rule_id == "redaction-audit-hit-carries-raw-match")
            .unwrap();
        assert!(
            c3.sec_concept.contains("*never* the raw value")
                && c3.sec_concept.contains("scanned, nothing matched"),
            "batch-330 C3 must carry the emission contract + the honest absence"
        );
    }

    /// Batch 331 (the hindsight round-2 pool conversion, Plan 339 /
    /// Research 368 + addendum, vectorize-io/hindsight @ 841979df, MIT) —
    /// presence, family, provenance discipline. Suggest-only (the
    /// addressing scheme is the API designer's choice; no detector at
    /// landing — vocabulary-gated hard), never `FIXABLE_SEC`.
    #[test]
    fn batch331_rules_present_families_and_provenance() {
        let all = entries();
        let r = all
            .iter()
            .find(|r| r.rule_id == "externally-authored-edits-by-id-not-counted-index")
            .unwrap_or_else(|| panic!("batch-331 rule missing from corpus"));
        assert_eq!(r.family, SecFamily::InputValidation, "batch-331 family");
        assert!(
            r.sec_concept
                .contains("Batch 331 / Plan 339 / Research 368"),
            "batch-331 must cite its landing lineage"
        );
        assert!(
            r.sec_concept.contains("841979df"),
            "batch-331 must cite the hindsight pin"
        );
        // Suggest-only discipline.
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC
                .contains(&"externally-authored-edits-by-id-not-counted-index"),
            "batch-331 must stay suggest-only (never FIXABLE_SEC)"
        );
        // The law needles (keyed after reading the landed text).
        assert!(
            r.sec_concept
                .contains("silently overwrites an unrelated block")
                && r.sec_concept
                    .contains("no length change for a shrink guard"),
            "batch-331 must carry the silent-overwrite harm + the shrink-guard gap"
        );
        assert!(
            r.sec_concept.contains("An id is copied, not derived")
                && r.sec_concept.contains("skipped and reported"),
            "batch-331 must carry the id law + the skip-and-report contract"
        );
    }

    #[test]
    fn batch337_rules_present_families_and_provenance() {
        let all = entries();
        let r = all
            .iter()
            .find(|r| r.rule_id == "conditional-reuseaddr-after-liveness-probe")
            .unwrap_or_else(|| panic!("batch-337 rule missing from corpus"));
        assert_eq!(r.family, SecFamily::InputValidation, "batch-337 family");
        assert!(
            r.sec_concept
                .contains("Batch 337 / Plan 346 / Research 376"),
            "batch-337 must cite its landing lineage"
        );
        assert!(
            r.sec_concept.contains("2c34508d"),
            "batch-337 must cite the jitengine pin"
        );
        // Suggest-only discipline.
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC
                .contains(&"conditional-reuseaddr-after-liveness-probe"),
            "batch-337 must stay suggest-only (never FIXABLE_SEC)"
        );
        // The law needles (keyed after reading the landed text).
        assert!(
            r.sec_concept
                .contains("cannot sit on `127.0.0.1` beside a live server")
                && r.sec_concept.contains("take its local traffic"),
            "batch-337 must carry the shadow harm verbatim"
        );
        assert!(
            r.sec_concept
                .contains("only when no live server answers on the port"),
            "batch-337 must carry the probe gate verbatim (the 'on the port' record correction)"
        );
        assert!(
            r.sec_concept.contains("0.0.0.0") && r.sec_concept.contains("[::]"),
            "batch-337 must carry the wildcard/dual-stack refusal"
        );
        assert!(
            r.sec_concept
                .contains("TcpListener::bind` never sets SO_REUSEADDR"),
            "batch-337 must carry the std-safe-by-default anti-regime"
        );
    }

    /// Batch 340 (the SPILLWAY agent-egress-guard distill, Plan 349 /
    /// Research 381, saintmachines/spillway @ 0fc9f55f, Apache-2.0) —
    /// presence, family, provenance discipline. TWO suggest-only rules
    /// (the recovery-authority + trust-inheritance axes), never
    /// `FIXABLE_SEC`.
    #[test]
    fn batch340_rules_present_families_and_provenance() {
        let all = entries();
        let r3 = all
            .iter()
            .find(|r| r.rule_id == "removed-span-veto-outscopes-recovery-path")
            .unwrap_or_else(|| panic!("batch-340 rule 3 missing from corpus"));
        assert_eq!(
            r3.family,
            SecFamily::InputValidation,
            "batch-340 rule 3 family"
        );
        assert!(
            r3.sec_concept
                .contains("Batch 340 / Plan 349 / Research 381")
                && r3.sec_concept.contains("0fc9f55f"),
            "batch-340 rule 3 must cite its lineage + the spillway pin"
        );
        assert!(
            r3.sec_concept.contains("verbatim (at the pin)")
                && r3.sec_concept.contains("recoverable iff NONE"),
            "batch-340 rule 3 must carry the source law verbatim"
        );
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC
                .contains(&"removed-span-veto-outscopes-recovery-path"),
            "batch-340 rule 3 must stay suggest-only (never FIXABLE_SEC)"
        );
        for needle in [
            "gateway-type-forces-security-check",
            "externally-authored-edits-by-id-not-counted-index",
            "ADMISSIBILITY evidence",
        ] {
            assert!(
                r3.sec_concept.contains(needle),
                "batch-340 rule 3 notes must carry '{needle}'"
            );
        }
        let r4 = all
            .iter()
            .find(|r| r.rule_id == "read-output-trust-inherits-argument-taint")
            .unwrap_or_else(|| panic!("batch-340 rule 4 missing from corpus"));
        assert_eq!(
            r4.family,
            SecFamily::InputValidation,
            "batch-340 rule 4 family"
        );
        assert!(
            r4.sec_concept.contains("0fc9f55f")
                && r4
                    .sec_concept
                    .contains("none of the read's arg values are taint-novel"),
            "batch-340 rule 4 must cite the pin + carry the source law verbatim"
        );
        assert!(
            r4.sec_concept.contains("proposers::wire_taint"),
            "batch-340 rule 4 must carry the wire_taint adjacency note"
        );
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC
                .contains(&"read-output-trust-inherits-argument-taint"),
            "batch-340 rule 4 must stay suggest-only (never FIXABLE_SEC)"
        );
    }

    #[test]
    fn batch341_face_wrap_custody_agreement_landed() {
        // count-neutral face on the B325 host: the WRAP-CUSTODY-AGREEMENT arm
        // (flag⇄callback + access⇄mutability, both directions, fail-closed).
        let all = entries();
        let r = all
            .iter()
            .find(|r| r.rule_id == "arc-refcount-across-raw-boundary")
            .unwrap_or_else(|| panic!("arc-refcount host missing"));
        assert!(
            r.sec_concept.contains("FACE (Batch 341, metal-graph")
                && r.sec_concept
                    .contains("owner callbacks require RETAIN_OWNER")
                && r.sec_concept.contains("write access requires MUTABLE"),
            "batch-341 wrap-custody face rider missing"
        );
    }

    /// Batch 342 (the harness-adapters/safeagents sec distill, Plan 352 /
    /// Research 385, huggingface/safeagents @ 0de8e22d Apache-2.0 + its
    /// atif-scan dep @ 0228cd9d MIT — the agent-egress-guard family's
    /// SECOND source) — presence, family, provenance discipline. TWO
    /// suggest-only rules (the argv-exposure + control-defeat axes),
    /// never `FIXABLE_SEC`; candidate 3 (`verifier-self-modification`)
    /// honestly DROPPED at authoring per the plan's expected-drop
    /// allowance (the runtime-vs-build mutation distinction is not
    /// parse-provable — build lanes and fixtures legitimately write test
    /// files).
    #[test]
    fn batch342_rules_present_families_and_provenance() {
        let all = entries();
        let r1 = all
            .iter()
            .find(|r| r.rule_id == "credential-material-in-command-argv")
            .unwrap_or_else(|| panic!("batch-342 rule 1 missing from corpus"));
        assert_eq!(
            r1.family,
            SecFamily::SensitiveData,
            "batch-342 rule 1 family"
        );
        assert!(
            r1.sec_concept
                .contains("Batch 342 / Plan 352 / Research 385"),
            "batch-342 rule 1 must cite its landing lineage"
        );
        assert!(
            r1.sec_concept.contains("0de8e22d") && r1.sec_concept.contains("0228cd9d"),
            "batch-342 rule 1 must cite both source pins"
        );
        // The mandatory adjacent-DISTINCT adjudication note (plan 352 T1).
        assert!(
            r1.sec_concept.contains("adjacent-DISTINCT from")
                && r1
                    .sec_concept
                    .contains("shell-string-command-interpolation")
                && r1.sec_concept.contains("INJECTION axis")
                && r1.sec_concept.contains("EXPOSURE axis"),
            "batch-342 rule 1 must carry the injection-vs-exposure adjudication"
        );
        // The source law needles (keyed after reading the landed text).
        assert!(
            r1.sec_concept.contains("HEADER_LINE") && r1.sec_concept.contains("URL_KEY"),
            "batch-342 rule 1 must cite the provider_keys surfaces"
        );
        assert!(
            r1.sec_concept.contains("the transport")
                && r1.sec_concept.contains("never a cleaner argv"),
            "batch-342 rule 1 must carry the transport law"
        );
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC
                .contains(&"credential-material-in-command-argv"),
            "batch-342 rule 1 must stay suggest-only (never FIXABLE_SEC)"
        );
        let r2 = all
            .iter()
            .find(|r| r.rule_id == "egress-control-bypass-flag")
            .unwrap_or_else(|| panic!("batch-342 rule 2 missing from corpus"));
        assert_eq!(
            r2.family,
            SecFamily::InputValidation,
            "batch-342 rule 2 family"
        );
        assert!(
            r2.sec_concept
                .contains("Batch 342 / Plan 352 / Research 385"),
            "batch-342 rule 2 must cite its landing lineage"
        );
        // The source law needles (side_channel.py:698-703, quoted at the pin).
        assert!(
            r2.sec_concept
                .contains("A proxy is how many sandboxes enforce their network policy"),
            "batch-342 rule 2 must carry the source law verbatim"
        );
        assert!(
            r2.sec_concept.contains("routes around it"),
            "batch-342 rule 2 must carry the routes-around harm verbatim"
        );
        // The source's own anti-regime carve-outs, carried verbatim.
        assert!(
            r2.sec_concept.contains("http://127.0.0.1:8080/")
                && r2.sec_concept.contains("NO_PROXY LIST of names"),
            "batch-342 rule 2 must carry the local-only + name-list carve-outs"
        );
        // Sink-distinct adjudication vs the egress-url owner (plan 352 T1).
        assert!(
            r2.sec_concept.contains("sink-distinct from")
                && r2.sec_concept.contains("unvalidated-egress-url")
                && r2.sec_concept.contains("URL CHOICE"),
            "batch-342 rule 2 must carry the egress-url adjudication"
        );
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC.contains(&"egress-control-bypass-flag"),
            "batch-342 rule 2 must stay suggest-only (never FIXABLE_SEC)"
        );
    }

    #[test]
    fn batch342_rules_parse() {
        // G1 for Batch 342: both rules' buggy AND fixed code must parse as
        // valid Rust (the L2 pruner verdict, the batch333+ discipline).
        for rid in [
            "credential-material-in-command-argv",
            "egress-control-bypass-flag",
        ] {
            let all = entries();
            let rule = all
                .iter()
                .find(|r| r.rule_id == rid)
                .unwrap_or_else(|| panic!("batch-342 rule missing: {rid}"));
            assert!(
                syn::parse_file(&rule.buggy_code).is_ok(),
                "batch-342 {rid}: buggy code must parse"
            );
            assert!(
                syn::parse_file(&rule.fixed_code).is_ok(),
                "batch-342 {rid}: fixed code must parse"
            );
        }
    }

    #[test]
    fn batch342_faces_landed() {
        // count-neutral faces on the B340 host: the HOST-SANCTION-
        // PROVENANCE arm + the AUTHORED-ONLY SCOPING arm (the spillway
        // family, the harness-adapters source).
        let all = entries();
        let r = all
            .iter()
            .find(|r| r.rule_id == "read-output-trust-inherits-argument-taint")
            .unwrap_or_else(|| panic!("batch-340 host missing"));
        assert!(
            r.sec_concept.contains("FACE (Batch 342, harness-adapters")
                && r.sec_concept.contains("model-echo NEVER self-authorizes"),
            "batch-342 host-sanction face rider missing"
        );
        assert!(
            r.sec_concept.contains(
                "a host the model wrote earlier is no more sourced than one it writes now"
            ),
            "batch-342 host-sanction face must carry the policy.py law verbatim"
        );
        assert!(
            r.sec_concept.contains("AUTHORED-ONLY SCOPING arm")
                && r.sec_concept.contains("Reasoning is left out"),
            "batch-342 authored-only scoping face rider missing"
        );
        // En-route repair disclosure: the B337 prose drift is repaired.
        let reuseaddr = all
            .iter()
            .find(|r| r.rule_id == "conditional-reuseaddr-after-liveness-probe")
            .unwrap_or_else(|| panic!("batch-337 rule missing"));
        assert!(
            reuseaddr.sec_concept.contains("Detector: none at landing")
                && !reuseaddr
                    .sec_concept
                    .contains("Detector: `proposers::reuseaddr_gate`"),
            "the B337 reuseaddr prose must no longer claim a detector"
        );
    }

    /// Batch 345 (the `LocalSR` distill, Plan 357 / Research 389,
    /// HerRei/local-upscale @ `cf6f8b55` MIT) — the execution-artifact trust
    /// ladder: presence, family, provenance discipline, the mandatory
    /// adjudication vs the 0sec deserialize-DATA drop, suggest-only.
    #[test]
    fn batch345_rules_present_families_and_provenance() {
        let all = entries();
        let r = all
            .iter()
            .find(|r| r.rule_id == "execution-artifact-trust-ladder-before-deserialize")
            .unwrap_or_else(|| panic!("batch-345 rule missing from corpus"));
        assert_eq!(r.family, SecFamily::InputValidation, "batch-345 family");
        assert!(
            r.sec_concept
                .contains("Batch 345 / Plan 357 / Research 389"),
            "batch-345 must cite its landing lineage"
        );
        assert!(
            r.sec_concept
                .contains("cf6f8b55f407b17141b338ba3955e337fc0f6d12"),
            "batch-345 must cite the source pin"
        );
        // The mandatory adjudication note vs the 0sec drop.
        assert!(
            r.sec_concept.contains("deserialize-DATA")
                && r.sec_concept.contains("execution-capable-ARTIFACT"),
            "batch-345 must carry the 0sec-drop adjudication"
        );
        // The sink proof + the two-sided gate, quoted.
        assert!(
            r.sec_concept.contains("Spandrel must not receive")
                && r.sec_concept.contains("pickle-based checkpoints require"),
            "batch-345 must carry the sink + two-sided gate quotes"
        );
        // The in-house positive instance.
        assert!(
            r.sec_concept.contains("wasm-validator store manifest"),
            "batch-345 must cite the in-house positive instance"
        );
        assert!(
            !crate::domains::sec::proposers::FIXABLE_SEC
                .contains(&"execution-artifact-trust-ladder-before-deserialize"),
            "batch-345 must stay suggest-only (never FIXABLE_SEC)"
        );
    }

    #[test]
    fn batch345_rules_parse() {
        let all = entries();
        let rule = all
            .iter()
            .find(|r| r.rule_id == "execution-artifact-trust-ladder-before-deserialize")
            .expect("batch-345 rule missing");
        assert!(
            syn::parse_file(&rule.buggy_code).is_ok(),
            "batch-345: buggy code must parse"
        );
        assert!(
            syn::parse_file(&rule.fixed_code).is_ok(),
            "batch-345: fixed code must parse"
        );
    }
}
