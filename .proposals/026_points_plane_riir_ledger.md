# Proposal 026 — `riir-ledger` v1: the contribution-points plane (wire, schedules, leaderboard rules)

**Status:** Proposed — T16.2 of issue 192; the model canon is [Proposal 025 §"The points plane in full"](025_mozilla_human_scale_ai_incubator.md) (units, three binding laws, ledger shape); implementation plan is [Plan 325](../.plans/325_public_goods_cli_surface.md); the privacy law is [`points_privacy_id_design.md`](../.docs/01_orientation/points_privacy_id_design.md) (T16.1). This proposal is the HOW: wire shapes, schedules, rank rules, substrate reuse, gates.
**Lease closed:** zed-m3-p2, 1793562411, claimed 2026-11-02, expired 2026-11-04T19:46:51+00:00 — P2 executed within the window (riir-dapps `1258bf1`+`b72b937`, Plan 061).

## TL;DR

`riir-ledger` v1 is a **leaf wire crate + a kat-service points plane**, not a chain:
balances live in the riir-dapps service DB on the existing TUNA substrate (fast
operator mint, expiry, non-transferability — already points semantics), epochs are
Merkle-anchored via the existing `anchor_root` substrate with roots published
public, and the only user-facing unit is **TUNA contribution points**. KAT is ripped
out of the user story (frozen, not deleted). Local `fix` is always free and never
blocked (binding law 1); the drain is NOT v1. Rank requires GitHub-joined identity;
expiry decays the rank weight only — history and corpus are permanent. Every public
surface obeys the vocabulary law (contribute/contribution, never mining) and the
client-UUID privacy law.

## Decisions recorded here (T16.4, resolved)

