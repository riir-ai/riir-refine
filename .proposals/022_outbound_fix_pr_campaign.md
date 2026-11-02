# Proposal 022 — Outbound fix-PR campaign: harvest → heal → upstream PR

Status: **draft — owner verdict RECORDED 2026-10-08: marketing posture DECIDED (measured perf deltas allowed for bench-measured rules, NotBenchable rules carry no number; logo + tool-name + site attribution allowed); account identity RULE (owner-approved sends → owner account; unattended daily cadence → labeled bot account). Round-1 review's blanket no-numbers caution SUPERSEDED by the owner decision; velocity/MSRV/reviewability discipline stands.**
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Proposal 003 (binary-only distribution) × Proposal 013 (consumer install tiers) × Proposal 016 (public self-evolve reveal) × riir-dapps Proposal 004 (fix-stats funnel card)
Related: [`.distill/001_mining_queue_snapshot.md`](../.distill/001_mining_queue_snapshot.md) (the INVERSE direction — inbound rule mining from upstream sources), [Research 203](../.research/203_rrsi_regularized_harness_evolution.md) (self-evolve series), Issue 089 T6 (provenance strata), [Proposal 015](015_replay_endorsement_reward_class.md) (reward classes — out of scope here, see §Out of scope)

## TL;DR

Run the healer on **third-party GitHub Rust repos** and send the fixes back upstream as
pull requests — the outbound half of the distill loop (we already mine upstream commits
FOR rules; this ships fixes TO upstream). The win is threefold: (1) **adoption evidence**
— real-world merge outcomes are the strongest proof the healer works outside our own
workspace; (2) **provenance strata from real-world code** — Issue 089's strata currently
measure only our own repos, which is a circular population; (3) **indirect marketing** —
the PR carries one honest disclosure line linking to refine.riir.ai, and the dashboard
carries the metrics. End-state model: Snyk's automatic fix-PR product (cited-only); the
starter phase is a **fork-based pipeline on repos we control** (a seeded-warning oracle
repo), then a curated owner-gated cold list, then a maintainer opt-in hook
(`cargo-refine.yml`, the dependabot/renovate convention). This is our own design over
our own measured machinery; the closest external patterns (Snyk auto-fix PRs,
Dependabot/Renovate opt-in config files) are cited, not distilled. **Verdict from the
round-1 review (AGREE): the marketing payload must NOT ride the PR body** — burn stats
and perf/sec figures in a cold PR to a stranger's repo read as spam which is the single most likely cause of the account getting flagged. **Owner amendment 2026-10-08: the AI-slop prior does NOT map onto this tool** — the healer is a deterministic corpus with compile-gated, span-preserving edits (the `cargo clippy --fix` PR category, which has merge precedent), not generation; what maintainers reject is unreviewable VOLUME, not tooling. The residue that stays load-bearing is behavioral: velocity (the rate ceiling), MSRV breaks (the GATE), and reformat noise (no-fmt law).**

## The problem this solves

The healer has production-grade evidence from exactly one population: **our own repos**
(score-bench heal 84.6% → 96.4%, created-warnings 5.8% → 1.8%, 162 runs — Proposal 016).
That population is circular: our repos are healed by our corpus, our corpus is mined
from sources we chose. There is no external adoption surface at all — `cargo-refine`
ships binary-only (Proposal 003) with an install funnel (Proposal 013) but zero
third-party proof. Meanwhile refine.riir.ai markets on volume metrics only. An outbound
campaign produces the missing artifact class: **a merged PR in someone else's repo**,
which is (a) an honest public demo, (b) a provenance stratum from code we did not
choose, and (c) a mining feed — every external fix run exercises the corpus against
unfamiliar code and surfaces misses into the post-mining intake
(`.distill/001_mining_queue_snapshot.md` §"Post-mining intake"). Without this channel,
external validation waits on organic adoption that nothing currently drives.

## The proposed design

New opt-in feature **`harvest`** (ops lane; NEVER joins the release feature set — the
`release_set_matches_workflow` test pins the release set and this must stay outside
it). One new CLI face, two commands:

```
cargo refine --harvest [PATHS|--seed harvest_seeds.toml]      # dry-run: scan + report, zero writes off-box
cargo refine --harvest-send                                    # owner-gated: execute the send queue
```

The pipeline per candidate repo (LLM-free, deterministic):

