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

| Rule | Family | `--fix` | The bounded fix |
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

| Rule | Family | `--fix` | The bounded fix / gate |
|---|---|---|---|
| `unsafe-impl-send-sync-unsound-field` | `UnsafeHygiene` | suggest-only | `unsafe impl Sync/Send` on a same-file type with `Rc`/`RefCell`/`Cell` fields — the Rudra unsound-impl class clippy cannot see; fix = Arc/Mutex redesign (type-level, never mechanical) |
| `ffi-export-panicking-body` | `Ffi` | suggest-only | panicking body (unwrap/expect/panic/indexing/assert) in a `#[no_mangle] extern` fn — the VALUE-return complement of Phase-1's catch-unwind (unit-return); fix needs a caller-chosen sentinel |
| `weak-hash-to-blake3` | `CryptoHygiene` | suggest-only, dep-gated | SHA1/MD5 (broken) + SHA256 (house loser) digest calls → blake3; fires only when the file imports blake3 |
| `password-hash-argon2` | `CryptoHygiene` | suggest-only, dep-gated | fn named `*password*` calling a bare digest → argon2 (memory-hard KDF); name-based trigger — false-NEGATIVES honest and expected |
| `constant-time-secret-compare` | `CryptoHygiene` | yes (strict-gated) | `a == b` → `a.ct_eq(b).into()` via `subtle::ConstantTimeEq`; gates: file imports subtle + (enclosing-fn name OR operand name hits the secret vocabulary) + both operands plain idents + `#[cfg(test)]` mods skipped |

Honest scope notes: the R5 proposer is shape-based, so a `String`-typed
secret (subtle has no `ConstantTimeEq` for `String`) FIRES and then the
`--fix --verify` compile gate reverts it — the type backstop, recorded as
a fixture. The `!=` form stays out of Phase-2 scope.

## P8 intake (2026-09-01) — the 12th rule, new `PanicSafety` family

| Rule | Family | `--fix` | The bounded fix / gate |
|---|---|---|---|
| `nan-unsafe-sort-comparator` | `PanicSafety` | yes (strict-gated) | `partial_cmp(..).unwrap_or(Equal)` (or `.unwrap()`) inside a sort comparator → `total_cmp(..)` — NaN becomes deterministically ordered instead of equal-to-everything (an inconsistent order the std sorts MAY PANIC on since 1.81). Gates: enclosing fn contains a `sort_by`/`sort_unstable_by`/`select_nth_unstable_by` call + exact chain shape + `Equal`-only default + comment guard + macro args refuse. The arg carries over VERBATIM (`PartialOrd::partial_cmp` pins `(&self, other: &Self)` — the same shape `total_cmp` takes), so the output is the idiomatic human form; non-float receivers (`Option<f32>`, other Ord primitives) fail E0599 → the `--verify` gate auto-reverts. Probe 2026-09-01: zero clippy lints fire on the shape (real clippy 1.97, pedantic+nursery+perf+suspicious+correctness+style) — the boundary holds with margin. Source: riir-ai Issue 832 T5's P8 intake (≈300 sibling library sites); the intake's type-blindness concern dissolved by `total_cmp` (direction-preserving recv/arg order + name-identical on f32/f64) — recorded in the intake close-out. Honest recall limit: a comparator defined in a helper fn and passed to `sort_by(helper)` by name is not detected — the sort call and the chain must share the enclosing fn. Property: KANI-VERIFIED (harnesses g1a/g1b/g1c, Issue 127 — see the "Property evidence" section below)

## Batch 131 (2026-09-08) — the `InputValidation` family opens, suggest-only

Two rules from the dhishwasher/Girder distill (BSL 1.1 — ORIGINAL fixtures
below; the evidence shapes are cited at the pin in `.research/153`), both
SUGGEST-ONLY: parse-only detectors with zero-FP negative suites ship in
`proposers::{length_prefix_alloc, secret_file_create}`; `FIXABLE_SEC` and
the `collect_edits` arms are deliberately untouched — these rules emit
suggestions, never auto-fixes (the `ffi-export-panicking-body` shape).

| Rule | Family | `--fix` | The detection + the bounded form |
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

| Rule | Family | `--fix` | The detection + the bounded form |
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

| Rule | Family | `--fix` | The detection + the bounded form |
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
`riir_ffi!`-wrapped (catch_unwind inside), and the dry `--fix --domain sec`
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
  split form, comment guard; `--fix --write --verify` end-to-end on a
  /tmp fixture crate: 2 edits kept, 0 reverted, the collapse-premise test
  flips red → green) + `softmax-onehot-collapse` / `neg-ln-softmax-prob`
  (suggest-only detectors, `proposers::softmax_collapse::detect`). Router
  annotation (sec arm) re-pinned 62/169 → 64/172 (measured 69/172 =
  40.1%; the 3 new spans routed Sec). Fixtures self-contained (σ/softmax
  defined in-fixture) so the boundary-gate probe crate compiles. Workspace
  sweep: `examples/softmax_collapse_sweep.rs` (read-only; hits recorded in
  the issue).
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
