# Issue 053 — unattributed container app `a03a70ff` (94% of container spend, 17.7 GB egress)

**Filed:** 2026-08-28
**Status:** OPEN — evidence gathered, attribution BLOCKED on an owner-only API scope (audit logs 403). Not a cost emergency ($0.33 total); filed as a **provenance/attribution gap**, not a runaway-spend incident.
**Repo:** riir-clippy (surfaced by the Issue 045 container collector)
**Parent:** [Issue 045](045_container_cost_housekeeping.md) T5 — this is the spun-out follow-up.

## What

Application `a03a70ff-88e9-45c5-af80-339ab7707fbe` ran on **2026-08-26** and
accounts for **94% of all container spend** on the account, yet nothing in the
repo deployed it and no control-plane record names it.

| Field | Value |
|---|---|
| est cost | **$0.3292** of $0.3780 total (30 d, all 6 apps) |
| memory | 117,149 GiB-s |
| CPU | 998.5 vCPU-s |
| **egress** | **17.7 GB** (`txBytes`) — 3 orders of magnitude above every sibling app |
| colo | `yyz04` |
| deployments | 3 (`d03e11cb…`, `d03675a3…`, `d03ce6e3…`) |
| backing DO | `4efa6682bedc0e8f…` |
| window | first + last seen 2026-08-26; peak uptime 915 s in the 22:00Z hour |

For scale: the next-largest app is $0.0242, and the other four sum to $0.0245.
The 17.7 GB egress is the part worth explaining — the entire rest of the
account's container egress for the period is ~39 MB.

## Why it was invisible until now

The collector truncated `applicationId` to 8 chars and the 2026-08-26 issue-045
snapshot listed only 3 apps — `a03a70ff` was **not among them**. The dashboard
therefore reported "combined burn ≈ $0.01–0.03/day" while the real 08-26 figure
was **$0.3453**. Fixed in commit `3101787` (full-id retention + correlation
meta); this issue is the surfaced consequence.

## What was ruled out (measured 2026-08-28)

- **Not a repo deploy.** `wrangler containers list` → none. Only 2 DO
  namespaces are container-backed (`hello-cloudflare-container_MyContainer`,
  `seal-poc-container_MyContainer`) against 6 billed apps.
- **Not the Bench 331 pair.** Those are `a03c634b` + `a033f790`, both
  2026-08-14 only.
- **Not `agent-board-worker`.** Its `RoomCoordinator` DO is the account's
  busiest (22k–75k req/day) but it ran continuously through 08-27 and 08-28
  while every container app stopped at 2026-08-26T22Z. Different lifecycle.
- **Not resolvable by name.** `GET /containers/applications/<uuid>` → 404
  `code 1609` (deleted); `label(name:)` empty for 20+ probed keys;
  `/cloudchamber/applications` → 401; no deployments endpoint.
- **Sibling correlation found:** `a035c2cc` + `a034b17e` share DO
  `8d3b8a1d…` and colo `atl15` — one Worker, two billed apps. `a03a70ff`
  shares its DO with nothing.

Working hypothesis (unproven): an ephemeral Cloudflare Sandboxes/Agents-family
workload created and destroyed the same day by tooling outside this repo.
Consistent with same-day create+delete and with zero wrangler-managed record.

## Blocker

`GET /accounts/{id}/audit_logs` → **403 `code 10000`**. The wrangler OAuth
token carries `containers:write` + `cloudchamber:write` but **not** audit-log
read. Attribution needs either:

- **(owner, 2 min)** CF dashboard → Manage Account → Audit Log, filter
  2026-08-26, look for container application create/delete — this names the
  actor and the tool; or
- an API token with **Account → Audit Logs : Read**, after which this is
  scriptable.

## Tasks

- [x] T1 quantify + isolate the app (cost, egress, colo, DO, deployments)
- [x] T2 rule out repo deploys / Bench 331 / agent-board-worker
- [x] T3 exhaust the name-bearing API surfaces (see Issue 045 T5)
- [ ] T4 **owner**: audit-log lookup for 2026-08-26 container create/delete
      (or grant Audit Logs:Read) → name the actor
- [ ] T5 once named: decide keep / kill / budget-alert it; if it is not ours,
      treat the 17.7 GB egress as a security finding and rotate account creds

## Notes

- Time-sensitive only insofar as audit-log retention applies; the GraphQL
  billing rows themselves are already aged (~14 d observed retention), so the
  usage evidence above is captured here rather than left to re-query.
- Cost is NOT the reason this is open — $0.33 is noise. The reason is that an
  unnamed workload moved 17.7 GB off this account and we cannot currently say
  what it was.
