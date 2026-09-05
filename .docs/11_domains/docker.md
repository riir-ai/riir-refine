# The `docker` Domain — Dockerfile Bounded-Fix Healing + Retrieval

**Feature:** `docker` (**DEFAULT-ON since 2026-08-28**, zero new dependencies)
**Oracle ladder:** `docker_verify` (Tier 1 — implies `docker`; needs `hadolint`
on PATH) and `docker_buildcheck` (Tier 2 — implies `docker_verify`; needs a
running Docker daemon). Tier 2 is `docker build --check`, BuildKit's own
linter, and it owns the 4 Class-A rules whose incumbent is a check NAME rather
than a `DL` code. **Landed 2026-08-29 (Phase 2)**: those 4 claims used to be a
hand-measured Bench 056 postscript and are now a standing gate
(`tests/docker_tier2_buildcheck.rs`), verified by falsification — three
injected defect classes (fix does not silence the check; the oracle does not
see the claimed defect; a mistyped ledger cross-ref) each turn it red with the
right verdict.
**Issue 071 (2026-09-06): the gate went red and the filing misread it as
"Docker 29.7.2 removed the check" — live probes proved the check set had not
drifted at all.** The real failure: the three check-only fixtures carried
`FROM alpine:3.20`, so the tier-2 oracle paid a registry round-trip per call,
and on a registry-blocked box each resolve DeadlineExceeded (~60 s × 6 calls ≈
the recorded 399 s red) → `Unavailable` verdicts, which the gate honestly
refuses. Repair, not re-pin: (1) the three check-only fixtures now ride
`FROM scratch` — the check is a frontend-parse verdict, so the base
contributed nothing, and the tier is registry-free (399 s red → 2.7 s green,
measured on the same daemon); (2) `classify()` gained the measured
completion-marker backstop (a run whose stdout never prints `Check complete`
lands `Errored`, never a fake `Ran(empty)`); (3) the gate's failure output
names `docker version` and separates could-not-check from true disagreement.
**Plan:** [030](../../.plans/030_docker_domain_phase0_batch1.md) ·
**Proposal:** [006](../../.proposals/006_docker_heal_domain_fifth_healdomain.md) ·
**Research:** [086](../../.research/086_dockerfile_detection_inventory_hadolint_buildkit.md) (dedup ledger) + [087](../../.research/087_docker_heal_seed_inventory.md) (seed inventory)

The fifth `HealDomain`, and the first with **no `syn`**: Dockerfiles are read
by a hand-written stanza scanner. It is also the first domain whose oracle is
an **external binary** (hadolint) rather than a compiler.

## Why it exists (the measured delta)

hadolint (71 DL rules) and `docker build --check` (21 checks) are both
**detection-only** — re-verified live against hadolint 2.15.1 on 2026-08-28.
Fixing is unoccupied. So the domain splits:

| Class | Count | Contract |
|---|---|---|
| **A — fix overlay** | 14 | the incumbent detects, we fix. Our detector must AGREE with hadolint on the fixture (`docker_verify`) |
| **B — novel detection** | 17 | both incumbents silent; we detect AND fix or annotate |

