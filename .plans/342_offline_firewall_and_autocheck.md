# 342 — the offline firewall + the run-start auto-check + the settings.toml surface

**Status:** DONE — code 530979967; distributed m3+amo (aa2fbb491 build) + 4090 (on-box build); e2e green on all three rigs (bmo skipped per the ask), 2026-11-05

The owner's five asks (2026-11-05): (1) auto-check app + corpus before any
`cargo refine`; (2) `cargo refine off --auto-update` with the same on/off
shape for the other auto features (`--auto-upgrade` …); (3) `cargo refine
offline` — a HARD firewall the config cannot override; (4) `cargo refine
online` — clear it and remind the user what their config chose; (5) the
saved config is a well-commented TOML. Then re-distribute + e2e every rig
(skip bmo).

## Substrate consumed (never rebuilt)

- `offline.rs` — the one predicate every network lane consults (8 heal.rs
  call sites + the points verbs + the lib lanes). The marker EXTENDS it;
  no parallel gate is created.
- `command_service_url()` — the ONE kat URL seam (50 call sites): one
  consult covers update/sync/login/topup/buy/corpus-pr/replay/genesis.
- `kat_mining_consent::default_dir()` — the machine-local dir resolution
  (env `RIIR_REFINE_KAT_ACCOUNT_DIR`, XDG/APPDATA fallbacks) — the
  settings.toml + the offline marker live beside the consent records.
- `run_update_surface_hooks` + `maybe_auto_corpus` (Issue 153 T4) — the
  run-start legs already exist; this plan PROMOTES the corpus leg to
  default-on (demote-only env, the env-switch law) and adds the settings
  gates.
- `first_run_consent` / `kat_stats_consent` — the atomic-write +
  never-panic-read file patterns the settings module follows.
- No TOML crate dep (the repo deliberately avoids one): a tiny hand parser
  over flat `[section]` + `key = bool` lines, comment-preserving on edit.

## Tasks

- [x] `src/refine_settings.rs` (ungated): `Settings { app_check,
      corpus_refresh, auto_upgrade }`, defaults on / off / off,
      `settings.toml` template (well-commented), comment-preserving
      `set()`, `flag_keys()` mapping (`--auto-update` → app_check +
      corpus_refresh; `--auto-corpus`; `--app-check`; `--auto-upgrade`),
      in-module unit tests (parse, defaults, comment preservation,
      malformed → defaults + warning, template-well-commented gate).
- [x] `src/offline.rs`: the marker file (`<dir>/offline`) —
      `offline() = marker || env` (env can NEVER clear the marker — the
      firewall law), `refusal_line` names the marker + `cargo refine
      online` when the marker is the cause (the stable `offline: <lane>
      skipped — ` prefix stays), marker tests (marker + `=0` env still
      offline).
- [x] heal.rs verbs: `offline` / `online` / `off --<flag>` / `on --<flag>`
      (ungated, local-only, parsed like the points verbs; hook exempt like
      `upgrade`/`--version`), `online` prints the settings reminder, bare
      `off`/`on` prints status + valid flags (exit 2).
- [x] heal.rs firewall consult at `command_service_url()` (exit 2 naming
      the marker + the `online` remedy) — covers every kat dial lane.
- [x] heal.rs hooks: app_check + auto_upgrade + corpus_refresh gates from
      settings (corpus leg PROMOTED to default-on, `RIIR_REFINE_AUTO_CORPUS=0`
      demote-only; `auto_upgrade` = config OR the env, still opt-in);
      `off --auto-update` warns when auto_upgrade is still on.
- [x] usage_text lines for the four verbs; AGENTS.md CLI UX contract
      bullets (the settings.toml shape + the promotion note).
- [x] clippy `-D warnings` at the build posture + fmt + tests green;
      commit + push (fix/feat prefix, Session line).
- [x] Distribute: m3 rebuild (symlink target in place), amo scp (same
      triple), 4090 gitsync + on-box build + cp install.
- [x] E2E every rig (skip bmo): version → `off --auto-update` (settings
      file + comments asserted) → bare `off` status → `on --auto-update` →
      `offline` marker → `update` refuses (firewall) → `online` reminder →
      `update` works again.

## Non-goals / notes

- No benchmark lane: the delta per healing run is one small file read +
  one stat (microseconds); the network legs keep their existing 12h caches
  and budgets. The promotion is an owner mandate (the ask itself), the
  charge plane stays the tier gate.
- ⛔ CORRECTED 2026-11-05 (was wrong here): the shipped release set does
  NOT compile the corpus leg out — it carries `points` (since aea8e270c),
  and the leg's gate is `any(kat_transport, points)`; release users get
  the update verb (charge-free points read), the corpus verb, and the
  auto-corpus hook. The plan's earlier "changes nothing for release
  users" note misread the posture; the honest residue was the four stale
  strings saying kat_transport-only — fixed in f5cf676af.
- `service_url.json` (config set) and `settings.toml` are different files
  in the same dir — no collision, no migration.