```
SELECT   curated seed list (TOML: repo, license class, notes)  — never a crawler in Phase 0/1
CLONE    shallow clone → .raw/harvest/<repo>  (ephemeral; removed after send — the .raw convention)
SCAN     auto-oracle (Issue 113) at the repo's own posture; MSRV detect:
         rust-version field > rust-toolchain.toml > clippy.toml msrv > ABSENT ⇒ refuse (conservative)
GATE     per-rule eligibility, ALL of:
           rule.min_rust ≤ target MSRV          (manual_is_multiple_of ≥1.87, let-chain
                                                 collapsible_if needs edition 2024 — a fix
                                                 that compiles on our 1.98.1 can break a crate
                                                 declaring rust-version = 1.70)
           target edition resolved; lint NOT in their [lints] table, #![allow] list,
             or clippy.toml disallowed set        ("fixing" a deliberate allow is noise)
           findings ≥ threshold (default 3 same-rule sites)
HEAL     --fix --repo-posture semantics: edits only where THEIR clippy flagged; span-preserving;
         NEVER cargo fmt over the target (whole-file reformat = instant close; cap per-PR
         diff size; one lint family per PR)
VERIFY   compile-gate at THEIR toolchain (rustup run <msrv> if installed) + created-warnings == 0
           (the local bar is 1.8% — the external bar is ZERO) + Snyk's severity-gate shape:
           ship only if kept-edits fix more than they touch
SEND     fork under our org/bot account → branch `refine/<rule>` → push → wait THEIR CI
         (wall-clock cap; no-CI repos skip the wait) → open PR:
           title: "fix: <rule> — <n> sites"
           body:  what/why/how-verified + disclosure line + [perf line if the rule is
                  bench-measured: the rule's measured delta + box provenance] + repro:
                  "Generated by [cargo refine](https://refine.riir.ai) — deterministic
                   clippy/perf fixes (compile-gated, no LLM). Repro:
                   `cargo install cargo-refine && cargo refine src/`. Happy to rework
                   or close." + logo
           ACCOUNT RULE (owner 2026-10-08): each send owner-approved via the queue →
           the owner's account is the honest identity; an UNATTENDED daily cadence → a
           LABELED bot account (GitHub automation-disclosure norm — an unlabeled bot-
           cadence human account is the deceptive variant, not the fixes).
           NO aggregate burn stats. The perf line is per-fix, rule-annotated only.
TELEMETRY provenance strata row per fix: (repo-id, rule, before/after AST shape CLASS,
         target MSRV, edition, outcome). Outcome poller: merged / closed / no-response →
         the campaign-level GOAT metric is ACCEPT RATE. Derived data ONLY — never
         verbatim target code in any store that ships inside the distributed binary
         (Proposal 003: the corpus + self-evolve memory ride the binary; verbatim GPL
         snippets in them are a contamination risk).
```

Rate ceiling (load-bearing, not etiquette): **≤3 PRs/day**, one PR per repo ever,
never re-PR after a close, honor `.github/cargo-refine-ignore`, detect CLA/DCO
requirements and **skip** those repos (never auto-sign). A repo block = permanent
blacklist. The send queue is a file the owner reviews; `--harvest-send` executes it.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **The spam cliff is real and close.** GitHub flags accounts that open PRs too fast;
   maintainers publicly complain about AI-generated PR floods; repos block bot accounts
   (References 3–6). One bad week of wording or rate can get the account suspended and
   the channel dead. The rate ceiling, one-PR-per-repo, and opt-out honoring are the
   product, not manners. **If the accept rate drops after any wording change, revert
   the wording** — the PR body is an experiment surface with a fast feedback loop.
2. **MSRV/edition divergence is the correctness cliff.** Our toolchain is 1.98.1;
   several corpus rules assume recent Rust (`manual_is_multiple_of` ≥1.87; `let`-chain
   `collapsible_if` forms need edition 2024). A fix that compiles here can fail the
   maintainer's declared floor. Undetected MSRV (no `rust-version`, no toolchain file)
   ⇒ refuse rather than guess. This gate is mandatory before ANY send.
3. **Licensing: derived data only.** Fix diffs are ephemeral and go back to their
   authors; but the persistent strata stores ship inside the distributed binary —
   verbatim external code (GPL-family copyleft) must never be persisted there. AST
   shape classes, rule ids, MSRV/edition, and outcomes are the store's entire schema.
