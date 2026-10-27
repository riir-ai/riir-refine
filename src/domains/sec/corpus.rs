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
(the Batch-258 rider). Batch 140 / Research 161 / Plan 120 (rustsec/advisory-db @ b50980aa, CC0 — cited \
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
cross-fn joins undetected. Batch 258 / Plan 262 / Research 292 \
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
}
