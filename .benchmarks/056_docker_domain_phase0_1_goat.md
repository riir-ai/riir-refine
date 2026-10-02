# Bench 056 — `docker` Domain Phase 0 + 1 GOAT

Status: **RECORD — GOAT PASS (2026-08-28); T1.6 adoption + DEFAULT-ON promotion appended the same day**
Executes: [Plan 030](../.plans/030_docker_domain_phase0_batch1.md) · [Proposal 006](../.proposals/006_docker_heal_domain_fifth_healdomain.md)
Oracle: hadolint **2.15.1** (installed for this run) · Docker **29.7.2** (aarch64, desktop-linux)
Box: M3 Max

## Result

**27 rules — 14 Class A, 13 Class B, 14 with bounded fixes** (15 at land time; `cargo-locked-flag` was demoted to suggest-only by the T1.6 adoption run — see the T1.6 section). Plan 030's close
condition was ≥10 bounded; Proposal 006's Phase-2 gate was ≥20 total. Both met,
so Phase 2 is UNBLOCKED (deferred per the plan).

| Gate | Verdict | Evidence |
|---|---|---|
| G1 fixtures | PASS | 14 bounded rules each fire on their buggy fixture AND reproduce the corpus `fixed_code` **byte-exactly** (`every_fixable_rule_reproduces_its_fixed_fixture`) |
| G1 clean-spans | PASS | every rule is silent on its own fixed fixture (2 documented exemptions: rules whose subject is outside the file) |
| G1 negative control | PASS | **zero fixable findings** on `riir-chain/deploy/Dockerfile` + mmorpg `authority-container` |
| G1 refusal gate | PASS | 13 tests; DL3059 / apt-clean / LABEL-merge / QEMU-rewrite all refused |
| Class-A agreement | PASS | every DL-coded rule agrees with LIVE hadolint 2.15.1 (fires on buggy, silent on fixed) |
| Class-B dedup | PASS after 2 reclassifications + 1 refutation (below) |
| G2 latency | 17.05 µs/scan release · 1.24 ms debug (302-byte file, steady state) |
| G3 no-regression | 5-domain router floors re-measured + re-pinned; all retrieval floors held |
| G4 alloc | PASS | scan buffers reach a steady state across 200×4 passes; a clean file produces zero findings, so it allocates none |
| **G5-real** | **PASS** | `edge-wallet-container` healed → `docker build` GREEN |

## G5-real, in detail

`cargo heal --fix --write --domain docker` on a copy of
`riir-chain/cloudflare/edge-wallet-container/Dockerfile` produced exactly two
edits, both span-preserving (every comment untouched):

1. `SHELL ["/bin/bash", "-o", "pipefail", "-c"]` before the `printf | tee` RUN
   — hadolint independently flags that line DL4006.
2. two `--mount=type=cache` mounts (`CARGO_HOME` registry + git) on the
   `cargo build` RUN.

Then: `docker build --check` green on BOTH arms; the real build exited 0, with
step `#11` — the healed cache-mount RUN — genuinely compiling for **167.7 s**
(rustup sync + cargo build in the log, not `CACHED`); image `edge-wallet-healed:g5`
**141 MB**; `/usr/local/bin/edge_wallet_agent` present and executable. The fix
is also **idempotent** (re-running finds nothing left to fix).

The target-dir cache mount was deliberately NOT emitted: its path depends on
`WORKDIR`, and a cache mount over the wrong path silently caches nothing.

## What mine-time falsification actually changed

This is the part worth reading — the plan predicted kills and got them, but
not the ones expected.

**Reclassified B → A (2 rules).** Running the dedup check against the LIVE
binary instead of the Research 086 snapshot found incumbents the snapshot did
not carry: `arg-secret-to-secret-mount` → **DL3064**, and
`entrypoint-shell-form-signal-block` → **DL3025** (which covers ENTRYPOINT,
not just CMD). Both rules stay — the FIX side is still unoccupied — but their
novelty claim was false.

