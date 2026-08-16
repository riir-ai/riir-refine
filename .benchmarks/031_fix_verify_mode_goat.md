# Bench 031 — Issue 015: `--fix --verify` compile-gate GOAT (real-repo revert proof + the skipped G5 sites land)

> **Renumbered from 027 (2026-08-16).** A same-day collision: T3.5's G1-recovery
> bench (`.benchmarks/027_t35_g1_recovery.md`, commit `0c5af53`) allocated 027
> first and carries ~2× the live references, so this bench moved to the free
> 031 slot per the collision tie-breaker (fewer references moves).

**Date:** 2026-08-16.
**Scope:** Issue 015 acceptance — the `cargo heal --fix --verify` mode (feature
`fix_verify`, commit `2ace442`) validated end-to-end on the real repo the
conservative gates were learned from (katgpt-pruners, the Bench 019 G5 crate),
plus one root-cause fix the GOAT run itself surfaced.

## What verify-mode is

`--fix --verify` closes the loop IN the tool: collect edits (with loosened
gates) → baseline `cargo check --all-targets --message-format=json` on the
unmodified tree → write every fixed file → check → map rustc JSON errors back
to file/line → revert ONLY the offending edits (line-matched span-level,
whole-file fallback when attribution is impossible) → re-check → repeat until
clean. The loosened gates under `--verify`:

- `vec-with-capacity` `ProposerOpts::untyped_paths` — plain-path capacity
  bounds (consts, associated consts like `Strategy::COUNT`, locals like
  `min_len`) are attempted; a wrong type surfaces as rustc E0308 and the loop
  auto-reverts just that edit. The **scope gate stays ON** in both modes
  (use-before-decl is a real correctness issue, not a type question).
- `VERIFY_LINTS` joins the fix set (`redundant_closure`,
  `comparison_to_empty`, `while_let_on_iterator`, `single_char_pattern`) —
  the type-blind shapes the parse-only emitter cannot prove, each backed by
  the compile gate (the E0631 failure class is exactly what the loop reverts).
- `fxhashmap` dep gate (always-on, no cargo run needed): the owning crate's
  `Cargo.toml` is text-scanned for `rustc-hash`; files in crates without the
  dep hard-skip the rule — the measured 422-edit katgpt-rs instance from
  Bench 019 cannot recur.

## The GOAT run (acceptance item 4)

Target: the 6 katgpt-pruners files holding the sites the G5 parse-only gates
skipped (Bench 019 §G5 addendum; the issue names 5 + the two revert classes).
Feature set (15 features covering every edited module):
`committee_boost,dreamer,fft,fpcg_selector,problem_mutator,memo_reflections,subterranean,sudoku,thicket_variance_probe,hydra_budget,collapse_aware_thinking,rosetta_pruner,egcs,monopoly,bfcf_lsh_cms`.

```
verify: 6 file(s) / 14 edit(s) — baseline + compile-gated apply
verify: 3 cargo check(s) — kept 6 file(s) / 12 edit(s), reverted 2 edit(s)
  reverted src/monopoly/players.rs: vec-with-capacity @ line 80
  reverted src/roaring_membership.rs: redundant_closure @ line 353
```

**Kept (12) — every named skipped site landed, compile-confirmed usize:**

| Site | Edit |
|---|---|
| `hydra_budget.rs:145` | `Vec::with_capacity(n)` — `n = skip_plan.n_layers.min(num_layers)` |
| `collapse_detector.rs:1336` | `Vec::with_capacity(N_TRACES)` — `const N_TRACES: usize = 24` |
| `rosetta.rs:164` | `Vec::with_capacity(len)` — `candidates.len().min(results.len())` |
| `episode_pruner.rs:112` | `Vec::with_capacity(min_len)` — `candidate.len().min(reference.len())` |
| `monopoly/players.rs:1046` | `Vec::with_capacity(Strategy::COUNT)` |
| + 7 structural | `match_bool` ×6 (collapse_detector 677/918, episode_pruner 115/223/273/330), `map_unwrap_or` ×1 (roaring_membership 241) |

**Reverted (2) — the acceptance's intentionally-bad classes, proven on the
REAL repo (not just the fixture):**

- `monopoly/players.rs:80` — `0..BOARD_SIZE` with the `u8` const → E0308
  (the G5 run 1 failure class).
- `roaring_membership.rs:353` — `|v| from_bool_vec(v)` → fn path loses the
  `&Vec<bool>` → `&[bool]` deref coercion → E0631 (the G5 run 1 failure
  class).

