---
name: outbound-pr
description: The gated lane for sending cargo refine's fixes to external repositories as upstream pull requests (fork → heal → prove → verdict → fingerprint send). Activate when the user says outbound PR, fix-PR campaign, send fixes upstream, harvest-and-send, or names an external repo to PR fixes into; when preparing, gating, or reviewing an upstream fix-PR; when a maintainer replies to one of our PRs; or when asked whether a fix is PR-worthy. Enforces the five send gates (G-PR0 healer provenance, G-PR1 GOAT proof, G-PR2 Claude AGREE, G-PR3 SVG chart, G-PR4 owner fingerprint), the attribution ban, the human-voice reply rules, the rate ceiling, and the bench-then-PR order. Not for inbound mining (use the distill skill) or local healing (use the cargo-refine skill).
---

# outbound-pr — the gated fix-PR lane (cargo refine → upstream)

Every upstream PR is the campaign's evidence. A PR that doesn't evidence the
healer is worse than no PR (the #2977 lesson, three owner corrections in one
day). Doctrine home: `../../.proposals/022_outbound_fix_pr_campaign.md` (the
binding gates + history); the worked example with every failure recorded:
`../../.benchmarks/127_mozilla_wetrun/README.md`. When this skill and those
records disagree, the records win — read them at activation.

## When to use

- The user names an external repo to send fixes to, or says "outbound PR",
  "fix-PR campaign", "harvest-and-send", "PR the fixes upstream".
- A maintainer replied to one of our open PRs (the review-response flow).
- The user asks whether a finding is PR-worthy, or asks to check the gates.

## The five gates — ALL required, in order, fails-closed

Run them in order. A gate that fails means NO send; that is the gate working.

### G-PR0 — healer provenance (the fix must BE a cargo refine output)

- The PR must originate from the healer's own scan of the target repo:
  `cargo refine --suggest <clone>` (dry run), oracle-anchored findings
  (`--clippy-json` with THEIR clippy run), or `--fix` output. Any domain is
  eligible — clippy_lints, rust_perf, rustc_errors, docker, sec, whatever the
  corpus carries. **perf/sec is the priority order, not exclusivity**: a
  rust_perf finding takes the next send slot over a clippy one; a
  healer-fixed clippy finding beats a hand-written perf feature.
