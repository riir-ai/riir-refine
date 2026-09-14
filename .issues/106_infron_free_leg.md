# Issue 106 — the infron free leg: groq → infron → CF ladder

Status: DONE (2026-09-14)

## Context

The owner provisioned a second FREE OpenAI-compatible endpoint
(`llm.onerouter.pro`, key `INFRON_API_KEY` in `.env`): same
chat-completions shape as groq, same qwen3.8-27b model family, measured
~33 tok/s (free but slow). Owner direction: slot it BETWEEN groq and the
metered Cloudflare fallback, so a rate-limited groq degrades to a slow
free leg before it ever burns paid neurons.

This is exactly the extension Proposal 005 anticipated ("the provider
ladder is the template for more free tiers — provider list + per-provider
quota state, not a hardcoded pair").

## Design (as landed)

- `GroqClient` generalized to `OaiClient` over a static `EndpointConf`
  (label / url / key_env / model_env / default_model); two consts
  (`GROQ_CONF`, `INFRON_CONF` — `qwen/qwen3.8-27b:free` default,
  `INFRON_MODEL` env override). Zero duplicated client code.
- Ladder per attempt: groq (fast free) → infron (slow free) → CF (paid,
  cap-guarded). Overflow on ANY free leg stays surfaced (never fallen
  back — a budget problem is deterministic). 429 on a free leg disables
  THAT leg for the window. `fallbacks_to_cf` counts actual CF reaches
  only (a groq→infron hop is measured by infron's own `calls`).
- Accounting: both free legs price `usd_avoided` at CF rates; the ledger
  row grows a `providers.infron` block beside `providers.groq`;
  `served_by` grows `"infron"` / `"groq+infron"` arms.
- Trust posture: llm.onerouter.pro is a third-party aggregator — egress
  stays behind the redaction gate + own-repo-only policy (same class as
  the groq leg; no new egress class).

## Tasks

- [x] T1 `src/groq.rs`: generalize client (`OaiClient` + `EndpointConf`),
      add `INFRON_CONF`, per-leg redaction placeholder + error labels
- [x] T2 router: `Provider::Infron`, 3-leg `LlmRouter` (`attempt_free`
      helper, per-leg disable state + stats), `providers_json` carries
      both free blocks
- [x] T3 `kat_drain`: preflight/arming lines, `served_by` infron arms,
      summary + ledger accounting for both free legs
- [x] T4 `.env.example` INFRON section + `node_tiers.md` ladder mentions
- [x] T5 tests: arm-plan 3-leg matrix, providers-block contract, qwen
      `:free` /no_think pin, live infron smoke (manual)
- [x] T6 validation: clippy `-D warnings` + full lib suite + live
      preflight with the real key

## Measured findings (live probes, 2026-09-14)

- **Wire contract VALID**: the endpoint answers with structured OpenAI
  error bodies; auth + `/error/message` parsing + 429 classification all
  work through `OaiClient`.
- **The free pool is account-gated RIGHT NOW**: every `:free` model
  (qwen/qwen3.8-27b:free, deepseek-v4-flash:free ×2,
  nemotron-3.5-lightning:free) answers `HTTP 429: "Free model requires
  Team balance greater than $4.999999"` — with AND without the owner's
  `provider.service_tier` hint (A/B'd by curl). The owner's 33 tok/s
  measurement predates this account state. **Activation = top up the
  onerouter team balance above $5.**
- **Degradation is CORRECT meanwhile**: the first 429 disables infron
  for the window (one wasted call, then out of the way) and the ladder
  proceeds exactly as the pre-106 path — measured live via
  `kat_drain --preflight`: groq ARMED + infron ARMED + CF unarmed →
  probe served via groq (21+2 tok, 0.0 neurons, 391 ms).
