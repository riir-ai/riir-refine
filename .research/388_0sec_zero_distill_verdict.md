# Research 388 — Distill verdict: 0sec-labs/0 (full-stack AI security harness)

**Status:** DISTILLED — batch landed (Batch 350, Plan 356; the owner-GO wait discharged per the 2026-11-11 gating decree, delegated reviewer verdict AGREE — session 6a5380ae recorded in the plan). Landed: 5 suggest-only rules (sql-identifier-interpolation-unallowlisted A− lead + predictable-rng-for-secret-material B+ + ecb-block-mode-selection B + jwt-alg-family-confusion-allowlist B− + untrusted-content-interpolated-into-prompt B−, the R6+R7 conjunction's lone survivor folded to InputValidation — NO new SecFamily variant) + the weak-hash co-occurrence rider face + the R5 SSTI drop (the defer-or-drop gate: an ATTACK playbook, no bounded detector vocabulary) + R7 dropped (dispatch-gating has no bounded parse surface). sec 51→56; fixtures `b350_fixtures.rs` 16/16 standalone-compiled AND executed pre-entry; gates: default lib 3842/0, sec 4248/0, clippy -D ×4 postures clean, retrieval floors HELD 29/0 no re-pin, pins_manifest written WITH the corpus change, score-bench runs appended (clippy lane FLAT 96.4%), release-set binary NOT owed (sec not in RELEASE_FEATURES).

## Source + provenance

- **URL:** https://github.com/0sec-labs/0 — "Full-stack AI security OS for your browser, terminal, and agents" (YC-backed, research preview, v0.28.1).
- **Pin:** `f7020eb2c5a271d9f0a46896883487d336603f60` (shallow clone at `riir-refine/.raw/0` 2026-11-09; quotes below read directly at this pin; clone deleted at close — re-clone at the pin for mine time).
- **License:** MIT OR Apache-2.0 (dual `LICENSE` + `LICENSE-MIT` + `NOTICE`). Fixtures for any batch are OUR original Rust reproductions (transcription law — the source is TypeScript/Python, which is never a gate; penthera precedent).
- **Provenance chain of the decision content:** OWASP "Top 10 for LLM Applications" (`owaspLlmTop10: LLM01/06/07` fields on every attack template) + standard CWE classes (CWE-327 weak crypto/ECB, CWE-338 predictable RNG, CWE-347 JWT, CWE-1336 SSTI) + their internal motivation issues (#662 crypto coverage, #104 indirect prompt injection, a 2026-04-06 npm-bench ceiling analysis). No upstream-port chain to cite — these are first-party detector implementations.
- **Upstream sibling:** `0sec-labs/foxguard` — the same org's Rust SAST scanner (NOT inspected this verdict; a named follow-up distill source — see product-coverage pool).

## What it is

A pnpm-workspace TS monorepo: `packages/core` (the audit engine), `packages/templates` + `templates/*.yaml` (30 LLM red-team attack templates), `packages/llm-redteam` (red-team executor), `packages/benchmark` (XBOW/AIxCC-style eval, honest-eval.ts), `test-targets/` (vulnerable demos), and `0verse/` — a research-stage **binary-native Cyber Reasoning System** (~180 Python modules + a pyo3 Rust fast-path) with a PoV-is-truth gate and a curated `NEGATIVE-RESULTS.md`.

**Module-level grading (the thermite law):** the deterministic source-audit detectors and the attack-template corpus are COMPLETE and TESTED (each detector ships a `.test.ts`); `0verse/` is explicitly research-stage and is EXCLUDED from mining scope (binary-analysis machinery — memory-safety/IOCTL classes on compiled C binaries have no Rust source-code fix-space; its `rust/` subcrate is an aho-corasick pyo3 fast-path, a known pattern).

The detector model is a mirror of ours — their header (crypto-misuse-detector.ts, verbatim): *"deterministic static-pattern oracles … a pure static pass that runs BEFORE the LLM agent and emits `Finding` objects"* — and their precision discipline is exactly the corpus's authoring shape: *"each precision-gated to real security-relevant usage to avoid the FP floods we've been fighting — we do NOT flag every MD5"*. They even publish the gate vocabulary (`SECURITY_CONTEXT_RX`: password|passwd|secret|token|api-key|auth|credential|signature|hmac|salt|nonce|session|csrf|otp|mfa|2fa|private-key|derive|pbkdf, with the boundary note *"no trailing `\b`, so camelCase identifiers like `resetToken` / `sessionKey` still match"*).

## Verdict

**YES, B+ corpus** (sec transcription batch) · **lane-intel: NONE** · **product-coverage: YES — seeds the corpus's FIRST agent-security family and fills the web-trust SQLi gap (zero sqli rules today).**

GOAT candidate note (the dial9 two-class law): no candidate is benchable — security correctness is not a Pareto-validator axis, so criterion 1 fails for a DOMAIN reason. Every candidate lands as `NotBenchable`, `bench_spec: None` (suggest-only), graded on decision content × novelty × dedup clearance × yield. That is an annotation, not a demotion; the verdict can be YES.

## Ranked candidate table

| # | pattern (working rule_id) | source file | grade | notes |
|---|---|---|---|---|
| 1 | `sql-identifier-interpolation-unallowlisted` — structural/JSON-key SQLi: `ORDER BY ${key}`, `SELECT ${key}`, dynamic `WHERE ${key}`; values bind, **identifiers cannot** — the fix is an allowlist translation | `packages/core/src/agent/skills/vulnerabilities/structural-sqli.yaml` | **A− lead** | the corpus's biggest web-trust gap: 0 sqli rules against sqlx-tier infrastructure. Two-regime decision content verbatim in-source: *"Values are usually parameterised, so they find nothing. The high-value miss is when the backend concatenates a JSON **key / field name** into SQL — these are almost never parameterised because you can't bind an identifier."* Detector: format!/push_str-built query + sql vocabulary (query/query_as/SELECT/ORDER BY); fix = allowlist map (or const enum → literal). Adjudicate vs `unchecked-wire-arithmetic` (owns arithmetic overflow, not injection) — distinct |
| 2 | `predictable-rng-for-secret-material` | `crypto-misuse-detector.ts` detector 5 | **B+** | Math.random()/random for token/key/nonce/OTP, gated on their security-context vocabulary; *"A `Math.random()` driving an animation is NOT flagged."* Rust face: predictably-seeded PRNG (`seed_from_u64(unix_time)`, `StepRng`, counter seeds) feeding token/nonce/salt material; fix = `OsRng`/getrandom (the house pattern — `password-hash-argon2`'s fixed_code already uses `SaltString::generate(&mut OsRng)`). Suggest-only, security-vocabulary-gated |
| 3 | `ecb-block-mode-selection` | `crypto-misuse-detector.ts` detector 3 | **B** | `aes-*-ecb` / `mode: ECB`; *"ECB leaks plaintext block structure; always security-relevant."* Rust face: `ecb::Ecb<Aes128>` type aliases / ECB-named cipher consts; fix = AesGcm/ChaCha20Poly1305 + random nonce. Suggest-only. Detector-vocabulary hazard: `ecb` is a substring of innocuous hex/hashes — word-boundary the vocabulary |
| 4 | `jwt-alg-family-confusion-allowlist` | `crypto-misuse-detector.ts` detector 4 | **B−** | verify paths accepting BOTH an HMAC and an asymmetric alg (HS-vs-RS confusion) or no algorithms allowlist. **Adjacent-DISTINCT from `jwt-insecure-validation-flags`** (verified at corpus.rs:672 — that rule's buggy code owns the DISABLED axis: `insecure_disable_signature_validation()` + `validate_exp = false`); this owns the WRONG/multi-ALG-FAMILY axis. Mandatory in-rule adjudication note per the B325 discipline |
| 5 | `untrusted-content-interpolated-into-prompt` (agent-family seed) | `packages/benchmark/src/adversarial-indirect-prompt-injection-cases.ts` | **B−** | the indirect prompt-injection class modeled with `attackerControlledField: response_body / result_snippet / file_contents` and `attemptedEffect: tool-selection / data-exfiltration` — *"tool output is untrusted"*. Rust face: prompt assembly interpolating network/file/tool-sourced strings into instruction context unmarked; fix = delimit + label untrusted content (structured blocks). **New SecFamily variant required (families APPEND-ONLY — schema decision, plan gate).** Precision honestly poor (vocabulary-gated hard — the `gateway-type-forces-security-check` posture); review decides |
| 6 | `model-driven-tool-invoke-without-policy-gate` (family sibling) | `templates/mcp-unauthorized-tool-call-20.yaml` + `llm-excessive-agency` skill | **C+** | action-AUTHORIZATION axis: a tool dispatcher executing model-requested privileged ops without a policy gate. Adjacent-DISTINCT from `shell-string-command-interpolation` (owns shell-feature interpolation) and from spillway's `read-output-trust-inherits-argument-taint` (owns output trust); this owns the dispatch gate. Precision-poor; **conjunction-or-demote with #5 — the new family variant lands only if BOTH agent rows survive falsification** |
| — | weak-hash security-context co-occurrence gate | `crypto-misuse-detector.ts` detector 1 | FACE | positive instance of `weak-hash-to-blake3` — their *"we do NOT flag every MD5"* co-occurrence gate is a refinement of ours; land as a count-neutral rider face, never a new rule |
| — | hardcoded key/IV/secret + entropy gate | `crypto-misuse-detector.ts` detector 2 | DROP | routing table: secrets-in-code detection → the scanner lane, never corpus (Issue 151 blind-surface adjacent). Their Shannon-entropy gate + literal-IV always-flag are worth a scanner-lane note, not a corpus entry |
| — | pickle/yaml.load/eval detectors | `unsafe-deser-detector.ts` | DROP | language-absent in safe Rust — typed serde deserialize is the fix by construction; `eval`/`new Function` has no Rust face. Recorded as the honest class-absence |
| — | typosquat / install-script oracles | `malicious-detector.ts` | DROP→scan lane | supply-chain SCREENING (report-only), not a fix-space; the Rust face (malicious build.rs) is detection, never repair. Their ceiling analysis is good intel: *"0's npm-bench malicious-detection rate was structurally stuck at 8% (vs 62.5% on known-CVE packages)"* because the LLM prompt asked only traditional classes — a lesson for OUR oracle-census lane |
| — | refusal classifier / score-only suppression | `triage/llm-output-guard.ts`, `can-auto-suppress.ts` | ANNOTATION | harness governance logic (retry budget, finding suppression), not Rust-code classes |

## Dedup-grep evidence (both vocabularies, run 2026-11-09 at riir-refine HEAD)

- **SQLi:** `sqlx|query_as|SELECT|parameteri` over `src/domains/sec/` → **0 hits**. Clean.
- **RNG:** `thread_rng|OsRng|seed_from_u64|from_entropy|StepRng` over sec → 1 hit = `password-hash-argon2`'s **fixed_code** using `OsRng` (the fix pattern, not a rule). Clean.
- **ECB:** `ecb` over `src/` → only `rust_perf/corpus/entries_19.rs` (TensorFold cost-rule entry — spurious substring, unrelated). Clean.
- **JWT alg:** corpus rule body read — owns disabled-validation only; multi-alg axis unowned. Adjacent-distinct (adjudication note owed at authoring).
- **SSTI:** `tera|handlebars|minijinja|askama|render_template` → only spurious substring hits in `rustc_errors/floor.rs` ("p**era**-it**era**tion"). Clean — AND the hazard is recorded: `tera` ⊂ "iteration", so the rule's detector vocabulary needs word boundaries.
- **Prompt/MCP/tool-call:** `prompt|mcp|tool_call` over sec → **0 hits**. The agent-security family is entirely unowned.
- **Prior verdicts:** `0sec-labs|0verse|foxguard` over `.distill/ .research/ .plans/` → 0 (fresh source).

## Lane-intel axis (mandatory output — recorded NONE)

Mechanisms: crypto-misuse detectors, web-vuln audit skills, LLM red-team templates, binary CRS. The open workspace lanes are inference-perf (prefill/decode/Metal/CUDA/GDN, riir-ai Issues 1146–1187 era). Grep of `../riir-ai/.issues/` for `OsRng|ecb|sqlx` → 0. **No mapping; no league line owed.** The 0verse binary-CRS machinery does not touch any open lane either.

## Product-coverage axis (mandatory output — FIRES)

The candidates live in top-tier Rust ecosystems: **sqlx** (the canonical web-infra gap), **rand/getrandom** (every crate), **RustCrypto aes/ecb**, **jsonwebtoken**, **tera/handlebars/minijinja**, plus the growing **agent/MCP-server** ecosystem (rig, rmcp, swiftide — and OUR OWN escalation chain: `hosted_expert` + `seat_rerank` + `escalation_dispatch` are LLM-consuming Rust the rules fire at home). This is the dial9-shape product verdict: family coverage is first-class even with zero workspace lane mapping.

**Follow-up source pool** (accumulation clause applies — pool sources read before the batch window fold into the same batch):
1. `0sec-labs/foxguard` — the same org's Rust SAST; direct sec-corpus transcription source (its own distill verdict, or a fold-in).
2. OWASP "Top 10 for LLM Applications" — the agent-family's class taxonomy upstream.
3. `promptfoo/promptfoo` — the ecosystem's open-source LLM red-team harness (the famous-lib analog of this source); secondary pool for agent-family rows.
4. RustSec advisory corpus — historical sec source, already cited; the RNG/ECB families' advisory rows corroborate.

## Honest caveats

- **Research preview**, fast-moving (commits 2 days before inspection) — re-clone + re-pin at mine time; quotes above are pinned to `f7020eb2`.
- **Unread files (re-read at mine time):** `sqli-advanced.yaml`, `ssti-exploitation.yaml`, `command-injection.yaml`, `request-smuggling.yaml`, `path-traversal.yaml`, `llm-insecure-output-handling.yaml`, `llm-prompt-injection.yaml` skills; `packages/core/src/{audit,assessment,agentic-scanner,analysis-prompts,remediation}.ts`; `packages/llm-redteam/src/{guards,strategies}`; the rest of `packages/benchmark/src`.
- Their detectors are line-regex heuristics for JS/Python; the Rust detector vocabularies must be RE-DERIVED (their `SECURITY_CONTEXT_RX` transcribes as a vocabulary list, not a regex).
- The 30-skill yaml corpus is AUDIT-PROCEDURE content (the cloudflare-audit-skill shape) — transcription candidates ride the CLASSES it names, not the procedures.
- Consumer-base note: 0 is new-but-notable (YC, trending); irrelevant to the grade — the classes are ecosystem-wide.
- 0verse excluded from scope (research-stage, binary-only classes, no Rust fix-space).

## Queue slot

**Batch 344** (nominal, sec domain) · Plan **356** · queue snapshot line appended in the same commit. Yield estimate after ~50% coordinator falsification: **3–4 rules + 1 rider face** (two-crypto + one-web-trust + the agent family pair on the conjunction gate).
