# Issue 135 — `cargo heal --fix` applies allow-by-default lints the target crate never enabled
Status: OPEN (filed 2026-09-26 from a katgpt-rs session's reflex-site registration; repro measured, fix not started)

## Observed (usage artifact, reflex-site `466e331` landing)

`cargo heal --fix` on reflex-site's `wasm-head` crate — which enables no
clippy group beyond the defaults — rewrote a doc-comment shell command:

```text
//!   cargo run -p arena-head-wasm --bin gen_corpus
//!   cargo run -p arena-head-wasm --bin `gen_corpus`      ← healer output
```

The edit was reverted by hand and the session used `cargo clippy --fix`
instead. Two independent defects, the first is the class:

1. **Level:** `doc_markdown` is PEDANTIC (allow-by-default). Plain
   `cargo clippy` on that tree emits **zero** warnings, so the healer
   "fixed" a finding the crate's own lint configuration says does not
   exist. The proposer's module doc justifies it by riir-clippy's own
   `#![warn(clippy::pedantic)]` root — a property of THIS crate, applied
   to every crate the global binary is pointed at.
2. **Shape:** the line is a command, not prose. 3 spaces after `//!` is
   below the 4-space indented-code threshold, so it IS a paragraph and
   clippy (under `-W clippy::doc_markdown`) suggests the identical edit —
   but the right repair is to fence/indent the command, never to
   backtick one argv token inside it. Backticking produces a line that no
   longer copy-pastes into a shell.

## Minimal repro (M3, 2026-09-26)

```bash
mkdir -p /tmp/healrepro/src && cd /tmp/healrepro
printf '[package]\nname="healrepro"\nversion="0.1.0"\nedition="2021"\n[workspace]\n' > Cargo.toml
printf '//! Regen.\n//!\n//!   cargo run -p healrepro --bin gen_corpus\n\n/// One.\npub fn one() -> u32 { 1 }\n' > src/lib.rs
cargo clippy -q                 # silent — nothing to fix at the crate's level
cargo heal --fix src/lib.rs     # 1 edit "verified", line 3 gains `gen_corpus`
```

## Blast radius (measured from source, not estimated)

Of the **77** `FIXABLE_LINTS`, **13** carry an allow-by-default
`lint_group` in `corpus.rs` (12 pedantic + 1 nursery):
`doc_markdown`, `items_after_statements`, `manual_let_else`,
`manual_string_new`, `map_unwrap_or`, `match_bool`,
`redundant_closure_for_method_calls`, `semicolon_if_nothing_returned`,
`single_match_else`, `string_lit_as_bytes`, `uninlined_format_args`,
`unnecessary_map_or`, `unreadable_literal`.

⚠ At least two of those corpus labels look STALE against clippy 1.98
(`uninlined_format_args` and `unnecessary_map_or` are `style` upstream
now), so the group column must be re-derived from the pinned clippy's
own lint table (`clippy-driver --explain` / `lint_groups`), not read off
the corpus — a gate keyed on a stale label would silently drop
default-on fixes.

Nothing in `heal.rs` consults the target crate's effective lint level
(grep: no `[lints.clippy]` / crate-root `#![warn(clippy::…)]` reader).

## Proposed fix (recommendation — owner picks)

- [ ] T1: group table from the pinned clippy (not the corpus), with a
      test that fails when a corpus `lint_group` disagrees with it.
- [ ] T2: effective-level gate in `--fix`: an allow-by-default lint's
      edits apply only if the target crate enables it (crate-root inner
      `#![warn|deny|forbid(clippy::<lint|group>)]`, `[lints.clippy]` in
      the package or `[workspace.lints.clippy]` when `lints.workspace =
      true`). Otherwise report as `skipped: allow-by-default at crate
      level` on the verdict line — never silently.
- [ ] T3: `--groups pedantic,nursery` opt-in to restore today's
      behaviour explicitly. ⚠ katgpt-rs AGENTS.md steers agents to
      `cargo heal` for `match_bool` / `map_or` / format-args — T2 changes
      that default for crates without pedantic, so the katgpt-rs /
      cargo-heal skill wording needs the flag named in the same landing.
- [ ] T4: `doc_markdown` command-line skip: a doc line whose first word
      after the marker is a command (`cargo`, `rustup`, `npm`, `$ `, a
      path to `scripts/…`) is skipped (divergence from clippy, the safe
      direction — add it to the SKILL.md Issue-120 skip table).

## Secondary observation (not the class)

The global `~/.cargo/bin/cargo-heal` → `target/release/cargo-heal`
printed `self-evolve: memory-only (learning disabled: built without the
clippy_verify oracle)`, while katgpt-rs AGENTS.md says the global binary
is built `--features fix_verify,clippy_verify`. Either the binary was
rebuilt without the feature or the doc is stale — the trajectory the
repro recorded was memory-only either way.
