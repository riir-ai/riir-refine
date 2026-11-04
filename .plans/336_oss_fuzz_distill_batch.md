# Plan 336 — Batch 328: the oss-fuzz distill conversion (dist + sec)

**Status:** STAGED — pending queue slot (verdict: Research 366, YES B−; nominal Batch 328)

Source: google/oss-fuzz @ `a380569f8dfef6d142d233725391a6168b41b639` (Apache-2.0,
inspected 2026-11-04; shallow clone removed at the verdict commit — T0 re-clones at
this pin; moved HEAD = re-pin + re-verify every quote BEFORE authoring).

Targets: `dist` 7→8 (+1 suggest-only) and `sec` 38→39 (+1 suggest-only, InputValidation)
+ 2 count-neutral rider faces. Both rules NotBenchable-by-domain (annotate, never
demote). Zero GPU throughout (M3 mining posture).

## Tasks

- [ ] T0 — re-clone `.raw/oss-fuzz` at the pin (shallow); verify HEAD == `a380569f…`;
      re-verify every load-bearing quote from Research 366 at the clone (the cargo
      wrapper's `-C debug-assertions=no` + the `# we do not want to trigger debug
      assertions and stops` comment; `compile:118-133` RUSTFLAGS rows;
      `http_utils.py::download_and_unpack_zip` extractall; `fuzz_target.py::_save_crash`
      basename-before-join). License row confirmed Apache-2.0.
- [ ] T1 — dedup re-verification at authoring (3+ vocabularies, both domains):
      D1 vs all 7 dist rule_ids (closest kin `pgo-training-set-covers-real-workload-hint`
      = PGO training-data coverage, distinct); S1 vs `untrusted-path-join-escape`
      (join-sink vs archive-sink, sink-distinct precedent `unvalidated-egress-url`) +
      `secret-file-create-without-0600` (creation-mode vs member-name, distinct);
      zero new rule_id collisions.
- [ ] T2 — author D1 `profiling-profile-debug-assertions` (dist): original manifest
      fixtures BOTH ARMS standalone-compiled AND executed (buggy:
      `[profile.profiling] inherits = "dev"` + `debug = true`, no assertion keys;
      fixed: `debug-assertions = false` + `overflow-checks = false` added). Trigger =
      `[profile.<name>]` where name matches profiling vocabulary
      (profiling|flamegraph|pprof|coverage|instrument) AND (`inherits = "dev"` OR
      `debug-assertions = true`). Negative arms MANDATORY: `inherits = "release"`
      silent; dev/test profiles silent; lib-only manifests silent (the
      `lib_only_and_complete_profiles_stay_silent` noise-discipline test must hold).
      Pick the DistFamily per corpus.rs taxonomy; notes carry the oss-fuzz provenance
      + the cargo-book corroboration + NotBenchable annotation.
- [ ] T3 — author S1 (sec InputValidation, suggest-only; final kebab rule_id at
      authoring — working name `untrusted-archive-extract-paths`): original Rust
      fixtures using the `zip` crate surface (buggy: `ZipArchive::extract(&dest)` on a
      downloaded archive with no member screen, or a `by_index` loop joining
      `entry.name()` unvalidated; fixed: per-member components() refusal / mangled
      extraction). Detector archive-API-shaped; gate per house convention (dep-gated
      or name-vocabulary-gated — adjudicate at authoring). Regime note: untrusted
      archive source (remote download, user upload); crate-version regime pinned from
      the RUSTSEC advisory set RE-DERIVED at authoring (never from memory); in-rule
      kinship note vs `untrusted-path-join-escape` (reviewer-flippable).
- [ ] T4 — rider faces (count-neutral, zero new rules): add the `file_name()` /
      basename fix-form face to `untrusted-path-join-escape` notes (oss-fuzz
      `_save_crash` cited at pin); add the extract-after-download second-expansion-stage
      corroboration note to `unbounded-read-into-vec`. Notes only — no detector or
      fixture edits unless a zero-cost note slot exists.
- [ ] T5 — standing gates: probe parity (real clippy 1.99.0 pedantic+nursery+suspicious
      +correctness, zero security/safety lints on both buggy shapes, the sec
      convention); clippy `-D` at the four postures (default / no-default / the
      domain feature sets / `--all-features`); default lib count-pinned;
      `scripts/pins_manifest.py --write` ONLY on the real corpus move (sec 38→39,
      dist 7→8) with AGENTS.md / README / `.docs/11_domains` prose in the SAME commit.
- [ ] T6 — retrieval floors re-measured with FULL pre/post attribution (the B327
      shape: pre-batch A/B in a detached worktree) if spans added; score-bench
      appended; freeze tables per P30 (zero clippy_lints delta expected → NOT owed;
      verify, never assume).
- [ ] T7 — release-set binary rebuild + install BEFORE push: `dist` is DEFAULT-ON, so
      a dist corpus edit touches release-set surface (the §0 bin-age law); verify the
      build stamp resolves `-dirty` at the landing.
- [ ] T8 — landing row at the top of `.distill/001_mining_queue_snapshot.md`
      (BATCH 328 LANDED shape, the claim discharged); `.research/366` Status updated
      DISTILLED→landed pointer if owed; commit `feat:` (corpus) per convention, push;
      `rm -rf .raw/oss-fuzz` at close.
- [ ] T9 — claim discipline: BEFORE the first corpus edit, append the claim line
      (Session, epoch, 48 h expiry) under this Status line and push — the push is the
      arbiter (the B324/B327 shape).

## Guardrails

- Fixtures ORIGINAL (never paste oss-fuzz bytes — Apache-2.0 attribution + license
  hygiene + the binary-only distribution rule).
- Suggest-only by construction: NO `--fix` claiming for either rule; advice text in
  the dist entry, refusal-envelope advice in the sec entry.
- Zero GPU; zero sibling-repo mutation; `.raw/` grep-scope hygiene (the clone is the
  source's own code, never coverage evidence).
