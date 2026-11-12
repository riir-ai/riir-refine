# `sec` — security-pattern fixing (4th retrieval domain)

> Phase 0+1 shipped 2026-08-22 (Plan 018 / Proposal 004 / Research 078-079 /
> Bench 046). Opt-in feature `sec`, default-off — promotion needs measured
> consumer pull (the `rust_perf` precedent). **Phase 2 (Batch 88) landed
> 2026-08-29** — see below. **The secret-blind scan surface (Issue 151)
> landed 2026-10-06 — see the section at the bottom.**

## What it is

Security-relevant **source-code patterns with bounded fix-spaces**: FFI
panic containment, Drop-glue panic safety, raw-pointer null hygiene, the
`from_raw_parts` len-zero contract, Rc backedge leaks — and since Phase 2:
unsound thread-safety impls, panicking FFI exports, weak-hash / password-
KDF hygiene, and constant-time secret comparison. NOT dependency auditing
(cargo-audit/cargo-deny), NOT a lint engine (dylint), NOT judgment-bearing
detection — every rule is parse-only provable with a bounded fix, or
suggest-only.

## The Phase-1 corpus (6 rules, 5 of 11 candidates killed)

| Rule | Family | fix | The bounded fix |
|---|---|---|---|
| `ffi-extern-fn-catch-unwind` | `Ffi` | ✓ | wrap unit-return extern fn bodies in `std::panic::catch_unwind` (the in-house `#[riir_ffi]` rule generalized) |
| `ptr-deref-null-check-first` | `UnsafeHygiene` | ✓ | `let Some(p) = (unsafe { p.as_ref() }) else { return None };` prologue in Option-returning unsafe fns |
| `drop-impl-no-panic` | `DropLeak` | ✓ | `.unwrap()`/`.expect()` → `.unwrap_or_default()` inside drop glue (double-panic = abort) |
| `slice-from-raw-parts-len-guard` | `UnsafeHygiene` | ✓ | `if len == 0 { &[] } else { from_raw_parts(..) }` (ptr must be non-null even at len 0) |
| `extern-fn-reference-param-to-pointer` | `Ffi` | suggest-only | `&T` param → `*const T` + explicit deref (signature-changing — the C-side caller ripples) |
| `rc-backedge-field-prefer-weak` | `DropLeak` | suggest-only | direct `Rc<Self>` backedge field → `Weak<Self>` (constructor sites change too) |

## The Phase-2 corpus (Batch 88 — 5 rules, probe vs clippy 1.97)

All 5 candidates probed OPEN against real clippy 1.97.0 (pedantic +
restriction + nursery; zero equivalent-fix firings). The crypto trio is
**dep-gated** (the `fxhashmap` pattern: fires only when the file already
imports the replacement crate — the rulewright never edits `Cargo.toml`).

| Rule | Family | fix | The bounded fix / gate |
|---|---|---|---|
| `unsafe-impl-send-sync-unsound-field` | `UnsafeHygiene` | suggest-only | `unsafe impl Sync/Send` on a same-file type with `Rc`/`RefCell`/`Cell` fields — the Rudra unsound-impl class clippy cannot see; fix = Arc/Mutex redesign (type-level, never mechanical) |
| `ffi-export-panicking-body` | `Ffi` | suggest-only | panicking body (unwrap/expect/panic/indexing/assert) in a `#[no_mangle] extern` fn — the VALUE-return complement of Phase-1's catch-unwind (unit-return); fix needs a caller-chosen sentinel |
| `weak-hash-to-blake3` | `CryptoHygiene` | suggest-only, dep-gated | SHA1/MD5 (broken) + SHA256 (house loser) digest calls → blake3; fires only when the file imports blake3 |
| `password-hash-argon2` | `CryptoHygiene` | suggest-only, dep-gated | fn named `*password*` calling a bare digest → argon2 (memory-hard KDF); name-based trigger — false-NEGATIVES honest and expected |
| `constant-time-secret-compare` | `CryptoHygiene` | yes (strict-gated) | `a == b` → `a.ct_eq(b).into()` via `subtle::ConstantTimeEq`; gates: file imports subtle + (enclosing-fn name OR operand name hits the secret vocabulary) + both operands plain idents + `#[cfg(test)]` mods skipped |

Honest scope notes: the R5 proposer is shape-based, so a `String`-typed
secret (subtle has no `ConstantTimeEq` for `String`) FIRES and then the
fix lane's compile gate reverts it — the type backstop, recorded as
a fixture. The `!=` form stays out of Phase-2 scope.

## P8 intake (2026-09-01) — the 12th rule, new `PanicSafety` family

| Rule | Family | fix | The bounded fix / gate |
|---|---|---|---|
| `nan-unsafe-sort-comparator` | `PanicSafety` | yes (strict-gated) | `partial_cmp(..).unwrap_or(Equal)` (or `.unwrap()`) inside a sort comparator → `total_cmp(..)` — NaN becomes deterministically ordered instead of equal-to-everything (an inconsistent order the std sorts MAY PANIC on since 1.81). Gates: enclosing fn contains a `sort_by`/`sort_unstable_by`/`select_nth_unstable_by` call + exact chain shape + `Equal`-only default + comment guard + macro args refuse. The arg carries over VERBATIM (`PartialOrd::partial_cmp` pins `(&self, other: &Self)` — the same shape `total_cmp` takes), so the output is the idiomatic human form; non-float receivers (`Option<f32>`, other Ord primitives) fail E0599 → the `--verify` gate auto-reverts. Probe 2026-09-01: zero clippy lints fire on the shape (real clippy 1.97, pedantic+nursery+perf+suspicious+correctness+style) — the boundary holds with margin. Source: riir-ai Issue 832 T5's P8 intake (≈300 sibling library sites); the intake's type-blindness concern dissolved by `total_cmp` (direction-preserving recv/arg order + name-identical on f32/f64) — recorded in the intake close-out. Honest recall limit: a comparator defined in a helper fn and passed to `sort_by(helper)` by name is not detected — the sort call and the chain must share the enclosing fn. Property: KANI-VERIFIED (harnesses g1a/g1b/g1c, Issue 127 — see the "Property evidence" section below)

## Batch 131 (2026-09-08) — the `InputValidation` family opens, suggest-only

Two rules from the dhishwasher/Girder distill (BSL 1.1 — ORIGINAL fixtures
below; the evidence shapes are cited at the pin in `.research/153`), both
SUGGEST-ONLY: parse-only detectors with zero-FP negative suites ship in
`proposers::{length_prefix_alloc, secret_file_create}`; `FIXABLE_SEC` and
the `collect_edits` arms are deliberately untouched — these rules emit
suggestions, never auto-fixes (the `ffi-export-panicking-body` shape).

| Rule | Family | fix | The detection + the bounded form |
|---|---|---|---|
| `length-prefix-validate-before-alloc` | `InputValidation` | suggest-only | a length parsed in-fn (`from_be_bytes`-class byte-swap or a header `.parse()`) reaching `vec![0u8; len]` with NO intervening bound comparison = allocation-size DoS (one peer-controlled header demands a multi-GB buffer). The bounded form: `if len == 0 \|\| len > MAX { return Err(..) }` BEFORE the alloc. Gates: same-fn chain only (cross-fn origins undetected — the helper-fn recall limit), identity tracking through casts/`?`/`ok_or`/`Some`/arithmetic, any comparison/match/assert on the tracked ident suppresses (validating a derived length bounds its source), `vec!`-repeat form only (`with_capacity` out of scope), `#[cfg(test)]` mods skipped. The evidence was BOTH WAYS in one codebase: an unbounded transport beside its own bounded twin (same author) — the class recurs even in careful code |
| `secret-file-create-without-0600` | `InputValidation` | suggest-only | a credential-named file (`secret`/`password`/`passwd`/`credential`/`token`/`hmac`, word-boundary, plural stems count — `secrets.json`) created by `File::create` / `fs::write` / an OpenOptions create-chain WITHOUT `.mode(..)` is 0644-by-umask, world-readable. The bounded forms: `#[cfg(unix)] OpenOptions::mode(0o600)` at create, or portably after create `#[cfg(unix)] set_permissions(Permissions::from_mode(0o600))` (the house `kat_account::write_private` form). Gates: narrow vocabulary (the ambiguous words are EXCLUDED — `key`/`identity`/`private`/`pem`, so `api_key` writes are missed, honest), create-class writes only (reads out), ANY hardening marker anywhere in the fn suppresses the whole fn (one hardened file hides a sibling — conservative). Suggest-only because a portable mechanical rewrite is not parse-only-provable (Windows `from_mode` maps coarsely) |

Gates at landing: sec lib 1261 passed / 0 failed (count pins: corpus 12→14,
`rule_ids_unique_and_kebab`, FIXABLE_SEC unchanged), boundary gate 2/2 (real
clippy over the grown probe crate — the new fixtures are compile-clean, the
E0425 class the parse gate cannot see), clippy 0 default + sec, score-bench
#80 FLAT 96.2/1.9 (suggest-only adds no fixer), retrieval: kernel+latent
byte-identical (94.7/96.9/97.7 blended, kernel 100/100/100, router 93/131),
sec self-variant top-1 holds for both new rules, ROUTER ANNOTATION re-pinned
per the Issue 020 contract — the two sec arms re-measured: 41/101 → 35/101
(floor 36→30/101) and 82/131 → 79/131 (floor 77→74/131); the new wire/fs
vocabulary ATTRACTS generic Rust spans to the sec centroid — first measured
sec-arm decay, recorded not weakened. The no-docker sec arms (Plan-018 pins)
are no longer measurable build states (the tree grew default-feature hard
deps) — noted in the floor fn.

## Batch 140 (2026-09-09) — the advisory-db distill, the family's 3rd suggest-only rule

One rule from the rustsec/advisory-db seed (CC0 @ `b50980aa`; Research 161 /
Plan 120 — the DB is a class TAXONOMY with real-world frequency, cited as
class provenance only; fixtures are ORIGINAL syntheses):

