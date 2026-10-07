# Plan 202 — Decision-stack alignment: refine as the FIRST CUSTOMER of the arsenal/expert pattern (reflex → instinct → rethink)

**Status:** PROPOSED — 2026-10-07 (owner ask: align the arsenal/expert-per-domain architecture across refine · reflex · instinct · rethink — "it should behave the same aka vessel"; outcome-based best-result selection ("any corpus/weight/shit that gains — we pick best result to fix the user problems"); adaptive escalation like the limelight rules with the fallback chain taken into account; refine = the first real customer, rethink = future decision-service customers; long-run per-user LRU expert cache; scale v2 without breaking much — v1 breaking is fine now, only us + low-impact users). Last batch before impl. · Verdict trail: (pending)

## 0. The thesis

Every domain expert — in every repo of this stack — is already the same SHAPE, whatever is inside it: a **signed, versioned, BLAKE3-sealed artifact** (corpus, weights, or both) + **an escalation manifest row** + **gates that decide serve vs abstain vs escalate**. refine ships this shape modelless today (the corpus-is-the-model drafter, sealed artifacts, GOAT-gated domains). instinct/rethink serve the same shape as lanes/seats/vessels. This plan does NOT merge the repos — it aligns the PATTERN so that: (1) refine's per-domain healing is the first real customer of the decision stack; (2) rethink's future decision-service customers ride the same seat; (3) the modelless→trained fallback is measured, not vibes ("we pick best result").

## 1. The alignment table — one concept, four local names (the vocabulary translation)

| Concept | riir-refine (first customer) | riir-reflex (public decision serving) | riir-instinct (open L1 stack) | riir-rethink (private moat) |
|---|---|---|---|---|
| Expert artifact | BLAKE3-sealed corpus (the `rule_embed_frozen_v1.bin` pattern) | sealed corpora | suite/seat manifests | **vessel** (`.vessel`, signed, A10-hosted-only for moat classes) |
| Per-domain routing | `pick_domain` corpus routing | `decision_wire` domains + `pick_domain` | one suite per lane | manifest rows + Choice-criteria law |
| Escalation | `L4Fallback` seam (`DaemonL4` = the remote precedent) | abstain (abstention-first) | `Ext` seat (`install_ext_boots`) | **ESC lane** (`escalating_backend.rs`) |
| Gates | L2 parse + GOAT G1–G4 | G1–G5 (conformal floor) | parity gates | rate guard `[min_rate, max_rate]` |
| Demotion | `--active-set-demote` | kill switches | — | `RIIR_RETHINK_ESC=0` exact-literal demote |

Prior art on record: `tests/g4_l4_composition.rs` (the L4 conservative contract is TESTED wiring, not convention), `DaemonL4` (an L4Fallback already talking to a remote `bonsai_clippy_l4_eval --serve` — the hosted-expert precedent), instinct Proposal 001 (the vessel protocol: A1 bytes-are-runtime/capability-compile-time, A8 no runtime minting, A10 moat).

## 2. The laws this alignment inherits (non-negotiable)

1. **A10 moat:** hosted-only vessels NEVER enter public binaries. refine's shipped artifact stays modelless; trained experts serve ONLY from the hosted tier. (This is why "refine as first customer" = refine CONSUMES the hosted decision plane when escalated — it does not embed it.)
2. **A8:** no runtime minting — the hosted tier loads + verifies vessels; minting is offline; cache eviction reloads from the artifact store, never re-derives.
3. **Abstention-first / decline-is-correct:** every fallback chain ENDS in decline (`L4Fallback::generate_fix → None` is the conservative contract); a hosted timeout, a refusal, or a consent gap all decline — never a forced bad fix.
4. **Manifest-only arming, demote-only kill switches** (the ESC law): no code flip ever arms escalation; the manifest `escalate` table is the only arming surface; the runtime rate guard auto-demotes outside `[min_rate, max_rate]` with receipts (`ESC:demoted(<arm>)`).
5. **GOAT discipline picks the winner:** modelless vs hosted-trained vs hybrid is decided per domain by measured G1–G4 on frozen splits — never by preference. Losers are demoted and recorded ("we don't care what gains — we pick best result").

## 3. The escalation chain — adaptive, limelight-shaped

`L1 modelless (local, free, private) → confidence/abstention gates → [ESC armed AND rate window open] → hosted expert (rethink posture over the decision-service API) → decline`.

Limelight mapping: a domain's **temperature** is a function of OBSERVED escalate-rate (like limelight rings follow the live focus, never existence): HOT (high escalate-rate on real traffic) justifies commissioning a trained head for that domain; COLD stays modelless; the rate guard bounds cost and demotes automatically. Escalation follows measurement, never existence.

