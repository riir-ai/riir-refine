# 417 — Security-Tools Sec Transcription Batch (nominal Batch 422)

**Status:** LANDED — Batch 422 (2026-11-26): T0–T8 done; the authoring lane's yield: 5 rules (tranche-A minus T-A3, which dropped at its pre-registered scope ruling — sec is `.rs`-only, the CI-YAML surface routes to a scan-lane check recorded never landed); tranche B (9 contestables) + the faces NOT LANDED, drop paths named in the plan, staged for a future sec window; fixtures b422_sec 5×4 arms standalone pre-entry (debug AND -O, the gate caught the F4 final-component check serving .git/config); gates: sec lib 4505/0 + default 4003/0 + no-default 1085/0, clippy −D ×2 postures clean, retrieval floors HELD (32/0/3 latent, no re-pin), fmt PASS, pins_manifest --write 94→99 + prose AGREE same-commit, numbering + dual-allocation + claim lint clean; count web one-landing (AGENTS sec head + README table + .docs/11_domains/sec.md Batch-422 row + health_views total 1844→1849). Score-bench NOT owed (zero clippy_lints delta asserted); release-set rebuild NOT owed (sec opt-in, not RELEASE_FEATURES — the B420 convention). **Claimed:** zed-5426a3b4@amo, 1795670039 — **lease FOLDED at the landing**: session zed-5426a3b4@amo, epoch 1795670039, was expires 2026-11-28T05:13:59+00:00; scope discharged in-window (T0–T8 in one session, 2026-11-26 12:13–13:5x +07).

## Goal

Execute the security-tools sweep distill (Research 451): transcribe the YES-band candidates into the
`sec` corpus as suggest-only rules with ORIGINAL Rust fixtures — the penthera/B258 shape. Sources and
candidates below; the batch lane decides tranche size under the 4–6-per-batch discipline.

## Sources + candidates

**Tranche A (the leads, near-certain):**
- [x] T-A1 `unicode-deception-trojan-source-identifiers` — NVIDIA/SkillSpector B.3.2 (B−). RTL **LANDED `unicode-deception-trojan-source-identifiers` (corpus.rs, the SkillSpector pin 02894ec5; the all-fields tamper arm + the homoglyph arm in the fixture).**
  overrides U+202E/D + U+2066–2069, zero-width U+200B/C/D + soft hyphen + CGJ + word joiner,
  homoglyph confusables, mixed-script identifiers, in identifiers AND shipped text metadata. Original
  Rust fixture: a struct/field named with an embedded RTL override that reads as a different name +
  the ASCII-restricted identifier scan as fixed arm.
- [x] T-A2 `nested-archive-unbounded-depth` — trufflehog archive.go (B−). Depth counter through **LANDED `nested-archive-unbounded-depth` (corpus.rs, the trufflehog pin 23255461 AGPL transcription-only; the counted-metric + typed-refusal + double-increment tamper arms).**
  extract/decompress recursion + typed refusal + counted metric. ⚠ AGPL-3.0 source — transcription
  only. Compose in-rule vs `unbounded-read-into-vec` (per-stage volume vs depth axis).
- [x] T-A3 `ci-action-ref-unpinned-to-commit-sha` — anthropics/claude-code-security-review commits **DROPPED at the pre-registered scope ruling — sec is `.rs`-only; the CI-YAML `uses:` surface routes to a scan-lane check, recorded never landed (the plan's own falsifier branch).**
  40616dd2/0c6a49f1 + strix cb581bf9 DUAL-SOURCE (B−). `uses:` ref not 40-hex SHA → suggest-only.
  ⛔ SCOPE DECISION FIRST: needs the YAML/CI path-scope ruling (docker-domain non-Rust precedent) —
  if sec stays `.rs`-only, this routes to a scan-lane check instead (record the decision either way).
- [x] T-A4 `consent-digest-bound-approval` — strix AGENTS.md (B−). approve(digest) refuses on drift; **LANDED `consent-digest-bound-approval` (corpus.rs, the strix pin 9ed84f99; the owed implementation quotes CLOSED in-source at source_scan.py:206-222 at T0).**
  blanket consent stamps only the invocation's own snapshot. ⚠ LICENSE FILE VERIFY at T0 (badge
  only). Implementation quotes owed at T0 (AGENTS.md-sourced verdict).
