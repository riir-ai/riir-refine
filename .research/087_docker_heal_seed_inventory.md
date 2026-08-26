# Research 087 — Dockerfile Heal Domain: Fused Seed Inventory (3 online lanes + in-tree)

**Status:** DISTILLED — pending owner decision (feeds [Proposal 006](../.proposals/006_docker_heal_domain_fifth_healdomain.md) + [Plan 030](../.plans/030_docker_domain_phase0_batch1.md); corpus authoring is Plan 030's job, not this note's)

Three parallel mining lanes over online sources (2026-08-26) + the in-tree
Dockerfile inventory. The do-not-reimplement detection ledger lives in
[Research 086](086_dockerfile_detection_inventory_hadolint_buildkit.md)
(hadolint 71 DL rules + BuildKit 21 checks, both verified detection-only) —
this note carries the RULE CANDIDATES, the measured numbers, and the honest
trade-off register. Companion to Research 078 (the `sec` equivalent).

## Source grades (lane verdicts)

| Source family | Grade | Note |
|---|---|---|
| docs.docker.com (cache/optimize, reference, multi-stage, best-practices, language/rust) | **A** | canonical, current |
| cargo-chef README + lpalmieri.com | **A** | canonical + measured (5×) |
| depot.dev Rust Dockerfile guide | B+ | vendor, measured, the sccache angle |
| oneuptime 2026 build-time matrix | B | current, measured table, minor vendor bias |
| whitfin.io | B− | 2018-era; the measured size ladder still cited |
| OWASP Docker Cheat Sheet | **A** | the sec-lane spine |
| tini / phusion-baseimage / distroless / muslrust / musl-wiki / cross / zigbuild / xwin | **A** | canonical repos |
| Docker blog: multi-platform cross-compilation (tonistiigi) | **A** | the $BUILDPLATFORM pattern + QEMU multiples |
| natalia.dev distroless healthcheck | B | measured binary sizes, the exit-code contract |

## The two rule classes (the load-bearing design finding)

**Class A — fix-overlay** (~25 rows in Research 086 marked `yes`): hadolint /
`docker build --check` DETECT, nobody FIXES (both tools verified detection-only —
no `--fix` flag, 0 autofix issues). Our rule's detector must AGREE with the
incumbent's (hadolint is the oracle, the `clippy_verify` symmetry); the delta
is the bounded rewrite. Examples: DL3009 apt-lists-rm, DL3014/3015/3019 flag
inserts, DL3020 ADD→COPY, DL3025 exec-form, DL4000 MAINTAINER→LABEL, DL4006
pipefail, DL3000 absolute WORKDIR, the BuildKit style family (casing,
legacy-key-value, no-empty-continuation).

**Class B — novel detection** (verified absent from BOTH incumbent lists in
Research 086 §open-space): the cache/build-speed axis (both incumbents silent),
dep-layer ordering, PID1/signal semantics, platform pinning, secrets-mount,
`.dockerignore` context bloat, syntax-directive pinning. We detect AND
fix/suggest. This is the high-value lane — "detection saturated, fixing
completely unoccupied" (086 verdict).

## Candidate inventory (fused; post-falsification landing zone ~25-40)

### B1. Build-cache / perf (lane 1)

