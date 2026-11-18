# Issue 244 — install name `cargo-refine` → `riir-refine` (crates.io + Homebrew)
**Status:** open — brew half LIVE 2026-11-18 (tap `4ae9879`/`1754512`, dist `a6c8da1`); publish-script + upgrade-hint code landed (see commit row); crates.io half BLOCKED on a katgpt-core registry refresh + the owner's publish token (the only owner gate).

Filed 2026-11-18 on owner request ("can I change `cargo install cargo-refine` to `cargo install refine` or `cargo install riir-refine`, same for brew").

## Verdict

| candidate | crates.io | Homebrew | verdict |
|---|---|---|---|
| `refine` | TAKEN — rsalmei/refine (file renamer, 3.1.0) | TAKEN — homebrew-core `refine` (grammar checker) | ❌ impossible |
| `riir-refine` | FREE (checked 2026-11-18) | free in tap; no core collision | ✅ GOAT — matches the repo + `[package]` name |
| `cargo-refine` | ours (0.5.0, 0.5.1) | ours (tap) | legacy; stays as the BINARY name |

Package name and binary name are separate axes. The binary stays
`cargo-refine` forever, because cargo dispatches `cargo refine` to it. Only
the thing you pass to `cargo install` / `brew install` changes.

## Tasks

- [x] T1 Publish surgery decouples the package and binary names: `scripts/cratesio_publish.sh` `NAME=riir-refine` (package) + `BIN=cargo-refine` (fixed `[[bin]]`). Sibling-mode gate GREEN: stage `name = "riir-refine"`, `[[bin]] name = "cargo-refine"`, built bin `--version` OK, package list 742.
- [x] T2 Stale `KC_VERSION` default `0.4.2` → `0.5.0` (the version 0.5.0/0.5.1 shipped against; with 0.4.2, registry mode could not resolve at all).
- [x] T3 Upgrade hint `CRATESIO_UPGRADE` → `cargo install --force riir-refine`. `--force` is required because a box with the legacy `cargo-refine` package owns the same binary path. update_check + build_stamp tests 37/37 pass.
- [x] T4 Homebrew: tap `Formula/cargo-refine.rb` → `Formula/riir-refine.rb` (`class RiirRefine`) + `formula_renames.json {"cargo-refine": "riir-refine"}`. Verified `brew info riir-ai/tap/riir-refine` and `…/cargo-refine` both resolve to riir-refine 0.5.0. `release.yml` + `manual_release.sh` now write `Formula/riir-refine.rb`.
- [x] T5 Brew lines flipped: README, `.docs/01_orientation/foo_end_to_end.md`, dist repo README (`a6c8da1`), and riir-dapps kat-service source. The copy contract now exempts the exact word `riir-refine` (public MIT package name); `riir-ai/riir-refine` still fires. 636/636 pass. The live site needs no deploy (its `brew install cargo-refine` still resolves through the rename map); the new copy goes out with the next deploy.
- [ ] T6 **BLOCKER — katgpt-core registry refresh.** Registry-mode gate (`--version 0.5.2`) is RED at metadata: HEAD's riir-refine enables katgpt-core feature `attention_memory` (and possibly more — the resolver reports the first) that katgpt-core 0.5.0 on crates.io lacks. This is NOT caused by the rename; it would block a `cargo-refine` 0.5.2 too. Unblock: katgpt-core (+ any leaf crates) semver-checked bump and publish (owner token, leaf-first, Issue 221 P1 cascade order), then re-run `scripts/cratesio_publish.sh --version 0.5.2 --kc-version <new>` until green.
- [ ] T7 **Owner — first publish of `riir-refine`** (credential gate): `CARGO_REFINE_PUBLISH_CONFIRM=yes scripts/cratesio_publish.sh --ref origin/develop --version 0.5.2 --kc-version <T6>`. Then a smoke test on amo: `cargo install --force riir-refine` → `cargo refine --version`. Configure OIDC trusted publishing on the new crate.
- [ ] T8 After T7: flip every `cargo install cargo-refine` to `cargo install riir-refine`. Deliberately NOT flipped yet, so no public surface names an unpublished crate. Population (grep 2026-11-18): riir-refine README (×4), AGENTS.md, `.docs/01_orientation/{role_flows,foo_end_to_end,mozilla_grant_readiness}.md`, `role_coder_flow.walk.json`, `.docs/07_examples/cargo_refine.md`, the `release.yml` caveat (×2), the tap formula caveat, the dist `install.sh`/`install.ps1` tips + README; riir-dapps kat-service `front_page.rs` (×3), `heal_skill.md` (×2), `refine_capture_contract.rs`, `contract/front.rs`, `assets/role_coder_walk.html`, `assets/role_coder_flow.walk.json`, and `crates.io/crates/cargo-refine` links. Also `mozilla-video/src/project.rs` (owned by the video lane; that lane is notified). Then deploy kat-service.
- [ ] T9 After T7: publish a final `cargo-refine` 0.5.3 whose upgrade hint already reads `cargo install --force riir-refine` (built from the post-T3 tree with `--name cargo-refine`), so legacy installs get pointed to the new name. Do not yank the old crate; it stays as a pointer.
- [-] T10 Scoop bucket `cargo-refine.json` → `riir-refine.json` — out of the asked scope (cargo + brew); do it with T8 for consistency.

## Notes

- Dated records (HISTORY, closed issues, benches) keep `cargo-refine` as written at the time.
- The video lane (Zed thread `20639bed`, Plan 386) was mailed the new names and the not-yet-live caveat on 2026-11-18.
