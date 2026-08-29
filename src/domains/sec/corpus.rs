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
//!
//! Phase-1 killed candidates (recorded in the batch research note):
//! `ffi-fnptr-type-unsafe-extern` + `ffi-by-value-no-drop-type`
//! (`improper_ctypes` fires WITH a fix), `ffi-portable-c-alias` (the alias
//! map is platform-dependent — not parse-only-provable),
//! `no-box-leak` + `maybe-uninit-requires-justification` (no bounded fix;
//! clippy owns the direct-chain uninit form at error level).

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
                id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
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
