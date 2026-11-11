---
name: outbound-pr
description: The gated lane for sending cargo refine's fixes to external repositories as upstream pull requests (fork → fix → prove → verdict → fingerprint send). Activate when the user says outbound PR, fix-PR campaign, send fixes upstream, harvest-and-send, or names an external repo to PR fixes into; when preparing, gating, or reviewing an upstream fix-PR; when a maintainer replies to one of our PRs; or when asked whether a fix is PR-worthy. Enforces the five send gates (G-PR0 rulewright provenance, G-PR1 GOAT proof, G-PR2 Claude AGREE, G-PR3 SVG chart, G-PR4 owner fingerprint), the attribution ban, the human-voice reply rules, the rate ceiling, and the bench-then-PR order. Not for inbound mining (use the distill skill) or local fixing (use the cargo-refine skill).
---

# outbound-pr — the gated fix-PR lane (cargo refine → upstream)

Every upstream PR is the campaign's evidence. A PR that doesn't evidence the
rulewright is worse than no PR (the #2977 lesson, three owner corrections in one
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

**G-PR0 provenance → G-PR1 proof → G-PR1b blast-radius harness → G-PR2 Claude
AGREE → G-PR3 SVG → G-PR4 fingerprint.** (Six checkpoints, five gates —
G-PR1b hangs off G-PR1: local proof alone never cleared a PR.)

### G-PR0 — rulewright provenance (the fix must BE a cargo refine output)

- The PR must originate from the rulewright's own scan of the target repo:
  `cargo refine --suggest <clone>` (dry run), oracle-anchored findings
  (`--clippy-json` with THEIR clippy run), or `--fix` output. Any domain is
  eligible — clippy_lints, rust_perf, rustc_errors, docker, sec, whatever the
  corpus carries. **perf/sec is the priority order, not exclusivity**: a
  rust_perf finding takes the next send slot over a clippy one; a
  rulewright-fixed clippy finding beats a hand-written perf feature.
- A fix the rulewright could not find (maintainer-issue-driven, hand-derived) is
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
- **Pin-staleness law (the 21-commit catch):** before ANY verification or
  bench reuses an earlier pin, COUNT the drift — `git rev-list --count
  <pin>..origin/main` — never eyeball "a couple commits" (the readiness
  verdict caught "two commits" that were 21, several bench-load-bearing:
  resolver swap, storage-creation refactor, jobserver passthrough). Name
  the classes the drift commits touch; if any touches the bench's inputs
  (the measured path, its client, the resolver, the dep graph), the bench
  RE-RUNS at the new pin with a FRESH pristine-main baseline — "same 3
  failures" is re-proven, never assumed (their test-suite rewrites move
  the pre-existing failure set under you).
- **Verified-real zero law:** a "0 warnings / 0 failures" claim carries its
  completion marker — a `build-finished` line with `success:true` (or the
  tool's equivalent), a non-empty capture, and stderr collected (never
  `/dev/null`). An invalid command shape that dies before compiling reads
  as a green zero (the live catch: `-A` lints placed before `--` made cargo
  refuse the whole invocation; the empty JSON read as "clean"). Per-lint
  classification beats count-matching: name WHICH lints fire and whether
  each is in their posture (their CI's allowlist + their MSRV's lint set),
  not just how many.
- NotBenchable domains (sec, correctness, docker): the domain's own oracle
  is the proof — compile green, created-warnings ZERO (the external bar;
  the local 1.8% bar does not transfer), their test suite green. No
  invented numbers, no percentages you did not measure.
- MSRV/edition gate BEFORE any fix: `rust-version` field > rust-toolchain >
  clippy.toml msrv > ABSENT ⇒ refuse (never guess). A fix that compiles on
  our toolchain but breaks their declared floor is the campaign killer.

### G-PR1b — the blast-radius harness (run BEFORE G-PR2, after the fix compiles)

Our own tests passing is NOT the gate — the gate is "their CI's known surfaces,
reproduced locally." The #2977 round-4 failure (`--debug-preprocessor-cache`
parsing our CACHEDIR.TAG) passed every local gate we ran because the failing
test was in a suite we never invoked (`--test system`). The harness, in order:

1. **Their CI, locally.** Read their workflow files (`.github/workflows/*`,
   `.circleci/config.yml`, …) and enumerate EVERY test/clippy/build invocation.
   Run each one locally at their toolchain that can run on this box — not just
   `--lib`. System/integration test suites are MANDATORY for any change touching
   files that tests read, write, or walk (`cargo test -p <pkg> --test system` was
   the miss; it takes minutes, not hours).
