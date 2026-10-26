# Research 329 — penthera distill verdict (CORRECTED)

**Status:** DISTILLED — pending owner decision

## Correction of record (the dial9 discipline — both grades kept)

**First pass 2026-10-26: NO corpus (C+).** Rejected on "zero `.rs` files in the source — no Rust surface to transcribe."

**CORRECTED 2026-10-26 on owner challenge: YES B− corpus.** The first-pass kill was a GATE THAT DOES NOT EXIST in the skill. The corpus fixtures are ALWAYS our own original Rust reproductions — the skill states it verbatim ("Corpus fixtures are ORIGINAL minimal reproductions. Never paste external source verbatim") — so the SOURCE's language was never a criterion anywhere. The corpus's own history refutes the gate: **B258 landed 3 sec rules from cloudflare/security-audit-skill, a 15-file pure-Markdown audit procedure with zero Rust**; the dial9 verdict landed 3 tokio rules from a blog. I cited the B258 precedent in the first-pass verdict (for the cookie/CORS decline) and then let the source's language kill the whole verdict anyway — inconsistent. The decision-content test is about whether the CLASS has a bounded fix-space + regime conditions, and whether a Rust shape for it can be authored — not whether the repo compiles Rust.

What the first pass got right and stands: lane-intel N/A (non-perf source); the B258-declined architectural bulk (rate limiting, IDOR/authz, session architecture, OpenAPI exposure, URL black-box templates) stays declined; the npm/PyPI-specific and scanner-surface material is not corpus material; and the AI-tooling rules (39) remain a SKILL-phase follow-up, not corpus entries.

## Source

- https://github.com/danoszz/penthera — offline rule-based security agent (Node 22+, 1 runtime dep), **v2.0.0 released 2026-10-25**, MIT, active (CI + OpenSSF Scorecard green), 91% AI-written by the maintainer's own disclosure with gates.
- Pin: `danoszz/penthera @ 07421f95ced357e0d7e184353e65821748478235` (full recursive tree enumerated via the GitHub API this session; no `.raw` clone — no byte-quote work is owed for a transcription verdict; rules carry "re-verify at mine time").
- Inspection date: 2026-10-26. Units: 132 readable rules — 16 black-box URL-scan templates, 21 secret + AI-key patterns, 15 npm/PyPI dependency-capability rules, 39 AI-tooling audit rules, 5 deterministic fix recipes (cookies/cors/headers/rate-limit/secrets), signed IOC packs.

## Routing

`sec` — the corpus EXISTS (19 rules: Ffi/DropLeak/UnsafeHygiene/CryptoHygiene/PanicSafety/InputValidation, `src/domains/sec/corpus.rs`). The tempting wrong route was "no domain" (first pass) — refused per this correction. Family for the candidates: `CryptoHygiene`/`InputValidation` — the exact home of the corpus's dep-gated crypto-hygiene precedents (`weak-hash-to-blake3`, `password-hash-argon2`, `constant-time-secret-compare`, `secret-file-create-without-0600`); a new `WebTrust` variant is NOT needed and must NOT be added append-only without cause (the enum's own append-only discipline).

## Dedup evidence (both vocabularies, this session)

Grep over `riir-refine/src/domains/sec/corpus.rs`: `cors|allow_origin|jsonwebtoken|same_site|http_only|secure|permissive|jwt|cookie` → **ZERO hits**. Zero under the source's terms AND zero under corpus operator-name variants (the corpus's auth surface today is `constant-time-secret-compare` + `webhook-signature-over-raw-bytes` — the COMPARE primitive and WHICH-BYTES axes; none of the three candidates below touches either). All three candidates pass.

## Ranked candidates (transcriptions, original Rust fixtures)

