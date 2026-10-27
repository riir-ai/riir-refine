# Issue 183 — the migration release gate: a filing-created proposal issue + the owner's 👍 REQUIRED (default-deny flipped from objection-endorsement to release-authorization)

**Status:** OPEN — owner-directed 2026-10-27 ("auto release migration is too
scary bc owner won't notice — no proposal to subscribe via github issues").
Amends the ratified release posture of
[riir-dao Proposal 001](../riir-dao/.proposals/001_kat_account_key_rotation_social_recovery.md)
(directive item 12's nobody-objected-release + the E3/E4 polarity). Blocks the
T7 devnet rehearsal + T-MIGRATE (rehearsal rehearses the REAL rules — this
lands first). Companion: `.issues/175`, `.plans/302` (T1 consumed by this).

## The owner's four requirements, audited against the shipped wire

| # | Requirement | State |
|---|---|---|
| 1 | Old account signs the agreement | ✅ EXISTS — dual-sign, `MIGRATE_RELEASE_DOMAIN` over the request body (`kat_protocol_migrate.rs:93`) |
| 2 | New account signs the agreement | ✅ EXISTS — dual-sign, `MIGRATE_CLAIM_DOMAIN`, same body |
| 3 | A GitHub issue FOR the proposal (filing-created, subscribable, others object ON it) | ❌ **GAP** — today the issue is composed by an OBJECTOR (`--object` prefilled URL); if nobody files one, `tally_verdict` line 213: **`issues.is_empty() → Release`** — a silent release with zero GitHub presence |
| 4 | katopz 👍 (the 51% vote) REQUIRED to release | ❌ **GAP, inverted** — today katopz 👍 on an objection CANCELS (endorse-objection polarity, E3); there is no positive-authorization gate anywhere |

**The hole, stated plainly:** a migration whose every party stays silent
releases itself after 24 h. The owner's defenses in that world are luck
(noticing the request via `--status`) — the lane has no notification surface,
no subscription, no mention. The owner's fix is the socially-correct one:
**GitHub is this lane's root of trust** (PoW gates, binding moves) — so
release must PROVE GitHub-side owner attention, not merely survive absence of
objection. A thief who controls both keys (stolen old + a qualified new) can
file and wait today; under this amendment they are dead in the water — they
cannot produce katopz's 👍.

**Verdict:** AGREE round 2 (Claude, 2026-10-27) — see the Verdict log at the
bottom of this file.

## The amendment (design)

### A. The filing creates the proposal issue (worker-side, fail-closed, IDEMPOTENT)

At `POST /account/migrate/request` acceptance, the worker creates ONE issue in
the migrate repo (`KAT_MIGRATE_REPO_BASE`, behind the existing reactions-reader
trait — a new `IssueCreator` seam + a GitHub token scoped `issues:write` on
that repo ONLY; new worker secret, a T-KEYS row).

**Mention targets (the notification surface — verdict R1):** the body mentions
1. **`@katopz` EXPLICITLY** — being the owner sends no notification unless
   katopz watches the repo; the mention IS the notice the owner asked for;
2. **the OLD account's bound GitHub login, if one exists** — the potential
   victim (`kat:mgithub:` reverse lookup); their 👎 is the hard cancel (E3d);
3. the claimant's `github_user` (informational — a thief filing gets notified
   of their own filing, which protects nobody, but keeps the flow honest).

The body also carries: old/new account hex (the wallet link — anyone can
cross-check what the CLI showed), the frozen PoW summary, the window expiry,
and the verdict rules verbatim. The ack carries `proposal_issue_url`.

**Idempotent creation (verdict R4):** before creating, SEARCH the repo for an
existing issue titled with the request id AND opened by the worker's own
token; persist that number into the request row (`proposal_issue`) instead of
opening a second one. The lost-response race (GitHub created it, the reply
died) then converges on the SAME issue — and under the single-issue tally
(below) any stray duplicate carries no votes by construction.

**Fail-closed:** creation failure does NOT refuse the filing (the request row
exists; signatures were verified) — but `proposal_issue: none` leaves the
request **unreleasable**: the close refuses until an issue matches (the lazy
close retries creation on each status read).

### B. The polarity flip — the issue is a PROPOSAL, reactions are votes ON it

`+1` (👍) ENDORSES THE MIGRATION (release vote); `👎` OBJECTS (cancel vote).
One venue per request; `--object` becomes "open the proposal issue and react
👎" (same prefilled-URL composer, now pointed at the proposal issue).

**Single-issue tally + one vote per login (verdict R2 — closes a LIVE hole):**
the tally reads ONLY the worker-created `proposal_issue` — today the matcher
aggregates EVERY issue whose title carries the request id with no per-login
dedupe, so one throwaway account reacting on a copycat issue scores
2 × 4900 bp and the kernel doc's "one non-owner can never out-vote the owner"
is already false; after the flip the same trick works on the RELEASE side
(three spam 👍 = 14 700 vs the owner's 5100). The snapshot pins the issue
number; all other issues are evidence-only. One login reacting BOTH ways =
no vote (the classical AB-strike rule). The snapshot domain tag bumps
`kat:migrate:snapshot:v1` → `v2` (verdict R6) — 👍 means the opposite now, so
a digest computed under the old meaning must not verify under the new.

### C. The release rule — owner endorsement REQUIRED (the owner's point 4)

`Released` ⟺ **owner_endorsed ∧ endorse_bp > reject_bp ∧ matched-issue
exists ∧ ¬victim_objected**, else `Cancelled` at window close. The owner's
👍 is a STRUCTURAL gate — no config knob exists or may exist (verdict R5: a
runtime `owner_required: bool` would bring auto-release back through config;
the dao changes the LAW by re-pinning the T5 artifact, never by flipping a
bool). The victim's 👎 is ALSO structural:

| Cell | Votes | Old verdict | NEW verdict |
|---|---|---|---|
| E1 | owner 👍 only | (objection era: n/a) | **RELEASED** |
| E2 | zero votes | released (nobody-objected) | **CANCELLED** (default-deny preserved — re-file is one line) |
| E3 | owner 👍 (objection era) | cancelled | **RELEASED** — the polarity cells SWAP |
| E4 | owner 👎 only | released | **CANCELLED** |
| E3b new | owner 👍 + one other 👎 | — | RELEASED (5100 > 4900) |
| E3c new | owner 👍 + two others 👎 | — | CANCELLED (9800 > 5100) — the two-human out-vote survives, now on the cancel side |
| **E3d new** | **owner 👍 + the OLD account's bound login 👎 (when a binding exists)** | — | **CANCELLED — UNCONDITIONALLY, weights ignored** (verdict R3): the potential victim's own objection always wins — the defense against owner fatigue/mis-click; the victim can always stop their own eviction |
| E12 new | no matched issue | release | **refuse release** (the item-12 flip; the fail-closed posture) |

E5 PoW · E6 tombstone · E7 claimant-cancel · E8 idempotence · E9 dispute
quorum (exactly-one passes its tally) · E10/E11 — UNCHANGED. A copycat
duplicate issue (post-R2) carries zero votes by construction.

**Filing-time freeze + lookup endpoint (round 2):** `old_bound_login` is
copied from the binding that exists AT FILING and pinned in the request row +
snapshot — a binding change mid-window can never move the E3d veto. Residual
risk, stated: a thief holding the old key could rebind the old account to a
login they control BEFORE filing, stealing the veto — the owner's 👍 still
blocks that path (the issue body names the old account's hex + who is
mentioned), so E3d protects a victim whose binding is still intact. The R4
search-before-create uses `GET /repos/{o}/{r}/issues?creator=<bot>&state=all`
(list + title match) — the search API's index lags seconds-to-minutes and
would miss a just-created issue in the lost-response race; the list endpoint
is consistent immediately.

### D. Kernel + wire deltas (additive, the pay-intent mirror pattern)

- `kat_protocol_migrate_tally.rs`: the owner gate + the victim-objection gate
  land as STRUCTURAL branches — no bool knobs (verdict R5); the snapshot
  shape gains `proposal_issue: u64` (only that issue's reactions are votes)
  + `old_bound_login: Option<String>` (the E3d veto subject); per-login
  dedupe (both-ways = no vote); the empty-snapshot arm flips to `Cancel`;
  the snapshot domain bumps v1→v2 (R6); every E-cell test re-pinned
  (E3/E4 swap, E3b/E3c/E3d/E12 new).
- Request row + `proposal_issue: Option<u64>` (serde default — a pre-amendment
  row reads `none` and can never release: honest posture).
- **Deploy gate (verdict R8):** before the worker ships, query the mainnet
  request table and ASSERT zero pending rows — "none exist" is an assumption
  until queried; the assertion is a deploy-time refusal, not prose.
- The dao T5 artifact re-pins through the kernel (it golden-locks kernel
  defaults BOTH directions — the weights table + the structural gates move
  in ONE decision; the dao amendment names the gates as LAW, so a future
  governor cannot re-open auto-release without a proposal-visible artifact
  change).

### E. What deliberately does NOT change

The dual-sign law, one-pending/dispute quorum, claimant-only cancel,
frozen PoW, census-before-release, sweep atomicity + conservation, the 24 h
window cadence, tombstone semantics.

**The lost-GitHub scenarios, stated precisely (verdict R7):** if katopz (the
`owner_login`) loses GitHub access, EVERY migration on the lane halts — the
owner's 👍 gates all of them, not katopz's own account alone. The escape is
explicit: the dao re-pins `owner_login` through the T5 artifact (a
proposal-visible governance act, the same lane that pinned 0.51). The
separate case — a HOLDER who lost their key AND their GitHub — is dao T9's
guardian escrow (deferred, the designed answer); this amendment does not
attempt it.

## Routing (BOUNDARY)

| Slice | Repo | Scope |
|---|---|---|
| T1 kernel | riir-kat | polarity flip + `owner_required` gate + E-cell re-pins (the ONE crate worker + dao both compile) |
| T2 close engine | riir-dapps | the `IssueCreator` seam + token + creation-at-file + the close gate (matched-issue + owner vote) + the ack field + lazy retry |
| T3 CLI | riir-refine | the ack renders the proposal-issue URL + the 👎/👍 rule text; `--object` re-points |
| T4 governance | riir-dao | Proposal 001 amendment section + the T5 artifact re-pin through the kernel |
| T5 battery | riir-dapps + riir-refine | the rewritten E-matrix (E3/E4 swap, E3b/E3c/E12 new) in `migrate_contract.rs` + the refine E-cells; THEN plan-302 T1 (the devnet rehearsal) runs the REAL rules |

## Open knobs (owner calls at verdict)

1. Zero-vote-at-close → Cancelled (this design's choice, default-deny —
   verdict round 1: AGREED; held-pending would block the one-pending slot
   indefinitely, a denial of service).
2. The `issues:write` token: one repo-scoped fine-grained PAT (recommended)
   vs the existing reactions-read token widened (refused — least privilege).
3. Window: 24 h stands (a missed 👍 = re-file; the re-file is one line).

## Verdict log

- **Round 1 — REVISE (Claude, 2026-10-27):** four must-fixes folded — (R1)
  the mention targets (katopz explicitly + the old account's bound login;
  the claimant mention alone notifies the thief); (R2) single-issue tally +
  one-vote-per-login (the copycat-issue double-count is a LIVE hole today —
  the kernel doc's "one non-owner can never out-vote" claim is already
  false); (R3) the victim-objection hard cancel (E3d) against owner
  fatigue/mis-clicks; (R4) idempotent issue creation (search-before-create,
  the lost-response race). Four should-fixes folded — (R5) no
  `owner_required` bool, the gate is structural; (R6) snapshot domain
  v1→v2; (R7) the lost-GitHub wording (holder vs owner; the dao re-pins
  `owner_login`; T9 is the lost-both answer); (R8) the zero-pending-rows
  deploy assertion. Judgment calls adjudicated AGREE: owner-👍 as root of
  trust (conditional on R1/R3); zero-vote → Cancelled; fail-closed empty
  snapshot (conditional on R4); the flip's new holes bounded (spam 👍 cannot
  pass the owner gate; two throwaway 👎 can grief-block any migration —
  already true, griefing not theft, left to the T5 weight refinement).
- **Round 2 — AGREE (Claude, 2026-10-27):** all eight items confirmed
  folded. Post-AGREE consistency fixes applied (the duplicated section-C
  table removed; the routing T1 row re-worded to the structural gates; the
  T5 row carries E3d). Two design lines added per the verdict: the
  filing-time `old_bound_login` freeze (with the rebind-before-filing
  residual risk stated — the owner's 👍 blocks that path) and the list-
  endpoint (not search-API) lookup for idempotent creation.

## Acceptance

- [ ] Kernel: E1/E2/E3/E4/E3b/E3c/E3d/E12 green at the new polarity; the
      single-issue + per-login-dedupe rule pinned; the snapshot domain v2;
      the dao artifact re-pin green both directions.
- [x] Worker: idempotent creation-at-file proven (search-before-create + the
      fixture reader); fail-closed close on no-issue proven; the ack carries
      the URL; the mainnet zero-pending-rows assertion is a deploy gate.
      — riir-dapps `e968b3e` (T2 + the worker half of T5): `IssueCreator`
      (new secret `KAT_MIGRATE_ISSUES_TOKEN`), row +`proposal_issue`
      +`old_bound_login` (338 B, legacy 289 B reads None), lazy retry,
      E12 close refusal, single-issue reader, R8 gate
      `cloudflare/kat-service/scripts/migrate_zero_pending_gate.sh` (wired
      into `scripts/setup.sh --mainnet`; NOT into deploy_kat_service.yml).
- [ ] CLI: the URL + rules render on file and on `--status`.
- [ ] The E-battery (rewritten) green in-process; THEN T7 devnet rehearsal
      (plan 302 T1) runs the SAME battery on the live worker.
- [ ] The amendment section lands in dao Proposal 001 citing this issue.