| Rule | Family | fix | The detection + the bounded form |
|---|---|---|---|
| `unbounded-read-into-vec` | `InputValidation` | suggest-only | an unbounded stream read into a growable Vec from a network reader (`read_to_end`/`read_to_string`, or a `loop { read?; extend }` drain, no `Read::take` cap) grows the buffer without limit on peer-controlled bytes — the RUSTSEC-2024-0003 / CVE-2019-9514 (h2) resource-exhaustion class. The bounded form: `Read::take(reader, MAX_BYTES)` + `Vec::with_capacity(..)`. Gates: untrusted-source vocabulary is parse-provable only (TcpStream-typed params/bindings, listener `accept()`/`incoming()` children, `connect()`/`reqwest::get()` results — a bare `R: Read` is an honest NO); File readers are the trusted regime and never track; `.take(` anywhere in the receiver chain suppresses; the loop-drain shape needs BOTH halves (read on a tracked reader + extend on a local Vec) and caps suppress (`resize` fn-wide, `buf.len()` in a loop comparison, the read-count binding vs a non-zero operand — `n == 0` is the EOF sentinel, not a cap); `#[cfg(test)]` mods skipped. Suggest-only: choosing the limit is a protocol decision |

Gates at landing: default lib 1188/0 + sec lib 1290/0 (batch140 detector suite
18/18); clippy 0 default + sec; boundary gate 2/2 (real clippy over the grown
probe crate — and the E0425-class lesson fired again as designed: the first
buggy fixture was E0596-broken (`stream` not `mut`) and the gate red'd via the
`unwrap_used` pin going vacuous BEFORE anything could claim fixture readiness);
score-bench #87 FLAT 96.2/1.9 (suggest-only adds no fixer, same as B131);
corpus-stats sec 14 → 15; retrieval: blended 93.1/96.0/97.0 of 101 (floors
93/95/97 held), sec self-variant top-1 holds for the new rule, ROUTER
ANNOTATION re-measured per the Issue-020 contract — sec+docker 34/101 (33.7%,
was 35/101; the read_to_end/TcpStream vocabulary keeps the B131 sec-centroid
drift ticking −1 span; floor re-pinned 30 → 29/101) and full state 83/131
(63.4%, was 79/131 — +4 UP, cross-arm divergence consistent with the B130
kernel-vocabulary raise; floor re-pinned 74 → 78/131); both deltas recorded,
neither weakened silently. En-route fix on develop: `ce74bac`'s rulewright
`redundant_closure` rewrite of the `--score-bench` filter was load-bearing
(`&String` → `&str` deref-coercion — the documented rulewright escape class) and
compiled to nothing under default features; restored + verified via the
score-bench run itself.

## Batch 165 (2026-09-20) — the overflow sink, the family's 4th suggest-only rule

One rule from Research 193's staged lead (Plan 156; the Issue-127 Kani
pre-validation carried it through the probe gate): unchecked arithmetic on
wire-byte origins — the overflow SINK beside the alloc sink, on the SHARED
walker. `proposers::wire_taint` extracts the Batch-131 taint walk as a
`WireSink`-parameterized substrate (`length_prefix_alloc` consumes the same
walker — its 14-test suite pins the extraction behavior-identical);
`unchecked_wire_arithmetic` adds the arithmetic sink: `+ - * << >>` and the
syn-2 assign spellings (`len += 8` parses as `Expr::Binary` with an
assign-suffixed `BinOp`) where either operand is a tracked, unvalidated
bare ident. The repair is the checked op + an INVENTED error path
(`len.checked_add(FRAME_OVERHEAD)` — the Option/Result type the fn may not
carry), so suggest-only like the rest of the family.

