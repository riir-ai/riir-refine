## HISTORY.md — riir-refine

Compacted 2026-10-16: every `##` heading kept verbatim, one compact entry per record; the full pre-compaction text is in git history (pre-compaction HEAD cf0f52dbb180). Operational rules live in `AGENTS.md`; removed issue files: git history.

## 2026-10-17 — Issue 166 CLOSED: the `tuna` verb names the account state, not the deployment, on the absent pair (file removed per the noise-reduction rule — this row is the durable record)
- The `/history` wire omits the zipped `tuna_balance_micro` pair when the account holds no SPENDABLE trial credit — a consumed free tier (the KAT-grandfathered fleet account `c79c…5345` on mainnet) and a genuinely pre-TUNA worker read identically, and the None-arm blamed the deployment ("pre-TUNA deployment") for the common case. Fix: `tuna_none_line(granted)` splits on the response's unified claim predicate — consumed names the once-per-account law; unclaimed names the automatic claim path (`cargo refine account` / `--mine` sync). Test `tuna_none_line_tests`; clippy clean at `latent_retrieval,kat_service,kat_transport`. Family: riir-dapps `9582bf8` (the grant-badge surfaces — same raw-flag-vs-claim-predicate defect class; live posture + deploy note in dapps HISTORY.md 2026-10-17).

## 2026-10-16 — Issue 164 CLOSED: the install-probe plain-copy hazard — installers now symlink-aware (file removed per the noise-reduction rule — this row is the durable record)
- Hazard: `install.sh`/`install.ps1` (gist-rs/cargo-refine) replaced a developer's `~/.cargo/bin/cargo-refine` symlink with a plain release-cut copy; later local rebuilds propagated nothing (measured both boxes 2026-10-13). Four arms: (1) `cargo refine upgrade` swap path VERIFIED non-clobbering (rename moves the link into `.bak-<version>`); (2) dev-box re-link/re-copy convention on `.plans/223` §R6 + `.plans/105` Notes (`26e935e5`); (3) upgrade-path courtesy notice `UpgradeReport.replaced_symlink_target` + `symlink_notice()` (`59bbcd0f`); (4) the dist-repo fix gist-rs/cargo-refine `d9a75b3` — a symlinked destination is moved aside as `cargo-refine.symlink.bak` with a restore-command notice, plain files unchanged. Probed against v0.3.1: install.sh on macOS and install.ps1 on the 4090 (sandboxed USERPROFILE), symlink + plain arms each. AGENTS.md §Production adoption UNLANDED line replaced.

## 2026-10-12 — Issue 160 CLOSED: CRLF line-ending contamination — all 7 legs normalized, workspace doc-surface census reads 0/7 (file removed per the noise-reduction rule — this row is the durable record)
- Six legs landed 2026-10-11 (refine `7fbca3a7b` 82 · reflex `9e517d2` 2 · reflexer `68cdaae` 1 · ai `ff56c5e113` 26 · katgpt-rs `f2a4fd5a7` 23 · infer `e7b0e49` 6 — all batch-verified endings-only, census-at-HEAD 0) + final leg 2026-10-12 riir-train `afc0a249` (28 files, `git diff --ignore-cr-at-eol` EMPTY, census-at-HEAD 0; sibling's out-of-scope dirty bench WIP (297) untouched). TOTAL: 166 doc-surface files normalized workspace-wide, 0 content bytes moved — the repaired set is the protected baseline (the cycle-13 re-scan verdict extends). Owner-gated OPEN: whether to arm the standing ratchet gate (doc-surface CRLF count must not increase; armable at 0 — normalize-first-then-ratchet was why a hard ceiling was wrong day one). Advisory: the trainer's 3 untracked CRLF-at-birth .rs files in katgpt-rs Plan-621 WIP — normalize before commit.

## 2026-10-12 — Issue 162 CLOSED: formatting-invariant dense retrieval — T2 measured, T3 decided NOT ARMED (no transferable formatting subspace) (file removed per the noise-reduction rule — this row is the durable record)
- T1 `cea4d679` (cycle-22): frozen pair dataset `assets/format_pairs_v1.jsonl` — 4,213 pairs / 37 families / 1.48 MB / sha256 `85dea25b…46e79e` (mined via the existing `--dump-trajectories` export; uninlined_format_args 72% dominance); T2 `e6c02dae`: report-only `examples/formatting_flip.rs` (feature `basis_cert` = katgpt-core `basis_split`); record `.benchmarks/115_issue162_format_flip_t2.md`. Headline (certified 1,935 / verified-lived 281 / all 4,213): fused pair flip 44.1%/68.0%/35.3% vs the 0–1.4% whitespace floor; population A channels fused 44.1% / full8 99.3% / struct4 79.1% — BM25 already carries formatting invariance, churn rides TOKEN changes. The k=1 basis is a giant-Δ outlier axis (λ₁ share 0.994–0.999, calibration ≈ holdout leakage 0.975–0.9967) — NO transferable formatting subspace. T3 NOT ARMED (a projected dense-KNN lane over this basis would be theater); T5 rider in Bench 115 DECLINED until a triage consumer asks; T4's gate list written if the lane re-opens (lane closed, gates not armed). Standing declines (owner gate G4/G5, 2026-10-12): `basis_split` promotion VACUOUS — both consumers (reflex, refine) opt-in, no default-path consumer; re-open when a hot-path consumer arms.

## 2026-10-12 — Issue 161 CLOSED: module-applicable pool candidates — pool birth point 3 (the proposer member) + the `rewrite` merge strategy + the behav driver repairs; T1.6 re-computed PASS (file removed per the noise-reduction rule — this row is the durable record)
- Landed `1afd5d15`: pool widened with proposer members (POOL-ONLY, `heal_with_pool.result` bit-identical to `heal` — the R6 seam contract; plain `heal` pays one branch, G4 untouched) + the `rewrite` whole-module merge (a file-preserving response merges WHOLE) + the behavioural driver's float/int-slice extension; `tests/t40_fixer_regate_harness.rs` repaired (broken since the 09-18 REACHABLE re-pick, controls repointed at `manual_strip`). Bench 114 (`.benchmarks/114_issue161_pool_widened_harvest.md`): pool≥2 643→1446, deciding 3 → 1416 vs T1.6's ≥ 621 — the Plan-225 bench lane UNBLOCKED behind owner-gated T2/T3. Follow-up snapshot P-item P48: the `useless_conversion` proposer gap (76 cases at 0 deciding). Plan 225 T2–T5 tails live in the plan, not the issue.

## 2026-10-11 — the 089–158 close-out audit: one missing record BACKFILLED (Issue 112), two index gaps named (137 → bench 103; 142 a legal burned number) — file:record agreement now total over every removed 089–158 file
- Issue 112 BACKFILLED (false-claim repair, the Issue-798 class): removal `fcb4a15b` (2026-09-16) claimed a HISTORY row that was never written; 112 = the probe-identity WITNESS — primary records are its removal message + the 235-line `.issues/112_probe_identity_witness.md` in git history (this row is the index); found by the 089–158 close-out's file↔record sweep. Issue 137's record home is bench 103 by design (`.benchmarks/103_generation_v9_crb_corruption_localization.md`, removal `8e67ee16` names it) — no backfill. Issue 142 is a legal burned number: no file ever existed (`.highwater` bumped by filing 141 `2839c361` and by batch196 `d25bb830`) — do not hunt for a missing 142. 095/096 deliberate git-history-only records (operative ceilings in katgpt-rs `citation_drift_floors`); 091 unpadded ("Issue 91"); 111 inside the "Issues 110 + 111" heading. En route: journey.rs rustdoc link to the removed `.issues/105` repointed to HISTORY.md (`941e049e`).

## 2026-10-09 — Issue 156 CLOSED: the FlyBy state-level rescue mining filter (b0=0 ∧ bd≥3) — T1–T6 all landed, yield honest-EMPTY locally, live-fire owner-gated (file removed per the noise-reduction rule — this row is the durable record)
- Code half 2026-10-08 `cf34f6c8` (T1/T2/T3/T5): ungated `src/rescue_mine.rs` (`rescue_keep`, `RescueTally`, BLAKE3 `keep_set_hash`) + arm-H per-state `b0`/`bd` tallies (`FLYBY_CONTINUATIONS_PER_ARM=4` documents the paper denominator) + five serde-default `TrajRowDoc` rescue fields (wire-safe) + the `--misses` rescue-band section; T5 truth table unit-pinned (6 tests). T4 at riir-reflex `86fe38a` (OPT-IN `--synth-rescue-prefilter`, default OFF byte-identical): the local modelless b0 leg rejects solved candidates WITHOUT a teacher forward; the openthai agreement VETO stays the acceptance authority — the filter only NARROWS. T6 honest EMPTY locally (manifest absent → arm-H SKIPs loud; sandbox smoke 9 states, keep-set EMPTY, never faked). Yield vs FlyBy's 95/800 awaits the first ARMED arm-H run (owner-gated spend + `RIIR_REFINE_CODE_EGRESS`). katgpt-rs `.research/609` §2.6 carries the landed hashes.

## 2026-10-08 — Issue 150 RESOLVED: the Homebrew cargo-refine shadow — option 3 (build-commit stamp) landed, option 2 (brew unlink, verdict-REVISEd to reversible) applied, the standing check fired twice in one day (file removed per the noise-reduction rule — this row is the durable record)
- Found 2026-10-06 (dapps plan-045 verdict round 2, M3): the tap Cellar 0.2.1 binary shadowed `~/.cargo/bin` on PATH — bare `cargo refine` silently ran the RELEASED build. Option 3 landed 2026-10-07 (`0a4c010c`, re-landed `3a9d1e96` after verdict rounds 1–3, final AGREE): `--version` prints `build: <12-hex>[-dirty]|unknown`; standing check = strip `-dirty`, compare vs `git rev-parse --short=12 HEAD`; existence-filtered rerun watches (a missing watched path is cargo-perpetual-dirty); gates `build_commit_logic` 11/11 · `build_stamp_cli` 6/6. The check FIRED twice 2026-10-08: M3 shadow + stale maintained binary (`6ee548ca` ≠ HEAD `32d0ac34`) → rebuilt at the pinned release set; 4090 stale `.exe` copy → pull `c3cf1865`, rebuild, re-copy. RESOLVED same day: option 2 `brew unlink cargo-refine` (verdict session `fc0f0611` REVISE — reversible, undo `brew link`; a future `brew upgrade` re-creates the link silently, the stamp check is the backstop). Standing rule: rebuild at the pinned release set when the stamp trails HEAD. Durable doc: AGENTS.md §Production adoption.

## 2026-10-08 — Issue 133 CLOSED-NEGATIVE: Matryoshka-k supervision for `rule_embed` — the shipped fixed objective wins at every measurable depth (file removed per the noise-reduction rule — this row is the durable record)
- Reopen fired on the count bar 2026-10-08 (1095 in-vocab span↔rule pairs = 4.1× the Bench 099 bar, tier 800 = 3.0×; doc_markdown 47% of pairs); Claude arbitration `9537e7f3` took B1 with three binding conditions. T1 randomized-k matrix (five arms × k ∈ {1,3,5,10,20} × both BLAKE3 splits): NEGATIVE (bench 109) — rig 134 (control) vs ≤130 every randomized-k arm; the only arm beating the shipped asset is the control itself, rejected by G1/G2-OOD exactly as designed; deep-k corpus-limited (1.16 in-vocab labels/span; withheld tower top-1 ≈ 0% vs BM25's 29.6%); the withheld-width NESTING effect is REAL (29.3→49.6 @k=20) — a reusable mechanism observation, not a promotion win. Landed: the `mattr_k_poc` replication harness (default-off, `007435e5`), record `.benchmarks/109_mattr_k_t1_negative.md`, re-pin of the stale `rule_embed_gate` population pin 138→141 (`3ca8ac4d`, floors held). No T2 re-freeze; nothing wired; shipped lane unchanged.

## 2026-10-08 — Issue 148 CLOSED: the traj-mining lane named + labeled + deployed — T-a/T-b/T-c all landed, no healer-side work remains (file removed per the noise-reduction rule — this row is the durable record)
- T-a the training-source designation paragraph in `src/kat_sync_payload.rs` (landed 2026-10-05; katgpt-rs joined the lane same day via the T-c lever-1 verdict: 385 rows / 353 new). T-b payload v4 (landed 2026-10-07 `34d32063` era: heal-time rejected-candidate capture, `TrajRowDoc.rejected`, `PAYLOAD_VERSION` 4, the dapps receiver RANGE `(2..=4)` + monotone merge; gates: refine lib 2718/0 + dapps kat_mining 20/20); the v4-ingesting devnet worker DEPLOYED 2026-10-08 (`18cb3f44…`), the maintained binary's sync tail drains. T-c analysis recorded 2026-10-05: all three growth levers route through EXISTING consent mechanisms (per-repo `.refine-sync-policy` + `--mine`; batch queue; fleet ops) — no healer-side code to write. Owner-gated remainders (tracked in riir-dapps AGENTS.md status, not lost by this closure): apex/mainnet v4 ingest (one-command deploy each); per-repo mining consent for the never-pushed local stores. Tier re-open bar (10³–10⁴ clean contexts) lives in the Issue-147 record, git history.

## 2026-10-08 — Issue 153 CLOSED: CLI version alert + binary self-update + the stale/new corpus surface — T1–T5 all LANDED, zero open code items (file removed per the noise-reduction rule — this row is the durable record)
- Owner ask 2026-10-08, verdict GO/GO/GO/GO + one deferred. T1 the alert (`src/update_check.rs`, `RIIR_REFINE_UPDATE_CHECK=0` opts out, `CI` skips); T2 `cargo refine upgrade` (verified download → SHA256SUMS → backup-swap, refuses dev builds + unknown triples; `RIIR_REFINE_AUTO_UPGRADE=1` opt-in); T3 the `--info` CORPUS section (compiled rule counts per domain + lease-corpus epoch + bin staleness, cache only); T4 `RIIR_REFINE_AUTO_CORPUS=1` auto lease refresh once per 12 h, charge-gated (the lease charge IS the tier gate); T5 the AGENTS.md docs tail `bdc2bad0` + the RELEASE_FEATURES re-land `fcd3f2db` (the `release_set_matches_workflow` gate caught its own pairing break). 15 lib tests in update_check.rs; non-vacuity via `--features update_check --lib`. Feature IN the release set (paired with release.yml in one commit). T6 DEFERRED (per-tier corpus-pack lane: dapps service contract + freeze/thaw overlay seam + committed-artifact manifest) — the lease charge covers the need today; rides this row, no separate issue.

## 2026-10-08 — Issue 155 FIXED: the 154 fix commit never backticked the allowlist cell — prose-green residue caught by the 172nd scoped boundary run (file removed per the noise-reduction rule — this row is the durable record)
- `6135b0f3`'s message claimed "backtick the cell; C3 green" but its diff touched ONLY the drift-ledger prose — the `May depend on` row kept its bare column-1 cell, so `allowlist()` (family-regex or backticked tokens only) still read `reflexer-vessel` as UNDECLARED; the scoped run stayed exit 0 only because C3 greps the raw ledger section for the name and `6135b0f3`'s own prose mentions it there — green by accident of the fix commit's narrative; the moment that prose aged out, this flips to a hard C3 violation. Found by the 172nd run (the 170th landing session gone — the 149th residue-repair precedent). Fix (this commit): column 1 backticked (`| `reflexer-vessel` |`, the script's own prescribed form for non-family deps) + the ledger prose corrected to name the false-fix honestly; scoped re-run exit 0 with ZERO known-drift lines — 15 edges, all declared. Filing commit = fix commit (issue existed on disk before the edit; this row is the record). The 154 row below carries the original false claim — corrected by this row, not rewritten.

## 2026-10-08 — Issue 154 FIXED: the reflexer-vessel BOUNDARY.md allowlist cell backticked — the 170th boundary run's C3 shape finding (file removed per the noise-reduction rule — this row is the durable record)
- The Plan-202-R4 row spelled column 1 bare (`| reflexer-vessel |`), and `ci_boundary_contract.sh`'s `allowlist()` recognises only family-regex names or BACKTICKED cells — the committed edge read as an undeclared C3 violation, workspace gate exit 1. Filing commit `7fdd7b08` (issue + drift row D1 per the issue-before-fix law); fix commit (this one) backticks the cell, removes the drift row + issue at close. No code, no dep change — a one-cell contract-shape repair (the 145th class; the R4 landing session is gone, repaired in-run per the 149th class). Gate re-run green in the 170th row of the boundary-guard skill log.

## 2026-10-08 — Issue 152 FIXED: sync_export_cli green again — the seeded rows carry their src_path now, and the fail-closed law is pinned at the process boundary (file removed per the noise-reduction rule — this row is the durable record)
- Root cause (as filed): `21f76593` (2026-10-03, riir-dapps 119 T2 — empty-path fail-closed) landed after the test file; the suite seeded legacy rows via `TrajStore::record` (no `src_path`), so the DEFAULT policy's non-empty `allow_paths` blocked them at `gate_row` → 2/4 red since then (pre-existing; the branding pass's detached-worktree baseline proved it, then this session fixed it). Fix `1759fd41`: `seed_store` records through `TrajStore::sourced("src/lib.rs")` — what a real `--fix` run leaves behind; the default policy's `src/**/*.rs` glob admits it; the two red assertions (rows_included 1 / stderr count line) hold unchanged. NEW `export_fails_closed_on_a_pathless_row_and_says_so` pins Issue 119 T2's law at the process boundary: exit 0, a verifiable EMPTY batch (commitment over zero row lines is symmetric builder/verifier), `export: 0 row(s) included, 1 blocked` loud on stderr, the row's snippet never leaves the machine. Suite 5/5 (was 2/4); clippy `-D` clean at the 5-feature set. Note: the export's no-policy remedy line (`print_no_policy_remedy`) fires on `NoPolicy` only — a fail-closed BLOCKED row is `PathNotAllowed("")`, so the count line is the disclosure; the blocked-variant test pins exactly that line.

