# `dist` — shipped-artifact build-config healing (6th retrieval domain)

> Phase 1 landed 2026-09-09 (Issue 085); the B1/B2 context extensions landed
> 2026-09-10 (`759e209d`). **DEFAULT-ON since 2026-09-13** (Issue 085 closed
> — owner pull). All four GOAT conditions PASS (suggest precision +
> score-bench FLAT at the canonical state + the router re-pin + real consumer
> cites — Bench 082 + §6 addendum). `--no-default-features` is the tested
> flag-OFF path; the `#[cfg(feature = "dist")]` gates are retained.

## What it is

**Cargo.toml build-configuration patterns whose fix is measured, not
guessed**: release-profile size knobs (strip / size profile / panic) for
crates that SHIP a binary, plus the workspace-member profile trap. The
second no-`syn` scanner after docker — a hand-written section-aware line
scanner, zero new deps (no `toml` crate, no regex).

## Why it exists (the measured delta)

This tool's own shipped binary rode plain `--release` through v0.1.4:
**11,903,168 bytes** stripped where the knob set measures **4,546,448**
(-61%) — found by the OWNER, not by any agent session, despite a rebuild-
for-freshness pass in between. `binary_size_gate.sh` watches this repo's
release lane at package time; this domain watches EVERY manifest the healer
scans at dev time, before the bloat ships. `cargo bloat` first: 70% of
`.text` was own-code corpus tables — codegen knobs win, dep removal does not.

## The Phase-1 corpus (5 rules, all suggest-only)

| Rule | Family | Fires when | Advice |
|---|---|---|---|
| `shipped-binary-unstripped` | `ShipSize` | `[[bin]]` + no `strip` key in any `[profile.*]` | `strip = true` (symtab = the architecture; 14,485 symbols measured on v0.1.3) |
| `shipped-binary-missing-size-profile` | `ShipSize` | `[[bin]]` + no `lto` key | add `[profile.dist]` (lto fat + codegen-units 1 + opt-level "s" + panic abort + strip), never edit release |
| `cli-panic-unwind-tables` | `ShipSize` | `[[bin]]` + no `panic` key | `panic = "abort"` for CLIs; NOT for panic-isolating servers (context note) |
| `member-profile-ignored` | `ProfilePlacement` | member manifest with `[profile.*]` + parent `[workspace]` evidence (ScanContext pattern) | the block belongs in the workspace root |
| `large-api-dependency-mostly-unused-hint` | `BuildTime` (Batch 143) | any `*dependencies` section naming a known-large API crate (`windows`, `windows-sys`, `rustix`, `aws-sdk-*`) | measure `-Zhint-mostly-unused` (nightly-only, `.cargo/config.toml` rustflags; nnethercote Dec-2025: big compile-time wins for small-fraction consumers of windows/rustix/aws-sdk-ec2) |

Context notes ride the corpus (`context_notes`): the bench-inheritance caveat
(rule 2), the panic-isolation caveat (rule 3), and the Batch-143 rule's
out-of-manifest honesty — the hint is a rustc flag the manifest-scoped
scanner cannot see adopted, so the fixed fixture IS the `.cargo/config.toml`
snippet and the advice carries the measure-first caveat.

## The refusals are load-bearing

[`refuses`](../../src/domains/dist/mod.rs) pins the advice this domain must
never emit, enforced by `tests/dist_gates.rs`:
- **`edit-release-profile-opt-level`** — `[profile.bench]` inherits release;
  opt-level/codegen-units there poison the criterion benches. The advice is
  ALWAYS a separate `[profile.dist]` + `--profile dist`.
- **`strip-in-dev-profile`** — stripped dev symbols break debuggers for zero
  shipped-byte gain.

## Context derivation — B1/B2 (`759e209d`, Bench 082 §6)

The first real consumer scan exposed two blind spots in the section-only
scanner, both closed by [`context.rs`](../../src/domains/dist/context.rs)'s
`DistContext::derive(manifest_path)` — ONE bounded ancestor walk (≤32
levels), resolved once per manifest:

