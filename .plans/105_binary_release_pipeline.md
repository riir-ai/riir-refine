# Plan 105 — Binary-only release pipeline (Proposal 003 execution)

**Status:** IN PROGRESS — Phase 1 + Phase 2 landing this session; T1.3 (PAT) and T3.1 (first-release G1–G4) are owner/first-tag gated.

Governing record: [`.proposals/003_binary_only_distribution.md`](../.proposals/003_binary_only_distribution.md).
Ready-audit basis (2026-09-06 session): policy ready + unblocked, execution absent.
Leak pre-scan on the exact shipping set (release, default + `latent_retrieval`)
measured: 14,485 symbols exposed; 61 absolute `/Users/katopz/...` strings (55
registry + 6 sibling source paths + 1 `env!` bake); 0 secret-shaped strings;
0 unexpected hosts; corpus strings clean. Those three gaps land HERE as
measured-required fixes, not as the proposal's "optional" hardening.

## Phase 1 — Scaffolding (T1.x)

- [x] T1.1 `gist-rs/cargo-heal` created (public, 2026-09-07) + seeded on `main` (commit `157ea2d`): `install.sh`, `install.ps1`, capabilities-only README, no-source CI guard (fails on any tracked `.rs`). Canonical install URLs now resolve — verified HTTP 200 the day of seeding.
- [x] T1.2 `gist-rs/homebrew-tap` (seed `8812009`) + `gist-rs/scoop-bucket` (seed `585a94a`) created (public, manifest-only, same guard); the Formula / bucket manifest arrive as release-job PRs (T2.4).
- [x] T1.3 Fine-grained robot PAT as the org secret `RELEASE_PAT` — OWNER ACT — **DONE 2026-09-07**: minted fine-grained (`gist-rs`, the three public dist repos ONLY), stored as the ORG secret `RELEASE_PAT`, visibility `selected` → `gist-rs/riir-clippy` (verified `gh api orgs/gist-rs/actions/secrets/RELEASE_PAT --jq .visibility`). Permission correction: the text said contents RW only, but the tap/bucket steps call `gh pr create`, which requires **Pull requests: write** on a fine-grained PAT — the minted token carries contents + pull_requests RW; `release.yml` header comment corrected in the same change.

## Phase 2 — Pipeline (T2.x)

