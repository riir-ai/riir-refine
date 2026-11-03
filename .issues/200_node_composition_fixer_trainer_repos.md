# Issue 200 — the node-composition family: `riir-refine-fixer` / `riir-refine-trainer` (superset repos)

**Status:** OPEN 2026-11-03 — owner-directed composition ("riir-refine-fixer = riir-refine + fixer tier code, so it can do anything riir-refine can do + fixer; same as riir-refine-trainer = riir-refine + fixer + trainer"); the design record + owner-act checklist. The lane plan is [Plan 335](../.plans/335_local_cluster_fixer_lane_qwen_local_gate_and_cli_runway.md) (P0 qwen-local gate → P1 posture → P2 this CLI).

## The composition (owner's model — supersets, never forks)

```
riir-refine (core rulewright — UNCHANGED, zero network deps in the default build)
   ▲ path-dep (one way)
riir-refine-fixer = core + FIXER TIER
   ▲ path-dep (one way)
riir-refine-trainer = fixer + TRAINER TIER
```

- **`riir-refine-fixer`** — new PRIVATE repo; its workspace path-deps `../riir-refine` (consume, never fork — the leaf law's shape) and adds the **fixer tier**: the serving-node binary (riir-infer serving seam + the `fix_verify` verify half), the provider ladder (local riir-infer first; **CF/Groq options via `../riir-llm`** — the shared transport leaf, protocol only, cap-checked consumer-side), the cluster client + consent layer (Plan 333 T4's per-repo store), node registration, the TUNA earning wire. Can do anything riir-refine can do **+ fix**.
- **`riir-refine-trainer`** — same pattern stacked one level: adds the **trainer tier** (the Proposal 005 trainer role): quorum replay client (`kat_replay_client` lineage), corpus building / the daily drain window, the prove leg.
- **A tier's heavy deps live in ITS repo** — the boundary-guard discipline: core stays clean; fixer owns riir-llm / riir-infer / the TUNA client class; trainer owns the kat/replay/corpus machinery. `ci_boundary_contract.sh` derives the graph once the repos are registered.

## The trust rule (owner, final form — bakes into both repos' BOUNDARY.md)

**The trust boundary follows CODE OWNERSHIP, not node class.** The contributor cluster serves the **open-source class** + own-repo/donated/public-corpus work, accruing TUNA point weights. **Private-repo targets are the code owner's problem to host: our job is to provide the fixer CLI so they can run their own node** — self-hosted on their own hardware = zero egress to anyone. One binary, two postures: join the cluster (OSS → point weights) or self-host (private code). We never promise custody of proprietary code; we provide the tool that makes custody unnecessary. **"Open-source class" is a CHECKED property, fail-closed (verdict round 3): a target is OSS only if its remote is publicly reachable AND carries an OSI license, verified client-side BEFORE any network connection; anything unconfirmable is PRIVATE → self-host route. The check lives in the client (both repos inherit it).**

## The credit rule (owner: "no KAT, only TUNA — read mozilla proposal properly")

- **TUNA point weights ONLY in the Mozilla lane** — operator-batched cadence per Proposal 006's own law, weights owner-armed, never an agent lever. **Wording per Proposal 025 (line 47, binding): TUNA is "non-transferable, expiring reputation — never purchase, never token value"; and (line 139) no reward attached in the funded scope.** "Earning" language is banned from the Mozilla-facing surfaces — contributors ACCRUE point weights, they do not earn money. **No KAT surfaces in the grant-facing lane** — Proposal 025's posture (charitable installs carry no commercial credit; Posture A builds exclude kat surfaces per issue 190 T14; the points plane is design-disclosed, W0/Q1).
- The **commercial KAT lane** (consumer pass-through pricing, mining rewards) is a separate surface, untouched by this composition — it rides the existing Proposal 005 economy when the owner arms it.

## Owner acts (the checklist)

- [ ] Create `riir-refine-fixer` (private, gist-rs), then `riir-refine-trainer` — with the 22-file registration op (`repo_set.txt` + the per-repo pins) in the same act
- [ ] Root `BOUNDARY.md` for each (owns / does-not-own / allowlist: fixer = core + riir-llm + riir-infer + the TUNA-client class; trainer = fixer's + the kat/replay/corpus class)
- [ ] Public dist repos per the Proposal 003 pattern (source stays private; installers ship from the dist repos; zero `.rs` in dist)
- [ ] TUNA point-weight table + operator-batch cadence (owner-armed; non-transferable, expiring, never token value — Proposal 025 line 47 wording)
- [ ] Feature gate: KAT surfaces in tier repos default-OFF, so a Mozilla-facing build carries none (the T14 discipline, carried up a layer)

## Open design questions (settle at execution)

- The tier bins' identity: new bins in the tier repos composing core's library + tier crates; dist binary names follow the `cargo-refine` lineage (owner picks).
- Versioning/release cadence per tier (independent tags; the `release_set_matches_workflow` pin pattern per repo).
- Contributor onboarding UX rides the points-plane CLI surface (`join`/`leave`/`rank`/`info`/`corpus` — Proposal 025 W0) + the first-run consent law.
- Q4 note: P0 runs Q4_K_M per owner; the trainer tier's prove leg replays on the VERIFIER's rig — the toolchain+model fingerprint skew rules already exist (Plan 127 lineage).
- Self-host posture docs: a runbook for private-code owners (the "setup a private node themselves" path — hardware floor, model staging, the consent gate in local-only mode).

## Related

[Plan 335](../.plans/335_local_cluster_fixer_lane_qwen_local_gate_and_cli_runway.md) (the lane) · Proposal 005 (the tier table + caveat 10's decentralization honesty) · Proposal 003 (the dist pattern) · Proposal 025 (the Mozilla posture: points plane, W3 cohorts, T14 charitable builds) · issue 190 T14 · riir-dapps TUNA plane (Proposals 006/009) · riir-llm (the transport leaf)