Both reverts were span-level (line-matched), both good edits in the same
files survived, and the loop converged in one revert round (3 cargo checks
total: baseline + fail-detect + clean-confirm).

## Zero regressions (acceptance item 4, second half)

- `cargo test -p katgpt-pruners --features <set> --lib`: **1245/1245 pass**
  (post-rustfmt re-run: 1245/1245 again; post-rebase onto the sibling's
  katgpt-core commit: 1245/1245 again).
- `cargo clippy --all-targets` under the same set: **no new warnings** — the
  only katgpt-pruners hits are the documented pre-existing sites
  (`sudoku_pruner::candidate_set` dead_code from Bench 019;
  `unnecessary_sort_by` in monopoly/players). One hand-polish: the
  `map_unwrap_or` rewrite at `roaring_membership.rs:241` produced
  `map_or(false, …)` which clippy 1.97 upgrades to `is_ok_and` — applied
  (the site warned under BOTH the old and new form; now it is silent).
- `rustfmt --check`: all 6 edited files clean (the span-preserving emitter's
  single-line constructs in collapse_detector + episode_pruner were formatted
  once after landing).

Landed as katgpt-rs `212ed7942` (develop, rebased non-interactively over the
sibling's same-window commit, pushed).

## Acceptance item 3 — fixture-repo revert proof (re-run this session)

`fix_verify_e2e_bad_edits_reverted_good_survive` (spawns real cargo, `#[ignore]`d
in the default suite):

```
cargo test --features fix_verify --lib -- --ignored --nocapture fix_verify_e2e
test fix_verify::tests::fix_verify_e2e_bad_edits_reverted_good_survive ... ok
```

One good vec edit (usize param) kept; the u8-const vec edit + the fn-path
coercion closure reverted; disk state asserted.

## Root-cause fix the GOAT run surfaced

First run died with `cargo spawn failed: … (os error 123)`. Cause: relative
path args (`src/hydra_budget.rs`) made `owning_package_root`'s `pop()` walk
to the EMPTY path, whose `Cargo.toml` join resolves against the process cwd —
returning `Some("")`, and `Command::current_dir("")` fails with
ERROR_INVALID_NAME. Fix: `owning_package_root` now normalizes via
`std::path::absolute` on entry (+ regression test
`owning_package_root_relative_input_is_absolute`). This also fixes the
fxhashmap gate's manifest lookup for relative invocations from any cwd.

## Honest notes + environment

- **The 293-edit observation.** A full `src/` scan of katgpt-pruners proposes
  **293 edits across 104 files** under verify-mode — Issue 016's structural
  expansion (FIXABLE 6→33) means the tool's detection surface is now ~15× the
  G5-era batch. This GOAT deliberately ran a **surgical file-scoped batch**
  (the 6 files holding the issue's named sites): landing 293 edits in a
  sibling repo is a consumer decision, not a tool default. The full-crate
  verify batch is now a plausible follow-up issue if wanted.
- **Windows toolchain quirk (box-specific, not a code change):** the
  `/e/git/.cargo/config.toml` shim (`/LIBPATH:…msvc_stl_shim` link-args,
  valid for link.exe) is fatal for the gnu host toolchain that
  `rust-toolchain.toml` `channel = "1.97.0"` resolves to (default host is
  gnu). Workaround for every cargo invocation this session:
  `RUSTUP_TOOLCHAIN=1.97.0-x86_64-pc-windows-msvc` (matches how sibling
  agents already invoke cargo on this box). The heal binary passes its env
  through to the verifier's cargo children, so one env var covers the whole
  loop.
- **Feature coverage is the caller's duty** (same lesson as G5's
  "vacuous default-features check"): `--verify-args "--features …"` must
  cover every edited module; the compile gate is only a gate for code that
  compiles. The post-run `git diff --name-only` vs the feature set is the
  check.
- Issue 015's "8 skipped sites": the issue names 5 + "…". At GOAT time the
  tree had drifted (sibling commits since 8c331cd9); the tool found exactly
  the 5 named vec sites + 7 structural in the 6 target files. The remaining
  ~3 unnamed sites could not be re-identified from the issue text alone —
  not claimed.

## GOAT verdict

**PASS.** All five acceptance items closed: the feature exists and implies
`latent_retrieval`; the fxhashmap manifest gate shipped (+ now
absolute-path-safe); fixture-repo revert proof re-run green; the real-repo
GOAT landed every named skipped site with the two bad-edit classes
auto-reverted in the same batch; zero test/clippy regressions. Issue 015
RESOLVED (this bench is the record; issue file removed per the
noise-reduction rule).
