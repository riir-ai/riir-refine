# 451 — Security-Tools Sweep Distill (12 sources, user-directed queue creation)

**Status:** LANDED — Batch 422 (2026-11-26, Plan 417; tranche A minus T-A3 landed as 5 sec rules 94→99 — unicode-deception / nested-archive-depth / consent-digest-bound / static-dotfiles / probe-tri-state; T-A3 dropped at its pre-registered scope ruling (sec is .rs-only, the CI-YAML surface routes to a scan-lane check); tranche B (9) + faces STAGED for a future sec window with drop paths pre-named in the plan; fixtures b422_sec 5×4 arms standalone pre-entry; pins_manifest 94→99 prose AGREE same-commit)

Wire-level inspection sweep (3 parallel subagent lanes + coordinator verification; NO clones this
session — every source carries the standing re-clone + re-pin caveat at mine time; tips move daily on
several). The user handed 12 security-tool repos and asked for the distill queue. Per-source verdicts
below; the YES-band sources stage into `.plans/417_sectools_sec_transcription_batch.md` (nominal
Batch 422, sec-domain transcription — the penthera/B258 shape: original Rust fixtures, cite
provenance, never paste). Dedup greps run by the lanes AND re-verified by the coordinator at
`src/domains/sec/corpus.rs` (94 rules per `.benchmarks/pins_manifest.json`); every load-bearing
zero-owner claim below carries its own grep evidence.

**Premise correction (controlling):** the user's list item 3 (Cloudflare Security Audit Skill) is
ALREADY MINED — Batch 258 landed 3 sec rules from `cloudflare/security-audit-skill @ c1c8a8c`
(Research 292, plan 262). Item 6 (anthropics/claude-code-security-review) is a DIFFERENT repo and was
NEVER mined (grep `claude-code-security` over `.research/` + `.distill/001` = 0 hits) — it grades
fresh below.

## Verdict table (the 12)

| # | source | verdict | license | yield after ~50% falsification |
|---|---|---|---|---|
| 1 | NVIDIA/SkillSpector | **YES B−** | Apache-2.0 | 3–4 sec rules |
| 2 | Cisco-Talos/clamav | **NO corpus** (corroboration only) | GPL-2.0 | 0 (+6 positive instances) |
| 3 | cloudflare/security-audit-skill | **ALREADY-MINED** (B258 @ c1c8a8c; delta unmeasured) | MIT | — |
| 4 | trufflesecurity/trufflehog | **YES B−** | AGPL-3.0 (transcription-only) | ~2 sec rules (+3 instances) |
| 5 | semgrep/semgrep | **YES B− thin** | LGPL-2.1 + Semgrep Rules License (transcription-only) | 1 probable + 2 C+ |
| 6 | anthropics/claude-code-security-review | **YES B−** | MIT | 1 rule (dual-source) |
| 7 | openai/codex-security | **NO corpus** | UNVERIFIED (blocks any transcription) | 0 |
| 8 | aquasecurity/trivy | **MARGINAL C+** (docker-domain neighbor) | Apache-2.0 | 0.5–1 faces |
| 9 | usestrix/strix | **YES B−** | Apache-2.0 (badge; LICENSE file UNREAD) | 2 sec rules |
| 10 | projectdiscovery/nuclei | **YES B− thin** | MIT | 1.5–2 rules (+2 faces) |
| 11 | microsoft/PyRIT | **MARGINAL C+ / EVAL-LANE INTEL** | MIT | 1 C+ + 3 faces + reflex/rethink intel |
| 12 | qybaihe/bla | **NO corpus** | MIT | 0 |

## 1. NVIDIA/SkillSpector — YES B− (sec transcription)

