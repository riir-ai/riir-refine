# Issue 221 — crates.io release lane: publish cargo-refine to crates.io, then deprecate the binary-only dist lane (executes 192 GAP 11 / Plan 325 T8)

**Status:** open — the GAP 11 trigger has FIRED: `riir-ai/riir-refine` is **PUBLIC** (GitHub API `visibility: public`, licence MIT, verified 2026-11-09). Issue 192 GAP 11 (owner, 2026-10-31) already directed "when the extraction opens, it publishes to crates.io directly — distribution upgrades from installers-only to normal `cargo add`/`cargo install` consumption", folding into [Plan 325 T8](../.plans/325_public_goods_cli_surface.md) (owner-gated). This issue is that work's execution tracker: the old-lane inventory (grep), the publish blockers (verified today), the lane design, and the deprecation sequencing — **deprecation steps run only after the crates.io lane is live** (never leave the public surface with no install path).
**Date:** 2026-11-09
**Claimed:** zed-m3-cratesio-221, 1794196655, expires 2026-11-11T03:57:35+00:00

## Premise — what changed and what the flip means

1. **The extraction shape resolved differently than T15 drafted.** [Issue 190 T15](190_mozilla_grant_readiness_gaps.md) drafted a FRESH extraction repo and refused a flip of the private repo ("history may carry old secrets"). The owner **flipped this very repo public instead** — the flip is the owner act and stands; T15's audit halves become POST-flip tasks (P0 below), not blockers to undo.
2. **Supersession scope reconfirmed:** GAP 11 supersedes katgpt-rs Research 003's no-cargo-publish law **for this repo only** — private siblings keep `publish = false` (riir-ledger, reflexer-vessel rows below are private-sibling decisions, not ours to flip).
3. **The org migration (issue 219) finished first:** the dist family is `riir-ai/*` everywhere, `RELEASE_PAT` is one-org (scoop bucket moved `3c0f8f94`), all live surfaces probe clean. The old lane was just renovated — this issue supersedes it, not repairs it.

## Verified facts (2026-11-09, this session)

| Check | Result |
|---|---|
| GitHub `api.github.com/repos/riir-ai/riir-refine` | HTTP 200, `"private": false`, `"visibility": "public"`, licence MIT detected |
| Root `LICENSE` | MIT, commit `54ad1c524` ("copied byte-for-byte from katgpt-rs LICENSE") |
| crates.io name `cargo-refine` | **404 = available** |
| crates.io name `riir-refine` | 404 = available (fallback) |
| crates.io `katgpt-core` | **200 = already published**, max_stable **0.4.1**, updated 2026-08-22 (≈3 months behind the tree — see P1) |
| crates.io `katgpt-micro-belief` | 200, max_stable 0.2.1 — **matches the Cargo.toml row's `version = "0.2.1"` exactly** (the one already publish-prepared dep row) |
| Manifest today | `publish = false`, `license-file = "release/LICENSE.md"` (an EULA, v1 eff. 2026-10-03), comment "The shipped binary is proprietary freeware… The source is private" — ALL stale vs the public MIT repo |

## Findings A — the old lane inventory (grep, 2026-11-09)

**Machinery (riir-refine):**

| Surface | Where | Role |
|---|---|---|
| Binary release workflow | `.github/workflows/release.yml` | tag `v*` → 5-target matrix (aarch64+x86_64 darwin, musl ×2, windows-gnu) → cargo-about licenses + cargo-auditable → leak scan + size gate → SHA256SUMS → `gh release` on `riir-ai/cargo-refine` → brew-formula + scoop-manifest bump PRs. Secrets: `SIBLING_REPOS_TOKEN` (private siblings) + `RELEASE_PAT`. Header pins: no `--locked`, `--remap-path-prefix` required, musl via setup-cross-toolchain-action |
| Manual lane | `scripts/manual_release.sh` | owner lane; `gh release create -R riir-ai/cargo-refine` |
| Release docs | `release/LICENSE.md` (EULA v1) + `release/notes-v0.3.0…v0.4.8.md` | EULA ships in every archive + is `license-file` in Cargo.toml |
| Self-update | `src/update_check.rs` | `DIST_REPO = "riir-ai/cargo-refine"` (L31), `RELEASES_API` latest-release poll, SHA256SUMS-verified fetch + backup-swap; L790 names install.sh/install.ps1 as the fallback |
| Dist repos | `riir-ai/cargo-refine`, `riir-ai/homebrew-tap`, `riir-ai/scoop-bucket` | the 3 source-free public repos (Proposal 003 / Plan 105); rescopingly touched by issue 219 B |
| Release-set pin | `src/build_stamp.rs` `RELEASE_FEATURES` + `release_set_matches_workflow` test | pins the release set to release.yml's build line — **the publish job must be a SEPARATE job, never an edit to the build line, or the pin reds** |