## 2026-10-08 — the RIIR_HEAL_* compat shim RETIRED + the live .heal path residue closed (owner call: "heal is retire for refine branding")
- The owner adjudicated plan 192 P3.1's legacy-name honoring lazy: `install_env_rename_compat()` deleted from lib.rs + all three bin mains (`5aa91f82`) — legacy `RIIR_HEAL_*` no longer honored anywhere in this repo; AGENTS.md kill-switch rows → the `RIIR_REFINE_*` spellings the code reads; 6 stale `.heal dir` expect messages → `.refine`; plan 192 P3.1 dated amendment. Internal vocabulary (heal()/healqual wire domain/resolve_heal_root) unchanged. Cross-repo tail: riir-kat Issue 004 dual-read fallbacks (~14 sites across kat_service_config/kat_stake_client/kat_lease_client/kat_redact/kat_mining_consent/kat_account) LANDED same day (`f71bf4a`, only `RIIR_REFINE_*` read there now). En-route: `sync_export_cli` 2/4 red since `21f76593` (2026-10-03) — PRE-EXISTING (detached-worktree baseline reproduced), filed as Issue 152. Gates: clippy `-D` clean at `kat_transport,latent_retrieval,rust_perf,clippy_lints,kat_service`; kat_join 8/8 · kat_refill 5/5 · kat_soft_gate 17/17.

