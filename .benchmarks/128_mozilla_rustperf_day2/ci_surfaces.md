# mozilla/sccache — CI surface map (for the next PR to this repo)

Per outbound-pr G-PR1b #4: the per-repo CI map so the next PR starts from the map, not a re-derivation. Derived 2026-11-04 from `.github/workflows/ci.yml` at pin `ba261b8` + the bench-128 G-PR1b run.

## Their CI invocations (what ci.yml spells)

| invocation | local-runnable? | notes |
|---|---|---|
| `cargo clippy --locked --all-targets -- -D warnings -A unknown-lints -A clippy::type_complexity -A clippy::new-without-default` (ci.yml:19, via a `cargo_cmd:` template) | YES at +1.91.0 (their MSRV = `rust-version`) | the harness regex CANNOT see the template — run by hand, always |
| `cargo fmt -- --check` | YES | whole package |
| `cargo test --locked --lib --bins --tests` (+ per-crate splits) | YES at +1.91.0 | on this macOS box: 3 lib failures are environment-dependent (unix-socket binding ×2, rust hash-env) and fail identically on the pristine pin — proven twice (bench 128 §7) |
| `cargo check --no-default-features` (feature-matrix lane) | YES | 6 pre-existing warnings, none diff-touching |
| `cargo build -p randomize_readdir` (integration helper) | YES | parses their CI matrix — the `outbound_pr_ci_harness.sh` inventory catches this one |
| `cargo test --test system` (system tests, CI = Linux) | PARTIAL | runs locally but some tests are Linux-shaped; the #2977 lesson lived here — invoke it explicitly for any change touching files tests walk |
| cross-compile matrix (vendored openssl, windows/freebsd/musl triples) | NO — SKIP-class | covered by the G-PR1b step-3 OS-conditional read-through by hand |

## Post-send discipline

- First-contribution CI may sit in `action_required` until a maintainer approves the workflow run — say so in the reply, never re-push to trigger.
- Poll the first CI run once; an unpredicted red is a finding about OUR gates — `gh run view --log-failed` immediately.
- The macOS env-failures above are the known false-red set when a maintainer reads CI green/Linux vs our local red — preempt in the PR body when relevant (the bench-128 body already carries the sentence).

## Campaign state on this repo

- PR #2977 (CACHEDIR.TAG, goodwill-class) OPEN since 2026-11-02, head `bd9915e`, 6 review rounds resolved, awaiting sylvestre.
- Standing constraint "ONE PR per repo ever": the #2968 send (bench 128) is the owner-sanctioned exception (plan 331 T6a, owner verdict 2026-11-02) — and the owner directed on 2026-11-04 that it additionally SEQUENCES behind #2977's outcome: merged → send may go; closed unmerged → the never-re-PR-after-close rule blocks it (repo block review is an owner call at that point).