Two-stage static+LLM scanner for agent-skill bundles (SKILL.md + scripts): 71 regex/AST/YARA/taint
patterns in 17 categories, risk score → SAFE/CAUTION/DO_NOT_INSTALL install gate, MCP server mode,
OSV dependency lookup, resource-bound fail-closed ingest. Python 3.12+, Apache-2.0 (README badge +
LICENSE link), CI + PSIRT policy, "NVIDIA Verified Skills pipeline". **Fast tip: the README moved
68→71 patterns between two fetches in one session; og-metadata sha `81f16b02` unverified — re-clone +
re-pin at T0.** Product-relevant: this workspace ships ~50 SKILL.md agent-skill files — the defensive
authority on our own surface.

Candidates (dedup zero-owner claims coordinator-verified):
- **TP2 `unicode-deception-trojan-source-identifiers` B− LEAD** — docs/B.3.2-mcp-tool-poisoning.md:
  RTL/directional overrides (U+202E/D, U+2066–2069), zero-width/invisible formatting (U+200B/C/D,
  soft hyphen, CGJ, word joiner), homoglyph confusables, mixed-script identifiers, in shipped
  identifiers AND shipped text metadata; remediation verbatim: "Restrict identifiers to ASCII-only
  characters. Remove RTL overrides and invisible formatting from all fields." Regime: Rust allows
  non-ASCII idents (the compiler does not refuse them). Grep `homoglyph|zero.?width|bidi|rtl|trojan|confusable`
  = the 14 hits are ALL the word "invisible" in unrelated prose — zero owners. Distinct-from
  `ascii-token-unicode-word-boundary-scan-miss` (boundary SEMANTICS of a pattern vs hidden CONTENT in
  a shipped name — adjudicate in-rule both directions). TP1's zero-width/base64/data-URI arms fold
  into this rule's hidden-content half.
- **LP1/LP3 `capability-declaration-drift` B−** — docs/B.3.1-mcp-least-privilege.md: a
  manifest-declared capability list (shell/network/file_read/file_write/env/mcp, each a bounded
  needle set) cross-checked against detected code categories; LP3 fires when NO declaration exists.
  Grep `capability|least.?priv` = 5 hits, all `callable-naming-wire-fields` (the capability-NAMING
  axis) + agent-child fence — declaration-drift unowned; adjudicate vs callable-naming (naming axis)
  + agent-child (spawn-fence axis) both directions. Suppression table carried: wildcard ⇒ LP2 only;
  no-list ⇒ LP3, never LP1/LP4. Drop risk: collapses to review-checklist at authoring (the doc's own
  §8 admits regex-only, transitive-free, English-only detection).
- **E2 `wholesale-env-harvest-to-egress` B−** — README E2: `env::vars()`/`vars_os()` enumeration
  flowing into a serialized outbound request. The source's carve-out is decision content: "A copy
  traced only to a child process's `env=` is reported LOW" (child-env handoff is normal). Regime:
  ENUMERATION over all vars, never a named lookup. Grep `env::vars|environ|harvest` = argv-transport
  rule + quantum-HNDL + env-lock prose — payload-mass axis unowned; nearest kin
  `credential-material-in-command-argv` (transport axis) + `unvalidated-egress-url` (URL-choice
  axis), adjudicate both.
- **SC10 `package-source-redirection` B−** — Cargo.toml `[patch]`/`[source]`/`[replace]` +
  `.cargo/config.toml` source replacement; suggest-only review gate. ⛔ ANTI-REGIME LOAD-BEARING FOR
  THIS HOUSE: we vendor forks via committed `[patch.crates-io]` with `vendor/` trees BY DESIGN — the
  rule must carve committed-vendored-with-local-tree patches and fire only on redirection to a
  FOREIGN registry/git source or newly-ADDED rows without a vendored twin, or it self-flags the
  workspace. Pin the anti-regime FIRST or drop.

