# Fixer leak posture — vendor verification record

**Status:** LIVE 2026-11-03 — CF Workers AI VERIFIED in writing (fetched + quoted this session); Groq OPEN (public pages explicitly route Customer Data to the Services Agreement + DPA) with a RECORDED EXPOSURE for the own-repo drain class (see the Groq section); model licenses: Llama 3.3 VERIFIED (fetched + quoted), qwen3.8-27b PARTIAL (ZDR confirmed, license name open). **Scope note (2026-11-03, owner pivot): this record's FIXER-lane relevance ENDS here — the fixer moved to the local cluster (Plan 335); the vendor record stays load-bearing for the TRAINER WINDOW lane only.** Consumer of: Proposal 005 caveat #2's leak half · cited by [Plan 333](../../.plans/333_p005_fixer_leak_signoff_and_all_tier_prep.md) · the memo: [fixer_leak_posture_signoff_memo.md](fixer_leak_posture_signoff_memo.md) (pivoted — do not sign for the fixer lane).

Rule for this doc: a vendor row is VERIFIED only when the commitment is quoted verbatim with URL + the page's own "last updated" date, and the exceptions are named. Re-verify on any provider-terms change before the sign-off memo cites it.

## VERIFIED — Cloudflare Workers AI

Source: <https://developers.cloudflare.com/workers-ai/platform/privacy/> ("Data usage", **last updated Oct 16, 2026**; fetched 2026-11-03).

Verbatim commitments:

- "**No training:** Cloudflare does not use Customer Content processed by Workers AI to train AI models."
- "**Zero data retention:** Workers AI processes Customer Content only for as long as needed to provide the service. For standard inference requests that do not invoke a stateful feature, Cloudflare does not retain inputs or outputs after they are no longer needed to process the request and deliver the response, subject to the limited exceptions below."
- Customer Content = inputs + outputs + embeddings + any training/adaptation data we provide (per the Agreements).
- "Cloudflare does not make it available to other Cloudflare customers."

Named exceptions (zero-retention does not cover): Customer Content stored via Cloudflare storage services (R2/KV/Durable Objects/Vectorize) used with Workers AI; **logging or caching enabled in AI Gateway or application logs**; asynchronous/batch features, file/adapter uploads, or other stateful features. **Our posture** (direct REST inference from the Worker, non-stateful, no AI-Gateway caching, and — per Plan 333 T3's numbered exclusion — NO request/response-body logging anywhere in the fixer path: no `console.log` of snippets, no Logpush/Workers Logs of payloads) sits in the retained-free class — keep it that way; any stateful feature or payload logging in the fixer path reopens this row, and the pre-T6 launch check verifies it.

Operational metadata (account/model ids, timestamps, sizes, token counts, latency, status codes) is retained but is NOT Customer Content.

**Model-surface note (2026-11-03, from the qwen3.8-27b model page):** CF model pages expose a **`store` parameter** ("whether to store the output for model distillation / evals") — the fixer/trainer path must leave it unset/false; a set `store` reopens the zero-retention row. qwen3.8-27b's page also confirms model-level **Zero data retention: Yes** + the pricing P005 cites ($0.45/$3.20 per M). (Posture note: qwen3.8-27b is the trainer-window designation; the FIXER lane moved local-cluster 2026-11-03 — Plan 335.)

Legal/safety exceptions: analysis or retention to fulfill legal obligations is still "not used to train AI models or improve Cloudflare or third-party products or services."

Third-party models: "Cloudflare does not create or train the third-party AI models made available through Workers AI. Those models may be subject to open source, acceptable use policies, or other license terms between you and the model provider. Review the license terms applicable to each model." → this is the model-license half (Plan 333 T2), separate from the platform commitment above.

## OPEN — Groq (the Issue 051 free leg)

Sources (fetched 2026-11-03): Privacy Policy <https://groq.com/privacy-policy/> (effective November 12, 2025) · Terms of Use <https://groq.com/terms-of-use/> (effective October 15, 2025).

What the public pages say:

- Privacy Policy: "This Policy does **not** apply to the information that we process as a 'data processor' on behalf of customers ('**Customer Data**') of our business offerings such as GroqCloud, GroqChat, and our Application Programming Interfaces (collectively, 'Cloud Services'). Our processing of Customer Data in connection with a customer's use of our Cloud Services is governed by our **Groq Services Agreement and Data Processing Addendum**."
- Terms of Use: "These Terms do not apply to you in connection with your use of Groq's cloud services... If you are interacting with Groq as a customer of our services, the **Groq Services Agreement governs**."
- The Service-Specific Terms (effective November 13, 2025) tie Inputs/Outputs processing for designated beta models to the DPA.

What that means: **no public no-training / retention commitment for API Customer Data exists at the pages fetched.** The commitment (if any) lives in the Groq Services Agreement + DPA — account-level documents. Free-tier vs paid-tier handling is unknown from public pages.

Open items (Plan 333 T1b):

1. Read the Groq Services Agreement + DPA from the owner's console account; quote the data-use/training/retention clauses here in the same format as the CF row.
2. If the terms do not carry a no-training commitment for the tier the `LlmRouter` free leg uses (Issue 051), either (a) get it in writing from Groq, or (b) route that leg's class of data elsewhere. This gates: any Groq arm in the fixer lane AND closes the trainer-window drift note (the window's free leg is groq-first over own-repo redacted data — lower bar than consumer code, but our own corpus spans are business IP).
3. Re-check on terms changes; date every re-verification.

**Recorded exposure (Plan 333 T1c, 2026-11-03):** the exposure has ALREADY HAPPENED for the own-repo drain class — a live window row shows `served_by: "groq"` absorbing 2/2 drain calls (`.benchmarks/llm_cost_ledger.jsonl`, run `drain-18d01cf6ab6e21c8-99237`, ts 1787960899). Data class: the unproven queue = own-repo redacted, sync-ToS-licensed trajectories (donated) — NOT consumer code; no consumer live code has ever left a machine (no fixer lane exists). T1b is the closure for both classes.

## Model license rows (Plan 333 T2 — filled 2026-11-03)

| model | hosted by | license | serving-user-code terms | status |
|---|---|---|---|---|
| llama-3.3-70b | CF (Bench-049 winner arm) / Groq (window free leg) | Meta **Llama 3.3 Community License** (release date December 6, 2024; text fetched 2026-11-03 from meta-llama/llama-models) + AUP | **VERIFIED** — commercial use granted; 700M-MAU clause far above us; attribution §1.b.i triggers on *distributing* the Materials or a product/service that *contains* them — calling a hosted provider distributes/contains nothing; the model-naming clause triggers on using outputs to TRAIN a distributed model (not our case). "Built with Llama" in product docs RECOMMENDED as cheap insurance, not required by the strict text. AUP (fetched): prohibits illegal/harmful classes; automated code-fix tooling is the intended commercial use; AUP item 4's end-user disclosure duty is satisfied by the memo's consumer notice. EU restriction = multimodal only; 3.3-70B is text-only (N/A). | CLOSED |
| qwen3.8-27b | CF (trainer-window designation, carries the §window gates) | expected Apache-2.0 family — **license NAME not stated on the CF model page** (quote it from the model card before any qwen arm in a paid lane) | **PARTIAL** — CF model page confirms model-level "Zero data retention: Yes" + the $0.45/$3.20-per-M pricing P005 cites; the `store` (distillation/eval storage) param must stay unset/false in every path; NOT load-bearing for first enablement (the fixer lane moved local-cluster 2026-11-03 — Plan 335; the window stays groq-first/CF-fallback) | PARTIAL (ZDR ✓, license name open) |
