//! Stage 2 proposal mechanism for the `sec` domain (Plan 018 T1.3) —
//! per-rule `syn`-level rewrites, mirroring the `rust_perf` proposers.
//!
//! Four of the six Phase-1 rules carry proposers. The other two are
//! suggest-only BY DESIGN:
//!
//! | Rule | Proposer | Why |
//! |---|---|---|
//! | `ffi-extern-fn-catch-unwind` | ✓ | signature-preserving wrap |
//! | `ptr-deref-null-check-first` | ✓ | prologue-only, signature-preserving |
//! | `drop-impl-no-panic` | ✓ | method-segment swap, compile-gated |
//! | `slice-from-raw-parts-len-guard` | ✓ | expression-local additive guard |
//! | `extern-fn-reference-param-to-pointer` | ✗ | signature-changing — the C-side caller ripples (the `static-str` caller-gate class, but the caller is in ANOTHER LANGUAGE; refuse outright) |
//! | `rc-backedge-field-prefer-weak` | ✗ | field-type change — every constructor site breaks; unbounded |
//!
//! ## Phase 2 (Batch 88) — one proposer joins
//!
//! | Rule | Proposer | Why |
//! |---|---|---|
//! | `unsafe-impl-send-sync-unsound-field` | ✗ | the fix is a type-level redesign (Arc/Mutex swap) — every constructor/usage site ripples |
//! | `ffi-export-panicking-body` | ✗ | the fix needs a caller-chosen sentinel value — a semantic decision, never mechanical |
//! | `weak-hash-to-blake3` | ✗ | output TYPE changes (`[u8; N]` → `blake3::Hash`) — signature-changing |
//! | `password-hash-argon2` | ✗ | argon2 is a salted KDF with its own API — a redesign, not a swap |
//! | `constant-time-secret-compare` | ✓ | `==` → `.ct_eq(..).into()` is signature-preserving (bool → bool), strict-gated (subtle import + plain-ident operands + secret vocabulary) |
//! | `nan-unsafe-sort-comparator` (P8) | ✓ | `partial_cmp(..).unwrap_or(Equal)` → `total_cmp(..)` is order-identical on non-NaN and NaN-deterministic after; strict-gated (sort context + exact chain + `Equal`-only default + comment guard); non-float receivers fail E0599 → the `--verify` gate reverts |
//!
//! ## Conservative gates (Proposal 004 caveat 3)
//!
//! A wrong security "fix" is worse than no fix. Every proposer refuses
//! more than it fires: signature-changing rewrites are refused outright
//! (not merely caller-gated — the FFI ripple is cross-language), guards
//! already present suppress the rule, and every applied fix rides the
//! `--fix --verify` compile gate.
//!
//! ## Batch 131 (2026-09-08) — the suggest-only detector pair
//!
//! The `InputValidation` family opens with two rules that ship DETECTORS,
//! not fixers: parse-only detection walks with zero-FP negative suites,
//! deliberately absent from `FIXABLE_SEC` and from `collect_edits`
//! (suggest emission only, the `ffi-export-panicking-body` shape — these
//! rules emit suggestions, never auto-fixes):
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `length-prefix-validate-before-alloc` | `length_prefix_alloc::detect` | the fix must invent a limit constant + an error-return path the fn may not have |
//! | `secret-file-create-without-0600` | `secret_file_create::detect` | hardening semantics are per-OS (`Permissions::from_mode` maps coarsely on Windows) — a portable rewrite is judgment-bearing |
//!
//! ## Batch 140 (2026-09-09) — the advisory-db distill joins the family
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `unbounded-read-into-vec` | `unbounded_read::detect` | the fix must CHOOSE the limit (bytes? messages? wall time?) — a protocol decision, never mechanical |
//!
//! ## Batch 165 (2026-09-20, Plan 156) — the overflow sink on the shared walker
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `unchecked-wire-arithmetic` | `unchecked_wire_arithmetic::detect` | the fix is a checked op + an INVENTED error path the fn's signature may not carry — a protocol decision; sink-level novelty over `length_prefix_alloc` on the shared `wire_taint` walker (Plan 156 T2: sink-distinct, no composition gate — the derived-validated shape's overflow already happened at the op and a post-hoc range check validates the wrapped value) |
//!
//! ## Batch 258 (2026-10-20, Plan 262) — the web-trust family seed
//!
//! Three detectors from the cloudflare/security-audit-skill distill
//! (Research 292), all transcriptions with ORIGINAL fixtures — the sec
//! family's web/multi-tenant trust classes:
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `untrusted-path-join-escape` | `path_join_escape::detect` | the refusal path's error type is the fn's choice (the `ffi-export-panicking-body` class); the `components()` walk refusal is the riir-deployer `48d63f7` in-house shape, cited in-rule |
//! | `webhook-signature-over-raw-bytes` | `webhook_raw_bytes::detect` | which key, which fields, and the timestamp/nonce binding are per-scheme decisions; distinct-from `constant-time-secret-compare` (compare primitive vs coverage axis) |
//! | `unvalidated-egress-url` | `egress_url::detect` | the allowlist is deployment policy (which hosts are legitimate is the operator's answer); sink-distinct from `unbounded_read` (URL choice vs read source, the Plan 156 T2 shape) |
//!
//! Plus the count-neutral rider on `unbounded_read`: decompression
//! decoders are transparent to the taint walk (`take` stays a taint
//! break) — the bound-after-every-expansion-stage face.
//! `poisoned-lock-fail-open` (the verdict's C-grade fifth candidate)
//! DROPPED at authoring: thin decision content — the guard is one method
//! call (`PoisonError::into_inner`), the regime arguable in
//! single-purpose daemons.
//!
//! ## Batch 291 (2026-10-26, Plan 295) — the penthera distill joins
//!
//! Three detectors from the danoszz/penthera distill (Research 329, MIT
//! @ `07421f9`), all transcriptions with ORIGINAL fixtures — the web
//! trust classes the B258 seed did not cover (CORS/JWT/cookies):
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `cors-any-origin-with-credentials` | `cors_any_creds::detect` | legitimate origins are deployment policy; `very_permissive()` fires alone (credentials + reflected origin by construction), `any()`/`permissive()` need the `allow_credentials(true)` marker in the same fn |
//! | `jwt-insecure-validation-flags` | `jwt_insecure_validation::detect` | some debug/tooling decodes deliberately skip validation; the signature surfaces fire on unique names (the 9.x method/field + the 10.x `dangerous::insecure_decode`), `validate_exp = false` is jwt-vocabulary-gated |
//! | `auth-cookie-without-secure-flags` | `auth_cookie_missing_flags::detect` | JS-readable/cross-site cookies are deliberate choices; the name-vocabulary literal gate is the FP control, the fn-wide flag marker the secret-file-0600 suppression shape |
//!
//! ## Batch 292 (2026-10-27, Plan 296) — the `CVEngine` distill joins
//!
//! Four detectors from the hevnsnt/`CVEngine` distill (Research 330, GPL-3.0
//! @ `9dd9c441`; pure-Python source — the transcription law, fixtures are
//! ORIGINAL Rust), all `InputValidation`, on the two most universal std
//! surfaces (`IpAddr`, `Command`) plus the identity/strike web classes:
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `client-ip-from-untrusted-forwarded-header` | `client_ip_forwarded::detect` | the trusted-proxy set is deployment policy; the header-literal + identity-vocabulary gates fire, the `trusted` marker suppresses fn-wide (the fixed three-regime resolver always names it) |
//! | `shell-string-command-interpolation` | `shell_string_command::detect` | the shell-features regime is a design decision; shell-literal + `-c`/`/C` + interpolated-arg gates, static scripts and non-shell binaries stay silent |
//! | `ipv4-mapped-range-check-unwrap` | `ipv4_mapped_range::detect` | some guards deliberately treat all V6 as external; the V4/V6 match pair + range-test vocabulary fires, any `to_ipv4_mapped` in the fn suppresses |
//! | `abuse-strike-counts-cross-site-requests` | `strike_cross_site::detect` | label-honoring policy is deployment-specific; strike + rejection vocabulary fires, any site-label token suppresses (the guard already classifies) |
//!
//! Plus the count-neutral USE-TIME rider on `unvalidated-egress-url`
//! (corpus DATA): stored names rot — the sink re-filters; narrows the
//! DNS-rebinding window, never closes it (`CVEngine` `scannable_hosts` /
//! `discovery.py` verbatim law).
//!
//! ## Batch 294 (2026-10-27, Plan 298) — the `RustSec` transcription joins
//!
//! Two detectors from the wasmtime RUSTSEC-2026-0380/0379 distill
//! (Research 332, CC0 — cited as class provenance; fixtures are ORIGINAL
//! syntheses), both suggest-only `InputValidation` — the JIT-compiler
//! failure classes generalized to their Rust-shaped userland forms:
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `packed-field-silent-overflow-wrong-key` | `packed_field_wrong_key::detect` | the corrupt key may be a DESIGNED alias (ring buffer, hash bucket — the vocabulary gate keeps the common spellings out, counter-named ring indices still fire and review decides); the range-assert / checked-newtype / widen-the-field choice is a protocol decision. Mask-site sink novelty: `&` is deliberately absent from `unchecked-wire-arithmetic`'s operator set; the B258 recorded drop (bare-op overflow + blanket as-narrowing) adjudicated in-rule — this rule owns neither half |
//! | `capacity-exceeded-silent-fallback-not-refusal` | `capacity_silent_truncate::detect` | the refusal path's error type is the fn's choice (the `ffi-export-panicking-body` class); benign truncation (log limits, warned drops) inside encode-vocabulary fns fires too and review decides. Distinct-from `length-prefix-validate-before-alloc`: that rule owns the MISSING-CHECK face (no cap — alloc-DoS sink), this the LYING-CAP face (a cap that silently drops — wrong-dispatch sink) |
//!
//! ## Issue 185 (2026-10-27) — the softmax-collapse family (`Numerics`)
//!
//! The f32 Softmax Collapse paste (arXiv:2610.31642): `p_t - 1`,
//! `sigmoid(z) - 1` and `-ln p_t` round to exactly 0.0 past the ~16.6
//! margin cliff. A numerics-correctness defect in valid code — the
//! `nan-unsafe-sort-comparator` shape, so it joins `sec` as one
//! APPEND-ONLY family rather than a new domain (rationale in
//! `softmax_collapse`'s module doc):
//!
//! | Rule | Proposer / detector | Why |
//! |---|---|---|
//! | `sigmoid-minus-one` | ✓ `sigmoid_minus_one::collect_edits` | `sigmoid(z) - 1.0` → `-sigmoid(-z)` is an exact logistic identity; strict-gated (name allowlist + no comparison/abs context + bare-context split form + comment guard) |
//! | `softmax-onehot-collapse` | `softmax_collapse::detect` | the excluded-LSE repair restructures the pass; shapes vary (SIMD, temperature, weighted one-hot) |
//! | `neg-ln-softmax-prob` | `softmax_collapse::detect` | the row no longer carries the logits the softplus loss needs |
//!
//! All three honour one allow marker (Issue 185 T4b):
//! `// softmax-collapse: allow — <reason>` on the site's line or the line
//! directly above silences it; a reason-less marker does not. Suppressed
//! sites stay countable (`softmax_collapse::detect_all`,
//! `sigmoid_minus_one::collect_edits_with_allow`).

