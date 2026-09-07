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

- [x] T1.1 `gist-rs/cargo-heal` created (public, 2026-09-07) + seeded: `install.sh`, `install.ps1`, capabilities-only README, no-source CI guard (fails on any tracked `.rs`).
- [x] T1.2 `gist-rs/homebrew-tap` + `gist-rs/scoop-bucket` created (public, manifest-only, same guard); the Formula / bucket manifest arrive as release-job PRs (T2.4).
- [-] T1.3 Fine-grained robot PAT as the org secret `RELEASE_PAT` — OWNER ACT (fine-grained PATs cannot be minted via the API). Scope: repository access = cargo-heal + homebrew-tap + scoop-bucket ONLY, permission = contents read/write. All three repos are public and source-free, so the token can neither read nor write any source anywhere. Publish job fails loud without it.

## Phase 2 — Pipeline (T2.x)

- [x] T2.1 `.github/workflows/release.yml`: tag `v*` + dispatch dry-run; 5-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-pc-windows-msvc`); sibling provisioning per rust.yml's pattern (katgpt-rs public `develop`; riir-ai + riir-neuron-db via `SIBLING_REPOS_TOKEN`); toolchain from `rust-toolchain.toml`.
- [x] T2.2 The three measured leak fixes: `[profile.release] strip = true` (Cargo.toml); `--remap-path-prefix` in every matrix row's RUSTFLAGS (measured-REQUIRED — 61 absolute paths); the `env!("CARGO_MANIFEST_DIR")` bake in `src/bin/heal.rs::resolve_corpus_repo_root` made debug-only (remap cannot touch macro string literals; release compiles the fallback to `None`).
- [x] T2.3 Packaging: tar.gz (unix) / zip (windows), every archive carries the binary + `THIRD_PARTY_LICENSES.md` (cargo-about, config in `.github/about/`); one `SHA256SUMS` over all assets.
- [x] T2.4 `gh release create -R gist-rs/cargo-heal` (tag events) + Formula / scoop-manifest bump PRs against the tap/bucket via the same PAT. `workflow_dispatch` without the `publish` input = dry run (build + scan + package only, no publish).
- [x] T2.5 `scripts/binary_leak_scan.sh` as the pipeline's fail-loud gate: symbol-table absence, `/Users/`+`/home/` machine paths, secret shapes (`ghp_`/`github_pat_`/`AKIA`/`sk-`/`xoxb`). The heal.rs fix closes the one source-level bake in the shipping set. Known out-of-set `env!` sites (both `kat_service`-gated, NOT shipping): `kat_lease_client.rs`, `kat_wetrun.rs` — re-audit when `kat_service` ships.
- [x] G2 pre-gate (local, 2026-09-07): a local release build with strip + remap (`RUSTFLAGS=--remap-path-prefix /Users/katopz=/build`) passes `binary_leak_scan.sh` clean — `nm` reports no symbol table, 0 machine-path strings, 0 secret shapes.

## Phase 3 — GOAT gate (T3.x)

- [ ] T3.1 G1–G4 at the first real tag: **G1** clean-machine install on all five assets (`curl | sh` on fresh macOS/Linux incl. an old-glibc container for the musl statics; `install.ps1` + scoop on Windows) with `cargo heal --suggest` end-to-end; **G2** the per-binary scan green in CI; **G3** `publish = false` intact + all private repos still PRIVATE + repo suite green; **G4** SHA256SUMS verifies for every asset + `THIRD_PARTY_LICENSES.md` present in every archive. Record in `.benchmarks/` (next free number) once a release actually exists to gate.

## Notes

- **Asset naming**: `cargo-heal-<tag>-<target>.tar.gz|.zip` + `SHA256SUMS` + `THIRD_PARTY_LICENSES.md`. Installers resolve the latest tag via the GitHub API (version-embedded names; no `latest/download` alias to maintain).
- **No `--locked`**: `Cargo.lock` is gitignored here (floating siblings churn it — rust.yml note), so release builds resolve fresh; `SHA256SUMS` is the pin. Recorded risk, not a defect.
- **Musl statics via cargo-zigbuild + zig**: `cross` cannot see the `../katgpt-rs`-style sibling path deps (it mounts only the project dir into its container), so the linux rows use zigbuild, which runs host cargo and only swaps the linker.
- **T1.3 scope deviation, recorded**: proposal T1.3 said "contents:write on cargo-heal ONLY"; T2.4's tap/bucket bump PRs need the same token to write those two repos, so the PAT scopes all THREE public repos. Still zero private access, zero source read.
- **Canonical install copy** (owned by Proposal 003, audited by `tests/install_copy_parity.rs`): `https://raw.githubusercontent.com/gist-rs/cargo-heal/main/install.sh` and `.../install.ps1` — now actually resolvable: the dist repo exists and is seeded on `main`.
