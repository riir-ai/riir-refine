# Plan 324 — Mozilla web alignment: the four fronts answer the six criteria

**Date:** 2026-10-31
**Status:** Active — Phase 1 pending; owner pivot v4 (issue 192) is the copy source
**Research:** [katgpt-rs Research 642](../../katgpt-rs/.research/642_Mozilla_Human_Scale_AI_Grant.md) (public program digest)
**Target:** `reflex-site` (reflex.riir.ai), `riir-refine` site lane (refine.riir.ai), `riir-rethink` site lane (rethink.riir.ai), `riir-dapps`/root (riir.ai) — per the web-family skill
**Deadline driver:** consult email ~Nov 7; application Nov 17–19 (grant lane: issue 190)

---

## Goal

Make the four public fronts state, consistently, exactly what the Mozilla application claims — Human-Scale AI's three principles answered on-page — and strip everything that contradicts the charitable posture (pricing, token/KAT marketing, storefront copy). After this plan, a reviewer who reads only the websites can answer all six evaluation criteria without the PDF.

Owner pivot v4 (issue 192) supplies the narrative:
- **Refine** = real-life use in the coding domain — open source, closed beta for the contribution network, corpus stacking, other coding domains (Python/Go) planned. **Public good.**
- **Rethink** = ongoing PoC — generic decision lanes, beating every decision lane we can — **no pricing anywhere**; commercial later only after trust (post-grant decision, not site copy).
- **The credit plane is gone from the story**: contribution points (TUNA, non-transferable, expiring) replace KAT/payment; no more mainnet for refine.
- **Moat line for the sites**: the family is open; the game business (seal-*, separate brand) is out of scope and unmentioned.

## Phase 1 — Strip the contradictions (BLOCKING, before the consult email)

### Tasks

- [ ] **T1.1** `refine.riir.ai`: remove every pricing/KAT/pack/tier mention; replace the value props with the open-source + closed-beta + points story (T1.4 copy); state "no payment, no token — contribution points are non-transferable reputation".
- [ ] **T1.2** `rethink.riir.ai`: remove pricing/storefront mentions; reframe as "ongoing PoC — generic decision lanes, benchmarked openly"; link the public bench tables as the honest-metrics story.
- [ ] **T1.3** `riir.ai` + `reflex.riir.ai`: family-bar copy pass — align descriptions with v4 positioning (reflex = the modelless engine + arenas; refine = the healer; rethink = the research lane); remove any KAT/token copy.
- [ ] **T1.4** Write the canonical copy block ONCE (in this repo's grant playbook §13) and mirror it everywhere: the three-principles answers (Small = zero-parameter modelless + local; Intentional = consent-first + community points + open governance; Tailored = Rust devs without cloud-AI budgets + closed beta), the six-criteria one-liners, the corpus/points/governance story.
- [ ] **T1.5** Cross-site conformance gate after chrome/copy changes (web-family skill) on all touched fronts.

## Phase 2 — Answer-the-criteria pages (application-supporting)

### Tasks

- [ ] **T2.1** `refine.riir.ai` gains a "How it sustains" section: open core (MIT), community corpus (MIT forever), contribution points (TUNA — earn by contributing, spend on hosted lanes, expiring, non-transferable), future coding domains (Python/Go), enterprise services — the sustainability answer in public.
- [ ] **T2.2** `rethink.riir.ai` gains a "Research status" block: the lanes on the board, what's beaten, what's open — the "PoC until every decision lane is beaten" line, verbatim from issue 192.
- [ ] **T2.3** Evidence surfaces on-page: installer counts, pilot call link, corpus stats, leaderboard (once the points plane ships — link, don't wait).
- [ ] **T2.4** Privacy answers on-page (the consent model: first-run consent, secret-blind redaction, anonymous vs joined, `leave`), so the Intentional principle is verifiable by a reviewer in one click.

## Phase 3 — Launch readiness

### Tasks

- [ ] **T3.1** Owner review of all four fronts against the six criteria (checklist in the grant playbook §13).
- [ ] **T3.2** Deploy + verify; the deploy-ledger audit rows for any worker touched.
- [ ] **T3.3** Freeze until submission (no content churn Nov 17–20).

## Constraints

- Web-family design system holds (family bar, tokens); no token/brand forks.
- No game-business content anywhere (seal-* is a separate brand and out of scope).
- Every claim on-page must be true on the date it ships (the grant lane's no-fabrication law).
- Site sync mirrors (rethink `05_resources`) stay concept-level — no strategy content.
