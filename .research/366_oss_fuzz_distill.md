# Research 366 — google/oss-fuzz distill verdict

**Status:** DISTILLED — pending owner decision

- **Source:** https://github.com/google/oss-fuzz — OSS-Fuzz, Google's continuous fuzzing
  service for open source (12.9k stars, ~13.5k commits; infra + thousands of per-project
  integration configs).
- **Pin:** `a380569f8dfef6d142d233725391a6168b41b639` (shallow clone of master at
  inspection; 89 MB; **inspected 2026-11-04**; clone removed at the verdict commit —
  re-clone at this pin for the batch, moved HEAD = re-pin + re-verify every quote).
- **License:** Apache-2.0.
- **Verdict:** **YES B− corpus** [2 own rules across TWO domains — dist +1, sec +1 — both
  suggest-only, both NotBenchable-by-domain per the dial9 split] / **league-lane N/A
  STATED** (no GPU/serving surface anywhere in the repo; zero hits on open-lane keywords)
  / **product coverage: seeds the archive-extract face of the sec InputValidation family**
  (zero archive owners today; modest family, not tokio-tier, but archive extraction is
  ubiquitous in Rust tooling).
- **Queue slot:** `.plans/336_oss_fuzz_distill_batch.md` (nominal Batch 328), STAGED
  behind the open queue.
- Session: zed-m3-ossfuzz-distill, epoch 1793799050

## Source shape (what this repo actually is)

Three surfaces, graded at module level per the thermite precedent:

1. **`infra/cifuzz/`** — the CI fuzzing integration, a real Python codebase (docker
   orchestration, filestore backends, SARIF report emission, stack-trace parsing via
   clusterfuzz, crash artifact handling). This is where untrusted-input surfaces live:
   crash reproducer names, downloaded corpora, remote zip artifacts, stack traces parsed
   into report fields. **The mineable surface.**
2. **`infra/base-images/base-builder/`** — the build tooling: the `compile` script
   (per-language flag stacks), `install_rust.sh`, the `cargo` wrapper (RUSTFLAGS
   expansion hacks). **Build-hardening knowledge for the dist domain.**
3. **`projects/`** (the bulk of the 89 MB) — thousands of per-project Dockerfiles +
   `build.sh`. Per-project ambient config; nothing mineable. Likewise `infra/indexer`
   (C++), `infra/uploader` (a Dockerfile), `infra/chronos` + `infra/tools` +
   `tools/vscode-extension` (ambient tooling; ls-level read only — unverified at mine
   time, no expected loss).

**Zero `.rs` files outside `projects/` — per the transcription law this is never a
kill.** The class knowledge + regime evidence below is transcribed into original Rust
fixtures at authoring.

## Route

Two domains, one batch (the B327 multi-domain precedent):

- **D1 → `dist`** (the tempting WRONG domain: `rust_perf`'s `measurement` category —
  D1 is profiling-ADJACENT but it is manifest build-config advice, which is dist's
  exact charter: "shipped-artifact build-config advice over name-matched Cargo.toml
  manifests"; the measurement category owns measurement-code patterns, not profile
  tables).
- **S1 → `sec` InputValidation** (the tempting WRONG call: folding S1 as a rider into
  `untrusted-path-join-escape` — adjudicated sink-distinct below, same granularity
  precedent as `unvalidated-egress-url` being sink-distinct from `unbounded-read-into-vec`).

## Ranked candidate table