2. **Blast-radius grep (the walk/file rule).** For every file or directory your
   change WRITES, CREATES, or MODERATES the contents of, grep the target repo
   for every other reader/writer/walker of that same path: `grep -rn
   '<dir-name>' src/ --include='*.rs'`, walk scanners (`WalkDir`, `read_dir`,
   `glob`), format parsers keyed to files in that dir. Our miss: THREE readers
   of the cache tree (LRU init, preprocessor scan, debug dump) — we gated two,
   the third lived in a different crate file. Any hit you did not update is a
   finding: either fix it or write down why it is unaffected.
   **Seam consumers (the multilevel-storage lesson, M3):** tree-walk greps do
   not find CALL-GRAPH edges. Every wrapper, composite, or dispatch layer that
   routes through a seam your diff modifies is a reader — the gate sat at
   `storage.get` and the outer blast radius was `MultiLevelStorage`'s read
   loop (healthy local-disk tiers get skipped by a gate meant for the remote
   tier). The harness's seam-grep (step 3b) extracts modified `fn` names and
   greps for call sites outside the diff; adjudicate every hit — per-level
   handling in the diff, or a body disclosure + a test naming the composite
   path.
   **Their docs are a reader (M2, the FC10 class):** a new env knob, flag, or
   config surface lands with its docs row in THEIR docs file in the SAME PR
   (sccache's `docs/Configuration.md` documents the sibling knobs; ours were
   missing — a near-certain review round). The harness's knob-grep (step 3c)
   extracts `env::var`/`var_os` names from added lines and fails them against
   `docs/`.
3. **The OS matrix spot-check.** If their CI runs freebsd/windows/mac beyond
   linux, the failure class is usually OS-conditional (case sensitivity,
   path separators, entry ORDER). You cannot run their OSes — so after the fix,
   re-read your diff asking only: "what does this do differently when
   `file_name()` ordering, separators, or case change?" One minute of reading;
   it catches the freebsd-only class.
