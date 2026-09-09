# `dist` — shipped-artifact build-config healing (6th retrieval domain)

> Phase 1 landed 2026-09-09 (Issue 085). Opt-in feature `dist`, default-off —
> promotion follows the `rust_perf` precedent and needs the issue's GOAT gate
> (suggest precision + score-bench FLAT at the canonical state + a real
> consumer cite).

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

## The Phase-1 corpus (4 rules, all suggest-only)

| Rule | Family | Fires when | Advice |
|---|---|---|---|
| `shipped-binary-unstripped` | `ShipSize` | `[[bin]]` + no `strip` key in any `[profile.*]` | `strip = true` (symtab = the architecture; 14,485 symbols measured on v0.1.3) |
| `shipped-binary-missing-size-profile` | `ShipSize` | `[[bin]]` + no `lto` key | add `[profile.dist]` (lto fat + codegen-units 1 + opt-level "s" + panic abort + strip), never edit release |
| `cli-panic-unwind-tables` | `ShipSize` | `[[bin]]` + no `panic` key | `panic = "abort"` for CLIs; NOT for panic-isolating servers (context note) |
| `member-profile-ignored` | `ProfilePlacement` | member manifest with `[profile.*]` + parent `[workspace]` evidence (ScanContext pattern) | the block belongs in the workspace root |

Context notes ride the corpus (`context_notes`): the bench-inheritance caveat
(rule 2) and the panic-isolation caveat (rule 3).

## The refusals are load-bearing

[`refuses`](../../src/domains/dist/mod.rs) pins the advice this domain must
never emit, enforced by `tests/dist_gates.rs`:
- **`edit-release-profile-opt-level`** — `[profile.bench]` inherits release;
  opt-level/codegen-units there poison the criterion benches. The advice is
  ALWAYS a separate `[profile.dist]` + `--profile dist`.
- **`strip-in-dev-profile`** — stripped dev symbols break debuggers for zero
  shipped-byte gain.

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

## See also

- [`.issues/085_dist_healing_domain.md`](../../.issues/085_dist_healing_domain.md) — the issue + GOAT gate
- [`docker.md`](docker.md) — the scanner + ScanContext precedent this domain follows
- [`scripts/binary_size_gate.sh`](../../scripts/binary_size_gate.sh) — the release-lane ceiling that complements this domain
