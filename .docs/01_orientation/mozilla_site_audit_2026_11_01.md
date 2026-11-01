# Mozilla alignment — live site audit, all four fronts (2026-11-01)

**Status:** RECORD — input to the Plan 324 Phase-1 re-scope + the §13 copy corrections
**Method:** four parallel read-only subagents, LIVE fetches primary (on-disk source only to
enumerate routes / verify drift), same day. Each front audited against the three principles
(Small / Intentional / Tailored) + six criteria (Human-Scale AI expression, technical
viability, problem-community clarity, product-community fit, impact and values,
sustainability potential). Sessions: reflex `3695ab37`, refine `32571cbd`, rethink
`cde2ce85`, riir.ai `8db8ff01`.

## Headline findings (what the 2026-10-31 plan got wrong)

1. **refine.riir.ai's source is NOT in riir-refine** — Plan 324's target was wrong. The site
   is host-routed from `riir-dapps/cloudflare/kat-service/src/` (`front_page.rs` landing,
   `family.rs` chrome, `env_view.rs` env views, `legal.rs` terms/privacy/risk). The
   `riir-refine/cloudflare/` dir holds only ops workers (housekeeping + dashboard).
2. **The refine strip is structural, not copy-editing.** The KAT/TUNA economy is a
   first-class pillar: a literal `<h2>Pricing</h2>` ("Free on your machine. Metered when you
   log in." + "What is KAT?" / "What is TUNA?"), a KAT contribution leaderboard
   (`/leaderboard`, ranked "by KAT minted"), a lite/pro/max/ultra **tier table**, mining copy
   through the hero and its own flow figure, **mainnet chips on the pulse/adoption cards**,
   refuel doors naming `--mine` **or top up at the network's payment page**, and three legal
   pages with **USDC staking where "the principal is real money"**.
3. **rethink.riir.ai's 2026-10-07 rework is confirmed — but only half the job.** No concrete
   price anywhere, quotas TBA, preview terms/privacy, status chip "hosted API: not open
   (waitlist)". BUT the payment-and-token narrative is still load-bearing: `#buy` section
   ("Pay only where the floor abstains… TUNA first, then KAT"), KAT/TUNA in the glossary and
   trust-flow walk, "One account · KAT · TUNA" in the footer of every page, stake/lease tier
   copy in the v2 roadmap. All fenced planned/test-network — the words `mine/mining/miner/
   mainnet/wallet/crypto` are ZERO on the site.
4. **riir.ai IS the network home — the structural finding Plan 324 missed.** The family
   bar's fourth member is "Network — One account · KAT · TUNA" and on riir.ai it is
   `aria-current="page"`: the umbrella hero sells "Pay only for the hard part", the pages
   host `/pay` (LIVE storefront: 1 USDC = 10 KAT packs, Solana Pay), `/wallet` (passkey
   send UI, "Press and HOLD to send"), `/leaderboard`, `/explorer`, and the footer legal row
   on EVERY front links "Token & staking risk". De-tokenizing the family home is an
   architecture decision (separate the network surface from the family home), not a copy pass.
5. **A moat-vocabulary leak on a public surface.** reflex.riir.ai's sizes-chart click-through
   notes (`reflex-site data/sizes.json`) render *"Instinct is the open teaching lane (052);
   Rethink is the private moat"* — internal strategy vocabulary, publicly visible, exactly
   what a grant reviewer must not read. GAP 9 keeps seal-*/riir-ai unmentioned; "private
   moat" for Rethink is v4-true internally but loud publicly.
6. **The fork/open-source dead end is live on the refine site TODAY:** "Is it on crates.io?
   No — Refine ships as checksum-verified prebuilt binaries only." This collides with the
   Intentional principle's fork/own/govern and makes issue 192 GAP 11 (crates.io
   publication via the T16.5 extraction) more load-bearing than scoped.

## Per-front verdicts (compact)