**Refuted (1 rule).** `apt-update-install-same-run` appeared to collide with
DL3009. It does not: a bare `apt-get update` RUN *necessarily* also trips the
apt-lists rule, so any fixture for this rule trips it. Probed directly — a
split whose update RUN already deletes its lists — **hadolint is entirely
silent** while our rule still fires. The cache-staleness axis is genuinely
unoccupied, as Research 086 said. That probe is now a test.

**Killed (4 candidates).**
- `pin-package-versions` (DL3008/DL3018) and `tag-the-base-image` (DL3006):
  detected by hadolint, but **no bounded fix exists** — the correct version is
  a registry fact and this domain makes no network calls. Shipping them would
  be an echo of hadolint's own message.
- `copy-from-references-defined-stage` (DL3022): **hadolint is WRONG here.** It
  cannot see BuildKit named contexts (`--build-context riir-ai=../riir-ai`), so
  it reports two phantom findings on `riir-chain/deploy/Dockerfile`, a correct
  file. We must not reimplement a detector that false-positives on our own tree.
- `consolidate-consecutive-runs` (DL3059): killed as ANTI-MODERN and promoted
  to an explicit **refusal** with its own test.

## A correction to Proposal 006's negative-control claim

The proposal asked for "zero findings" on two in-tree files. Measured, the
honest form is **zero FIXABLE findings**: `authority-container` carries one
real suggest-only citation — its distroless runtime declares no `USER`, so it
runs as root, and distroless ships `:nonroot` variants. That is a genuine
observation about a good file, not a defect claim against it. It is pinned so
it cannot silently grow. The control asserts *no fix is proposed on a
known-good file*, which is the property that actually matters.

Two rules moved to the **caller-gated** class for the same reason, on
principle rather than convenience: `build-stage-missing-buildplatform` (the
build matrix is an invocation fact, `buildx --platform a,b` — nothing in a
Dockerfile states it) and `cargo-locked-flag` (`--locked` fails the build
outright when no lockfile is committed). Silent without evidence; firing with
it — both directions asserted.

## Router annotation (Issue 020) — re-measured, all four docker states

| State | Before docker | With docker | Floor pinned |
|---|---|---|---|
| clippy+perf | 71/101 (70.3%) | **73/101 (72.3%)** | 68/101 |
| +kernel | 89/130 | **94/131 (71.8%)** | 89/131 |
| +sec | 45/101 (44.6%) | **41/101 (40.6%)** | 36/101 |
| +sec+kernel | 78/131 (59.5%) | **82/131 (62.6%)** | 77/131 |

Adding docker **never lowers** the annotation and twice raises it. Its 27
patterns are Dockerfile text, so a distinctive off-vocabulary centroid absorbs
fewer generic Rust spans than a Rust-shaped one does. This is the measured
answer to Proposal 006's open question about a 5th domain's router cost: for
this domain, there is none.

## Two latent defects found en route

1. **`EvalReport::per_domain` was `[DomainMetrics; 3]` while
   `domain_index(Sec)` already returned 3.** A latent out-of-bounds that never
   fired only because the eval fixture set carries no sec-labeled spans. A 5th
   domain would have made it two indices out. Now sized by `DomainId::ALL.len()`.
2. **The Issue 020 floors block was 4 `#[cfg]`-gated consts** keyed on
   `(kernel_opt, sec)`. A 5th domain doubles that to 8, each of whose
   declaration cfg must match its use cfg exactly — the precise shape that
   turned CI Layer 5a red on the sec landing. Replaced with ONE `cfg!`-selected
   function: every arm compiles in every state, so the dead-code failure class
   is gone.

## Honest limits

- ~~**`docker build --check` (Tier 2) is wired conceptually but not into the
  library**~~ — **CLOSED 2026-08-29 (Plan 030 T2.1(a))**: Tier 2 is now
  `src/domains/docker/buildcheck.rs` behind `docker_buildcheck`, with
  `tests/docker_tier2_buildcheck.rs` as the standing gate. See §Tier-2 below.
  Tier 3 (a real build) still ran only in this bench, by hand.