- **B1 `autodiscovered_bin`** — cargo discovers bins from `src/main.rs` and
  directly-nested `src/bin/*.rs` with no `[[bin]]` section at all (gated on
  `[package]`, the shared `ships_bin` helper). Missed a real true positive
  (`riir-gm-tool`) before this landed.
- **B2 `root_profile_knobs`** — a workspace MEMBER cannot carry effective
  profile knobs; the strip/lto/panic presence of the nearest `[workspace]`
  ancestor is read and consumed by the shared `effective_knobs`
  self-or-root resolver. Eliminated two false cites on `riir-chaind`.

`member-profile-ignored` is UNCHANGED by both; the panic rule keeps its
daemon caveat.

### Honest caveats — where the scanner still disagrees with cargo

The B1 class is NARROWED, not provably closed. Verbatim from Bench 082 §6.4
(kept here so a reader of the domain never has to find the bench):

- **`autobins = false` is not read** — a manifest opting out of
  autodiscovery beside a present `src/main.rs` still fires.
- **Deeper bin shapes are not derived** — `src/bin/<dir>/main.rs` targets
  and `[[bin]]` sections whose `path` points OUTSIDE the manifest's
  directory are invisible to the tree check. (The converse is safe: the
  check can never invent a bin cargo would not build — it requires
  `[package]`.)
- **Knobs are name-agnostic** — a root carrying `strip` under
  `[profile.dist]` satisfies the strip rule even though a plain
  `cargo build --release` does not take that profile. Key presence in ANY
  profile is the deliberate Phase-1 semantic; per-name matching is the
  honest next narrowing if the owner wants it.
- **A member's own knobs still satisfy rules 1–3** even though a root build
  ignores them — the union trades that gap for not double-citing, and
  `member-profile-ignored` fires simultaneously to move the block.
- **`exclude`d members** still inherit `under_workspace_root` (and now root
  knobs) from the nearest-ancestor walk — a hedged Phase-1 inaccuracy,
  unchanged.
- **Per-manifest, not per-bin** — a crate with N bins gets 3 citations, not
  3N (pre-existing design).

## The oracle question, answered by precedent

Fixes verify by scan well-formedness (Tier 0, the docker pattern) — never by
rebuilding and measuring bytes. `rust_perf`'s fixes verify by `cargo test`
re-run, not by benchmarking each fix; the GAIN is proven once at class level.
That class-level proof is the -61% measurement above. The retrieval-side
cost is measured too: the router-annotation floor at `default + dist` is
pinned at 74/101 measured (73.3%, +1 vs the dist-off twin — the docker
precedent: an off-vocabulary centroid absorbs fewer generic Rust spans).

## Not this domain

NOT a general TOML linter, NOT dependency auditing (cargo-machete /
cargo-deny), NOT profile surgery on library-only crates (every ship-size
rule requires `[[bin]]`), and Phase 1 never WRITES a manifest (the bounded
`[profile.dist]` insert is the Phase-2 `--fix` lane).

## The sixth rule (Batch 146, 2026-09-16)

`pgo-training-set-covers-real-workload-hint` is PROXY-TRIGGERED BY DESIGN:
the scanner cannot see whether PGO exists (it is an out-of-manifest build
pipeline property), so the rule fires ONLY on the fully-tuned audience — a
shipped binary whose effective `strip`+`lto`+`panic` knobs (own OR root)
are ALL set, i.e. every manifest-level lever this domain advises is
exhausted. Strictly additive: silent for every manifest with a knob still
missing, fires ALONE on a fully-tuned one. Measured provenance: rustc
#159091 (adding rustdoc benches to the PGO training set — 2.85% mean
wall-time across rustdoc benchmarks, >6% best; "the training set
matters"). Record: [Research 173](../../.research/173_batch146_accumulated_readlane_pack.md).

## See also

- [`HISTORY.md`](../../HISTORY.md) §Issue 085 — the issue record + GOAT gate (issue file removed at closure)
- [`docker.md`](docker.md) — the scanner + ScanContext precedent this domain follows
- [`scripts/binary_size_gate.sh`](../../scripts/binary_size_gate.sh) — the release-lane ceiling that complements this domain