//! ## Batch 325 (2026-11-02, Plan 327) — the uniffi-rs FFI-hygiene distill
//!
//! Four detectors from the mozilla/uniffi-rs distill (Research 361, MPL-2.0
//! @ `ef9625b6` — cited as class provenance; fixtures are ORIGINAL
//! syntheses), the sec FFI-hygiene family's extension onto the
//! pyo3/cxx/uniffi-class mixed-language surface. ALL suggest-only — every
//! candidate is a correctness/liveness win at the boundary (double-free,
//! deadlock, leak), never latency (the dial9 NotBenchable-by-domain split;
//! the proof path is the fixture falsification arms + the integer-exact
//! refcount ledger):
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `arc-refcount-across-raw-boundary` | `arc_raw_boundary::detect` | the borrowed-vs-consume regime is the boundary protocol's answer (waker clone borrows, wake consumes) — a mechanical fix would have to invent the vtable's semantics and a wrong one double-frees; the `into_raw` round-trip is the legitimate shape and suppresses the fn |
//! | `ffi-callback-invoke-outside-lock` | `callback_lock_invoke::detect` | whether the callback can re-enter (or holds a foreign GIL/event-loop lock) is a protocol property; ADJACENT-DISTINCT from `host-lock-across-reentrant-blocking-call` (same-lock re-entry vs cross-FFI ABBA two-lock) — the in-rule adjudication note is mandatory, the tokio-guard precedent cited, reviewer-flippable |
//! | `out-param-manuallydrop-not-maybeuninit` | `outparam_init_contract::detect` | switching the param type changes the ABI contract shared with foreign callers; full-write sites fire, partial-init keeps `MaybeUninit` (its legitimate home) |
//! | `panic-handler-nested-catch-unwind` | `panic_handler_guard::detect` | the graceful-degradation choice is a protocol decision; distinct-from `ffi-extern-fn-catch-unwind` (extern BODY wrapper) and `drop-impl-no-panic` (Drop bodies) — the handler PATH was unowned |
//!
//! The two stretch candidates (Research 361 #5 `null-sentinel-companion-conjunction` B−, #6 `vec-promote-manuallydrop-not-forget` C+) are HONEST NON-CONVERSIONS at this batch: #5 held at its own conjunction-or-demote bar (the conjunction fixture set needs the authoring budget the four mains consumed — it stays a named lead); #6 is thin and perf-neutral (borrowck-prose novelty). `io-copy-write-adapter-hash-loop`'s sibling honesty class: recorded here, recoverable at a future FFI batch.

