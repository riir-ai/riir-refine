# Plan 329 — engine distill T0b: the Mozilla-quartet methodology port (issue 197)

**Status:** T0b LANDED 2026-11-03 at `7d0752b07` (zed-m3-24c8b43a; owner reassignment from the amo lease, claim per issue 197); P4-b filed as issue 198; P5 audit verdict recorded in issue 197

Executes [issue 197](../.issues/197_engine_mozilla_methodology_port.md) — the blaze-style methodology port (uniffi checksum-handshake × supply-chain vet-aggregate × sccache mock_command × sccache cache-key doctrines onto the healer). T0b is test/script-only: zero ENGINE behavior change, zero corpus change; the ONE report-behavior delta (the score_bench pooling split in P4-a's G1) is labeled in Phase 4.

## Phase 0 — claim + substrate re-verify

- [x] Append the claim line to issue 197 (`**Claimed:** <session>, <epoch>, expires <ISO-UTC>`), push before implementing (commit `7702625e9`; the amo lease superseded by owner directive, agent-mail 01a19ed3)
- [x] `py ../katgpt-rs/scripts/dual_allocation_gate.py` (0/0/0 expected — measured 0/0/0)
- [x] Re-run the substrate greps (issue 197 §substrate): the six `pub fn with_cargo_bin` verifiers ✓ unchanged, `traj_store.rs` zero-toolchain ✓ re-verified, the score_history row union schema + the generation-less count ✓ re-measured **43/217 at claim time** (the issue's 45 was the filing-time count; 174 carry `generation`), the `src/domains/` population ✓ unchanged (8 dirs + probe_witness.rs). Nothing moved scope.
- [x] Housekeeping found en route: `.plans/327`'s stale `**Claimed:**` line (the malformed-lease-blind class — claim_lease_check exit 1 BLIND) folded into its `**Lease closed:**` row in the same commit

## Phase 1 — P1: the corpus pins manifest (T0b)

- [x] `scripts/pins_manifest.py --write` emits `.benchmarks/pins_manifest.json`: derived from the compiled corpus builders via the `pins_manifest::dump_pins_manifest` test (`src/pins_manifest.rs`; build posture `--features kernel_opt,sec,rust_perf_bench`) — live truth at landing: clippy_lints 112 · rust_perf 292/14 categories/125 benchable/167 notbenchable · kernel_opt 1051 + witness 15/15/8 · sec 35. Deviation from the sketch, disclosed: the `unannotated` key is NOT derivable (the corpus has no annotation field; `bench_spec None` IS the notbenchable bucket by the pins' own semantics) — the manifest carries `notbenchable` and drops `unannotated`
- [x] Bare run diffs and exits 1 on drift naming every diverging site (measured: caught README's STALE "100 corpus lints" vs live 112 — a live B308-class catch by the gate's first run)
- [x] DRY (the round-1 shape): the count asserts READ the manifest via `crate::pins_manifest::count` (include_str in `src/pins_manifest.rs`) — rust_perf total+categories, the `rust_perf_bench`-gated 125/167 pair, clippy 112, sec (FIRST count pin — the domain had none), kernel total ×35 witness sites (all verified live-total pins, each batch's re-pin burden now one manifest diff) + both grouped sums + KERNEL_CATEGORIES, router experts/activity_dim. The asserts stay speed bumps; `--write` can never silence a red assert
- [x] The PROSE half: the script gates AGENTS.md (4 rows) + README.md (4 rows) + `.docs/11_domains/rust_perf.md`'s live "now N rules across N categories" row against the same manifest; historical rows and era-witness pins stay untouched history; `EXEMPT_PROSE` starts EMPTY (deletions-only ratchet in place)
- [x] Home: wired as `scripts/ci_feature_guard.sh` **Layer 7** (header updated; the pins build is warm after Layers 2–5)
- [x] G1 canary (a): drift one pin row in a fixture copy → red names it (`--self-test` canary a, PASS — the B308 class)
- [x] G1 canary (b): change the corpus WITHOUT `--write` → red on the stale manifest (`--self-test` canaries b+c, PASS; the structural half is the include_str assert itself)
- [x] Ratchet: `EXEMPT_PROSE` takes deletions only (documented in the script)

## Phase 2 — P2: the failure-injection matrix (T0b)

- [x] Shared fake-cargo helper in `tests/fix_verify_failure_matrix.rs` (pid-suffixed temp dirs, marker-file state machines)
- [x] The matrix runs over `fix_verify` + `compile_oracle` MINIMUM: 6 fix_verify arms + 3 CheckRunner arms, each asserting loud refusal + tree restored + UNVERIFIED stats + the per-class counter exactly 1
- [x] The other four verifiers listed covered-or-deferred, by name + reason (test-file header): rustc_errors/driver.rs + rust_perf/{bench,verify}.rs + clippy_lints/verify.rs all delegate to the SAME `CheckRunner`, so the run_check tightening is inherited by all six (covered at the seam; their loop-level postures keep their existing posture tests)
- [x] Per-class refusal COUNTERS in the verify stats: `VerifyReport.refusals: RefusalTally` (7 fixed-order fields + `is_empty`/`rollup_line`/`add`), classified at every refusal site; the run roll-up prints `verify refusals: …` when non-zero (silent when zero), aggregated at both heal.rs verify call sites. ⛔ Output-only, as labeled
- [x] G1: the mutation arm (`completion_sentinel_accepts_a_complete_pass`) proves the two-sided contract — the mutant exit-0+artifact-JSON pass is ACCEPTED (kept_edits 1, tally empty) while every ambiguous shape refuses
- [x] The two existing arms (`kill_path`, `loop_exhaustion`) stay untouched — the matrix extends, never rewrites. Deviation, disclosed: each preserved test's fake cargo gained ONE artifact-JSON echo on its baseline arm (physics: the zero-JSON tightening refuses an empty-success baseline; a shape real cargo never produces under `--message-format=json`). Scenarios and every original assertion unchanged
- [x] ⛔ LABELED FAIL-CLOSED DELTA (the P2 fix itself): `CheckRunner::run_check` now REFUSES zero-JSON output regardless of exit status (harmonized with run_clippy's existing posture) — before this, a fake/wrapper cargo exiting 0 with empty or non-JSON output read as a CLEAN pass keeping the edit. Refusal-class only (a refusal can only restore + unverify, never keep); the spawn-failure prefix is a pinned const (`CARGO_SPAWN_FAIL_PREFIX`)

## Phase 3 — P3: the domain-wiring registry gate (T0b)

- [x] ONE central `pub(crate)` const `DOMAIN_REGISTRY` in `src/domains/mod.rs` (no `inventory`/`linkme` — zero new deps, zero public-API surface); rows carry dir + feature + roll-up label + canonical key
- [x] Test 1: every `src/domains/*/` directory with a `mod.rs` ∈ registry ∪ EXEMPTIONS — the walk's predicate excludes FILES (probe_witness.rs noted); the exemption list is by-name with a reason per entry, starting exactly `dist/` + `docker/` (scan-only domains — no rule corpus, nothing to pin); the deletions-only ratchet on the const
- [x] Test 2: the heal.rs roll-up label map keys ⊆ registry ∪ exemption labels (via `DomainId::ALL` labels; latent_retrieval-gated — `score_bench` implies it)
- [x] Test 3: score_bench domain rows ⊆ registry canonicals (via the ungated `demonstrability::T3_DOMAINS` — consumed, not replicated)
- [x] G1 canary: the pure classifier test feeds a synthetic unregistered dir and asserts it is named (no fs writes — never a dir inside src/)
- [x] Ratchet: the exemption list takes deletions only
- [x] ⛔ NO heal.rs refactor — the gate reads the label map, it does not move the 20k-line bin
- [x] (en route) the 5th test the -D gate forced: every registry `feature` name must exist in Cargo.toml (`registry_feature_names_exist_in_the_manifest` — registry fields are READ, never dead)

## Phase 4 — P4-a: score_history provenance axes (T0b with ONE labeled report delta)

- [x] `rustc_version` + `clippy_driver_version` added to the `HistoryRecord` append row (additive `Option<String>` law, `#[serde(default, skip_serializing_if)]`); readers tolerate old rows; the Warm-tier mirror drops them (the instrumentation law) — `traj_store_tests.rs` struct literal extended
- [x] The generation-less class folds in HERE: `default_generation()` → **0 = the LEGACY UNPINNED class** (rows written before Plan 092's tag, and only those); a pinned generation is 1..=255; `default_score_generation()` flipped to 0 (the store carries the same class); every "deserialize as 1" doc claim updated
- [x] ⛔ THE LABELED REPORT DELTA: `detect_saturation` anchors each series on the LAST row's toolchain class and never pools rows differing on (rustc_version ∨ clippy_driver_version ∨ generation); generation 0 never enters a pinned series. G1 tests: split-on-toolchain, generation-0 exclusion, None-pools-with-None stability, roundtrip + skip-when-absent
- [x] The landing's own score-bench run appended row #220 carrying `rustc_version: rustc 1.99.0 (b940084d7 2026-09-28)` + `clippy_driver_version: clippy 0.1.99 (b940084d7e 2026-09-28)` — the fields' first live evidence. The honest reset observed and disclosed: the pinned-toolchain row anchors a FRESH gen-1 series (`saturation: gen-1 no verdict (1/6 runs)`) until 6 same-toolchain rows accumulate; the v1 SVG shrinks by pre-gen-tag rows
- [x] heal.rs labeled-delta comments at the two generation-pooling sites (`admission_gate`'s v1 baseline, `gen_series`) — an all-legacy history now refuses admission (honest); `--score-report` re-renders share `gen_series`
- [x] P4-b filed: `.issues/202_traj_store_toolchain_key.md` (the migration rule carried verbatim; behavior change with its own gates, NOT this plan; renumbered 198→202 2026-11-04 — the wetrun issue's first allocation won the 198 collision)

## Phase 5 — P5: the uniffi golden-fixture audit (T0a)

- [x] Audit verdict: **already satisfied, in a stronger form than byte-snapshots** — recorded in issue 197 (the Candidate-C outcome cited as precedent). Evidence: `all_buggy_codes_fire_their_lint` (every clippy corpus entry's `propose_fix` fires + re-parses, per landing) · per-rule proposers assert fix-OUTPUT SHAPES (`out.contains("if b { 1 } else { 0 }")`, clippy's arm-order-preserving emission pinned, 3-arm negative control) · the corpus `fixed_code` fields ARE the golden fix texts, BLAKE3-frozen (tamper test at corpus.rs:149) · the score-bench frozen fixture set + frozen-label drift detection (the checksum-handshake analog at the bench axis). The honest gap vs uniffi's letter — no whole-output byte-equality per domain — is DELIBERATE (fix text varies in span placement; the repo's law is shape-pinned, not byte-pinned); a byte-snapshot lane stays ledgered as NOT WORTH ITS COST unless a cross-toolchain fix-text drift class appears

## Phase 6 — gates

- [x] `cargo test --lib` default posture: **2807 passed, 0 failed**
- [x] The pins posture (`kernel_opt,sec,rust_perf_bench`): **3668 passed, 0 failed** (the full rewired pin set + the registry gates)
- [x] The failure matrix: 11 passed (`--features fix_verify`); fix_verify lib 41 + compile_oracle lib 10 (subagent-verified at integration)
- [x] `cargo clippy --all-targets -- -D warnings` × postures: default **0**, `--all-features` **0**, `--features latent_retrieval` **0**, `--no-default-features` **0** — three committed pre-existing lints fixed en route (tests.rs doc_markdown backticks; retrieval_eval.rs match_same_arms merged after the Batch-325 re-pin made both arms identical, both histories kept; the sec proposers' unnecessary_hashes ×17, quote-safe conversion only)
- [x] The pins manifest: `--write` once at landing (the diff IS the landing proof); bare run green; `--self-test` 3/3
- [x] score_history: row #220's `rustc_version` verified non-empty (above)

## Phase 7 — docs + close

- [x] Issue 197: the landed ports marked (P1/P2/P3/P4-a ✅, P5 verdict recorded); P4-b cross-ref'd to issue 198; lease folded
- [x] AGENTS.md: the pins-manifest line (Build Commands) + the refusal-counters line (the `--fix` bullet)
- [x] The claim line folds into `**Lease closed:**`; commit `feat:` prefix, Session line in the body; push