Drops: SC4/SC5/SC6 external registry/OSV data (scanner-lane); TP4 description-behavior mismatch
(LLM-powered, no bounded static fix-space); AST family (our shell-string + subprocess-pinned-flags
own the Rust analogs); taint TT1/TT2/TT4 (dataflow a parse-only pass can't bind); report-0600 /
atomic-write / no-symlink-parent (owned: `secret-file-create-without-0600`,
`untrusted-download-verify-then-atomic-install`, `sqlite-store-path-nofollow-parent-resolve`);
suppression-file quarantine + registry `unmodeled_changes` (C+, adoption-gate lane intel);
scan-completeness demotion (already house doctrine — "a skip is a deferral, never a green").

## 2. Cisco-Talos/clamav — NO corpus (corroboration record)

GPL-2.0 VERIFIED; C (+ `libclamav_rust`); 7.7k stars; config-level + repo-level read (parser
internals UNREAD). Zero new rules; SIX positive instances recorded, never re-landed — they strengthen
existing rules' provenance at zero cost:
1. `AlertExceedsMax` breach-as-verdict ("files exceeding the MaxFileSize, MaxScanSize, or MaxRecursion
   limit will be flagged with the virus name starting with 'Heuristics.Limits.Exceeded'") = the
   canonical positive pole of `capacity-exceeded-silent-fallback-not-refusal`.
2. PCREMatchLimit/PCRERecMatchLimit = the engine-side match-budget defense beside
   `regex-backtracking-engine-untrusted-pattern`.
3. Signed-CVD verify-then-install + TrustSigned bytecode + FailIfCvdOlderThan = instances of the
   trust-ladder + stale-cert refusal laws.
4. VirusEvent `%f` → `CLAM_VIRUSEVENT_FILENAME` env ("disabled… due to command injection security
   concerns") = the argv→transport fix of the injection family.
5. FollowDirectorySymlinks/FollowFileSymlinks default-no = path-escape posture instance.
6. The four-axis archive-bomb bounds (MaxScanSize 400M / MaxFileSize 100M / MaxRecursion default 17
   max 100 / MaxFiles 10000) = the class provenance FEEDING trufflehog T1's depth/count rule.

## 3. cloudflare/security-audit-skill — ALREADY-MINED

Batch 258 @ `c1c8a8c` (2026-10-20, plan 262, Research 292) — 3 sec rules landed. Delta re-read
unmeasured this session; a return-visit delta check is cheap if the repo moved materially.

## 4. trufflesecurity/trufflehog — YES B− (sec transcription, AGPL-3.0)

AGPL-3.0 VERIFIED (README "Since v3.0, TruffleHog is released under a AGPL 3 license") —
**transcription-only posture: original Rust fixtures, cite provenance, never paste.** Go, 29.6k
stars, 4.7k commits. A NAMED follow-up-pool source (the hindsight verdict's sensitive-data family
names "detect-secrets + trufflehog catalogs") — the pool read folds into that family per the
accumulation clause; **the DETECTION surface itself stays scanner-lane per the Issue-151 law** (never
corpus); everything below is the non-detection surface.

Candidates:
- **T1 `nested-archive-unbounded-depth` B−** — pkg/handlers/archive.go: recursive extract/decompress
  over untrusted archives with NO depth cap (mholt/archives layer). Fix-space: thread a depth counter
  through the recursion + typed refusal past cap + counted metric; regime: decompress→extract loops,
  flat single-stage extractor = anti-regime. Grep `recursion|nested.*archive` = ONE hit, the wasmtime
  renewal-check dropped-set prose (L943) — the DEPTH axis unowned. Distinct-from
  `unbounded-read-into-vec` (per-stage VOLUME cap vs RECURSION-DEPTH axis; compose). ClamAV's
  MaxRecursion/MaxFiles are the class provenance (source 2 above).
- **T4 `probe-unknown-tri-state` B−** — the README verification taxonomy: every detector's live API
  verifier is a fallible probe with a tri-state outcome; folding it to a bool erases Unknown.
  Fix-space: probe fns return `{Confirmed, Refused, Unknown}`; consumers refuse/surface Unknown.
  Regime: external probes whose negative result clears a security decision. Grep `tri.?state` = 0.
  Nearest kin `capacity-exceeded-silent-fallback-not-refusal` (silent-drop axis) — result-type
  collapse unowned.
- Positive instances ×3: archive symlink-entry skip (corroborates
  `untrusted-archive-extract-paths`' symlink-wave note); CVE-2025-41390 clone-to-tmp git-config
  defense ("To guard against malicious git configs in local scanning… clones local git repositories
  to a temporary directory prior to scanning") ≈ `external-helper-config-pinned-invocation`;
  verification-cache ≈ rust_perf `singleflight-identical-work-per-content-key`.
- Drop: `trufflehog:ignore` tag + SARIF notes (UX); per-entry error-continue + panic recovery (C+,
  tolerance policy review-shaped).

## 5. semgrep/semgrep — YES B− thin

Engine LGPL-2.1 (verified); the rules registry `semgrep/semgrep-rules` is under the NON-STANDARD
"Semgrep Rules License" — transcription-only posture with the license prominently noted; original
fixtures. **Product-coverage note is the real value: the ecosystem's top-tier SAST ships NINE
community Rust rules, all single-pattern, severities INFO×7/WARNING×2, mostly audit — against our
94-rule sec corpus with fix-space drafting.** OCaml core + Python CLI (internals UNREAD beyond the
README's CE single-function/file limitation).

Candidates:
- **S1 `predictable-temp-dir-security-operation` B−** — rust/lang/security/temp-dir.yml verbatim:
  "temp_dir should not be used for security operations… creation of any files or directories in the
  temporary directory must use a secure method to create a uniquely named file." Fix-space: unique
  name (pid/OsRng) + `create_new`/O_EXCL; regime: multi-user unix /tmp, security-relevant files;
  anti-regime: per-user temp dirs. Grep `temp.?dir|mktemp` = ONE hit, the agent-child scratch
  prose (L3346) — the NAME-uniqueness/exclusivity axis unowned; compose note:
  `secret-file-create-without-0600` owns the PERMISSIONS axis. (The katgpt-rs
  `shared_temp_path_gate` is a sibling gate, not a refine corpus rule — the same class arriving from
  a THIRD independent source is corroboration, and the security-operation scoping here is the new
  axis.)
- **S2 `auth-header-not-marked-sensitive` C+** — reqwest-set-sensitive.yml: `set_sensitive(true)`
  before inserting an Authorization header (log-redaction surface unowned; credential-argv owns
  headers on COMMAND lines — different surface).
- **S3 `env-args-non-utf8-panic` C+ face-or-drop** — `env::args()` panics on non-UTF-8 argv →
  `args_os`; semgrep's own CWE-807 framing has no fix-space, the panic face is the salvage.
- Kills (dedup evidence): `insecure-hashes.yml` → `weak-hash-to-blake3` owns (ours name-and-dep
  scoped, MORE precise); `reqwest-accept-invalid.yml` → `egress-control-bypass-flag` owns VERBATIM
  (corpus names `danger_accept_invalid_certs(true)` + live fixture b342); ssl-verify-none/rustls
  YAMLs UNREAD (500s) — name-inferred face text on B342, never new rules; unsafe-usage audit +
  current-exe (live FP: our own `src/update_check.rs:725` uses `current_exe()` legitimately) — no
  fix-space.

## 6. anthropics/claude-code-security-review — YES B− (fresh grade; never mined)

MIT; Python + GitHub Action YAML; ~8 files; **tip `0c6a49f1` 2026-02-11 — 9 months stale** (frozen-tip
mining, no freshness race). A diff-aware PR security review Action with a two-stage FP filter
(findings_filter.py read: hard regex exclusions → per-finding LLM confidence).

- **C1 `ci-action-ref-unpinned-to-commit-sha` B− — DUAL-SOURCE convergence** — commits
  `40616dd2`/`0c6a49f1` (2026-02-11) verbatim: "Pin all versions of any uses: in the sast,
  test-claudecode.yaml and action.yaml files — Instead of using @Version, pin all github actions uses
  to a specific commit hash. This makes is that much harder for a supply chain attack to occur." AND
  strix independently landed the same rule in its own sandbox (`cb581bf9`, 2026-11-04: "add Strix
  rule for GitHub-owned actions not pinned to a commit SHA") — two independent sources, the
  strongest provenance shape. Fix-space: `uses: owner/action@ref` whose ref is not a 40-hex SHA →
  suggest-only "pin to the release's immutable SHA". Grep `uses:|commit.?sha` = 0. ⛔ SCOPE CAVEAT:
  sec scans `.rs` spans; this needs a YAML/CI path-scope decision recorded first (the docker
  domain's non-Rust-surface precedent shows the seam exists). Exceptions for official actions are
  policy, not detector.
- C2 `filter-fail-open-to-keep` C+ ("# Claude API call failed for this finding - keep it with
  warning" + default `confidence_score: 10.0`): an automated veto stage that cannot run defaults to
  NOT vetoing (keep + disclose, never silent-drop); the house twin is EscalationDispatch's
  StoodByModelless — consumer coverage thin; C+ lead only.
- Drops: the FP-suppression exclusion list (policy, not vulnerability classes);
  language-applicability exclusions (already house doctrine — the not-transferable discipline);
  `--disallowed-tools` least-privilege (policy); the audit-prompt OWASP list (owned/architectural).

## 7. openai/codex-security — NO corpus

License UNVERIFIED (README shows none; npm page unfetched) — **blocks any transcription until
verified**. TS CLI client for a CLOUD-HOSTED scan service: `cs login/scan/dedupe/policy`, TS SDK,
SARIF export; the detection meat is cloud-side; in-repo = transport + SQLite dedup plumbing.
Candidates inspected and dropped: validation-before-report (hosted-agent posture, no in-repo
mechanism); `stopAfterNoNew/maxDiscoveryRuns` (harness posture); embedding-similarity findings dedup
(our score-bench lanes own the discipline); SECURITY.md draft-outside-tree (owned
verify-before-promote genus). `src/` + SDK internals UNREAD. A deep dive could surface an
ingestion-validation class, but the README evidence says client shell — not worth a batch. Re-ask
requires: license + src tree first.

## 8. aquasecurity/trivy — MARGINAL C+ (docker-domain neighbor)

Apache-2.0 VERIFIED; Go; docs v0.67 visible. Unified scanner (CVE/SBOM/secret/misconfig). **The
Dockerfile misconfig axis is FULLY DEDUPED** vs the docker corpus's hadolint-derived set (31 rules):
DS001 tag-pin ≈ `base-tag-not-digest-pinned`; DS002 USER-root ≈ `runtime-stage-missing-user`; DS004
HEALTHCHECK ≈ `healthcheck-tool-absent-in-runtime`; secrets-in-env ≈ `arg-secret-to-secret-mount`
(possible count-neutral WIDENING FACE: ENV vs ARG at detector-read time); ADD-remote ≈ DL3020/3021
(possible ADD-tarball face); apt/apk cache + MAINTAINER all owned. **K8s checks (KSV*) are OUT OF
DOMAIN** — a k8s corpus is a NEW-DOMAIN decision needing its own oracle (e.g. kube-linter), never a
rule batch. C+ rows: tar-whiteout handling (layered-tar walker ignoring `.wh.` entries — OCI-layer
consumers only, niche); counting-reader streamed-size refusal = face on `unbounded-read-into-vec`;
checks-bundle download = `untrusted-download-verify-then-atomic-install` positive instance.
⚠ CAVEAT: a Mar-2026 "Trivy supply chain compromise" Docker Hub blog surfaced in search UNREAD —
verify before citing as incident fact. Detector internals UNREAD (listing 500'd twice).

## 9. usestrix/strix — YES B− (sec transcription)

Apache-2.0 (README badge; LICENSE file UNREAD — verify at T0); Python 3.12+, very active (tip
`4719329` 2026-11-13, v1.7.1 2026-11-10). AI pentest agents in a Docker sandbox; multi-agent
orchestration, SARIF reports, auto-fix PRs. The `strix/skills/` attack knowledge packs are the
standing DROP class (playbook, no bounded detector vocabulary). Harness classes:

- **R1 `consent-digest-bound-approval` B−** — AGENTS.md verbatim: review
  `scans start --source . --dry-run --show-files --json`, capture `source.archive_sha256`, then
  rerun with the same selection flags plus `--approve-sha256 HASH`; "A changed snapshot is
  rejected"; "`--yes` approves only the snapshot built in that invocation". Fix-space: an approval
  API binding consent to the reviewed content's digest — `approve(digest)` refuses on drift; a
  blanket consent flag stamps only the invocation's own snapshot. Grep `consent|approve` = the
  false-consent surface (callable-naming) + cookie/secret vocabulary — consent-DRIFT-BINDING
  unowned. Distinct-from `untrusted-download-verify-then-atomic-install` (DOWNLOAD
  verify-before-install vs UPLOAD consent-binding twin — adjudicate both directions in-rule).
  Regime: agent-initiated material uploads gated on human/second-step review; CI handoffs.
- **R2 `ambiguous-outcome-staging-retention` B−** — AGENTS.md verbatim: "A staged upload is deleted
  after a definitive rejection, but retained when a network error, `5xx`, malformed success response,
  or interruption leaves the scan launch ambiguous… JSON reports its `upload_id` with
  `launch_outcome_unknown: true`, or with `cleanup_unknown: true` when automatic deletion cannot be
  confirmed. Check `scans list` before retrying; if no scan is linked, run `uploads delete
  UPLOAD_ID`." Fix-space: cleanup only on DEFINITIVE outcomes; ambiguity ⇒ retain + typed
  `Outcome::Unknown{id}` + a reconciliation path; anti-pattern pinned: blind retry after ambiguous
  launch. Grep `ambiguous|reconcil` = secret-name ambiguity + approval-argv — the axis unowned.
  Kinship mandatory: agent-child's REMOTE-STATUS-STANDIN face (a stub that reads as success must be
  killed — the don't-let-ambiguity-read-as-a-verdict genus at the exec sink); THIS owns the CLEANUP
  side — adjudicate in-rule.
- C+ rows (honest): per-CVE required-verdict-no-unknown ("there is no unknown") CUTS AGAINST the
  house Undecidable-preserving law (ESC VoI) — likely DROP at authoring on the doctrine conflict;
  authority-actions-end-at-human (payment/DNS/OAuth never completed by the automation —
  review-checklist); workspace-switch-never-widens-past-login-ceiling (thin).
- CI-pinning rule (their `cb581bf9`) counted under source 6's dual-source row, never a separate
  candidate. ⚠ Caveat: both B-grades sourced from AGENTS.md + commit messages — implementation
  quotes owed at T0 (`strix/report` + upload code).

## 10. projectdiscovery/nuclei — YES B− thin

MIT VERIFIED; Go, 33.3k stars; **default branch `dev` — moving, re-pin against dev**. YAML-template
scanner engine; the DSL is detection-side (no Rust fix-space); the ENGINE's trust/sandbox mechanics
carry the candidates:

- **N1 `static-handler-serves-dotfiles` B−** — class attested by the README template table
  ("Secret files or data exposure | Sensitive File Disclosure"); the template YAML itself UNREAD
  (404). Fix-space: a static file handler refuses dot-prefixed path components (with `.well-known`
  exemption); regime: any static-serving fn. Grep `dotfile|hidden.?file` = 0. Distinct-from
  `path-join-escape` (caller-path traversal vs the hidden-file component surface — adjudicate
  in-rule).
- **N2 `sandbox-bypass-read-path` B−/C+** — pkg/catalog/loader/loader.go verbatim: "ReadTemplateFromURI
  should only be used for viewing templates and should not be used anywhere else like loading and
  executing templates there is no sandbox restriction here" — an UNSANDBOXED second reader enforced
  by doc comment only. Fix-space: make the unsandboxed path unrepresentable from the exec path
  (sealed type / private parameter). Grep `sandbox` = RUSTSEC headers + agent-child — the dual-path
  bypass axis unowned; distinct-from `execution-artifact-trust-ladder-before-deserialize`
  (trust-before-deserialize vs dual-path bypass — adjudicate).
- F1 `trust-refusal-ordered-before-capability` — loader.go verbatim: "code-protocol-based templates
  run arbitrary commands, so an unsigned one must be reported as an unverified code template before
  the generic missing-capability gate can classify it as just missing -code" → FACE on
  `execution-artifact-trust-ladder`: classify the WORST reason first.
- F2 `suppressed-findings-attribution` — loader.go: "Without it the exclusion is silent at every
  verbosity level" + trufflehog's `filtered_unverified` ("unverified but would have been filtered
  out") — FACE on the measurement-honesty derived-totals family: a filter that drops findings must
  count + attribute them (two independent sources, count-neutral).
- Positive instances ×6: TrustedTemplateDomains egress allowlist; `-mup` shape-keyed enumeration cap
  (≈ request-amplification); `-lfa`/`-lna` sandbox caps (≈ agent-child); `-redact` default-ON with
  `-nrd` demote (≈ redaction-audit posture); `-hpd` honeypot match-concentration (≈ A/A
  calibration); `-ac` catch-all auto-calibrate. Plus an intel record: the strict validator disabled
  in-tree to unblock CI ("TODO: until nuclei-templates#11324 is deployed…").

## 11. microsoft/PyRIT — MARGINAL C+ / EVAL-LANE INTEL (reflex/rethink)

MIT VERIFIED; Python; tip `066b4e7` 2026-11-25. Red-team orchestrator (targets/converters/datasets/
scorers). The score layer read (`pyrit/score/__init__.py`): ~30 true_false/regex output scorers,
BatchScorer, `scorer_selection` (`NoCompatibleScorerError`, `ScorerMismatch`), `scorer_stability`
(`compute_scorer_stability`, `PresentationEffect`), typed `expectation_errors`
(AmbiguousConditionError / MissingConditionError / ExpectationMismatchError), fallback scorer chains,
`find_objective_metrics_by_eval_hash`.

- **P1 `batched-query-order-limit-pairing` C+** — commit `aef5401` verbatim: "FIX: hold the limit and
  order of batched queries across batches (#4285)" — results of an unordered concurrent batch
  mispair with inputs. Rust face: `buffer_unordered`/`join_all` collected in arrival order where
  results pair positionally → index-stamped reassembly (or `buffered`). Zero pairing owner in
  rust_perf (nearest `parallel-disjoint-pread` = order-independence BY CONSTRUCTION for disjoint
  writes — different axis). Domain routing awkward (concurrency-correctness, not perf/sec); scorer
  sources UNREAD — grade rests on the commit title + BatchScorer's existence; file-read owed before
  authoring.
- Faces: linear-time scorer default patterns (`861a69e` LDAP null-byte, `a453984` XXE) = second/
  third instances beside `regex-backtracking-engine-untrusted-pattern`; fanout fail-fast (#4538) =
  second instance of `fanout-first-error-leaves-siblings-running`; converter work off the event loop
  (#4318/#4251) = instances of the blocking-in-async family. Attack converters/datasets (AsciiSmuggler,
  AttaQ, AIR-Bench) = the standing playbook drop.
- **EVAL-LANE INTEL for reflex/rethink (named, not filed — the lane owners adjudicate):**
  (1) `PresentationEffect`/`compute_scorer_stability` — score flip under presentation variants = the
  same axis as reflex's `--gate-stability`/`--perm-probe` (corroboration + vocabulary);
  (2) `NoCompatibleScorerError`/`ScorerMismatch` — typed (dataset, scorer) compatibility refusals
  BEFORE a run; (3) typed `expectation_errors` = the verdict-authority vocabulary our typed-abstain
  lanes already model; (4) `find_objective_metrics_by_eval_hash` = the BLAKE3-row identity law's
  corroboration; (5) fallback-scorer chains = the ESC modelless→escalate shape.

## 12. qybaihe/bla — NO corpus

MIT (README); TS/Node (`bla-agents` npm); tip `41f00295` (approx, og metadata); last-commit date
UNREAD (atom fetch failed twice). PRODUCER-SIDE anti-reverse-engineering hardening of one's OWN
release artifacts (decoy/canary injection, analyzer-budget exhaustion against REA ceilings,
sibling-artifact self-sealing, Electron fuses, ed25519-signed attestation). **Wrong polarity for our
corpus: we ship PUBLIC substrate deliberately (katgpt-rs public per Research 003; riir-refine MIT
public per issue 192) — "make analysis expensive" is never "fix a vulnerability"; no consumer
fix-pattern exists.** Tool-lane note only: the analyzer-budget matrix is attacker-capability intel
about OTHER scanners; runtime-equivalence verification for transforms = the lossy-surface
retention-gate law's independent arrival; `bla lint` refusing "single-point integrity checks" in its
own hardening output is self-quality law, NOT transcribable (the stretched CWE-602 analog would FP
over every feature-flag gate we ship). Yield 0. Clean NO.

## Cross-source synthesis

- **GO batch = `.plans/417_sectools_sec_transcription_batch.md`** (nominal Batch 422, sec-domain,
  transcription-only where licenses demand). Tranche A (the leads): TP2 unicode-deception (SkillSpector),
  T1 nested-archive-depth (trufflehog), C1 CI-pinning (ccsr + strix dual-source), R1
  consent-digest-bound (strix), N1 static-dotfiles (nuclei). Tranche B (the contestables): T4
  tri-state probe, LP capability-declaration drift, E2 env-harvest→egress, SC10 source-redirection
  (anti-regime first), S1 temp-dir, S2 set_sensitive, N2 sandbox-bypass-read, R2
  ambiguous-outcome-retention, P1 batch-pairing. Expected yield ~7–9 rules + ~5 faces after ~50%
  falsification, inside the 4–6-per-batch size discipline if split into two tranches.
- **Positive-instance bank:** ~15 recorded instances (clamav ×6, trufflehog ×3, nuclei ×6, trivy ×1)
  strengthening existing rules' provenance at zero rule cost.
- **License summary:** transcription-only postures — trufflehog (AGPL-3.0), clamav (GPL-2.0),
  semgrep (LGPL-2.1 + non-standard Rules License). UNVERIFIED: codex-security (blocks any
  transcription), strix LICENSE file (badge only). Clean: Apache-2.0 (skillspector, trivy), MIT
  (ccsr, nuclei, pyrit, bla).
- **Scanner-lane note:** skillspector's and trufflehog's DETECTION catalogs (and trivy's checks
  bundle) are adoption intel for the `--secret-scan`/scanner lane (Issue-151 law), never corpus.

## Caveats (sweep-level)

Wire-level fetches only — NO clones; pins are approximate (og-metadata shas / commit-atom dates
where seen) and several repos are fast-moving: **re-clone + re-pin + quote re-verification at every
source's T0** (the standing law). Lane-A's og-sha values and dates are provenance leads, not pins.
Unread residues named per source above; the strix B-grades rest on AGENTS.md + commit messages
(implementation reads owed at T0); the PyRIT P1 rests on a commit title (scorer-file read owed).

NOTHING INVENTED — every quote a live fetch this session (3 subagent lanes + coordinator dedup
re-verification at the live corpus); the verdicts, grades, and landings recorded in
`.plans/417_sectools_sec_transcription_batch.md` + the queue snapshot, same commit.
