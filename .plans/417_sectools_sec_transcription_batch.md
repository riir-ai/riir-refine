# 417 — Security-Tools Sec Transcription Batch (nominal Batch 422)

**Status:** STAGED — plan-first filed at verdict time (Research 451); no corpus edits yet; authoring awaits a free mining slot; claim via lease per the claim-lease law.

## Goal

Execute the security-tools sweep distill (Research 451): transcribe the YES-band candidates into the
`sec` corpus as suggest-only rules with ORIGINAL Rust fixtures — the penthera/B258 shape. Sources and
candidates below; the batch lane decides tranche size under the 4–6-per-batch discipline.

## Sources + candidates

**Tranche A (the leads, near-certain):**
- [ ] T-A1 `unicode-deception-trojan-source-identifiers` — NVIDIA/SkillSpector B.3.2 (B−). RTL
  overrides U+202E/D + U+2066–2069, zero-width U+200B/C/D + soft hyphen + CGJ + word joiner,
  homoglyph confusables, mixed-script identifiers, in identifiers AND shipped text metadata. Original
  Rust fixture: a struct/field named with an embedded RTL override that reads as a different name +
  the ASCII-restricted identifier scan as fixed arm.
- [ ] T-A2 `nested-archive-unbounded-depth` — trufflehog archive.go (B−). Depth counter through
  extract/decompress recursion + typed refusal + counted metric. ⚠ AGPL-3.0 source — transcription
  only. Compose in-rule vs `unbounded-read-into-vec` (per-stage volume vs depth axis).
- [ ] T-A3 `ci-action-ref-unpinned-to-commit-sha` — anthropics/claude-code-security-review commits
  40616dd2/0c6a49f1 + strix cb581bf9 DUAL-SOURCE (B−). `uses:` ref not 40-hex SHA → suggest-only.
  ⛔ SCOPE DECISION FIRST: needs the YAML/CI path-scope ruling (docker-domain non-Rust precedent) —
  if sec stays `.rs`-only, this routes to a scan-lane check instead (record the decision either way).
- [ ] T-A4 `consent-digest-bound-approval` — strix AGENTS.md (B−). approve(digest) refuses on drift;
  blanket consent stamps only the invocation's own snapshot. ⚠ LICENSE FILE VERIFY at T0 (badge
  only). Implementation quotes owed at T0 (AGENTS.md-sourced verdict).
- [ ] T-A5 `static-handler-serves-dotfiles` — nuclei (B−). Static handler refuses dot-prefixed
  components, `.well-known` exemption. Compose in-rule vs `path-join-escape`.

**Tranche B (contestables, drop paths named):**
- [ ] T-B1 `probe-unknown-tri-state` — trufflehog README verification taxonomy (B−). Probe fns return
  {Confirmed, Refused, Unknown}; consumers refuse/surface Unknown.
- [ ] T-B2 `capability-declaration-drift` — SkillSpector B.3.1 (B−; drop risk = review-checklist).
  Declared capability list vs detected categories; LP3 = no declaration at all. Adjudicate vs
  callable-naming + agent-child.
- [ ] T-B3 `wholesale-env-harvest-to-egress` — SkillSpector README E2 (B−). env::vars() enumeration →
  outbound serialization; child-env= carve-out is decision content. Adjudicate vs argv rule +
  unvalidated-egress-url.
- [ ] T-B4 `package-source-redirection` — SkillSpector SC10 (B−). ⛔ ANTI-REGIME FIRST: committed
  `[patch.crates-io]` + local `vendor/` trees must be carved or the rule self-flags this workspace.
- [ ] T-B5 `predictable-temp-dir-security-operation` — semgrep temp-dir.yml (B−). Unique name
  (pid/OsRng) + create_new/O_EXCL; security-operation scoping. ⚠ LGPL-2.1 + Rules License —
  transcription only. Compose vs secret-file-create-without-0600.
- [ ] T-B6 `auth-header-not-marked-sensitive` — semgrep reqwest-set-sensitive.yml (C+ drop-or-keep).
- [ ] T-B7 `sandbox-bypass-read-path` — nuclei loader.go (B−/C+). Unsandboxed view reader
  unrepresentable from the exec path. Adjudicate vs execution-artifact-trust-ladder.
