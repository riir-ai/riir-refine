# Plan 295 — Batch 291: penthera→sec transcription batch

**Status:** STAGED — authoring slot-gated behind live claims (plan-289 bbtree to 2026-10-27T16:44+07, plan-294 resin behind)

Source: danoszz/penthera @ `07421f95ced357e0d7e184353e65821748478235` (MIT) — transcription verdict Research 329 (corrected from the first-pass NO; both grades in-record). Fixtures are OUR original Rust reproductions; the source contributes class knowledge only. Zero `.raw` clone owed; class re-verification against the pin at mine time.

- [ ] T0 re-verify at the pin: read `lib/fix/recipes/{cors,cookies}.js`, `lib/blackbox/jwt.js`, `lib/rules/core/secrets.json` — confirm the three classes as inventoried; adjust grades honestly if the source differs
- [ ] T1 dedup re-verification at the batch's own corpus generation (both vocabularies ×3 candidates; zero-owner clearance recorded in the note)
- [ ] T2 probe the trigger shapes BEFORE authoring (the sec probe-parity substitute for GOAT, plan-262 T7 + P63–P75 discipline): module-testable positives AND the negative matrix — cors-without-credentials silent · jwt with explicit allowlist + exp-on silent · cookie with non-auth name (preference/tracking) silent · owned neighbors unaffected
- [ ] T3 author 3 suggest-only sec entries with original fixtures (parse-validated): `cors-any-origin-with-credentials` (tower-http/actix dep-gated co-occurrence), `jwt-insecure-validation-flags` (jsonwebtoken field-literal), `auth-cookie-without-secure-flags` (cookie crate + session-vocabulary name gate) — family adjudicated per rule (CryptoHygiene/InputValidation; NO new SecFamily variant without cause)
- [ ] T4 honest drops recorded in-rule (rate-limit/IDOR/authz/open-redirect rows carry their decline reasons — the B258 declined bulk + the owned-adjacency)
- [ ] T5 gates: sec needle + parse gates · sec-module trigger tests at the sec feature posture · lib tests default + sec posture · clippy −D ×3 postures
- [ ] T6 count pins + AGENTS.md sec bullet + README/.docs ledger rows for corpus 19→22
- [ ] T7 retrieval floors re-measure (corpus delta) — HELD/no-re-pin expected per B258; re-pin with attribution if moved
- [ ] T8 research note finalized (329 addendum with the landing commit) + queue snapshot landing row
- [ ] T9 commit+push develop, `feat:` prefix (corpus delta), single unit with T5–T8
- [-] Secret-scan AI-key vocabulary probe (penthera `secrets.json` as positive-detection vocabulary for `secret_blind.rs`) — DEFERRED to its own unit: scanner lane, not corpus; needs its own Batch-33 probe against real secret corpora
- [-] Agent-tooling audit phase for the katgpt-rs `security-audit` skill — DEFERRED: sibling-repo edit, owner-directed; citation recorded in Research 329
