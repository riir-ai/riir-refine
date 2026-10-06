# Issue 150 — the Homebrew-installed cargo-refine shadows the maintained symlink on PATH

**Status:** OPEN — 2026-10-06 (found by the dapps plan-045 verdict round 2 during the B7 privacy-cut live smoke) · Machine: the M3 box · No code change proposed yet — this is the decision record.

## The shape

`/opt/homebrew/bin/cargo-refine` (a Homebrew Cellar install from the public
tap, `../Cellar/cargo-refine/0.2.1/bin/cargo-refine`, installed 2026-10-05
23:30) precedes `~/.cargo/bin` on this box's PATH — so bare `cargo refine`
runs the RELEASED 0.2.1 build, not the maintained
`~/.cargo/bin/cargo-refine → ../git/riir-refine/target/release/cargo-refine`
symlink. Every post-release fix in this repo's working tree is invisible to
the subcommand that ran: measured live, the Cellar binary rendered NO
purchases section for `account --history` (it predates the B7 render,
`c80a5794`) while the maintained binary parsed and rendered the same
deployed worker's page correctly. `--version` reports the same 0.2.1 +
release-set-complete stamp on BOTH — the build stamp cannot distinguish
them (it stamps the compiled feature set, not the commit), so the shadow is
silent.

Scope of the exposure: it is not just convenience — the global lint-heal
rule (`cargo refine` as the dev-time default across the workspace) resolves
through PATH, so heals run the release build. Nothing breaks today (both
binaries report the same version + feature set; neither carries
`clippy_verify` — consistent with C1 not having landed), but fixes made
after the release are invisible until the next tag.

## The verdict's interim ruling (2026-10-06, session `1123c233`)

- **Do NOT `brew unlink` this session** — machine-wide state shared with
  sibling sessions, and the Cellar install may be the owner's deliberate
  check of the public release lane.
- **Until decided: smokes and gates that care call the binary by ABSOLUTE
  PATH** (`~/.cargo/bin/cargo-refine`). Recorded precedent: the dapps 018
  deploy-ledger smoke note.

## Options for the owner

1. Keep both; document the precedence (the tap install is the "stable"
   lane, the symlink the dev lane) — smokes/gates pin the absolute path.
2. `brew uninstall cargo-refine` on this box (the symlink becomes the only
   cargo-refine) — the cleanest dev posture; re-install via the tap is one
   command when the release lane is being tested.
3. Ship the build stamp with the commit (build.rs already derives the
   feature set from cargo's own exports; adding
   `CARGO_GIT_HASH`-class env would make `--version` name the commit and
   the shadow detectable) — helps every box, not just this one.
