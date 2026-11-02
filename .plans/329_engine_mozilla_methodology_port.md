# Plan 329 — engine distill T0b: the Mozilla-quartet methodology port (issue 197)

**Status:** STAGED — revised at verdict round 1 (REVISE addressed); claim per the issue before any engine edit

Executes [issue 197](../.issues/197_engine_mozilla_methodology_port.md) — the blaze-style methodology port (uniffi checksum-handshake × supply-chain vet-aggregate × sccache mock_command × sccache cache-key doctrines onto the healer). T0b is test/script-only: zero ENGINE behavior change, zero corpus change; the ONE report-behavior delta (the score_bench pooling split in P4-a's G1) is labeled in Phase 4.

## Phase 0 — claim + substrate re-verify

- [ ] Append the claim line to issue 197 (`**Claimed:** <session>, <epoch>, expires <ISO-UTC>`), push before implementing
- [ ] `py ../katgpt-rs/scripts/dual_allocation_gate.py` (0/0/0 expected)
- [ ] Re-run the substrate greps (issue 197 §substrate): the six `pub fn with_cargo_bin` verifiers, `traj_store.rs` zero-toolchain state, the score_history row union schema + the 45/217 generation-less count, the `src/domains/` population — anything moved re-scopes the phase

## Phase 1 — P1: the corpus pins manifest (T0b)

- [ ] `scripts/pins_manifest.py --write` emits `.benchmarks/pins_manifest.json`: derive from the corpus source `{rust_perf: {rules, categories, notbenchable, benchable, unannotated}, clippy_lints: {rules}, sec: {rules}, kernel_opt: {rules, witness}}` + the `src/domains/kernel_opt/router.rs` expert totals (NOT `src/draft/domain_router.rs` — the draft's router, never a pin site)
- [ ] Bare run diffs and exits 1 on drift naming every diverging site
- [ ] DRY (the round-1 shape): `corpus/tests.rs` + `router.rs` asserts READ the manifest via `include_str!` + serde_json — hand-typed numbers become derived reads; the asserts stay speed bumps, `--write` can never silence a red assert
- [ ] The PROSE half: the script gates mod.rs doc headers + AGENTS/README/`.docs` count rows against the same manifest
- [ ] Home: wire as a `scripts/ci_feature_guard.sh` layer (no docs-gate family exists here)
- [ ] G1 canary (a): drift one pin row in a fixture copy → red names it (the B308 class)
- [ ] G1 canary (b): change the corpus WITHOUT `--write` → red on the stale manifest (the reverse direction; `--write` is never run-to-pass)
- [ ] Ratchet: any exemption list the gate grows takes deletions only (the vet/fmt_pending drain-list shape)

## Phase 2 — P2: the failure-injection matrix (T0b)

- [ ] Shared `#[cfg(test)]` helper building the fake-cargo failure scripts ONCE (spawn-fail / garbage-stdout / signal-kill / empty-output / permission-denied)
- [ ] The matrix runs over `fix_verify` + `compile_oracle` MINIMUM; each arm asserts: loud refusal (`report.error` names the class) + tree restored byte-identical + stats outcome UNVERIFIED (never certified)
- [ ] The other four verifiers listed covered-or-deferred, by name + reason (`rustc_errors/driver.rs`, `rust_perf/{bench,verify}.rs`, `clippy_lints/verify.rs` — the latter two already carry `with_cargo_bin_overrides` posture tests)
- [ ] Per-class refusal COUNTERS in the verify stats (the round-1 leverage half): each refusal reason tallied; the run roll-up prints the tally — the failure taxonomy becomes first-class. ⛔ The counters are the SECOND T0b output delta (stats fields + roll-up line, output-only); the pooling split in Phase 4 stays the only verdict-affecting delta
- [ ] G1: each arm's assertions survive a "keep the edit anyway" mutation of the fake cargo (the refusal path is under test, not the outcome)
- [ ] The two existing arms (`kill_path`, `loop_exhaustion`) stay untouched — the matrix extends, never rewrites

## Phase 3 — P3: the domain-wiring registry gate (T0b)

- [ ] ONE central `pub(crate)` const `DOMAIN_REGISTRY` in `src/domains/mod.rs` (no `inventory`/`linkme` — zero new deps, zero public-API surface); rows carry name + roll-up label + score_bench domain key
- [ ] Test 1: every `src/domains/*/` directory with a `mod.rs` ∈ registry ∪ EXEMPTIONS — the exemption list is by-name with a reason per entry, starting exactly: `dist/` (scan-only domain), `docker/` (scan-only domain); `probe_witness.rs` is a FILE, not a directory (the test's predicate excludes files, noted in the test)
- [ ] Test 2: the heal.rs roll-up label map keys ⊆ registry labels
- [ ] Test 3: score_bench domain rows ⊆ registry
- [ ] G1 canary: a fixture dir in neither registry nor exemptions reds naming it
- [ ] Ratchet: the exemption list takes deletions only
- [ ] ⛔ NO heal.rs refactor — the gate reads the label map, it does not move the 20k-line bin

## Phase 4 — P4-a: score_history provenance axes (T0b with ONE labeled report delta)

- [ ] Add `rustc_version` (+ `clippy_driver_version` when the oracle lane ran) to the score_history append row; readers tolerate unknown fields (no `deny_unknown_fields` — verified at round 1)
- [ ] The generation-less class folds in HERE: legacy rows without `generation` read as `generation: unknown`, never pooled with pinned rows
- [ ] ⛔ THE LABELED REPORT DELTA: `score_bench/report.rs` comparison logic splits on (toolchain ∨ generation) — the G1 asserts two rows differing on either axis never pool into one comparison. This is report behavior, not engine behavior; it is the only behavior delta in T0b
- [ ] The landing's own score-bench run appends ONE new row carrying `rustc_version` — the field's first live evidence
- [ ] P4-b (trajectory-store key) is NOT this plan — file the follow-up issue at claim time (`197-followup`), carrying the migration rule: unpinned rows read `unknown-toolchain`, never pooled; it is a behavior change with its own gates

## Phase 5 — P5: the uniffi golden-fixture audit (T0a)

- [ ] Audit: do the score-bench frozen generations + fixture gates already snapshot `--fix` OUTPUT per domain (the healer's "golden fixture per binding")? Grep `score_bench/fixtures.rs` + the frozen-label e2e for fix-text assertions
- [ ] If YES → record P5 already-satisfied in issue 197 (the Candidate-C outcome, cited as precedent); if NO → price the gap into the plan ledger BEFORE any further claim (snapshot tests are new surface, never smuggled into T0b)

## Phase 6 — gates

- [ ] `cargo test --lib` default posture (count floor + the new tests)
- [ ] `cargo test` the touched targets (fix_verify matrix arms in fix_verify's target; registry tests in lib)
- [ ] `cargo clippy --all-targets -- -D warnings` × postures (default / no-default / all-features)
- [ ] The pins manifest: `--write` once at landing; the manifest diff is the landing's own proof; the include_str! asserts compile against it
- [ ] score_history: the new row's `rustc_version` field verified non-empty in the appended row

## Phase 7 — docs + close

- [ ] Issue 197: mark the landed ports (P1/P2/P3/P4-a ✅, P5 verdict recorded); P4-b cross-ref'd to its follow-up issue; keep the issue open only if something deferred
- [ ] AGENTS.md: one line under the standing-gates section for the pins manifest (`--write` is the count-pin landing pass) + one line for the refusal counters
- [ ] The claim line folds into `**Lease closed:**`; commit `feat:` prefix, Session line in the body; push
