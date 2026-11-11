# mozilla/sccache — CI surface map (for the next PR to this repo)

Per outbound-pr G-PR1b #4: the per-repo CI map so the next PR starts from the map, not a re-derivation. Derived 2026-11-04 from `.github/workflows/ci.yml` at pin `ba261b8` + the bench-128 G-PR1b run. Re-verified 2026-11-10 at pin `33d12fb` (clippy line ci.yml:19 byte-identical; the drift touched only the codspeed/grcov lanes).

## Their CI invocations (what ci.yml spells)

| invocation | local-runnable? | notes |
|---|---|---|
| `cargo clippy --locked --all-targets -- -D warnings -A unknown-lints -A clippy::type_complexity -A clippy::new-without-default` (ci.yml:19, via a `cargo_cmd:` template) | YES at +1.91.0 (their MSRV = `rust-version`) | the harness regex CANNOT see the template — run by hand, always |
| `cargo fmt -- --check` | YES | whole package |
| `cargo test --locked --lib --bins --tests` (+ per-crate splits) | YES at +1.91.0 | on this macOS box the known-failing set at 33d12fb is: lib unix-socket binding pair, the `cache_hit_rate` integration suite (2), and the `sccache_cargo` suite (8) — ALL fail identically on the pristine pin (re-proven 2026-11-10 at 33d12fb; the old rust hash-env lib failure was fixed upstream by #2990 and no longer appears) |
| `cargo check --no-default-features` (feature-matrix lane) | YES | 6 pre-existing warnings, none diff-touching (re-verified at 33d12fb) |
| `cargo build -p randomize_readdir` (integration helper) | YES | parses their CI matrix — the `outbound_pr_ci_harness.sh` inventory catches this one |
| `cargo test --test system` (system tests, CI = Linux) | PARTIAL | runs locally but some tests are Linux-shaped; the #2977 lesson lived here — invoke it explicitly for any change touching files tests walk |
| cross-compile matrix (vendored openssl, windows/freebsd/musl triples) | NO — SKIP-class | covered by the G-PR1b step-3 OS-conditional read-through by hand |

## Post-send discipline

- First-contribution CI may sit in `action_required` until a maintainer approves the workflow run — say so in the reply, never re-push to trigger. (#3025 auto-ran instead: fork-PR checks started within a minute — the repo's `pull_request` workflows fire on fork heads.)
- Poll the first CI run once; an unpredicted red is a finding about OUR gates — `gh run view --log-failed` immediately.
- The macOS env-failures above are the known false-red set when a maintainer reads CI green/Linux vs our local red — preempt in the PR body when relevant (the bench-128 body already carries the sentence).
- **Codecov bot is the first responder on this repo** (worked example #3025, 04:15Z, ~30 min after the send): `codecov-commenter`, author_association NONE — telemetry, never a review, never replied to. Its project-coverage delta is an artifact on fork PRs (HEAD uploaded 5 coverage reports vs BASE's 14 — "HEAD has 9 uploads less than BASE" — so #3025 read "−6.77% project" while the real signal was patch coverage 88.62% / 90 lines missing, mostly the new module's `info!`/`debug!` arms and error paths). Read the report's own uploads note before quoting any delta; act on patch-coverage gaps only if a maintainer raises them. Added to the `pr` + `outbound-pr` skills 2026-11-11.

## Campaign state on this repo

- PR #2977 (CACHEDIR.TAG, goodwill-class) **MERGED 2026-11-07 12:31Z** — six review rounds resolved; the owner's sequencing condition for #2968 ("wait for prev PR accepted first") is satisfied.
- Standing constraint "ONE PR per repo ever": the #2968 send (bench 128) is the owner-sanctioned exception (plan 331 T6a, owner verdict 2026-11-02); the send itself remains owner-gated (G-PR4 seal + go).