**In-repo md that speaks the old lane:** `README.md:29-30` (blockquote: "source-free dist repo (macOS/Linux install.sh…)") + `README.md:307` (upgrade row); `AGENTS.md` L59 "Source secrecy & binary-only distribution (non-negotiable)" (the numbered 1–4 law incl. "no `cargo publish` (already blocked: `publish = false`)"), L68 "Why this is a separate repo", L41 "Production adoption" (install-probe hazard + Homebrew-shadow rows reference the dist lane — those hazards survive crates.io, only the lane wording moves), L161 CLI UX contract (`upgrade`/`update` rows); `.docs/01_orientation/foo_end_to_end.md:17,30-31`; `.docs/01_orientation/role_flows.md:99`; `.docs/01_orientation/mozilla_grant_readiness.md:79` + `:438` (the grant narrative already promises "crates published to crates.io" — this issue DELIVERS it); `.docs/01_orientation/mozilla_evidence_pack.md` (release-download counts; crates.io downloads join per GAP 11); `.docs/07_examples/cargo_refine.md:15` (`cargo install --path .`).

**Website (served from riir-dapps `cloudflare/kat-service/src/` — NOT this repo):** `front_page.rs:1567,1702,1704,1710,1716,1718` (the install matrix: curl install.sh ×3, brew tap, install.ps1, scoop) and **`:1731-1732` the FAQ "Is it on crates.io?" → "No — Refine ships as checksum-verified prebuilt binaries only…"** (the fork dead-end the 192 site audit flagged as load-bearing); `heal_skill.md:40-49` (install block + "It is NOT on crates.io") + `:263` ("Anything else named cargo-refine is…"); **contract tests pin every line**: `contract/front.rs:177-195` (asserts brew tap + install.sh + scoop + "crates.io"), `copy_contract.rs:65,237`, `heal_skill.rs:90` ("not on crates.io"), `refine_capture_contract.rs:217`. The site flip is a code+contract change in riir-dapps, deployed via the owner touch-deploy lane, with a dapps deploy-ledger row + audit (riir-dapps law).

## Findings B — why `cargo publish` of this tree fails TODAY (the blockers)

1. **Manifest posture** (blocker 0, trivial): `publish = false`; `license-file` EULA vs root MIT; the proprietary comment. Reconcile in P0.
2. **Path deps without registry versions** — cargo publish requires EVERY dep row to carry a registry version; private-sibling rows can never have one:

| dep row | gate | sibling | publish verdict |
|---|---|---|---|
| `katgpt-core` | non-optional | katgpt-rs (PUBLIC) | **publishable — already on crates.io at 0.4.1, stale vs tree** (P1) |
| `katgpt-micro-belief` | opt-in `nonergodic_poc` | katgpt-rs | published 0.2.1, row carries `version` — the template row |
| `katgpt-percepta` | opt-in `wasm_validator` | katgpt-rs | publishable if the feature survives; recommend trim |
| `reflexer-vessel` | opt-in `vessel_artifacts` | riir-reflexer (PUBLIC) | publish=false today; recommend trim (feature stays binaries-only) |
| `riir-ledger` | opt-in `points` | riir-ledger (publish=false) | stays out of the publish posture |
| `riir-kat` | **NON-optional** (`redact`) | riir-kat (private-forever) | **cannot publish** — the redact secret-scan tier + `points`' client/provider_login/lease_http arms ride it |
| `riir-infer-core`/`-gpu` | opt-in `ternary_inference` | riir-infer | stays out (+ the `[patch.crates-io]` vendor forks at Cargo.toml:1429 ride this lane) |
| `riir-rag`/`riir-neuron-db` | opt-in `latent_retrieval` | riir-neuron-db (private) | stays out |
| `riir-rag-bonsai` | opt-in `bonsai_retrieval` | riir-ai (private) | stays out |
| `riir-llm` | opt-in `hosted_expert` | riir-llm | stays out |