| Rule | Family | fix | The detection + the bounded form |
|---|---|---|---|
| `unchecked-wire-arithmetic` | `InputValidation` | suggest-only | a wire-byte-origin integer (`from_{be,le,ne}_bytes` / header `.parse()`) through a BARE overflow-prone operator with no intervening bound check — panic in debug, silent WRAP in release, and the wrapped length corrupts the frame parse ("check the multiplication, not just the result": the derived-validated shape's post-hoc range check validates the WRAPPED value). Bounded form: `checked_add`/`checked_sub`/`checked_mul`/`checked_shl` + an error path. Sink-distinct from `length-prefix-validate-before-alloc` (Plan 156 T2, recorded): distinct defect classes, distinct repairs, no composition gate — both sinks fire on the unvalidated composition shape with disjoint spans, and in the derived-validated shape the alloc rule is correctly silent while this one correctly still fires. Boundary (measured, Plan 156 T1 + T7 refinement): `arithmetic_side_effects` is SILENT on literal/const operands (`len + 8`, `len + CONST`) and where it fires (variable operands) it is blanket/unscoped/no-suggestion, restriction/allow-by-default; `manual_checked_ops` is Div-only at this pin; Div/Rem are NOT this sink. Honest recall limits: same-fn bare-ident origins only; `vec![0u8; len + 8]` (arithmetic INLINE in the repeat) is unreached — the shared walk never descends into macro tokens |

Gates at landing: sec lib 2156/0 (batch165 suite 15/15; the walker-extraction
pinned by the unchanged 131 suite); clippy 0 default + sec (--all-targets sec);
boundary gate 2/2 (real clippy over the grown probe crate — our fixture adds
NO banned-lint firing at all: the restriction is silent on const operands);
score-bench FLAT 96.36 (suggest-only adds no fixer, 15th consecutive FLAT);
corpus-stats sec 15 → 16; retrieval floors HELD zero re-pins (blended
93.1/95.0/96.0 of 101 vs floors 93/95/97), sec self-variant top-1 holds for
the new rule, ROUTER ANNOTATION re-measured per the Issue-020 contract —
sec+docker 31/101 (30.7%, was 34/101 at B140; the sec-centroid drift ticks
−3 spans; floor 29/101 STANDS, not weakened) and full state 78/131 (59.5%,
was 77/131 — +1, the kernel-vocabulary raise dominating; floor 72/131
stands). Property evidence: the landed rule's Kani row pair (g2c/g2d)
passed both ways on the WSL2 lane — see §Property evidence; the
pre-validation block returns to EMPTY.

## Batch 258 (2026-10-20) — the web-trust family seed

The cloudflare/security-audit-skill distill (Research 292 / Plan 262; source
@ `c1c8a8c`, MIT, a PROSE skill — every fixture an original synthesis).
Three suggest-only rules across TWO existing families (no new family —
adjudicated at authoring per the plan's prefer-existing-if-honest rule),
the sec domain's web/multi-tenant trust classes:

| Rule | Family | The class + the bounded form |
|---|---|---|
| `untrusted-path-join-escape` | `InputValidation` | a caller-supplied name joined onto a trusted base and sunk into `fs::*`/`File::*` with no component check lets `..`/absolute/prefix components escape the base (path traversal). Bounded form: the `components()` walk refusing everything but `Component::Normal(_)` — the riir-deployer `48d63f7` in-house shape. Gates: guard vocabulary (`components()`/`canonicalize`/`Component::{ParentDir,RootDir,CurDir,Prefix}`) suppresses fn-wide; lexical `starts_with`/`strip_prefix` do NOT suppress (the `..` escape survives both — a TRUE finding); literal/const args never fire; field origins (`entry.file_name()`) are an honest NO; detector `path_join_escape::detect` |
| `webhook-signature-over-raw-bytes` | `CryptoHygiene` | a verify computed over a re-serialized body (`serde_json::to_string`/`to_vec` of a just-parsed value) instead of the exact received bytes — canonicalization drift makes attacker bodies verify (envelope/payload identity mismatch). Bounded form: verify over the raw bytes before/alongside any parse. Distinct-from `constant-time-secret-compare`: compare primitive (timing) vs coverage axis. Honest limits: canonical-signature schemes fire by design (review decides); detector `webhook_raw_bytes::detect` |
| `unvalidated-egress-url` | `InputValidation` | a URL assembled from request/tenant input reaching a `reqwest::`/`ureq::` sink with no scheme/host allowlist and no private-range refusal — the caller picks the egress target (SSRF: cloud metadata, loopback admin edges). Bounded form: policy (allowlist + private-range refusal), never mechanical. Sink-distinct from `unbounded-read-into-vec` (URL choice vs read source, the Plan 156 T2 shape). Honest limits: client-binding method sinks + framework extractors are honest NOs; detector `egress_url::detect` |

## Batch 291 (2026-10-26) — the penthera distill, the CORS/JWT/cookie classes

Three suggest-only rules from the danoszz/penthera distill (MIT @ `07421f9` —
an offline rule-based security agent, Node/TS; Research 329's corrected
transcription: the source contributes CLASS knowledge, the fixtures are
original Rust). The first kill-gate refuted in-record: the corpus's own
history (B258 = a pure-Markdown audit source, dial9 = a blog) already
routed non-Rust sources.

| Rule | Family | Shape |
|---|---|---|
| `cors-any-origin-with-credentials` | `InputValidation` | permissive origins coupled with credentialed requests: `allow_origin(Any)`/`AllowOrigin::any()`/`permissive()` + `allow_credentials(true)` in one fn — or `CorsLayer::very_permissive()` ALONE (tower-http 0.7: credentials + the preflight origin reflected back, the one-call leak). The wildcard+credentials pair is protocol-inert for credentialed reads (dead misconfiguration); the naive repair (`very_permissive`) ACTIVATES the leak. Bounded form: `AllowOrigin::list` / the origins-array form beside credentials, or drop credentials. Gates: `very_permissive` fires unconditionally; `any()`/`permissive()` need the credentials marker in the same fn; `allow_origin(Any)` WITHOUT credentials is silent (not this graded class); `allow_credentials(false)` silent; list/predicate/dynamic forms never fire. Honest limits: same-fn co-occurrence only; cross-fn builder assembly undetected; detector `cors_any_creds::detect` |
| `jwt-insecure-validation-flags` | `CryptoHygiene` | a JWT decode configured to NOT verify: the 9.x `insecure_disable_signature_validation()` (method or field form — REMOVED at jsonwebtoken 10.0), its 10.x replacement `dangerous::insecure_decode` (the alg:none bypass class), and `validate_exp = false` (the expiry skip). Bounded form: the default posture (alg list pinned, sig + exp ON). Gates: signature surfaces fire on unique names (any receiver); `validate_exp = false` is jwt-vocabulary-gated (`jsonwebtoken::` path or a `Validation`/`DecodingKey`/`Algorithm` token in the fn) — other crates' fields silent; explicit `validate_exp = true` silent. Honest limits: statement/field-init shapes only; helper-based decodes unconnected. The probe crate pins jsonwebtoken 9 (the 9.x fixture must COMPILE — the pin is load-bearing); detector `jwt_insecure_validation::detect` |
| `auth-cookie-without-secure-flags` | `InputValidation` | an auth/session-named cookie (session/sid/token/auth/jwt literal vocabulary — penthera's severity gate) built via `Cookie::build`/`Cookie::new` with NO `.secure(..)`/`.http_only(..` marker anywhere in the fn — the session ships JS-readable over plain http. Bounded form: `.http_only(true).secure(true).same_site(SameSite::Lax)` on the builder chain. Gates: the name must be a string literal (direct or tuple-first — variable names an honest NO); non-auth names never fire (the FP control); ANY flag marker suppresses fn-wide (the secret-file-0600 conservative rule — an explicit `.secure(false)` is a written decision, never flagged). Honest limits: framework `SET_COOKIE` writes out of scope; detector `auth_cookie_missing_flags::detect` |

## Batch 292 (2026-10-27) — the CVEngine distill, the std `IpAddr`/`Command` + identity classes

Four suggest-only rules from the hevnsnt/CVEngine distill (GPL-3.0 @
`9dd9c441` — a hardened attack-surface-management scanner, pure Python;
Research 330: the transcription law applies — the source contributes
class knowledge, the fixtures are original Rust). All `InputValidation`
— the corpus's first rules on the two most universal std surfaces.

| Rule | Family | Shape |
|---|---|---|
| `client-ip-from-untrusted-forwarded-header` | `InputValidation` | an `x-forwarded-for` header read used as the client identity (rate-limit key, ban key, audit actor) with no trusted-proxy gate — the header is attacker-settable, so the leftmost read rotates the identity a throttle keys on. Bounded form: the three-regime resolver (no trusted proxies → ignore; peer trusted → right-to-left walk, first untrusted hop; malformed hop → fail closed to the peer). Gates: the header STRING LITERAL + identity/throttle vocabulary (client_ip/rate_limit/ratelimit/throttle/identity/audit/banned) co-fire; any `trusted` token suppresses fn-wide (the fixed resolver names it — `untrusted` implies the same awareness); a bare header read with no identity use stays silent. Honest limits: framework extractors that never name the literal are honest NOs; cross-fn identity flow undetected; detector `client_ip_forwarded::detect` |
| `shell-string-command-interpolation` | `InputValidation` | user data interpolated into a shell-string command (`Command::new("sh")` + `.arg("-c")` + a `format!`/concat arg) — a metacharacter allowlist does NOT make the shell string safe (argument injection, CWE-88, survives it; the CVEngine tlsx site is the live specimen). Bounded form: the ARG ARRAY on the real binary — no shell, no metacharacter layer. Gates: shell-name literal (sh/bash/zsh/cmd/powershell/pwsh) + a `-c`/`-Command`/`/C` literal + an interpolated `.arg(..)` in the same fn; a format! arg on a NON-shell binary stays silent (a different, narrower class); a static `-c` script with no interpolation stays silent; `Command::new(..)` is an ASSOCIATED-FN Call, not a method call (the cors_any_creds parse precedent). Honest limits: builder assembled across helpers unconnected; detector `shell_string_command::detect` |
| `ipv4-mapped-range-check-unwrap` | `InputValidation` | an `IpAddr` match whose V4 arm runs a range/allowlist test and whose V6 arm rejects WITHOUT unwrapping — `::ffff:127.0.0.1` is a V6 address whose bits are loopback, so a version-split check waves it through every guard (the bypass class for SSRF refusals, allowlists, firewall rules). Bounded form: `to_ipv4_mapped()` before the test (the V6 arm routes mapped addresses down the V4 path). The companion law rides the context: pass the PARSED form onward, never the raw token (Rust's parser already refuses scope ids — `fe80::1%eth0`). Gates: the V4/V6 match PAIR + range-test vocabulary (contains/is_loopback/is_private/is_link_local/allowlist/blocked/nets) fire; ANY `to_ipv4_mapped` in the fn suppresses (the normalize-up-front form); a debug-print match stays silent. Honest limits: a folded single-arm match is parse-only unreachable; non-IpAddr enums with V4/V6-shaped variants fire and review decides; detector `ipv4_mapped_range::detect` |
| `abuse-strike-counts-cross-site-requests` | `InputValidation` | an abuse/strike counter incremented on EVERY rejected request regardless of the browser's site label — a hostile page can forge attack-shaped requests from a victim's browser, so counting those rejections bans the VICTIM (denial-of-service via the defense itself; CVEngine's guard docstring is the decision content). Bounded form: the site-label gate before the increment (`sec-fetch-site` cross-site/same-site — or Origin/Referer fallback) rejects but never counts. Gates: strike/abuse/penalty vocabulary + a rejection marker (400/403/BAD_REQUEST/FORBIDDEN/reject) co-fire; ANY site-label token (sec_fetch/same-site/cross-site/origin_header/referer) suppresses fn-wide (the guard already classifies); a strike counter with no rejection path stays silent. Honest limits: counting split across fns unconnected; detector `strike_cross_site::detect` |

PLUS the count-neutral USE-TIME rider on `unvalidated-egress-url` (corpus
DATA): an admission-time check on a STORED name rots — DNS moves, and a
name a tenant controls can be pointed at metadata or an internal range
AFTER it was validated — so every path that aims an outbound sink at a
stored host list re-filters at the sink; use-time re-validation narrows
the DNS-rebinding window, never closes it (CVEngine `scannable_hosts` /
`discovery.py` verbatim law). The `abuse-strike` rule's honest-drop path
recorded in the plan never fired — the fixture carried a zero-FP
negative suite.

PLUS the count-neutral rider on `unbounded-read-into-vec`: decompression
decoders (`GzDecoder`/`ZlibDecoder`/`BzDecoder`/`XzDecoder`/`ZstdDecoder`)
are transparent to the taint walk (`take` stays a taint break) and the
concept gains the bound-after-every-expansion-stage face — `Read::take`
belongs on the DECODER (the decompressed side); the wrong-side-take
composition (`GzDecoder::new(stream.take(n))`) is a disclosed recall
limit, not a verified-safe form. `poisoned-lock-fail-open` (the verdict's
C-grade fifth candidate) DROPPED at authoring: thin decision content.

Gates at landing: sec lib 2848/0, default lib 2700/0; clippy `-D` ×3
postures (`--all-targets` default/all-features/no-default); T7 probe (real
clippy, pedantic+nursery+suspicious+correctness: ZERO lints on the three
buggy fixtures — the doc-presence family excluded at the crate level, the
only remaining output rustc `dead_code` on the private probe fixtures);
boundary gate 2/2 (the probe crate grew `serde_json` + `ureq` deps — the
Batch-88 compile contract); score-bench #192 FLAT (96.4/1.0, the P74
expectation — 16th consecutive FLAT); retrieval floors HELD no re-pin
(default state blended 125/131/133 of 141 == the B244 measured values;
sec self-variants 19/19 top-1 at the sec posture, n=16 → 19;
`poisoned-lock-fail-open` — see the drop above); freeze NOT owed (zero
clippy-corpus delta — `generations.rs` builds from the clippy seed corpus
only, verified); maintained-binary rebuild NOT owed (sec not in the
release set).

## Batch 294 (2026-10-27) — the RustSec transcription, the JIT classes userland-shaped

Two suggest-only rules from the wasmtime RUSTSEC-2026-0380/0379 distill
(Research 332, CC0 — the two CVSS-9.9 sandbox-escape advisories behind
riir-ai's pinned 28.0.1; the JIT-internal failures generalized to their
Rust-shaped userland classes). Both `InputValidation`.

| Rule | Family | Shape |
|---|---|---|
| `packed-field-silent-overflow-wrong-key` | `InputValidation` | a count/ID masked to a 2^k−1 literal (`& 0xFFF`) and used as a LOOKUP KEY silently loses the high bits — values past the field width alias onto wrong keys: wrong-item access, no panic (CVE-2026-115889: the 24-bit packed value-number class). Bounded form: range assert at the packing site / checked newtype / widen the field. Gates: lookup-key scoping (index form only) + origin gate (same-fn parse origin or counter/ID vocabulary segments) + 2^k−1 mask literals; `try_from`/`checked_` anywhere in the fn suppresses, any comparison/match/assert naming the origin suppresses. Distinct-from: `unchecked-wire-arithmetic` owns the bare-op sink (BitAnd is not in its operator set); `cast_possible_truncation` is blanket as-cast pedantic with no key scoping; the B258 recorded drop adjudicated in-rule — this rule owns neither half. Honest limits: designed aliases (ring buffers, hash buckets) fire and review decides; type-blind (`u8 & 0xFF` no-ops fire); named-const masks and method lookups out; detector `packed_field_wrong_key::detect` |
| `capacity-exceeded-silent-fallback-not-refusal` | `InputValidation` | the cap-exceeded branch TRUNCATES and falls through — the encode/build continues with silently-dropped entries: wrong encoding, wrong dispatch, no refusal (CVE-2026-115893: the br_table class; the fix's >65,520-targets REFUSAL half is the lesson). Bounded form: `if items.len() > CAP { return Err(..) }` — a cap that lies is worse than no cap. Gates: the same-cap shape (`.len() > CAP` + the branch truncates the SAME receiver to the SAME cap text) + encode/build vocabulary (fn-name segments or `to_{be,le,ne}_bytes` in the body); the refusal twin (any return/?/panic-family in the branch) suppresses. Distinct-from `length-prefix-validate-before-alloc`: missing-check (alloc-DoS sink) vs lying-cap (wrong-dispatch sink). Honest limits: benign truncation inside encode-vocabulary fns fires and review decides; slice form + `take(n)` out; cross-fn builds unconnected; detector `capacity_silent_truncate::detect` |

Gates at landing: sec module 215/0 (19 new detector tests); sec lib 2964/0;
default lib 2749/0; clippy `-D` ×4 postures (default / sec / all-features /
no-default) + all-targets+sec; T7 wide probe (real clippy,
pedantic+nursery+suspicious+correctness+perf+style+complexity: ZERO
default/suspicious/correctness safety lints on either buggy shape —
`cast_possible_truncation` fires on r2's unrelated `as u16` length-encode
(the B258-owned axis, not the rule's site, no fix) + two nursery
fixture-noise candidates; boundary gate 2/2 — `indexing_slicing` fires on
the packed-key fixture with no machine fix (the ffi-export additive
precedent), the silent-cap fixture fully silent); retrieval floors
re-measured at the sec posture: sec self-variants 28/28 top-1, fixture
labels green, router annotation (sec arm) re-pinned 60/167 → 62/169
(measured 67/169 = 39.6%; both new spans routed correctly — delta +2
correct +0.7pp); score-bench run 204 appended (sec spans ride the
demonstrability column at n=28, constant 1.000); freeze NOT owed
(`generations.rs::atoms()` builds from the clippy seed corpus only —
verified live; the P30 confirm, sec-only delta); release-set binary
rebuild + install executed (sec compiles into the default build).

## Batch 351 (2026-11-12) — the 0sec-labs/foxguard distill: FOUR new families + the SQL value-position half

corpus 56 -> 61, FIVE suggest-only rules + two rider faces + two honest drops (`0sec-labs/foxguard` @ fbeab16 MIT OR Apache-2.0, Plan 361 / Research 394): `sql-query-value-interpolation-unbound` (InputValidation — the VALUE-position SQLi face: a dynamic value formatted into statement TEXT becomes executable SQL; the bind-parameter fix; MANDATORY in-rule adjudication vs B350's identifier rule BOTH directions — value vs identifier position, ORDER BY/GROUP BY refused here, `.bind(` suppresses here never there; the source's no-placeholder FP law + the trusted-constant anti-regime carried; detector `sql_query_value_bind`) + `quantum-vulnerable-primitive-selection` (CryptoHygiene — the PQ family seed, ZERO prior owners: the classical inventory, the ml_dsa/ml_kem/slh_dsa/fn_dsa/hqc skip list, the crypto_token_in_path leading-word boundary law, the direct-call law, the CNSA 2.0 migration table; fires at home on the SSH Ed25519 surface — honest, not wrong; detector `pq_primitive_selection`) + `regex-backtracking-engine-untrusted-pattern` (InputValidation — the ReDoS family seed: the nested-quantifier shape gate; THE ENGINE SPLIT IS THE DECISION CONTENT — the `regex` crate is linear-time and NEVER flagged on shape (the anti-regime), fancy-regex/pcre2 are the surface; the plain-alternation FP reduction carried; detector `redos_engine_pattern`) + `server-connection-without-read-timeout` (InputValidation — the slowloris family seed: a std accept loop with no set_read_timeout/write_timeout pins the worker; ADJACENT-DISTINCT from `untrusted-keyed-state-unbounded` in-rule (per-CONNECTION wait vs peer-KEYED table); the deadline is a VALUE never a kill — long-poll bounded-but-long suppresses; detector `server_read_timeout`) + `open-redirect-unvalidated-target` (InputValidation — CWE-601: a caller-controlled redirect target bounces through your origin; relative-only + route-name builders + the allowlist fix; the source's safe half (url_for/reverse + constant bindings) carried as suppressions; detector `open_redirect_gate`). FACES: weak-hash use-site-only + complete-`::`-segment riders on `weak-hash-to-blake3` ("deliberately do NOT flag `use md5;` imports" + the sha1sum_path boundary) + the PROGRAM-IDENTITY face on `shell-string-command-interpolation` (R6 rescoped-dropped: foxguard flags every dynamic Command::new — the FP pressure is structural; CWE-426/427 carried as face text). TWO honest drops — the gate worked: the jwt `decode-without-verify` face (the WRONG-API axis in Rust IS `dangerous::insecure_decode`, already named by `jwt-insecure-validation-flags`) + R6 (the face above) + the full Research-394 ledger (unsafe-block/transmute ambient; no-unwrap-in-lib clippy-owned; hardcoded-secret scanner lane; tls-verify-disabled + no-ssrf POSITIVE INSTANCES of B342's egress rules; no-path-traversal ambient cousin; pq-inventory/certscan scanner lane; every taint-* rule product architecture; js/py/rb/swift-only classes language-absent). Fixtures `fixtures/b351_fixtures.rs` 20/20 standalone-executed pre-entry (the four-arm discipline; the R1 fixture carries the trusted-constant anti-regime arm + the identifier-vs-value disambiguation pair). Gates: sec lib 4300/0, default lib 3855/0, clippy -D x4 postures (sec + default clean; no-default/all-features failures PROVEN PRE-EXISTING at HEAD by stash test — latent_matcher + index_cache, not this batch's files), retrieval floors HELD both lanes (31/0 sec+latent, 29/0 default; no re-pin), pins_manifest sec 61 + health_views 1618 rows same-commit, score-bench run appended (245 runs, gen-1 SATURATED, heal_rate 0.964, parse_safety 1.0), release-set binary NOT owed (sec not in RELEASE_FEATURES — build_stamp verified at landing). Home-firing evidence: reflex serve.rs:72 + instinct serve_edge.rs:113 carry set_read_timeout (the R4 class fixed at both named sites — the rules ship clean); riir-auth's SSH id_ed25519 = R2's home class (documented in-rule); katgpt-rs + riir-data carry fancy-regex deps (R3's surface).

## Issue 227 (2026-11-12) — the D1–D7 fidelity adjudication (the verification must-fixes, fixed)

The pre-landing verification report lost a race with the landing commit by ~40 seconds; the seven rows are the durable record, now all fixed (this commit). D1 block descent in `scan_stmts` (all four stmt-walking proposers descend If/Match/Block/Unsafe/Return — the arm ordered BEFORE the bare-Expr arm; the R5 corpus `fixed_code` restructured to the route-builder shape it was accidentally passing through blindness for) · D2 explicit-path precedence in the R3 Call arm (a crate-prefixed `regex::Regex::new` — 3+ segments, first seg `regex` — is NEVER flagged whatever the file imports; a bare `Regex::new` keeps the import-scope gate and fires in a fancy-importing file, review decides — the honest limit added to the doc + mod row) · D3 the R1 const-binding suppression IMPLEMENTED (file-level `const` idents collected at scan; every `{}`-slot operand a declared const → silent; any dynamic operand keeps the finding — the corpus's trusted-constant anti-regime contract now true in code, +3 detector tests incl. the mixed-const-dynamic still-fires arm) · D4 ROUTE_BUILDERS aligned to the source's exact five (`url_for`/`url_for_external`/`reverse`/`reverse_lazy`/`get_absolute_url` — the invented `route_to`/`path_for` removed) · D5 all five proposers' spans now `span().byte_range()` (house convention, `sql_identifier_allowlist.rs:229` — line-columns were misused as byte offsets) · D6 the F1 identifier-vs-value disambiguation arm LANDED (the toy model: the value axis crosses the quote, the identifier axis absorbs the craft and a quote-free identifier rides as bare structure — the allowlist's domain; the R1 detector's ORDER BY refusal was already tested) · D7 the R4 dead stub deleted. Nits: the R2 "recognised **here**" quote restored, `.bind(` suppression tightened to the documented shape (the vacuous test now uses the buggy shape + `.bind(`), the fixture `unused_mut` silenced. Fixtures 21/21 standalone-executed post-fix (was 20/20 — D6 added the pair). Gates: sec lib 4303/0, default 3855/0, clippy -D sec+default clean (no-default/all-features crate-wide failures remain the PRE-EXISTING draft-domain cluster — latent_matcher unused-macro + index_cache, baseline-proven at the landing state, sec-disjoint), retrieval floors re-held 29/0, pins manifest+prose agree (the landing's README prose row said 56 — repaired to 61 same-commit), score-bench run 246 appended (gen-1 SATURATED, sec DROPPED-by-design — no labeled measurement spans).

## Issue 228 (2026-11-12) - the draft-domain posture cluster FIXED (file removed at the doc-sync sweep)

The PRE-EXISTING no-default/all-features failures the Issue-227 row above disclosed are since fixed (issue 228, all rows): D-a the feature-gated decline arm asserted unconditionally (the test now exists only under clippy_lints; the compiled-out posture gets its own loudly-named test) (773700fc) - D-b the no-default unused-macro allow with the gating posture named (6ed552a6) - D-c the index_cache clippy trio: doc position, LATENT_DIM-bounded cast allow, iter_mut (6ed552a6) - D-d the all-features posture: the dual-net overlap guard is BY DESIGN so the 5 service-dialing tests are cfg-skipped at any net pin, and the kernel_route_narrow store-skip is asserted posture-aware; the env-race hypothesis is REFUTED (the PoisonErrors were cascades) (90b99aec). Gates: all-features lib 5073/0 (was 8 failures), no-default 1070/0, default 3855/0, clippy -D clean at every posture, fmt clean. Full ledger: the HISTORY.md Batch-351 row; the removed issue file: git history.

## Batch 350 (2026-11-11) — the 0sec-labs/0 distill: the SQLi + crypto-misuse + agent folds

corpus 51 -> 56, FIVE suggest-only rules + one rider face (`0sec-labs/0` @ f7020eb2 MIT OR Apache-2.0, Plan 356 / Research 388; the owner-GO wait discharged per the 2026-11-11 gating decree, delegated reviewer verdict AGREE recorded in the plan): `sql-identifier-interpolation-unallowlisted` (InputValidation — the corpus's FIRST SQLi rule; the identifier-position two-regime law — values bind, identifiers cannot — with the bind-never-suppresses law; the allowlist fix; detector `sql_identifier_allowlist`) + `predictable-rng-for-secret-material` (CryptoHygiene — deterministic-seed PRNGs feeding secret vocabulary, CWE-338; the OsRng/getrandom fix; detector `predictable_rng_secret`) + `ecb-block-mode-selection` (CryptoHygiene — the structure leak, no context gate by the source's own law; the Ecb-suffix + cipher-stem literal anchors bound the substring hazard; detector `ecb_mode_selection`) + `jwt-alg-family-confusion-allowlist` (CryptoHygiene — mixed HS/asymmetric or NONE-accepting allowlists; the DISABLED-vs-WRONG-allowlist adjudication vs jwt-insecure-validation-flags mandatory in-rule; the no-allowlist face an honest NO — Validation::new defaults safe; detector `jwt_alg_allowlist`) + `untrusted-content-interpolated-into-prompt` (InputValidation — the indirect prompt-injection class, the R6+R7 conjunction's LONE SURVIVOR, R7 dropped — dispatch-gating has no bounded parse surface; NO new SecFamily variant, the schema untouched; detector `untrusted_prompt_content`) + the weak-hash security-context co-occurrence rider face on `weak-hash-to-blake3` ("we do NOT flag every MD5"). SIX recorded drops — the gate worked: R5 SSTI (the defer-or-drop gate: an ATTACK playbook, no bounded detector vocabulary), R7, hardcoded-key → scanner lane, pickle/deser + eval (language-absent), typosquat/install-script → scan lane, refusal-classifier + score-only-suppression (annotations). Fixtures b350_fixtures.rs 16/16 standalone-executed (the four-arm discipline); gates: default lib 3842/0, sec 4248/0, clippy −D ×4 postures, retrieval floors HELD 29/0 no re-pin, pins_manifest sec 56 + prose rows same-commit, score-bench clippy lane FLAT, release-set binary NOT owed (sec not in RELEASE_FEATURES).

## Batch 345 (2026-11-09) — the `LocalSR` distill: the execution-artifact trust ladder

corpus 50 -> 51, ONE suggest-only rule (`execution-artifact-trust-ladder-before-deserialize`, InputValidation; `HerRei/local-upscale` @ cf6f8b55 MIT, Plan 357 / Research 389): an artifact handed to an execution-capable runtime is gated through a trust ladder (safe format / catalog digest / explicit consent) evaluated BEFORE the loader is invoked, two-sided across the process boundary (the Rust host refuses independently at commands.rs:1614); adjudicated vs the 0sec verdict's deserialize-DATA drop -- THIS is the execution-capable-ARTIFACT class; the in-house positive instance = the wasm-validator store manifest; fixtures 5/5 executed pre-entry.

## Batch 342 (2026-11-08) — the harness-adapters/safeagents distill: the agent-egress-guard family's second source

corpus 48 -> 50, TWO suggest-only rules (huggingface/safeagents @ 0de8e22d, Apache-2.0 + its atif-scan dependency @ 0228cd9d, MIT — Plan 352 / Research 385):

- `credential-material-in-command-argv` (SensitiveData, no detector at landing — token-shape + secret-name vocabulary reaching `.arg` sites is the seed, deferred) — credential MATERIAL in a spawned command's argv is world-readable for the process lifetime (ps / /proc/<pid>/cmdline): token shapes, `Authorization:`/`x-api-key:` header values, secret-named env interpolation. The fix is the TRANSPORT (`Command::env` / stdin / config file), never a cleaner argv. Adjacent-DISTINCT from `shell-string-command-interpolation` (injection axis vs exposure axis), in-rule. Fixtures F1 (the bearer-header exposure, the env-transport fix, the token-file + stdin anti-regimes, the all-safe stub arm).
- `egress-control-bypass-flag` (InputValidation, no detector at landing — the wildcard/empty proxy shapes + the `danger_accept_invalid_certs` method name are the seed, deferred) — a flag that DEFEATS an egress control the deployment operates: wildcard proxy exclusions, proxy vars removed/emptied, TLS-verification opt-outs. The source law verbatim at the pin ('A proxy is how many sandboxes enforce their network policy, so this routes around it'); the source's own carve-outs carried in-rule (local-only clients, `--noproxy localhost` name-lists, non-network-client programs). Sink-distinct from `unvalidated-egress-url` (URL CHOICE vs policy-LAYER defeat). Fixtures F2 (wildcard defeat, carve-outs, git empty-proxy, TLS opt-outs, the tampered-verdict arm).

+2 count-neutral faces on the spillway `read-output-trust-inherits-argument-taint` host: HOST-SANCTION-PROVENANCE (the sanction source derives from the human turns only — policy.py:59-62 'a host the model wrote earlier is no more sourced than one it writes now'; model-echo never self-authorizes) + AUTHORED-ONLY SCOPING (detectors match agent-authored fields, never tool outputs nor the model's reasoning — 'Reasoning is left out' — the per-field-class trust inversion). Candidate 3 `verifier-self-modification` honestly DROPPED at authoring (the runtime-vs-build mutation distinction is not parse-provable — the plan's expected-drop allowance). En-route repair: the B337 prose that claimed `proposers::reuseaddr_gate` exists (it never landed) repaired to the honest none-at-landing posture, disclosed in the landing commit. Fixtures `fixtures/b342_fixtures.rs` 11/11 standalone-executed pre-entry.

## Batch 337 (2026-11-06) — the jitengine distill's sec half: InputValidation +1

corpus 45 -> 46: `conditional-reuseaddr-after-liveness-probe` (InputValidation, suggest-only) — the explicit SO_REUSEADDR call site is the take-over surface: on BSD/macOS a loopback bind WITH reuse sits beside a LIVE wildcard listener and shadows it for local clients (the port hijack); the law binds WITHOUT reuse first, on EADDRINUSE runs a LIVENESS PROBE, rebinds with reuse ONLY when no live server answers on the port (TIME_WAIT ends only) — the quick restart keeps working, the hijack posture refuses. The std-safe-by-default anti-regime carried in-rule (`TcpListener::bind` never sets reuseaddr); the platform split is the decision content (BSD/macOS permits the wildcard-shadow bind, Linux refuses live-over-live same-addr — probe-first is the portable posture). Zero corpus owners at T0 (reuseaddr/SO_REUSE/EADDRINUSE all zero). Detector: none at landing (the seed vocabulary is an explicit reuseaddr/set_reuseaddr call site — a future lane, deferred; an earlier version of this page wrongly named `proposers::reuseaddr_gate` as landed — repaired in Batch 342). Fixture F6 in b337_fixtures.rs (the bind-table model: beside-live refusal loopback AND wildcard, the probe-pass quick restart, the unconditional-reuse hazard demonstrated). skydiscover-ai/jitengine @ 2c34508d, Apache-2.0; quotes re-verified at the pin with the 'on the port' record correction carried. Full record: Research 376 + Plan 346.

## Batch 331 (2026-11-05) — the hindsight round-2 pool: InputValidation +1

`externally-authored-edits-by-id-not-counted-index` (InputValidation, suggest-only, no detector at landing) — an edit API addressed by an index the AUTHOR COUNTED from a rendered view cannot be range-validated into correctness: an off-by-one is still in range, silently overwrites an unrelated block, is recorded as success, and leaves length unchanged so no shrink guard notices. The repair: address externally-authored edits by a COPIED id, resolve it, skip-and-report a non-resolving one — validity is identity membership, never range membership. The same genus as `luhn-before-redact` (syntactic validity ≠ semantic validity before acting) at the edit-addressing sink. vectorize-io/hindsight @ 841979df (MIT), the round-2 pool (Plan 339 / Research 368 + addendum); the pool's fusion pair (A/B) recorded as cited drops — decision content without a fix-space (Research 368's own Drops rows); the pool's retry-classification candidate landed as rust_perf's `batch-retry-ladder-classifies-failure-before-permanent-state` (entries_64). Corpus 44 → 45.

## Batch 330 (2026-11-04) — the hindsight distill: the SensitiveData family

Three suggest-only rules from the vectorize-io/hindsight distill (Research
368, Plan 338, MIT @ `841979df` — an agent-memory product's redaction lane;
cited as class provenance; the fixtures are ORIGINAL syntheses, standalone
rustc-compiled AND executed before landing). This batch FOUNDs the
`SensitiveData` family (the scan/redact/audit lane that keeps secrets and
PII out of stored text — detection precision, redaction precision,
audit-emission hygiene; distinct from `InputValidation` (untrusted input
into sinks) and `CryptoHygiene` (crypto-API misuse)):

- `ascii-token-unicode-word-boundary-scan-miss` — a `\\b`-anchored
  ASCII-prefix secret pattern silently MISSES tokens butted against
  non-ASCII word characters (regex `\\b`/`\\w` are Unicode-aware by
  default: there is no boundary between 为 and s). The bounded Rust
  spellings: `(?-u:\\b)`, fancy-regex lookarounds, flanking-byte
  `is_ascii_alphanumeric` validation. The ASCII alphabet is a subset of
  Unicode `\\w`, so nothing that matched before stops matching. Detector:
  `sensitive_data_boundary`.
- `luhn-validate-numeric-pii-candidates-before-redact` — digit-format-only
  detectors redact timestamps/port lists as cards; Luhn-validate + the
  repeated-digit guard + the bounded UUID-neighborhood before substituting,
  and a REJECTED candidate must not consume a later valid start (rescan
  from rejected_start+1, never resume past the span). Detector:
  `luhn_before_redact`.
- `redaction-audit-hit-carries-raw-match` — audit emissions carrying the
  matched value verbatim re-leak the secret into logs/APMs (the audit wire
  IS a second copy of the data at rest); emit length-tiered fingerprint
  previews (<6 mask / 6-15 first2+last2 / >15 first4+last4), and absent
  stays absent — receivers must distinguish 'no preview info' from
  'scanned, nothing matched'. Detector: `audit_hit_fingerprint`.

All three suggest-only (the fix choices are dependency/compliance
decisions), never `FIXABLE_SEC`; T0 found the fan-out sibling candidate
(#3) STRENGTHENED by the consolidator body read (the await-before-propagate
law) and the audit-preview candidate (#4) corroborated verbatim.

## Batch 329 (2026-11-04) — the tembo/voxl distill: the browser-reachable service gate

Two suggest-only rules from the tembo/voxl distill (Research 367, Plan 337,
MIT @ `eba3b7ce` — a browser-reachable local AI service; cited as class
provenance; the fixtures are ORIGINAL syntheses, never voxl source).
NotBenchable-by-domain (security axis, never latency — the dial9 split):

| Rule | Family | Shape |
|---|---|---|
| `browser-reachable-service-host-origin-rebinding-gate` | `InputValidation` | a browser-reachable local service (dev tool, dashboard, debugger, agent API — anything bound to localhost/0.0.0.0 with a browser-facing API or WS endpoint) answering requests whose Host it never checks: ANY web page can open a WebSocket to it or POST to it (WS handshakes are NOT CORS-gated — no preflight exists, so response-header CORS policy is irrelevant), and a DNS-rebound page even presents a same-origin-looking Host. Fix (the source's gate, verbatim structure): refuse the Host unless it parses as an IP literal, is the localhost family, or the allowlist names it (`--allowed-hosts`; entries like `.example.com` cover subdomains, `*` any); refuse the Origin unless its canonical key equals the requested host:port or the allowlist names it (`--allowed-origins`); the 403 refusal names the fix flag. The missing-Origin allowance is fail-open BY DESIGN with the reason in-source: "Browsers always send a Host, so one without it comes from another kind of client." — the regime boundary: a non-browser client is allowed; the gate is anti-browser-drive-by, not auth. DISTINCT-FROM `cors-any-origin-with-credentials` IN-RULE: that owns the RESPONSE-header issuance policy (tower-http co-occurrence), this the REQUEST-side handshake gate — the mechanism reason is distinct: a WS server setting no CORS headers is still reachable from any page, so the server itself must gate; also distinct-from `client-ip-from-untrusted-forwarded-header` (spoofed PROXY-header trust — different header, different axis) and `gateway-type-forces-security-check` (constructor reachability); the Host-rebinding/allowlist vocabulary has ZERO corpus owners (the `unvalidated-egress-url` rider's DNS-rebinding phrase is the OUTBOUND sink axis, not an inbound Host gate). Two count-neutral faces carried in-concept: origin default-port canonicalization BEFORE comparison (`https://a.example` == `https://a.example:443` — compare canonical keys, never raw header strings), and non-UTF-8 header values read as "" → fail-closed refusal. Honest recall limits: the shape proves a MISSING GATE, not an attacker — an internal-only tool fires too and review decides (the regime IS browser-reachability, unknowable parse-only); same-fn only, cross-fn gate assembly undetected; a handler that never reads Host/Origin vocabulary is an honest NO; the gate vocabulary anywhere in the fn suppresses. Detector `origin_rebind_gate::detect` |
| `custom-header-mutation-gate-unanswered-preflight` | `InputValidation` | a browser-reachable MUTATING route (the upload class: an upload/multipart handler streaming a request body to disk) performing NO custom-header / CSRF-token check is writable by ANY web page the victim's browser visits — a hostile page POSTs the upload cross-site (auto-submitted form or simple fetch) and the browser attaches the victim's ambient credentials. The defense is MECHANICAL, the source's law verbatim: "Uploads must carry this header. Browsers only send custom headers cross-origin after a CORS preflight, which we never answer, so other sites can't upload." — require a custom header on the mutating route and never answer the CORS preflight; the gate closes the cross-site write path while same-origin clients keep working. Suggest-only: the header name is a deployment decision, and bearer-auth/CSRF-token/SameSite=Strict APIs deliberately skip the gate — bearer auth is the SAME mechanism class (Authorization also cannot cross sites without a preflight), so an authorization-checked handler is the FIXED shape, not a finding. DISTINCT-FROM the B258 declined bulk IN-RULE (adjudicated at authoring): that is session ARCHITECTURE (rate limiting, IDOR, authz, session management, OpenAPI), this the specific MECHANICAL header-gate form with an in-source mechanism reason; DISTINCT-FROM the sibling `browser-reachable-service-host-origin-rebinding-gate`: that gates WHO may connect (Host/Origin handshake), this gates WHICH requests may mutate (the write path) — a server can pass the handshake gate and still take a cross-site upload; also distinct-from `cors-any-origin-with-credentials` (response-header issuance) and `abuse-strike-counts-cross-site-requests` (classification policy, not a write gate); the csrf/preflight/custom-header vocabulary has ZERO corpus owners. Honest recall limits: both gates are fn-level — a handler whose write delegates to a helper with no upload vocabulary, or whose gate lives in middleware (cross-fn), is undetected; an offline batch uploader fires too and review decides; the broader mutating-route surface (rename/delete/create without an upload file-write shape) is OUT of the fire set — same class, deliberately refused for precision. Detector `mutation_header_gate::detect` |

## Batch 328 (2026-11-04) — the oss-fuzz distill: the archive-member zip-slip

One suggest-only rule from the google/oss-fuzz distill (Research 366, Plan
336, Apache-2.0 @ `a380569f` — the OSS-fuzz CI infrastructure itself;
`infra/cifuzz/http_utils.py` downloads a remote zip then `extractall` with
no member screen; the fixtures are ORIGINAL syntheses, never oss-fuzz
source). NotBenchable-by-domain (security axis, never latency — the dial9
split):

| Rule | Family | Shape |
|---|---|---|
| `untrusted-archive-extract-paths` | `InputValidation` | a path extracted from an UNTRUSTED ARCHIVE whose member names are attacker-chosen: `ZipArchive::extract(dest)` with no member screen, or a `by_index`/`by_name` loop joining `entry.name()` into a destination path — a member named `../../etc/cron.d/x` (or an absolute path, or a Windows prefix) escapes the destination directory: arbitrary file write (zip-slip). Fix: the crate's own screened accessor `entry.enclosed_name()` (`Some(p) => dest.join(p), None => skip/refuse`), the generic `components()` walk refusing everything but Normal, or mangled extraction (sanitize separators before the join). DISTINCT-FROM `untrusted-path-join-escape` IN-RULE: that owns the CALLER-SUPPLIED name joined onto a trusted base (fn-parameter args → fs sinks), this the ARCHIVE-MEMBER surface — names come from the untrusted file's own central directory and the repair is the crate's own screened accessor, a different fix-space (the `unvalidated-egress-url` sink-distinct precedent); also distinct-from `unbounded-read-into-vec`: that owns the VOLUME of the read, this the PATHS the members carry. Advisory posture VERIFIED at authoring (rustsec.org fetch 2026-11-04): RUSTSEC-2025-0168 (zip crate — incorrect path canonicalization during archive extraction → arbitrary file write, https://rustsec.org/advisories/RUSTSEC-2025-0168) plus a same-surface 2026 wave (symlink targets, SUID/SGID preservation, permissions through symlinks); the class fires in consumers too (RUSTSEC-2026-0434 ant-releases, RUSTSEC-2026-0427 sn-releases — both zip-slip); the extract() surface's screening posture is version-dependent — consumers re-derive the advisory posture at adoption. Detector `archive_extract::detect` |

Two count-neutral rider faces landed on the neighbors (notes only, no new
triggers): the BASENAME-REDUCTION fix-form on `untrusted-path-join-escape`
(a single untrusted NAME → extracting the basename before the join IS the
repair — the source's own `fuzz_target.py::_save_crash` does
`os.path.basename(crash.input_path)` before joining the artifact
directory), and the SECOND-EXPANSION-STAGE corroboration on
`unbounded-read-into-vec` (download-then-extract = untrusted read source +
a second unbounded expansion — `http_utils.py::download_and_unpack_zip` is
the real regime instance; the zip-bomb adjacency).

## Batch 327 (2026-11-03) — the sanstun trio: secret lifetime, cardinality, checkpoint bypass

Three suggest-only rules from the firezone/sanstun distill (Research 364,
Plan 332, Apache-2.0 @ `35cb1932` — a sans-IO WireGuard protocol
implementation in Rust; the fixtures are ORIGINAL syntheses, never sanstun
source). ALL NotBenchable-by-domain (correctness/security axes, never
latency — the dial9 split):

| Rule | Family | Shape |
|---|---|---|
| `secret-key-without-zeroize` | `CryptoHygiene` | a struct in the secret vocabulary (Key/Secret/Seed/Token/Password/Credential as underscore segments) holding RAW key material (`[u8; N]`/`Vec<u8>`/`String`) with no Drop impl and no zeroization wrapper keeps secret bytes alive in freed memory (heap spray, core dumps, swap). Fix: `Zeroizing::new(..)` or `Zeroize`/`ZeroizeOnDrop`. DEP-GATED on a zeroize import (the constant-time-secret-compare shape — the suggestion cannot break the build). DISTINCT-FROM `constant-time-secret-compare` IN-RULE: that owns the COMPARE primitive (timing leak at comparison), this the LIFETIME/ERASE axis (bytes persisting after drop) — CryptoHygiene's first lifetime member. Source anchor: keys.rs 'An X25519 static private key, zeroized on drop.' over `Zeroizing<[u8; 32]>` fields; Zeroizing wraps owned key material in 8 source files. Detector `zeroize_secret::detect` |
| `untrusted-keyed-state-unbounded` | `InputValidation` | the resource-exhaustion family's CARDINALITY face: a HashMap/HashSet keyed by peer-derived distinct keys (addr/ip/peer/session/device/tenant/origin/client/conn vocabulary) growing via insert with NO cap guard admits a many-keys memory DoS — one adversarial peer family minting distinct keys exhausts memory. The fix shape (the source's own): a bounded table + FAIL-CLOSED refusal of NEW keys when full (`const MAX_SOURCES: usize = 65536;` + `if self.buckets.len() >= MAX_SOURCES { return false; }` before insert — eviction would reward the attacker by admitting them over honest entries; the source's test name says it: `full_table_refuses_new_sources_until_idle_ones_are_dropped`). DISTINCT-FROM `length-prefix-validate-before-alloc` + `unbounded-read-into-vec` IN-RULE: those own the VOLUME faces (one length → one alloc; one stream → one growing buffer), this the CARDINALITY face (many distinct keys → unbounded entry count). Detector `keyed_state_cap::detect` |
| `gateway-type-forces-security-check` | `InputValidation` | a security-relevant message/handler type with PUBLIC constructors lets any caller synthesize inputs and bypass the gateway's checkpoint (rate limit, MAC verification, replay check) — the checks exist but are skippable. Fix: crate-private constructors + the gateway type as the only producer, so the dangerous handler is UNREACHABLE without passing the checkpoint. Source anchor: message.rs — every message-type constructor crate-private, the only public consumption `ReceivedMessage` produced by `Interface::receive` (mac1 ct_eq → under-load window → cookie → rate_limiter.allow); the source doc: 'Messages can only be obtained through that method and cannot be cloned or constructed directly.' DISTINCT-FROM `client-ip-from-untrusted-forwarded-header` IN-RULE: that owns rate-limit KEY SPOOFING (untrusted header as key), this CHECKPOINT BYPASS via constructor reachability. Honest recall limits: 'which handler is security-relevant' is vocabulary-gated, cross-file gateway relationships invisible — policy-class suggest-only, the SSRF-rule shape. Detector `gateway_ctor::detect` |

The verdict's fifth candidate (`reject-before-expensive-sink`, the
pre-decryption replay check) DROPPED at T0: the buggy direction requires
INVENTING the check (nobody writes decrypt-then-replay-check — the B258
recorded-drop class); the admission-before-work genus is owned by
`batch-load-admission-sized-from-files-refuses-past-the-hard-budget`
(rust_perf entries_47) and its family.

## Batch 325 (2026-11-02) — the uniffi-rs FFI-hygiene quartet

Four suggest-only rules from the mozilla/uniffi-rs distill (Research 361,
MPL-2.0 @ `ef9625b6` — cited as class provenance; the fixtures are ORIGINAL
syntheses, never uniffi source). The sec FFI-hygiene family extended onto
the pyo3/cxx/uniffi-class mixed-language surface. ALL
NotBenchable-by-domain (correctness/liveness wins at the boundary, never
latency — the dial9 split; the proof path is the fixture falsification
arms + the integer-exact refcount ledger):

| Rule | Family | Shape |
|---|---|---|
| `arc-refcount-across-raw-boundary` | `DropLeak` | a raw `Arc` pointer crossing an unsafe boundary has TWO custody regimes — BORROWED (`increment_strong_count` + deref) or CONSUMING (`from_raw` exactly once); mixing them (from_raw+forget, double from_raw, increment stacked on a consume) double-frees or leaks. The `into_raw` round-trip is the legitimate shape and suppresses the fn; `ManuallyDrop` custody suppresses. Detector `arc_raw_boundary::detect`; the waker-vtable class (uniffi future.rs clone/wake arms) |
| `ffi-callback-invoke-outside-lock` | `Ffi` | a callback/waker INVOKED while its mutex guard is live — the cross-FFI ABBA deadlock (internal Mutex vs the foreign GIL/event-loop lock). Extract-under-lock, invoke-after-release. ADJACENT-DISTINCT from `host-lock-across-reentrant-blocking-call` (same-lock re-entry vs cross-FFI two-lock — in-rule adjudication note mandatory, reviewer-flippable). Detector `callback_lock_invoke::detect`; drop(g)/inner-scope/guard-tail gates |
| `out-param-manuallydrop-not-maybeuninit` | `Ffi` | a caller-initialized repr(C) out-param takes `&mut ManuallyDrop<T>` (safe reads, set-twice = a visible leak), never `&mut MaybeUninit<T>`; MaybeUninit stays for callee-initializes scratch. Full-write sites fire; partial field-init keeps MaybeUninit (its legitimate home). Detector `outparam_init_contract::detect` |
| `panic-handler-nested-catch-unwind` | `PanicSafety` | an error-REPORTING handler doing fallible work (`format!`/alloc) carries its own `catch_unwind` + graceful degradation — a panic on the handler path double-panics an active unwind and masks the original error. The handler PATH was unowned (`ffi-extern-fn-catch-unwind` owns the extern body wrapper, `drop-impl-no-panic` Drop bodies). Detector `panic_handler_guard::detect`; vocabulary-gated, logging-only bodies silent |

The two stretch candidates are HONEST NON-CONVERSIONS at this batch: #5
`null-sentinel-companion-conjunction` (B−) held at its own
conjunction-or-demote bar (the conjunction fixture set needs the authoring
budget the four mains consumed — a named lead for a future FFI batch); #6
`vec-promote-manuallydrop-not-forget` (C+) thin + perf-neutral.

Gates at landing: sec self-variants 35/35 top-1 (the out-param fixture
carries a status-code return — the unit-return extern shape is the
catch-unwind proposer's territory by its own scope line, and a fixture in
that territory reads as that rule's bug under Structural rerank);
router annotation (full seven-domain arm) re-pinned 84/195 → 85/210
(measured 90/210 = 42.9%; the B131 sec-centroid mechanism compounding with
the kernel arm's generic Rust-span attraction — the re-pin recorded
in-code, gating stays killed); all-features lib 4466/0 (the b324-missed
kernel-side rust_perf count witness repaired en route 289 → 292, the
B266/B271/B305 missed-pin class); clippy −D × postures
(default/sec/no-default/all-features) clean; score-bench run #220
recorded (gen-1 heal 0.9636 FLAT, parse 1.0 — the clippy-seeded bench
corpus unmoved, freeze tables NOT moved); binary rebuild NOT owed (sec is
NOT in the release set — the manifest verified live).

## Property evidence — machine-checked (Kani, Issue 127)

A second evidence axis beside probe evidence: Kani (model checking over
CBMC) decides the property BOTH ways on real MIR — the buggy form must
VIOLATE it (existence witness), the fixed form must SATISFY it. The
harness crate is deliberately DETACHED:
[`.proofs/kani_sec_poc/`](../../.proofs/kani_sec_poc/README.md)
(`[workspace]`-marked, zero deps, Kani 0.68.0 / CBMC 6.11.0 / its own
pinned `nightly-2026-08-21` — never the workspace's stable
`rust-toolchain.toml`), run on the 4090 box's WSL2 lane only
(Windows-native Kani is unsupported), **never in CI, never a per-push
gate** — a dev-time oracle. Measured 2026-09-20 (5 harnesses at the POC
landing; 7 after the Batch 165 g2c/g2d extension); witnesses captured via
`-Z concrete-playback --concrete-playback print`.

| Rule | Family | Harness | Property | Verdict | Solver |
|---|---|---|---|---|---|
| `nan-unsafe-sort-comparator` | `PanicSafety` | `g1a_buggy_cmp_violates_transitivity_of_equal` | the `partial_cmp(..).unwrap_or(Equal)` comparator is non-total | **VIOLATED + witness** — x=`-inf`, y=`+NaN`, z=`+inf`: cmp(x,y)=Equal, cmp(y,z)=Equal, cmp(x,z)=Less | 0.046 s |
| `nan-unsafe-sort-comparator` | `PanicSafety` | `g1b_total_cmp_equal_is_transitive` | `total_cmp` transitive on Equal | **VERIFIED** | 0.060 s |
| `nan-unsafe-sort-comparator` | `PanicSafety` | `g1c_total_cmp_is_total_antisymmetric_nan_self_equal` | `total_cmp` total + antisymmetric + NaN-self-Equal | **VERIFIED** | 0.074 s |
| `unchecked-wire-arithmetic` | `InputValidation` | `g2c_buggy_framed_len_reaches_overflow` | the corpus fixture's bare `parse + FRAME_OVERHEAD` reaches overflow (its u32-domain twin — the fixture's `as usize` widening absorbs the overflow on 64-bit hosts, so the machine-checkable property lives in the wire-width domain the rule's advice targets: checked_add BEFORE any widening) | **VIOLATED + witness** — `"attempt to add with overflow"` inside the buggy callee (any header above `u32::MAX - 8`) | 0.023 s |
| `unchecked-wire-arithmetic` | `InputValidation` | `g2d_checked_framed_len_never_silently_wraps` | the bounded repair is exact two-sided: Some ⟺ u64-exact length, None ⟺ true length > u32::MAX | **VERIFIED** (every check SUCCESS, 0 failures) | 0.027 s |

### Pre-validation (staged leads, NOT shipped rules)

This block is deliberately separate: a machine-checked property on a rule
that does not exist yet is the strongest-looking row in the section and
the weakest claim in it — it must never read as corpus coverage.

**EMPTY — as designed.** The `unchecked-wire-arithmetic` rows moved UP
into the shipped table at the Batch 165 landing (Plan 156 T6): the
g2a/g2b probe-shape harnesses (two parses summed) pre-validated the
Research-193 staged lead; the g2c/g2d fixture-shape harnesses
post-validate the shipped rule. The block refills when the next staged
lead earns machine-checked pre-validation before its batch.

**Scope caveat (verbatim from the POC record):** these numbers are the
FLOOR, not the ceiling. The harnesses are loop-free, dependency-free,
≤6 symbolic variables, and needed no unwinding config; solver times of
0.03–0.07 s say nothing about loop/unwind bounds, heap structures, or
workspace-real-code harnesses — the "minutes per fixture" ceiling was
never approached and remains UNMEASURED. Every cost figure in this
section is a loop-free-fixture figure.

## The boundary discipline (non-negotiable)

`tests/sec_boundary_gate.rs` runs REAL clippy over the corpus fixtures with
the banned list enabled (`mem_forget`, `indexing_slicing`,
`arithmetic_side_effects`, `unwrap_used`/`expect_used`, `improper_ctypes*`,
`partialeq_ne_impl`, `undocumented_unsafe_blocks`, `disallowed_methods`):
no banned lint may fire MachineApplicable on a sec fixture — a future
clippy that auto-fixes one of our territories FAILS the gate and that rule
retires. The pinned allowed overlaps (all fire with NO suggestion — our
fixes are additive): `unwrap_used` on the drop fixture, `indexing_slicing`
on the panicking-FFI fixture, `undocumented_unsafe_blocks` on the
unsound-impl fixture.

## The unsafe counter (geiger taxonomy, zero deps)

`count_unsafe` / `UnsafeCounts` / `file_forbids_unsafe`: unsafe fns +
methods, attribute-unsafety (`#[no_mangle]`/`#[export_name]` on otherwise
safe fns), unsafe impls/traits, unsafe exprs. Feeds
`SecVerdict.unsafe_count_delta` — a review SIGNAL, never a gate.

## Negative control

riir-viewbridge IS the negative control: its whole FFI surface is
`riir_ffi!`-wrapped (catch_unwind inside), and the dry fix-lane `--domain sec`
run over its 24 files proposes 0 edits. A nonzero there falsifies the
proposer.

## Status

- Phase 0 (scaffold + wiring + boundary gate + unsafe counter): DONE
- Phase 1 (corpus + 4 proposers + GOAT): DONE (Bench 046)
- Phase 2 (unsafe + crypto hygiene): UNBLOCKED (≥6-rule gate met) but
  deferred per Plan 018 — a future batch
- Batch 291 (2026-10-26): corpus 19 → 22 (6 fixable + 16 suggest-only;
  the penthera distill, Plan 295) — retrieval floors re-measured at the
  sec posture: sec self-variants 22/22 top-1, router annotation (sec arm)
  re-pinned 85/157 → 58/163 with the PRE-BATCH ALREADY-RED attribution
  recorded in-code (the Issue-170 shared-vocab decay, not this batch —
  pre-batch baseline measured 62/160 = 38.8% in a sibling worktree at the
  landing base)
- Batch 292 (2026-10-27): corpus 22 → 26 (6 fixable + 20 suggest-only;
  the CVEngine distill, Plan 296) — sec self-variants 26/26 top-1 (the
  sec Fixture_labels gate green at the new population), router annotation
  (sec arm) re-pinned 58/163 → 60/167 (measured 65/167 = 38.9%; all 4 new
  spans routed correctly, delta +2 correct +0.2pp — the std-only
  identity/IpAddr/Command vocabulary is retrieval-neutral); boundary gate
  2/2 (real clippy fires no equivalent-fix lint on the std-only buggy
  fixtures — no probe deps added); sec-module detectors 196/0; clippy −D
  ×4 postures (default/sec/all-features/no-default) + all-targets+sec
- Batch 294 (2026-10-27): corpus 26 → 28 (6 fixable + 22 suggest-only;
  the RustSec transcription, Plan 298) — sec self-variants 28/28 top-1,
  router annotation (sec arm) re-pinned 60/167 → 62/169 (measured 67/169
  = 39.6%; both new spans routed correctly, delta +2 correct +0.7pp);
  boundary gate 2/2 (indexing_slicing on the packed-key fixture, no
  machine fix — additive; silent-cap fully silent); wide probe clean
  (zero default/suspicious/correctness safety lints); freeze NOT owed
  (clippy-seeded corpus unmoved); binary rebuilt + installed
- Issue 185 (2026-10-27): corpus 28 → 31 (7 fixable + 24 suggest-only;
  the softmax-collapse family, new APPEND-ONLY `SecFamily::Numerics`,
  arXiv:2610.31642) — `sigmoid-minus-one` (mechanical, strict-gated:
  logistic-name allowlist, no comparison/`.abs()` context, bare-context
  split form, comment guard; a `cargo refine fix` end-to-end on a
  /tmp fixture crate: 2 edits kept, 0 reverted, the collapse-premise test
  flips red → green) + `softmax-onehot-collapse` / `neg-ln-softmax-prob`
  (suggest-only detectors, `proposers::softmax_collapse::detect`). Router
  annotation (sec arm) re-pinned 62/169 → 64/172 (measured 69/172 =
  40.1%; the 3 new spans routed Sec). Fixtures self-contained (σ/softmax
  defined in-fixture) so the boundary-gate probe crate compiles. Workspace
  sweep: `examples/softmax_collapse_sweep.rs` (read-only; hits recorded in
  the issue).
- Batch 329 (2026-11-04): corpus 39 → 41 (7 fixable + 34 suggest-only;
  the tembo/voxl distill, Plan 337 / Research 367) — 2 detectors joined
  (`browser-reachable-service-host-origin-rebinding-gate` +
  `custom-header-mutation-gate-unanswered-preflight`); the B258
  CSRF-bulk drop adjudicated IN-RULE (the declined bulk is session
  ARCHITECTURE, the mechanical header-gate form survives); +2
  count-neutral rider faces this batch: the symlink-listing re-check on
  `untrusted-path-join-escape` (the listing must re-check each entry's
  canonical target, not just the base path — voxl files.rs::list L94-105)
  + the wire-header reserve-in-place law on rust_perf's
  `reserve-then-commit-capacity-accounting` (cross-domain face,
  rust_perf.md); R5 (untrusted-shared-memory-attach-pid-and-content-
  verify) DROPPED honestly at T0 (niche population); corpus-count pin
  (`sec.rules` 39 → 41) landed via the `pins_manifest.py --write`
  manifest diff; retrieval floors + freeze/rebuild disposition are this
  landing's calls (recorded in the landing commit).
- Batch 328 (2026-11-04): corpus 38 → 39 (7 fixable + 32 suggest-only;
  the oss-fuzz distill, Plan 336 / Research 366) — 1 detector joined;
  boundary-gate probe crate gains the `zip` dep (2.6.1 verified);
  corpus-count pin (`sec.rules` 38 → 39) owed the `pins_manifest.py
  --write` manifest diff at the central landing; retrieval floors +
  freeze/rebuild disposition are the central landing's calls (this block
  authored by the batch agent).
- Batch 327 (2026-11-03): corpus 35 → 38 (7 fixable + 31 suggest-only;
  the sanstun distill, Plan 332 / Research 364) — 3 detectors joined;
  retrieval floors re-measured at the landing; freeze NOT owed
  (clippy-seeded corpus unmoved); binary rebuild OWED at the landing HEAD
  (rust_perf IS release-set — the batch's rust_perf half rides the same
  rebuild; sec itself is not release-set).
- Batch 325 (2026-11-02): corpus 31 → 35 (7 fixable + 28 suggest-only;
  the uniffi-rs FFI-hygiene distill, Plan 327) — sec self-variants 35/35
  top-1, router annotation (full seven-domain arm) re-pinned 84/195 →
  85/210 (measured 90/210 = 42.9%, the re-pin recorded in-code); freeze
  NOT owed (clippy-seeded corpus unmoved); binary rebuild NOT owed (sec
  not in the release set).
- Promotion to default: needs ≥1 workspace repo adopting sec in CI

## See also

- [Proposal 004](../../.proposals/004_sec_domain_fourth_heal_domain.md) — the design argument
- [Plan 018](../../.plans/018_sec_domain_phase0_batch1.md) — execution plan
- [Research 078](../../.research/078_sec_domain_seed_verdict_rust_secure_code_index.md) — the seed survey
- [Research 079](../../.research/079_sec_domain_phase0_1.md) — the Phase-0+1 record
- [Bench 046](../../.benchmarks/046_sec_domain_phase0_1_goat.md) — the GOAT numbers

## The secret-blind surface (Issue 151, 2026-10-06)

Detection of secrets in code is **scan-based, not corpus-match** — the
verdict record: secret formats are exact enumerable facts (fixed prefix +
body charset + length), so detection is deterministic and versioned
(`redact v4`), while corpus-match (latent retrieval over buggy fixtures)
stays the FIX-suggestion machinery. A "hardcoded secret" corpus entry would
carry a fake key in its fixture (tripping our own scanners), match fuzzily
where exactness matters, and its fix is a process change (env var / secret
store) — never a mechanical rewrite.

One canonical scanner, three consumers:

| Consumer | Boundary | Posture |
|---|---|---|
| `kat_redact::RedactionGate` (riir-kat v4) | **egress** — mining sync, the LLM harness lanes | fail-closed deny-by-default; any finding drops the span |
| `structural_admission::scan_secret_shapes` (reef P2) | **persistence** — structural proposals | the SAME table, post-filtered: prefix kinds ∪ private-key blocks only + the word-boundary check (entropy rules would block every payload carrying hashes) |
| `secret_blind` (`--secret-scan`) | **developer surface** | suggest-only warn + report + blind transform; values and fingerprints never shown |

The v4 pattern set consolidates the industry sources: gitleaks `b58d3f1`
(MIT — the transcription source, parity-fixture-pinned),
secrets-patterns-db (CC BY-SA 4.0 — coverage cross-check ONLY; ShareAlike
forbids table copying), secret-regex-list (no license — reference only).
Provider prefixes are functional facts, not creative expression; the
scanner's transcription is prefix + charset predicate + length, not a regex
copy (no ReDoS surface by construction).

**The shareable-output rule:** a finding's fingerprint is an unkeyed
`blake3(bytes)[..12]` — safe in the gate's LOCAL JSONL log, a
dictionary-reversal leak on stdout. Every stdout surface (report, warn
lines, `--blind`) carries kind + location only. Recorded upgrade path if
cross-run correlation is ever needed: a per-install `blake3::keyed_hash`
key.

**Digest-pin adjudication (P63, 2026-10-15):** the developer surface
splits from the gate on ONE class — contextually-evidenced digest pins.
A high-entropy-hex run carrying a digest-scheme tag (blake3:/sha256:/…,
glued or across whitespace + one opening quote — the blake3: "<hex>"
field shape) or opening the fresh-literal value of a pin-named
declaration (_SHA/_DIGEST/_BASELINE/_COMMITMENT + algo spellings,
rustfmt-wrap carried) is adjudicated benign in the REPORT count only,
with the suppressed count DISCLOSED (never a silent drop). Exact
provider formats are never adjudicated (aws/sk-/AQ. keep firing under
both guards; probe-pinned by test), and the egress gate + --blind keep
the RAW canonical verdicts — a report FP costs noise, a gate FP never
ships a secret, and only the former is a defect (REDACT_VERSION
untouched). Live basis: 65 suppressions workspace-wide at first run,
100% digest-pin class (instinct 4, refine 53 incl. the 49-row corruption
corpus, reflex 3 wrapped BLAKE3 fixture pins, katgpt-rs 5 _BLAKE3/_DIGEST
fixture pins); zero non-digest findings moved.

**Egress census (the trust claim's basis):** every span-text egress passes
the gate — `push_mining_batch`/`load_mining_rows` (heal.rs), the
`fixer_regate` harness (`gate_samples` before any arm), the `kat_drain`
daily loop (same gate; all-blocked = exit 4). Transports that carry no code
text: burn/lease/fixstat/replay (counts + rule ids only). Honest limits:
unknown formats are caught only by the entropy rules (the Bench-052 hex
story — 37% of hex-wrapped AWS-shaped keys pre-v2, unconditional since);
arbitrary-transform classes (byte arrays, xor, split literals) are uncaught
BY DISCLOSURE; PII is out of scope (the span itself is the asset — no
pattern set closes that); a redacted payload is terminal and is never
re-gated.