//! ## Batch 327 (2026-11-03, Plan 332) — the sanstun distill joins
//!
//! Three detectors from the firezone/sanstun distill (Research 364,
//! Apache-2.0 @ `35cb1932948bd0776` — a `sans-IO` `WireGuard` protocol
//! implementation; cited as class provenance, fixtures are ORIGINAL
//! syntheses), two families, all suggest-only:
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `secret-key-without-zeroize` | `zeroize_secret::detect` | the wrapper choice (`Zeroizing::new(..)` vs `#[derive(Zeroize, ZeroizeOnDrop)]` vs a custom Drop) is an API decision the struct's users feel; DEP-GATED on a `zeroize` import (the `constant-time-secret-compare` shape — the suggestion cannot break the build); distinct-from that rule in-rule: it owns the COMPARE primitive, this the LIFETIME/ERASE axis |
//! | `untrusted-keyed-state-unbounded` | `keyed_state_cap::detect` | choosing the cap + the full-table policy is a protocol decision (refuse-new vs evict-idle vs shed-oldest changes what an attacker can do to honest tenants); the fix is the source's own fail-closed shape — bounded table, refuse NEW keys when full, eviction would reward the attacker; the CARDINALITY face, distinct-from `length-prefix-validate-before-alloc` + `unbounded-read-into-vec` (both VOLUME faces) |
//! | `gateway-type-forces-security-check` | `gateway_ctor::detect` | the pub-vs-`pub(crate)` ctor flip ripples to every caller (an API-design decision); precision is honestly poor — vocabulary-gated security-handler detection, same-file only, suggest-only is the blast-radius control; distinct-from `client-ip-from-untrusted-forwarded-header` (rate-limit KEY SPOOFING vs checkpoint BYPASS via constructor reachability) |