3. **Feature-row closure:** removing dep rows forces removing every `[features]` row naming them AND every row implying one — the known chain is `kernel_route_narrow ⇒ latent_retrieval`; also `points`, `net-mainnet`/`net-devnet`/`allow-local` (forward riir-kat features), `ternary_inference`, `vessel_artifacts`, `bonsai_retrieval`, `hosted_expert`/`seat_rerank` (riir-llm + escalation classes), `l4_lora` (seam-only, review on trim). The surgery must be verified by `cargo metadata` on the publish manifest showing ZERO path deps + a full standalone compile.
4. **The redact question (owner-gated):** the secret scanner is a product feature (issue 151) that rides private riir-kat. Options: (a) crates.io build ships without `--secret-scan` (disclosed in the crate README + the site FAQ wording), (b) vendor the redact tables into this repo (publishes private-repo data tables — generic secret patterns, low sensitivity, but it IS publishing riir-kat-derived content; owner call), (c) 190 D3's leaf-crate idea, published (`kat-redact` fresh public repo). Default for the first publish: (a) — smallest, honest, reversible.
5. **`[patch]` sections:** `[patch.crates-io]` vendor forks (ternary lane) + `[patch."https://github.com/katopz/katgpt-rs"]` git-source patch (Cargo.toml:1615). Patch sections apply only in the declaring workspace — verify `cargo publish --dry-run` tolerance; trim the unusable rows in surgery anyway (a published manifest carrying patches to sibling paths is dead weight).
6. **Packaging hygiene:** the package dir carries `.docs/ .issues/ .plans/ .distill/ .refine/ assets/ release/ scripts/ vendor/ corpus fixtures…`. Without `[package] include`/`exclude`, the `.crate` ships internal docs. The surgery adds a tight include list (src/, build.rs, any include_str!/include_bytes assets the drafter embeds, README, LICENSE) — verified by `cargo package --list` audit in the gate (P2).
7. **History exposure (flip-specific):** the public repo carries full private-era history (AGENTS.md/HISTORY.md/.docs internal posture are all public NOW). P0 audits it; anything found leaks into the issue, not a silent rewrite.

## The lane design (recommended — owner-gated items marked 👤)

- **Shape — publish-time manifest surgery from the release tag (Path C).** No repo restructure: the dev workspace keeps its sibling path deps. A deterministic script (P2) generates the publish manifest from the tag tree: `publish = true`, `license = "MIT"` (drop license-file), 👤 crate name `cargo-refine` (verified available; matches bin + brand + the FAQ search — `riir-refine` is the fallback; [lib] name stays `riir_refine`), private dep rows + their feature rows + patch rows removed, include list added. Then: standalone build gate at the publish feature set with **ZERO sibling checkouts** → `cargo package --list` audit → `cargo publish --dry-run` → `cargo publish`.
- **Publish feature set:** `default` (clippy_lints, rust_perf, self_evolve, docker, rustc_errors, dist, kernel_pregate, ruliology_search, shader_perf) + `fix_verify` + `update_check` — all registry-only (ureq/sha2/base64/getrandom are registry rows already). `latent_retrieval`/`points`/`net-mainnet` stay **binaries-only** — the prebuilt lane keeps the full product (retrieval + join/rank/corpus); the crate README and the site FAQ say so HONESTLY (the contribution verbs need the hosted binaries).
- **Pipeline:** the publish job joins `release.yml`'s tag lane (192 decision 3: main+tag) as a separate job — never touching the pinned build line. 👤 Credentials: GitHub **OIDC trusted publishing** (no long-lived secret; matches the touch-deploy era's secret-minimization) or a fine-grained crates.io token secret — owner setup either way.
- **Version:** 👤 first crates.io release rides the current line (0.4.8) or steps to 0.5.0; release-notes continuity lives in `release/notes-v*.md`.
- **docs.rs:** builds from the trimmed manifest — verify default-features doc build green; `repository` = this repo, `homepage`/docs links → refine.riir.ai. README born-compliant (the 190-T6 rewrite landed 2026-11-05; the install row flips in P6).
- **`cargo refine upgrade` coexistence:** a crates.io-installed user upgrades with `cargo install cargo-refine` (re-run installs over). First pass is docs + the alert line gaining the cargo-install hint; detecting install origin is a later nicety, not a gate.

## Deprecation sequencing (only after P3 — the crates.io lane is LIVE)

