# Issue 117 — Global-binary staleness guard (the 00:21 class: silent re-stale on bare `cargo build --release`)
**Status:** RESOLVED 2026-09-16 — T1-T4 all green (`build.rs`, `src/build_stamp.rs`, `tests/build_stamp_cli.rs`). Filed same day from the room-to-improve triage.

⛔ **Landed at `3dc472c7`, whose message describes P32/P33 and not this issue.**
A concurrent session ran `git commit` against the shared index in the second
between this work being staged and its own `git commit … -- <pathspec>`, and
git cannot tell one session's staged files from another's. The content is
intact and verified at that commit; the message is not recoverable without
rewriting a commit that is half somebody else's work. Recorded here rather
than repaired, because the record is the part a reader needs. The narrower
habit the incident argues for: stage and commit in ONE `git commit --only
<paths>` rather than `git add` followed by `git commit -- <paths>`, which
leaves a window in which the shared index carries your files and anyone's
bare commit takes them.

## Why

Any bare `cargo build --release` of this crate **overwrites the symlink
target** (`~/.cargo/bin/cargo-heal`) with a default-features binary and
silently re-stales `sync` / `--refill` / burn-push / auto-sync / the consent
heartbeat — while `--help` still ADVERTISES the flags (the 00:21 incident,
AGENTS.md "Standing caveat"; the blocker was cleared 2026-09-10 but the
footgun remains armed). The shipped binary's `required-features` are
`latent_retrieval,rust_perf,clippy_lints,kat_service` — a build naming less
compiles **no cargo-heal binary at all** for `-p` runs, but a plain
workspace/release build of the crate still produces a binary that exists and
quietly lacks the gated surfaces.

The failure mode is silent by construction: the gated commands are `cfg`'d
out, so the CLI either hides them or parse-errors them — the user sees a
"working" healer that cannot talk to the network.

## Shape of the fix

- **Stub arms, not absence**: the network-gated subcommands (`sync`,
  `--refill`, burn/consent tails, `--mine`'s sync tail) always PARSE; when
  `kat_transport`/`kat_service`/`latent_retrieval` are compiled out, they
  print one loud refusal line naming the release feature set + the rebuild
  command, exit 2 — never "unknown flag", never silence.
- **`--version` feature-set stamp**: print the compiled feature set beside
  the version (a `Cargo.toml`-driven or `build.rs`-generated const), so a
  stale binary is identifiable in one command from any terminal.
- Doc row in the cargo-heal SKILL.md: the rebuild recipe (release set, the
  isolated-worktree precedent from 2026-09-10).

## Honest caveats

- This cannot protect the SYMLINK itself (a build outside this repo's
  control); it converts silent staleness into loud refusal at the CLI
  boundary — the `--refill` exit-2 precedent is the model.
- Keep the stub arms out of the release binary's parse path cost: they are
  `cfg`-branches on the same match arms, not extra dispatch.

## Tasks
- [x] T1 — `--version` / `-V` prints the compiled feature set and the
      release-set verdict (`complete` or `STALE — missing …` + the rebuild).
      **Derived, not typed:** `build.rs` reads cargo's own `CARGO_FEATURE_*`
      exports for the compilation actually happening, so the stamp cannot go
      stale the way a hand-maintained `cfg!` ladder would. Line 0 stays the
      unchanged `cargo-heal <version>` — anything parsing the old
      single-line output keeps working.
- [x] T2 — The gated commands already PARSED ungated (the subcommand words
      and `--refill` both reach a `cfg`'d arm), so the gap was the message,
      not the dispatch: all five refusal sites (`sync`/`update`, `replay`,
      the whole `kat_service` family, `--refill`) now print the three facts
      the 00:21 diagnosis had to reconstruct by hand — what refused and why,
      this binary's own feature set, and a rebuild command — and exit 2.
- [x] T3 — `tests/build_stamp_cli.rs` at the process boundary. The refusal
      arms are `#[cfg]`'d per FUNCTION rather than per file: a whole-file
      `#![cfg(not(feature = "kat_transport"))]` would compile this target to
      an empty binary under the release set and under `--all-features` and
      print a green `0 passed` — the Issue-713 green-zero trap. Measured:
      5 pass without the transport (2 refusal arms), 4 with it (1 healthy
      arm). Revert-probed: replacing the refusal with `unknown flag` reds it.
- [x] T4 — SKILL.md §Setup row (the footgun + the one-command diagnosis) and
      the `--version` usage row; the AGENTS.md "Standing caveat" sentence now
      records the repair beside the incident.

## What the landing measured that the filing did not

⛔ **"Print the release feature set" was the wrong remedy for one of the five
sites, and the error is the silent kind.** `replay`'s gate is
`kat_transport + clippy_verify + latent_retrieval`, and `clippy_verify` is NOT
in the shipped release set — so a user who pasted the bare release recipe
would spend a full release rebuild and hit the SAME refusal, with every reason
to conclude the guard is broken. `rebuild_command_for(needed)` unions the
release set with whatever the refusal named, de-duplicated and release-order
first; `the_rebuild_covers_every_feature_the_refusal_named` is the arm. **A
remedy that does not remedy is worse than no remedy** — it spends the user's
trust and their build time.

⚠ **What this does NOT do, unchanged from the filing:** it cannot protect the
SYMLINK. A `cargo build --release` outside this repo's control still
overwrites the target; the guard converts silent staleness into loud refusal
at the CLI boundary, which is the whole of the claim.

⛔ **A collateral defect, carried here because Issue 115 landed concurrently
from another session (`a9ac4ee5`) and its file is already removed:** Issue
113's landing (`3f0eecb9`) inserted `pub mod clippy_oracle;` BETWEEN a
`#[cfg(feature = "compile_oracle")]` attribute and the item that attribute was
written for, so the gate landed on `clippy_oracle` (which is std-only and is
called UNGATED from the bin) and `compile_oracle` — the module the adjacent
comment describes — lost its gate entirely. Silent under default features,
because both are on; a `--no-default-features` build of the bin's own
`required-features` set did not compile. Repaired in this commit. It is the
orphaned-attribute class the workspace gates for one repo over, reached by an
INSERTION rather than by a blank line — which is why `orphaned_attr_gate.py`'s
blank-line predicate would not have seen it.

⚠ `RELEASE_FEATURES` is the one constant here that is TYPED rather than
derived, because it is a claim about `.github/workflows/release.yml`. It
carries `release_set_matches_the_release_workflow`, which parses that file and
compares — and a companion arm proving the parser REFUSES (rather than
returning a plausible wrong set) on the shapes that would make the comparison
vacuous.

## Related
- AGENTS.md §"Production adoption" (the 00:21 class, the 2026-09-10
  clear-and-smoke), `tests/kat_refill_cli.rs` / `tests/kat_soft_gate_cli.rs`
  (the exit-2 contract pattern).
