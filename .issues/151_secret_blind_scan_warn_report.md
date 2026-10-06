# Issue 151 — the secret-blind surface: scan, warn (suggest-only), and blind — the consolidated pattern set consumed where the user works

**Status:** OPEN — 2026-10-06 (owner-directed: "add regexp to blind sensitive data — api key, private key, … — in comments or strings; warn in sec but never autofix, suggest like clippy"; scanner half = riir-kat Issue 002)

## The shape

The egress gate (`kat_redact`, via riir-kat Issue 002's v4 pattern set)
already stops secrets from LEAVING the machine (mining sync, CF Workers AI
prompts, reef persistence). What does not exist is the surface the user
works in:

1. **No in-run warn.** A scan run over a file carrying a planted AWS key
   prints nothing — the secret is invisible until egress (and then it is
   silently DROPPED from the mining batch, which is correct for egress but
   tells the developer nothing about their code).
2. **No user-facing report.** gitleaks/trufflehog fill this niche in other
   stacks; here the scanner is reachable only through the sync internals.
3. **No blind transform.** `redact()` exists (byte-exact, deterministic
   placeholder substitution) but no surface hands it to the user — e.g.
   pasting code into an LLM chat, a ticket, or a log.

Today's DISPLAY surface is already secret-safe by inspection: the fix
review prints rule ids + line numbers only (`format_fix_review`), and the
only snippets ever printed are OUR corpus fixtures (`snippet_head`) — no
user span text. This issue keeps that true and adds the explicit surfaces.

## Design (scan-based, NOT corpus-match — the verdict answer)

Detection is deterministic scanning (`scan_secrets`): secret formats are
exact prefixes/shapes, and the byte ranges feed `redact()` directly.
Corpus-match (latent retrieval over buggy fixtures) stays the
FIX-suggestion machinery and is deliberately NOT used here: a "hardcoded
secret" corpus entry would carry a FAKE key in its fixture (tripping our
own scanners), match fuzzily where detection must be exact, and its "fix"
is not a mechanical rewrite (the remedy is a process change — env var /
secret store — never an autofix).

## Tasks