- ~~**The agreement gate covers DL codes only**~~ — **CLOSED 2026-08-29**:
  `class_a_agreement` now dispatches a non-`DL` incumbent to Tier 2. The 4
  BuildKit-check claims were already no longer documentation-only. **Measured 2026-08-28** (see the
  T1.6 section's postscript): each of the 4 fires its named check on the corpus
  `buggy_code` and is silent on the `fixed_code`, against a live
  `docker build --check`. The library still does not call that oracle — wiring
  it in is Phase 2 — so this is a verified claim, not a standing gate.
- **G5-real was measured on ONE file, and on a COPY of it.** That mattered —
  see the T1.6 section below: the copy sat where no `Cargo.lock` was adjacent,
  so a caller-gated rule stayed silent that fires on the tracked file. A GOAT
  measured on a copy cannot see evidence the caller gate derives from the
  file's surroundings.
- **hadolint had to be installed** for this run; it is not a repo dependency.
  Without it the agreement gate SKIPS with a printed line rather than passing.


# ─────────────────────────────────────────────────────────────────────────

# T1.6 — adoption, and what adoption falsified (2026-08-28)

The plan's last open task was the promotion rule's other half: a consumer repo
running the healer against real files, in CI. `riir-chain` is that consumer —
it owns both Dockerfiles the domain was mined against. Running the healer on
the **tracked** files (not a copy) immediately produced a result the GOAT
could not have produced.

## The finding: a bounded fix that breaks the build

`cargo-locked-flag` fired on `cloudflare/edge-wallet-container/Dockerfile` and
`--write` applied it. The image build then **died at resolve**:

```
error: cannot update the lock file /app/Cargo.lock because --locked was passed
exit code: 101
```

The rule's caller gate asked *"is a `Cargo.lock` committed?"* — it is — and
that is **necessary but not sufficient**. `build.sh` stages a hermetic build
context whose workspace members are trimmed to the wallet closure, so cargo
must prune the lock's absent-member entries; `--locked` refuses. The
distinguishing fact — *does THIS context's manifest set resolve against the
committed lock?* — is an invocation fact with no in-file signal.

Why the GOAT missed it: G5-real healed a **copy** in a scratch directory. The
CLI derives lockfile evidence from the repo around the file, and around the
copy there was none, so the gate stayed silent. Two lessons, both cheap:

1. **Measure on the tracked file.** Caller-gated rules are functions of the
   file's *surroundings*; a copy changes the answer silently.
2. **A `--write` lane with no build gate must not carry a rule whose failure
   mode is "the build no longer resolves."** This domain's `--write` has no
   `cargo check` equivalent (the honest oracle is a real `docker build`).

**Decision: `cargo-locked-flag` demoted to suggest-only** — the same contract
as `build-stage-missing-buildplatform` and `arg-secret-to-secret-mount`. It
still fires (with lockfile evidence), it still explains itself, it no longer
writes. **14 bounded / 13 suggest**, down from 15/12.

`deploy/Dockerfile` DOES now carry `--locked`, applied by hand with evidence:
its context is the whole workspace and `cargo fetch --locked --target
x86_64-unknown-linux-musl` resolves clean. That is the shape the rule wants —
a human holding the fact the file cannot state.

## What adoption changed in riir-chain

| File | Change | Evidence |
|---|---|---|
| `cloudflare/edge-wallet-container/Dockerfile` | `SHELL [... -o pipefail ...]` before the piped stub-manifest RUN | hadolint DL4006 **gone**: the file is now hadolint-CLEAN (was 1 finding) |
| " | 2 `--mount=type=cache` mounts on the `cargo build` | `docker build` green; the healed RUN compiled 167.7 s cold, **21.4 s** warm |
| " | `USER 65534:65534` in the runtime stage (the suggest-only `runtime-stage-missing-user` citation, taken) | container serves `/health` and `docker exec id` reports `uid=65534(nobody)` — measured, not argued |
| `deploy/Dockerfile` | `cargo build --locked` | `cargo fetch --locked --target x86_64-unknown-linux-musl` exit 0 |

The pipefail fix is the one that matters most and reads smallest: that RUN
pipes `echo | tee` to write stub manifests. Without `pipefail` a failed stage
yields a **silently broken build closure** rather than a failed build.

## The standing gate (the actual T1.6 deliverable)

`riir-chain/scripts/dockerfile_heal_gate.sh`, wired into `.github/workflows/rust.yml`'s
`standalone` job (and therefore auto-picked-up by `ci_local.sh`, which derives
its step list from the workflows). Two tiers, neither silently lenient:

- **Tier 1 — hadolint** against a pinned ledger. Installed in CI, so this tier
  is live there. Fails on an unpinned finding, on a count change, **and on a
  pin that no longer fires** (a stale pin is how a ledger rots into decoration).
  Ledger today: `deploy/Dockerfile` DL3018 ×1 (version pinning is a registry
  fact) and DL3022 ×2 (hadolint cannot see BuildKit named contexts — the
  false-positive that killed the rule at mine time, now written down where the
  next reader will look).
- **Tier 2 — `cargo heal --fix --domain docker`**: any bounded fix being
  *available* means the tree drifted off the healed state. Suggest-only
  citations are pinned the same way.

Verified by falsification, not by a green run — each defect class was
introduced and the gate was confirmed to fail:

| Injected | Gate response |
|---|---|
| delete the `SHELL` pipefail line | FAIL ×2 (unpinned hadolint DL4006 **and** Tier-2 bounded fix available) |
| add a `MAINTAINER` line | FAIL ×2 (unpinned DL4000 + `maintainer-to-label` available) |
| pin the apk versions (removes DL3018's cause) | FAIL — "pinned DL3018 no longer fires; delete its ledger row" |
| `HEAL_BIN=/nonexistent` | SKIP, PASS-with-skips; `--strict` → exit 1 |

**En-route fix in the consumer:** `ci_local.sh` aborted on every Mac —
`mapfile` is bash 4+ and macOS ships bash 3.2 as `/bin/bash`. Since riir-chain's
CI is main-only, "run `ci_local.sh` before pushing" is the *documented*
mitigation for work landing on `develop` — and it had never been available
where most of the work happens. Replaced with a portable read loop; it now
derives and runs 15 steps.

## Promotion

Both halves of the promotion rule are met — GOAT PASS **and** measured consumer
pull — so `docker` is **DEFAULT-ON** as of 2026-08-28. `docker_verify` stays
opt-in (it shells out to an external binary; the sec precedent).

The promotion also removes a fragility the gate had to work around: a stock
`cargo-heal` rejected `--domain docker`, so Tier 2 could only SKIP for anyone
who had not rebuilt with the feature. It is now in every default build.

Residual risk, stated plainly: default-on means `--fix --write` can edit
Dockerfiles. Bounded to what it was already bounded to — files matched by NAME
(`Dockerfile`, `<n>.Dockerfile`, `Dockerfile.<n>`), every bounded rule
fixture-verified byte-exact and idempotent, and the one rule whose failure mode
was a broken build is no longer a bounded rule.


## Postscript — the 4 BuildKit-check claims, measured (2026-08-28)

Oracle: `docker build --check`, Docker **29.7.2** / buildx **v0.36.1-desktop.1**,
run over the corpus fixtures verbatim in a throwaway context.

| Rule | Claimed check | `buggy_code` | `fixed_code` |
|---|---|---|---|
| `instruction-casing` | `ConsistentInstructionCasing` | fires ✅ | clean ✅ |
| `legacy-key-value-format` | `LegacyKeyValueFormat` | fires ✅ | clean ✅ |
| `no-empty-continuation` | `NoEmptyContinuation` | fires ✅ | clean ✅ |
| `cmd-entrypoint-exec-form` | `JSONArgsRecommended` | fires ✅ | clean ✅ |

4/4 — the incumbent names in the ledger are the incumbent's actual names, and
each fix genuinely silences the check it claims. What this does NOT do is make
the oracle a standing gate: the library calls hadolint, not `docker build
--check`, so this is a one-shot verification of a documented claim. The
standing tier-2 oracle stays Phase 2, where the plan put it.


## Tier-2 — the standing `docker build --check` oracle (Plan 030 T2.1(a), 2026-08-29)

The postscript above verified 4 claims once, by hand, and said so. That is
exactly the shape that rots: a measured fact with no gate behind it survives
until the corpus moves and then quietly stops being true. Phase 2's
highest-value item was to close it, and it is closed —
`src/domains/docker/buildcheck.rs` behind `docker_buildcheck`, dispatched from
`class_a_agreement`, gated by `tests/docker_tier2_buildcheck.rs`.

Live re-run, Docker **29.7.2** / buildx **v0.36.1-desktop.1**:

```
tier2 OK: cmd-entrypoint-exec-form -> JSONArgsRecommended
tier2 OK: instruction-casing       -> ConsistentInstructionCasing
tier2 OK: legacy-key-value-format  -> LegacyKeyValueFormat
tier2 OK: no-empty-continuation    -> NoEmptyContinuation
test result: ok. 4 passed; 0 failed — finished in 5.86s
```

### Verified by falsification, not by the green run

A gate that has only ever passed has demonstrated nothing about what it
catches. Three defect classes injected into the corpus, each reverted after:

| Injection | Expected verdict | Result |
|---|---|---|
| `legacy-key-value-format.fixed_code` keeps `ENV RUST_LOG info` | `StillFiresAfterFix` | **FAILED correctly** |
| `instruction-casing.buggy_code` uppercased to `RUN` | `OracleSilentOnBuggy` | **FAILED correctly** |
| ledger cross-ref mistyped `NoEmptyContinuationTypo` | `OracleSilentOnBuggy` | **FAILED correctly** — and the set-pin test failed too (2 of 4), so the ledger has two independent guards |

### Three measured facts the design rests on

None are in the upstream documentation; each was probed directly before a line
of the parser was written.

1. **There is no structured output.** `--progress=rawjson` emits only
   `vertexes` and `statuses`; the check report is printed as plain text
   *outside* the JSON stream. Text parsing is the only interface, not a
   shortcut taken for convenience.
2. **Findings go to stdout, build errors go to stderr** — cleanly separable,
   which is what makes fact 3 handleable at all.
3. **A resolve failure silently truncates the finding set.** A two-defect file
   (`from` lowercase + legacy `ENV`) on an unresolvable base reported **one**
   warning, not two, and exited **1** — the same exit code as a healthy run
   that found warnings. Shape and exit code both fail to distinguish partial
   from complete. `CheckRun::Errored` therefore **discards** the findings it
   received rather than reporting a subset that reads like a near-clean file.
   `tests/docker_tier2_buildcheck.rs::a_resolve_failure_is_errored_not_a_short_finding_list`
   pins it, and is written so that a future BuildKit which stops truncating
   passes for the right reason instead of silently going green.

### Cost, and why this one does NOT get promoted

`docker` went default-on because the gain was modelless and the dependency was
nothing. Tier 2 is the opposite trade: it needs a **running daemon**, and it
resolves source metadata **over the network** for every fixture not based on
`scratch`.

| Fixture base | Cold | Repeat |
|---|---|---|
| `FROM scratch` | 0.195 s | 0.195 s |
| `FROM alpine:3.20` | 1.29 s | **2.17 s** |

The registry round-trip does **not** warm — the repeat run paid it again, and
paid more. So the suite is ~9.6 s, dominated by four `alpine`-based fixtures,
and the tier never runs in the heal path: only in gates and benches. Promotion
would put a daemon requirement into a default `cargo test`, which is not a
GOAT question at all — it is a dependency-class question, and the answer is no.
The availability probe is itself the oracle on `FROM scratch`, an end-to-end
capability test that costs 0.2 s once per process and touches no registry.


## T2.1(b) — the suggest-only family, and what measuring it changed (2026-08-29)

Four rules, chosen by whether they **discriminate on the real in-tree images**
rather than by working down the Research 087 seed table. **27 → 31 rules;
bounded stays 14** — nothing new touches the `--write` lane, because each
rule's correct action depends on a fact the Dockerfile does not carry, and a
`--write` lane guessing at one would be `cargo-locked-flag` again.

| rule | family | fires on | silent on |
|---|---|---|---|
| `base-tag-not-digest-pinned` | base-choice | edge-wallet (debian), authority (distroless) | deploy (scratch — no image to pin) |
| `copy-link-artifact-rebase` | cache-perf | authority | deploy (scratch), edge-wallet (taken) |
| `musl-allocator-perf-annotation` | rust-idiom | deploy | edge-wallet, authority (glibc) |
| `healthcheck-tool-absent-in-runtime` | signal-pid1 | — | all three (none declares a HEALTHCHECK) |

### Two rules were NARROWED by measurement, after being written

Both narrowings were forced by an existing gate, not noticed by reading.

**`healthcheck-tool-absent-in-runtime` cites the TOOL, not the FORM.** The
Class-B "unoccupied territory" gate failed it against DL3025. Probed live:

```
$ hadolint  # HEALTHCHECK CMD curl -f http://localhost:8080/health || exit 1
DL3025 warning: Use arguments JSON notation for CMD and ENTRYPOINT arguments
$ hadolint  # HEALTHCHECK CMD ["/usr/bin/curl", "-f", "http://localhost:8080/health"]
(silent)
```

So the shell FORM is occupied and the exec-form probe on a `curl`-less base is
not: well-formed, passes every linter, and can never run. Detecting only the
unoccupied half is the Research 086 dedup discipline doing its job.

**`copy-link-artifact-rebase` stands down on `scratch`.** It first fired on
`riir-chain/deploy` *and* on the G4 clean-file fixture. There is no parent
filesystem on scratch to rebase onto, so `--link` buys nothing there — and a
rule that fires on every well-written multi-stage Dockerfile carries no
information. The G4 clean-file gate is what caught it.

### A prediction the controls corrected

`authority-container` was predicted to draw neither new base citation, on the
theory that `distroless-distro-suffix-pin` would absorb the digest one. Wrong:
that rule only fires on a base MISSING the `-debianNN` suffix, and
`gcr.io/distroless/cc-debian13` carries it. The citation is legitimate and is
now pinned at the measured 3.

### The bug this batch found in the domain itself

Adopting `copy-link` into riir-chain meant adding the `# syntax=` line `--link`
requires — and writing Docker's own canonically recommended form,
`# syntax=docker/dockerfile:1`, made `missing-syntax-directive` fire on a
now-correct file.

`SyntaxVersion::from_directive` parsed a bare major as `1.0`. A minor-less tag
is a **moving label** — always the latest release of that major, so it
satisfies every floor in it. Measured rather than inferred from the docs:

```
# syntax=docker/dockerfile:1
FROM scratch
COPY --exclude=*.log ctx/ /data/     # --exclude is 1.19-only
→ docker build --check: Check complete, no warnings found.
→ docker build:         exit 0
```

Reading `:1` as `1.0` got the ordering backwards: the healer would demand
`1.4` on a file already carrying something strictly newer, and refuse
V1_4-floor fixes the frontend supports. An explicitly pinned `1.0` still reads
as `Builtin`; only the minor-less form is latest-of-major. **This is what
adoption buys that a fixture cannot** — the same lesson as T1.6's
`cargo-locked-flag`, arriving from the opposite direction: there, adoption
falsified a rule; here, adoption falsified the *domain's model of the
frontend*.

### Adoption: one taken, two pinned — the decisions, not the deferrals

| citation | decision | why |
|---|---|---|
| edge-wallet `copy-link-artifact-rebase` | **TAKEN** | rebuilt GREEN in 44.7 s; container verified serving `/health` as `uid=65534(nobody)`; binary root:root 755; image 141 MB, unchanged. Safe here because nothing about the copy is order-dependent |
| edge-wallet `base-tag-not-digest-pinned` | **DECLINED on a security trade** | a digest is reproducible but FROZEN, and this repo runs no renovate/dependabot. For an image holding a signing key, a base quietly accumulating CVEs is the worse risk. The ledger row FLIPS the day a bump policy exists |
| deploy `musl-allocator-perf-annotation` | **NOT INVESTIGATED**, said plainly | the annotation is unmeasured here. Worth measuring because `.docs` claims a ≤200 µs tx-path budget. What would settle it is named in the row: the tx-path bench against the musl artefact ± a mimalloc `#[global_allocator]`. Swapping a consensus daemon's allocator on reasoning alone is not something that repo does |

`riir-chain/scripts/dockerfile_heal_gate.sh` went RED on both new citations
before any row existed — the closed loop working exactly as designed — and is
green again with the rows in place.