//! ## Batch 328 (2026-11-04, Plan 336) — the oss-fuzz distill joins
//!
//! One detector from the google/oss-fuzz distill (Research 366, Apache-2.0
//! @ `a380569f` — the OSS-fuzz CI infrastructure itself; cited as class
//! provenance, fixtures are ORIGINAL syntheses), suggest-only
//! `InputValidation`:
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `untrusted-archive-extract-paths` | `archive_extract::detect` | the screen IS the deployment's trust decision (which archive sources are legitimate is policy) and the repair choice (skip/refuse/mangle) changes what the caller observes; distinct-from `untrusted-path-join-escape` IN-RULE: that owns the CALLER-SUPPLIED name joined onto a trusted base (fn-parameter str/Path args → fs sinks), this the ARCHIVE-MEMBER surface — the names come from the untrusted file's own central directory, and the repair is the crate's own screened accessor (`enclosed_name`), a different fix-space; also distinct-from `unbounded-read-into-vec` (that owns the VOLUME of the read, this the PATHS the members carry) |

//! ## Batch 329 (2026-11-04, Plan 337) — the tembo/voxl distill joins
//!
//! Two detectors from the tembo/voxl distill (Research 367, MIT @
//! `eba3b7ce` — a browser-reachable local AI service; cited as class
//! provenance, fixtures are ORIGINAL syntheses), suggest-only
//! `InputValidation`:
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `browser-reachable-service-host-origin-rebinding-gate` | `origin_rebind_gate::detect` | the allowlist + the missing-Origin fail-open stance are deployment policy (which hosts/origins are legitimate, whether non-browser clients are trusted) and the refusal shape (403 body, fix-flag name) is the fn's choice; distinct-from `cors-any-origin-with-credentials` IN-RULE: that owns the RESPONSE-header issuance policy (tower-http co-occurrence), this the REQUEST-side handshake gate — the mechanism reason is distinct: WS handshakes are NOT CORS-gated, a WS server setting no CORS headers is still reachable from any page, so the server itself must gate; also distinct-from `client-ip-from-untrusted-forwarded-header` (spoofed PROXY-header trust — different header, different axis) and `gateway-type-forces-security-check` (constructor reachability); the Host-rebinding/allowlist vocabulary has ZERO corpus owners (the `unvalidated-egress-url` rider's DNS-rebinding phrase is the OUTBOUND sink axis, not an inbound Host gate) |
//! | `custom-header-mutation-gate-unanswered-preflight` | `mutation_header_gate::detect` | the header name is a deployment decision (server and clients must agree) and bearer-auth/CSRF-token/SameSite=Strict APIs deliberately skip the gate; distinct-from the B258 declined bulk IN-RULE (adjudicated at authoring): that is session ARCHITECTURE (rate limiting, IDOR, authz, session management, `OpenAPI`), this the specific MECHANICAL header-gate form with an in-source mechanism reason — browsers cannot attach custom headers cross-site without a preflight, so a required custom header + an unanswered preflight closes the write path; distinct-from `browser-reachable-service-host-origin-rebinding-gate` (sibling, this batch): that gates WHO may connect (Host/Origin handshake), this gates WHICH requests may mutate (the write path); also distinct-from `cors-any-origin-with-credentials` (response-header issuance, not a request gate); the csrf/preflight/custom-header vocabulary has ZERO corpus owners (`auth_cookie_missing_flags`' `SameSite` mention is the cookie axis, `cors_any_creds`' preflight mention is the response axis) |