| Front | Small | Intentional | Tailored | Sustainability | Main liability |
|---|---|---|---|---|---|
| reflex.riir.ai | STRONG (92.1 KB wasm / 4.38 MB native / 1.9–2.1 MB archives vs 6.8–47.3 GB opponents; 151 µs; "no GPU") | STRONG (loopback-only, no telemetry, user-owned corpora, "MIT engine — read, audit, fork", provenance law) | good for devs/agents only | weak (no narrative; KAT·TUNA footer card) | footer "Network — One account · KAT · TUNA" on every page; Rethink storefront/lease copy on /resources; "private moat" in sizes.json notes |
| refine.riir.ai | largely answered, weakly framed ("never calls an AI service" buried in Q&A) | strong transparency ("Our promises — and how to check them", sample batch, sync --export) but FORK contradicted (binaries-only) | partial (Rust/agents clear; no dialogue evidence) | WEAK — token treasury is the only story; post-trial refusal gate reads extractive | the whole economy layer (Pricing/tiers/mining/mainnet/wallet/USDC staking) |
| rethink.riir.ai | STRONG (421M vs 27.8B = "66× fewer parameters"; 484 MB vs 6.80 GB = "14× smaller"; 328 µs vs 681 ms) | STRONG (best axis: every number provenance-chipped, losses visible, "Sharing is a switch you flip", no cookies/analytics) | partial (agent-first; no named community/dialogue channel) | mixed ("Pay only where the floor abstains" is coherent; no published price/GPU costs) | KAT/TUNA/stake/lease vocabulary load-bearing though fenced; two "decision index" meanings collide (leads 72.3% vs JDI #80-of-244); hero bench 2026-09-24 stalest data on page |
| riir.ai (umbrella) | strongly answered via member cards ("without a trained model", "No LLM is on this path") | strongly answered ("counts only, never your text") | partial | "present but token-shaped" | the umbrella IS the network: hero "Pay only for the hard part", /pay + /wallet live, "Token & staking risk" in every footer |

## Urgent small fixes (independent of the strip decision)

- **/pay mint wording** — "USDC on the pinned **devnet** mint" on a **mainnet**-badged,
  armed payment page. On a payment surface this is wrong-copy-or-wrong-mint, urgent
  regardless of Mozilla.
- **Posture-label whiplash** — mainnet badges + live hourly ledger passes vs "Test network
  today" / "runs test environments today" step copy across refine + the risk page. Pick one
  posture story per surface.
- **"private moat" scrub** — reflex-site `data/sizes.json` note → neutral wording
  ("Rethink is the hosted research lane").
- **Stale "today"** — refine corpus-lease card shows the 2026-10-03 snapshot labeled
  "today" beside the 2026-10-31 shipped-rules chart (same page, two counts, three weeks
  apart); account counts disagree across pages (leaderboard "4 accounts" vs adoption
  "1 accounts · 4 mining").
- **Version drift** — reflex SKILL.md stamped "tested against 0.2.3", site ships 0.2.4.
- **Naming** — rethink's "decision index" collision (our board leads vs JDI #80-of-244);
  one-word rename closes it ("our decision-index board" vs "the Jev Decision Index").

## What the fronts already prove (framing, not inventing)

- **Measured-everything culture**: "every byte count is measured … never hand-typed"
  (reflex sizes law); "Rendered from the network's own records … nothing here is typed in"
  (refine); provenance chips on every rethink number (board run, machine, feed hash,
  snapshot sha); losses stay visible; "A missing bar reads 'not run', never zero".
- **Verifiable promises**: "Our promises — and how to check them" with a `check it:`
  recipe per promise; "run it with the network off… watch it in nettop"; a real redacted
  sample batch published; receipts carrying input/decision hashes.
- **Privacy posture**: "Nothing, by default" boxes; loopback-only; no cookies/analytics/
  trackers; pseudonymous key accounts; counts-only sharing.
- **The healing traction story exists but is visually subordinate to empty economy cards**
  (63 fixes/epoch, 96% p50 fix rate, 1,546 rules, ladder gen 8, vs "0 KAT minted · pool
  backing 0" pulse cards).

## Consequences for Plan 324 / the §13 copy

- T1.1 re-scoped: structural rework in `riir-dapps/cloudflare/kat-service/src/` (not
  riir-refine); shared network pages (leaderboard/explorer/wallet on riir.ai) are OUT of
  T1.1 scope — separating them from the family home is an owner decision (T1.3).
- T1.2 re-scoped: vocabulary de-emphasis + naming fix + bench refresh (already price-clean).
- T1.3 re-scoped: the family-home-vs-network-home separation decision (owner-gated), with a
  light pre-consult option (reword the umbrella hero; keep the network surfaces honest).
- NEW T1.7: the urgent small fixes above (all copy-level, no architecture).
- §13 correction (truth law): the impact row's "Open core (MIT)" is aspirational today —
  the public surface is binaries-only; reworded to "open corpus (MIT, live today) + a
  committed, dated open-core plan" (the consult email already carries that commitment).
- Sequencing: the consult email (~Nov 7) does NOT require stripped sites — it already
  discloses "we are removing the old credit/token lane from the public builds entirely
  before any public release". The Nov 17 application does require the fronts consistent
  with the narrative (T3.1 owner review gates that).
