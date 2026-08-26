# Plan 030 — `docker` Domain Phase 0 + First Batch (Class A overlay core + Class B Rust lane)

Executes [Proposal 006](../.proposals/006_docker_heal_domain_fifth_healdomain.md). Seeded from [Research 087](../.research/087_docker_heal_seed_inventory.md) with the dedup ledger at [Research 086](../.research/086_dockerfile_detection_inventory_hadolint_buildkit.md). Plan-first discipline (the Batch 60 rule): no corpus edit before this plan exists — it does now. The batch number is assigned at land time per the queue snapshot.

## Phase 0 — domain scaffold

- [ ] T0.1 `DockerDomain` impl at `src/domains/docker/` (mod.rs + corpus.rs + scanner.rs), mirroring the `sec` shape. `DockerRule { rule_id, docker_concept, trigger, buggy_code, fixed_code, family: DockerFamily, min_syntax: SyntaxVersion }`. If the `HealDomain` trait signature does NOT fit → STOP per Proposal 006 (the Proposal 004 caveat-5 discipline), file an issue.
- [ ] T0.2 DockerfileStanzaScanner: instruction-stanza spans (continuations, heredocs, ARG/FROM global-scope handling) + preceding-comment-block attachment; precision guards per the Issue 025 WGSL-scan precedent; a T5-style control gate (`tests/docker_t5_control.rs`) pinning scanner behavior on the 4 in-tree Dockerfiles BEFORE any rule exists.
- [ ] T0.3 Feature flag `docker = []` (opt-in, default-off, zero new deps) + `docker_verify` oracle tier: hadolint-on-PATH detection, Tier-0 stanza-validity fallback with ONE degradation log line (the Issue 018 oracle-split), Class-A agreement hook.
- [ ] T0.4 Wiring: `DomainRouter` 5th-domain fan-out annotation + `--domain docker` CLI + `fix_domain_scopes` arm + retrieval index inclusion (`DomainId::Docker`, `ALL` 4→5 — re-pin the Issue 020 router-annotation floors for the 5-domain state; count-pin sites swept, the 18th-site lesson).
- [ ] T0.5 Boundary + refusal gates: (a) Class-A rules must agree with hadolint's detection on fixtures; (b) **DL3059-refusal test** — the healer must NEVER consolidate consecutive RUNs (anti-modern, fights BuildKit caching) nor emit `apt-get clean` / LABEL-merging; (c) syntax-version floor enforcement (rules refuse below their `min_syntax`).
- [ ] T0.6 Phase-0 gate: `cargo check --features docker` + clippy `--all-targets` clean both states; default build unchanged (`cargo tree` diff empty).

## Phase 1 — first batch (bounded-fix core; the GOAT candidate: `edge-wallet-container`)

- [ ] T1.1 Mine-time dedup re-verification: every candidate checked against Research 086 (and the LIVE hadolint/build-check, not the snapshot — the standing mine-time rule); record one evidence line per rule in the batch research note. Expect kills (the Plan 018 rate was 5/11).
- [ ] T1.2 Author **Class A overlay batch** (~12): DL3009 apt-lists-rm · DL3015 no-install-recommends · DL3019 apk-no-cache · DL3020 ADD→COPY · DL3025 exec-form · DL3027 apt-not-apt-get · DL4000 MAINTAINER→LABEL · DL4006 pipefail · DL3000 workdir-abs · the BuildKit style family (casing, legacy-key-value, no-empty-continuation). Each rule documents its DL/check equivalent (the ledger cross-ref).
- [ ] T1.3 Author **Class B Rust lane** (~12): `dockerignore-target-entry` · `manifest-first-layer-split` · `cache-mount-cargo-home-coverage` + `cache-mount-id-scope` · `apt-update-install-same-run` · `apt-cache-mount-keep-downloaded` · `missing-syntax-directive` · `cargo-locked-flag` (guard: lockfile committed) · `workspace-single-member-build` (-p/--bin) · `arg-secret-to-secret-mount` (caller gate) · `shell-form-entrypoint-signal-block` · `entrypoint-script-missing-exec` (script-visible gate) · `build-stage-pin-buildplatform` + `arg-targetarch-missing-in-stage` (decision-table annotated — QEMU-native is in-tree-valid) · `distroless-distro-suffix-pin` · `musl-scratch-ca-certs` · `chef-stage-toolchain-skew`. Original fixtures; in-tree lessons (deploy/Dockerfile) are seeds, never pastes.
- [ ] T1.4 Proposers for the bounded subset; conservative gates (refuse when the fix needs data we can't derive — version pins, digests; refuse invocation-dependent rewrites — the secret-mount caller gate).
- [ ] T1.5 GOAT gates (Proposal 006's, executed): G1 fixtures + `clean_spans` in-tree excerpts + **negative control zero-finding on `riir-chain/deploy/Dockerfile` + mmorpg `authority-container`** + DL3059-refusal green; G2 latency; G3 no-regression + 5-domain floors re-pin; G4 alloc-free; **G5-real: heal `riir-chain/cloudflare/edge-wallet-container/Dockerfile`** — hadolint pre/post, `docker build` green, image bytes measured (expect: USER non-root + cache mounts landed; base swap stays suggest-only per the trade-off register), auto-reverts documented.
- [ ] T1.6 Adoption wiring (the standing consumer pull, sec-promotion-rule shaped): a CI-runnable `cargo heal --domain docker` (+ hadolint) check on the CF container Dockerfiles — the natural adopters are `riir-chain/deploy` + the edge-wallet path; record which repo ran it in CI as the promotion evidence.
- [ ] T1.7 Docs: AGENTS.md domain row + feature entry; `.docs/11_domains/docker.md` + README feature list; queue snapshot batch entry; batch research note (`Status: RECORD` on land) + Research 087 status pointer; this plan's checkboxes; commit/push.

## Phase 2 — suggest-only family + tier-2 oracle (GATED)

- [ ] T2.1 Opens ONLY if Phase 1 lands ≥ ~20 rules post-falsification (Proposal 006 caveat 1). Then: digest-pin, healthcheck-distroless (static Rust checker pattern), musl annotations family, attestations note, cargo-xwin/zigbuild routing, chef-vs-mounts-vs-sccache decision table, `docker build --check` tier-2 oracle, COPY `--exclude`/`--link` guard maturation.

## Close condition (honest)

If Phase 1 falsification leaves <10 BOUNDED rules, the domain closes at Phase 1: the suggest-only remainder becomes retrieval-only entries (still useful for `--suggest` during generation) or the feature stays dormant with a doc note — the `entropic-selection` negative-result precedent. Record the number either way.
