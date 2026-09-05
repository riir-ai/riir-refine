# Plan 059 — kat-011 T0.2 onboarding CLI (login default-SSH, account welcome/history) + the corpus-count runtime fix (Proposal 009 F2 residue)

Status: **COMPLETE (2026-09-01)** — all tasks landed and validated; see the Landed
notes per task. Two work items bundled because they share the `cargo-heal` bin surface
and both trace to the KAT-proposal → issue conversion session:
Issue 061 T0.2 (the tracker was resolved + removed 2026-09-05; its record lives in
riir-clippy AGENTS.md "Issues 061 + 068" + git history) and
[Proposal 009](../.proposals/009_trained_clippy_drafter_moka_puct.md) §F2 item (ii).
Branch: `develop` (per global rule — no feature branches)

## Item A — 011 T0.2: the onboarding CLI slice (the only Phase-0 task native to this repo)

- [x] A1 **`login` default-SSH adoption** — a fresh machine with no kat account key
      adopts the machine's default SSH key as the account identity: the
      BLAKE3(pubkey)-derived account id then matches what the GitHub `.keys` key-match
      (011 ask 1's web wallet) will derive, so `cargo heal login` needs zero
      configuration. Landed as `KatAccount::login_default(dir)`:
      (1) an existing account key still wins → `Existing` (idempotence unchanged);
      (2) `RIIR_HEAL_KAT_KEY` (a private-key path) overrides the default location,
      failing LOUDLY when set but unusable (explicit config must not silently fall
      back);
      (3) otherwise adopt `~/.ssh/id_ed25519` (`HOME`/`USERPROFILE`) when it exists,
      parses as an unencrypted Ed25519 OpenSSH key — copied into the account dir with
      the standard 0600 discipline + pub sidecar + record, disclosed by printed source
      path (caveat #8's "print exactly which keys land where");
      (4) anything else (missing key, encrypted, non-Ed25519) → fall back to generate
      with the reason recorded in the outcome (the new `LoginOutcome::Generated`
      carries a `fallback_reason` — no new variant needed, the existing matches keep
      compiling).
      The copy (not reference) semantics match `login_import`: one code path, the
      key-file-is-the-authority invariant preserved. SSH-key rotation strands the
      account on the OLD identity — the same ops-assisted re-binding posture 011
      caveat #7 already documents for 007.
- [x] A2 **`account` welcome + `--history`** — `cargo heal account --history` prints
      the earned/spent view from LOCAL state: earned = the login grant projection
      (+ nothing else yet — honest), spent = the repo-local `kat_charge.json` burn
      (the existing meter), balance = the per-repo projection. The welcome line prints
      when the account is fresh (grant untouched by local burn). Every line obeys the
      caveat-#15 copy rule (no `$` beside a KAT figure) — pinned by a test that walks
      the rendered lines. The service half (live `/balance`, the all-synced +
      nodes-healthy gate from 011 §8.3, server-side history) is 061's T0.2b — it needs
      the dapps routes to exist and cannot be honestly rendered from here.

## Item B — Proposal 009 §F2 residue: the corpus projection must count at runtime, never by grep

- [x] B1 **`--corpus-stats` runtime counts** — F2 measured the docker series at 2
      while `entries()` returns 31 (the corpus is built through a `rule(...)` helper,
      so the `rule_id:` needle cannot see it; "any domain using a builder helper is
      equally suspect"). Fix: `run_corpus_stats` now prefers the COMPILED corpus
      builder per domain (`build_seed_corpus` / `build_perf_seed_corpus` /
      `build_kernel_seed_corpus` / `build_sec_seed_corpus` / `build_docker_seed_corpus`,
      each behind its own feature) and falls back to the source-walk needle only for
      domains not compiled into the binary. Provenance recorded per row:
      `runtime-corpus` (first-hand, compiled) vs `source-walk` (text needle,
      non-compiled fallback) — the printed table carries the source so a needle-
      derived number can never masquerade as a runtime count.
- [x] B2 **import precedence extended** — the jsonl import yields to first-hand
      stamps; the check covered only `"source-walk"`, so a `runtime-corpus` stamp
      would have been clobbered by a stale committed projection on the next
      `--import-series`. The yield check now covers both first-hand provenances;
      pinned by test (a `runtime-corpus` row survives a re-import carrying a
      different count).
- [x] B3 **today's projection row corrected** — after the fix, `--corpus-stats` was
      run and the committed `.benchmarks/corpus_history.jsonl` row for 2026-09-01
      updated to the runtime truth (docker 2 → 31). Historical rows stay as the
      honest record of what the broken instrument reported at the time — rewriting
      them would fabricate history (the docker count varied per batch; it is not
      reconstructible).

## Gates

- G1 `cargo clippy --all-targets` clean in the touched states; the bin + lib suites
  count-identical or explicitly extended (new tests for A1/A2/B2).
- G2 feature states: default + `latent_retrieval` (the bin's shipping set) — the
  runtime-count path compiles for clippy_lints/rust_perf/docker (all default-on) and
  the fallback path covers kernel_opt/sec.
- G3 copy rule: the account render test asserts no `$` adjacent to a KAT figure.
- G4 zero new dependencies.

## Landed (2026-09-01)

- A1/A2/B1/B2/B3 all landed. Validation (RUSTUP_TOOLCHAIN=1.97.0-x86_64-pc-windows-msvc,
  the box pin):
  - `cargo clippy --lib --bins --all-targets` — clean in BOTH states (default;
    `kat_service,latent_retrieval`), 0 errors, 0 lint warnings in touched files. The
    6 unused-variable warnings in `run_kat_stake`/`run_kat_vote` are PRE-EXISTING
    (verified via a stash-baseline clippy run — 6 at HEAD too).
  - lib tests: default **1008/0/0** (count-identical — the new tests are all
    feature-gated); `kat_service,latent_retrieval` **1247/0/1** (includes the 4 new
    kat_account tests + the new traj_store precedence test).
  - Standing gates re-run green: `kat_account_g9` 6/6, `kat_stake_client_g2s` 9/9
    (kat_transport), `kat_vote_client_g9gov` 5/5 (kat_transport),
    `kat_sync_payload_g4` 11/11 (kat_service,latent_retrieval), `kat_meter_g9` 11/11,
    `t049_t023_stop_verdict` 1/1 (latent_retrieval).
  - rustfmt --check clean on all three touched files (edition 2021).
  - Live smoke (env-isolated temp account dirs — the real account dir untouched):
    `login` with no SSH key → generated + disclosed fallback note; `login` with a
    seeded `~/.ssh/id_ed25519` → adopted with the source-path disclosure line and
    the account id = the adopted key's derivation; `account --history` → identity +
    welcome + the 4-line history, no `$` anywhere; `--history` without `account` →
    usage error; `--corpus-stats --json` contract intact.
  - `--corpus-stats` live: docker **2 → 31** (runtime-corpus, compiled), sec
    11 → **12** (the needle picked up today's P8 rule — correct), clippy_lints 45 /
    rust_perf 97 via runtime-corpus (needle cross-validation: identical),
    kernel_opt 422 via source-walk (kernel_heal off in this build). Today's row
    stamped to the store; the committed jsonl row corrected (B3).
- The `.plans/.highwater` malformation (`57\n58\n` — two lines) repaired in the same
  commit (found while bumping; the stray 57 line predates this session).
- Toolchain note: the default-features clippy/test runs DO NOT compile
  `kat_account` (module gated on `kat_service`) — validating the T0.2 work requires
  `--features kat_service` explicitly (the gate-coverage lesson, applied).