//! ## Batch 330 (2026-11-04, Plan 338) — the hindsight distill founds the
//! `SensitiveData` family
//!
//! Three detectors from the vectorize-io/hindsight distill (Research 368,
//! MIT @ `841979df` — an agent-memory product's redaction lane; cited as
//! class provenance, fixtures are ORIGINAL syntheses), suggest-only
//! `SensitiveData` (the scan/redact/audit lane — detection precision,
//! redaction precision, audit-emission hygiene):
//!
//! | Rule | Detector | Why suggest-only |
//! |---|---|---|
//! | `ascii-token-unicode-word-boundary-scan-miss` | `sensitive_data_boundary::detect` | the fix choice among the three Rust spellings (`(?-u:\\b)` / fancy-regex lookarounds / flanking-byte validation) is a dependency + correctness decision, and a Unicode-aware boundary is sometimes deliberate (a CJK word-segmenter wants Unicode `\\b`) — review decides; distinct-from the corpus's identifier-vocabulary gates (`secret_file_create`, `secret_ct_eq`) IN-RULE: those gate WHERE a detector looks (code identifiers), this WHAT a pattern's boundary means (scan/regex boundary semantics); the unicode-boundary-semantics vocabulary has ZERO corpus owners (greps `word.bound|(?-u|look.?around|cjk` at authoring: only the identifier-gate hits + the text-normalization SIMD family — byte classification, not boundary semantics) |
//! | `luhn-validate-numeric-pii-candidates-before-redact` | `luhn_before_redact::detect` | the validator scheme + the rescan policy are the detector's design decisions (Luhn owns cards; other PII classes carry other schemes), and a compliance-first deployment deliberately over-redacts — that stance is legitimate; distinct-from `packed-field-silent-overflow-wrong-key` IN-RULE: a mask/checksum for overflow KEYS in packed fields (wire arithmetic), this checksum-before-substitute in PII DETECTION; `luhn|credit.card` have ZERO corpus owners (grep at authoring) |
//! | `redaction-audit-hit-carries-raw-match` | `audit_hit_fingerprint::detect` | whether previews ship at all and the tier boundaries are compliance decisions, not physics — a fingerprint is derived data (first4+last4 of a 40-char token still leaks 8 chars) and a compliance-first deployment may want full masks or nothing; distinct-from `constant-time-secret-compare` IN-RULE (comparison TIMING vs emission CONTENT — different leak axis) and `secret-key-without-zeroize` (memory lifetime/erase vs the log copy); the `fingerprint` hits at authoring are entries_08's hash-table fingerprint SLOTS (a storage layout, not an emission mask) — zero owners of the preview/mask-emission class |