| candidate | penthera surface | grade | notes |
|---|---|---|---|
| `cors-any-origin-with-credentials` | `lib/fix/recipes/cors.js` + the URL-scan CORS template | B | tower-http/actix dep-gated co-occurrence: `AllowOrigin::any()`/`CorsLayer::permissive()` + `allow_credentials(true)`/`supports_credentials()` in the same builder chain — the spec-forbidden combination; bounded fix: explicit origin allowlist or drop credentials. Suggest-only; NotBenchable (domain annotation, the dial9 split) |
| `jwt-insecure-validation-flags` | `lib/blackbox/jwt.js` (alg-confusion + expiry acceptance, black-box form) | B− | jsonwebtoken dep-gated field-literal shape: `Validation` built with `validate_exp: false` OR `insecure_disable_signature_validation`/`validate_signature: false` (header-alg trust + expired-token acceptance). Bounded fix: explicit `validation.algorithms` allowlist + exp validation on. Suggest-only; NotBenchable |
| `auth-cookie-without-secure-flags` | `lib/fix/recipes/cookies.js` (Secure/HttpOnly/SameSite) | B− | cookie-crate dep-gated + session-vocabulary name gate (session/token/sid/auth, word-boundary — the `secret-file-create-without-0600` shape): `Cookie::build(..)`/`Cookie::new(..)` without `.secure(true)`/`.http_only(true)` in the same expression. Bounded fix: append the builder calls. Suggest-only; false-negatives honest (non-auth cookies legitimately omit flags); NotBenchable |

