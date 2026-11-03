# Fixer serving lane — leak-posture sign-off memo (for owner signature)

> **PIVOT 2026-11-03 (owner directive — [Plan 335](../../.plans/335_local_cluster_fixer_lane_qwen_local_gate_and_cli_runway.md)):** the fixer serving posture moves to the **LOCAL cluster** (amo/bmo via riir-infer) — consumer code never crosses to a third party. This memo's vendor-vetting half (CF no-training, Groq DPA, provider kill switches) **re-scopes to the TRAINER WINDOW lane** (own-repo redacted data; still groq-first/CF-fallback). The fixer lane's gate is now **Plan 335 P0** (the qwen-local quality gate) + the first-party posture: the consent gate, the scanner-before-egress, and OUR no-retention commitment survive verbatim (items 2–6 re-anchored: the recipient is the operator's cluster, not a vendor). **Do not sign this memo as-is for the fixer lane** — the re-anchored signature doc lands with Plan 335 P1.1.

**Status:** DRAFTED 2026-11-03, READY FOR SIGNATURE (Plan 333 T3 · Claude verdict round 1 AGREE · OwnerGate target: signature ≤ 2026-11-14, ahead of the 11-17 prose window). Sign by replacing the last line; changing any pinned text below reopens this memo.

**Decision requested:** enable the consumer-facing fixer serving lane (Proposal 005 caveat #2's remaining half) under the posture below. Signing flips Phase B of [Plan 333](../../.plans/333_p005_fixer_leak_signoff_and_all_tier_prep.md) (ENABLE flip + deployer manifest rows + copy lifts). Nothing else changes.

**1 — What the lane is.** The modelless pre-pass stays first and unchanged (the default; decline remains a correct answer). Only reachable misses route onward: span + the modelless candidate diff + retrieved corpus few-shots → **CF Workers AI `llama-3.3-70b`** (the Bench-049-measured winner: verify-filtered strict-keep 5/9 A / 6/9 B vs decline 0/9) → `fix_verify` compile gate + clippy oracle strict-keep → keep or decline. The never-wrong contract is unchanged.

**2 — The consent gate.** Explicit per-run consent: `RIIR_REFINE_CODE_EGRESS=snippet` — unset/junk refuses BEFORE any socket (shipped, `src/code_egress.rs`); snippet-only payload, never whole files; the interactive prompt asks once per repo and persists the choice. The prompt text is pinned word for word (item 5) — a change to it reopens this memo.

**3 — The numbered exclusions (each is a pre-T6 launch check).**

- a. **NO payload logging**: no `console.log` of snippets; no Logpush; no Workers Logs / Analytics Engine of request or response bodies. (CF's zero-retention exceptions name application logging — our own logs would reopen the retention row.)
- b. **NO stateful features in the fixer path**: no AI-Gateway caching; no R2/KV/DO/Vectorize of payload; no batch/async modes. Standard stateless inference only — that is the class the vendor commitment covers.
- c. **NO provider other than CF at first enable.** Groq is gated on T1b (Services Agreement + DPA verification — OwnerGate decision, verdict round 1: CF-only launch approved, Groq deferred).
- d. **NO egress without the scanner**: `kat_redact` (REDACT_VERSION 5, fail-closed) runs BEFORE the socket; any finding drops the span.
- e. **The USD circuit breaker is unchanged**: `DEFAULT_LLM_DAILY_CAP_USD = 1.0`, `llm_spend.rs` reserve-then-reconcile, hard-decline at cap.

**4 — Vendor posture (verified, quotes in `fixer_leak_posture_vendors.md`).** CF Workers AI "Data usage" page (updated 2026-10-16): "**No training:** Cloudflare does not use Customer Content processed by Workers AI to train AI models." + zero data retention for standard non-stateful inference; operational metadata is not Customer Content. Model license — Llama 3.3 Community License (fetched + quoted 2026-11-03): commercial use granted; the 700M-MAU additional-commercial-terms clause is far above us; the attribution duty (§1.b.i) triggers on *distributing* the Materials or a product/service that *contains* them — a service calling a hosted provider distributes nothing and contains nothing, and the model-naming clause triggers on using outputs to *train a distributed model* (not our case). "Built with Llama" in the product docs is RECOMMENDED as cheap insurance, not required by the strict text. The AUP (incorporated by reference; fetched 2026-11-03) prohibits illegal/harmful classes — a developer tool drafting code fixes is the intended commercial use; its item 4 end-user disclosure duty is satisfied by item 5 below. The EU restriction applies to multimodal models only; Llama 3.3 70B is text-only.

**5 — The consumer-facing disclosure (T8 copy; the consent prompt, pinned).**

> This fix sends the code snippet (≤ N lines, the span plus minimal context) to our LLM provider (Cloudflare Workers AI) over an encrypted connection. Their terms commit that it is not used to train their models and is not retained after the response. A local secret scanner runs first; anything with a finding never leaves your machine. Everything else — the scan, the draft, the verify — stays local. \[Ask once per this repo / Always / Never\]

**6 — Kill switches.** `RIIR_REFINE_CODE_EGRESS` unset = refuse (always, shipped); the per-provider kill (LlmRouter provider state); the USD cap; the standing demote literal restores byte-identical incumbent behavior.

**7 — What signing flips (Phase B, Plan 333).** The ENABLE flip (`l4_fallback.md` §5 item 2's lane — CF only at first); the deployer fixer-node manifest rows + the Trainer-window `[-]` un-gate (the same sign-off covers both — Proposal 005: "plus a leak-posture sign-off either way"); the copy lifts (Proposal 005 caveat #2, `node_tiers.md` Pro row, Plan 107's marketing non-goal, dist README + the disclosure notice).

**8 — What does NOT change.** The modelless default; never-wrong; redaction-before-sync for mining; the mining novelty/quorum machinery; the trainer window's own designation + gates (its provider history is the trainer lane's record — Issue 051 lineage — and stays out of this memo's scope).

**Exposure note (honesty item, Plan 333 T1c):** own-repo redacted drain data has ALREADY traversed Groq's free tier (the Issue 051 groq-first window; ledger row `drain-18d01cf6ab6e21c8-99237`, `served_by: "groq"`, 2 calls). That is the donated/mined data class with the sync ToS license — a lower bar than consumer code, which has never left a machine (no fixer lane exists). T1b closes the Groq question for both.

Sign: ______________  Date: ________