use syn::File;

pub mod arc_raw_boundary;
pub mod archive_extract;
pub mod audit_hit_fingerprint;
pub mod auth_cookie_missing_flags;
pub mod callback_lock_invoke;
pub mod capacity_silent_truncate;
pub mod client_ip_forwarded;
pub mod cors_any_creds;
pub mod drop_no_panic;
pub mod egress_url;
pub mod ffi_catch_unwind;
pub mod gateway_ctor;
pub mod ipv4_mapped_range;
pub mod jwt_insecure_validation;
pub mod keyed_state_cap;
pub mod length_prefix_alloc;
pub mod luhn_before_redact;
pub mod mutation_header_gate;
pub mod nan_safe_cmp;
pub mod origin_rebind_gate;
pub mod outparam_init_contract;
pub mod packed_field_wrong_key;
pub mod panic_handler_guard;
pub mod path_join_escape;
pub mod ptr_deref_null_check;
pub mod secret_ct_eq;
pub mod secret_file_create;
pub mod sensitive_data_boundary;
pub mod shell_string_command;
pub mod sigmoid_minus_one;
pub mod slice_len_guard;
pub mod softmax_collapse;
pub mod strike_cross_site;
pub mod unbounded_read;
pub mod unchecked_wire_arithmetic;
pub mod webhook_raw_bytes;
pub mod zeroize_secret;