- [x] T2.1 `.github/workflows/release.yml`: tag `v*` + dispatch dry-run; 5-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-pc-windows-msvc`); sibling provisioning per rust.yml's pattern (katgpt-rs public `develop`; riir-ai + riir-neuron-db via `SIBLING_REPOS_TOKEN`); toolchain from `rust-toolchain.toml`.
- [x] T2.2 The three measured leak fixes (commit `35901c2`): `[profile.release] strip = true` (Cargo.toml); `--remap-path-prefix` in every matrix row's RUSTFLAGS (measured-REQUIRED — 61 absolute paths); the `env!("CARGO_MANIFEST_DIR")` bake in `src/bin/heal.rs::resolve_corpus_repo_root` made debug-only (remap cannot touch macro string literals; release compiles the fallback to `None`).
- [x] T2.3 Packaging: tar.gz (unix) / zip (windows), every archive carries the binary + `THIRD_PARTY_LICENSES.md` (cargo-about, config in `.github/about/`); one `SHA256SUMS` over all assets. **License gate validated locally 2026-09-07**: `cargo about generate` against the real shipping tree produced a clean 1,798-line doc (96 crates, 5 license families, all accepted) — commit `2e6f0cf`.
- [x] T2.4 `gh release create -R gist-rs/cargo-heal` (tag events) + Formula / scoop-manifest bump PRs against the tap/bucket via the same PAT. `workflow_dispatch` = dry run (build + scan + package only, no publish) — commit `2e6f0cf`.
- [x] T2.5 `scripts/binary_leak_scan.sh` as the pipeline's fail-loud gate: defined-symbol absence (dyld imports + the `__mh_execute_header` linker marker allowlisted — both measured on the stripped binary), `/Users/`+`/home/` machine paths, drive-letter paths (two-segment regex — `http://` substrings and compressed-byte noise measured as false positives and tuned out), secret shapes (`ghp_`/`github_pat_`/`AKIA`/`sk-`/`xoxb`). Negative probes: a planted `D:\a\...` string and an unstripped `.o` both FAIL the gate. The heal.rs fix closes the one source-level bake in the shipping set. Known out-of-set `env!` sites (both `kat_service`-gated, NOT shipping): `kat_lease_client.rs`, `kat_wetrun.rs` — re-audit when `kat_service` ships.
- [x] G2 pre-gate (local, 2026-09-07, commit `35901c2` + `2e6f0cf`): a local release build with strip + remap (`RUSTFLAGS=--remap-path-prefix /Users/katopz=/build`) passes `binary_leak_scan.sh` clean — 0 defined symbols (vs 14,485 unstripped), 0 machine paths (vs 61), 0 secret shapes.
- [x] **CI dry runs (workflow_dispatch, 2026-09-07)** — the dispatch path is the pre-tag proving ground; publish stays tag-gated:
  - Run `34072493969`: licenses ✓ (cargo-about@0.9.2 via install-action, 1m9s); macOS ×2 ✓; Windows ✓; musl ×2 ✗ (mlugg/setup-zig default download 404) → fixed via taiki-e/setup-cross-toolchain-action + uniform plain cargo (`64604af`).
  - Run `34073335938`: musl ×2 ✓ (3m45s / 4m28s — the zig-based cross toolchain compiles blake3's C clean); package ✗ at the LEAK SCAN, which caught two real classes: `C:\Users\runneradmin\.cargo\registry\...` panic-Location paths on Windows (workspace and CARGO_HOME sit on different drives — remap gap, fixed `ad71a85`) and a scan false positive (`n:/usr/bin` = the musl default-PATH constant; drive-letter arm now uppercase-only, which still catches the real `C:\Users` leak — verified against the saved artifacts).
  - Run `34074402607`: **ALL GREEN** — licenses, 5 builds, package (scan clean ×5, SHA256SUMS verifies ×5, licenses in every archive). Gate evidence recorded as [`.benchmarks/078_binary_dist_pipeline_dryrun.md`](../.benchmarks/078_binary_dist_pipeline_dryrun.md); only G1 (clean-machine install) remains, at the first tag.
- [x] Numbering incident (same session): `.plans/104` was consumed concurrently by the batch-124 agent (commit `52abfb5`) — this plan renumbered to 105 with all 11 in-repo citations, per the renumber rule.

## Phase 3 — GOAT gate (T3.x)

- [ ] T3.1 G1–G4 at the first real tag: **G1** clean-machine install on all five assets (`curl | sh` on fresh macOS/Linux incl. an old-glibc container for the musl statics; `install.ps1` + scoop on Windows) with `cargo heal --suggest` end-to-end; **G2** the per-binary scan green in CI (**measured ✓ — Bench 078**); **G3** `publish = false` intact + all private repos still PRIVATE + repo suite green (**partials ✓ — Bench 078**); **G4** SHA256SUMS verifies for every asset + `THIRD_PARTY_LICENSES.md` present in every archive (**measured ✓ on the dry-run artifact — Bench 078**). Formal record update in `.benchmarks/` once a release actually exists to gate.
- [ ] T3.2 (owner-gated chain): set `RELEASE_PAT` (T1.3) → tag `v0.1.0` → the publish job creates the release + opens the tap/bucket PRs → merge them → run G1 → append the G1 result to Bench 078.

## Notes

- **Asset naming**: `cargo-heal-<tag>-<target>.tar.gz|.zip` + `SHA256SUMS` + `THIRD_PARTY_LICENSES.md`. Installers resolve the latest tag via the GitHub API (version-embedded names; no `latest/download` alias to maintain).
- **No `--locked`**: `Cargo.lock` is gitignored here (floating siblings churn it — rust.yml note), so release builds resolve fresh; `SHA256SUMS` is the pin. Recorded risk, not a defect.
- **Musl statics via cargo-zigbuild + zig**: `cross` cannot see the `../katgpt-rs`-style sibling path deps (it mounts only the project dir into its container), so the linux rows use zigbuild, which runs host cargo and only swaps the linker.
- **T1.3 scope deviation, recorded**: proposal T1.3 said "contents:write on cargo-heal ONLY"; T2.4's tap/bucket bump PRs need the same token to write those two repos, so the PAT scopes all THREE public repos. Still zero private access, zero source read.
- **Canonical install copy** (owned by Proposal 003, audited by `tests/install_copy_parity.rs`): `https://raw.githubusercontent.com/gist-rs/cargo-heal/main/install.sh` and `.../install.ps1` — now actually resolvable: the dist repo exists and is seeded on `main`.