4. **First-CI-wait discipline.** After the push, poll their CI runs once;
   if any job fails, pull `--log-failed` and diagnose IMMEDIATELY (a red CI we
   did not predict is a finding about our gates, not noise to wait out).
   Maintain a per-repo `ci_surfaces.md` note in the campaign record listing
   every invocation their CI runs, so the next PR to the same repo starts from
   the map instead of re-deriving it.
   **Fork-CI blindness (FC12):** first-contribution CI is maintainer-triggered
   (`action_required`) — expect ZERO runs for days; the local harness is the
   only proxy. When runs appear, EXPECT main's own pre-existing reds to ride
   along (a repo's own broken tests land on main while your PR sits — run
   41806194289 showed 13 red jobs, all ONE upstream test broken by their own
   dep-info commit, fixed upstream via #2989→revert→#2990 while we waited).
   Classify every red by grepping its log for YOUR artifact names (FC8) —
   never by job name alone.
   **Coverage/CI bots are part of their CI surface (the Codecov class).**
   The first response on a PR is usually a coverage bot, not a maintainer —
   record the bot in the per-repo `ci_surfaces.md` note, classify the
   comment by `author_association` (NONE + bot login = telemetry, never
   reply), and expect the fork-PR artifact: the PR head uploads coverage
   from fewer CI jobs than main, so the bot's project-coverage delta reads
   as a large fake regression (#3025: "−6.77%" driven by "HEAD has 9
   uploads less than BASE" — 5 vs 14). The actionable number is patch
   coverage on the added lines (#3025: 88.62%, 90 missing — mostly the new
   module's logging/error arms); act on it only if a maintainer raises it.
   **And the fork itself runs their push-event workflows** — secret-
   dependent steps fail there (no CODECOV_TOKEN on the fork), and those
   runs never appear on the PR's checks panel. After the branch push,
   disable every fork workflow + cancel queued runs (`gh api -X PUT
   repos/<org>/<repo>/actions/workflows/<name>/disable`), and note it in
   the send record (#3025: both fork workflows `disabled_manually`, 4 runs
   terminal).

G-PR1b failure = the fix is NOT ready, regardless of G-PR1's local green.

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
  inconsistent lock) by checking the registry manifests itself. The HARD
  pass (owner-directed, post-#2977) goes further — it attacks the frozen
  artifact against the post-mortem classes AND the live upstream state, and
  found five classes no local gate sees: **claimants** (M1: someone else
  claimed the issue in a comment; the body acknowledges + offers to fold,
  or takes the design-comment-first lane — opening over an open claim
  unacknowledged reads as sniping), **their docs** (M2), **composite-layer
  consumers** (M3), **workload-shape framing** (M4: say "sequential" when
  the bench is sequential and the issue's repro is parallel — run their
  shipped repro script if one exists, or state the parallel workload was
  not measured), **issue-requirements sweep** (M5: every requirement the
  issue states gets a body answer or an explicit "not addressed"). All
  five are pre-verdict self-checks now.
- **Ledger status is ruled per send, never assumed:** an issue-driven or
  hand-written fix is goodwill-class (G-PR0) — it enters the accept-rate
  denominator ONLY with an explicit owner exception, else it ships as
  goodwill outside it. State the class in the send record; the
  one-PR-per-repo exception is also an explicit owner ruling (the
  maintainer-burden rationale applies to goodwill PRs equally).

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
3. **Re-verify at THEIR toolchain (tests, clippy at their CI flags, fmt) and
   run the CI harness (`scripts/outbound_pr_ci_harness.sh`) — THEN request the
   G-PR2 verdict on the response commit + reply draft.**
4. Root-cause claims get VERIFIED before they are written: registry
   manifests, `git log -S`, their CI runs on main. One wrong diagnosis in a
   reply burns the account's credibility (the round-2 lockfile failure).
   ⛔ The round-4 corollary: before replying "that failure reproduces on
   main, not mine", pull the failing job's OWN log and grep it for your
   artifact names (file names your change writes, your crate names). The
   round-3 reply blamed main while the freebsd log contained the literal
   string `preprocessor/CACHEDIR.TAG` — the evidence was in the log we
   already had, unread.
5. Reply in short, plain sentences — human voice: no exclamation openers, no
   "Fixed all four:" ledgers, no three-paragraph structure on a small fix.
   Name the commit sha, use their vocabulary. If main's CI is broken,
   say so plainly with the reproducing fact, one sentence, and leave their
   bug alone unless asked.
6. Patch the existing comment (PATCH API) rather than posting duplicates.

## The pre-push harness script (mechanized G-PR1b — `scripts/outbound_pr_ci_harness.sh`)

The script IS the gate's executor — run it before every push (new PR or
review-response); the judgment gates (G-PR2) come after it, never before:

```
DIFF_WRITTEN_PATHS="CACHEDIR.TAG" BASE_REF=origin/main \
  sh scripts/outbound_pr_ci_harness.sh <clone-dir> <their-msrv>
```

What it does (exit contract: 0 = READY, 1 = NOT READY (push waits), 2 = BLIND
(never silently green)):
1. **CI inventory** — parses their workflow files (`cargo test/clippy/build/
   check/fmt` lines), strips CI-template variables (`\${{ }}`), drops commands
   mangled beyond running, and SKIP-classifies the not-locally-runnable ones
   (matrix/cross-target/all-features/windows/combinatorial) as named skips for
   manual review.
2. **Runs the runnable set** at THEIR toolchain, collecting failures (never
   fast-exit — the report is the whole surface, not the first break).
3. **Blast-radius grep** — for every CREATED file name (`DIFF_WRITTEN_PATHS`),
   greps `src/` for OTHER code referencing it (excluding the diff's own
   files); each hit prints file:line. This is the #2977 round-4 catch: on the
   pre-fix tree the `commands.rs` debug-walk reference would have made the
   harness NOT READY and stopped the push. **Seam-grep (3b):** modified `fn`
   names from the diff are extracted and grepped for call sites outside the
   diff's files — the composite/wrapper consumers (the multilevel-storage
   M3 class) that no tree-walk finds. **Knob-grep (3c):** `env::var`/`var_os`
   names in added lines are checked against their `docs/` — an undocumented
   knob fails the harness (M2).

Self-test protocol (every harness change re-proves both directions on a live
clone): apply the campaign's durable patch to a scratch branch — the
pre-fix tree must go NOT READY on a real class (the seam-grep fired on the
`get_cached_or_compile` call-site web and the knob-grep flagged both
`SCCACHE_BACKEND_BACKOFF_*` knobs UNDOCUMENTED on the live #2968 fixture),
the pristine baseline is adjudicated (pre-existing upstream reds recorded
first: the env-dependent unix-socket lib pair + the env-dep test pair + the
sccache_cargo integration suite fail identically on pristine main), then
restore pristine. Never trust a harness change that only ever ran on a
clean tree.

Known limits (never hidden): OS-gated surfaces (freebsd/windows) are
SKIP-classified — the OS-conditional read-through in G-PR1b step 3 covers
them by hand; workflow parsing only sees what their files literally spell.
After the push, poll their CI once; an unpredicted red job is a finding about
OUR gates — pull `--log-failed` and diagnose immediately. Keep a per-repo
`ci_surfaces.md` note in the campaign record so the next PR to the same repo
starts from the map.
**Pre-existing main reds are the expected shape, not a surprise (FC12):**
adjudicate the pristine-main baseline FIRST (same command on a clean
checkout, failures named), then read the diff-tree run as a DELTA — a red
that fails identically on pristine main is theirs, a red that appears only
under the diff is ours. The 11-07 sccache adjudication: pristine main fails
the same unix-socket lib pair + env-dep pair + the sccache_cargo suite that
the harness flags — recorded as the baseline, never as a regression.

State to encode from the #2977 rounds: the miss was `--test system` (a suite
we never ran), the freebsd class was OS-conditional entry parsing, and the
lockfile class needs `--locked` on the TARGET toolchain, not ours. From the
2026-11-07 round: the green-zero class (an invalid command shape dies before
compiling and an empty capture reads as clean — completion marker + stderr
capture or the claim is unproven), and the staleness class (count commits
behind, never eyeball; the "two commits" that were 21 included the resolver
swap + storage refactor + jobserver passthrough, all bench-load-bearing).

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