- **Name:** `riir-ledger` — the wire/leaf crate name (issue 192 GAP 6 recommendation,
  approved by the owner's 2026-11-01 execution approval of Proposal 025).
- **Anchoring tier:** service DB (riir-dapps) + `anchor_root` epoch anchoring;
  **chaind deferred to seal-***; no Refine chain consumer (decision 6); a
  chain-anchored variant needs an explicit new owner amendment and only if anchoring
  proves insufficient. No ssh-key custody plane.
- **No new storage** (Proposal 025 perf/sec posture): the dapps `kat_ledger` TUNA
  arm already IS the durability-tier store (`local_kv::{LocalKvStore, WalBackend}` +
  `MerkleTree`) with the per-batch `kat:statehash:v1` LtHash fold and the shipped
  per-op conservation invariant (`Σ balances + healed + expired == TUNA_RESERVOIR +
  topups`, `conservation_holds: true` live today). `riir-ledger` v1 adds rows + a
  rank table + an epoch anchor ON this substrate; new storage is refused unless a
  future proposal proves one necessary.

## 1. Where the code lives

| Piece | Home | Shape |
|---|---|---|
| Wire DTOs + the points protocol | **`riir-ledger`** — new leaf repo (the `riir-kat` shape: zero sibling deps, serde-only protocol tier, self-contained manifest, a test asserting it) | the wire is the contract: field names/order/serde attributes; `FORMAT_VERSION` + golden fixture bumps in the same commit |
| Service routes + balances + rank table + epoch settle | **`riir-dapps` `cloudflare/kat-service`** — new `points_*` route modules over the existing ledger DO | reads/writes the TUNA plane's store; no new storage engine |
| Client verbs | **`riir-refine`** — new `src/points_client.rs` (std-only HTTP, the `seat_rerank` shape) + the verbs per Plan 325 | the public build's network half; NOT the `kat_transport` ureq plane (that stays internal/KAT) |
| Identity substrate | existing: riir-auth account key (signing) + the GitHub device flow (`read:user`) + the client UUID (T16.1) | join = a signed binding row; no new auth code |

Leaf law check: `riir-ledger` depends on nothing (serde + zero). The dapps plane and
the refine client each consume it one-way; no game crate ever names it (fails the
three-test value test and rate test by construction — non-transferable, expiring,
written at service rates — a service-plane product, never a chain program).

## 2. The wire (v1)

Envelope: the same shape family as the existing service lanes (canonical JSON,
`format_version`, typed refusals naming a code + a human sentence; the decision
envelope's abstention-first pattern). Every request that mutates is signed by the
account key; every read is scoped to the caller's own rows or to public aggregates.

| Endpoint | Method | Auth | Behavior |
|---|---|---|---|
| `/points/join` | POST | account-key signature + GitHub OAuth token (the existing device flow) | validates the token (`read:user`), writes the binding row `(client_uuid, github_id, account_pubkey)`, returns the joined posture. Idempotent per `(github_id, account_pubkey)`. |
| `/points/leave` | POST | account-key signature | deletes/deactivates the binding row server-side, stops all rank eligibility, returns ok. **Zero data is sent afterwards** — the client simply never calls again; there is no goodbye round-trip dependency. |
| `/points/balance` | GET | account-key signature (or client UUID for anonymous local-view echo) | own current balance + epoch + rank eligibility (joined only). Never another account's rows. |
| `/points/rank` | GET | none for the top-ten (public aggregate); account-key signature adds "own rank" | top-ten joined identities + (if signed) the caller's rank. Anonymous callers get the public top-ten and a local-stats pointer, never a rank. |
| `/points/info` | GET | none | aggregated, redaction-policy-scoped counts: top contributed-to rule domains + (coarse, count-only) top repos **as counted by the existing sync policy's repo bucketing** — counts + domains only, never repo content, never per-user rows. |
| epoch anchor | published | none | per-epoch `anchor_root`/`chunked_root` hex + epoch number + row counts on the existing status/explorer JSON. **Roots only** — no client UUIDs, no GitHub ids in any published anchor or stat. |

Wire purity rule (the `riir-kat` precedent): `cargo check --no-default-features`
compiles the protocol tier with zero optional deps; anything needing policy or
transport is in the wrong tier.

**Privacy fields:** the only client-supplied identifier is `client_uuid` (T16.1 §1).
No hardware, hostname, MAC, or OS-machine-GUID field exists in any DTO — the T16.1
§2 refusal list is enforced by the DTO shape itself (a field that could carry it
does not exist), and a wire-contract test asserts the absence.

## 3. Earn schedule (v1 shape)

- **Source of truth:** the existing contribution lane — signed sync payloads
  (fix metadata, secret-blind, redaction-policy-scoped) already flow from opt-in
  machines; `proven` corpus contributions are the points-earning events.
- **When:** at **epoch settle** (the one-week accounting period the network already
  runs). Contribution activity inside an epoch is accumulated; settle mints once.
- **How:** the mint rides the existing **TUNA operator batch** — the fast shape, no
  2/3 quorum wait (exactly the operator-batch semantics TUNA ships today). No new
  mint path.
- **Rate shape:** `points_epoch = Σ_proven contributions weight(contribution)`, where
  weight is a small public table keyed by (domain, rule class, proven-duplicate
  discount) — L1 distillation-worthy contributions weigh more than duplicates.
  ⛔ **The numeric table is an owner gate before any mint ships** (the honest-targets
  discipline: the shape ships, the numbers are ratified, then the plane goes live).
- **No earn from:** hosted-lane usage, install counts, or anything a user could
  farm by volume without a proven contribution. Batched per-machine syncs stay in
  the shipped burn-sync posture (~5 s budget, never blocks the run, monotone
  watermark idempotent replay) — 10K hourly syncers ≈ 3 QPS, trivial.

## 4. Drain — deliberately NOT v1

Binding law 1: local `fix` is always free, points never go negative-locked, and v1
public copy leaves the drain OUT entirely. The numeric fair-use policy (drain rate,
balance cap, refill paths) is defined by the follow-up proposal —
[**Proposal 027**](027_points_drain_fair_use.md), filed + AGREE'd 2026-11-02 —
BEFORE any drain ships. Until ratification the only public sentence is "points
meter hosted lanes as fair use" (roadmap framing on all fronts — Plan 324 G-B law).
The design bar is already pinned (Proposal 025): a cached-balance O(1) pre-check
≤ 1 ms p99 or post-hoc accounting — never a synchronous remote-permission check on
the decision path (027's client pre-check meets it; enforcement is server-side at
the seat).

## 5. Leaderboard rules (rank)

1. **Eligibility:** GitHub-joined identities only. Anonymous/derived IDs are never
   rank-eligible (the anti-Sybil rule — a rotating token could farm the board);
   anonymous callers get local stats only.
2. **The rank weight is derived and decaying; history is permanent** (verdict r1
   GAP 4b): `weight_i = Σ_j contribution_j · 2^(−age_j / H)` with `H` the half-life
   in epochs (default 2, provisional pending owner ratification alongside the earn
   table). The ledger's shipped tranche-expiry mechanics (expired tranches burn to
   `expired_micro`, Proposal 006 B4) are unchanged substrate behavior; this weight
   is a rank-view over the same permanent rows, not a second balance.
3. **Computation:** ONCE per epoch at settle (the natural recompute tick), served
   from cache — O(1) queries, never a per-query scan. A mid-epoch join appears at
   the next settle (stated, not hidden).
4. **Ties:** fixed strict tie rule — higher weight, then earlier first-contribution
   epoch, then lexicographic handle. Deterministic recompute is a gate (same rows →
   byte-identical board).
5. **Privacy on the board:** handles + points only. No client UUIDs, no GitHub
   numeric ids, no repo names in the board payload.

## 6. Anchoring + tamper evidence

- Per batch: the existing `kat:statehash:v1` LtHash fold (O(1)-class per mutation).
- Per epoch: `anchor_root` over the epoch's row set — **`chunked_root` REQUIRED**
  (plain `MerkleTree` hard-asserts at `MAX_LEAVES = 2²⁰`; a million-mutation epoch
  sits at the panic edge by construction — the Bench 488 chunk-safe lane is the one
  that ships).
- Publication: roots (+ epoch, counts) on the public status/explorer JSON via the
  existing hourly pass machinery. **OFF-CHAIN by decision 6.**
- No Cold-tier mirror in v1: the statehash trail + public roots + per-op
  conservation are the tamper-evidence story (a mirror would need a new dapps
  BOUNDARY row — declined for v1).

## 7. Existing accounts (GAP 7, one honest paragraph for the FAQ)

Live KAT accounts: marketing is frozen (the posture), internal devnet flows keep
working, and the refine site FAQ carries the "what happens to existing balances"
paragraph — honesty beats silence. No KAT→points conversion is promised or implied:
points are a new plane; KAT lanes are frozen, not deleted (proposal 023
DEFERRED-FROZEN; issue 188 carries the un-freeze note).

## 8. Phases + gates

- [x] **P1 — `riir-ledger` leaf crate**: protocol DTOs (§2), `FORMAT_VERSION` + golden fixture, the no-hardware-field wire-contract test (T16.1 §2), zero-dep leaf assertions; `cargo check --no-default-features` clean. **— LANDED 2026-11-02:** local repo `/Users/kat/git/riir-ledger` commit `e679634` (serde + serde_json only; 23 tests green: 14 wire goldens + 6 privacy-law arms incl. the v7-shape refusal + 3 leaf-law; clippy `-D warnings` clean). **— REMOTE LIVE 2026-11-02 (`gist-rs/riir-ledger`, owner act):** develop pushed (`ad86ce6` + the repo-init LICENSE merged in, `e4c319a`) + the leaf-gate workflow (`fd68aa0`: clippy at both feature tiers, the full suite, the leaf-law test by name with a passed-count floor — no siblings, no token); main fast-forwarded to the same commit; the CI checkout rows (riir-refine pre-wired, dapps rust.yml + deploy_kat_service.yml joined this session) are unblocked. Default-branch flip to develop: a dashboard click (gh unauthed on this box); both branches carry the same tree so nothing waits on it.
- [x] **P2 — service plane (local/devnet)**: `points_*` route modules over the TUNA store; binding rows; per-op conservation assertion extended to points rows; epoch settle + `chunked_root` anchor publish; rank recompute + cache; contract tests (typed refusals, anonymous-get-local-only, leave-unbinds). **— LANDED 2026-11-02 (riir-dapps Plan 061, `1258bf1`+`b72b937`):** `src/kat/points.rs` (the `points:` row family on the SAME ledger store — bindings + github-id index + points state + handles + the rank cache + the anchor chain; the FIRST dapps consumer of neuron-db's `chunked_root`) + the five `/points/*` routes + `GET /points/anchor` in `cloudflare/kat-service` (riding KatLedgerDo's default fall-through — no new DO, no edge forward); the crank's points-settle leg recomputes the board + anchors each ENDED epoch once (the exclusive watermark); 616 worker tests (incl. the 14-test points contract suite replaying the client's exact signing compositions) + 7 lib tests; clippy `-D warnings` + wasm32 check green. **Dated amendments, both recorded in Plan 061's decisions:** (1) **join carries NO OAuth token** (the landed wire — the client drops it in memory per the privacy law), so the §2 "validate the token" posture is amended to the login↔id PUBLIC-profile consistency check (mismatch `403 provider_mismatch`; transport fault fails open, counted — Plan 061 decision 4); (2) **the refusal envelope is the leaf's `ErrorEnvelope`** `{format_version,code,message}` on EVERY `/points/*` non-2xx — the dapps `{"error","code"}` family would render as the client's false W0 sentence. **Dated decisions:** points rows do NOT join `kat:statehash:v1` (the shared-width money predicate is load-bearing) — the anchor chain is the plane's tamper story; rank decay ships as the documented integer step (`points >> age/H`, H provisional 2, owner-gated); board display = balance, order = decayed lifetime weight, ties per §5.4 + the account-bytes terminator. ⛔ Owner residue: the earn/mint half (ratified table first), the `tuna.riir.ai` custom_domain row, the crates.io publish (T8).
- [x] **P3 — client verbs**: per [Plan 325](../.plans/325_public_goods_cli_surface.md) (`join`/`leave`/`rank`/`info`/`corpus`/`version`, first-run consent, `RIIR_REFINE_OFFLINE=1`, the feature-gate map). **— LANDED 2026-11-02:** Plan 325 T1–T7 + T9 (T8 crates.io owner-gated). The service-facing verbs answer the not-yet-deployed P2 honestly (`ServiceNotLive` → the W0 sentence); the default dial is the points plane's OWN host `https://tuna.riir.ai` (renamed from `points.riir.ai` 2026-11-02 pre-deploy — owner: the old name read boring; same worker family, route-based at P2; keeps the charitable binary free of the KAT ledger host literals — the charitable gate's hard-zero host class stays intact; env `RIIR_REFINE_POINTS_SERVICE_URL` overrides).
- [x] **P4 — GOAT + bench gates**: earn-sync batch cost (≤ 5 s budget, never blocks the run — measured, box state recorded); rank query O(1) from cache (p50/p99); epoch settle + anchor cost per 1M rows; determinism (same rows → byte-identical board + anchor); privacy gates (no client UUID/GitHub id in any published anchor, board payload, or stat — asserted by test, not prose). **— CLIENT-SIDE HALF 2026-11-02:** the transport is budget-bounded (`POINTS_TIMEOUT_SECS = 5`, never blocks the run); the privacy gates ride the wire crate's shape tests (the no-hardware walk) + the board payload law (handles + points only). The service-side measurements (rank O(1), settle/anchor cost, determinism) are P2's — the gates price them when the routes exist. **— SERVICE HALF LANDED 2026-11-02 (dapps `.benchmarks/025_points_scale_goat.md`, the `scale_bench` lane):** rank O(1) **PASS** — p99 flat 0.50–1.38 µs across a 1000× account range (bars: p50 ≤ 20 µs / p99 ≤ 100 µs at ANY scale); settle + anchor @ exactly 1M `points:` rows (worst case — every binding active + positive) **PASS** at 0.593 s vs the 30 s DO-wall bar; determinism **PASS** at the 1M-row scale (byte-identical root + board across fresh stores, on top of the contract suite's toy-scale assert); redaction by shape re-asserted at scale. Earn-sync service ingest: **nothing to measure by design** — the earn table is the owner gate (§3), the path does not exist as code; the client half's 5 s budget landed with P2. The lane re-runs: `cargo test --release --lib scale -- --ignored --nocapture`.
- [x] **P5 — sweep + posture**: the T14-family sweep re-scope's positive assertions (join/points/corpus surfaces present + consent-gated — Plan 325's task row); vocabulary law sweep over all new user-facing strings; charitable-gate green at both feature extremes. **— LANDED 2026-11-02:** the sweep's positive half IS Plan 325 T6 (the charitable gate PASSes with the arms); the vocabulary law is pinned by tests + the gate's consent-text pins.
- **Owner gates before any public mint:** the numeric earn table (§3) + the rank
  half-life (§5.2); the drain follow-up proposal (§4 → **[Proposal 027](027_points_drain_fair_use.md)**,
  FILED + AGREE'd 2026-11-02 — the ratification act (earn table + H + A/B/R/C/P)
  clears it) before any drain ships; the deployment act itself (devnet first, prod
  via the touch-deploy lane).

## 9. Compliance

- Vocabulary law in every wire-facing string, help text, and doc (contribute/
  contribution/contributor/contribution points; internal identifiers rename only
  when touched; the T14 detector patterns keep the old words by design).
- Roadmap law until W0 ships: no front describes the plane as live (Plan 324 G-B).
- Build law (Plan 324 G-C): the sweep re-scope (P5) lands before network code joins
  the public build.
- Verify-before-claim does not bind here (Posture A/C — no payment/token claims
  anywhere; points are disclosed as reputation design).