- [ ] T-B8 `ambiguous-outcome-staging-retention` — strix AGENTS.md (B−). Cleanup only on definitive
  outcomes; typed Unknown{id} + reconcile path. Adjudicate vs agent-child REMOTE-STATUS-STANDIN face.
- [ ] T-B9 `batched-query-order-limit-pairing` — PyRIT #4285 (C+; domain-routing decision owed —
  concurrency-correctness; scorer-file read owed at T0).

**Faces (count-neutral, ride their owners):**
- [ ] T-F1 trust-refusal-ordered-before-capability → face on `execution-artifact-trust-ladder`
  (nuclei loader.go verbatim).
- [ ] T-F2 suppressed-findings-attribution → face on the measurement-honesty derived-totals family
  (nuclei + trufflehog filtered_unverified dual-source).
- [ ] T-F3 env-args-non-utf8-panic → face-or-drop (semgrep).
- [ ] Positive-instance bank (~15: clamav ×6, trufflehog ×3, nuclei ×6, trivy ×1) — recorded in
  Research 451, cited in the touched rules' notes where a face lands, never re-landed as rules.

**Explicitly NOT in this batch:** clamav/codex-security/bla (NO corpus — Research 451 carries the
reasons); trivy (MARGINAL — its 2 widening faces can ride a docker-domain lane's detector read);
PyRIT eval-lane intel (reflex/rethink lane owners); scanner-lane adoption intel (skillspector +
trufflehog detection catalogs, Issue-151 law); codex-security re-ask needs license verification first.

## Protocol (per the riir-refine skill)

- [ ] T0 per source: re-clone into `.raw/<repo>`, re-pin sha + license (AGPL/GPL/LGPL sources
  transcription-only, never paste), byte-verify every Research-451 quote at the pin; tips move daily
  on skillspector/strix/nuclei/PyRIT — moved HEAD = re-pin + re-verify BEFORE authoring.
- [ ] T0b unread residue per Research 451's per-source caveats (strix `strix/report` + upload code;
  PyRIT scorer files; nuclei template YAML + loader bodies; trufflehog analyzer path; trivy detector
  internals; ccsr prompts.py).
- [ ] T1 dedup re-verification at landing HEAD: every candidate 3+ vocabularies over
  `src/domains/sec/corpus.rs` (+ rust_perf for T-B9), each hit read and ruled in-row; dedup-at-mining
  binds.
- [ ] T2 authoring: suggest-only entries, `bench_spec: None` (NotBenchable-by-domain — the dial9/136
  annotation class, never a demotion); every rule carries its source provenance + license + the
  in-rule adjudications named above.
- [ ] T3 fixtures: ORIGINAL minimal Rust reproductions (Constraints law), standalone-compiled AND
  executed BEFORE entry (debug AND -O, four-arm discipline: buggy/fixed/all-safe/tampered).
- [ ] T4 gates: batch needles + parse gate; default lib; clippy −D × postures; retrieval floors
  (re-pin only if moved, denominator MUST follow); fmt on touched files.
- [ ] T5 count pins one-landing: pins_manifest --write + AGENTS + README + .docs/11_domains +
  health_views + narrative heads (sec 94 → target per landed count).
- [ ] T6 freeze/score-bench owed only if clippy_lints delta (asserted otherwise, P30).
- [ ] T7 Research 451 → RECORD status + queue snapshot batch-landed row + `.raw/*` removed at close.
- [ ] T8 commit+push; dual_allocation + numbering gates before push; release-set binary rebuild owed
  (sec is release-set surface) per the §0 bin-age law.

## Pre-registered falsifiers

- T-A3 fails the scope ruling → reroutes to scan-lane check (recorded), never forced into sec.
- T-B2 cross-file detector refuses to bind → review-checklist fallback (the B378 detector-note
  posture), demote in the same commit.
- T-B4's vendored anti-regime cannot be pinned cleanly → DROP (the self-flag defect is
  disqualifying).
- T-B9 scorer-file read finds no Rust-shaped pairing contract → DROP.
- Any G1 fixture that cannot separate buggy from fixed honestly → the rule dies loud, never ships.