1. `release.yml`: ADD the publish job (P2/P3); the binary matrix **stays initially** (no-toolchain users + the full-feature build + update_check's feed). 👤 After 1–2 crates.io releases, owner decides retire-vs-keep-secondary.
2. Tap + bucket: final bump carries an "install via `cargo install cargo-refine` / see crates.io" formula homepage note, then the repos freeze (archive = 👤 owner act; issue 219's A1 PAT rescope already done, so the freeze touches only README rows).
3. `install.sh`/`install.ps1`: append the crates.io notice line (dist-repo side; this repo carries no installers — Plan 105).
4. `scripts/manual_release.sh`: retire (delete after the last manual use; the tag lane owns releases now).
5. `src/update_check.rs`: alert + upgrade messaging gains the crates.io route; `DIST_REPO` const stays while the binary lane lives.
6. Website (riir-dapps kat-service): FAQ flip ("Yes — `cargo install cargo-refine`; the prebuilt binaries carry the hosted contribution surface"), install matrix gains the cargo-install row, `heal_skill.md` rows; all four contract test files updated in the same commit; web-family conformance gate; deploy = 👤 owner touch-deploy lane + dapps deploy-ledger row.
7. In-repo md sweep (same-day-or-later, never before the lane is live): `README.md:29-30` + `:307`; `AGENTS.md` L59 section rewritten (the secrecy law becomes "the crate is public on crates.io; private siblings stay closed"; keep the leak-discipline halves that remain true), L41/L68/L161 lane wording; `.docs/01_orientation/foo_end_to_end.md`, `role_flows.md:99`, `07_examples/cargo_refine.md:15` (cargo install from the registry replaces `--path`); mozilla docs (readiness + evidence pack — crates.io downloads join T3 per GAP 11); **`BOUNDARY.md` drift row** (the distribution contract row flips — run via the boundary-guard skill, not ad-hoc); `HISTORY.md` record with the first-publish hash.

## Tasks

- [ ] **P0 — post-flip hygiene audit (blocks the first publish claim):** (a) secret-scan over HISTORY (`git log -p --all` through `cargo refine --secret-scan` or the dist lane's leak scanner), (b) hostname/URL audit of tracked files (T15(b) — internal hosts/runbook paths), (c) manifest reconciliation (`license = "MIT"`, drop the EULA `license-file`, fix the proprietary comment — 👤 EULA's fate for the binary archives: retire or keep-as-binaries-terms), (d) internal-docs posture decision recorded (AGENTS/HISTORY/.docs stay public by the flip — 👤 owner confirms or names redactions).
- [ ] **P1 — registry substrate legs (katgpt-rs lane):** refresh `katgpt-core` on crates.io (0.4.1 → current; the local tree has moved — API parity check before the publish posture pins `version`), confirm `katgpt-micro-belief` 0.2.1 still matches, decide `katgpt-percepta` in/out of the publish posture (recommend out ⇒ trim the row + `wasm_validator`).
- [ ] **P2 — the publish surgery + gate:** `scripts/cratesio_publish.sh` (deterministic manifest generation, zero-sibling standalone build at `default,fix_verify,update_check`, `cargo metadata` path-dep zero assertion, `cargo package --list` internal-doc audit, `cargo publish --dry-run`), wired as a separate `release.yml` job (the pinned build line untouched).
- [ ] **P3 — first publish + docs.rs verification** (👤 name + version + credentials before this runs); tag + release notes row.
- [ ] **P4 — old-lane deprecation execution** per the sequencing above (each step its own commit; tap/bucket archive 👤).
- [ ] **P5 — website flip (riir-dapps)**: copy + the four contract files + conformance gate + 👤 deploy + deploy-ledger row.
- [ ] **P6 — in-repo md sweep + `BOUNDARY.md` row + HISTORY record.**
- [ ] **P7 — evidence/grant notes:** crates.io download counts join the evidence pack (192 GAP 11 → 190 T3's pack); 190 T6's dist-README residue row updated (superseded by the deprecation, not the rewrite); katgpt-rs Research 003 amendment note carries the first-publish hash.

## Explicitly out of scope

- Publishing any private sibling (riir-kat, riir-neuron-db, riir-ai crates) — GAP 11's supersession is THIS repo only.
- Removing the prebuilt-binary lane in the same breath as the first publish — sequencing above is deliberate; `update_check` and no-toolchain users depend on it until 👤 retirement.
- Renaming the package beyond the crates.io crate-name decision (the lib name `riir_refine`, the bin name `cargo-refine`, and every internal path stay).