| # | Pattern | Source (verbatim at pin) | Grade | Notes |
|---|---|---|---|---|
| 1 | **`profiling-profile-debug-assertions`** — a profiling/coverage custom profile that keeps debug assertions (inherits dev, or explicit true) measures a DIFFERENT program: asserts fire on profile inputs, overflow checks alter the measured hot path, flamegraphs/coverage diverge from shipped behavior | `infra/base-images/base-builder/cargo` (the coverage-build arm): comment VERBATIM `# we do not want to trigger debug assertions and stops` + `export RUSTFLAGS="$RUSTFLAGS -C debug-assertions=no"`; corroborated by `compile:118-128` VERBATIM `# Rust does not support sanitizers and coverage flags via CFLAGS/CXXFLAGS, so / # use RUSTFLAGS.` and the debuginfo stacks (`-Cdebuginfo=2 -Cforce-frame-pointers` introspector / `--cfg fuzzing -Zsanitizer=${SANITIZER} -Cdebuginfo=1 -Cforce-frame-pointers`) | **B−** | dist, suggest-only-by-domain. Dedup CLEAN (below). Manifest-expressible fix (`debug-assertions = false` + `overflow-checks = false`). Clean regime: fires on profiling-vocabulary profile names inheriting dev or setting debug-assertions = true; silent on `inherits = "release"` (assertions already off), silent on dev/test (assertions are the point there), silent on lib-only manifests (the noise-discipline test). Detector = the dist domain's existing name-matched-manifest scan shape — lowest-friction landing in this verdict. |
| 2 | **archive-extract traversal (zip-slip face)** — a downloaded remote archive extracted into a directory with no member-name screening; untrusted member names join the destination | `infra/cifuzz/http_utils.py::download_and_unpack_zip` VERBATIM: downloads a zip from an HTTP URL into a NamedTemporaryFile, then `zip_file.extractall(extract_directory)` — no member inspection (Python's own zipfile docs warn extractall never inspects `../` members) | **B−** | sec InputValidation, suggest-only. Dedup CLEAN (below). Sink-distinct from `untrusted-path-join-escape` (its detector is a `join()`-call shape; this is archive-API-shaped: `ZipArchive::extract(` without member screening, or a `by_index` loop joining `member.name()` without a components check). Bounded fix-space: per-member name validation (components refusal / mangled extraction) or hardened extract. Regime: untrusted archive source (remote download, user upload) — oss-fuzz's own code is a REAL regime instance (CI infra unpacking remote zips). Crate-version regime must be pinned at authoring from the RUSTSEC set (older zip-crate `extract()` itself vulnerable — re-derive the advisory IDs at authoring, never from memory). |
| 3 | basename-before-join artifact placement — `os.path.basename(crash.input_path)` before joining into the server-chosen artifacts tree | `infra/cifuzz/fuzz_target.py::_save_crash` VERBATIM: `os.path.join(self._target_artifact_path(), os.path.basename(crash.input_path))` | **rider** | Count-neutral FACE on existing `untrusted-path-join-escape` — the sanitize-by-`file_name()` fix form of the same mechanism (untrusted name crossing into a join). Fold into the host rule's notes; never land new. |
| 4 | extract-after-download as a second expansion stage | `http_utils.py` download → unpack chain | **rider** | Count-neutral corroboration of the existing `unbounded-read-into-vec` decompression rider (GzDecoder/ZstdDecoder transparency face): the extract-after-download chain is the same bound-after-every-expansion-stage mechanism at the archive sink. Note only. |
| — | DROP: SARIF CWE rules table | `sarif_utils.py` `SARIF_RULES` | drop | Static CWE-mapping data; no decision content. |
| — | DROP: `redact_src_path` prefix-strip redaction | `sarif_utils.py:150` (`normpath` → strip `/src/` → drop first component) | drop | One-off deployment policy; thin; the "drop first component" step is oss-fuzz-specific (project-name removal). |
| — | DROP: chmod-mode-normalize after artifact copy | `fuzz_target.py::_save_crash` (`os.chmod(target, sibling_mode & 0o777)`) | drop | No attacker regime (multi-user readability, not secret privacy); adjacent to `secret-file-create-without-0600` but a different purpose with no bounded security fix-space. |
| — | DROP: `--remap-path-prefix` coverage reproducibility, `--cfg fuzzing`, `-Zsanitizer=` stacks | `compile` + `cargo` wrapper | drop | RUSTFLAGS/env-level — NOT manifest-expressible; unrepresentable in dist's manifest-fixture shape (dist scans Cargo.toml `[profile]`/`[dependencies]` tables, not `.cargo/config.toml`). |
| — | DROP: `shell=True` absence, list-args git in `repo_manager.py` | cifuzz + `infra/repo_manager.py` | drop | Negative instances — their hygiene is GOOD (zero `shell=True` in cifuzz; git invoked with list args). The corpus rules `shell-string-command-interpolation` etc. already own the fix side. |
| — | DROP: `projects/` tree, `infra/indexer` (C++), chronos, tools, vscode-extension | repo bulk | drop | Per-project ambient config / wrong language-domain / ambient tooling. |

## Dedup-gate evidence (both vocabularies, run at the verdict against the live corpus)

- **D1:** `debug-assertions|overflow-checks|profiling|remap-path` over `src/domains/dist/`
  + `src/domains/rust_perf/` → ZERO rule-owner hits (only unrelated prose: "copy-profiling"
  in entries_03's memcpy rule narrative, "profiling proves" in a rayon entry). Dist's 7
  rule_ids enumerated (shipped-binary-unstripped, shipped-binary-missing-size-profile,
  cli-panic-unwind-tables, pgo-training-set-covers-real-workload-hint,
  member-profile-ignored, large-api-dependency-mostly-unused-hint,
  bench-targets-shared-dependency-cluster-hint) — none covers profile assertion flags.
  Closest kin `pgo-training-set-covers-real-workload-hint` owns PGO TRAINING-DATA
  coverage, not assertion divergence — distinct.
- **S1:** `zip|ZipArchive|extractall|zip-slip|archive|extract(` over `src/domains/sec/`
  → only iterator `.zip()` hits (softmax fixtures). ZERO archive owners.
  `untrusted-path-join-escape` verified = the `components()` refusal at join sinks
  (riir-deployer `48d63f7` shape) — adjacent mechanism, distinct detector + sink;
  in-rule kinship note mandatory at authoring (the reviewer-flippable pattern from the
  uniffi-rs batch).

## GOAT candidate identification

**No candidate is benchable — criterion 1 fails for DOMAIN reasons on both**, which is
the per-rule ANNOTATION case, never a demotion (the dial9 correction; Batch 136
precedent): dist rules are suggest-only manifest advice by construction; sec
suggest-only entries are first-class corpus members. Criteria 2–4 where they apply:
D1 exact semantics = manifest text identity (trivial); magnitude = behavioral
divergence (asserts fire on profile inputs → abort/divergence), NOT a cycle number —
recorded honestly as such; deterministic. S1 = a security correctness class
(path escape = UB/write-outside), not a latency claim. **Grade rests on decision
content × novelty × dedup clearance, the corpus grade stands at YES B−.**

## Product-coverage axis

The sec InputValidation family carries zero archive-extract faces; the `zip`/`tar`
crate ecosystem (cargo, build scripts, asset pipelines) is a broad real surface.
**Follow-up pool for family continuation:** the `zip` crate's RUSTSEC advisory history,
the `tar` crate's unpack hardening history (re-derive both from the advisory-db at
authoring — never from memory), CWE-23 as the canonical class, plus cargo build-script
unpack flows. Modest family; priority set by this seeding, not inflated.

## Lane-intel axis

**N/A STATED.** No GPU/serving surface anywhere in the repo; grep of the league record
and open-lane keywords (prefill/decode/quant/Metal/CUDA) has no mapping. Zero issue
filing owed.

## Yield estimate

2 rules + 2 count-neutral rider faces → **1–2 rules land after ~50% coordinator
falsification** (in line with recent batches).

## Honest caveats

1. **Zero Rust in the source** — everything is transcription (class knowledge + regime
   evidence); fixtures are original Rust per the constraints.
2. **The repo's bulk is ambient** — `projects/` (thousands of per-project configs),
   Dockerfiles, CI configs are corpus-MARGINAL by construction; the verdict grades the
   `infra/cifuzz` + `base-builder` mineable surface only.
3. **S1's provenance is split** — the FIX pattern is standard zip-slip hygiene (public
   knowledge); oss-fuzz contributes the REAL regime instance (CI infra unpacking remote
   zips) and a live demonstration of the unfixed shape. Recorded per the provenance
   chain rule.
4. **No measured numbers in the source** — infra Python + shell; no benchmarking
   culture; D1's mechanism evidence is oss-fuzz's own in-code comment (they hit this
   live in coverage builds) plus the cargo book's profiling convention
   (`inherits = "release"` + `debug = true` is the canonical profiling profile).
5. **Unread surface disclosed:** `infra/chronos`, `infra/tools`, `infra/indexer`
   (C++), `tools/vscode-extension` read at ls-level only — unverified at mine time, no
   expected loss (ambient tooling).
6. **Fast-moving upstream** — the batch lane re-clones at the pin and re-verifies every
   quote before authoring (T0).

## Verdict recording

Artifacts this push (one commit, `docs:` prefix, on develop): `.research/366` (this
note) + `.plans/336` (the batch plan, STAGED) + the DISTILL VERDICT line at the top of
`.distill/001_mining_queue_snapshot.md` + highwaters 365→366 / 335→336 + dual-allocation
gate at allocation. `.raw/oss-fuzz` removed at the verdict commit per the ephemeral
law; the batch lane re-clones at the pin.
