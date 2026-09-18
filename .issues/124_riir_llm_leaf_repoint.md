# Issue 124 — re-point the ops-plane ladder at the riir-llm leaf

**Status:** LANDED — leaf `3eb31e5` + `6240360`; clippy re-point committed on develop (groq/cf/both/default clippy-clean, 2229 + 1919 lib tests green, standalone gate 12 deps, live groq smoke PASS).

## Problem

riir-llm exists (the Issue-051/106 extraction leaf, first consumer riir-dao)
but riir-clippy still carries its own copies of everything the leaf now
owns: the OpenAI-compatible body builder (with the qwen `/no_think`
mitigation), the CF body builder (with the GLM switches), both wire URL
shapes, both response parsers, the overflow classifier prefix (defined
twice — groq.rs AND cf_workers_ai.rs — with a drift-pin test holding them
equal), the failure classification, and the timeouts. Two copies of a wire
protocol is the drift class the extraction exists to kill — the dao smoke
already caught one URL-shaped bug in the leaf's copy while clippy's copy
carried its own spelling.

## The re-point (one seam decision, stated)

The leaf owns **wire mechanics**: URL shapes, body builders (wire-aware
model placement), response parsers, failure classification, timeouts,
bearer hygiene. riir-clippy keeps **policy**: the windowed-disable router
(`LlmRouter`), FreeTierStats + the `providers.*` ledger/dashboard contract,
`avoided_usd` pricing, the USD cap + reserve-then-reconcile spend ledger,
the GraphQL usage meter, the regate harness. The leaf's ladder is
stateless per attempt; clippy's router disables a rate-limited leg for the
whole WINDOW — that policy asymmetry is deliberate, not duplication.

## Changes

- `groq.rs`: `OaiClient` slims to `{ leg: riir_llm::Leg, agent }` —
  `generate` = leaf `chat_body_for` + `post_chat_on` + `Leg::parse`.
  `OaiError` becomes an alias of `riir_llm::LlmFailure` (the Auth variant
  joins). `OaiGen` slims to what the router consumes (drops `total_tokens`
  + the quota-header fields — dead diagnostics only the live-smoke print
  read). `build_oai_body` / `parse_oai_result` / `parse_quota_header` /
  `ParseFail` / the private scrub machinery: deleted (leaf-owned).
  EndpointConf carries base URLs; the leaf appends the route.
- `cf_workers_ai.rs`: `CfClient::generate_billed` = leaf
  `chat_body_for(Cf, …)` (model-free body, byte-compatible with the
  measured pre-124 shape) + `post_chat_on` + `parse_cf_result`. The
  `billed?` distinction survives: transport-class failures are not billed
  (no result), parse failures after a 200 are billed (the qwen-incident
  rule). `build_request_body` / `parse_chat_result`: deleted (moved to the
  leaf as `chat_body_for` + `parse_cf_result`, tests moved with them).
  `url` field → `base` (leaf owns the route shape). Both `CALL_TIMEOUT` /
  `CONNECT_TIMEOUT` consts alias the leaf's. `REASONING_OVERFLOW` aliases
  the leaf's const (the drift-pin test retires — the const has one home).
- Router: `LlmFailure::Auth` is STOP-class (surfaced verbatim, counted as
  a new `auth_errors` stat) — a 401 from a free leg no longer silently
  shifts the window onto the paid CF leg. Behavior change, deliberate:
  the leaf rule ("a present-but-rejected credential is a config bug")
  now holds on every consumer.
- `Cargo.toml`: `riir-llm` optional path dep (`../riir-llm`,
  default-features = false); `groq_first = ["riir-llm/transport"]` (drops
  its `dep:ureq` — the free legs no longer touch ureq directly);
  `cf_workers_ai` adds `riir-llm/transport` (it already pulled ureq, so
  the platform posture is unchanged: native-only, as before).
- Live-smoke tests keep their names/invocations; quota-header prints go
  (the fields are gone).

## Validation

- Leaf: clippy clean at default + transport; 26 tests green (12 before).
- dao (the other consumer): full suite green against the evolved leaf.
- clippy: clippy + tests at groq_first, cf_workers_ai, both, and default.
- One-call CF live smoke if CF_API_TOKEN is present (the body gains no
  new fields vs the measured shape — the "model" field stays absent on
  the CF wire — so this is a belt-and-braces check, not a premise).

## Residuals

- `fetch_usage_today` (GraphQL) keeps CfClient's own ureq agent + auth —
  the leaf transport speaks chat-completions, not GraphQL. Not drift.
- dao's CF leg keeps its own `request_body` (transcript-pinned byte
  shape, dao-side decision) — noted in riir-llm's BOUNDARY, not drift.