Drops (kept from the first pass, re-adjudicated): rate-limiting / IDOR / authz / session-architecture / OpenAPI exposure — architectural (the B258 declined bulk, and correct); URL-scan templates — black-box probes, no static-code shape; IOC/malware packs — threat intel, not healable code; npm/PyPI capability rules — ecosystem-specific (the Rust analogue would be OUR invention, not the source's content); secrets-in-code — scanner lane (Issue-151 law stands); open-redirect — adjacent to owned `unvalidated-egress-url`, second-tier.

## GOAT split (the dial9 two-class rule)

No candidate is benchable — **criterion 1 fails for a DOMAIN reason** (sec has no perf validator; probe-parity is the substitute per plan-262 T7). That is a per-rule `NotBenchable` annotation, never a demotion (Batch-136 precedent). The batch proves trigger fire/hold with module tests (positives + the negative matrix: cors-without-credentials stays silent, jwt with the allowlist set stays silent, non-auth cookie names stay silent — the P63–P75 probe discipline).

## Product coverage (the axis fires properly under the correction)

The corpus's auth/web surface targets Rust HTTP stacks; penthera's benchmark-winning domain (cookies/CORS/JWT — SecBench.js tied-#1) maps onto exactly the crates `cargo refine`'s public consumers run (tower-http, actix-web, jsonwebtoken, cookie). Family size today: ~5 web-trust rules (unbounded-read, wire-arithmetic, path-join, webhook-sig, egress-url) — NONE covers cookie/CORS/JWT. Follow-up pool (accumulation clause per the skill): OWASP Session Management + CORS cheat sheets, axum/tower-http RUSTSEC classes, jsonwebtoken SECURITY.md, actix-web security advisories, subtle/ring verification recipes; sources read before the batch window fold into the same batch.

## Yield estimate

1–3 rules after ~50% coordinator falsification (3 staged candidates; the cors candidate is the strongest trigger shape — pure builder-chain co-occurrence).

## Honest caveats

- Rule JSONs, fix-recipe bodies, `METHODOLOGY.md`, benchmark runners: UNREAD — disclosed. The transcription candidates are graded from the class inventory (README + tree), and every landed rule re-verifies its class against the pin `07421f9` at mine time. A quote-level batch on this repo is NOT owed (fixtures are ours; provenance is the class, not the bytes).
- NotBenchable ×3 — all suggestion-grade decision content; the batch's value axis is the sec family's public-consumer coverage, not measured numbers.
- First-pass grade kept in-record above per the corrected-verdict discipline (Research 179 / dial9).

## LANDING ADDENDUM (2026-10-26 — Batch 291 LANDED)

Claimed + authoring slot re-adjudicated by this lane (no sec-domain claim was live; the deferred claims are disjoint rust_perf/kernel/serving lanes). T0 at the pin `.raw/penthera` @ `07421f9` confirmed ALL THREE classes; TWO live-API adjustments adopted honestly (the clone swept after):

1. **tower-http 0.7 docs** — `CorsLayer::very_permissive()` is the ONE-CALL combo ("Credentials allowed" + the preflight origin reflected back = the `origin: true` + `credentials: true` shape verbatim); plain `permissive()` sets NO credentials; `any()+allow_credentials(true)` is NOT runtime-refused (protocol-inert for credentialed reads — dead misconfiguration whose naive fix IS very_permissive, the actual leak). Rule widened: `very_permissive()` fires unconditionally; `any()`/`permissive()` need the `allow_credentials(true)` marker in the same fn.
2. **jsonwebtoken CHANGELOG** — `Validation::insecure_disable_signature_validation` is the 9.x-era surface (method/field), REMOVED at 10.0 → replaced by `dangerous::insecure_decode` (10.1). The rule fires all three shapes (9.x method/field + the 10.x replacement + `validate_exp = false` jwt-vocabulary-gated); the boundary-gate probe crate pins jsonwebtoken 9 so the 9.x fixture compiles (the pin is load-bearing, commented in the gate).

**Landed (3 suggest-only rules, corpus 19 → 22; 6 fixable + 16 suggest-only):** `cors-any-origin-with-credentials` (InputValidation), `jwt-insecure-validation-flags` (CryptoHygiene), `auth-cookie-without-secure-flags` (InputValidation). Detectors `cors_any_creds` / `jwt_insecure_validation` / `auth_cookie_missing_flags` (27 module fire/hold tests); 3 SEC_SPANS variants (19 → 22, fixture count); boundary-gate probe deps extended (tower-http[cors] + jsonwebtoken 9 + cookie 0.18 + serde[derive]); the stale retrieval_eval labeled-set comment ("sec (16") repaired → 22.

**T0.4 probe evidence (the plan-262 T7 protocol):** real clippy (pinned 1.98.1) over the probe crate, groups pedantic+nursery+suspicious+correctness: the crate COMPILES at the pins; exactly ONE lint class fires across the three buggy fixtures — `clippy::must_use_candidate` ×2 (generic style, no equivalent-fix semantics for the misconfiguration). Zero security-shaped lints on the insecure shapes: **the T0.4 boundary holds ×3**; `sec_boundary_gate` (the banned-list protocol, now compiling all 22 fixtures) 2/2 PASS.

**Retrieval floors (T7):** sec self-variants **22/22 top-1** at the sec posture; the router-annotation (false,true,true) arm RE-PINNED 85/157 → 58/163 (measured 63/163 post-batch, floor = measured − 5 per the Plan-030 convention) — with the PRE-BATCH baseline MEASURED in a sibling-level worktree at the landing base `71e1cba9`: **62/160 (38.8%), ALREADY RED at the B179 pin** before this batch — the decay is the Issue-170 shared-vocab competition (the same mechanism that took the docker arm −22 spans, re-pinned there), NOT this batch (delta: +1 correct route, −0.1pp). Attribution recorded in-code at the pin site. The standing (false,false,true) arm + clippy/perf/kernel floors HELD (21/21 at the latent posture).

**Gates all green:** sec-module 172/0 (27 new detector tests); default lib 2734/0; sec-posture lib 2906/0; retrieval suite 23/0 at sec+latent and 21/0 at the standing latent posture; sec_boundary_gate 2/2; clippy −D ×3 postures (default/all-features/no-default). Freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild NOT owed (sec outside the release set). **Landing commit: `ce99cf61` (14 files, +1440/−18).**

**Drops at authoring:** none beyond the first pass — all three staged candidates survived T0 verification + falsification; the honest-drops rows (rate-limit/IDOR/authz/OpenAPI/IOC/URL-templates/npm-capability/secrets-scanner) stand from the verdict table above.

**Defers unchanged (named, not filed):** the secrets.json AI-key vocabulary probe (secret-scan lane, own unit); the agent-tooling audit phase for the katgpt-rs security-audit skill (sibling edit, owner-directed).

## Queue slot

Nominal **Batch 291** / plan **295** — authoring STAGED behind the live mining-slot claims (plan-289 bbtree to 2026-10-27T16:44+07, plan-294 resin staged behind it). Gates at authoring per the B258/plan-262 precedent: sec needle + parse gates, sec-module trigger tests (probe discipline), lib tests at default + the sec-feature posture, clippy −D ×3 postures; freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild NOT owed (sec outside the release set); retrieval floors re-measure per the corpus-delta rule.