- [x] T1 — SUPERSEDED at verdict round 1 (was: `sec = ["riir-kat/client"]` — REFUSED: `client` drags `account_key` → riir-auth + ssh-key + ed25519-dalek into a std-only scanner). FINAL: riir-kat gains a `redact = []` tier (blake3 already non-optional there, zero new deps); this repo's base dep becomes `riir-kat = { path = "../riir-kat", default-features = false, features = ["redact"] }`; `kat_redact` moves OUT of the `kat_service`-gated re-export list into an UNGATED `pub use riir_kat::kat_redact;` (public path unchanged). The scanner is local-only compute — the config-audit ungated precedent applies; `sec = []` stays (the sec label is the warn's `[sec]` tag + docs, not a gate).
- [x] T2 — new module `src/secret_blind.rs` (UNGATED, pure, zero new deps): `scan_file(text) -> Vec<SpanFinding>` (kind + line:col resolved from byte offset), `blind(text) -> String` (scan + `redact`), `audit_paths(targets) -> SecretReport` (walk + render). ⛔ SHAREABLE-OUTPUT RULE (verdict R1): the finding fingerprint is an UNKEYED `blake3(bytes)[..12]` — safe in the LOCAL JSONL log, a dictionary-reversal leak on stdout (shareable output is exactly what users paste). Every stdout surface (report, warn lines, `--blind`) OMITS the fingerprint; only kind + path:line:col + blinded context. Test: shareable output contains no unkeyed fingerprint of any planted finding. (Recorded future path: `blake3::keyed_hash` with a per-install key if cross-run correlation is ever needed.)
- [x] T3 — CLI `--secret-scan [PATHS…]` (the config-audit lane convention: paths default `.`, report-first, always exit 0 on a real scan, REFUSES exit 1 on an empty walk — never a confident green zero, the frontier-report law). Output per finding: `kind`, `path:line:col`, fingerprint — and the finding's CONTEXT LINE shown BLINDED (placeholder substituted, never the raw bytes). Refuses loud via the build-stamp gate when `sec` is compiled out, naming the rebuild command (the `--refill` precedent).
- [x] T4 — in-run disclosure (suggest/fix runs at `sec` builds): per file with findings, one clippy-style line per finding — `[sec] secret-shaped literal (aws-access-key) at src/config.rs:42 [suggest-only — never auto-fixed; value not shown]` + a run-summary total. Zero findings prints NOTHING (the quiet-by-default law — a warn firing on every clean run is a warn nobody reads). `--fix` never touches these (no proposer exists; a secret's fix is a process change, not a rewrite).
- [x] T5 — `blind()` riding the one text-exit seam: `cargo refine --secret-scan --blind <path>` prints the whole file text with every finding byte-replaced by its `[REDACTED:<kind>]` placeholder (byte-exact outside findings — safe to paste into a ticket/chat/LLM). Read-only (never writes; in-place mutation would destroy code). `--blind` without `--secret-scan` = usage error.
- [x] T6 — tests: positive fixture per family (from the v4 table), near-miss negatives, the blinded-context assertion (render output contains ZERO bytes of any planted secret — the G4 discipline at the display layer), the refuse-on-empty walk arm, the build-stamp refusal arm (cfg'd per function so no feature set turns the target into a green `0 passed`).
- [x] T7 — `--mine` intro copy + README/AGENTS.md note: the egress gate and the sec warn now share ONE pattern set (v4); the brag line for refine.gist.rs (riir-dapps half, Issue 151's marketing task).
- [x] T8 — docs: `.docs/11_domains/sec.md` (or the domain doc page) gains the secret-blind section; AGENTS.md `sec` bullet updated (16 rules → the scan surface named; the count stays the CORPUS count, the scanner is not a corpus rule).
- [x] T9 — (verdict R1) RETIRE the second table: `structural_admission::scan_secret_shapes` consumes `kat_redact::scan_secrets` — filter to the prefix-rule kinds ∪ `PrivateKeyBlock` (NOT entropy/env/bearer/conn/rust-literal/JWT/telegram/discord — reef is the persistence boundary, and entropy rules would block every legit payload carrying hashes), apply reef's word-boundary check on the finding offset, map legacy shape names (openai_key/github_pat_classic/github_pat_finegrained/aws_access_key_id/slack_token/google_api_key/pem_private_key — the `github_pat_` split resolves by peeking the text at the finding offset) and fall back to `kind.tag()` for new kinds. DELETE `SECRET_PREFIX_TABLE`. `github_pat_` min body 22 keeps reef's fine-grained fixture (26 chars) firing. Tests: the existing reef tests pass unmodified; one new test pins that reef and the raw scanner find the same secrets on the shared fixtures (modulo the boundary + kind filter).
- [x] T10 — (verdict R1) EGRESS CENSUS before any brag: inventory EVERY path code/span text can leave the machine (mining sync `push_mining_batch` → `load_mining_rows` → `RedactionGate`; `fixer_regate`; the CF Workers AI / groq / kat_drain LLM lanes — verify each caller gates its prompt; the lease/wetrun transports; any telemetry) and pin the chokepoints with a test or a named refusal. The refine.gist.rs claim is worded to EXACTLY what the census proves, with the honest limits stated: unknown formats are caught only by the entropy rule; PII is out of scope (Bench 052); a redacted payload is terminal.

## Marketing (the trustable-answers leak concern — refine.gist.rs)

The mining pipeline ("trustable answers" — community-contributed
fix-pairs) is exactly where a pasted secret would leave the machine. The
brag, once v4 + T1–T6 land: the healer's egress gate scans every snippet
against the consolidated provider-format table before ANY byte leaves the
machine, the sec warn shows the developer the same findings locally, and
`--secret-scan --blind` hands them safe-to-share text. Copy lands in the
riir-dapps refine.gist.rs marketing module (privacy/trust card), numbers
rendered from data per the site law (the pattern-set count is a countable
fact, quoted as the version's changelog states it).

⛔ (verdict R1) HOLD until T10's census exists — "every snippet passes the
gate before any byte leaves the machine" is only true if every egress path
is pinned. Word the card to the census: which lanes are gated, and the
three honest limits (entropy-only for unknown formats, PII out, terminal
redacted payloads). An overclaimed trust card is the overclaim this whole
feature exists to avoid.

## Non-goals

- PII regexes: Bench 052's measured posture stands (the span itself is the
  asset; patterns cannot close it). The brag says exactly that — no
  overclaim.
- Autofixing secrets: never. A "fixed" secret literal is a broken program
  and a false sense of security.
- Re-scanning redacted payloads (the gate's terminal-payload rule) — the
  placeholder site can re-flag; `blind()` output carries the same note.


## Landing record (2026-10-06)

- **T10 census (the trust claim's basis):** every span-text egress passes
  the gate — `push_mining_batch`/`load_mining_rows` (heal.rs:1712,
  RedactionGate::discover + per-row check), `fixer_regate` (`gate_samples`
  before any arm, :468), `kat_drain` (same gate; all-blocked = exit 4, the
  module doc + kat_redact import at :91). Transports carrying NO code text:
  burn/lease/fixstat/replay (counts + rule ids + signed burn reports).
  `groq.rs`/`cf_workers_ai.rs` are transports; their span-carrying callers
  are exactly the gated three. The refine.gist.rs card is worded to THIS
  census + the three honest limits (entropy-only unknown formats, PII out
  per Bench 052, terminal redacted payloads).
- **T13 measurement** (filled into riir-kat's REDACT_CHANGELOG v4 entry):
  redact v4 over both repos' tracked src/ trees = 340 findings, **+1 over
  v3** (the reef fixture's AIza sample — the fragmentation closing in
  mirror); the putty doc self-reference was reworded to never fire its own
  scanner. tests/ delta = the planted v4 fixtures themselves (by design).
  Mining-sync impact: +1 span would drop, and it is self-referential
  documentation.
- **Golden re-pin:** `kat_sync_payload`'s `golden_payload_bytes_are_pinned`
  re-pinned to the v4 header bytes + signature (the golden's own comment
  prescribes the coordinated re-pin on a redact_version bump).
- **Commits:** riir-kat `eed88c6`, riir-refine `deca02fc`.
