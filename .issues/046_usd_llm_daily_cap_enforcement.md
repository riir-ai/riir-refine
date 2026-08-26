# Issue 046 — USD LLM daily-cap enforcement (`DEFAULT_LLM_DAILY_CAP_USD = 1.0`)

Pins [Proposal 005](../.proposals/005_kat_heal_service_network.md) §The daily trainer window → Window gates (added 2026-08-26) into code. **Today nothing enforces a USD bound on LLM spend**: the call surface (`src/cf_workers_ai.rs` `CfClient`) is bounded only by Bench 049's harness-level neuron accounting (manual, not standing, not USD). Every future autonomous loop — the trainer-window drain (T2.0), the sweeper miss-class (T2.6), the Phase 3 fixer — calls through this surface. The cap must exist and be green BEFORE any of them runs unattended. This is the incident-class killer: the Issue 040 169-request burn and the qwen null-at-`finish_reason:"length"` retry loop are both "keep calling past what you meant to spend".

Why an issue, not a plan: single crate, ~100 LOC + fixtures, no phases. Dependency OF [Plan 031](../.plans/031_kat_sweeper_temp_container_r2_tier.md) Phase A.

## Design (the reserve-then-reconcile ledger)

- **Reserve before the call**: worst-case estimate = `in_tokens × in_rate + max_tokens × out_rate` (thinking tokens bill on the out side — never visible-only sizing). Reserve against the day's accumulator; if `spent + reserve > cap` → **hard-decline for the rest of the UTC day** (`Err::CapReached`), never a retry loop.
- **Reconcile after the call**: actual usage from the response's usage field (both response schemas are already parsed per Proposal 005 gate (e)); release the reserve, record the actual.
- **Pricing table**: per-model-id in/out per-M constants (`@cf/qwen/qwen3.8-27b` pinned from published rates; version-stamped — the `CHARGE_VERSION` discipline). **Unknown model id → refuse the call** (never price a guess as $0).
- **Persistence**: `<root>/.heal/kat_llm_spend.json` — per-UTC-day accumulator, version-stamped (the `kat_meter.json` pattern). Missing/corrupt → recover to zero + warn (never inflate); the fail-safe direction is over-declining early in a day, never over-spend.
- **Free tier note**: neurons bind first on the free allocation (10K n/day ≈ $0.11 sticker) — the cap is the paid-mode / re-designation backstop. It still runs on the free tier (defense in depth: a runaway loop burns the day's neurons AND trips the cap).

## Tasks

- [ ] T1 Constants + env: `DEFAULT_LLM_DAILY_CAP_USD: f64 = 1.0`; `KAT_LLM_DAILY_CAP_USD` override (parse-once; invalid → default + warn, never abort the run); `0` = decline-all (the safe test mode)
- [ ] T2 The spend ledger (load/reserve/reconcile/release/persist; UTC-day rollover; version-stamped; recovery-to-zero)
- [ ] T3 The pricing table (model-id → in/out per-M; unknown → refuse; version-stamped)
- [ ] T4 The gate in `CfClient::generate`: pre-call reserve check → post-call reconcile; `Err::CapReached` is a terminal-for-today verdict the CALLER must honor (no built-in retry)
- [ ] T5 Fixtures: cap-reached declines + day-rollover resets; env raise/lower/zero; reserve-vs-actual reconcile (partial release); unknown-model refusal; corrupt-ledger recovery; determinism
- [ ] T6 Wire `src/bin/fixer_regate.rs` (the Bench 049 harness) through the gate — the standing consumer until the drain loop exists; its neuron cap stays (two independent bounds, whichever first)

## Gates

- G1: a planted $0.99-spent state declines the next worst-case-over-reserve call; a fresh UTC day passes it
- G2: no new deps; the check path is pre-call cold — allocation-discipline fixture
- G3: default features unchanged (rides the same feature gate `cf_workers_ai.rs` already compiles under)

## Ref

Proposal 005 §Window gates + §The own-repo hourly sweeper lane · [Bench 049](../.benchmarks/049_fixer_regate_poc.md) (the incident class) · Issue 040 (resolved; record in git history) · Plan 031 (consumer).