## 2026-10-08 — Issue 102 CLOSED: corpus lifecycle management (Library Drift adoption) — T1–T5 all LANDED, zero open items (file removed per the noise-reduction rule — this row is the durable record)
- Landed: Research 167 `1cb35cce` (2026-09-16) · T1 the `--frontier-report` fade section `56f0b0fd` (report-first, live REFUSAL under the population floor; FADED-vs-NEVER-CONTRIBUTED split `d0457204`) · T5 the authoring-discipline guide `8e0d52f6` (`.docs/02_corpus/authoring_discipline.md`) · T2/T3/T4 the active-set lifecycle `d9d69cfc` (`.heal/active_set.json` marker + fan-out/fix-dispatch consumers + `--necessity-bench --necessity-rule`; gate-CLOSED: the mechanism, not the wiring). Standing: the T2–T4 WIRING gate (fade proven predictive on ≥3 future generations from 2026-09-21) stays CLOSED; nothing demotes automatically (law on the `--active-set` line in AGENTS.md). Accumulation `4233f7b9` (sweep row #168) + `0768a4ad` (row #174): ZERO FADED candidates over 6 sweep rows (36→47→66→66→67→71 rules). Split of record: 089 owns ADMISSION, 102 owned the LIFECYCLE half. Cross-file links to the removed issue re-pointed to the durable records.

## 2026-10-08 — Issue 148 T-b follow-up LANDED: the rustc_errors driver-lane strategy-menu capture (the second lane on the v4 label path)
- Capture (`34d32063`): the `--fix-compile` driver computes the strategy menu at each apply decision — chosen-site candidate strategy ids MINUS the chosen one, sorted + deduped, capped at `REJECTED_CAP` — on `AppliedRecord.rejected`/`RevertedRecord.rejected` for every attempt outcome (revive-suppressed and bench-declined candidates were not considered, so not in the menu). Join key (by design): the per-error-code decision (site identity), compile-oracle-driven, not span-driven; same cap/stamp/None-keep semantics; ZERO store/wire changes (v4 already carries the label). `rejected_opt` cfg widened to `latent_retrieval` (the driver lane compiles without fix_verify; `rustc_errors` stays off the retrieval dep tree — the T0.3 law; without it the playbook-bounded menu travels whole and the store clamps on read). Tests: `strategy_menu_is_menu_minus_chosen_at_the_chosen_site` (driver.rs) + `driver_lane_record_shape_keeps_label_on_none` (traj_store.rs). Gates: lib 2721/0 at `kat_service,latent_retrieval,kat_transport,fix_verify` (floor 2718); clippy clean at default/heavy/all-features + both latent combos. Issue file updated in-place (`.issues/148` scope note).

## 2026-10-07 — Issue 148 T-b: payload v4 — the rejected-candidate label (the suite-mint training source rides the traj lane)
- Capture: ONE `matcher.suggest_file` per edited file at `--fix --write`; each edit joins its OWN-PASS line (chunk-neighborhood, exact at pass 0) and carries span rules minus the applied rule (cap 16) into `record_applied` + the post-verify `record_traj`/`record_traj_fail` via `EditAtLine.rejected`; `rejected_opt(empty)=None` never erases (the `src_path` law); the rustc_errors driver lane records `None`. Store: `TrajRowDoc.rejected` (serde-default, skip-when-empty; `stamp_rejected` sorts+dedups+clamps; Some refreshes / None keeps). Wire: PAYLOAD_VERSION 3→4; `PayloadRow.rejected` UNGATED (the `domain` vocabulary-id class) → no-label rows byte-identical to v3 (golden commitment did not move; only payload_version + covering signature re-pinned). Receiver (riir-dapps): {2,3,4} range acceptance, `QueueRow` second codec tail, monotone refresh merge — full record in that repo's HISTORY. Deploy: live worker {2,3} until the owner deploys; v4 clients stack offline, drain post-deploy (the Issue-103 law). En-route (pre-existing reds fixed same landing): 4 doc_markdown backticks (batch-206 `entries_gpu_76.rs` ×3 + escalation_manifest) + 3 stale corpus-count pins the batch-204/206 sessions left (kernel 769→771 at `batch116_rules_present`, Dispatch 171→173, rust_perf NotBenchable 81→82).

## 2026-10-07 — Issue 151 CLOSED: the secret-blind surface shipped end to end — scan/warn/blind live, marketing card deployed, the --blind red-window closed (file removed per the noise-reduction rule — this row is the durable record)
- Surface (this repo): `56d73162` (`src/secret_blind.rs` — `--secret-scan` report with blinded contexts + the quiet `[sec]` in-run warn + `--secret-scan --blind` shareable text; fingerprint never on stdout) · `675b2acf` (record) · `3d2c6208` (reef fixture) · `fd414fc9` (docs). Scanner half = riir-kat Issue 002, closed AGREE (`eed88c6`/`e8ab39f`/`47057d3`/`4b4125a`); consumed via the ungated `redact` tier. T10 egress census pinned the three chokepoints (`push_mining_batch`/`fixer_regate`/`kat_drain`); the refine.gist.rs card worded to exactly that census + the three honest limits (entropy-only unknown formats, PII out per Bench 052, terminal redacted payloads). Marketing LIVE: riir-dapps `79f3673` deployed apex `fdde59ce` / devnet `e9848ad3` (record `ae49c13`). Follow-up `be31512a`: the `--blind` standalone guard + the v3 policy literals migrated to redact v5; all-features blocker fixed at riir-infer `d09d1ec`. Known gaps (disclosed, not owed): facebook `app_id|secret` digits-joined shape; PII out of scope per Bench 052; unknown formats entropy-rule only.

## 2026-10-07 — the 9cfd0c33 corpus expansion's red window closed: the ncswap fixture E0422 + the gen4 re-freeze + the all-features lane unblocked
- Three compounding pre-existing reds behind the recorded riir-infer-gpu E0308 (which blocked every all-features lane): (1) the sibling E0308 (riir-infer `d09d1ec`, the metal upload arm's `attn_wo` missed the `q4_refuse` wrapper); (2) ncswap ground-truth `Tkn` unqualified from the child `mod ground_truth` — E0422 alongside the gold, every ncswap spec interference-class (fixed `super::Tkn` in corpus_gen.rs); (3) `FROZEN_LABELS_V4` stale vs the stride-40 slice — re-frozen 76/76 verified (40 k1 + 16 k2 + 12 k3 + 8 k4). The freeze gates compile only at `score_bench` + `rustc_errors` — a posture nobody's documented lane runs; the freeze e2e caught the fixture defect on its first post-expansion run (the label fence working). Gates: `generation_v4_freeze --ignored` 76/76 ×3; generations lib 35/35 at all-features; all-features lib 3600/0; `--tests` 29 targets green after the g4_l4_composition tripwire swap (`ptr_arg` was mined into the corpus by Batch 155 `fac3dc36`; `module_inception` the new out-of-corpus pick); shipping lib 2674/0; score-bench not re-run (clippy seed untouched).

## 2026-10-06 — dapps plan 045 (issue 117): the refine-side lanes recorded (E1 sweep half) — A1/A2/A3, A8, the verdict follow-ups, B7
- A1/A2/A3 first-join env choice `8c4e67b0` (ships v0.2.1) via the existing config-set writer; A8 stranger drill `8b08ed43` PASS live on devnet; verdict follow-ups `e5a24918`. B7 `cargo refine topup [--pack N]` `c80a5794` — USDC→KAT checkout, composer canonical in riir-kat `26ea7f4` (the Issue-082 one-definition law); LIVE SMOKE PASS (dapps `.benchmarks/018` B7 addendum); global binary rebuilt v0.2.1.

## 2026-10-06 — Issue 149 RESOLVED: the v0.2.1 capture QA findings — pre-scan disclosure, cross-pass resplice dedup, and the measured 0.1.98 firing-surface correction
- T1 `scan root:` disclosure line before the first read; T2 `dedup_respliced_edits` with authority-follows-replacement + one loud `accounting:` line (file text + revert machinery unchanged); T3 probe on clippy 0.1.98: both disputed lints still EXIST — the coverage report's declared-lint axis cannot see firing-surface narrowing; both rules carry context_notes.

## 2026-10-05 — Issue 146 CLOSED (T0.6 executed): the profile-guided-healing REAL-workload arm — rethink ESC loop profiled end-to-end, alloc gate flat, the G4 pin held, the heal landed at rethink `4cd8dcf`
- First REAL-repo `[hot]` feed (rethink ESC decide loop, sst5, 1205 decides, pinned SHA `d01a8e5`): measured-path allocs are ownership-driven exact clones — no corpus surface; structural surface filed as leads to riir-instinct/riir-infer. 3 verified edits landed at rethink `4cd8dcf`; alloc gate +0.0% everywhere, `esc_g4_alloc` pin 190 held. En-route: `~/.cargo/bin/cargo-refine` was a plain FILE COPY, now the documented symlink → rebuilds propagate.

## 2026-10-05 — Issue 146 T0.2+T0.3+T0.4: the profile-guided-healing oracle lane LANDED end-to-end — the T0 POC gate PASSED on a real hotpath run (alloc −51.1%)
- hotpath JSON = second oracle beside `--clippy-json` (`src/hotpath_report.rs`); `[hot]` spans hoist first by time share (ranking-only), marker precedence `[oracle] > [hot] > [heuristic]`; fix lane declines edits overlapping hotpath's own instrumentation lines. E2e gate `scripts/hotpath_e2e_poc.sh` PASSED: alloc bytes −51.1% (4.7 GB → 2.3 GB, N=3), timing 2.1×, instrumentation survived byte-identical.

## 2026-10-05 — Issue 147 CLOSED (T0′ executed): the Decision-CPI escalation tier killed on the corpus axis — headroom GO, corpus 19× off + converged
- Headroom GO: 141 spans, θ=0.05 subset top-1 0.750, max recoverable +2.8pp banked. Corpus KILL: fleet contributed 7,513 / credited 533 = 19× off the 10⁴ target — the tier CLOSES, never-per-span law stands; re-open at the 10³–10⁴ clean-context class (riir-dapps Issue 148 T-c levers).

## 2026-10-05 — Issue 138 CLOSED (owner-gate pickup): E7 ruled free-tier-only — operative DENY until a cap lands; E8 C8/C9 draft ADOPTED (master riir-ai `.issues/1016`; plan 181 / issue 138)
- E7: the standing CF sign-off loop is free-tier-only; metered use DENIED until an enforced daily cap with auto-stop lands. E8 ADOPTED (delegated verdict): C8 audits each git-rev pin individually; a multi-rev split on one owner is C9; same-branch lag-pin splits read as C9 NOTES, branch-mixed splits stay loud C9.

## 2026-10-03 — Issue 145 CLOSED: first-run output reads like hand-written Rust (`d40f7b5e`)
- `needless_range_loop` binding = collection's singular else `item`; `DomainId::applies_to_path` display filter (a span lists only domains that can touch its file kind; `--fix` unchanged). T2 deferred as optional polish.

## 2026-10-02 — Issue 143 LANDED: the retrieval GOAT's G2 bar dual-pinned per host (the Issue-filed platform gap closed at root cause)
- `tests/latent_suggest_goat.rs` carries `G2_BAR_US`: `1000.0` under `#[cfg(target_os = "macos")]` (historical bar unchanged), `3000.0` elsewhere; reading always prints beside its bar + OS. G2 1505 µs/chunk PASS on this box; 5/5.

## 2026-10-02 — Issue 141 CLOSED: the four pre-existing integration-red classes (A fixture-era floor, B the two-consumer fixture, C a REAL v2 client regression in riir-kat, D the level-gate fixture)
- Full integration repro green: 69 targets / 3086 passed. A: wetrun fixture attests at the floor (3). B: eval-span spelling `b.count_ones()` — a fixture feeding BOTH the clippy oracle AND labeled eval spans must be verified against both in one landing. C: REAL riir-kat client regression — `seen_live` flag pins fully-down→queued vs live-then-quiet→bail. D: the e2e fixture now DECLARES the lint it teaches (`[lints.clippy] match_bool = "warn"`).

## 2026-10-01 — Issue 141: the `cargo refine` cargo-convention argv leak (the rename's one real regression, found from the owner's brew question)
- cargo passes the binary's own name as argv[1]; the strip now removes BOTH `refine` and `heal` spellings; `tests/cargo_convention_cli.rs` 4 arms at the process boundary. Brew answer (owner-gated T1.2f): the tap still serves `cargo-heal` until the owner-side dist rename flips.

## 2026-10-01 — `repo_posture_cli` red CLOSED + the mid-posture clippy red + three owed `[[test]]` rows (the rename's flagged remainder)
- Root cause: the 3 applying arms ran `--fix` without `--no-verify` — red only under `--all-features` where `fix_verify` compiles in; fixed with `--no-verify`. Plus: `heal.rs` unreachable-`return` at the mid posture moved inside the `kat_transport` block; three `[[test]] required-features` rows added (`repo_posture_cli`, `auto_oracle_features_cli`, `coverage_report_cli`). 4090 exe refreshed; 4090 git-over-SSH root-caused (IPv6 black-hole fixed via `C:\Users\katop\.ssh\config` AddressFamily inet; sshd-session hang mitigated by the `gitsync` Task Scheduler task).

## 2026-10-01 — The refine rename, Phase 1 landed (`ac4b3d16` + docs; Proposal 018 / Plan 192): binary `cargo heal` → `cargo refine`
- `[[bin]] cargo-refine`, all display strings + test pins in lockstep; riir-kat leaf swept too (18 files / 71 replacements). Internal vocabulary unchanged (`heal.rs`, `HealDomain`, `RIIR_HEAL_*`); dist URLs stay `cargo-heal` until the owner-side flip.

## 2026-09-28 — Owner-gate pickup E3/E4/E5/C7/E8 (master riir-ai `.issues/1016`; plan 181 / issue 138)
- E3 dead `gist-rs/riir-unity` checkout gone from `ops_dashboard.yml`; E4 stale-no-op (the 4 never-defined-feature targets never existed — verified); E5 one-line disambiguator in `.research/083_omlx_external_distill_batch71.md`; C7 `structural_admission` stays INERT until a priced first consumer; E8 draft recorded (ADOPTED 2026-10-05, see above).

## 2026-09-26 — Issue 135 CLOSED (`c1c6cf21`): `cargo heal --fix` gates allow-by-default lints on the target crate's effective level
- `LINT_GROUPS` from the pinned `clippy-driver -W help` (20 allow-by-default lints, not 13); `LevelResolver` over `[lints.clippy]`/workspace lints/crate+file attrs; `--groups pedantic,nursery,restriction,cargo|all` (requires `--fix`); `doc_markdown` skips shell-command doc lines. Measured 0 live instances of the undocumented skip-direction forms workspace-wide.

## 2026-09-25 — Batch 179 LANDED (Plan 172 / Research 210): the rsconstmap distill — the std-map-probe family's layout + MLP + map-selection cells
- lemire/rsconstmap @ `bf7b3341`: `paired-probe-layout-present-vs-absent` (1.68×/1.45×/1.80×), `block-hash-before-gather-mlp` (1.68×/1.30×/1.02×), `immutable-static-map-filter-selection` (2.09×/4.27×, `[dep: constmap]`) — all ParetoDominant; fixture lesson: derive index arithmetic from the source (`h * m` multiply-high), not habit. rust_perf 153→156.

## 2026-09-24 — Issue 134 CLOSED (T3 re-arm executed): seven-domain sweep, the ρ=0.536 artifact, and the degenerate-axis guard
- The sweep refused on a degenerate gain axis (Spearman against a constant vector renders a pseudo-value); final verdict REFUSED — `demonstrability_gate` stays default-off BY VERDICT; T1 standing instrumentation survives. Latent `HeadroomReport.per_domain` `[_; 3]` OOB fixed.

## 2026-09-24 — Issue 134 T4: the fanout controls — best-single-domain + homogeneous-at-budget
- `EvalReport.homogeneous` (equalized-budget pass) + best-single-domain ceiling row. First live read: homogeneous@budget = blended EXACTLY — blended top-1 is entirely per-domain selection, ZERO fan-out contribution. Rider: `print_report`'s `names` had 6 entries vs 7 — fixed.

## 2026-09-24 — Issue 134 T3: the ρ-prediction depth sweep — instrument landed, verdict REFUSED at the power floor
- `examples/demonstrability_t3_sweep.rs` (pool ladder 28→56→112, hand-rolled Spearman): every depth curve FLAT; n=3 qualifying domains < the ≥5 power floor — honest refusal, re-run after the T4 fixture re-arm. Kernel fixtures are corpus-derived (de-bias note).

## 2026-09-24 — Issue 134 T2: the pool-depth allocation gate (`demonstrability_gate`)
- `src/demonstrability_gate.rs` (ungated policy) + `cfg(demonstrability_gate)` matcher wiring scaling pool AND truncation K; kill-switch `RIIR_HEAL_DEMONSTRABILITY=0` guards the auto-arm loader only. Standing constants round INERT at pool 28/K 3 — correct at two nearly-equal constants. Landing's own `bool::then_some` inversion caught by its tests pre-commit.

## 2026-09-24 — Issue 134 T1: the standing demonstrability column (SAT / katgpt-rs Research 585)
- Per-domain demonstrability constant (formula corrected to `resolved / (resolved + persist)`); `src/rule_domains.rs` moved UNGATED from `kat_sync_payload` (re-exported at its historical path — goldens unchanged). First readings: clippy 0.935, rust_perf 0.962.

## 2026-09-24 — Batch 174: the cubecl/bevy merged-fix distill (host-lock + flush-hoist + the B56 rider)
- `host-lock-across-reentrant-blocking-call` (NotBenchable) + `deferred-command-flush-hoist-out-of-batch-loop` (ParetoDominant 21.3×/13.3×/16.5×, alloc flat; the bench-honesty arc took 3 iterations to an honest toy). rust_perf 151→153 + kernel_opt B56 rider (cubecl #1686 double-checked-lookup).

## 2026-09-23 — Issue 132: the role-perspective e2e flows — doc + four per-tab SVGs + the front-page embed (file removed per the noise-reduction rule — this row is the durable record)
- Four role e2e SVGs (`e884f0f5` here) + the riir-dapps front-page embed (`eefd8cd`, mermaid id-suffixing for 5 inline SVGs sharing one document, `svg_contract.rs` extracted). Deploy live-verified devnet `323d4ff6` + apex `375f95f2`.

## 2026-09-23 — Issue 131: the e2e self-evolve flywheel as a doc diagram + the KAT-pages static SVG hero (file removed per the noise-reduction rule — this row is the durable record)
- `.docs/10_self_evolve/self_evolve_flow.md` + static SVG (`ed7bf3aa` here) + riir-dapps mirror `82933de`; hero LIVE on devnet verified same day.

## 2026-09-23 — Batch 173 / Plan 167: the serde zero-copy family distill, rust_perf 149 → 151 — the corpus's first dedicated serde rules
- `serde-borrow-deserialize` (ParetoDominant 4/0/0; alloc axis EXACT 300016 → 16 at n=100k; escaped-payload regime witness printed in-bench) + `rawvalue-passthrough-defer-parse` (3/0/0; alloc 950000 → 100000 at n=50k). Both suggest-only → freeze tables structurally unmoved.

## 2026-09-23 — Issues 089+102+130 batch: active-set lifecycle (T2/T3/T4), the admission-gate REPORT half (T1+T3), the owed gen-3 sweep #161, and the switch_cost cross-fire adjudication
- `src/active_set.rs` landed gate-CLOSED (`--active-set-demote/readmit/clear/cap`; nothing demotes automatically); `src/gate_calibration.rs` report-only (δ=0 prints UNCALIBRATED, both stores STALLED); gen-3 sweep row #161 in a detached worktree; switch_cost cross-fire adjudicated TRUE_POSITIVE (clippy `map_clone` on the perf fixture idiom).

## 2026-09-23 — CI sibling provisioning: riir-infer joins every checkout (the carve's T2.3 riir-clippy half) + release.yml's pre-existing kat-plane gap fixed
- cargo resolves path deps even behind `optional = true` — `riir-infer` joined rust.yml's sibling loop; release.yml was BROADER and PRE-EXISTING broken since 2026-09-10 (riir-kat spin-out): both loops now clone `riir-ai riir-neuron-db riir-kat riir-auth riir-llm riir-infer`.

## 2026-09-23 — P36: the `[C2·documented-write-only]` downgrade (config-audit; the seal-remake `FeatureFlags`/`SessionMeta`/`WasmLimits` idle-pass intake)
- `classify_self_referential` takes `field_docs` (one marker vocabulary home); the EVERY-field rule is load-bearing (one unmarked field keeps the strong verdict — negative arm pinned). seal-remake re-audit live-verifies all three C2s render weak.

## 2026-09-23 — Batch 172 / Plan 166: the mezura rust_perf distill, rust_perf 145 → 149 — the dual-mask GOAT lead at 43×/18×
- `.raw/mezura` @ `2dbacb7`: `dual-mask-fused-block-scan` (ParetoDominant 4/0/0: 43.11×/43.95×/23.10×/18.29×; AVX2+NEON arms; exhaustive G1 caught the `\n`-in-needle-set contract subtlety), `mmap-settled-size-gated-read`, `steal-batch-not-per-item`, `nonblocking-open-for-head-probes` + 2 riders.

## 2026-09-21 — Issue 128 EXECUTED (T1-T7): the fixseq cascade-law + derailment-index report shipped, two mint corpora measured — the law is GENERATOR-CONDITIONAL, G4 fired live, HORIZON(prev) does not raise
- `src/fixseq_stats.rs` + `--fixseq-stats` + T6 transition arm in `horizon_report`; two mint configs committed (`.benchmarks/data/128_t5/`). Cascade law GENERATOR-CONDITIONAL; G4 fired live (first-wrong is a point mass at position 0); silent failure 100% on minted corpora; HORIZON(prev)=1 honest null. Reopen: organic multi-revert rings.

## 2026-09-21 — P35: the `format_collect` import goes to the INNERMOST enclosing mod — the cfg(test) unused-import class closed (queue item 14, Protocol C)
- Probe reproduced the incident live (import at file scope + usage under `#[cfg(test)]` → unused_imports warning; `--verify` blind by construction). Repair: mod-stack tracking lands the import at each innermost mod; any top-level site keeps one file-level import. 5 fixtures, module 19/19.

## 2026-09-21 — Plan 165: `--repo-posture` — the repo-lint-posture fix mode (the intake's OPEN product suggestion, closed)
- Clippy-lane edits apply only where the target repo's own clippy feed carries a same-lint diagnostic whose primary span overlaps (215 proposed vs 16 real in the motivating intake); `map_span_through_splices` + `posture_spans`/`posture_allows`. No feed → REFUSE exit 2; `tests/repo_posture_cli.rs` hermetic.

## 2026-09-21 — Plan 164 (queue item 14, Protocol C): the three 09-21 post-mining intake classes repaired at root cause (`eb4827ea`)
- Class 1: const-generic-arg-for-runtime-size refused via a file-wide binding pre-pass. Class 2: `unreadable_literal` consumes `excessive_precision::shortest_round_trip` (one decision, two consumers); the real shape was unsuffixed literals in `[f32; N]` fixtures — refusal CORRECT. Class 2b: `--all-targets` is the auto-oracle DEFAULT (`RIIR_HEAL_AUTO_ORACLE_NO_ALL_TARGETS=1` restores). Class 3: `[wire-const]` runs on non-test sites only (`in_test` bit).

## 2026-09-21 — Issue 089/102 verdict session: the frontier snapshot store discriminator (the x-axis was non-monotone on two unnamed stores) + Issue 102 T1's fade report (report-first, live state REFUSES)
- Every snapshot row now carries `store` = blake3(store root)[..8]; the 09-20 per-store readings were WITHDRAWN as unrecoverable. Issue 102 T1: fade keyed on DISTINCT FULL-CORPUS SWEEPS (≥25 rules attributed); FADED ≠ NEVER-CONTRIBUTED split; live state is a loud REFUSAL until 3 sweeps exist; the ≥3-future-generations clock started.

## 2026-09-20 — Batch 165 LANDED (Plan 156 / Research 193 → RECORD): `unchecked-wire-arithmetic` — the overflow sink beside the alloc sink, on the SHARED wire-taint walker; sec 15 → 16, InputValidation 3 → 4; the Kani property axis closes the loop (g2c/g2d run on the WSL2 lane, the pre-validation block returns to EMPTY)
- `proposers/wire_taint.rs` extracts the Batch-131 walk as a `WireSink`-parameterized substrate; arithmetic sink `7ab7c628`; sink-DISTINCT adjudication (no composition gate). Kani g2c FAILED as desired / g2d SUCCESSFUL on the 4090 WSL2 lane; probe found `arithmetic_side_effects` SILENT on literal/const operands — strengthens the rule's premise.

## 2026-09-20 — the auto-oracle's empty-project-root spawn failure FIXED (idle-sweep find): `detect_project_root` cwd-anchored — the documented `cargo heal --suggest src/` form works on every platform again
- Relative 1-component targets walked to the EMPTY path (os error 123 on Windows / ENOENT on Unix); cwd-anchored before the parent climb (`0f053d0a`), 3 regression tests. The never-generalised-repair shape: `compile_oracle::owning_package_root` fixed this exact class and the repair was never ported. The 9-repo sweep then ran clean.

## 2026-09-20 — Issue 126 COMPLETE (both lanes): the Lemma-4.2 likelihood-ratio tier guard gates trajectory certification — wrong-goal credit and lucky-keep overcertification are now excluded with a priced bound (`src/tier_guard.rs`)
- `success && verified` certifies only when the goal-side Elo model and the row's Laplace history agree within ε=0.5 (γ=0.9 prices the 2γε/(1−γ)=9 slack); counters always record, only the promotion gates; cold start behavior-identical; non-finite ratings fail closed at the input. Store arms in `tests/tier_guard_store_gates.rs`. T0 hindsight-label corpus committed at `.benchmarks/data/126_t0/` (172 runs / 242 rows, BLAKE3 manifest).

## 2026-09-20 — Issue 127 POC EXECUTED, verdict POSITIVE: Kani machine-checks the sec rules' property claims — witnesses exhibited both directions, cost floor sub-second per harness (WSL2 lane)
- Kani 0.68.0 (pinned nightly, WSL2-only — Windows-native unsupported): G1 NaN-unsafe comparator witness exhibited (`-inf, +NaN, +inf`); G2 wire-arithmetic two-sided verified. Harnesses at `.proofs/kani_sec_poc/` — never built by the workspace's stable rustc, never in CI. Owner pulled the staged lead → Batch 165.

## 2026-09-20 — Issue 123 RESOLVED: the TUNA client surfaces are complete + live on devnet — ack v4, the soft gate's second plane, the fuel-gauge tank, the plane-split tail, the depleted-run survey; only the owner's apex/mainnet light-up remains
- ack v4 (`tuna_balance_micro`), soft gate lifts on a positive TUNA balance (`None` never an invented zero), footer `TunaLine`, `plane_split_note()`, depleted-run survey (count-only), `--info` TUNA line. Sole remainder is the owner's apex/mainnet deploy + `/tuna/genesis`.

## 2026-09-20 — P34 CLOSED: the `needless_range_loop` LENGTH-MISMATCH class guarded at all three lanes (oracle door + proposer fixture-pins + the manual-conversion divergence row) — the riir-ai Issue 981 healer lesson
- The 981 rewrite was NOT healer output (hand-applied); all three lanes guarded: oracle door requires a LOCAL SYNTACTIC length proof (receiver-identity correction: the `.len()` must name the collection it indexes); proposer fixture-pins; SKILL divergence row. Residue: all 18 conversions in the known emitter commit audited — 17 sound, 1 already fixed.

## 2026-09-20 — Issue 125 RESOLVED (REFUTED): the CUA-S1 specialist option-scorer PoC loses to the modelless baseline at our corpus scale — harness + 719k tinyx land as the re-run lane ([Bench 098](.benchmarks/098_choice_scorer_poc.md))
- G1 27.7% vs baseline 88.0% in-dist, G1-OOD 5.6% vs 100%, G2 8868 µs/span; shuffled-context control PASSED, frozen control 7.2% — genuinely learned, genuinely starves on 101 gold contexts. `RerankMode::Structural` default stands; nothing wired into any heal path. Reopen: ~10⁴–10⁵ labeled contexts.

## 2026-09-20 — Issue 125's metric-taxonomy half: the CUA-S1 decision-outcome rows land in `retrieval_eval` (wrong-rule/wrong-span, acceptable-alternative gold, the three-way decline axis)
- `ChoiceTaxonomy` on `EvalReport` + `OwnedEvalSpan.acceptable` (zero sets authored by design — an acceptable set is a corpus claim) + covered/declined/proposer-blind axis. First live: coverage 50.5%, selective-accuracy 100% — every error lives in the declined half.

## 2026-09-19 — the two cross-repo hardening fixes (katgpt-rs Issues 832 T5 + 841): scratch-root read-before-write + verify's incomplete-check refusal
- `651728c0`: a killed/truncated recheck is now a REFUSAL, never a VERIFIED keep. `83f4cd89`: scratch-crate work roots get the read-before-write mtime guard (the riir-clippy 9 re-offers = deliberate scratch roots).

## 2026-09-19 — Batch 164 / Plan 155: the coverage-report re-arm — the frontier-lead mining pack X (the EIGHTH consecutive all-FIXABLE pack; corpus 84 → 86, clippy 65/16 → 67/16; TWO freeze bad-atom joins — a first for one batch)
- `f84037c2`: `match_like_matches_macro` + `excessive_precision` (record: `.research/191_batch164_coverage_report_rearm.md`). Bad sets 6→8 (v2) / 7→9 (v3); ladder 86 labeled. Score-bench #140/#141 96.36% FLAT; global binary rebuilt (was 6 batches stale).

## 2026-09-19 — Batch 163 / Plan 154: the frontier-lead mining pack IX (the SEVENTH consecutive all-FIXABLE pack; corpus 82 → 84, clippy 63/16 → 65/16; the recorded-spare queue EMPTIES)
- `6408ea44`: `manual_string_new` + `needless_late_init`; the syn finding: statement attributes fold into the statement span and syn RESPANS inner exprs over it. Freeze regen v2 90/11 + v3 89/16; score-bench #139 96.36% FLAT.

## 2026-09-19 — Batch 162 / Plan 153: the frontier-lead mining pack VIII (the SIXTH consecutive all-FIXABLE pack; corpus 80 → 82, clippy 61/16 → 63/16; THE FRONTIER IS EMPTY — all 17 leads graduated)
- `bytes_count_to_len` + `bool_comparison`; the e2e now records the `all_graduated` state explicitly. `pool_depth_rescue_guard` re-fired third time — RERANK_POOL 24 → 28 + the 32/40 rungs. Freeze regen refusal guard: non-id rows under an EXCLUDED header refused.

## 2026-09-18 — Batch 161 / Plan 152: the frontier-lead mining pack VII (the FIFTH consecutive all-FIXABLE pack; corpus 78 → 80, clippy 59/16 → 61/16)
- `unit_arg` (hoist the side-effect block, re-spell the arg as `()`; the heldout marker must be the in-parens BROKEN-only shape) + `string_lit_as_bytes` (`b"abc"`; refuses the `\u{` escape — clippy's own suggestion there is INVALID RUST). Freeze v2 84/13 + v3 85/16, both e2es PASS.

## 2026-09-18 — Batch 160 / Plan 151: the frontier-lead mining pack VI (the FOURTH consecutive all-FIXABLE pack; corpus 76 → 78, clippy 57/16 → 59/16)
- `from_str_radix_10` (construction identity: std's `FromStr` IS radix-10; no shadow guard) + `assign_op_pattern` (exactly-once binding derivation; swapped-operand arm commutative-only). RERANK_POOL 20 → 24 (the rescue guard re-fired); Batch 158's missing freeze record paragraph + stale `.research/.highwater` repaired.

## 2026-09-18 — Issue 103: sync payload v3 — the per-row `domain` (the corpus-view Phase-2 axis)
- `PayloadRow.domain` (v2→3) from the compiled `RuleDomains` table; receiver `verify_sync_payload` accepts {2,3}, monotone refresh merge; cap hardening — `other` can never be evicted; golden dual-pinned to v3 bytes (v2 preserved as `GOLDEN_PAYLOAD_V2`). En-route: the all-features lane had been red since `8fc52d22` (import at groq.rs top level, module lives in `mod router`); the Issue-088 redact pin was keying on a coincidence (now asserts floor == `REDACT_VERSION`).

## 2026-09-18 — Batch 159 / Plan 150: the frontier-lead mining pack V (the THIRD consecutive all-FIXABLE pack; corpus 74 → 76, clippy 55/16 → 57/16)
- `single_char_add_str` (mut_string_param_heads_over twin; `escape_default` emit) + `manual_clamp` (lattice-absorption; FLOATS refuse on NaN max/min-vs-clamp divergence, inverted bounds refuse on the release-mode assert — both in-rule). Archive step is LOAD-BEARING (archiving consumed rows breaks the family-done deadlock). Freeze v2 78/15 + v3 80/17.

## 2026-09-18 — Batch 158 / Plan 149: the frontier-lead mining pack IV (the second all-FIXABLE pack; corpus 72 → 74, clippy 53/16 → 55/16)
- `to_string_in_format_args` (perf/default-on, scalar-derivation provenance; bare-`{}` placeholder guard; composes with `uninlined_format_args` by design) + `manual_range_contains` (contains IS the two leg comparisons; literal-only bounds). Freeze v2 77/17 + v3 78/18; score-bench #133 96.36% byte-identical.

## 2026-09-18 — Batch 157 / Plan 148: the frontier-lead mining pack III (the first all-FIXABLE pack; corpus 70 → 72, clippy 51/16 → 53/16)
- `redundant_closure_for_method_calls` (construction-identity whitelist: String/Vec/HashMap/HashSet × len/is_empty/capacity; `helpers::slice_param_heads_over` substrate) + `iter_skip_next` (slice containers heal to `v.get(n)` — one step better than clippy's `.nth()`). `unnecessary_sort_by` joined the bad-atom set; 12 orphaned EXCLUDED rows pruned.

## 2026-09-18 — Batch 156 / Plan 147: the frontier-lead mining pack II (the re-armed 2 leads consumed; corpus 68 → 70, clippy 50/15 → 51/16)
- `io_other_error` (FIXABLE; identical payload bound) + `wrong_self_convention` (VERIFY; `into_*(&self)` → `into_*(self)`, NO-vis inherent only). Re-arm probe: `redundant_closure_for_method_calls` + `iter_skip_next`. Score-bench #131 96.36% FLAT.

## 2026-09-18 — Batch 155 / Plan 146: the frontier-lead mining pack (the miner's first 5 leads consumed; corpus 64 → 68, clippy 47/14 → 50/15)
- `let_and_return`, `redundant_slicing`, `map_flatten` (FIXABLE) + `ptr_arg` (VERIFY); `needless_borrow` receiver whitelist extended (str-inherent). `let_and_return` measured BROAD — 3 pre-existing atoms' fixtures fire it too (bad set 2→5). The e2e runs one loop per covered family.

## 2026-09-18 — Issue 122 / Plan 145: the PowerPlay frontier miner PoC (G1–G4 PASS, opt-in)
- `src/frontier_miner.rs` + `examples/frontier_miner.rs`: modelless, deterministic, zero-LLM generator; full invariant loop (unsolved-before / solved-after / regression-green / exactly-once BLAKE3) + per-family K-minimality; 16/16 seeds clean, 6 rows committed. Promotion NOT claimed — rides the Issue-089 band-predictivity clock.

## 2026-09-18 — Batch 154 / Plan 144: the tokio upstream-surface refresh (CHANGELOG 1.47–1.52.3 + "Actors with Tokio"), rust_perf 142 → 145
- `write-vectored-capability-flag`, `oneshot-response-send-unwrap-panics`, `actor-bounded-channel-cycle-deadlock` (all NotBenchable, domain reasons) + 4 annotation folds (spawn_blocking sharded-queue REVERT, LIFO-steal REVERT, coop public API, socket-option family). Process note: build corpus entries with the Python path for multi-line fixtures.

## 2026-09-18 — Plan 141: the devnet burn-process e2e, both machines live (Bench 097)
- `scripts/burn_e2e.sh` 9/9 hard assertions green twice (refuses off-devnet configs + fleet account ids). Headline server-read: 100 KAT grant burned to exactly 0 by 90 metered scans; depleted+never-contributed refused at run start; clamp held; contributed-lift ran clean; conservation held. Findings recorded in Bench 097 §4.

## 2026-09-18 — Plan 035 wave 2 (T3/T10/T6-client/T12): the client becomes a typed µ consumer + the ×1_000_000 grep gate
- `MicroKat` adoption (kat_footer is the unit-vocabulary module; `MICRO_PER_KAT as f64`, never bare 1e6); L4 value-level display pin; `tests/micro_literal_gate.rs` bans the ANONYMOUS million (TOKENS_PER_MTOK beside MICRO_PER_KAT — same value, different dimension). Latent: dead `wallet_lines`/`receipts_line` at kat sets without `kat_transport`.

## 2026-09-17 — Bench 094 wave 4: the third default-eco extension (501 → 663 clean crates, flip 110/442 → 129/442) + the dead cross-wave base-dedup found by audit
- The cross-wave base-dedup in `cmd_sample` was DEAD CODE (bare name vs versioned keys — never matched); fixed with the constructed-key law + a merge-time hard-refusal. Fresh multi-crate head: `io_other_error` 57/8, `unit_arg` 46/2, `wrong_self_convention` 19/6.

## 2026-09-17 — Bench 094 wave 3: the second default-eco extension (342 → 501 clean crates, flip 94/442 → 110/442) + the resume prefix-collision fix
- Resume prefix-match over-matched (`gsk4-sys-0.11.4` startswith `gsk4-`); caught by the reconstruction diff before any number was used; fixed to the exact constructed key. Head: `needless_borrow` 92/30, `redundant_field_names` 58/9.

## 2026-09-17 — the fresh-pass pedantic residue closes (34 + 6 findings, the stale-cache bite confirmed on the Batch-150 close-out itself)
- A fresh-target `cargo clippy --all-targets` at HEAD reported 34 findings the stale-cache close-out had read as 0 — a fresh worktree + empty target dir is the only clippy reading that means "0". The landed doc_markdown rule healed 38 edits (dogfood closing the loop). Fixed the one fresh lib-test failure (worktree cwd artifact in `downstream_hint::relative_target_fires`).

## 2026-09-17 — Issue 121 RESOLVED: the set_rerank consumer landed, promotion REFUTED, the all_rules canonical-order fix is the GOAT (`2aa53e1d5`, Bench 096)
- Bench 096: the canonical-merge control beat both admission arms (85.2% vs 70.4/74.1% top-1); every α-ablation lost to shipped on the OOD rig. The real yield: `all_rules` discarded the T3.5.4 instantiation re-rank across domains — canonical key restored on the DEFAULT path (+55.6pp top-1), regression test landed. Re-open trigger: `pool_depth_rescue_guard` firing again.

## 2026-09-17 — Plan 134 T2.1 COMPLETE: the frame-time budget tier ships (Bench 095)
- `src/frame_time_report.rs` (ungated) + `--frame-time-report` (`--frame-budget`/`--vs`/`--frame-delta-pct`); warmup-parity + pairing-uniqueness evidence guards; refuses exit 1 on empty/unparsable/zero-pairs. Walker matched `Path::extension()` (= `json`) — every real sweep dir read EMPTY; fixed to full suffix match.

## 2026-09-17 — Bench 094 wave 2: the default-eco histogram extension (184 → 342 clean crates, flip 74/448 → 94/442) + the T7.5 fold
- Sampler is tracked instrumentation (`scripts/sample_default_fired_histogram.py`). Two incidents caught before any number was used: cargo's JSON carries diagnostics under `message` not `msg` (the known-dirty control caught it); resume skip compared bare names against versioned keys (merge dedups keep-first). T7.5 fold: corpus-touching landings require the freeze-table regeneration in the SAME batch.

## 2026-09-18 — Batch 153: the async-blocking distill lands (rust_perf 139 → 142)
- `blocking-sleep-in-async`, `rayon-bridge-oneshot`, `tcp-nodelay-request-response-accepted-socket` (NotBenchable, domain reasons) + 5 annotation folds. All source quotes re-verified live at mine time. The stale "Not benchable 36/127" ledger line repaired to 50/142.

## 2026-09-18 — Batch 152: the dial9 tokio-principles distill lands (rust_perf 136 → 139)
- `batch-blocking-pool-segment`, `adaptive-yield-consecutive-ready`, `bounded-fanout-semaphore`. Honest corrections: the mini-redis p50/p99 figures are NOT on the live page — the rule carries only the verifiable ≈10× and the K=4.

## 2026-09-17 — Batch 151: the doc-line state-machine port lands (corpus 62 → 64, FIXABLE 45 → 47)
- `doc_lazy_continuation` + `doc_overindented_list_items` in ONE proposer module; measured: clippy's zero-width insert span sits at the CONTENT start (after the conventional space); +4-over still fires (pulldown reads it as paragraph text); `helpers::doc_lines` substrate extracted.

## 2026-09-17 — Batch 150: the staged default-level + pedantic probe pack lands (corpus 56 → 62)
- FIXABLE +4 (`print_with_newline`/`write_with_newline` one module two names; `unreadable_literal` digit-count ≥6; `legacy_numeric_constants` with a shadow guard); VERIFY +2 (`partialeq_to_none`, `cast_lossless` safe-let gate). DECLINED with evidence: `redundant_closure_for_method_calls` (typeck-resolved path). Rider: the Bench 094 dead test (missing `#[test]` attr — an uninvoked assertion is unknown, not passing).

## 2026-09-17 — the default-level fired histogram lands (Bench 094, Issue 119's T4 successor)
- Sibling-git-history REFUTED as the population (CI `-D warnings` since before the healer); Li et al. transcription REFUTED (Figure 2 carries no printed value labels); measured first-party: 184 clean crates / 761 warnings / 105 distinct default-firing lints. The join flips the cross-tab 0 → 74 of 448; the CONCENTRATION LAW: the nominal top-1 came from ONE crate — read the spread column.

## 2026-09-17 — the manual_range_patterns corpus rule lands (the intake MISS folded, P31)
- VERBATIM port of clippy's own coverage-walk algorithm (`53269d3a`) after probes refuted two naive models; order-free, duplicates absorb, range alternatives join the walk, boundary texts splice verbatim. `manual_is_multiple_of` adjudicated by-design-silence-plus-oracle-lane (E0689 class is check-INVISIBLE).

## 2026-09-17 — the doc_markdown bare-URL arm lands (the staged follow-up lead)
- Real clippy's URL branch is `Url::parse` + `!cannot_be_a_base` — any RFC scheme angle-wraps; `is_absolute_url` re-derived bounded (no new dep); autolink guard on the RAW token. Follow-up: the link-target guard moved to the raw token (`](` stripping made the trimmed word miss it) — `broken_intra_doc_links` is a rustdoc not rustc diagnostic.

## 2026-09-16 — Issue 120, the parallel session's side: the yield + the rust_perf debt closes (the allow lifted, 157 edits healed by the landed rule)
- The rust_perf module-level `#![allow(clippy::doc_markdown)]` LIFTED — 157 edits healed by the landed rule itself; 3 feature-gated residues fixed by hand. Score-bench rows #123/#124 independent derivations agreed.

## 2026-09-16 — Issue 120: `doc_markdown` lands — the every-landing manual backtick pass is the healer's now
- Clippy's word predicates ported VERBATIM; located as REAL `#[doc]` attributes (string literals unreachable by construction — the load-bearing FIXABLE property). Freeze tables regenerated + re-verified; router floor re-pinned 78→72 (−6 from the rule's PRESENCE, proven by a reword control); the rust_perf wholesale allow deliberately NOT lifted that day (paid 09-17).

## 2026-09-16 — Issue 114 resolved: the refusal-code client half (landed in riir-kat `a117163`)
- `kat_refusal.rs` (13-code table + `RetryVerdict`) + the code-aware burn stack; absent/unknown code = the prose-era default, never a guessed retry; monotone watermark makes replay-retention always safe.

## 2026-09-16 — Issue 116: the corpus-truth delta surface — `[no corpus rule]` carries TWO opposite messages
- `src/corpus_delta.rs` + `--corpus-delta`: "not leased YET" (the fix lane writes it today) vs "unsupported" (mining intake); the partition is asserted; FIXABLE/VERIFY tiers stay split (a VERIFY row only applies under `--fix --verify`). Freshness bucket is 0 BY CONSTRUCTION on a lease-less build — the exercise arm drives the REAL producer.

## 2026-09-16 — Issue 119: the corpus coverage-debt sweep — and the blind spot its first run found
- `cargo heal --coverage-report` (`300411ed`): population DERIVED from the installed driver; first measurement 822 derived / 54 covered / 768 UNCOVERED / 449 default-firing. The finding: 0 of the 449 carried ANY frequency evidence (the local histogram was captured pedantic-enabled — structurally blind to default-level debt). Fix-set-only = 0 (corpus-lockstep holds from the converse side); 0 cross-domain collisions.

## 2026-09-16 — Issue 119, the parallel session's side: the yield + the process-boundary delta (e2e arms + the never-bare row)
- Parallel implementation measured the identical first numbers (822/54/768/449 — a two-way cross-check); yielded; deltas re-landed: `tests/coverage_report_cli.rs` (exit contracts at the process boundary) + the `standalone_modes_are_never_bare` row.

## 2026-09-16 — Issue 118: the bench-target guard extended to the compile-repair surfaces (`--fix-compile` writes + the oracle lane)
- A compile repair can hoist a loop or delete the timed "dead" result — P30's "compile-shaped is safe" superseded. `bench_guard` promoted to crate top level (ungated); driver writes filtered before the pick; `user_path_forces` explicit override; P21 oracle lane rides the same door; `tests/fix_compile_bench_guard.rs` (real cargo).

## 2026-09-16 — Issue 117 completed (the delta half): the six remaining stubs + the three silent tails — and the dual-implementation yield behind it
- The staleness guard landed TWICE in one window; this session's duplicate YIELDED, additive delta re-landed: six stubs (`stake`/`unstake`/`vote`/`pay`/`faucet`/`wetrun`) route through `build_stamp::refuse`; the three auto-push tails print loud SKIPPED lines naming the rebuild.

## 2026-09-16 — Issue 115 resolved: the auto-oracle feature knob (`RIIR_HEAL_AUTO_ORACLE_FEATURES`) — gated diagnostics leave the `[heuristic]` blind spot
- `features_from` → `--features <csv>` injection, loud knob-active line, both failure arms name the knob; `RIIR_HEAL_AUTO_ORACLE=0` still wins; manual `--clippy-json` wins over both. E2E proven on a gated scratch crate (with the knob: `[oracle] needless_return` anchored).

## 2026-09-16 — Issue 113: the auto-oracle — `cargo heal` now runs the clippy oracle for you (the "agave 6951 rows → +1 credit" session)
- With no manual feed and a rust lane, a scan over a cargo project spawns `cargo clippy --workspace --no-deps --message-format=json --quiet` at DEFAULT features in the detected root; loud heuristic-only fallback; kill-switch `RIIR_HEAL_AUTO_ORACLE=0`. A gain path behind a discoverability wall is not a gain path.

## 2026-09-16 — Issues 110 + 111: two silent-wrong-verdict defects, both found by running this crate somewhere it had never been run
- 110: all three scratch-crate oracles BLAKE3-key the directory but wrote a FIXED package name — under a shared `CARGO_TARGET_DIR` two crates are one cargo unit and the second gets the first's cached answer (the more carefully somebody isolates, the more reliably the oracles lied); fixed: package/bin name carries the BLAKE3 stem + pinned `CARGO_TARGET_DIR`. Found via riir-train Issue 555 (the child shed it first). 111: `ModRowLattice` `Path::join` emits `\` on Windows against `/` keys — unresolved mod rows degrade the lattice to empty and INVENT platform-dead findings; separator-insensitive index. Exposure adjudicated from artifacts: Bench 013's 31/31 distinct profiles prove no shipped perf row was contaminated; `ClippyVerifier` `control_fires` shows the n−1 signature in fixer_quality_history.

## 2026-09-16 — P30: the bench-target guard (rust_perf edits declined-and-flagged on measurement targets)
- `is_bench_target(path, src)` (path axis + harness-signature axis; `black_box` NOT a marker); `--fix` declines perf edits on bench targets with loud lines; `--suggest` stamps `⚠ bench-guard`. Residue counted: 0 measurement-altering heal edits landed in consumer bench files. Collision note: the wiring rode a sibling's commit; the module + docs commit completed the landing.

## 2026-09-16 — P33: use-row lexical scoping (platform-audit; the riir-chain `BrowserNodeState` intake)
- A module-PRIVATE use row reads only same-file mentions at its scope or descendants; pub re-exports keep name-global evidence. Measured: riir-chain CLEAN; the two `use anyhow::Context` rows mechanized away; full workspace re-sweep zero new rows.

## 2026-09-16 — P32: the `[C2·test-rig]` downgrade (config-audit; the riir-ai `Outcome`/`ProbeConfig` intake)
- A [C2] row renders `[C2·test-rig]` when the decl is tests-tree AND every read is under tests-path files (path axis kept separate from cfg(test) confinement; name-in-both-trees not eligible). riir-ai converts exactly the two predicted rows.

## 2026-09-16 — P29: the `[A·documented-write-only]` class-A extension (closes the P28 handoff)
- Class-A rows take the same marker arm class B uses (`is_deliberate_write_only_marker`); gpu-uniform keeps precedence. riir-ai `HlaKarcConfig.delay_k` renders weak; riir-engine field docs carry the marker wording (their repo).

## 2026-09-16 — P28: the `[gpu-uniform]` downgrade (config-audit × shader uniform blocks)
- `collect_gpu_uniform_structs` closes `#[uniform(…)]` blocks transitively; rows render `[gpu-uniform]` (downgrade-not-suppress — WGSL consumes the bytes, the detector cannot see it). seal-game-editor 2 [A] → 0 [A] + 2 gpu-uniform; canaries hold.

## 2026-09-16 — Issue 102 T5: the corpus authoring meta-rule guide (Library Drift's meta-skill, formalized)
- `.docs/02_corpus/authoring_discipline.md`: route-before-propose, vocabulary-translation dedup, probe-first, decision-content, bounded fix-space, honest tiering, fixpoint safety, GOAT + count-pin walks, provenance/`.raw` discipline, distinct-from rows, meta-recursion. Nothing new is policy — every rule cites the practice it formalizes.

## Issue 107 CLOSED 2026-09-15 — NEGATIVE on evidence; T3–T6 closed UNBUILT (record; file removed per the noise-reduction rule)
- T1: horizon 1 < 2 on every corpus; where a readout AUC is 1.000 the history floor MATCHES it (sufficient-statistic verdict). T2: the span-embed column did NOT revive the lane on synthetic corpora either. No corpus on which outcomes are NOT strategy-determined → T3–T6 closed UNBUILT. Reopen: organic fixseq rings with ≥2-revert runs + the embed column; store densification; any corpus where the resolved rate moves between orderings.

## 2026-09-15 — Intake class P23: the `useless_format` escape guard on the oracle lane (clippy's own suggestion, refused where it is wrong)
- The oracle lane drops any `useless_format` diagnostic whose covered call text contains ANY brace (a brace in a compiling no-args `format!` literal is an escape; clippy's `.to_string()` suggestion reuses the ESCAPED literal and would freeze the doubled form).

## Issue 107 T2 — the span-shape embedding column lands (write-time capture) + the embed arm (`src/span_embed.rs`)
- Write-time capture at the one `push_attempt` chokepoint (span bytes DRIFT as fixes land); hashed unigram+bigram bag, signed feature-hashing, L2-normalized, ×127 i8. Measured: auc(embed) = 1.000 WITH floor 1.000 at k=1 — BELOW-FLOOR, the same sufficient-statistic verdict.

## Issue 107 T1 — the fixseq horizon report + the lane-viability verdict (`src/horizon_report.rs`)
- Strict-lead features at i-k, LORO-held-out ridge readout, base + history-aggregate floors, three refusal classes. Measured: horizon = 1 on every corpus; the unblock was riir-kat `4bc8672` (ungated transport-tier imports E0432 at default features).

## Plan 127 — P015 Phase 2: the `cargo heal replay` client landed (`4e1983ee`, byte movers riir-kat `a80ec7a`)
- Lease → local clippy-oracle replay → signed verdict; `blake3(rustc -vV)` fingerprint, step-0 canary hash check, the pinned single-item rate rule (determinism law making K=2 agreement possible); the measured rule change: REMOVED-set evidence (ambient lints cancel in the diff BY CONSTRUCTION). Kill-switch `RIIR_HEAL_REPLAY=0`; lane inert until Phase 3.

## Issue 106 — the infron free leg (groq → infron → CF ladder) landed (`859dc6de`)
- `GroqClient` → generic `OaiClient` over static `EndpointConf`; 3-leg `LlmRouter` with windowed disable; live probes: the free pool is account-gated (429 "requires Team balance > $4.99") — activation = top up; degradation is exactly right (first 429 disables infron for the window).

## Issue 105 — the journey-credit third axis (PC-ALM dual accumulator) landed (`e58ff9b9`)
- The strand-keep poison (a KEEP ending at a typeck-masked E0614 scored as full success): `JourneyCredit` over the fixseq ring + `journey_factor` discounts success evidence in tainted journeys; EXACTLY 1.0 with no journey data (pre-105 stores bit-identical). Gate `tests/journey_credit_gate` 3/3.

## The 09-14 evening queue-4 hygiene sweep — eleven more seal-game-editor issues closed on code evidence, the numbering-collision defect measured and filed as issue 151 (2026-09-14)
- 11 issue files removed on landed-at-HEAD evidence (seal-game-editor `2415427b`); issue 125 deliberately left OPEN (verified not fixed); issue 151 filed (TWELVE dual-allocated plan numbers measured).

## The 09-14 evening queue-1 unit — seal-game-editor issues 088/099/100 closed on verified HEAD evidence, the 088 sqlite_store half fixed (2026-09-14)
- 099/100 verified fixed at HEAD (carrier FilterDropdown + relation-driven options); 088's remaining half (two orphaned doc blocks) deleted. seal-game-editor `bbbeac56`.

## P21 post-mining intake — the oracle-suggestion-children consumption: the `--clippy-json` fix lane applies clippy's own MachineApplicable suggestions verbatim (the P15 Unknown-receiver residue closed) (2026-09-14)
- `FeedSuggestion` capture (MachineApplicable-only, single-child, primary file) + `oracle_edits` merge (gap text preserved; whole-file parse gate, all-or-nothing) + pass-0 wiring (proposer edits win on overlap). E2E: three Unknown receivers healed verbatim; stale-feed re-run drops loud, file byte-identical.

## P15 post-mining intake — `unnecessary_map_or` lands: the born-lint closure, the `receiver_kind` substrate extraction + the SAFE-LET extension, and the gen2/gen3 re-freeze (2026-09-14)
- Probe-first: the full firing envelope (Option+false→`is_some_and`, Option+true→`is_none_or`, Result+false→`is_ok_and`; Result+true SILENT — no `is_err_or` in std). `receiver_kind` substrate + SAFE-LET extension (exactly-once binding, never assigned, not a param) + the proposer (Unknown REFUSES). P-numbering note: the P-series is per-session and has dual-allocated before.

## The 09-14 production re-heal of seal-game-editor — the P13 "owning lane's move" discharged same-day (2026-09-14)
- Oracle-fed `--fix --write --verify` at `d310c030`: 25 verified edits / 9 files, findings 49 → 19 (the remainder is exactly the documented divergence classes). The fresh feed caught 3 `unused_must_use` in the same session's own new tests (cfg(test) code needs `--all-targets`).

## The 09-14 morning re-arm pass — sweep blockers narrowed to ONE (riir-ai's stale checkout), healer HEAD binary staged clean-of-sibling-WIP, seal-game-editor #147 closed as the queue-1 unit (2026-09-14)
- riir-viewbridge CLONED (16/20); every sweep re-arm gated on riir-ai advancing. Issue 104 RETRACTED (false alarm; lesson: `git ls-remote` lists ref TIPS not ancestry; `git merge-base --is-ancestor <sha> origin/<branch>` settles any "history loss" allegation). Junction farm at `C:\git2` (E: is exFAT — no junctions). seal-game-editor #147 closed at `d310c030`.

## The 2026-09-14 Protocol B sweep — 10 repos screened, seal-game-editor healed oracle-fed (the queue-3 unit; first large multi-crate `--clippy-json --fix --write --verify` run) (2026-09-14)
- Dev-sweep network discipline: `RIIR_HEAL_KAT_METER=0 RIIR_HEAL_KAT_GATE=0 RIIR_HEAL_KAT_SERVICE_URL=http://127.0.0.1:9` neutralizes the lease context. Screen: 4 CLEAN, seal-game-editor healed (`e2112bce`, 61→49), 4 blocked with named re-arm conditions, riir-shader recorded-not-touched. Lessons: `RUSTUP_TOOLCHAIN=1.98.1` when the target has no pin file; single-file checkout beats stash for HEAD-vs-heal adjudication; branch `e2e_item` healed (52 × future-incompatible `float_literal_f32_fallback`).

## P13 post-mining intake — the `chunks_exact_to_as_chunks` CHAIN-ROOT extension lands (the sweep's own product consumed same-day) (2026-09-14)
- Real clippy fires on iterator-CHAIN roots; the ROOT is rewritten, downstream adapters compose unchanged; the `_mut` twin inserts `.0.iter_mut()` — a MEASURED safer-than-clippy divergence. LIVE-FIRE: 37 proposed / 24 kept, all oracle-Certified; new divergence class: MACRO-ARGUMENT chains (syn's visitor does not descend into macro token streams).

## P12 post-mining intake — `needless_pass_by_ref_mut` lands: the 2026-09-09 nursery-gap row closes, probe-corrected twice (2026-09-14)
- Probe corrections: `pub` methods SILENT, `pub(crate)` FIRES, trait-impls SILENT; the "must emit both halves" premise wrong (the gate is errors-only; clippy's own suggestion is definition-site-ONLY — the cascade emits as a narrow provable shape). VERIFY tier; the corpus LEASE gap signal was honest for that config (the rule reaches leased users at the next sync).

## P14 post-mining intake — the cross-repo consumer-surface hint landed: the FP-class #3 open candidate closes (2026-09-14)
- `src/downstream_hint.rs`: after `--config-audit`/`--orphan-report`, names every sibling repo ON DISK whose manifests path-dep into the scanned repo (measured-graph discovery from manifests, NOT BOUNDARY.md tables — prose drifts, manifests don't); root + member manifests only, ~50 ms.

## P13 post-mining intake — the FP-class #3 refinement (a) landed: trait-impl reads are external, and the probe surfaced the payload-Result class (2026-09-14)
- katgpt-rs 772 trio reproduced exactly; fix 1 trait-impl externality (3→0); the probe then corrected the intake's own attribution — `SchurSolver` is INHERENT payload-`Result` (fix 2: `Result<Payload, E>` = flow-out, `Result<(), E>` = confining). riir-train 1→0.

## P12 post-mining intake — `useless_conversion` shape 2 (the `.into_iter()` call-arg drop): the probe re-keyed the lint, the extension rode the existing type-blind tier (2026-09-14, `115a9185`)
- The probe inverted the intake's key: the fn-arg shape fires as `useless_conversion` (`useless_into_iter` is the for-loop shape, pedantic); direct call argument `<expr>.into_iter()` replaced by its receiver. Windows red fixed portable in `bbea191e`.

## The physician-heal-thyself self-audit — config-audit + orphan-report over riir-clippy itself at FULL scope (2026-09-14)
- config-audit clean both scopes; orphan-report 14 stillborn, ALL adjudicated benign (0 actionable TPs). The scope lesson: the src-only invocation's 15th row was a SCAN-SCOPE FP — shipped as `src/scope_hint.rs` (`d4e87f54`).

## `cargo heal --version` exit-2 regression — found by the global-binary rebuild, closed by merging the two hand-mirrored flag lists (2026-09-13, `ad31a04a`)
- The 09-12 fix completed `is_bare_posture` but the SECOND hand-mirrored paths-guard list kept its missing row; both now consume ONE shared `standalone_mode()` predicate — the mirror-drift class structurally closed.

## The `--corpus-json` lane retired (2026-09-13, riir-dapps Plan 026)
- `gen_dashboard.py`'s `--corpus-json` + `write_corpus_growth_json` deleted (the front-page corpus card went chain-native); only the consumer-facing export lane is gone. Phase-2 follow-up = Issue 103.

## P11 post-mining intake — `format_collect` (the first two-span clippy rule) + the `needless_borrow` receiver-shape extension (2026-09-13)
- `format_collect` two-span capability resolved BY MEASUREMENT (anonymous `as _` cannot collide; io::Write ambiguity impossible). `needless_borrow` receiver extension `(&x).m()` → `x.m()` for the whitelist. RERANK_POOL 16→20 (the rescue guard's first designed fire).

## P10 post-mining intake — the 089-T3 corpus-gap pair landed, and the probe inverted the intake's own description (2026-09-13)
- `filter_map_bool_then` fires on `bool::then` INSIDE `filter_map` (the split is the FIX), `vec_init_then_push` help-only in real clippy; both VERIFY-tier, corpus 49→51; attributed pushes refuse (a `vec![]` literal cannot carry cfg'd elements).

## Issue 101 resolved — the E0502 read-relocate: F3 cleared, interacting corpus 100%, the first ordering IMPROVEMENT (2026-09-13, `32b19a1b`+ / Bench 091; hashes post-rebase on `5042af31`)
- Design 1 (relocate-after-last-use) MEASURED DEAD before landing; the shipped hoist-to-first-use template cleared F3: interacting corpus 100%, driver 66.7% (unfixed 12→0), G1 172/172. T4 A/B: every arm 46/46, learned arms at 10 reverts vs static's 24 — the gain is WASTED ATTEMPTS, not resolved rate; Issue 093's negative verdict STANDS.

## Issue 089 T4 hardening — the bench-tail frontier snapshot: generation boundaries record themselves (2026-09-13, `11c1799d`; the 7 lane runs + snapshot row #2 recorded the pre-rebase build sha `b17c2dd8` — identical diff, the rebase only moved the base)
- `finish_score_bench` (the ONE tail every generation lane runs) appends the snapshot — a boundary nobody reports at is one the correlation never samples. T8 accumulation rode along (8 rows carry `rule_outcomes`).

## Issue 093 resolved — per-run strategy posterior: PoC win real, wired policy NO GAIN anywhere, ordering closed NEGATIVE on evidence (2026-09-13, `0fec5257`+`8993ec51`+`27e8ffcf`+`b08ef85d` / Benches 084+085+088+089+090)
- PoC WON on the toy (41.7% vs 35.2/36.7); wired green with honest gain-NULL (no real surface with revert dynamics); on the interacting corpus every learned ordering REGRESSED (10/34) — the keep signal was the defect. Final: static min-delta stays shipped; opt-in, out of the release set; never ship a long-lived `NonergodicOrdering` handle. Reopen triggers: store densification, organic rings, any corpus where the resolved rate moves.

## Issue 100 resolved — the unmasking trade: E0614 playbook + lexicographic keep rule, the T4 regression REPAIRED (2026-09-13, `65b9ff11`+ / Bench 090; hashes post-rebase on `ae9da3d6`)
- The playbook alone was inert; the load-bearing half is the lexicographic keep `(masking, count)` with `MASKING_STAGE_CODES = ["E0614"]`; F4 = the first legal equal-count keep; regression repaired to 34/46 parity for every arm. Floor: N 246→258, G1 160/160.

## Issue 099 resolved — the interacting-error corpus: gate 34/34, first non-zero reverts, learned orderings REGRESS clean 22→10 (2026-09-13, `a384acc1`+ / Bench 089)
- 34 shared-binding E0502×E0499 pairs in 3 families; first non-zero revert rate on the stock driver; the per-attempt keep signal is NOT a run-outcome signal on interacting corpora (clone-out strand-keeps score well in counters but strand the run at a masked E0614).

## Issue 91 resolved — fix-loop trap detector: T2 measured NEGATIVE (structural), T3 recorded, detector NOT built (2026-09-12, `e44e885f` / Bench 087)
- 0 flips on all three keys over 230 attempts — the shipped `--fix-compile` loop carries a well-founded measure making a within-run state revisit IMPOSSIBLE; a detector with no phenomenon is not built. Reopen triggers in Bench 087 §6.

## Issue 097 — linking↔confusion PoC: NO-GO, the fusion idea retired as measured-refuted; RESOLVED (2026-09-12)
- Both config classes refute (k=8 degenerates to 99.9% linked; k=3 shows ANTI-enrichment); the Spearman trap named (ρ moved with tie structure, not the confused-pair set). Mechanism: rename ORBITS, 1 exemplar/rule — the manifold assumption fails on this corpus. Reopen: a real-repo mined span set.

## Orphan-report serde boundary — the riir-ai Issue 934 follow-on repair (2026-09-12, `d2e39d1f`)
- The boundary is the ATTR, not the derive pair: `skip_serializing_if` fields excluded (conditionally-emitted OUTPUT schema), plain `#[serde(default)]` fields stay candidates (the canonical `BehaviorConfig` Ser+De case); `#[serde(skip)]` stays a candidate.

## Plan 126 — orphan-report: intake class C1, the config knob orphaned by optimization (2026-09-12)
- `--orphan-report`: two-leg report (zero-external-read + git-history pickaxe evidence of vanished readers); serde-literal candidate extension forced by the canonical case (no `impl Default`); the intake's "reader that vanished" narrative was syntactically FALSE — the knob was BORN unwired, fired honestly as `[stillborn]`. En-route: canonicalize scan paths before `strip_prefix` on macOS.

## Plan 125 — config-audit class C2: the self-referential config struct detector (2026-09-12)
- Whole struct whose every Default-impl field has zero EXTERNAL reads; the landing's own bug: syn 2 impl methods are `ImplItem::Fn` — `visit_item_fn` never fires inside impls (need `visit_impl_item_fn`). First-contact yield: 1 verified TP / 0 FP (riir-games `PetAiConfig`).

## Issue 91 T1.5 — the per-attempt ring: `--fix-compile` attempt sequences to `.heal/fixseq/` (2026-09-12)
- `AttemptRecord { seq, code, span_hash, strategy_id, outcome, errors }` at ALL five loop outcome sites; `span_hash` = blake3(`file:line`) 16-hex; one `run-<ts>.jsonl` per write-mode run. The instrumentation prerequisite for T2 (oscillation) + Issue 093 (per-run posterior).

## Issue 089 T4+T8 — the evidence-emission halves: band-mass snapshots + per-rule bench rows (2026-09-12)
- T4: `--frontier-report` appends band-mass snapshots to the committed `.benchmarks/frontier_snapshots.jsonl` (state-change dedup; `generation` = the FRONTIER, lanes interleave). T8: every bench row carries per-rule `rule_outcomes` (single-target attribution only). Both stay open on ACCUMULATION, emission now live.

## Issue 089 T7 — the shaped 0.5 verdict: generation 8 + the Elo-tie / Beta-half-count feeds (2026-09-12)
- "Target cleared ∧ new diags" = first-class: priced 0.5 in the bench, TIE in Elo, HALF-COUNT (+0.5/+0.5) in Beta; gen-8 admission is SHAPE non-vacuity (n_shaped ≥ 1), deliberately not the M5 heal_rate comparison. Freeze fences: INNOCENCE + SHAPE gates.

## Issue 089 T6 — the stratified score-bench read + the D-formula decline (2026-09-12)
- Every clippy-lane bench run splits heal rate by fixture provenance (repo/authored/construction/other) via the `ood` mining join; v1's 96.2% aggregate is authored-dominated (authored 98.0% vs repo 50.0%). D declined on the record. En-route: the score_bench BIN lane had been E0631-broken (a lane nobody compiles).

## Issue 089 T5 — provenance weighting: the priced pair namespace + the enforced rating fence (2026-09-12)
- `PROVENANCE_ORGANIC` / `SYNTHETIC_PAIR_WEIGHT = 0.05` / unknown fail-closed 0.0; `rate_pairs` ENFORCES the non-circularity fence (only construction rows feed the BT fit — observed wins are never rating evidence); the outcome-feed split measures band-mass PER FEED.

## Issue 088 resolved — the kat service-plane spin-out: every task + fleet follow-up landed, file removed at close (2026-09-10 → 2026-09-12)
- The KAT service plane (25 modules) → the private sibling `gist-rs/riir-kat`; riir-clippy is a consumer via non-optional path dep, root re-exports at historical paths; three healer-coupled keepers stayed root. Full guard GREEN after 8 latent rot classes fixed; fleet follow-ups closed 09-11 (katgpt-rs repo count 17→18; dapps/dao protocol-tier adoption).

## Issue 090 resolved — latent-only test lane: 17 domain-panic tests gated on the calibration state (2026-09-11, 23e051bc)
- The three draft test modules gate on `all(test, latent_retrieval, clippy_lints, rust_perf)` — the state the fixture set + floors are calibrated on; the issue's sketched `any(single-domain)` gate would have been WRONG (the tests assert Clippy rows).

## Issue 085 — the `dist` domain: the promotion-evidence narrative (2026-09-09 → 2026-09-10)
- Phase 1 `79dcf9b` (6th retrieval domain, no-syn Cargo.toml scanner, suggest-only); Bench 082: score-bench FLAT by construction, consumer scans true-positive on game-sdk/deployer; B1 (autodiscovered bin) + B2 (root profile knobs) closed `759e209d`; PROMOTED DEFAULT-ON 2026-09-13 (owner pull; all four GOAT conditions PASS; `--no-default-features` is the tested flag-OFF path).

## The domain swap — the client half + the config surface, post-flip state (2026-09-10, riir-dapps Plans 023+024)
- The apex IS mainnet now; `devnet.ai.gist.rs` hosts the MOVED fleet ledger; the fallback pair (default→devnet ONLY) is load-bearing in the other direction; `cargo heal config set --url <alias|url>` / `config get` (`cf236f1a`).

## Issue 081 resolved — burn wire unit mismatch: end-to-end closed, rescale STOOD DOWN (2026-09-10)
- Client sent micro book value in the whole-KAT wire field (clamp-wiped the grant, poisoned the watermark); fix `e9eca71d` + the cross-repo unit golden; the rescale STOOD DOWN — the watermark row was already WHOLE-SCALE (the CAS guard refused without touching it). Lesson: a money unit has ONE name per side.

## Issue 087 resolved — kat_burn_stack e2e: stale harness units, not the account plane (2026-09-10)
- The harness bills 11 µKAT/run; `wire_kat_total(11) = 0 ≤ ack 0` is a legitimate Skipped — the harness was the stale side; fixture now bills exactly 1 whole KAT with a self-assert. Lesson generalizes Issue 081's: a contract change must re-pin its e2e the same day.

## Issue 080 resolved — the sticky-grace marker retired, server truth only (2026-09-10)
- `kat_contribution.rs` deleted (the forgeable `contributed_once.json` is gone entirely); gate predicate `ack.contributed == Some(true)` (server truth ONLY); tests re-pinned both directions; dated removal landed once the fleet was v3 (`143c5a8`).

## Issues 077 + 082 resolved — margin-weighted mining + the OOD axis (2026-09-09)
- Issue 077: the e^{−2M} margin law, OOD axis always-on; T3 verdict DEMOTE (retrieval-similarity margins do not select lint-bearing spans). Issue 082: R1 CONVERGENT VALIDITY DEMONSTRATED (gap −0.7608 on BOTH fresh families — the composition attribution held); `regime_probe` stays opt-in.

## Issues 083 + 084 resolved — the green-zero class and the telemetry-key consumer (2026-09-09)
- Issue 083: all 16 `#![cfg]`-gated test targets got their `[[test]]` rows, verified in BOTH directions (13 exact feature-set builds; SILENT-NOW 16 → 0). Issue 084: filed as 082, RENUMBERED 084 before removal (the OOD issue had consumed 082 — the standing fix is renumber + citation rewrite); `parse_supply` reads `*_micro` first, falls back to `*_kat`.

## Plan 117 — `cargo heal --refill`, the dev/test faucet refill (2026-09-09)
- Composes shipped wire surfaces (`HttpPayreqTransport` + `HttpWetrunTransport`); the client adds no host gate deliberately (prod never distributes a crank token); server truth wins on the gate input; exit 0 iff the final synced balance is positive; crank leg fail-soft.

## Both shell gates could have reported a PASS on an abort (fixed 2026-09-07)
- macOS `/bin/bash` 3.2.57 enters the EXIT trap with `$?` already 0 on an unbound-expansion/eval abort — a trap whose last command succeeds makes the abort exit 0; only a completion sentinel catches it. Both `scripts/ci_feature_guard.sh` + `scripts/ops_nightly.sh` repaired and verified against their real preambles. shellcheck does NOT find this.

## Batch-mining batch records + Role/domain/pipeline full record (was AGENTS.md lines 23–3043)
- Compacted out of AGENTS.md 2026-09-06; the Role/domain records + corpus-build batch narratives live below and in git history. NOTHING was deleted.

## Role
Multi-domain modelless code healer behind the `HealDomain` trait (Plan 001) — developer tooling, NOT game/training/chain. L0 rule distillation → L1 ternary SIMD drafter → L2 syn validator (+ opt-in domain oracles) → L3 Pareto search (opt-in) → L4 LoRA seam (inert unless installed; NOT promoted — riir-train Plan 336 measured FAIL 0/60; `decline` is a correct answer).
- **clippy_lints** (default-on): 49 corpus lints (Batch 91) — 38 FIXABLE + 8 VERIFY + 3 self_evolve-only (Batches 89–91; `option_if_let_else` REFUTED fixpoint ping-pong, `missing_const_for_fn` KILLED).
- **rust_perf** (default-on since 2026-08-13): 101 rules (Batches 48→112: 48→58, 53→62, 59→63, 61→65, 62→67, 64→69, 65→76, 66→71, 67→72, 68→73, 69→74, 73→78, 74→80, 75→83, 77→88, 78→88, 83→89, 85→91, 86→94, 91→96, 92→97, 94→97, 99→98, 101→99, 109→100, 112→101) — Pareto validator the promotion gate; 34+ rules carry `context_notes`.
- **kernel_opt** (opt-in): 552 rules (Batches 1→119: popcorn 1–13, riir-gpu production kernels 14–51 incl. Mining CONFIRMED production bugs filed as riir-ai Issues 674–710, measured-GOAT distills 49/52/54–71, paper distills 70/79/80/82/83/87/92, external engine distills 71/77/84/92/95–118); 15 category experts + thermal router (Warm/Cold/Freeze = 4090 LoRA, deferred).
- **`rustc_errors`** (6th domain, driver-based): `--fix-compile` oracle-driven compile-error repair (see the dated records).
- Plan 005 history: T3.5 op-tokenizer (top-1 54.2→70.8%); T3.5.4 instantiation re-rank (CONTAMINATED deltas — `RerankMode::default()` stays Off); T3.5.5 uncontaminated oracle (Structural promoted default, Phase 3 CLOSED at 96.0%); Issue 024 pool crowding (RERANK_POOL 8→12); Issue 016 structural expansion 6→33 + VERIFY tier; Issue 017 retrieval eval at scale (58.3% top-1 — the 12-fixture claim dead at scale); Issue 018 `self_evolve` DEFAULT-ON (`RIIR_HEAL_SELF_EVOLVE=0`); Issue 022 corpus real-clippy drift (5 entries fixed); Issue 020 router spectral POC KILLED (lexical ~70% ceiling); Issue 021 EvidenceTier (Certified/Heuristic/Withdrawn, absorbing-Withdrawn); Issue 015 `--fix --verify` compile gate (2 real reverts in the GOAT run); Issue 040 fixer re-gate (strict-keep 6/9 vs decline 0/9; qwen3.8 unusable on free tier).
- Issue 031: 3 healer defects from the katgpt-pruners heal (H1 CRITICAL `comparison_to_empty` self-recursion → stack overflow, invisible to the compile gate; H2 `--domain` never scoped the fix path; H3 verify baseline default-features). Plan 014/Issue 032: frozen score bench + run #1 caught a real proposer bug (`needless_range_loop` non-atomic rewrite; heal 84.6% → 90.4% → 98.1% at run #3). Native rustfmt: fmt-clean-at-HEAD gate, CRLF skip, edition from Cargo.toml.
- Cross-repo heal-sweep campaign (katgpt-rs / riir-train / riir-neuron-db / riir-chain / riir-ai slices, 2026-08-18/19): katgpt-rs 30 member surfaces + riir-train engine tests/examples/gpu + ndb + riir-chain terminal; riir-ai engine 174/979 + games-civ 142/1552 + examples 28/278. Gate-scan lessons: the baseline run is the reliable detector ("0 passed" ⇒ exclude); whole-file `#![cfg]` grep mandatory; feature-gated heals MUST pass `--verify-args`; `--fix --verify` WITHOUT `--write` writes; residue scans filter by warning TEXT; `git fetch origin` BEFORE sizing; `git add` commits the whole index (shared checkouts); exFAT has no junctions — worktrees under C:\git2; /tmp worktrees break sibling path deps.
- Issues 044+045 (CF ops) / 050 (Warm-tier corpus+score series) / 059+060+061+068 (KAT product loop: D1–D9 recorded; devnet advisory soak EXECUTED riir-dao `e7b1f89`; D9 create `gist-rs/cargo-heal` was the blocking owner act) / 062 (TPR fusion CLOSED-NEGATIVE — OOD at chance; `tpr` stays opt-in) / 065 (cost model: alert reconciles the account-wide CF bill, `CF_BILLING_ANNIVERSARY_DAY` pin) / 066 (docker corpus counts wrong since landing — repaired via the sanctioned dated writer) / 067 (Trace-as-State arm G2 PASS; growing-input class REOPENED) / 069 (four duplicate doc numbers resolved by citation-weight arbitration; one counter per directory ENFORCED) / 070 (ThoughtComm α-fusion dead — cross-domain rule-id overlap 0/129; T3 collapse detector landed) / 071 (docker tier-2 misread as Docker drift — registry round-trip per `FROM alpine` fixture; fixtures ride `FROM scratch`, 399 s red → 2.7 s green) / 072 (Frontier-evidence Challenger: k-ladder generations 2–7 built — the Bench-069 difficulty-axis list FULLY EXECUTED) / 073 (XBridge anchor-primacy A/B FAIL — instrument-structural; zero identity-collapse events in 48 calls) / 074 (trust channels: `rule_context_note` per-domain tables; T4 DEFERRED) / 076 (burn fast-settle e2e + the 075 soft gate) / 078 (QR goldens encoded the RETIRED host — re-derived + a tie test failing AT THE URL LINE).

## CF ops surface + the `.env` contract — full record (was AGENTS.md lines 3044–3177; Issues 044/045/051/062/065 resolutions + dashboard-auth state + deploy hazard)

## CF ops surface + the `.env` contract (Issues 044/045 — read before touching ops)
LLM ops plane: `riir-kat-housekeeping` worker, `riir-ops-dashboard` worker, nightly generator (`scripts/ops_nightly.sh`); credentials in `.env` (gitignored; pipe into `wrangler secret put` via stdin, never echo). Issue 051 T8 defer preserved: `fixer_regate` arms stay CF-pinned BY DESIGN (model comparisons). Issues 044+045 all landed + live-verified; residual owner-gated items: dashboard FLIPPED PUBLIC 2026-09-04 (`AUTH_MODE="public"`; the token gate DORMANT, deleting the var re-arms it), push paging owner-gated (`ALERT_WEBHOOK_URL`/`MONTHLY_BUDGET_USD` absent; the budget alert now reconciles the account-wide CF bill, period rides `CF_BILLING_ANNIVERSARY_DAY`), KAT mint armed-but-INERT until the owner pushes `KAT_SERVICE_URL` + `KAT_MINT_TOKEN`.

## The `riir-clippy` auto-continue skill — full description (was AGENTS.md lines 3178–3212)

## The `riir-clippy` skill (auto-continue idle loop)
`.agents/skills/riir-clippy/SKILL.md` (canonical home THIS repo, global is a symlink) encodes the idle-time workflow — 13 queue items (fix issues → finish plans → clippy sweeps → resolved-issue hygiene → batch mining + dry-run → boundary check → substrate-first → benchmark sweep → online unblock → defer-marking → online perf → paper research → doc-sync), one unit per cycle, commit + push each. Resume state: `.distill/001_mining_queue_snapshot.md` (moved out of the skill file after it grew to 104KB and broke Zed's loader). Issue 019 T6 closed: all 13 items exercised ≥1×.

## Production adoption — full record (was AGENTS.md lines 3213–3525; verdict evidence + resolved issues 037/038/050/059/060/061/066/067/068/069/070/071)

## Production adoption — `cargo heal` as the dev-time default (2026-08-24)
Verdict GO (live trial): 12/12 planted fixes landed, 2/2 divergence classes correctly refused, compile gate 2 checks / 0 reverts, idempotent. Global binary + post-mining intake (mining improvement candidates from the healer's OWN usage artifacts; seed inventory 8 ranked classes; the 8,235-trajectory zero-outcome-verdict gap closed by Issue 037 T2). Issue 038: self-evolve memory onto the neuron-db Warm-tier store under `latent_retrieval` (one-time envelope migration, once-only fence; DAG/AST storage DEFERRED with a named consumer). Issue 037 CLOSED 2026-08-26 — ALL GATES GREEN, A1 cadence MET day 3, T7 discharged (Proposals 003/004 UNBLOCKED).

## Guard-landing protocol — full record incl. the measured story (was AGENTS.md lines 3526–3583)

## Guard-landing protocol — a guard is not landed until the residue is counted
Measured, not hypothetical: the `227637e` guards were correct yet 32 pre-guard residue sites landed the next day. The screen is the claim: `cargo clippy --workspace --all-features --all-targets --keep-going` (not `check` — the guarded classes are accepted by check's typeck and rejected by clippy's; not default features — all 32 sites sat behind non-default ones). Traps handled: an up-to-date build emits nothing (→ `cargo clean -p` per member in a sweep-local `CARGO_TARGET_DIR`); the JSON shape is `"code":{"code":"clippy::<lint>"}`.

```bash
scripts/consumer_residue_sweep.sh <lint> [<lint> ...]        # the screen
scripts/consumer_residue_sweep.sh --grep '<regex>' <lint>    # narrow first
```

## Plan 001 multi-domain architecture — full record (was AGENTS.md lines 3639–3665)
### Plan 001 multi-domain architecture (2026-08-13)
Plan 001 generalized the clippy-only healer into the multi-domain `HealDomain` framework; Proposal 012's GPU kernel optimizer absorbed as `kernel_opt` (user correction overrode the "new repo" recommendation); cross-domain `--domain auto` fans out with the router verdict as a per-span ANNOTATION, never a gate (the 8-D centroids mis-routed fixtures).

## `kernel_opt`-specific layers — full record (was AGENTS.md lines 3666–3685)
### `kernel_opt`-specific layers (Phase 3)
1. `KernelExpertRouter` wraps katgpt-core `pick_domain<15, 8>` (8-dim activity vector, 15 per-category centroids, 15/15 reachable; `build_activity()` the consumer constructor; modelless argmax, zero alloc).
2. `KernelThermalRouter` — Plasma→Hot→Warm→Cold→Freeze (`ThermalTier` `#[repr(u8)]`); the scaffold returns Plasma (modelless-first); Warm/Cold/Freeze require 4090 CUDA + LoRA (deferred to riir-train).

## Feature Flag Discipline — full record (was AGENTS.md lines 3760–3809; per-feature batch narratives + corpus-growth history)

## Feature Flag Discipline
`default` = `["clippy_lints"]`. Per-feature records: rust_perf promotion (Bench 013/015/016/017/020 GOAT; 37/40 benchable rules Pareto-dominant incl. `regex-hoist-lazylock` 5443–5675×; 2 honestly dominated, 1 Mixed), latent_retrieval (Bench 018 G1–G4), rustc_errors default-on (Bench 045 N=212, uncontested 105/105 where cargo fix is 0/105), docker default-on (Bench 056 + riir-chain adoption; the T1.6 falsification: `cargo-locked-flag` broke the build → demoted suggest-only, 14 bounded not 15; tier-2 oracle `docker_buildcheck` opt-in by dependency class), sec (Bench 046 + Batch 88 + P8 `nan-unsafe-sort-comparator`), self_evolve (Bench 023/024, G6 40/40 truthfulness), l4_lora seam (NOT promoted; three-condition reachability contract incl. Issue 030 relevance gate — a parsing AND relevant modelless fix is never replaced), `clippy_lora` DEAD PLACEHOLDER, contrastive_scope (Bench 040), ternary_inference (Bench 010; the riir-ai cfg-gate fix `4bfe3b30`), kernel_verify STUB, ruliology_search, wasm_pruner deferred. The modelless path (L0–L3) is PRIMARY; **a syntactic check (`parses`) is not a relevance check** (Issue 030).

## Full text of remaining sections compacted in AGENTS.md (operative boilerplate — archived so nothing removed is unpreserved)

## Boundary contract — read `BOUNDARY.md` first
Full text preserved in AGENTS.md (canonical). Domain test: developer tool, zero game-domain coupling in the default build; enforcement via `../riir-ai/scripts/ci_boundary_contract.sh`; violations file the issue FIRST, add the drift row, then fix.

## Why this is a separate repo (not in riir-ai)
Full text in AGENTS.md. Borrows the drafter+pruner+corpus pattern from `riir-games-quest/quest_grammar` with zero game-domain coupling; default build's only public dep is katgpt-core; opt-in arms consume `../riir-infer` (ternary) + riir-rag (latent_retrieval).

## Documentation Shape
Full text in AGENTS.md: 11-folder numbered `.docs/` book; files inside numbered folders have no number prefix; `README.md` is the build surface.

## Numbering Discipline
Full text in AGENTS.md: monotonic never-reused counters per directory, one counter per directory ENFORCED (Issue 069 T3); katgpt-rs `scripts/numbering_drift_sweep.py` covers this repo. Research-note shape: born-complete records with a `**Status:**` line (RECORD / DISTILLED / NEGATIVE).

## 2026-09-11 — Plan 124: `--config-audit` (post-mining intake classes A+B consumed)
- Class A (default-is-a-named-const-nobody-field-reads) + Class B (assert-only-against-own-Default); probe-verified clippy fires ZERO on the inert shape; riir-chain 1 verified TP / 0 FP. Class C stays an intake row (needs git history).

## 2026-09-12 — Issue 089 T3: `--misses` generalized to every domain + the 09-08 bare-default regression fixed
- Per-domain frontier-ordered authoring queues; en-route: `--misses` had been exit-2 broken since 09-08 (bare default under every standalone mode); the first loader was an undercounting `collect()`; two "clean" clippy `--bins` lanes were INERT (required-features silently skip the target).

## 2026-09-12 — Issue 092 T1: the GW corpus-alignment PoC, measured and closed (defend-wrong NEGATIVE)
- GW precision@60 = base rate 0.017; name/keyword 0.083 = 5× base. Mechanism: surface-trigram distance is not fix-shape structure. Reopen: a semantic fix-shape embedding or a disjoint-vocabulary corpus pair.

## 2026-09-12 — Issue 093 T1: the per-run strategy-posterior PoC, measured WIN (defend-wrong)
- `NonergodicFilter` 41.7% vs 35.2/36.7 (McNemar p<1e-4), misspec-robust, store-independent. Harness lesson: revive-on-posterior-flip fired 356 times, 0 helped (frozen-context retry is the same bytes); revive requires the suppression CONTEXT to have moved.

## 2026-09-12 — Issue 094: trajectory covariability-class mining PoC, measured and closed (defend-wrong NEGATIVE)
- AUC 0.497 = chance; the failure is the STORE SCHEMA's (cumulative counters, last-verdict timestamp — no session axis). Reopen: per-event trajectory rows or balanced per-rule labels.

## 2026-09-12 — Issue 093 T2: the nonergodic driver wiring LANDED (gain-null on no-surface) + the orphan-report Windows blind-leg fix
- Driver `with_observer`/`with_revive` seams + the policy; both arms first-try-perfect on every real scaffold; the family-error test moves to the reopen trigger. Windows fix: canonicalize the git root before pathspec comparison (`\\?\` verbatim form).

## 2026-09-12 — Issue 098: the rule-corpus cross-OOO axis, measured PASS + wired (absorption candidates found)
- TF-IDF cosine (plain TF-cosine REJECTED — generic scaffold swamped rare op signatures); 3 absorption candidates found incl. clippy's own split of one lint detected unprompted (the calibration anchor); `score_history.jsonl` rows gain `cross_ooo`.

## 2026-09-14 — Plan 195 executed (seal-game-editor issue 150): the UNIQUE carrier index, its reconcile twin, and the clash report that opens the file
- Landed on seal-game-editor develop `e25f98a4`: UNIQUE index + `working_schema::reconcile_indexes` + the clash row; five plan amendments recorded (EDITOR_SCHEMA mirror + version bump; `drift_split`; duplicates counted before drop). editor-data 876/876 + 10/10.

## 2026-09-15 — Issue 108 T1+T2: the component-necessity flip-rate harness (CVRR at the dispatch seam)
- `ComponentOverride` + `heal_fixpoint_cfg` (empty = byte-identical, PINNED) + `--necessity-bench`; smoke e2e's first real finding: `needless_return` solo_resolved=1 but Δdisabled=−17 — solo rate massively understates composed importance.

### 2026-09-15 addendum — Issue 108 T3+T4: the full measurement, the schedule amendment, and the honest negative
- T3 ran in ~3 minutes (the "measurement-heavy" deferral evaporated). Amendments: presence-ranked schedule (top-20-by-heal-rate schedules 4 rules and misses exactly where composed necessity lives) + Δ-first verdict ladder (13 of 18 rows would have been mislabeled). Verdict: 18/18 LOAD-BEARING, zero decoratives; Δswap ≈ Δdis everywhere — donors are shape-disjoint.

## 2026-09-15 — Intake class P22: `--platform-audit`, the platform-dead_code decl-vs-use cfg lattice
- Items declared ungated whose every use sits inside a platform-gated context — invisible to any single-platform CI matrix. Validated against katgpt-pruners @ `ea4c2873^` (exactly NEON_U8; 1 TP / 0 FP over 5950 decls). First workspace sweep 14 rows → 3 TP / 11 FP; the method finding: a second-target `cargo check --target x86_64-apple-darwin` (~40 s, never links) is the cheap oracle complement.

### v2, same day — P24 (a)+(c) consumed (the fold + the prune)
- The file-level `#![cfg]` fold seeds each file's root ctx — fixes BOTH blindness directions by construction; 6 reverse-direction rows surfaced and fixed in riir-ai. Walk prune: skip symlinked + `.git`-file children.

### v3, same day — P24 (b)+(d) consumed (the mod-row lattice + the macro arg scan) — P24 COMPLETE
- `ModRowLattice` resolves every external `mod x;` row by rustc's path rules and folds atoms transitively; the format/assert macro-arg scan counts inline captures + ident tokens (overcounting is the safe direction). Re-sweep: 6 target FPs gone; 4 TRUE rows surfaced and fixed in riir-mmorpg-examples; 9 standing rows / 6 documented classes.

### P25, same day — the tracked-only walk for the `read_sources` family (intake P25 consumed)
- `read_sources_walk` derives the file set from `git ls-files -z --cached --others --exclude-standard` (tracked + untracked-NON-IGNORED — a fresh WIP file stays, a tracked-but-later-ignored file stays); every report prints a `walk:` line. riir-train 4119 → 1131 (the 2988-file gitignored corpus out; the 3 masquerade rows GONE); riir-chain 812 → 482. Orphan sweeps: 1 TP fixed in riir-train (`TripleFusionConfig.asft_lambda`), 1 TP fixed in katgpt-rs (`ParallaxConfig.zero_init`), the DumpEntity `Serialize`-wire-contract class recorded NO-ACTION.

## 2026-09-15 — Issue 109: the orphan-report FP specimens repaired (foreign-literal attribution + generic serde bridges)
- (a) per-file foreign-literal attribution with the workspace-sibling rescue (`scan_local_crate_roots`); (b) generic serde bridges harvested at turbofish call sites (stillborn → `bridge-cleared`, orphaned → `wire-dto` weak). Lesson: raw-string terminators `}"#` swallow following tests silently — when tests vanish from `--list`, suspect the terminators first.

## 2026-09-16 — the Issue-109 re-sight closed: scan-global struct field sets (the foreign-glob + local-glob shape)
- The bevy migration re-entered 4 rows through the documented undecidable shape; closed by the compile argument (`literal_is_field_mismatch` — a literal naming a field no local same-named decl has provably cannot be any of them); canaries byte-identical.

## 2026-09-16 — P31: the fn-A/fn-B inter-procedural wire-clearance (orphan-report; the riir-kat queue-writer intake)
- `serialized_concrete_param_types` (a fn serde-serializing its own concrete param contributes that type) + macro token-stream scanning (serde calls inside `writeln!`); riir-kat 16 stillborn → 4; remaining conservative gaps on record (turbofish `to_string::<T>`, method-shaped serializers, Deserialize-side classes).

## 2026-09-18 — Issue 124: the ops-plane ladder re-pointed at the riir-llm leaf
- The leaf owns WIRE MECHANICS (URLs, body builders, parsers, `OaiError` = alias of `riir_llm::LlmFailure`, timeouts, bearer hygiene); riir-clippy keeps POLICY (windowed-disable router, stats, pricing, cap). Behavior changes: free-leg 401/403 is STOP-class (was silently shifting the window onto the PAID leg); empty-content falls through; `groq_first` no longer names `dep:ureq`.

## 2026-09-22 — Issue 129: the cargo-heal scoop-bucket 0.1.4 manifest 404 (URL pointed at the v0.1.3 tag)
- One-line URL tag fix, verified end-to-end (sha256 byte-identical). Prevention: prefer the release.yml bucket-PR lane; when hand-editing, copy BOTH `url` and `hash` from the fresh release and curl once before pushing.

## 2026-09-26 — Issue 130: RRSI admission-generalization gate — closed report-only, both unblock paths measured shut
- T1: repo stratum δ = 0.000 (the oracle axis is deterministic per pinned toolchain — a zero-δ floor admits everything). T2 repeats: refuted BY CONSTRUCTION (no timeout/parallelism in the bench path). T2-OOD: the δ=0.247 window spread is corpus-state drift, NOT repeat noise (within-state repeats bit-identical). Live verdict wired into `--gate-calibration`.

## 2026-09-27 — Issue 136: the structural self-mutation lane landed (reef P1+P3+P5, Plan 175)
- `src/structural_admission.rs` (ungated pure: per-kind shape gates, kind-keyed review classes — 4 of 5 `AlwaysReviewed`, the reef P2 secret-shape scan, the `ChainState` CAS machine) + the gated content-addressed store (`.heal/structural/`; every touch re-verifies the hash; `CasRace` on moved state). NOTHING auto-mutates — activation is a separate owner call.

## 2026-10-06 — Issue 144: the dist README + CLI-help audit — T1–T5 landed; T6 dispositioned on the owner's re-arm posture
- T1 dist README rewrite + CHANGELOG (incl. three corrections beyond the audit's evidence table); T2 repo rename; T3 `--help`/`--help-all` split with internal plan/issue ids banned from the user page; T4 `--domain` choices derived from the compiled feature set; T5 EULA/LICENSE + the private-crate leak fixed at the `about.toml` level. T6 dispositioned NOT done: the testnet alias stays pinned (the recorded re-arm posture wins over a cosmetic help-text win).

## 2026-10-10 — Issue 159 CLOSED: the net-pin conflict posture refuses every run at startup — fixed `2e54d205`, verified
- Found during the kat_ua (riir-kat 003 T15) landing's guard run; pre-existing and reproducible since `492381b2`. Root cause: under `--all-features` both net pins are on, so the `command_service_url()` conflict refusal fired at STARTUP (via the Issue-153 update check's URL resolution) — before any dial, killing every local-only scan an all-features-built binary tried to run, including the guard's own Layer 5c e2e (`auto_oracle_features_cli`). Fix (`2e54d205`): `net_partition_url()` downgrades the both-pins conflict to one advisory line for LOCAL state partitions (LeaseStore open_net + mining outbox read — state partitions under the default net); every DIAL still exits 2 through the strict seam (the pin contract unchanged); `build_stamp_cli`'s 'multi' pin assertion is posture-aware. Ride-along: `run_docker_lane` gained the `#[cfg(feature = "docker")]` its caller already carried (E0433 on docker-less builds). Verified 2026-10-10: `cargo test --all-features --test auto_oracle_features_cli` → 1 passed. Issue file removed per the noise-reduction rule; this entry is the record.

## 2026-10-11 — Issue 089 CLOSED: the frontier-band curriculum delivered report-first; the admission gate stays declined on the record
- Landed 2026-09-11→09-12 (`11c1799d` the self-winding tail): T1 `--frontier-report` (per-rule `r/f/n/p̂/LCB₅` + band-mass; snapshots append to `.benchmarks/frontier_snapshots.jsonl` with the `store` discriminator), T2 saturation candidates (report-only), T3 `--misses` generalized to all domains, T5 provenance weighting + the score-bench provenance strata, T6 split verdict — the D formula DECLINED on the record (data-blocked at mining time; exposure-volume-shaped at heal time), the stratified read LANDED, T7 the gen-8 shaped-verdict ladder, T8's emission half (per-rule `rule_outcomes` on every bench row). T4 (admission wiring) + T8's α grid-fit close ON ACCUMULATION ONLY; both clocks self-wind. Final re-verify 2026-10-11: NO clippy-corpus generation since B191 — batches 213–223 are all rust_perf/kernel_opt/dist landings, so no sweep was owed; the gate was never wired BY DESIGN (the report-first law; R135/Bench 047: at pool sparsity, selection sophistication ≤ 0.04pp — evidence volume is the bottleneck). Reopen (as a FRESH issue) only if the accumulated snapshots show band-mass PREDICTIVE — per-store band-mass at ≥3 distinct generation boundaries correlating with the next generation's in-band yield, sample size named; until then `--frontier-report` is the shipped, sufficient surface. Issue file removed per the noise-reduction rule; this entry is the record (research 167's direct link de-linked in the same commit).

## 2026-10-11 — Issue 139 CLOSED: noise-weighted evidence consumed report-only (T3); T1/T2/T4 terminal-deferred behind the qualified tripwire
- T3 LANDED 2026-10-01 behind `noise_weighted_evidence` (report-only `--frontier-report` censoring decomposition; live store 28 rules / 0 unseen — armed but vacuous on this data). T1/T2/T4 DEFERRED 2026-10-01 as measured VACUOUS (10 of 657 n≥2 groups mixed = 1.5%; prior-epoch σ̂ is 0 for 99.75% of groups); the 2026-10-08 re-measure (20 stores) read the pooled population UNCHANGED, the only letter-fire (riir-chain 3/5 = 60%) a pooling artifact. Owner arbitration 2026-10-08 (verdict session `9537e7f3`, arm A2) QUALIFIED the trigger, carried VERBATIM — reopen T1/T2/T4 ONLY on: "mixed groups ≥ 10% of n ≥ 2 groups on any live store WHERE that store carries ≥ 30 n≥2 groups AND each counted mixed group has n ≥ 3; OR the first compile-lane (E-code) rows plus a real-use miss family." No grandfathering; the trigger also rides the AGENTS.md row. katgpt-rs side unchanged: Issue 913 landed `9ad5e21ce` (Bench 905, GOAT (b) PASS — the readout is a RANKING score, coverage 0.883 at nominal 0.95, never a calibrated interval). Natural grouped consumer of record: `traj_store::shape_conditional_score`, not `select_best_candidate`. Issue file removed per the noise-reduction rule; this entry is the record.

## 2026-10-11 — Issue 140 CLOSED: insight-conditioned retry landed with its measured verdict; T4/T5 have an empty input population
- T1 (distiller `src/insight.rs`), T2 (admission predicate), T3 (`InsightConditioning` at the `with_ordering`+`with_observer` seams; eval lane `--only insight`) LANDED 2026-10-01; verdict in `.benchmarks/104_insight_conditioned_retry_ab.md`: reverts 24→10 (−58%), +0.128 attempts/case LB95>0 vs static AND vs the raw negative control; the CONTROL ARM collapsed (insight == control byte-identically, refused-admission 0) — the deterministic arm IS ordering on this domain. T4/T5 DEFERRED-TERMINAL: the admission filter never refuses on rustc diagnostics — the "admitted insight" population is empty; internalizing the control arm's statistics would duplicate the shipped `(code, strategy)` trajectory→Beta substrate. Re-checked 2026-10-08 at HEAD `977ff1e4651c` on the v2 corpus (3 corpora × 4 arms): trigger did NOT fire, both conjuncts fail (`cd756252`). Reopen ONLY on the CONJUNCTION: a verifier domain refusing admission at a nontrivial rate AND `refused-admission > 0` with a paired insight-vs-control difference; the internalization lane is riir-train plan 432's frozen-generator reader (cross-refs `9cfd0c33` + `b72bb62f`). Issue file removed per the noise-reduction rule; this entry is the record (research 228's pointer survives by number).

## 2026-10-11 — Issue 157 CLOSED: span-conditional contrastive negatives DECLINED-for-now on volume; the tripwire lives in bench 111
- Owner triage RESOLVED 2026-10-10 (verdict session `2b8a1316`, rounds 1–3 AGREE): the retrieval-negative consumer is NOT wanted today — ~12 negative pairs over ONE span (the v4 `rejected` label landed 2026-10-08), kill criterion 1 (volume-vacuous); NOT a mechanism negative — the census found the flip target exists (4 of 15 miss spans have gold at rank 1; 2–5 near-tie at the pre-registered margins; miss-set n = 15). Full record: `.benchmarks/111_issue157_negative_channel_p0.md` (landed `b41e1403`). Standing re-open trigger (in bench 111): ≥ 60 negative pairs across ≥ 10 distinct spans in any live store → certification FIRST (needs the two data-model fixes: pre-fix span text persistence + authored acceptable sets), census re-run, only then the P1 A/B (paired LB95 top-1 delta + zero regressions + per-family retention + pre-registered λ/threshold). The mint consumer untouched; fleet rows out of scope pending a consent decision. Issue file removed per the noise-reduction rule; bench 111 is the record.

## 2026-10-11 — Issue 158 CLOSED: the Matryoshka-k re-test tripwire absorbed into its record (bench 109 + AGENTS.md)
- The file was a tripwire by its own text; none of T-A/T-B/T-C has fired (corpus still at the Bench-109 generation; no k-subvector consumer shipped). The 133 negative's boundary stands: the shipped FIXED objective wins shallow-k everywhere (rig 134 vs ≤ 130); deep-k corpus-limited at 1.16 labels/span; the withheld-width nesting effect is REAL (29.3 → 49.6 @ k=20 vs 92.0% full). Re-test triggers, carried verbatim — ANY ONE fires a re-test: **T-A** a clippy-corpus generation whose mined spans carry mean label-support ≥ 3 labels/span at the deep-k widths (today 1.16); **T-B** the corpus doubling in rules-per-domain such that per-domain shallow-k retrieval measurably saturates; **T-C** a downstream lane consuming k-subvectors at serve time. On fire: re-run the T1 harness (arms + seeds + gates = `.benchmarks/109_mattr_k_t1_negative.md`, harness `007435e5`); G1 + G2-OOD bind as before; **the control is the shipped fixed objective AT THE SAME corpus generation — never the old 134 number across corpus boundaries**; record under a NEW bench number. Issue file removed per the noise-reduction rule; this entry + the AGENTS.md `mattr_k_poc` row are the record.

## 2026-10-14 — Issue 163 CLOSED: the conflict posture keeps its loopback lane (the --all-features hermetic tier green again)
- Verdict AGREE (reviewer session `6bb9b19a`, 2026-10-14): option 1 — reorder the seam in riir-kat — landed `riir-kat 9a36759`: `ensure_net_allowed` checks the loopback/`allow-local` branch BEFORE the empty-overlap refusal (EMPTY check stays ahead of the allowed-set match; parse-rule failures carry the "at most one pin" hint; conflict label now "conflict: net-mainnet+net-devnet (every hosted host refuses; loopback follows allow-local)", the `conflict:` prefix unchanged). Option 2 (pin the suites off `--all-features`) declined — the guard owns that posture and the money-CLI lanes must be exercised there. riir-refine half (this commit): `build_stamp_cli.rs` NET_PIN expectation posture-correct for all four postures; `heal.rs` conflict advisory says HOSTED dials refuse; `net_flag_selects_the_invocation_net` three-armed; `tests/topup_cli.rs` pre-existing `useless_format` (from `2682d7a0`) healed; AGENTS.md `--net` row amended. **STALE-RESULT MARK:** the batch-228 Layer-5c "3920/0" record (2026-10-13, `.distill/001_mining_queue_snapshot.md`) is STALE from `492381b2` (2026-10-09) — a fresh `cargo test --all-features --tests` on any post-pin tree red's deterministically on the three hermetic suites; the number stands only for the pre-pin tree. Validation: kat_net_pin 7/7 at five postures; riir-kat 294/294 at four; clippy `-D warnings` clean; riir-refine `--all-features` topup_cli 12/12 (was 8/12), kat_adopt_state_cli 6/6, kat_burn_stack 7/7 (was 0/7), build_stamp_cli 5/5; required-features posture 30/30. Issue file removed per the noise-reduction rule; this entry + the AGENTS.md `--net` row + `riir-kat/src/kat_net_pin.rs`'s module doc are the record.

## Lessons
- A fresh worktree + empty target dir is the only clippy reading that means "0" — a stale-cache close-out claimed 0 and 34 findings were live.
- `git fetch origin` BEFORE any sizing/dry-run on a repo not touched this session — the reflog can be 8 commits stale and the residue phantom.
- A fixture feeding BOTH the real-clippy oracle AND labeled eval spans must be verified against both consumers in the same landing (disjoint feature sets; neither sees the other's red).
- A contract change must re-pin its e2e the same day, not just its unit goldens (Issues 081/087).
- "0 passed" on a default run is the only reliable gate detector; greps on attribute forms lie.
- Feature-gated heals MUST pass `--verify-args "--features F"` — a default-features check compiles gated files EMPTY and proves nothing.
- `--fix --verify` without `--write` is NOT a dry run — it writes edits unconditionally.
- Residue scans must filter by warning TEXT, not `--> path` alone.
- Multi-line rewrites evade same-line grep scans — the per-gate compile check is the only trustworthy detector.
- Probe-first: the probe corrects the intake's own description (P10/P12/P13 pattern) — never land on an unprobed premise.
- Never put a stash call in a check script; /tmp is unreliable on the Windows box and background nohup jobs die with shell teardown.
- Raw-string terminators (`}"#`) and let-chains/edition gates are load-bearing — re-read file state after edits near them.
- Line-initial `+ `/`- ` in doc comments parses as markdown list items; `ensure_ascii=False` is non-optional in Python corpus generators; `.goat.rs` files map to underscored cargo target names via the `[[test]]` name field.
- Unpinned binaries need PRE-heal baseline runs; a mismatch on a perf-gated test needs a re-run before it counts as a regression (load/thermal class).
- Cargo's `Running benches/...-<hash>` filename is crate+features identity, NOT source content; criterion noise annotations differ between ANY two runs — verdict lines are the load-bearing comparison.
- A static manifest read is not the tool's verdict; a committed lockfile is necessary but not sufficient evidence (docker `cargo-locked-flag` falsification).
- Text is the only interface for `docker build --check`; a resolve failure silently TRUNCATES the finding set while exiting 1.
- Sharing a scratch target dir with a fixed package name returns cached answers — one cargo unit per name (Issue 110); the more carefully somebody isolates, the more reliably the oracles lied.
- `cargo tree --bench` is a false-negative probe; `cargo build --benches --features F` silently skips unsatisfied targets — cross-check built-vs-expected.
- Archive consumed frontier rows or the family-done guard deadlocks (Batch 159); the harness's own G1 fixture can refute a draft fix-pattern (Batch 109).
- cfg-promotion: parse-time discarded ≠ cfg-time discarded — a mid-block `#[cfg]` statement recurses only when the enclosing tail is already discarded.
- Deterministic rows must not carry wall-clock fields; `elo_table()` reads PERSISTED ratings (explicit `recompute_elo()` first).
- An uninvoked assertion is unknown, not passing (the Bench 094 dead test); a freeze label fence cannot see a scoring-semantics defect — the first live bench run is itself a gate.
- BARE TOP-LEVEL `let` IS NOT A RUST FILE (fn-wrap fixtures); command-line-requested lints render HYPHENATED in human output — grep JSON.
- A "remaining queued" line that outlives its batch breeds stale re-mining — disposition or delete at landing time.
- extract: per-record landing + hash + verdict discipline; this section is the one-liner index for the long lesson paragraphs the compact entries replaced.
