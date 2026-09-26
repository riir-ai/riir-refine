# Plan 175 — Issue 136: admission-gated structural self-mutation lane (reef distill)

**Status:** COMPLETE — landed at `503f03b5` (2026-09-27). All tasks done; issue 136
removed per the noise-reduction rule (HISTORY.md 2026-09-27 row is the durable
record). No auto-serve kind activated — owner call reserved.

## Source

- Issue: `.issues/136_reef_admission_gated_structural_self_mutation.md`
- Research: `.research/213_reef_continual_self_improving_agent_infra.md` (reef @
  `ecec4e20b29e7dcbd96150d4b4f1a80b146e120d`, quotes verbatim-verified at distill time)
- Substrate-first verification (2026-09-27): grep for `structural_admission|shape_gate|
  admission_gate|proposal_kind|review_class|commit_chain` over `src/` → only unrelated
  hits (score-bench `admission_gate`, kat_wetrun host admission); `checkpoint` hits are
  fixer_regate arm-C prose. The lane is genuinely absent. DRY reuse:
  `active_set::KNOWN_DOMAIN_KEYS` (the 7-domain retrieval vocabulary, drift-pinned by the
  `latent_matcher` test) for domain validation; blake3 + serde_json already main deps
  (zero new deps).

## Design decisions (binding)

- **Ungated pure vocabulary** (`src/structural_admission.rs`): kinds, review classes,
  per-kind shape gates, secret-shape scan, chain state machine, canonical artifact
  hashing — pure functions over plain data (the `active_set`/`demonstrability_gate`
  precedent). Fully lib-tested at default features (no green-zero surface).
- **Feature `structural_admission` (opt-in)** gates the persistent store binding
  (`src/structural_admission_store.rs`): writes `.heal/structural/artifacts/<blake3>`
  + `heads.json` CAS. Nothing persists structural state without the feature — this is
  the issue's "behind an opt-in feature" made real; activation of any auto-serve kind
  stays a separate owner call.
- **Review classes keyed on KIND** (reef P3): Playbook / Fixture / ConfigKnob /
  ActiveSetMarker → ALWAYS_REVIEWED (each executes or changes executed behavior; the
  issue's own examples); CorpusRule → auto-servable class (data-shaped; oracle + bench
  gates stay the quality layers above). The finding that 4 of 5 kinds are privileged is
  recorded in the module doc, not smoothed over.
- **Flat-tree invariant** enforced by the type: one `ProposalPayload` = exactly one edit;
  no list-of-edits variant exists to construct. Payload size cap 1 MiB (the reflexer
  vessel cap, reused as a shape bound).
- **P2 secret shapes fold in now** (the issue's fixture-gate example names the class):
  prefix+run-length scanner, no regex dep. Directive-shaped text stays folded-off
  (no prompt-bearing store persists today — Research 213 verdict).
- **Two-head chain** (reef P5): `current` + `checkpoint` heads, CAS advance
  (expected-state compare), `prepare_live` builds a live child of the checkpoint
  WITHOUT moving a head; rollback = a CAS repoint of `current` (a commit, not a revert).
  Artifacts content-addressed (BLAKE3), immutable (existing-file byte re-verify).

## Tasks

- [x] Substrate-first verification greps + DRY reuse decision (recorded above)
- [x] Write this plan (T0)
- [x] `src/structural_admission.rs` — vocabulary: `ProposalKind`, `ReviewClass`,
      `ProposalPayload` (5 kinds), `AdmissionCtx`, `admit()`, refusal taxonomy,
      `scan_secret_shapes` (hand-rolled, boundary-guarded prefixes),
      `canonical_artifact_bytes` + `artifact_hash` (blake3), `ChainState` /
      `ChainHead` / `advance` / `install_checkpoint` / `prepare_live` pure CAS
      state machine
- [x] Inline lib tests (ungated): review class per kind (all 5 enumerated),
      corpus gate bench-spec-or-NotBenchable, unknown-knob refusal, unknown-domain
      refusal, token-shape refusal, secret-shape fires/near-miss-no-fire per class,
      payload-cap refusal, chain CAS ok + mismatch + seq monotone + checkpoint CAS +
      prepare-live-no-head-move + repoint-rollback, heads JSON round-trip
- [x] `src/structural_admission_store.rs` (gated): `ChainStore::open` /
      `write_artifact` (content-addressed, immutable re-verify) / `read_artifact` /
      `load_state` (absent = default) / `cas_save` (re-load + full-state compare);
      temp paths pid-suffixed (shared_temp_path rule)
- [x] Feature row in Cargo.toml (`structural_admission = []`, implies nothing —
      blake3/serde_json are main deps) + lib.rs registration with posture comments
- [x] Gates: `cargo clippy --all-targets` default + `--features structural_admission`
      + `--all-features` (healed via `cargo heal` where mechanical); lib tests at
      default AND `--features structural_admission` (counts asserted, not exit codes);
      `cargo check --no-default-features` (the gated store must compile to nothing)
- [x] Docs: `.docs/10_self_evolve/structural_admission.md` (bare slug) + index line in
      that folder's README; feature bullet in AGENTS.md; Research 213 one-line landing
      status update
- [x] Close: HISTORY.md entry with commit hash; `rm .issues/136_*` (noise-reduction)
      in the same commit as the docs that carry the record; commit + push
