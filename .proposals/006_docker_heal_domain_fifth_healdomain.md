# Proposal 006 — The `docker` Domain: Dockerfile Bounded-Fix Healing + Retrieval as the Fifth HealDomain

Status: **LANDED — Phase 0 + Phase 1 EXECUTED 2026-08-28, GOAT PASS** (riir-clippy `3839457`; [Plan 030](../.plans/030_docker_domain_phase0_batch1.md) / [Bench 056](../.benchmarks/056_docker_domain_phase0_1_goat.md)). *Supersedes the 2026-08-28 tree audit that read this as uncommitted sibling WIP — that reading was accurate when written; the work is now committed and HEAD is the 5-domain state.* Its three flip conditions, answered: checkboxes synced ✅ · the G1–G5 record incl. the edge-wallet G5-real ✅ · **T1.6 standing-consumer evidence ✅ — closed 2026-08-28: riir-chain adopted the fixes on both deployment images and gates them in CI (`scripts/dockerfile_heal_gate.sh`), so the feature is now DEFAULT-ON.** Shipped **27 rules — 14 Class A, 13 Class B, 14 with bounded fixes** (15 at land time; the T1.6 adoption run demoted `cargo-locked-flag` to suggest-only after a measured build break) (caveat-1 landing zone was 25-40; close condition ≥10 bounded). Features `docker` + `docker_verify`, zero new deps. Four corrections this proposal's own text needs, each measured rather than argued: **(1)** the negative-control claim holds in the form "zero **fixable** findings", not zero findings — `authority-container` carries one real suggest-only citation (its distroless runtime declares no `USER`), a true observation about a good file rather than a defect claim, now pinned so it cannot silently grow; **(2)** the caveat-1 suggest/bounded ratio landed at **14 bounded / 13 suggest** (15/12 at land time), better than the "~half suggest-only" estimate; **(3)** the T1.1 live re-verification **reclassified 2 rules B→A** (`arg-secret-to-secret-mount` → DL3064; `entrypoint-shell-form-signal-block` → DL3025, which covers ENTRYPOINT) because the Research 086 snapshot did not carry those incumbents — while a third apparent overlap (`apt-update-install-same-run` vs DL3009) was **refuted** by direct probe and is genuinely unoccupied; **(4)** the open question about a 5th domain's router cost is answered — adding docker never lowers annotation accuracy and twice **raises** it (2-domain 71→73/101; 4D+K 78→82/131), because an off-vocabulary centroid absorbs fewer generic Rust spans. Two rules moved to the caller-gated class on principle (`build-stage-missing-buildplatform`, `cargo-locked-flag` — both need invocation/repo facts absent from the file). **Phase 2 EXECUTED 2026-08-29 — the proposal is fully discharged.** T2.1(a): `docker build --check` is now a standing, falsification-verified gate behind `docker_buildcheck` (opt-in — a daemon plus a registry round-trip is a heavier dependency class than hadolint's static binary, which is a dependency question, not a GOAT one). T2.1(b): 4 suggest-only rules, **27 → 31, bounded unchanged at 14**; two of them were NARROWED by measurement after being written, each caught by an existing gate, and the batch found a real bug in the domain's model of the frontend (`from_directive` read Docker's own canonical `:1` as `1.0` instead of latest-of-major). riir-chain adopted one citation and recorded reasoned decisions for the other two the same day. T2.1(c) — attestations, cross-toolchain routing, the chef/sccache decision table, `COPY --exclude` maturation — is DEFERRED with a stated reason per item: each needs a fact that is not in the Dockerfile (a build-invocation flag, the dep graph's C content, a target triple), which is the `cargo-locked-flag` class this domain has already been burned by once.
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Proposal 001 (HealDomain meta-architecture) × Proposal 004 / Plan 018 (the `sec` domain pattern: boundary gate + negative control + close condition) × Research 086 + 087 (detection ledger + fused seed inventory) × the in-tree Dockerfiles (riir-chain `deploy/Dockerfile` as the measured-GOAT artifact — the Batch 49 mine-our-own-artifacts path)
Related: [Research 086](../.research/086_dockerfile_detection_inventory_hadolint_buildkit.md), [Research 087](../.research/087_docker_heal_seed_inventory.md), [Plan 030](../.plans/030_docker_domain_phase0_batch1.md), [Proposal 004](004_sec_domain_fourth_heal_domain.md)

## TL;DR

Open a fifth healing domain — `docker` — for **Dockerfile source patterns with bounded fix-spaces**, covering build-cache performance, image size, package hygiene, secrets, PID1/signal runtime correctness, and multi-platform cross-builds, Rust-toolchain-first. Seeded from Research 087 (~70 candidates, ~30 mechanically bounded; post-falsification landing zone 25-40 rules — denser than `sec`'s marginal 15-19). The delta vs incumbents is verified with receipts: hadolint (71 DL rules) and `docker build --check` (21 checks) are both **detection-only** — fixing is completely unoccupied, and the cache/build-speed axis is novel detection territory nobody covers (Research 086). This is a distillation of online + in-tree prior art, not an invention.

## The problem this solves

1. **Agent-authored Dockerfiles are reliably wrong in the same ways** (owner attestation + universal): no multi-stage, no layer caching, fat bases, root user, shell-form CMD, broken cross-target builds. The workspace's own 4 Dockerfiles prove the pattern: the good ones (`riir-chain/deploy`, mmorpg `authority-container`) encode hard-won MEASURED lessons in comments — 7.9 MB vs 138 MB scratch/debian, 5.3-min-vs-3.3-s toolchain-pin cost, the glibc pairing break — while `edge-wallet-container` ships TODAY with live findings (root user, no cache mounts, ~10× fatter base than its sibling's measured alternative). The knowledge exists; it is discipline-per-file instead of substrate.
2. **The tool gap is real and pinned**: hadolint and `docker build --check` detect; neither fixes (verified live — no fix flag, zero autofix issues; Research 086 §3). A wrong Dockerfile today gets a warning list, not a heal.
3. **Retrieval > fixing for the primary pain**: the owner's use case is agents GENERATING Dockerfiles. A latent corpus the drafter can suggest from at authoring time (the `rust_perf` value prop) prevents the mistakes; `--fix` repairs the rest.

## The proposed design

### `DockerDomain: HealDomain` — trait holds (the sec precedent, twice-proven)

```rust
pub struct DockerDomain;

impl HealDomain for DockerDomain {
    type Entry = DockerRule;   // { rule_id, docker_concept, trigger, buggy_code,
                               //   fixed_code, family: DockerFamily }
    type Verdict = DockerVerdict; // { parses, stanza_valid, hadolint_agrees: Option<bool> }
}
// DockerFamily: CachePerf | ImageSize | BaseChoice | PkgHygiene |
//               RunCorrectness | SignalPid1 | SecretsHygiene | MultiPlatform | RustIdiom
```

Feature `docker` (opt-in, default-off, **zero new dependencies** — stanza scanning is regex/line-level, the Issue 025 WGSL-scan precedent). The chunker carries each instruction stanza + its preceding comment block (in-tree Dockerfiles are comment-rich; comments are load-bearing retrieval context).

### Two rule classes (the load-bearing design finding from Research 086)

**Class A — fix-overlay (~25 rules):** hadolint/BuildKit-check detects, we fix. Our detector must AGREE with the incumbent's detection (hadolint = the oracle — the exact `clippy_lints`/`clippy_verify` symmetry). Examples: DL3009 apt-lists-rm, DL3015 no-install-recommends, DL3020 ADD→COPY, DL3025 exec-form, DL4006 pipefail, the BuildKit style family.

**Class B — novel detection (both incumbents silent):** the cache/build-speed axis (manifest-first dep layers, cache-mount coverage/id-scoping, chef/sccache), `.dockerignore` context bloat, PID1/signal semantics (shell-form block, entrypoint-script exec), platform pinning (`--platform=$BUILDPLATFORM`, in-stage `ARG TARGETARCH`), secrets-mount rewrite. We detect AND fix/suggest.

### Oracle ladder (`docker_verify`, graceful degradation per Issue 018)

| Tier | Check | Cost | When |
|---|---|---|---|
| 0 | stanza parse (known instruction, well-formed flags) | free | always |
| 1 | hadolint re-run (external binary if on PATH) | ~100ms class | `--verify`, Class-A agreement gate |
| 2 | `docker build --check` (daemon, no execution) | fast | optional |
| 3 | `docker build` ground truth + `docker images` bytes | slow | GOAT gates only |

Tier 1 absent → degrade to Tier 0 with one log line (the oracle-split precedent). The healer never edits anything but the Dockerfile itself.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **The suggest-only share is HIGH (~half the candidate space)** — base-image choice, digest pinning, tini necessity, musl trade-offs, cross-toolchain routing are judgment calls with measured trade-offs (Research 087 §register). Say the bounded/suggest ratio at authoring time; if <10 BOUNDED rules survive falsification, the domain closes (close condition in Plan 030).
2. **No syn.** The scanner is stanza/regex-level — precision guards + a T5-style control gate are mandatory (Issue 025 precedent). Dockerfile semantics are FRONTEND-VERSION-DEPENDENT (`# syntax=` gates: heredoc 1.4, `--exclude` 1.19) — every rule declares its minimum syntax version and refuses below it.
3. **External oracle dependency.** hadolint must be installed for the strong verify tier; without it we degrade honestly (Tier 0) — never silently claim verification.
4. **PID1/signal fixes change runtime behavior**, not just text (exec-form changes who receives SIGTERM). G5-real must include an actual `docker build` (+ smoke `docker run` where cheap), not parse-green alone.
5. **Anti-cargo-cult is load-bearing**: DL3059 (consolidate RUNs) is ANTI-MODERN — it fights BuildKit layer caching. The healer must REFUSE fixes toward it; encode as an explicit refusal + test (a heuristic that only merges RUNs would be actively harmful). Same class: `apt-get clean` (redundant), LABEL-merging (lore).
6. **QEMU-native vs $BUILDPLATFORM cross: both valid** (in-tree `deploy/Dockerfile` deliberately builds QEMU-native alpine; Docker blog pushes cross at 5.2×-7× for compile-heavy). Encode as a decision-table annotation (dep complexity × matrix width), never a one-sided rewrite.
7. **Density is pre-falsification.** ~70 candidates → honest landing zone 25-40 after dedup vs Research 086 + mine-time falsification (the Plan 018 sec discipline: 5 of 11 died there; expect kills).
8. **Consumer pull follows the sec promotion rule**: the edge-wallet heal is a ONE-SHOT artifact, not standing pull. Promotion to default-on requires ≥1 workspace repo running `cargo heal --domain docker` (+ hadolint oracle) in its container CI path — the Cloudflare container pipelines are the natural adopters.

## Fusion lineage

- **Proposal 001** — the HealDomain architecture; sec proved the trait holds twice.
- **Proposal 004 / Plan 018** — the fourth-domain playbook verbatim: do-not-reimplement boundary gate, real-repo negative control, falsification discipline, close condition, one-batch-cycle sizing.
- **Research 086 + 087** — the dedup ledger (incumbent detection inventory, autofix-status fact) + the fused seed inventory with measured numbers.
- **The in-tree Dockerfiles** — `riir-chain/deploy/Dockerfile` is the measured-GOAT corpus seed (the Batch 49 path: distill our own production artifacts); `edge-wallet-container` is the live-findings validation target; `deploy` + `authority-container` are the zero-finding negative controls (the riir-viewbridge role).

## GOAT gate

- **G1**: every fixture fires its rule; `clean_spans_fire_no_lint` extended with real excerpts from the 4 in-tree Dockerfiles; **negative control: zero findings on `deploy/Dockerfile` + `authority-container`** (nonzero falsifies the proposers); Class-A rules pass the hadolint-agreement gate; the DL3059-refusal test green.
- **G2**: suggest/fix latency within the existing domain family (µs-scale span processing; no new hot-path work).
- **G3**: all lib tests + feature-combo checks green; other domains' retrieval floors unchanged; the 5-domain router annotation re-pinned per the Issue 020 contract; every count-pin site updated (the 18th-site lesson).
- **G4**: scan-path alloc-free (`SuggestScratch` precedent).
- **G5-real**: heal `edge-wallet-container` end-to-end (`--fix --verify` + hadolint oracle): `docker build` green pre/post, findings resolved (USER non-root, cache mounts, base decision annotated), image bytes measured before/after, auto-reverts documented.
- **Promotion** (default-on): measured standing consumer pull — the CF container path (or another repo) running the healer in CI, the `rust_perf` precedent.

## What ships now (riir-clippy) vs deferred

### Ships now — Phase 0 + Phase 1 (Plan 030)
`src/domains/docker/` (mod + corpus + stanza scanner + proposers), feature `docker`, `docker_verify` oracle tier, DomainRouter 5th-domain wiring + CLI, the DL3059-refusal + boundary gates, first batch: **Class A overlay core (~12 mechanical)** + **Class B Rust lane (~12: dockerignore, manifest-first, cache-mount coverage/id, apt-same-run, syntax-directive, `-p` member, `--locked`, arg-secret-mount, exec-form signal, buildplatform-pin, arg-redeclare, distroless-suffix)**, docs.

### Deferred — Phase 2 (gated on Phase 1 ≥ ~20 rules landed)
Suggest-only family as retrieval entries (digest-pin, healthcheck-distroless, musl annotations, attestations, cargo-xwin, chef-vs-mounts-vs-sccache decision table, zigbuild), `docker build --check` tier-2 oracle wiring, COPY `--exclude`/`--link` guards maturation.

### Explicitly NOT shipped
Compose/k8s manifests, GitHub Actions (zizmor owns it), image scanning (trivy/grype/dockle), base-image version auditing (renovate/dependabot), runtime-flag policy, any Cargo.toml/lockfile editing, any registry network calls (digest resolution is suggest-only text, never a lookup).

## Phased rollout (sketch — Plan 030 expands)

### Phase 0 — scaffold
- [x] domain + corpus + feature + scanner + router/CLI wiring + control gates — **DONE 2026-08-28**

### Phase 1 — first batch (Class A core + Class B Rust lane)
- [x] mine-time dedup vs Research 086 → author rules/fixtures → proposers → GOAT → edge-wallet G5-real → docs — **DONE 2026-08-28, GOAT PASS** ([Bench 056](../.benchmarks/056_docker_domain_phase0_1_goat.md))
- [-] CF adoption wiring (T1.6) — **NOT DONE**: an act in the consumer repo's CI, and the unmet half of the promotion rule

### Phase 2 — suggest-only family + tier-2 oracle (gated)
- [x] **DONE 2026-08-29** — T2.1(a) tier-2 oracle + T2.1(b) suggest-only family (31 rules, 14 bounded). T2.1(c) deferred with a per-item reason

## Risks

1. **Scanner precision** (correctness): stanza regex over a small grammar — mitigated by precision guards + the negative controls + the T5 control-gate precedent; BuildKit syntax-version conditionals.
2. **Behavior-adjacent fixes** (PID1/exec-form): mitigated by G5-real `docker build` + smoke.
3. **Effort split** (opportunity): rust_perf external intake stays the productive lane; Phase 0+1 sized as ONE batch-cycle unit (the sec sizing).
4. **Incumbent drift**: hadolint/BuildKit checks evolve — re-pin the ledger at authoring time (the standing mine-time rule; Research 086 is a snapshot).

## Out of scope

Everything in "Explicitly NOT shipped", plus: the WASM domain, L4 fallback changes, binary-distribution surface, existing domains' corpora.

## References

1. [Research 086](../.research/086_dockerfile_detection_inventory_hadolint_buildkit.md) (in-house, distilled) — hadolint 71 DL + BuildKit 21 checks, autofix-status fact, open-space map.
2. [Research 087](../.research/087_docker_heal_seed_inventory.md) (in-house, distilled) — the fused seed inventory + measured numbers + trade-off register.
3. Docker docs (cited-only, canonical): build/cache/optimize, build/building/multi-stage + multi-platform, reference/dockerfile (RUN --mount, COPY --link/--exclude, platform args), build/building/secrets, build/metadata/attestations, reference/build-checks, language/rust.
4. cargo-chef (LukeMathWalker) + lpalmieri "Fast Rust Docker builds" (measured 5×).
5. depot.dev Rust Dockerfile best practices (sccache angle) + oneuptime 2026 build-matrix (measured).
6. OWASP Docker Security Cheat Sheet (the sec-lane spine).
7. tini (krallin) + phusion/baseimage-docker (PID1/signal lore) + GoogleContainerTools/distroless (+ issue 183 healthcheck).
8. Docker blog "Faster multi-platform builds" (tonistiigi; QEMU 5.2×-7×, $BUILDPLATFORM pattern) + nttlabs buildx post (Suda; 498 s vs 95.3 s).
9. cross-rs/cross, rust-cross/cargo-zigbuild, rust-cross/cargo-xwin, clux/muslrust, musl-libc wiki (cross-toolchain + musl realities).
10. whitfin.io (measured size ladder, 2018-era — numbers re-verified against newer sources where cited).

## TL;DR

Ship it: open `docker` as the fifth `HealDomain` — the sec playbook applied to a denser, better-measured space where fixing is verified-unoccupied and the primary pain (agent generation) is served by retrieval. **Executed 2026-08-28** ([Plan 030](../.plans/030_docker_domain_phase0_batch1.md), [Bench 056](../.benchmarks/056_docker_domain_phase0_1_goat.md)): 27 rules, 14 bounded, GOAT PASS incl. a real `docker build`. **T1.6 closed and the feature PROMOTED to default-on 2026-08-28** — riir-chain is the standing consumer. The adoption run paid for itself: it demoted `cargo-locked-flag` to suggest-only after a measured build break that G5-real (which healed a copy) structurally could not see. Next action: Phase 2, unblocked but deferred on its own gate.