- [x] T-A5 `static-handler-serves-dotfiles` — nuclei (B−). Static handler refuses dot-prefixed **LANDED `static-handler-serves-dotfiles` (corpus.rs, the nuclei pin 12d12a40; the gate caught the final-component-only arm serving .git/config pre-entry).**
  components, `.well-known` exemption. Compose in-rule vs `path-join-escape`.

**Tranche B (contestables, drop paths named) — `- [-]` STAGED for a future sec window (the tranche-size discipline: the 4–6 cap consumed by tranche A's five; drop paths pre-named):**
- [x] T-B1 `probe-unknown-tri-state` — **LANDED in Batch 422** (the tranche-A promotion — it rode the
  five, corpus.rs; the falsifier never fired).
- [-] T-B2 `capability-declaration-drift` — SkillSpector B.3.1 (B−; drop risk = review-checklist).
  Declared capability list vs detected categories; LP3 = no declaration at all. Adjudicate vs
  callable-naming + agent-child.
- [-] T-B3 `wholesale-env-harvest-to-egress` — SkillSpector README E2 (B−). env::vars() enumeration →
  outbound serialization; child-env= carve-out is decision content. Adjudicate vs argv rule +
  unvalidated-egress-url.
- [-] T-B4 `package-source-redirection` — SkillSpector SC10 (B−). ⛔ ANTI-REGIME FIRST: committed
  `[patch.crates-io]` + local `vendor/` trees must be carved or the rule self-flags this workspace.
- [-] T-B5 `predictable-temp-dir-security-operation` — semgrep temp-dir.yml (B−). Unique name
  (pid/OsRng) + create_new/O_EXCL; security-operation scoping. ⚠ LGPL-2.1 + Rules License —
  transcription only. Compose vs secret-file-create-without-0600.
- [-] T-B6 `auth-header-not-marked-sensitive` — semgrep reqwest-set-sensitive.yml (C+ drop-or-keep).
- [-] T-B7 `sandbox-bypass-read-path` — nuclei loader.go (B−/C+). Unsandboxed view reader
  unrepresentable from the exec path. Adjudicate vs execution-artifact-trust-ladder.
- [-] T-B8 `ambiguous-outcome-staging-retention` — strix AGENTS.md (B−). Cleanup only on definitive
  outcomes; typed Unknown{id} + reconcile path. Adjudicate vs agent-child REMOTE-STATUS-STANDIN face.
- [-] T-B9 `batched-query-order-limit-pairing` — PyRIT #4285 (C+; domain-routing decision owed —
  concurrency-correctness; scorer-file read owed at T0). **DROPPED at the T0b read (2026-11-26):**
  the #4285 surface is a SQL PAGINATION law (attack_analytics_query.py — order_by(typed keys/ties)
  + offset + limit+1, the extra-row sentinel), not a Rust-shaped decision contract; the
  domain-routing decision lands NEGATIVE (concurrency-correctness is the wrong home; a pagination
  tie-break law is rust_perf/complexity shaped at best, and the corpus's rate-limit rows own the
  adjacent axis).

**Faces (count-neutral, ride their owners) — `- [-]` STAGED beside tranche B:**
- [-] T-F1 trust-refusal-ordered-before-capability → face on `execution-artifact-trust-ladder`
  (nuclei loader.go verbatim).
- [-] T-F2 suppressed-findings-attribution → face on the measurement-honesty derived-totals family
  (nuclei + trufflehog filtered_unverified dual-source).
- [-] T-F3 env-args-non-utf8-panic → face-or-drop (semgrep).
- [-] Positive-instance bank (~15: clamav ×6, trufflehog ×3, nuclei ×6, trivy ×1) — recorded in
  Research 451, cited in the touched rules' notes where a face lands, never re-landed as rules.

