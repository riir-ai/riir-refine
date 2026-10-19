# Research 292 — Distill verdict: cloudflare/security-audit-skill

**Status:** DISTILLED — LANDED (Batch 258, plan 262; the landing addendum at the bottom of this note)

## Source

- URL: https://github.com/cloudflare/security-audit-skill
- Pin: `c1c8a8c1471069fb0e188eeaff69b8e8db6564a8` (shallow clone HEAD, 2026-10-19, `.raw/security-audit-skill`, removed at close of this verdict's commit per the ephemeral-clone law)
- License: MIT (Copyright 2025-2026 Cloudflare, Inc.)
- Shape: an **agent skill**, not a code repo — 15 markdown files (~2,240 lines: SKILL.md + 12 attack-class files + HUNTING/RECONNAISSANCE/VALIDATION-AND-REPORTING), `report-schema.json` (461 lines), two zero-dependency `.cjs` validators with tests. No Rust, no kernels, no benches.
- Provenance: Cloudflare's own blog "Build your own vulnerability harness" names this as the seed of their fleet-wide harness. First-party practitioner source, current (2025-2026 copyright, active).

## Verdict

**YES B− corpus / lane-intel N/A (non-perf source) / product-coverage HIGH — seeds the sec web-trust family.**

Two consumers of this source, in priority order:

1. **The workspace skill (the user's actual ask, delivered):** an original workspace-adapted audit skill landed at `../katgpt-rs/.agents/skills/security-audit/SKILL.md` — six-phase procedure (recon → coverage-led hunting → candidate validation → structured output → independent record verification → reporting), the three-verdict taxonomy, execution-safety rules, and a Rust-shaped attack-class quick reference. See that file for the adaptation notes.
2. **The sec-corpus batch (staged, Batch 258 / plan 262):** the attack-class files carry several named patterns that transcribe to bounded, Rust-shaped `sec` rules the corpus does not own. This note records them; the plan authors them.

## Why this grades YES despite zero benchable candidates

The strongest candidates all fail GOAT criterion 1 (benchable on the rust_perf Pareto validator / measured-GOAT shape) — **for a DOMAIN reason**: the `sec` domain's acceptance instrument is probe-parity + parse gates + decision content, never a perf validator (the domain has no perf axis; its 16 standing rules are all fix-pattern/suggest-only entries landed exactly this way). Per the skill's two-class split (the dial9 correction, Research 179), a domain-level limitation is a per-rule `NotBenchable` annotation, never a demotion. Graded on decision content × novelty × dedup clearance × yield.

## Ranked candidate table

| # | Pattern | Source file | Grade | Rust shape | Fix space |
|---|---|---|---|---|---|
| 1 | **untrusted-path-join-escape** (path traversal) | DATA-ISOLATION (export/backup/restore classes), DESKTOP-MOBILE (deep links), universal file-handling | **B** | `Path::new(trusted_base).join(untrusted)` (str/&Path from a wire/query/config field) flowing into a `fs::*` sink; `..`, absolute, or Windows-prefix components escape the base | `components()` walk refusing `ParentDir`/`RootDir`/`Prefix` — bounded, mechanical, **in-house precedent** (riir-deployer `48d63f7` landed exactly this walk for the files-row escape fix, found by the 4090 Windows-arm clippy sweep). Suggest-only (the refusal path's error type is the fn's choice) |
| 2 | **webhook-signature-over-raw-bytes** | PROTOCOLS-RPC ("Untrusted producer treated as control plane" + "Envelope and payload identity mismatch") | **B−** | verify(HMAC/Ed25519 sig) computed over a **re-serialized** body (`serde_json::to_string(&parsed)`) or over envelope fields instead of the exact received bytes — canonicalization drift makes attacker bodies verify | fix: verify over the raw received bytes before/independent of parsing. Suggest-only (per-scheme: which key, which fields, timestamp/nonce binding). Distinct from `constant-time-secret-compare` (owned) — that owns the COMPARE primitive, this owns WHICH BYTES the signature covers |
| 3 | **unvalidated-egress-url** (SSRF) | CLOUD-AND-DEPLOYMENT (ingress/runtime config), WEB-PROTOCOL (forwarded-header trust), AI-AND-LLM (tool fetch targets) | **B−→C+** | a URL assembled from user/request/config input reaching an outbound client sink (`reqwest::get(u)`, a transport dial) with no scheme/host allowlist and no private-range refusal → attacker picks the egress target (cloud metadata, loopback admin edges, internal peers) | fix-space is POLICY (allowlist + private-range deny) → suggest-only. Honest recall limits like `unbounded-read-into-vec`'s: parse-provable untrusted-source vocabulary is thin for URLs (framework-specific extraction); the trigger scope needs a named-sink vocabulary |
| 4 | decompression-amplification-cap (rider) | RESOURCE-EXHAUSTION ("Decompression and representation amplification") | **C+** | `GzDecoder`/`ZlibDecoder`/`bz2` feeding `read_to_end`/grow-into-Vec with no post-expansion output cap — a small compressed body expands past any transfer-size check | RIDER on owned `unbounded-read-into-vec`: the law generalizes — "verify limits after every expansion stage". Check adjacency at authoring; fold if the owner rule can carry the face without mushing the trigger |
| 5 | poisoned-lock-fail-open | RESOURCE-EXHAUSTION (unsafe recovery / fail-open amplification) | **C** | `.lock().unwrap_or_else(\|e\| e.into_inner())` discards the poison flag — the process continues on potentially-corrupted shared state after a panic | Expected drop: thin decision content (the guard is one method call; the regime is arguable in single-purpose daemons). Keep only if the batch needs a third suggest-only body |

## Dedup gate — evidence per candidate (grep `riir-refine/src`, both vocabularies, 2026-10-19)

- traversal / `ParentDir` / `components()` / path-join: **zero** sec-corpus hits (16 rules enumerated in full: all Ffi/UnsafeHygiene/DropLeak/CryptoHygiene/wire families). PASS.
- webhook / hmac / raw-body / signature-coverage: **zero** corpus hits (the `secret_ct_eq` proposer owns compare, not coverage). PASS.
- SSRF / egress-url / outbound-fetch: **zero** corpus rules (the `egress` grep hits are the hosted-expert consent gate — product code, a different mechanism). PASS.
- decompression / `GzDecoder` / bomb: **zero**. PASS (rider adjacency to `unbounded-read-into-vec` adjudicated at authoring).
- poison / fail-open: **zero**. PASS (grade is the killer, not dedup).

## Drops (killed at verdict, for transparency)

- **Superlinear regex backtracking** (RESOURCE's flagship computational-amplification class): NOT TRANSFERABLE to the Rust `regex` crate — linear-time by construction (RE2 lineage, no backtracking). The class survives only for hand-rolled matchers/`glob`-style engines; not a corpus rule for the ecosystem's default regex. Recorded as the honest not-transferable.
- **Integer overflow/truncation 64→32** (MEMORY-SAFETY high-hit list): `unchecked-wire-arithmetic` owns the bare-op overflow half; `as`-narrowing is covered by clippy `cast_possible_truncation` (pedantic) → wrong domain (clippy-covered shapes belong to `clippy_lints`, already upstream-covered).
- **Unbounded buffering / read caps / length-prefix**: owned (`unbounded-read-into-vec`, `length-prefix-validate-before-alloc`, `unchecked-wire-arithmetic`). Positive instances of existing rules — not new rules.
- **Timing-safe comparison / API-key verification**: owned (`constant-time-secret-compare`).
- **JWT/CSRF/session-fixation/passkey/OAuth-binding/cache-poisoning/framing classes** (WEB-PROTOCOL's bulk): architectural, framework-shaped, no bounded parse-provable Rust fix-space — they feed the **skill's** checklist, not the corpus.

## Lane-intel axis

N/A — the source is not perf-relevant (no kernel, no serving config, no measured numbers). No league/lane mapping is offered; the league record is untouched.

## Product-coverage axis

**HIGH.** The `sec` family is young (16 rules, all FFI/unsafe/crypto/wire-socket shaped). Candidates 1-3 found its **web/multi-tenant trust family** — the classes the ecosystem's axum/actix/reqwest consumers actually hit (path handling, webhook verification, egress selection) — matching why the corpus carries bevy/wasm/rayon/tokio families at all. Follow-up pool for the family (accumulate into Batch 258 if read before its window):

- OWASP Rust cheat-sheet shapes (path traversal, SSRF countermeasures as implemented in axum/actix extensions)
- `axum`/`actix-web` security advisories (RUSTSEC) — the crash/DoS classes that reached stable Rust web stacks
- the `subtle`/`ring`/`ed25519-dalek` verification-recipes literature (the raw-bytes coverage law's canonical forms)

## Caveats

- The source is a skill (prose + validators), not code — every corpus candidate above is a **transcription**; the upstream carries no Rust fixtures, so each entry needs an original minimal fixture (license hygiene + original-fixture law) and a trigger-scope design pass. That is exactly what plan 262 prices.
- VALIDATION-AND-REPORTING.md, HUNTING.md, RECONNAISSANCE.md were skimmed via SKILL.md's contracts + README (the six-phase summary, verdict definitions, additive-run rules are quoted there verbatim); the three files were not read line-by-line this session — the skill adaptation consumed their structure secondhand. Re-read at any future deep-audit use; the upstream repo is the durable reference, this note's pin is the provenance.
- The validators (`validate-findings.cjs`, `validate-coverage-ledger.cjs`) were not executed (no node run this session) — their contracts are taken from the schema file and README.

## Recording

- Queue line: `.distill/001_mining_queue_snapshot.md` (this commit).
- Batch plan: `.plans/262_batch258_sec_web_trust_family.md` (this commit, staged — tasks open).
- Skill deliverable: `../katgpt-rs/.agents/skills/security-audit/SKILL.md` (same day, katgpt-rs commit).
- NOTHING INVENTED — every class quote above read from the pinned clone this session; every dedup grep run this session.

## LANDING ADDENDUM (2026-10-20, Batch 258 landed)

All three authorable candidates LANDED as suggest-only rules; the rider
folded; the C-grade candidate dropped per the plan. Landed counts + the
per-task dispositions (Plan 262's task numbering):

- T0 re-clone + re-pin: PASS — `c1c8a8c1471069fb0e188eeaff69b8e8db6564a8`
  re-verified byte-exact at the landing clone (zero drift, the B193
  freshness law).
- T1 dedup re-verification: PASS — all four greps re-run over
  `src/domains/sec/` at the landing tree; zero new owners. The
  `reqwest` grep hits are `unbounded_read`'s untrusted READ-source face
  (distinct axis from candidate 3's URL-choice sink face, in-rule).
- T2 `untrusted-path-join-escape` LANDED (B, InputValidation — no new
  family; the plan's prefer-existing-if-honest adjudication). Detector
  `proposers::path_join_escape`.
- T3 `webhook-signature-over-raw-bytes` LANDED (B−, CryptoHygiene).
  Detector `proposers::webhook_raw_bytes`; distinct-from note in-rule.
- T4 `unvalidated-egress-url` LANDED (B−→C+ kept: the trigger scope IS
  parse-shaped without fiction — `reqwest::`/`ureq::` first-segment
  sinks + URL-vocabulary params / `format!`-placeholder locals; the
  framework-extraction honesty is disclosed in-rule, the
  `unbounded-read` style). Detector `proposers::egress_url`.
- T5 decompression rider FOLDED into `unbounded-read-into-vec`
  (count-neutral): decoder see-through in `is_source_expr` +
  `source_label` (`take` stays a taint break — the bounded twin's
  binding form stays silent, test-pinned), concept face
  "verify limits after every expansion stage". One disclosed recall
  limit: the wrong-side-take composition
  (`GzDecoder::new(stream.take(n))`) stays silent (take-anywhere
  suppression) — test-named as an honest limit, not a safe form.
- T6 `poisoned-lock-fail-open` DROPPED (the batch carries three bodies;
  thin decision content — one method call).
- T7 probe PASS: real clippy (pinned 1.98.1 toolchain),
  pedantic+nursery+suspicious+correctness: ZERO lints on all three
  buggy fixtures. Two fixture-artifact lints on the first webhook stub
  draft (`missing_const_for_fn` nursery + `must_use_candidate` pedantic)
  were repaired AT THE FIXTURE (the stub made genuinely non-const,
  fns made private) rather than excluded — the probe claim is a clean
  zero. The doc-presence family (`missing_docs`,
  `missing_errors_doc`, `missing_panics_doc`,
  `module_name_repetitions`) is crate-level `#![allow]`ed in the probe,
  disclosed.
- T8 gates ALL GREEN: sec lib 2848/0; default lib 2700/0; clippy `-D`
  ×3 postures (`--all-targets`, default/all-features/no-default);
  boundary gate 2/2 (probe deps grew `serde_json` + `ureq`); the
  webhook corpus fixture was made SELF-CONTAINED (a non-const private
  `verify_sig` stub inside the fixture) so the boundary gate's probe
  crate compiles it — the corpus parse contract is unchanged.
  score-bench #192 appended FLAT (heal 0.9636, parse 1.0 — the P74
  expectation). Retrieval floors HELD no re-pin: the default state
  blended 125/131/133 of 141 == the B244 measured values; the sec
  posture re-measured n=16 → 19 with all three new spans retrieving
  their rule top-1 (sec 100/100/100). Freeze NOT owed (zero
  clippy-corpus delta; `generations.rs` verified to build from the
  clippy seed corpus only). Maintained-binary rebuild NOT owed (sec
  not in the release set).
- T9 count pins: sec 16 → 19 (6 fixable + 13 suggest-only; suggest-only
  detectors 4 → 7) at AGENTS.md ×2 + README ×2 rows +
  `eval_fixtures_sec.rs` header. Family table UNCHANGED (no new
  family).
- T10 docs: this addendum; the queue-snapshot landing row (same commit);
  `.docs/11_domains/sec.md` Batch 258 section; `.raw/security-audit-skill`
  REMOVED at close.
- T11 commit + push: `feat:` prefix, Session marker in the body.
