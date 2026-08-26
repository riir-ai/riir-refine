# Plan 026 — KAT Phase 0 T0.2: the redaction gate

Executes [Proposal 005](../.proposals/005_kat_heal_service_network.md) **Phase 0 T0.2 only** —
the redaction gate + `.heal-sync-policy` + adversarial fixtures (the G4 local half). Opened
2026-08-26 after T0.5/T0.6/T0.1 (Plans 023/024/025) landed; the session decision picked it over
landing the two pending Proposal 005 amendments (free-tier trainer drain + outcome-anchored
mint — notes recorded in the prior thread, to fold in at the T2 plan-opening).

## Scope discipline

- **In scope:** T0.2 — the gate (`src/kat_redact.rs`), the policy file format + the shipped
  `.heal-sync-policy`, the adversarial fixture gate (`tests/kat_redact_g4.rs`), the harness
  wiring (`fixer_regate` gates every sample before any arm), the CI layer that makes G4
  standing.
- **Out of scope (later Phase 0 tasks, own commits/plans):** T0.3 sync payload schema over
  TrajStore rows (consumes [`RedactionGate::check`] — the consumer this gate waits for),
  T0.4 pending-record estimator.
- **The `heal.rs` main-loop wiring is deliberately NOT here:** the offline heal loop never
  sends anything off-machine. The gate attaches at the surfaces that DO egress (the fixer
  harness now; T0.3's sync payload builder next). Premature wiring would be dead code today.

## Design (from Proposal 005 §Redaction, restated as the contract)

1. **No payload leaves the machine before passing the gate.** The gate is two layers:
   - **Eligibility (deny-by-default):** sync is opt-in per project via
     `.heal-sync-policy`. No file → deny-all. The policy allows by origin
     (`own-repo`/`authored-fixture`/`mined-corpus`; `external` — user-submitted code — is
     absent from the shipped policy by design) and by path glob when the span carries a
     path. `max_span_bytes` (default 64 KiB) enforces span minimization — whole files
     refuse.
   - **Content scan:** 12 hand-rolled rules (NO regex dep — pure std):
     AWS access keys (AKIA/ASIA + ≥16 `[0-9A-Z]`, longer runs also flag — fail-closed),
     GitHub (`gh[pousr]_`) / GitLab (`glpat-`) / Slack (`xox[baprs]-`) / OpenAI-style
     (`sk-`) tokens, `-----BEGIN … PRIVATE KEY-----` whole blocks (the base64 body is key
     material without the header; CERTIFICATE headers do not flag), credentialed
     connection strings (`scheme://user:pass@` — `$`/`{` userinfo = template, skipped),
     Bearer tokens, `.env`-shape secret assignments (identifier-policy word list on
     `_`-segments — `author` does not match, segment-exact `auth` does), Rust
     secret-literal assignments (`let/const/static NAME…= "…"`), high-entropy hex
     (≥32 chars @ ≥3.0 bits/byte) + base64ish (≥32 @ ≥4.5).
   - **Redaction:** findings → `[REDACTED:<kind>]` placeholders, byte-exact outside the
     finding ranges; overlapping findings de-overlap deterministically
     (start-then-longest). Default policy refuses redacted payloads
     (`allow_redacted: false`) — a finding BLOCKS. With `true` the redacted payload may
     sync.
2. **The log cannot leak.** `<root>/.heal/redaction_log.jsonl` (`RIIR_HEAL_REDACT_LOG`
   override) carries finding kind + byte range + `blake3(finding)[..12]` fingerprint —
   never the bytes (impossible by construction: `Finding` does not retain them).
3. **Versioning (G9b discipline):** `REDACT_VERSION` + `REDACT_CHANGELOG`; the pattern
   set + policy semantics are part of the version. A policy file with a different version
   DENIES (fail-closed — a stale allowlist must never accidentally allow; the opposite
   of the meter's fresh-start). Missing policy fields default deny (`sync_enabled`
   defaults false, lists default empty) — a hand-edit that drops a field tightens.
4. **Harness wiring:** `fixer_regate` gates every sample in `main()` BEFORE arm dispatch —
   arms A/B send prompts to CF Workers AI (real egress) and every arm writes corpus rows
   via `record_case`; a blocked span reaches neither. Origins: `reachable:` →
   `authored-fixture`, v2edit → `mined-corpus`. All-blocked → exit 4 (fail-loud; running
   zero samples is pointless). `cf_workers_ai` now implies `kat_service` — the harness
   must never compile without the gate it gates through.
5. **G4 is standing CI:** Layer 5c runs the KAT integration gates under `--all-features`
   (this file + the meter + account G9 binaries — previously none of them ran in CI).

## Honest scope (documented in the module, not papered over)

- No layer-stripping (base64/gzip) — an encoded secret trips only via the entropy rule.
- Committed hash literals in test fixtures (BLAKE3 digests, 64-char hex @ ~3.9 entropy)
  WILL trip the hex rule — fail-closed by design; a future version may add
  allow-by-fingerprint.
- Re-scanning a redacted payload can re-flag the placeholder site (the secret-shaped
  *name* survives value redaction) — fail-closed; treat redacted payloads as terminal.
- The scanner is line/token-based, not AST-based — a secret split across two lines evades
  the env/literal rules (entropy + known formats still catch most such values).
- No alloc-free gate here (unlike the meter): the gate runs per-span pre-sync, off the
  heal hot path; findings/payload allocation is bounded by span count.

## Tasks

- [x] T1 `src/kat_redact.rs` behind `kat_service`: version/changelog, `SpanOrigin`,
  `FindingKind`/`Finding` (fingerprint-only), `scan_secrets` (12 rules), `redact`
  (byte-exact), `SyncPolicy` + `glob_match`, `RedactionGate`
  (`load`/`discover`/`deny_all`/`check`), `RedactionLog` (JSONL) + unit tests
- [x] T2 `src/lib.rs`: module declaration + root re-export
- [x] T3 `Cargo.toml`: `cf_workers_ai` implies `kat_service`
- [x] T4 shipped `.heal-sync-policy` at the repo root (own-repo +
  authored-fixture + mined-corpus; `external` deliberately absent)
- [x] T5 `fixer_regate.rs`: `--policy` flag + `gate_samples` before arm dispatch +
  `.heal/redaction_log.jsonl` recording + exit-4 fail-loud
- [x] T6 `tests/kat_redact_g4.rs`: adversarial fixtures — AWS key (block + byte-exact
  redact), AWS-secret env shape, GitHub token, whole-block private key (body must not
  survive), connection strings, `.env` leakage, credential-shaped Rust literal,
  high-entropy blobs, Bearer/JWT, mixed span (clean code survives byte-identical),
  external-origin deny, no-policy deny, path deny, oversize deny, clean-allow,
  log-never-leaks, determinism, changelog completeness, glob pins, aggregate
  zero-secrets meta-gate
- [x] T7 `scripts/ci_feature_guard.sh` Layer 5c: the KAT integration gates run standing
  under `--all-features`
- [x] T8 validation: `cargo test --features kat_service` (module + G4) + Layer 5a/5b
  equivalent + `fixer_regate --arm DRY` e2e (gate lines + DRY instrument verdict) + clippy
  clean
- [x] T9 Proposal 005 T0.2 checkbox → `[x]` with this plan's record
