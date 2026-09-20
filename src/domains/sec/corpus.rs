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
listener accept()/incoming() children, connect()/reqwest::get() results) — a bare `R: Read` \
generic is an honest NO; File readers are the trusted regime and never fire; cross-fn drains \
undetected. Batch 140 / Research 161 / Plan 120 (rustsec/advisory-db @ b50980aa, CC0 — cited \
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
