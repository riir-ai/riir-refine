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
| **B — novel detection** | 13 | both incumbents silent; we detect AND fix or annotate |

**27 rules total, 14 with bounded fixes.** (Plan 030's close condition was
≥10 bounded; Proposal 006's Phase-2 gate was ≥20 total. Both met.)

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
| **Tier-2 agreement** | PASS — 4/4 BuildKit-check rules agree with live `docker build --check` (Docker 29.7.2 / buildx v0.36.1). Falsified with 3 injected defect classes; ~9.6 s for the suite, dominated by the registry round-trip on the `alpine`-based fixtures |

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
