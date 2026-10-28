# Issue 188 — `cargo refine migrate` for a NORMAL user: GitHub device login proves the login, the browser files the issue

**Status:** OPEN — filed 2026-10-28 (owner steer: "act like a normal user — `cargo refine migrate …` should provide GitHub login or other auth, only to sign/prove, not to file the issue"). Supersedes the `gh` leg of Issue 187 T4. One owner act: tick **Enable Device Flow** on the GitHub OAuth app whose public client id the CLI bakes in (T0).

## Problem (measured on the CLI, 2026-10-28)

Today a migration is an expert ritual: `cargo refine kat account migrate --github <login> --old-key <path> [--attest-cosign …]`, and a binding-move filing refuses `403 login_unproven` unless the NEW key is already listed on `github.com/<login>.keys` (Issue 144) — or the user ran the provider-bind lane first (Issue 146), which has NO CLI verb. The issue itself is filed through `gh` when logged in, else a printed URL (Issue 187 T1).

## Decision

Normal-user shape — one command, two browser touches, nothing installed:

1. `cargo refine migrate` (top-level alias of the `kat account migrate` mode).
2. **Prove the login with the GitHub device flow** (riir-auth `provider_login`, riir-kat `kat_recovery_client::github_device_login`): the CLI prints `open https://github.com/login/device, enter ABCD-1234`; the user approves in the browser they are already signed into. Scope `read:user` — the token can NOT file issues or react (it cannot cast the 👍), lives in memory only, never persisted or logged. The login comes from that identity — no `--github` typing.
3. If the new key is not on `.keys`, the CLI runs `nonce_flow` → `bind_flow` (the new key SSHSIG-signs the numeric-id claim; the worker re-checks the token) — exactly the Issue 146 pbind proof, now automatic. A listed key skips it.
4. Sign the filing with the old + new keys (unchanged), POST it.
5. **File the issue in the browser** — the CLI opens/prints the prefilled `issues/new` URL; the user (already signed into github.com) clicks Submit. `gh` stays an optional fast path, never required. (GitHub requires a signed-in session to create any issue — the browser session is that sign-in; the CLI never holds a write credential.)
6. The status read pins the issue; the owner 👍 → the cron releases.

## Tasks

- [ ] T0 (owner, 1 min): GitHub → Settings → Developer settings → OAuth Apps → the KAT wallet app (`Ov23liAe58wk2byrYeOv`, already the wallet's public id) → **Enable Device Flow** ✓. (Or register a dedicated "KAT CLI" OAuth app and give its client id.) The id is PUBLIC by design.
- [ ] T1 Bake the default public client id (env `RIIR_AUTH_GITHUB_CLIENT_ID` still overrides); `cargo refine migrate` alias; derive `--github` from the device identity (an explicit `--github` must MATCH it or refuse).
- [ ] T2 Auto-pbind when the new key is unlisted (`.keys` miss → nonce/bind flows), typed refusals surfaced in plain words; the token dropped after the bind.
- [ ] T3 Default the browser path: print + `open` the prefilled URL; `gh` only when already logged in as the same login.
- [ ] T4 Tests: fake device transport (no network), listed-key skip, unlisted → bind → file order, `--github` mismatch refuses, the token never reaches disk/logs.
- [ ] T5 Devnet rehearsal as a normal user (replaces Issue 187 T4's `gh` leg): one filing end to end, owner 👍, cron release.