## 4. Rows

- [ ] **R1 — the decision/escalate manifest (arsenal-shaped, one grammar, dual-pinned).** refine gains a manifest with the SAME grammar as rethink's `escalate` table: `domain → { modelless cap, escalate: { min_rate, max_rate, target }, demote-only kill switch }`. Dual-pinned goldens (the Issue-082 one-definition law): one wire shape, both sides pin the bytes. Ships ALL-EMPTY (arming is always a manifest edit, never a release).
- [ ] **R2 — `HostedExpertL4`: an `L4Fallback` pointed at the decision-service endpoint.** The `DaemonL4` network precedent, productized: fuel-bounded, timeout → decline, refusal → decline, **consent-gated** (the refine consent law — `Unset` never dials; hosted calls bill through the kat plane's meter when that arms; owner dogfood is free until priced). Registered in the manifest, not in code.
- [ ] **R3 — the instinct `Ext` seat is the hosted side.** refine's escalation target = an `AnySuiteServer` ext seat (`install_ext_boots`) running the healing-domain suite — so rethink's future decision-service customers ride the SAME seat refine dogfooded. refine is the first customer: its traffic hardens the seat (the AOI-reveal-bridge precedent: the sanctioned both-sides consumer).
- [ ] **R4 — vessel-shaped artifacts, formalized.** refine's sealed corpora adopt the vessel protocol's verify path (sign / blake3 / monotonic apply / strict refusal on tamper — A1/A8). Where a trained healing head exists (hosted-only), it IS a `.vessel`; public-release vessels never carry non-public corpus (A10). One artifact protocol across the stack.
- [ ] **R5 — per-user LRU expert cache (hosted tier).** `LaneSlot`-style LRU of loaded user experts/vessels behind the seat: memory-capped, hit/evict metrics, eviction = reload from the artifact store (A8: never mint). The scale law for v2: user count grows, resident experts do not.
- [ ] **R6 — outcome-based selection harness.** Per-domain A/B: modelless vs hosted-expert on the frozen test split (the ab_timing interleaved harness discipline — interleaved pairs, median, loud-zero defence); demote the loser via the manifest/active-set; record the measurement. This row is the standing answer to "we pick best result".
- [ ] **R7 — the v1-breaking ledger.** Enumerate NOW the v1 wire/manifest breaks this alignment wants (decision_wire fields, the R1 manifest grammar, the escalation contract version) and take them ALL in one coordinated break while the fleet is tiny — the owner's "v1 breaking is fine rn" is a budget; spend it once, here.

## 5. Scale-v2-without-breaking (the constraint, restated as laws)

1. A new domain = corpus + manifest row + GOAT — never a pipeline change (the manifest grows rows, not code).
2. The shipped binary never grows trained weights (A10) — the hosted tier scales experts independently (R5's LRU).
3. The escalation contract is versioned; v1 breaks are spent ONCE (R7), then additive forever.

## 6. Tests

- R1 grammar dual-pin (refine manifest ↔ rethink escalate table — the bytes-pinned golden, both repos).
- `HostedExpertL4`: timeout → decline; service refusal → decline; consent `Unset`/`OptedOut` → NEVER dials (asserted by a no-socket test, the consent-law pattern).
- Rate guard: outside `[min_rate, max_rate]` → demote receipt + cheap-only latch (the rethink N=200 rolling test ported).
- A10 leak scan: no vessel bytes in any shipped binary (the `binary_leak_scan.sh` pattern, extended to the escalation lane).
- LRU (R5): eviction/reload correctness under A8; a reload NEVER mints; cold-start latency bounded.
- R6 harness: interleaved A/B with loud-zero defence (the sequential-AB lesson — an optimiser-deleted arm must FAIL the gate, not pass).

## 7. Cross-refs

[Plan 001](001_plan_heal_domain_pipeline.md) (HealDomain) · the `L4Fallback` conservative contract + [tests/g4_l4_composition.rs](../tests/g4_l4_composition.rs) · `DaemonL4` (the remote precedent) · instinct Proposal 001 (the vessel protocol — A1/A8/A10) · rethink AGENTS laws (seam law, pin lockstep, A10, consent) + `escalating_backend.rs` (the ESC grammar) · reflex G1–G5 (the conformal floor) · dapps [Plan 048 §Phase C](../../riir-dapps/.plans/048_master_mainnet_direct_cutover_and_corpus_expansion.md) (the domain expansion feeds this stack) · the mining-queue slot discipline (`.distill/001_mining_queue_snapshot.md`).
