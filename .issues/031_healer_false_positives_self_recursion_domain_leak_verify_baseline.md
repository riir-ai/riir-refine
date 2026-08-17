# Issue 031 — heal validation on katgpt-pruners surfaced 3 healer defects (1 CRITICAL runtime class)

**Status:** OPEN
**Filed:** 2026-08-17
**Source:** live validation of the 587-edit heal batch (katgpt-rs `4b6675dc5`)

## Context

The previous session applied `cargo-heal --fix --write --verify --domain clippy`
to `katgpt-rs/crates/katgpt-pruners/` (587 edits / 121 files). Sequential
post-hoc validation (clippy all-features + `cargo test --lib --all-features`
3006 tests + default-features check) caught what the healer's own gates did
not. Three distinct defects, all reproducible from the committed diff.

## H1 (CRITICAL) — `comparison_to_empty` proposer lacks a self-recursion guard

The proposer rewrote `self.len() == 0` → `self.is_empty()` **inside the
`fn is_empty` method itself**, at 3 sites:

- `decision_explainer.rs` `SensitivityCache::is_empty`
- `bfcp_region_cache.rs` `BfcpRegionCache::is_empty`
- `skill_catalog.rs` `SkillCatalog::is_empty`

Result: infinite recursion → `STATUS_STACK_OVERFLOW` (0xc00000fd) at test
runtime. **The verify tier (cargo check) is structurally blind to this
class** — it compiles fine; only runtime catches it. This is the most
dangerous false positive found in the 027/028/029 series: prior classes
produced wrong text or missed lints; this one breaks the program.

Fix shape: the proposer must inspect the enclosing function — if the
rewrite target receiver is `self` and the enclosing `fn` is `is_empty`
(and generally: if the suggested method call resolves to the function
currently being defined), refuse the edit. Fixture: both the direct
`fn is_empty(&self) -> bool { self.len() == 0 }` (must NOT fire) and a
legitimate distant call site (must still fire).

Note: real clippy does NOT fire on this shape (empirically confirmed —
the pre-heal file had zero `comparison_to_empty` findings at these lines
under `-W clippy::pedantic`). Our proposer is stricter-than-real in the
wrong direction.

- [ ] H1a: self-recursion guard in the `comparison_to_empty` proposer
- [ ] H1b: fixtures — `fn is_empty { self.len() == 0 }` stays silent;
      distant `x.len() == 0` still fires
- [ ] H1c: extend `clean_spans_fire_no_lint` with the self-recursion shape

## H2 — `--domain clippy` does not exclude rust_perf proposers

Evidence: `monopoly/players.rs` received a `vec-with-capacity` edit
(`Vec::new()` → `Vec::with_capacity(BOARD_SIZE)`) under
`--domain clippy`. vec-with-capacity is a rust_perf rule. The domain
filter is supposed to scope the fix set; it leaked. (That specific edit
was also an E0308 break — see H3 — but the domain leak is independent:
correct rust_perf edits also applied under `--domain clippy`.)

- [ ] H2a: reproduce — `--domain clippy` dry-run on a file with a known
      rust_perf-eligible span; assert zero rust_perf proposals
- [ ] H2b: fix the domain filter at the proposer-selection layer; add
      regression test

## H3 — verify baseline is default-features; feature-gated modules escape the compile gate

Two breaks compiled fine under the healer's default-features verify
baseline but failed `--all-features`:

- `Vec::with_capacity(BOARD_SIZE)` where `BOARD_SIZE: u8` (E0308) — the
  exact Bench 031 class, previously auto-reverted there, now escaping via
  feature-gating of the `monopoly` module
- `.map(CompactBitmap::from_bool_vec)` (E0631 fn-path coercion,
  `&Vec<bool>` → `&[bool]` deref coercion lost) — also the exact Bench 031
  class, module feature-gated

The verify tier's value proposition is "compile-gated edits"; a baseline
that skips gated modules silently narrows the guarantee. Options (pick
one, document):

- [ ] H3a: verify baseline switches to `--all-features --all-targets`
      (cost: slower verify; crates with broken all-features need opt-out)
- [ ] H3b (cheaper): verify runs BOTH default and `--all-features`
      baselines; an edit must survive both
- [ ] H3c (minimal): `--verify` prints a loud warning when the target
      crate has non-default features ("gated modules unverified; run
      cargo check --all-features before committing")

## Validation record (the heal commit itself)

katgpt-rs `4b6675dc5` — gates run post-fix:
- `cargo clippy -p katgpt-pruners --all-features --all-targets`: 0 errors
- `cargo test -p katgpt-pruners --lib --all-features`: 3006/3006
- `cargo check -p katgpt-pruners --all-targets` (default): clean
- target-lint delta: single_match_else 49→13 (remainder = documented
  deliberate skips: comment guards, spec aborts, user-enum arms)

## Rule

Healer fixes follow the 027/028/029 pattern: fix the proposer + add
fixtures, never suppress. H1 is the priority — it's the first
false-positive class that produces a runtime crash.
