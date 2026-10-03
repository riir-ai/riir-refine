# Issue 144 — gist-rs/cargo-refine public README + `cargo refine` help: stale names, stale caveats, domains the release cannot run
**Status:** OPEN — owner-gated remainder only (T5 licence terms; T6 deferred to riir-kat). T1-T4 DONE 2026-10-03 (web trust audit, riir-ai Issue 1028)

**Priority:** P0 (instructions/claims that are wrong for the current release) + P2 (help-text jargon).
**Audience:** developer installing from refine.gist.rs → GitHub (the "cargo-refine on GitHub —
install, docs, releases" link is the site's only docs link).
**Owner:** riir-refine (Proposal 003 binary-only distribution — the dist repo
`gist-rs/cargo-refine` is this repo's public face).

## Evidence (2026-10-03; release `v0.2.0`, published 2026-10-01, downloaded and run on the M3)

| # | Where | Text | Verdict | Evidence |
|---|---|---|---|---|
| 1 | GitHub repo description | "cargo-heal — binary releases" | **STALE** | `api.github.com/repos/gist-rs/cargo-refine` `.description`; product is "Refine" (riir-dapps `a3029a2`) |
| 2 | README L3-4 | "A modelless code **healer** … **heal** time" + headings "What it heals", "The KAT heal network" | **STALE naming** | public product name is "Refine — the code fixer that learns" |
| 3 | README L40-50 | "Caveat — v0.1.2 / v0.1.3 binaries default `RIIR_HEAL_KAT_SERVICE_URL` to `heal.gist.rs` … until v0.1.4 ships" | **STALE** | v0.1.4 (2026-09-09) and v0.2.0 shipped; v0.2.0 defaults to `https://ai.gist.rs` (`strings cargo-refine` → `learn: https://ai.gist.rs`; `src/bin/heal.rs:1745-1753` `MAINNET_SERVICE_URL`). A new user is told to export an env var they do not need |
| 4 | README L147-151 | "Service plane … `https://heal.gist.rs` … not publicly resolvable yet" / v0.1.0-0.1.2 history | **STALE** | same; belongs in a changelog, not the TL;DR path |
| 5 | README L94-95, L112 | "Your meter and charge ledger live under **`.heal/`**" | **STALE** (verify) | CLI writes `.refine/outbox/` (`src/bin/heal.rs:775,1089-1090`); katgpt-rs `d12868b65` / Proposal 018 retired `.heal`. Verify every `.heal/` path the README names |
| 6 | README L76 | KAT account "default … **devnet**" | **STALE** (verify) | v0.2.0 default service host is the mainnet apex `ai.gist.rs` |
| 7 | `cargo refine --suggest` help: `--domain <auto\|clippy\|perf\|kernel\|sec\|docker\|dist>` | **Advertises domains the release refuses** | `--domain sec` → "requested but its Cargo feature is not enabled (have: [clippy, perf, docker, dist])"; same for `kernel`. Print only the compiled domains (the binary knows its feature set — `--version` prints it) |
| 8 | `cargo refine --help` | prints `unknown flag "--help"` then usage | **DX bug** | v0.2.0, M3 |
| 9 | usage text | "(Plan 005)", "(050 T2)", "(089 T1+2)", "(Issue 130 T1+T3+OOD …)", "(Proposal 007 staking …)", "needs --features score_bench" | **jargon** | internal plan/issue ids and dev-only flags in the public binary's help; split into `--help` (user verbs: dry run, `--fix`, `--mine`, `--unmine`, `login`, `account`, `--info`) and `--help-all` |
| 10 | binary strings | `https://testnet.ai.gist.rs` | **STALE host** | testnet torn down 2026-10-03 (riir-dapps `9a22231`); DNS no longer resolves. Harmless if only an env-name allow-list, but prune at the next release |
| 11 | README + site | "seven rule domains" (refine.gist.rs hero) | **FALSE for the release** | released feature line: `clippy_lints, dist, docker, rust_perf` (+ `rustc_errors`). Filed for the page in riir-dapps Issue 110; the README should state the shipped set too |
| 12 | Install lines (curl/iwr/brew with `brew trust`/scoop) | TRUE | every URL 200; tarball = `cargo-refine` + `THIRD_PARTY_LICENSES.md`; scoop manifest `cargo-refine.json` present |
| 13 | Licence | — | **MISSING** | `license: null` on the dist repo; no EULA/terms for a closed-source binary that moves service credit |

## Found while fixing (2026-10-03)

| # | Where | Verdict | Fix |
|---|---|---|---|
| 14 | `--mine` explainer, refine.gist.rs glossary/FAQ, miner role flow + SVG, umbrella glossary, agent SKILL.md | **FALSE privacy claim** — "rule verdicts and counters, never your source". The sync payload's `shape`/`fix` fields ARE the replaced code span and its replacement (`kat_sync_payload.rs` `PayloadRow`; `record_traj(restore, replacement)`), secret-scanned, row dropped on any finding, no paths, own-repo src/tests/benches/examples only | riir-refine `a25a0a4a`; riir-dapps `2b6852d`, deployed apex `bc08bd93` (ai + refine.gist.rs) + devnet `aac7f186`, curl-verified |
| 15 | `--unmine` / `--purge` | **BUG** — deleted `.refine/kat_sync_watermark.json`, a file nothing writes; sync saves `kat_sync_state.json`, so the watermark survived every opt-out | `595d0eef` (+ regression test) |
| 16 | `login` help text | STALE — key dir named `~/.config/riir-heal` | `595d0eef` → `~/.config/riir-auth` |

## Proposed fix

- Rewrite the dist README top half for v0.2.x: Refine naming, no env-var caveat, shipped domain
  list, `.refine/` paths, link to refine.gist.rs; move v0.1.x history to `CHANGELOG.md`.
- Repo description → "cargo-refine — binary releases of Refine, the code fixer that learns".
- `--help` accepted; user help vs `--help-all`; `--domain` choices printed from the compiled set.
- Add a LICENSE/EULA file to the dist repo (owner decision on terms).
- Prune `testnet.ai.gist.rs` from the next release.

## Tasks

- [x] T1 dist README rewrite for v0.2.x (+ CHANGELOG.md for v0.1.x caveats) — gist-rs/cargo-refine
      `5802123`, every claim checked against v0.2.0 source. Corrected beyond the evidence table:
      the free grant was printed as 100,000,000 KAT (it is 100 KAT = 100,000,000 µKAT, and signup
      now grants TUNA — riir-kat `kat_units.rs:344`); the FAQ said `--fix` without `--write`
      only reviews (it writes; bare `cargo refine` is the dry run); the kill switch was spelled
      `RIIR_HEAL_KAT_METER` (only `RIIR_REFINE_KAT_METER` is read); the account key is under
      `~/.config/riir-auth/`, not `riir-heal/`.
- [x] T2 repo description renamed — "cargo-refine — binary releases of Refine, the code fixer
      that learns", homepage https://refine.gist.rs (`gh repo edit`, 2026-10-03).
- [x] T3 `--help` flag + user/advanced help split; no internal plan/issue ids in user help —
      `69727010`: `--help`/`-h` → short user page on stdout, exit 0 (user verbs, compiled domains,
      `cargo refine --help-all` pointer); `--help-all` → the full reference, exit 0; the arg-error
      page is unchanged (exit 2). `tests/heal_usage_flag_parity.rs` now audits both pages.
- [x] T4 `--domain` choices derived from compiled features — `69727010`:
      `DomainId::{is_compiled, compiled, from_label}` (same feature gates the matcher indexes
      under); a known-but-absent domain refuses at parse time naming this build's set
      (`--domain sec is not in this build (this build has: auto|clippy|perf|docker|dist)`).
      Ships with the next release; v0.2.0 still prints the old text.
- [ ] T5 LICENSE / EULA in the dist repo (owner)
- [-] T6 drop `testnet.ai.gist.rs` from the next release — DEFERRED to riir-kat: the host is
      `TESTNET_SERVICE_URL` in riir-kat `src/kat_service_urls.rs`, one of the three PINNED
      `devnet|testnet|mainnet` aliases (`kat_service_config.rs` `PINNED_NETWORKS`, pinned by tests
      there). Removing a pinned network alias is a protocol-tier change for every consumer, not a
      refine help-text edit; re-point or drop it in riir-kat if testnet stays down.