- A fix the healer could not find (maintainer-issue-driven, hand-derived) is
  GOODWILL: it may still be sent if the owner explicitly approves, but it
  sits OUTSIDE the campaign ledger and never counts as evidence. Do not
  invent a "pivot" to justify one (that was the #2977 error).
- Scan at THEIR posture first (their CI clippy flags, their lint tables,
  their `[lints]`/`#![allow]`s — "fixing" a deliberate allow is noise).
  Clean tree at their posture = honest outcome "nothing to send", a PASS.

### G-PR1 — GOAT proof (benchable domains bench; others verify by oracle)

- Benchable domains (rust_perf, anything latency/alloc-shaped): before/after
  benched AT the target repo, THEIR toolchain (rustup the MSRV if missing),
  a pinned reproducible harness committed to OUR bench record, box-state
  provenance beside every number (free RAM, 1-min load, AC/battery, host).
  Bench-then-PR, in that order — never PR a number you have not measured.
- NotBenchable domains (sec, correctness, docker): the domain's own oracle
  is the proof — compile green, created-warnings ZERO (the external bar;
  the local 1.8% bar does not transfer), their test suite green. No
  invented numbers, no percentages you did not measure.
- MSRV/edition gate BEFORE any fix: `rust-version` field > rust-toolchain >
  clippy.toml msrv > ABSENT ⇒ refuse (never guess). A fix that compiles on
  our toolchain but breaks their declared floor is the campaign killer.

### G-PR2 — Claude verdict AGREE (no send without it)

- Before ANY push (new PR or review-response commit), send the full artifact
  (verbatim body + verbatim diff + the bench numbers + the reply draft) to
  the delegated-owner verdict. Claude judges: (a) slop quality — ZERO
  invented facts (the canonical failure: #2977's "rsync with an exclude
  plugin" — a confident tool claim the spec's own page contradicts; check
  every factual assertion against the primary source BEFORE the verdict),
  plain fix voice; (b) the gain/proof claim vs the actual bench.
- REVISE loops back to fix-and-re-verdict. AGREE unlocks G-PR3.
- Ask it to re-derive, not just review: the round-2 verdict caught a wrong
  root-cause diagnosis (the "unpinned dep" that was actually a hand-merged
  inconsistent lock) by checking the registry manifests itself.

### G-PR3 — SVG proof (benchable domains)

- The PR body carries the before/after chart (the Plan 224 shape: bars +
  per-row speedup, box provenance in the caption, our bench record
  referenced). No chart → no PR. NotBenchable domains are exempt — their
  oracle output is quoted in the body instead.

### G-PR4 — owner fingerprint send

- The send identity is released by the owner's Touch ID, PER send:
  `kat-secret exec gh-outbound-pr --env GH_TOKEN --why "send <repo> PR" -- gh pr create …`
  (the touch-deploy pattern; the token is owner-sealed in their own terminal,
  never visible to the agent). The agent never pushes upstream on its own
  authority. No fingerprint → no send.

## The PR body — what rides and what never does

Rides: the fix, the verification story, the SVG (G-PR3), `Fixes #N` when it
closes their issue, "Happy to rework/adjust." as the closer.

⛔ **NEVER**: an attribution/disclosure/"Generated by cargo refine" block,
tool name, site link, logo, repro-advertising line, aggregate burn stats,
unearned perf numbers. Owner ban 2026-11-02: it reads as AI slop +
self-promo; the public-goods posture keeps self-promotion out of upstream
work until the grant lands and the owner explicitly allows it.

⛔ **Slop tells to self-check before G-PR2**: structured What/Why/How headers
on a small PR (plain paragraphs instead), em-dash density, polite-formula
closers ("or close" = defeatism), tool lists with entries you did not verify
against the primary source, claims about THEIR code you did not grep (the
#2977 round-2 failure: "logs a warning" while the diff called `error!` —
body and diff must agree), "tens of GB"-class rounding where their config
has an exact default.

## Review-response flow (a maintainer replied)

1. Read every inline comment + the issue comment + CI state before touching
   anything. Respect the review: their reading of their codebase outranks
   ours, always (the preprocessor-cache catch beat our root-path "fix").
2. Fix what they asked, nothing else. Shorten comments when asked. Drop
   redundant tests when asked. Their preference between two correct shapes
   wins even when ours is prettier.
3. Re-verify at THEIR toolchain (tests, clippy at their CI flags, fmt) and
   ONLY THEN request the G-PR2 verdict on the response commit + reply draft.
4. Root-cause claims get VERIFIED before they are written: registry
   manifests, `git log -S`, their CI runs on main. One wrong diagnosis in a
   reply burns the account's credibility (the round-2 lockfile failure).
5. Reply in short, plain sentences — human voice: no exclamation openers, no
   "Fixed all four:" ledgers, no three-paragraph structure on a small fix.
   Name the commit sha, use their vocabulary. If main's CI is broken,
   say so plainly with the reproducing fact, one sentence, and leave their
   bug alone unless asked.
6. Patch the existing comment (PATCH API) rather than posting duplicates.

## Standing constraints

- Rate ceiling: ≤3 PRs/day, ONE PR per repo ever, never re-PR after a close.
  A repo block = permanent blacklist. Never CLA/DCO auto-sign — skip those
  repos.
- Fork under the owner's org (`riir-ai`); branch naming `refine/<topic>`.
- No crawler/discovery, no issue-filing on external repos, no merge
  automation, no dependency-bump PRs, no security-advisory PRs (sec stays
  suggest-only on the outbound lane).
- Clones are ephemeral: `.raw/<campaign>/<repo>`, removed at close-out
  (T8), pins + bench artifacts carried in OUR records first.
- Copyleft safety: only derived data (rule ids, shape classes, outcomes)
  persists in any store that ships; verbatim external code never does.
- After the send: outcome poller (merged/closed → the accept-rate ledger),
  misses → the post-mining intake (`.distill/001` §Post-mining intake).

## Campaign-record discipline

Every campaign writes: a plan (`.plans/NNN_*.md`, claim-leased), a bench
record (`.benchmarks/NNN_*/README.md`: scan-before inventory, gates table,
bench after, campaign ledger, lessons), and the send-queue file the owner
reviews. Records land BEFORE the send so the send is auditable; the ledger
updates on merge/close.