4. **Marketing posture — owner-decided 2026-10-08, with one mechanical discipline:**
   perf numbers ride a PR ONLY when the fix rule is bench-measured in the corpus — the
   existing **NotBenchable annotation decides, mechanically, per rule** (no per-PR
   judgement calls, no unproven claims ever). A bench-measured PR quotes the rule's own
   delta with box provenance (`<bench>/<n> 259.5→66.5 µs (×0.26), measured by the
   tool's bench suite on M3 Max` — the G2 box-state law); a NotBenchable PR carries the
   disclosure line only. Attribution = tool name + refine.riir.ai + logo, nothing else.
   The proof-by-tool surface is the repro command:
   `cargo install cargo-refine && cargo refine src/` — the maintainer verifies it
   themselves; a runnable command beats any claim. The dashboard still carries the
   aggregate burn/perf story (funnel card, self-evolve card).
5. **CI-wait is constrained by the Actions free tier.** External repos run THEIR CI at
   their cost (no minutes of ours); the wait is wall-clock and capped. Phase 0's CI leg
   must run on our own gist-rs repos or locally until the Actions budget resolves.
6. **Accept rate can legitimately be near-zero for cold outreach** and that is a
   measurement, not a failure: the strata will show which rule classes maintainers
   reject, which feeds corpus `context_notes`. Phase 2 (opt-in) is where Snyk-grade
   accept rates live; do not judge the channel on Phase 1 alone.

## Fusion lineage

- **Proposal 003 (binary-only distribution)** — why the strata store must hold derived
  data only: the corpus and self-evolve memory ship inside the distributed binary;
  contamination there is irreversible.
- **Proposal 013 (consumer install tiers) + Proposal 016 (public self-evolve reveal)**
  — the dashboard is where burn/perf metrics legitimately live; Phase 2's opt-in hook
  (`cargo-refine.yml`) rides the tier language (Lite/Pro users enable the hook).
- **riir-dapps Proposal 004 (fix-stats funnel card)** — the marketing surface already
  exists; the campaign feeds it a new evidence class (external merges).
- **`.distill/001_mining_queue_snapshot.md`** — the inverse direction. Same source
  graph, both lanes: inbound mining pulls rules from upstream commits; outbound harvest
  pushes fixes to upstream repos. Each lane feeds the other (harvest misses → mining
  intake; mined rules → harvest corpus).

## GOAT gate

- **G1 (correctness, per-PR):** created-warnings == 0; compile green at the target's
  declared MSRV + edition; their CI green (or repo verified no-CI); diff contains only
  healer spans (no reformat, no drive-by edits). Any failure ⇒ the PR is not opened.
- **G2 (campaign metrics):** the honest GOAT is **accept rate** (merged ÷ (merged +
  closed), sliding 90-day window). Phase 1 continuation floor: **≥50%** on the curated
  list; below floor ⇒ stop sends, read the closed-unmerged strata by rule class, annotate
  or demote the rejected classes before any restart. Created-warning rate floor: 0
  (a single created-warning escape on an external repo is a stop-the-line event).
- **G3 (no regression):** the `harvest` feature never joins the release set
  (`release_set_matches_workflow` pins it); local repos and the default heal path are
  untouched; the harvester adds zero code to the `heal()` hot path.
- **G4 (alloc):** not applicable to the ops lane (no hot path); the core heal path is
  unchanged by construction.

## What ships now (riir-refine) vs deferred

### Ships now — Phase 0 (dogfood, zero external exposure)
- `harvest` feature (opt-in, not in release set; `ureq` is already an optional direct
  dep — the feature only enables it, zero new deps).
- SELECT/CLONE/SCAN/GATE/HEAL/VERIFY/TELEMETRY implemented; SEND implemented but
  pointed at **our own gist-rs org only**.
- Oracle repo: `gist-rs/refine-harvest-seed` — a repo with SEEDED mechanical warnings
  and a **pinned expected fix set** (the pipeline's golden: "it opened a PR" is not a
  verdict; the merged diff must equal the pin). Full loop exercised: fork → heal →
  verify → PR → merge our own PR, CI leg on our own repos (free tier).

### Deferred — Phase 1 (curated cold outreach, owner-gated)
- 10–20 hand-picked repos (permissive license, active, no CLA), `--harvest-send` with
  the rate ceiling, weekly outcome review, accept-rate tracking.