| rule_id | fix class | trigger → fix | source |
|---|---|---|---|
| `manifest-first-layer-split` | BOUNDED | `COPY . .` before build, no manifest-first dep layer → insert `COPY Cargo.toml Cargo.lock` + dummy-build + `COPY src` (deps ≈90% of build time, change rarely) | docs.docker.com/build/cache/optimize; oneuptime |
| `cargo-chef-stage-introduction` | BOUNDED (template) | manual dummy trick or no dep caching in a workspace → chef planner/builder 3-stage (measured 5×; recipe.json isolates cache from source churn) | cargo-chef README; lpalmieri |
| `dummy-deps-artifact-cleanup` | BOUNDED | dummy-build present, no `rm` of dummy artifacts → add rm before real-source COPY (cargo reuses stale dummy bins — regression since ~1.28) | whitfin; oneuptime |
| `cache-mount-cargo-home-coverage` | BOUNDED | mounts cover registry but not `/usr/local/cargo/git`, or split mounts race → add git mount OR the docs-canonical single `CARGO_HOME=/var/cache/cargo` mount | docs reference; docs language/rust |
| `cache-mount-id-scope` | BOUNDED | cache mount without `id=` on a shared builder → add `id=<project>` (default id = target path; two projects sharing `/app/target` pollute each other) | docs reference #run---mounttypecache |
| `sccache-wrapper-cache-mount` | BOUNDED | sccache installed but `RUSTC_WRAPPER` unset / no `SCCACHE_DIR` mount → env trio + `sharing=locked` mount (per-crate granularity where chef is lockfile-granular) | depot.dev |
| `chef-stage-toolchain-skew` | BOUNDED | chef stages FROM different rust tags → align (caching silently broken; chef README's own warning) | cargo-chef README |
| `apt-update-install-same-run` | BOUNDED | `apt-get update` in its own layer → merge update+install+lists-rm into one RUN (cached update + edited install = stale/404 "cache busting") | docs best-practices #apt-get |
| `apt-cache-mount-keep-downloaded` | BOUNDED | apt in hot path without mounts → disable docker-clean + Keep-Downloaded-Packages + `sharing=locked` mounts | docs reference (apt example) |
| `arg-not-env-build-only` | BOUNDED | build-only `ENV` (DEBIAN_FRONTEND) → `ARG` or per-RUN prefix (ENV persists + adds a layer) | docs reference #env |
| `bind-mount-source-instead-of-copy` | BOUNDED+guard | builder `COPY . .` solely to feed one RUN → `RUN --mount=type=bind,target=.` with output written outside the mount | docs cache/optimize #bind-mounts |
| `missing-syntax-directive` | BOUNDED | uses `--mount`/heredoc/`--link` without `# syntax=docker/dockerfile:1` → prepend | docs reference #syntax |
| `chef-target-flag-parity` | BOUNDED | `cook` without `--target` while build has it → propagate (dep layer compiled for wrong triple = useless under cross) | cargo-chef README |
| `copy-exclude-in-dockerfile` | SUGGEST | coarse `.dockerignore`; `COPY . .` pulls known junk → `COPY --exclude=` (needs syntax ≥1.19) | docs reference #copy---exclude |
| `heredoc-multi-command-run` | SUGGEST | long `\`-chains → heredoc (readability, not perf) | docs reference #here-documents |

### B2. Image size (lane 1 + in-tree)

| rule_id | fix class | trigger → fix | source |
|---|---|---|---|
| `dockerignore-target-entry` | BOUNDED | Rust repo, `.dockerignore` missing `target`/`.git` → add (in-tree measured: .git alone 184 MB at katgpt-rs; target "easily GBs") | oneuptime; in-tree (riir-chain deploy docs) |
| `distroless-distro-suffix-pin` | BOUNDED | `distroless/cc` without `-debianNN` → append (unversioned tag silently moves; Debian 13 = UsrMerge breakage class) | distroless README |
| `distroless-entrypoint-vector-form` | BOUNDED (≡DL3025 overlay) | shell-form ENTRYPOINT/CMD on distroless/scratch → exec form (no shell to exec it AT ALL) | distroless README |
| `musl-scratch-ca-certs` | BOUNDED | `FROM scratch` TLS-capable binary, no cert copy → `COPY --from=builder /etc/ssl/certs/ca-certificates.crt` (in-tree: riir-chain deploy ships this) | oneuptime; in-tree |
| `workspace-single-member-build` | BOUNDED | bare `cargo build` in a workspace Dockerfile → `-p <member>`/`--bin` (in-tree: deploy/Dockerfile documents `-p riir-chaind` as load-bearing) | oneuptime; in-tree |
| `cargo-locked-flag` | BOUNDED+guard | `cargo build/install` without `--locked` (guard: Cargo.lock committed) → add | cargo-chef; oneuptime |
| `copy-link-layer-rebase` | BOUNDED+guard | plain COPY of artifacts in final stage → `COPY --link` (independent layer; survives upstream invalidation) | docs reference #copy---link |
| `shared-base-stage-extract` | BOUNDED | 2+ stages repeating the same prologue → `FROM base` common stage | docs best-practices |
| `named-stage-not-index` | BOUNDED | `COPY --from=0` → named stages | docs multi-stage |
| `run-cd-workdir` | BOUNDED | `RUN cd /app && …` → WORKDIR | docs best-practices |
| `pipe-missing-pipefail` | BOUNDED (≡DL4006 overlay) | pipes in RUN without pipefail (guard: dash lacks it → bash exec-form) | docs best-practices |
| `musl-static-final-stage` | SUGGEST | final `FROM debian/alpine` for a statically-linkable `*-musl` build → scratch / `distroless/static-debian13:nonroot` (2 MiB vs 124 MiB) | muslrust; distroless |

### B3. Security (lane 3; OWASP-anchored)

| rule_id | fix class | trigger → fix | source |
|---|---|---|---|
| `arg-secret-to-secret-mount` | BOUNDED + caller gate | `ARG *_TOKEN/*_KEY/*_SECRET` consumed in RUN → `--mount=type=secret` + read `/run/secrets/`; refuse when the build invocation isn't visible | docs build/secrets; OWASP |
| `add-local-to-copy` | BOUNDED (≡DL3020 overlay) | `ADD <local>` non-tar → COPY (no surprise-extract surface) | OWASP #9 |
| `curl-pipe-shell` | SUGGEST | `curl … \| sh` → checksum-verify or vendor (needs data we can't derive) | OWASP #9 |
| `from-latest-or-untagged` | SUGGEST | `FROM img` / `:latest` → explicit tag, ideally digest | OWASP; docs digests |
| `from-tag-to-digest-pin` | SUGGEST | tag in prod stage → `@sha256:` (pick the INDEX digest not platform manifest — multi-arch trap; freezes patches → needs bump policy; resolution needs registry) | candrews; serverfault |
| `apt-unpinned-package` | SUGGEST | unpinned pkg → `=version` (needs package index) | OWASP #9 |
| `attestations-sbom-provenance` | SUGGEST (flag-level) | prod image → `--sbom=true --provenance=mode=max` (mode=min already default; containerd image store constraint) | docs attestations |
| `readonly-fs-runtime-note` | SUGGEST | prod image → document `--read-only --tmpfs /tmp` (runtime flag, not Dockerfile) | OWASP #8 |

### B4. Runtime correctness / PID1 (lane 3 — the family I originally missed)

| rule_id | fix class | trigger → fix | source |
|---|---|---|---|
| `shell-form-entrypoint-signal-block` | BOUNDED (≡DL3025's semantic core) | shell-form ENTRYPOINT/CMD in final stage → exec form: shell form makes `/bin/sh -c` PID 1; sh doesn't forward SIGTERM; `docker stop` waits 10 s then SIGKILLs (no graceful shutdown) | tini; phusion |
| `entrypoint-script-missing-exec` | BOUNDED (script visible) / SUGGEST | entrypoint script's daemon line lacks `exec` → prefix (else the shell gets the signal, not the app) | phusion |
| `pid1-init-for-child-spawning-apps` | SUGGEST | app spawns children → `docker run --init` (tini bundled since Docker 1.13) or COPY tini (~10 KB); zombie reaping + signal forwarding | tini |
| `stopsignal-mismatch` | SUGGEST | graceful-shutdown signal ≠ SIGTERM, or shutdown >10 s → STOPSIGNAL / --stop-time | phusion; docker stop docs |
| `healthcheck-distroless-no-shell` | SUGGEST | HEALTHCHECK wget/curl on distroless (no shell, no wget) → static Rust checker (610 KB no-TLS / 1.62 MB rustls, exit 0/1 contract) / app-side `/healthz` / `:debug` dev-only | distroless#183; natalia.dev |
| `healthcheck-exit-code-contract` | SUGGEST | checker can exit ≠0/1 (Rust panic!=101) → map to 1 (0=healthy, 1=unhealthy, others reserved) | natalia.dev |

### B5. Multi-platform (lane 3 — the "targets" pain)

| rule_id | fix class | trigger → fix | source |
|---|---|---|---|
| `build-stage-pin-buildplatform` | BOUNDED | builder stage compiles without `--platform=$BUILDPLATFORM` (runs toolchain under QEMU) → pin builder + `--target <triple>`; runtime stage stays unpinned. Measured: QEMU 498 s vs cross 95.3 s (5.2×); docker/cli "almost 7×"; cache-mount incremental "100×" | docs multi-platform; Docker blog |
| `arg-targetarch-missing-in-stage` | BOUNDED | stage uses $TARGETARCH in RUN but no `ARG TARGETARCH` line (automatic platform args are GLOBAL scope; invisible in-stage until redeclared valueless) → insert | Docker blog |
| `cross-toolchain-choice` | SUGGEST (routing) | foreign target with C deps / glibc floor / Windows / macOS → `cross` / `cargo zigbuild --target <triple>.<glibc>` / `cargo xwin` (judgment on deps+target) | cross; zigbuild; xwin |
| `musl-openssl-rustls-or-vendored` | SUGGEST | `*-musl` + openssl-sys → rustls or `vendored` (dynamic openssl has no static-musl path) | muslrust |
| `musl-dns-resolver-annotation` | SUGGEST (annotation) | network-facing musl service → parallel queries/≤3 NS, search/ndots divergence, no IDN, DNS-TCP ≥1.2.4 | musl wiki |
| `musl-allocator-perf-annotation` | SUGGEST (annotation) | memory-sensitive musl service → jemalloc/mimalloc global_allocator | muslrust#142 |
| `cross-compile-no-tests-caveat` | SUGGEST | cross stage tries `cargo test` → test natively/remote (`cross test` = sequential QEMU, thread tests hang) | Docker blog; cross |

### B6. Class A fix-overlay quick-list (from Research 086; full ledger there)

~25 `yes` rows: DL3000 (workdir-abs), DL3009 (apt-lists-rm), DL3014/3030/3034/3038
(-y flags), DL3015 (no-install-recommends), DL3019 (apk no-cache), DL3020
(ADD→COPY), DL3021 (COPY dest trailing /), DL3025 (exec form), DL3027 (apt not
apt-get), DL3029/3065 (platform flag), DL3032/3036/3040 (clean-all appends),
DL3042 (pip no-cache), DL3044 (ENV self-ref split), DL3046 (useradd -l),
DL3047 (wget progress), DL3060 (yarn clean), DL4000 (MAINTAINER→LABEL),
DL4006 (pipefail) + the BuildKit style family (StageNameCasing, FromAsCasing,
NoEmptyContinuation, ConsistentInstructionCasing, LegacyKeyValueFormat,
JSONArgsRecommended≡DL3025, WorkdirRelativePath≡DL3000, RedundantTargetPlatform,
ExposeProtoCasing, ExposeInvalidFormat, MaintainerDeprecated≡DL4000).

## Measured numbers (cite-ready)

| measurement | number | source |
|---|---|---|
| cargo-chef, ~500-dep codebase, CI layer caching | **~10 min → ~2 min (5×)** | lpalmieri.com/posts/fast-rust-docker-builds |
| depot incremental: naive → +chef → +sccache | 34 s → ~15 s → **~7 s** | depot.dev/blog/rust-dockerfile-best-practices |
| oneuptime matrix (clean/incremental) | naive 10-15m/10-15m · cache mounts 3-5m/**20-40s** · chef+mounts 3-5m/**15-30s** | oneuptime.com 2026-02-08 |
| whitfin size ladder | 1.66 GB → **86.5 MB (5.2%)**; time 266.6 s → 33.6 s | whitfin.io |
| QEMU vs cross (GNU hello, 2 platforms) | **498.0 s vs 95.3 s (5.2×)** | nttlabs (Suda) |
| docker/cli 2-arch: QEMU vs cross | "**almost 7×**" vs +70%; cache-mount incremental "**100×**" | Docker blog (tonistiigi) |
| distroless ladder | static-debian13 **≈2 MiB**; alpine ≈5 MiB; debian 124 MiB | distroless README |
| musl+scratch typical | 5-15 MB | oneuptime |
| Rust deps share of build time | ~90% | oneuptime |
| tini | ~10 KB; `--init` since Docker 1.13 | tini |
| static Rust healthcheck binary | 610 KB (no TLS) / 1.62 MB (rustls) | natalia.dev |
| `docker stop` grace | 10 s → SIGKILL | phusion/docker docs |
| in-tree: scratch+musl vs debian (riir-chaind) | **7.9 MB vs 138 MB** | riir-chain/deploy/Dockerfile |
| in-tree: toolchain-image pin mismatch cost | **5.3 min vs 3.3 s** per rebuild | riir-chain/deploy/Dockerfile |
| in-tree: .git in un-ignored context | **184 MB** (katgpt-rs) | riir-chain/deploy/Dockerfile docs |

## In-tree precedent inventory (the B49 measured-GOAT path)

The workspace's 4 Dockerfiles already encode hard-won, MEASURED lessons in
comments — mine them as corpus seeds (original fixtures, never paste):

1. **riir-chain/deploy/Dockerfile** — the GOAT artifact: scratch+musl (7.9 MB,
   measured table vs distroless/debian), toolchain-pin==rust-toolchain.toml
   (5.3-min-vs-3.3-s analysis), named contexts for sibling path deps,
   TARGETPLATFORM→triple case map, numeric USER on scratch, CA-cert path for
   rustls-native-certs, `-p riir-chaind` load-bearing, opt-level=z/panic=abort
   DELIBERATELY rejected (hot path + crash semantics — the honest-annotation
   class, in-tree precedent), musl DNS + no-shell trade-offs documented.
2. **riir-mmorpg-examples authority-container** — distroless cc variant choice
   + glibc-version pairing (the bookworm-slim break), cache mounts + copy-out-
   within-same-RUN, mkdir-in-builder-for-shellless-runtime.
3. **riir-chain edge-wallet-container** — the LIVE-FINDINGS artifact (Phase-1
   validation target): no USER (root), no cache mounts, bookworm-slim runtime
   (~85-108 MB base per our own measurement two files away).
4. **container-tick-test** — the Issue 030 measured template (CF Containers
   sizing: standard-2 1 vCPU; lite tier throttled 12-16×).

## Honest trade-off register (annotated rules cite this)

- **cargo-chef vs cache mounts vs sccache are complementary**: chef =
  lockfile-granular LAYER cache (exportable `--cache-to mode=max`, survives
  ephemeral runners); cache mounts are builder-local and NOT exported
  (depot FAQ: on ephemeral CI "you lose the cache after each build"); sccache
  = per-crate, needs own persistence. Best stack: chef + mounts (+sccache on
  dep churn). Cache mounts are perf-only: builds must stay correct under
  ARBITRARY cache contents (the `id=`-scope hazard follows).
- **QEMU-native vs $BUILDPLATFORM cross — both valid** (in-tree deploy
  deliberately uses QEMU-native alpine arm builds: no cross-toolchain, no
  zig; Docker blog pushes cross for compile-heavy). Encode as a decision
  table, not a one-sided rule. QEMU is fine for package installs; compilation
  and compression pay the multiple.
- **tini vs exec-form sufficiency**: single static binary + exec-form +
  own SIGTERM handler needs no init; tini only for zombie-producing children.
- **Digest pinning**: maximal reproducibility but freezes upstream security
  patches (needs a bump workflow); tag-digest ≠ manifest-digest (multi-arch
  trap); not offline-resolvable → suggest-only forever.
- **musl**: DNS model differs (parallel/≤3, search/ndots, no IDN); allocator
  slower for some workloads; openssl-sys needs vendored; backtraces raw.
- **distroless debuggability**: no shell/ldd → `:debug` tags dev-only.
- **cargo-xwin**: MSVC/CRT license consent; cmake-crates need ninja.
- **zigbuild**: glibc suffix best-effort (invalid → silent fallback);
  `+crt-static` with glibc unsupported.
- **Attestations**: buildx-flag level, not Dockerfile level; `docker` driver
  needs containerd image store.
- **Anti-cargo-cult (encode as REFUSALS, not rules)**: `apt-get clean`
  redundant on official Debian/Ubuntu (debuerreotype runs it); LABEL-merging
  for size is pre-1.10 lore; **DL3059 (consolidate consecutive RUNs) is
  ANTI-MODERN** — it fights BuildKit layer caching; the dep-layer-splitting
  direction is the opposite. The healer must never "fix" toward DL3059.
- **Not verified by the lanes — re-verify before citing**: strip/upx/
  opt-level=z binary-size deltas, alpine-musl runtime-perf penalty numbers,
  C-sys-dep preinstall lists (cmake/g++/perl for openssl-sys) beyond the
  in-tree sentencepiece lesson.

## Verdict

Denser than `sec` (whose 15-19 was marginal): ~70 candidates fused, ~30 of
them mechanically BOUNDED, post-falsification landing zone **25-40 rules**.
Detection is saturated by incumbents; fixing is completely unoccupied; the
cache/build-speed + PID1 + platform lanes are novel detection territory.
Proceed via Proposal 006 / Plan 030.