**Explicitly NOT in this batch:** clamav/codex-security/bla (NO corpus — Research 451 carries the
reasons); trivy (MARGINAL — its 2 widening faces can ride a docker-domain lane's detector read);
PyRIT eval-lane intel (reflex/rethink lane owners); scanner-lane adoption intel (skillspector +
trufflehog detection catalogs, Issue-151 law); codex-security re-ask needs license verification first.

## Protocol (per the riir-refine skill)

- [x] T0 per source: re-clone into `.raw/<repo>`, re-pin sha + license (AGPL/GPL/LGPL sources
  transcription-only, never paste), byte-verify every Research-451 quote at the pin; tips move daily
  on skillspector/strix/nuclei/PyRIT — moved HEAD = re-pin + re-verify BEFORE authoring.
- [x] T0b unread residue (strix `strix/report` + upload code read — the owed implementation quotes CLOSED at source_scan.py:206-222; PyRIT/nuclei-template/trufflehog-analyzer/trivy/ccsr residues stay with their staged tranches) per Research 451's per-source caveats (strix `strix/report` + upload code;
  PyRIT scorer files; nuclei template YAML + loader bodies; trufflehog analyzer path; trivy detector
  internals; ccsr prompts.py).
- [x] T1 dedup re-verification at landing HEAD: every candidate 3+ vocabularies over
  `src/domains/sec/corpus.rs` (+ rust_perf for T-B9), each hit read and ruled in-row; dedup-at-mining
  binds.
- [x] T2 authoring: suggest-only entries, `bench_spec: None` (NotBenchable-by-domain — the dial9/136
  annotation class, never a demotion); every rule carries its source provenance + license + the
  in-rule adjudications named above.
- [x] T3 fixtures: ORIGINAL minimal Rust reproductions (Constraints law), standalone-compiled AND
  executed BEFORE entry (debug AND -O, four-arm discipline: buggy/fixed/all-safe/tampered).
- [x] T4 gates: batch needles + parse gate; default lib; clippy −D × postures; retrieval floors
  (re-pin only if moved, denominator MUST follow); fmt on touched files.
- [x] T5 count pins one-landing: pins_manifest --write + AGENTS + README + .docs/11_domains +
  health_views + narrative heads (sec 94 → target per landed count).
- [x] T6 freeze/score-bench NOT owed — zero clippy_lints delta asserted (all five rules suggest-only sec; the B398/B420 convention).
- [x] T7 Research 451 → RECORD status + queue snapshot batch-landed row + `.raw/*` removed at close. (Research 451 Status → LANDED with the landed-count row; snapshot CLAIM row appended at the landing commit; `.raw/sectools` removed at close.)
- [x] T8 commit+push; dual_allocation + numbering gates before push; release-set binary rebuild NOT owed (sec is opt-in, NOT in RELEASE_FEATURES — the B420 convention asserted; the standing disclosure rides the next maintained-binary lane). (dual_allocation 0/0/0 + numbering PASSED + claim lint clean at the landing push.)

## Pre-registered falsifiers

- T-A3 fails the scope ruling → reroutes to scan-lane check (recorded), never forced into sec.
- T-B2 cross-file detector refuses to bind → review-checklist fallback (the B378 detector-note
  posture), demote in the same commit.
- T-B4's vendored anti-regime cannot be pinned cleanly → DROP (the self-flag defect is
  disqualifying).
- T-B9 scorer-file read finds no Rust-shaped pairing contract → DROP.
- Any G1 fixture that cannot separate buggy from fixed honestly → the rule dies loud, never ships.

**Lease closed:** zed-5426a3b4@amo, 1795686007, was expires 2026-11-28T09:40:07+00:00 — tranche B LANDED same session (Batch 423; the lease folds at the landing per the reshape law; scope discharged: 4 rules + the B1-recheck + 3 drops)
(scope: tranche B — the plan's 9 contestables + the 3 faces + the positive-instance bank, nominal Batch 423, per the boss routing via zed-b636a705@m3; T0 re-clones + quote re-verifications + T1 dedup + T2 authoring-with-falsification + T3 fixture writing proceed zero-contention; fixture EXECUTION + T4 gates on the loaded box use the isolated-target precedent from the Batch 422 landing same-day; yield ACTUAL 4 rules (B3/B5/B8/B6 + B4 = 5 with the source rule; B1 was already 422's) + 0 faces landed; drops B2/B9 + B7-staged-thin — the pre-named paths fired. NOTHING INVENTED — names/grades verbatim from Research 451 + this plan.)