### Deferred — Phase 2 (opt-in hook, Snyk-model)
- `.github/cargo-refine.yml` repo config (the dependabot/renovate convention) + a
  README badge; maintainer-enabled repos get priority + richer PR bodies (still no
  unsolicited burn/perf). Hosted webhook/service surface only after Phase 1 evidence.

### Explicitly NOT shipped by this proposal
- No crawler/discovery (candidate selection is always curated).
- No CLA/DCO auto-signing; no merge automation; no issue-filing on external repos.
- No non-Rust harvesting (docker/sec/dist domains stay inbound-only).
- No KAT reward wiring for outbound fixes (an outbound fix CONSUMES the corpus; whether
  it earns receipts is Proposal 015/013 territory, owner-gated, deliberately out of scope).
- No marketing payload beyond the one disclosure line, at any phase.

## Phased rollout (sketch — a plan would expand this)

### Phase 0 — dogfood the pipeline (this quarter)
- [ ] T1.1 `harvest` feature + `--harvest` scan/report over a local path
- [ ] T1.2 MSRV/edition detection + per-rule eligibility gate + unit tests
- [ ] T1.3 fork→branch→push→PR over the GitHub API (ureq); strata row schema (derived data only)
- [ ] T1.4 the seed repo `gist-rs/refine-harvest-seed` with seeded warnings + pinned expected fix set
- [ ] T1.5 full end-to-end on the seed repo; golden = merged diff equals the pin
- [ ] T1.6 `--harvest-send` queue file + rate ceiling + `--harvest` report prints the queue
- [ ] T1.7 perf-line formatter: reads the rule's bench annotation (NotBenchable ⇒ no
      number, mechanically) and renders the measured delta + box-provenance line

### Phase 1 — curated cold outreach (owner-gated)
- [ ] T2.1 seed list of 10–20 repos; CLA/DCO detection; opt-out file handling
- [ ] T2.2 outcome poller + accept-rate stratum; weekly review ritual
- [ ] T2.3 post-mining intake wiring (external misses → `.distill` queue)

### Phase 2 — opt-in hook
- [ ] T3.1 `.github/cargo-refine.yml` schema + badge; PR-body enrichment for opted-in repos
- [ ] T3.2 hosted enable surface (dashboard button) — only after Phase 1 accept-rate evidence

## Risks

1. **Reputation (correctness of the CHANNEL):** a single sloppy PR (reformat, created
   warning, MSRV break) can get the account blocked workspace-wide. Mitigation: G1
   per-PR gate, one-PR-per-repo, blacklist on block.
2. **Legal (licensing):** copyleft contamination via persistent stores. Mitigation:
   derived-data-only schema, ephemeral clones, `binary_leak_scan.sh` posture extended
   to the strata store.
3. **Architectural:** none — the harvester is an ops lane beside the healer, zero
   coupling into `HealPipeline` beyond the existing `--fix` face.
4. **Perf:** none claimed; any perf statement about external repos waits for measured
   bench deltas per fix class.

## Out of scope

Auto-merge, dependency-upgrade PRs (Dependabot/Renovate's job, not ours — we ship
code fixes, not version bumps), security-advisory PRs (sec domain stays suggest-only,
Issue 151), multi-repo fan-out blasts, and any hosted SaaS surface in Phase 0/1.

## References

1. Snyk docs — enable automatic fix PRs (severity-gated auto-PR model). Cited-only. <https://docs.snyk.io>
2. Wessel et al. 2022, "Bots for Pull Requests: The Good, the Bad, and the Promising" — PR-bot accept/reject dynamics. Cited-only.
3. arXiv:2103.03591, "Bots Don't Mind Waiting" — ~⅓ of GitHub PRs come from bots; merge-rate differentials. Cited-only.
4. GitHub community discussions on PR-spam floods + accounts flagged for PR velocity. Cited-only. <https://github.com/orgs/community/discussions/185387>
5. Dependabot/Renovate config-file opt-in convention (`dependabot.yml`/`renovate.json`) — the Phase 2 hook's shape. Cited-only.

## TL;DR

**Ship Phase 0** (dogfood pipeline on our own seeded repo, golden-pinned), gate Phase 1
on owner review of the first 10 curated sends, gate Phase 2 on a ≥50% Phase-1 accept
rate. The PR body carries the fix and one disclosure line — nothing else. Next action:
open the Phase 0 plan (riir-refine `.plans/.highwater` 212 → 213) when the owner
confirms the bot account identity and the marketing-posture verdict above.
