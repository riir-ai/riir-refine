# Research 292 — Distill verdict: cloudflare/security-audit-skill

**Status:** DISTILLED — pending owner decision (Batch 258 staged, plan 262; corpus edits deferred to the next cargo-quiet slot)

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
