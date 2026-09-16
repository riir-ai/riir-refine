# Issue 117 — Global-binary staleness guard (the 00:21 class: silent re-stale on bare `cargo build --release`)
**Status:** OPEN — P1, unblocked. Filed 2026-09-16 from the room-to-improve triage (session record: HISTORY.md same-day entry).

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
- [ ] T1 — `--version` feature-set stamp (const from `cfg!`/build script;
      shown on `-V` and in the startup banner when any gated surface is
      invoked).
- [ ] T2 — Stub refusal arms for the network-gated commands under
      `not(feature = "kat_transport")` etc. — loud line + exit 2.
- [ ] T3 — Contract test (the `kat_soft_gate_cli.rs` pattern): a
      default-features build's `sync` invocation exits 2 with the refusal
      line, never "unknown flag".
- [ ] T4 — SKILL.md rebuild-recipe row.

## Related
- AGENTS.md §"Production adoption" (the 00:21 class, the 2026-09-10
  clear-and-smoke), `tests/kat_refill_cli.rs` / `tests/kat_soft_gate_cli.rs`
  (the exit-2 contract pattern).