**31 rules total, 14 with bounded fixes.** (Plan 030's close condition was
≥10 bounded; Proposal 006's Phase-2 gate was ≥20 total. Both met.)

### Phase 2 T2.1(b) — the suggest-only family (2026-08-29)

Four rules that all CITE and none of which fix, because each one's correct
action depends on a fact the Dockerfile does not carry. That is not a
shortfall — a `--write` lane guessing at any of them would repeat the
`cargo-locked-flag` break, where a rule with a plausible in-file signal broke
a real build on a fact that lived outside the file.

| rule | family | what it says | why it cannot fix |
|---|---|---|---|
| `base-tag-not-digest-pinned` | base-choice | a runtime base carried by tag moves under you | the digest is a registry fact (no network calls), *and* pinning freezes patches, so it is only correct next to a bump policy |
| `copy-link-artifact-rebase` | cache-perf | the artefact copy invalidates everything above it without `--link` | `--link` changes semantics — the copy cannot see parent layers, and whether a copy is order-dependent is not in the file |
| `musl-allocator-perf-annotation` | rust-idiom | musl's allocator is not glibc's, and a service profiled on one does not carry that profile to the other | the `global_allocator` lives in Rust source and Cargo.toml, which this domain does not edit |
| `healthcheck-tool-absent-in-runtime` | signal-pid1 | an exec-form `curl`/`wget` probe on scratch/distroless can never run — the container reports UNHEALTHY forever | which endpoint a static checker should poll is an application fact |

Two boundaries in that set were drawn by MEASUREMENT rather than by taste,
and both narrowed a rule that had already been written:

- **`healthcheck-tool-absent-in-runtime` cites the TOOL, not the FORM.** A
  shell-form `HEALTHCHECK` is hadolint's DL3025 — verified live, it fires.
  What no incumbent sees is an *exec-form* probe invoking `curl` on a base
  that ships no `curl`: well-formed, passes every linter, never runs. Citing
  the form too would have been an echo, and the Class-B "unoccupied
  territory" gate caught it.
- **`copy-link-artifact-rebase` stands down on `scratch`.** There is no parent
  filesystem there to rebase onto, so `--link` buys nothing. Without that
  guard the rule fired on `riir-chain/deploy` and on the G4 clean-file
  fixture — a rule that fires on every well-written multi-stage file carries
  no information.

The controls now discriminate cleanly: `deploy` (musl on scratch) draws only
the allocator annotation; `edge-wallet` (glibc on debian) draws the digest
citation; `authority-container` (distroless) draws the digest and `--link`
citations and not the allocator one.

### The bug this batch found in the domain itself

Adopting `copy-link-artifact-rebase` into riir-chain meant adding the
`# syntax=` line `--link` requires — and writing Docker's own canonically
recommended form, `# syntax=docker/dockerfile:1`, made
`missing-syntax-directive` fire on a now-correct file.

`SyntaxVersion::from_directive` was parsing a bare major as `1.0`. But a tag
with no minor is a **moving label**: it always resolves to the latest release
of that major, so it satisfies every floor in it. **Measured, not inferred
from the docs:** under `# syntax=docker/dockerfile:1`, `COPY --exclude=` — a
1.19-only feature — passes `docker build --check` and builds green. Reading
it as `1.0` got the ordering exactly backwards: the healer would demand `1.4`
on a file already carrying something strictly newer, and would refuse
V1_4-floor fixes the frontend supports. An explicitly pinned `1.0` still reads
as `Builtin`; only the minor-less form is treated as latest-of-major.

## Usage

```bash
cargo heal --suggest --domain docker path/to/Dockerfile   # cite findings
cargo heal --fix     --domain docker path/                # show bounded fixes
cargo heal --fix --write --domain docker path/            # apply them
```

Dockerfiles are matched by NAME — `Dockerfile`, `<name>.Dockerfile`,
`Dockerfile.<name>` — because a suffix match misses the third spelling.

## The three things that keep it honest

### 1. Refusals are data, not prose

Some incumbent advice is ANTI-MODERN and the healer must move AWAY from it.
`refuses()` names four, and `tests/docker_refusal_gate.rs` fails the moment a
proposer emits one:

| Refusal | Why |
|---|---|
| `consolidate-consecutive-runs` (DL3059) | separate `RUN`s are separate BuildKit cache layers; merging makes every rebuild redo the merged work |
| `apt-get-clean` | redundant under the default `docker-clean` config; the useful half is DL3009, which we DO fix |
| `merge-label-instructions` | pre-BuildKit layer-count lore; LABELs are metadata, not layers |
| `rewrite-qemu-native-to-buildplatform-cross` | BOTH are valid — the in-tree `deploy/Dockerfile` builds QEMU-native deliberately, while cross is 5.2x-7x faster on compile-heavy trees. Annotated by a decision table, never rewritten |

**The one sanctioned merge** is the adjacent `apt-get update` / install pair.
That is not a DL3059 loophole: a cached update layer paired with a fresh
install is a *correctness* bug, the merge is scoped to that pair alone, and
the refusal gate asserts it does not generalize.

### 2. The caller gate — evidence, never guesses

Five rules need facts that are not in the Dockerfile. They stay **silent**
without evidence, which is what keeps known-good files clean:

| Rule | Evidence required | Why it cannot be guessed |
|---|---|---|
| `cargo-locked-flag` | a committed `Cargo.lock` — **necessary, not sufficient** (suggest-only since the T1.6 adoption run; see below) | `--locked` fails the build outright when the lock does not match the context |
| `workspace-single-member-build` | a multi-member workspace | which member to build is not in the file |
| `dockerignore-missing` | the context dir listing | the fix is a NEW FILE, not an edit |
| `build-stage-missing-buildplatform` | a multi-platform build | the build matrix is an *invocation* fact (`buildx --platform a,b`) |
| `arg-secret-to-secret-mount` | — (suggest-only) | the fix requires the caller to pass `--secret` |

The CLI derives what it can from the repo around the file (nearest
`Cargo.toml`'s `Cargo.lock` / `.dockerignore`) and leaves the rest `None`.

### 3. `--write` has no build gate — know this before using it

The Rust domains gate every written edit behind a `cargo check` (`--verify`).
This lane has no equivalent, because the analogous oracle is a real
`docker build`: minutes to tens of minutes, a daemon, and a build context. It
is deliberately not run per fix.

What does protect a write: every bounded rule reproduces its corpus fixture
**byte-exactly**, the proposers refuse more than they fire, and `fix_all` is
idempotent. Review the diff and run your image build. The G5-real gate proves
the shape once per release, not once per edit.

### 4. Degradation is a fact, not a pass

`docker_verify` returns `None` when hadolint is absent and logs ONE line.
`None` is never coerced to `false` or `true`: *"we could not check"* and
*"the check failed"* are different facts.

Tier 2 needs a third variant, because `docker build --check` has a failure
mode hadolint does not. **Measured 2026-08-29:** feed it a two-defect
Dockerfile whose base image cannot be resolved and it reports **one** warning,
not two — and exits 1, exactly as it does when it found warnings and nothing
went wrong. A partial result is indistinguishable from a complete one by shape
or exit code alone. So `CheckRun` has three variants and `Errored` **discards**
the findings it did receive: reporting a strict subset would let a truncated
run masquerade as a clean-ish one. The only usable signal is `ERROR:` on
stderr; findings arrive on stdout, and the two streams are cleanly separable.
**Fourth measured fact (2026-09-06, Issue 071): a completed run always prints
`Check complete` on stdout and a failed resolve never does** — so
`classify()` refuses to return `Ran` for any run without the marker, whatever
its stderr says. If a future buildx reroutes or rewords the error summary, the
`ERROR:`-prefix gate stops firing and THIS gate catches the run: `Errored`,
never a fake `Ran(empty)`.

Two more measured facts the Tier-2 parser is built on, neither of them
guessable from the docs: `--progress=rawjson` carries **only** `vertexes` and
`statuses` — the check results are printed as plain text outside the JSON
stream, so text parsing is the only interface, not a shortcut. And the
availability probe is the oracle itself on `FROM scratch` (~0.2 s, no registry
round-trip) rather than a version-string guess: it tests the capability
end to end.

## Gates

| Gate | Result |
|---|---|
| **G1** fixtures + `clean_spans` + negative controls | PASS — 14 bounded rules each fire on their fixture AND reproduce the corpus `fixed_code` byte-exactly |
| **G1** negative control | PASS — **zero fixable findings** on `riir-chain/deploy/Dockerfile` and mmorpg `authority-container` |
| **G2** latency | 17.05 µs/scan release (1.24 ms debug), 302-byte file, M3 Max |
| **G3** no-regression | 5-domain router floors re-measured + re-pinned (below) |
| **G4** alloc | scan buffers reach a steady state; a clean file allocates no findings at all |
| **G5-real** | PASS — `edge-wallet-container` healed, `docker build` GREEN (the healed cache-mount `RUN` compiled for 167.7 s), image 141 MB, binary present |
| **Class-A agreement** | PASS — all DL-coded rules agree with live hadolint 2.15.1 |
| **Tier-2 agreement** | PASS — 4/4 BuildKit-check rules agree with live `docker build --check` (Docker 29.7.2 / buildx v0.36.1). Falsified with 3 injected defect classes. Since the Issue-071 repair the fixtures are registry-free: 2.7 s for the suite (was ~9.6 s with `alpine` round-trips, and 399 s + red on a registry-blocked box) |

### The negative controls, stated precisely

The controls are clean of **fixes**, not of every citation.
`authority-container` carries one suggest-only finding — its distroless
runtime declares no `USER`, so it runs as root (distroless ships `:nonroot`
variants). That is a genuine observation about a good file, not a defect
claim against it, and it is pinned so it cannot silently grow.

### Router-annotation floors (Issue 020, re-measured 2026-08-28)

Adding docker **never lowers** the annotation and twice RAISES it — a
distinctive off-vocabulary centroid absorbs fewer generic Rust spans than a
Rust-shaped one does. This is the measured answer to Proposal 006's open
question about a 5th domain's router cost: for this domain there is none.

| State | Before docker | With docker |
|---|---|---|
| clippy+perf | 71/101 (70.3%) | **73/101 (72.3%)** |
| +kernel | 89/130 | **94/131 (71.8%)** |
| +sec | 45/101 (44.6%) | **41/101 (40.6%)** |
| +sec+kernel | 78/131 (59.5%) | **82/131 (62.6%)** |

The floors block was also **restructured**: four `#[cfg]`-gated consts keyed
on `(kernel_heal, sec)` became one `cfg!`-selected function. A 5th domain
would have made it eight consts whose declaration cfg and use cfg must match
exactly — the precise shape that turned CI Layer 5a red on the sec landing.
Runtime selection compiles every arm in every state, so that failure class
is gone.

## Killed at mine time (recorded, not silently dropped)

- **`pin-package-versions` (DL3008/DL3018)** — hadolint detects and our own
  in-tree files trip it, but there is no bounded fix: the correct version is
  a registry fact, and this domain makes no network calls. Shipping it would
  be an echo of hadolint's message.
- **`tag-the-base-image` (DL3006)** — same, plus renovate/dependabot own
  version auditing.
- **`copy-from-references-defined-stage` (DL3022)** — hadolint FALSE-POSITIVES
  here: `deploy/Dockerfile` uses BuildKit **named contexts**
  (`--build-context riir-ai=../riir-ai`), which hadolint cannot see, so it
  reports two phantom findings on a correct file. We must not reimplement a
  detector that is wrong on our own tree.

## Reclassified at mine time (T1.1, the live re-verification)

Two rules authored as Class B turned out to have live incumbents the
Research 086 snapshot did not carry: `arg-secret-to-secret-mount` (DL3064)
and `entrypoint-shell-form-signal-block` (DL3025 covers ENTRYPOINT too).
Both stay — the fix side is still unoccupied — but their novelty claim was
false and the ledger says so.

A third apparent overlap was **refuted**: `apt-update-install-same-run`
looked like it collided with DL3009, but that was a fixture artifact (a bare
`apt-get update` RUN necessarily also trips the apt-lists rule). Probed
directly — a split where the update RUN already deletes its lists — hadolint
is entirely silent, so the cache-staleness axis is genuinely unoccupied.

## Not this domain

Compose/k8s manifests · GitHub Actions (zizmor) · image scanning
(trivy/grype/dockle) · base-image version auditing (renovate/dependabot) ·
runtime-flag policy · any `Cargo.toml`/lockfile editing · any registry
network call (digest resolution is suggest-only TEXT, never a lookup).
