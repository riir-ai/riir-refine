# 404 — ARTEX sec agent-harness transcription batch

**Status:** open — staged verdict (Research 439), NOT landed

**Claimed:** zed-m3-artex-distill@m3, 1795407218, expires 2026-11-25T18:00Z

Batch: nominal **Batch 405** (the "Batch 404" name belongs to Plan 403's mlx PR #4927 lane). Domain: sec (transcription-only — source AGPL-3.0, Go; fixtures are ORIGINAL Rust syntheses; source never pasted). Source pin `38c7477c51636c9221c78683f4aa9c4f08f65c19` (BaaRai/ARTEX; upstream Autumn-27 removed Oct 2026 — provenance in Research 439).

- [ ] T0 re-clone `.raw/artex` at pin `38c7477c` + tip-move check + byte-quote re-verification per candidate row (R1-R8 quotes in Research 439)
- [ ] T0b unread-residue re-read: `intercept/target_*.go` bodies + `target_dial.go`, `intercept/prompt.go`/`review_context.go`/`risk.go`, `alwaysallow_denylist.go`, `guard/guard.go` (reconcile the guard/ vs intercept/ dual path), `agent/planner.go`/`worker.go`, `llmpool`, `archivestore/s3.go`, `notify/mask.go`, `mcphttp/client.go` — new candidates join only under the 4-6-rule size discipline
- [ ] T1 dedup re-verification at landing HEAD (greps named in Research 439; all adjudications carried in-rule: R1 vs B350/B340/B367; R2 vs B373/shell-string/B393/B342; R4 vs B340/B367; R5 vs untrusted-keyed-state-unbounded; R8 vs B393; the clientIP positive instance cited on `client-ip-from-untrusted-forwarded-header` at batch time)
- [ ] T2 authoring sec corpus entries: R1 `safety-prompt-hardening-code-owned-tail` (lead) + R2 `approval-rule-parsed-argv-prefix-not-string` + R4 `llm-gate-closed-schema-decision-in-code` + R6 `first-run-admin-claim-one-time-token`; face-or-rule gates decided at authoring: R3 fold-into-R2 (bounded re-parse depth clause), R5 face on `untrusted-keyed-state-unbounded` (+ R6's per-account-lockout-DoS face, fold decision), R7 `secrets-at-rest-refuse-start-missing-key`, R8 face-or-rule on `agent-child-unsandboxed-account-authority` (executor identity separation + root-owned script dir); family decision per rule (InputValidation vs a new APPEND-ONLY family for the judge/gate shape)
- [ ] T3 fixtures `b405_sec_fixtures` — original Rust syntheses, standalone-compiled AND executed BEFORE entry (debug AND -O): R1 needs the double-render + seed-clean arms; R2 needs the compound/substitution/wrapper non-match arms + the flag-before-subcommand no-prefix arm; R4 needs the destructive-override + oversize-to-human + unknown-not-guess arms
- [ ] T4 standing gates (clippy -D × postures incl. the sec posture, lib tests, batch needles by name, retrieval floors HELD no re-pin, bare rustfmt on touched files)
- [ ] T5 `pins_manifest --write` + the count web one-landing (sec count, health_views total, AGENTS/README/.docs heads) + annotated pins if any row lands ALL-ANNOTATED
- [ ] T6 assert freeze/score-bench NOT owed (zero clippy_lints delta) + release-set binary rebuild NOT owed (sec is opt-in, not release set) — record the assertion
- [ ] T7 landing: Research 439 status → DISTILLED — batch-405-landed (or the honest NEGATIVE/ungoat row per any demoted candidate) + queue landing row, same unit; ONE feat commit + push develop; `rm -rf .raw/artex` at close + df check

Yield per Research 439: 3-4 rules + 4-5 faces after ~50% falsification. Falsifier discipline: a candidate failing its gate at landing is DEMOTED in the landing commit — grade struck, tier → Withdrawn, queue line updated (ungoat law).
