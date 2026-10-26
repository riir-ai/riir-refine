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

## Queue slot

Nominal **Batch 291** / plan **295** — authoring STAGED behind the live mining-slot claims (plan-289 bbtree to 2026-10-27T16:44+07, plan-294 resin staged behind it). Gates at authoring per the B258/plan-262 precedent: sec needle + parse gates, sec-module trigger tests (probe discipline), lib tests at default + the sec-feature posture, clippy −D ×3 postures; freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild NOT owed (sec outside the release set); retrieval floors re-measure per the corpus-delta rule.