/// The shared wire-taint walker (Batch 165) — one walker, the
/// `InputValidation` wire sinks. Crate-internal: the public surface is the
/// two detectors.
pub(crate) mod wire_taint;

// Shared emitter substrate (re-exported like rust_perf's proposers).
pub use crate::edit_emitter::{
    apply_edits, edit, line_indent, span_range, src, RuleEdit, SourceEdit,
};

/// The `--fix`-eligible sec rules.
pub const FIXABLE_SEC: &[&str] = &[
    "ffi-extern-fn-catch-unwind",
    "ptr-deref-null-check-first",
    "drop-impl-no-panic",
    "slice-from-raw-parts-len-guard",
    "constant-time-secret-compare",
    "nan-unsafe-sort-comparator",
    "sigmoid-minus-one",
];

/// Collect the rule's edits for the file (empty for unknown rules).
#[must_use]
pub fn collect_edits(rule_id: &str, source: &str, file: &File) -> Vec<RuleEdit> {
    let edits = match rule_id {
        "ffi-extern-fn-catch-unwind" => ffi_catch_unwind::collect_edits(source, file),
        "ptr-deref-null-check-first" => ptr_deref_null_check::collect_edits(source, file),
        "drop-impl-no-panic" => drop_no_panic::collect_edits(source, file),
        "slice-from-raw-parts-len-guard" => slice_len_guard::collect_edits(source, file),
        "constant-time-secret-compare" => secret_ct_eq::collect_edits(source, file),
        "nan-unsafe-sort-comparator" => nan_safe_cmp::collect_edits(source, file),
        "sigmoid-minus-one" => sigmoid_minus_one::collect_edits(source, file),
        _ => Vec::new(),
    };
    edits
        .into_iter()
        .map(|e| RuleEdit {
            rule_id: rule_id.to_string(),
            edit: e,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixable_sec_are_corpus_rules() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        for id in FIXABLE_SEC {
            assert!(
                corpus.entries.iter().any(|r| r.rule_id == *id),
                "{id} must be a corpus rule"
            );
        }
    }

    #[test]
    fn every_fixable_rule_fires_on_its_fixture() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        for id in FIXABLE_SEC {
            let rule = corpus
                .entries
                .iter()
                .find(|r| r.rule_id == *id)
                .expect("rule");
            let file = syn::parse_file(&rule.buggy_code).expect("buggy parses");
            let edits = collect_edits(id, &rule.buggy_code, &file);
            assert!(
                !edits.is_empty(),
                "{id} proposer must fire on its own buggy fixture"
            );
        }
    }

    #[test]
    fn clean_spans_fire_no_lint() {
        // Real production excerpts (shaped from riir-viewbridge /
        // riir-chain patterns) — none of the four proposers may fire.
        let clean = [
            // Already-contained extern fn (the #[riir_ffi] output shape).
            "#[no_mangle]\npub extern \"C\" fn rb_shutdown(handle: u32) {\n    let _ = std::panic::catch_unwind(|| inner(handle));\n}\n",
            // Guarded pointer read.
            "pub unsafe fn peek(p: *const u32) -> Option<u32> {\n    if p.is_null() { return None; }\n    Some(*p)\n}\n",
            // Non-Drop unwrap (clippy restriction's territory, not ours).
            "pub fn cfg_of(m: &std::collections::HashMap<String, String>, k: &str) -> String {\n    m.get(k).cloned().unwrap_or_default()\n}\n",
            // Literal-len slice construction.
            "pub unsafe fn head(b: *const u8) -> &'static [u8] {\n    std::slice::from_raw_parts(b, 16)\n}\n",
        ];
        for src_text in clean {
            let file = syn::parse_file(src_text).expect("clean parses");
            for id in FIXABLE_SEC {
                assert!(
                    collect_edits(id, src_text, &file).is_empty(),
                    "{id} fired on a clean span:\n{src_text}"
                );
            }
        }
    }
}
