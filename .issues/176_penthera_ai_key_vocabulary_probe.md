# Issue 176 — penthera AI-key vocabulary probe for the secret-scan lane (the Research-329 defer, converted)

**Status:** OPEN — probe DONE, yield = 3 clean misses, ready for its own unit (the
"Batch-33 discipline": scanner-table changes ride their own batch, never a corpus
batch sidecar). Filed from the penthera DEFER named in
[Research 329](../.research/329_penthera_sec_distill_corrected.md)
("Two DEFERS named, not filed"); this filing converts it.
**Source:** danoszz/penthera @ `07421f95ced357e0d7e184353e65821748478235`
(the Research-329 pin; local `.raw/penthera` HEAD == pin, verified at probe time).
**Probe date:** 2026-10-26.

## What was probed

`lib/rules/core/secrets.json` — penthera's secret-detection table: `kind:
"secret"`, **12 rules**, each `{id, name, pattern, references, severity,
confidence, date, source}`. All 12 are AI-provider key shapes (anthropic,
openrouter, openai ×2, google, groq, huggingface, replicate, xai, perplexity,
langsmith, pinecone).

## Dedup verdict vs the ONE canonical table (`riir-kat/src/kat_redact.rs` PREFIX_RULES, 86 FindingKind rows)

| penthera rule | shape | ours | verdict |
|---|---|---|---|
| `secret.anthropic-api-key` | `sk-ant-(?:api\|admin)\d{2}-…32,` | `sk-` row (OpenAiStyleKey, is_gitlab_body, 20) | COVERED — the `-` is in the gitlab body charset; class label is OpenAiStyleKey (the REDACT_CHANGELOG 2026-10-06 entry records exactly this parity proof) |
| `secret.openai-api-key` | `sk-(?:proj\|svcacct\|admin)-…20,` | `sk-` row | COVERED (the row's own intent) |
| `secret.openai-api-key-legacy-format` | `sk-[a-zA-Z0-9]{20,}` | `sk-` row | COVERED |
| `secret.openrouter-api-key` | `sk-or-v1-[a-f0-9]{64}` | `sk-` row | COVERED (mislabel-only, same class shape) |
| `secret.google-api-key-gemini-firebase-maps` | `AIza…35` | `AIza` GoogleApiKey, is_word, 35 | COVERED exact |
| `secret.groq-api-key` | `gsk_…48,` | `gsk_` GroqKey, is_alnum, 40 | COVERED — our min_body 40 < 48, MORE sensitive |
| `secret.hugging-face-token` | `hf_…34,` | `hf_` HuggingFaceToken, is_alnum, 34 | COVERED exact |
| `secret.replicate-api-token` | `r8_…37,` | — | **MISS — no `r8_` row** |
| `secret.xai-api-key` | `xai-…70,` | `xai-` XaiKey, is_alnum, 30 | COVERED — more sensitive |
| `secret.perplexity-api-key` | `pplx-…40,` | `pplx-` PerplexityKey, is_alnum, 48 | COVERED at shape; **sensitivity NOTE**: our min_body 48 > their 40 — a 40–47-char pplx body would be missed by ours (gitleaks pins 48; penthera's 40 is the loose outlier — keep 48, record the disagreement) |
| `secret.langsmith-api-key` | `lsv2_(?:pt\|sk)_[a-f0-9]{32}_[a-f0-9]{10}` | — | **MISS — no `lsv2_` row** |
| `secret.pinecone-api-key` | `pcsk_…50,` | — | **MISS — no `pcsk_` row** |

Verified at read time: `grep "r8_\|lsv2\|pcsk"` over `kat_redact.rs` → zero hits
(the one `sk-ant` mention is the changelog's parity-proof prose, not a rule row).

## The unit (what lands when this issue is taken)

Add 3 prefix rows + 3 `FindingKind` variants + 3 kind-labels to
`riir-kat/src/kat_redact.rs` (the same shape as the 2026-10-06 AI intel pass):

- `("r8_", FindingKind::ReplicateToken, is_alnum, 37)` — replicate r8_ tokens
- `("lsv2_pt_", FindingKind::LangsmithKey, is_b64ish_or_hex, 48)` + the
  `lsv2_sk_` twin (penthera pins the body split `32 hex _ 10 hex` = 43; our
  prefix split keeps the charset check simple — decide hex-vs-alnum at unit
  time from a live sample, never from penthera alone)
- `("pcsk_", FindingKind::PineconeKey, is_alnum, 50)` — pinecone pcsk_ keys

Laws that bind the unit: `REDACT_VERSION` bump + changelog row (the v4
two-needle precedent); the self-scan delta printed beside the landing (the
2026-10-06 pass printed 339 — re-run the same measure); no keyword gating
(the scanner stays keyword-free by design); penthera is MIT but the rows are
key-format FACTS — cite the pin, write our own row bodies (the fixtures law).
`pplx-` stays at 48 with the penthera-40 disagreement recorded in the row
comment. Consumers are the ONE canonical table — refine re-exports already
flow (`riir_refine::kat_redact::scan_secrets`).

## Not in scope (recorded so nobody re-opens them blind)

- penthera's severity/confidence/date metadata — their scoring model, ours is
  Finding-kind-only by design.
- The 12 rules' `references` (their doc links) — kept in the source repo.
- The second Research-329 defer (agent-tooling audit for the katgpt-rs
  security-audit skill) — owner-directed, NOT this issue.

## Acceptance

- [ ] 3 rows land with FindingKind variants + labels; `cargo test -p riir-kat --lib` green; refine consumers re-compile (the re-export plane untouched).
- [ ] Self-scan delta measured and printed in the landing row (vs the 339 datum).
- [ ] REDACT_VERSION bumped + REDACT_CHANGELOG row cites this issue + the penthera pin.
