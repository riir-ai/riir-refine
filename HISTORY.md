## HISTORY.md — riir-refine

Compacted 2026-10-30: every `##` heading kept verbatim, one compact entry per record (≤2 short lines); the full pre-compaction text is in git history (pre-compaction HEAD 403d72bbb703; the 2026-10-16 pass: cf0f52dbb180). Operational rules live in `AGENTS.md`; removed issue files: git history.

## 2026-10-30 — AGENTS.md compaction: the Role-bullet per-batch rule arcs (Batches 282–319) moved out of AGENTS.md

rust_perf: B282 Plan 286 · B283 Plan 287 · B284 Plan 288 · B288 Plan 293 · B298+B305 Plan 289 (entries_52) · B308 Plan 313 (entries_54) · B316 Plan 316 (entries_55) · B318 Plan 318 (entries_56) → 287 rules. kernel_opt: B283 Plan 287 · B284 Plan 288 · B286 Plan 291 · B288 Plan 293 · B295 Plan 300 · B296 Plan 301 (gpu_132) · B297 Plan 303 (gpu_133) · B299 Plan 304 (gpu_134) · B302 Plan 307 (gpu_135) · B303 Plan 308 (gpu_136) · B304 Plan 309 (gpu_137+138) · B307 Plan 312 (gpu_139) · B309 Plan 314 (gpu_140) · B310 Plan 315 (gpu_141) · B317 Plan 317 (gpu_142) · B319 Plan 319 (gpu_143) → 1033 rules.
Rule-by-rule arcs: the batch plans + the pre-compaction AGENTS.md at `403d72bbb703`.

## 2026-10-30 — Issue 189 tails DONE: the wire-KAT census render + the E-5 live refusal + plan-302 T1 ticked (file removed per the noise-reduction rule — plan 302 + this row are the durable record; T3b + promote stay owner-parked in plan 302)

Three tails of the Issue-175/plan-302 T-E2E battery, landed `1efc918d` (session zed-m3-189-tails): (1) the two census render sites (`kat_migrate_client.rs` …)

## 2026-10-28 — Issue 186 CLOSED: structure-grounded synthetic retrieval-eval fixtures land as `draft::eval_synth` (file removed per the noise-reduction rule — Research 339 is the durable record)

The STAIR recipe (arXiv:2609.03874 via ndb Research 311) landed tier-1: gold derives from the corpus's own structure (every span from rule R's `buggy_code` carries gold = R, zero teacher opinion), questions are three deterministic mutations (`ident-rename` / `comment-strip` / `reindent`, identity-skipped).

## 2026-10-27 — Issue 185 CLOSED: the `softmax-collapse` rule family ships with a zero-unexplained-hit workspace sweep (file removed per the noise-reduction rule — this row is the durable record)

T1–T4 (`17965377`): `sigmoid-minus-one` (MECHANICAL, `FIXABLE_SEC`: `σ(z) − 1` → `−σ(−z)`, split one-hot form, name-allowlisted σ), `softmax-onehot-collapse` + `neg-ln-softmax-prob` (suggest-only, row evidence in walk order per innermost fn), the new APPEND-ONLY `SecFamily::Numerics`, before/after fixtures from six …
Commits: `ad394fc6`, `38b62684e`, `11a00d6`, `b07f40af`.

## 2026-10-27 — Issue 181 CLOSED: the gitsync `:selfheal` proved itself on the first real wedge — close condition met in the scheduled 12:15:33+07 run (file removed per the noise-reduction rule — this row is the durable record)

The wedge (sync-repos.log, the 2026-10-27 12:15:33+07 run): riir-infer's sync died with "Your local changes to the following files would be overwritten by merge: .issues/1134_norm_pair_epilogue_fold_rung.md" …

## 2026-10-24 — Issue 169 CLOSED: FLINT multi-vector T0 kill-test measured KILL — full-token MaxSim ≤ the shipped matcher at EVERY width (file removed per the noise-reduction rule — this row is the durable record)

The pre-registered kill-test ran FIRST (T0 before any pooling code): hashed `op_tokenizer` op-token embeddings (BLAKE3-XOF → D u32 → f32 [-1,1] → L2, scheme pin `3466d6ed36f96a0c`, document-side dedup EXACT under MaxSim) scored by the shipped `maxsim_score`, vs the shipped `LatentMatcher`, ONE binary at default …
Commits: `794771c3`, `9a7399c6`.

## 2026-10-19 — Issue 168 CLOSED: the "5 stale lint keys" were NOT renamed/removed — all five are Batch-230 AHEAD-OF-PIN; the coverage report partitions them by the registry (file removed per the noise-reduction rule — this row is the durable record)

The issue's premise refuted by probe at both drivers (2026-10-19, `clippy-driver -W help`, checks sections only — the `-W help` lint names are kebab-case, the underscore greps silently read zero): at the pinned 1.98.1 none of the five names exist …
Commits: `dfd4dd11`.

## 2026-10-18 — Issue 166 record CORRECTED: the T1 inventory's `evolve.rs` row was false — `score_candidate` has a production weighted-sum consumer; the capacity-law ABSENT verdict survives on corrected grounds

Found by the closure-verification pass (a read-only re-audit of `cdb53f81`/`5012782d` against the code at `origin/develop`). The T1 row "`evolve.rs` `score_candidate` has zero production consumers (test-only API)" and the verdict sentence "no serve-path weighted-sum/absolute-threshold consumer of any scored sigmoid …

## 2026-10-18 — Issue 166 CLOSED: the capacity-law lever class is ABSENT in the healer (file removed per the noise-reduction rule — this row is the durable record)

T1 inventory OVERTURNED two of the issue's own table rows: (1) `traj_store` `sigmoid_dot_8` is NOT pure ranking — its one consumer fuses it into a weighted sum (`sort_by(bm25/max + latent)` before the top-k truncate), and the serving consumer (`cf_workers_ai::growing_exemplars`) re-sorts the fused pool by (tier …)
Commits: `cdb53f81`, `78d20871`, `272995d4`.

## 2026-10-17 — Issue 166 CLOSED: the `tuna` verb names the account state, not the deployment, on the absent pair (file removed per the noise-reduction rule — this row is the durable record)

The `/history` wire omits the zipped `tuna_balance_micro` pair when the account holds no SPENDABLE trial credit — a consumed free tier (the KAT-grandfathered fleet account `c79c…5345` on mainnet) and a genuinely pre-TUNA worker read identically, and the None-arm blamed the deployment ("pre-TUNA deployment") for the …
Commits: `9582bf8`.

## 2026-10-16 — Issue 164 CLOSED: the install-probe plain-copy hazard — installers now symlink-aware (file removed per the noise-reduction rule — this row is the durable record)

Hazard: `install.sh`/`install.ps1` (gist-rs/cargo-refine) replaced a developer's `~/.cargo/bin/cargo-refine` symlink with a plain release-cut copy; later local rebuilds propagated nothing (measured both boxes 2026-10-13).
Commits: `26e935e5`, `59bbcd0f`, `d9a75b3`.

## 2026-10-12 — Issue 160 CLOSED: CRLF line-ending contamination — all 7 legs normalized, workspace doc-surface census reads 0/7 (file removed per the noise-reduction rule — this row is the durable record)

Six legs landed 2026-10-11 (refine `7fbca3a7b` 82 · reflex `9e517d2` 2 · reflexer `68cdaae` 1 · ai `ff56c5e113` 26 · katgpt-rs `f2a4fd5a7` 23 · infer `e7b0e49` 6 …)
Commits: `afc0a249`.

## 2026-10-12 — Issue 162 CLOSED: formatting-invariant dense retrieval — T2 measured, T3 decided NOT ARMED (no transferable formatting subspace) (file removed per the noise-reduction rule — this row is the durable record)

T1 `cea4d679` (cycle-22): frozen pair dataset `assets/format_pairs_v1.jsonl` — 4,213 pairs / 37 families / 1.48 MB / sha256 `85dea25b…46e79e` (mined via the existing `--dump-trajectories` export; uninlined_format_args 72% dominance) …
Commits: `e6c02dae`.

## 2026-10-12 — Issue 161 CLOSED: module-applicable pool candidates — pool birth point 3 (the proposer member) + the `rewrite` merge strategy + the behav driver repairs; T1.6 re-computed PASS (file removed per the noise-reduction rule — this row is the durable record)

Landed `1afd5d15`: pool widened with proposer members (POOL-ONLY, `heal_with_pool.result` bit-identical to `heal` — the R6 seam contract; plain `heal` pays one branch, G4 untouched) + the `rewrite` whole-module merge (a file-preserving response merges WHOLE) + the behavioural driver's float/int-slice extension …

## 2026-10-11 — the 089–158 close-out audit: one missing record BACKFILLED (Issue 112), two index gaps named (137 → bench 103; 142 a legal burned number) — file:record agreement now total over every removed 089–158 file

Issue 112 BACKFILLED (false-claim repair, the Issue-798 class): removal `fcb4a15b` (2026-09-16) claimed a HISTORY row that was never written; 112 = the probe-identity WITNESS — primary records are its removal message + the 235-line `.issues/112_probe_identity_witness.md` in git history (this row is the index) …
Commits: `8e67ee16`, `2839c361`, `d25bb830`, `941e049e`.

## 2026-10-09 — Issue 156 CLOSED: the FlyBy state-level rescue mining filter (b0=0 ∧ bd≥3) — T1–T6 all landed, yield honest-EMPTY locally, live-fire owner-gated (file removed per the noise-reduction rule — this row is the durable record)

Code half 2026-10-08 `cf34f6c8` (T1/T2/T3/T5): ungated `src/rescue_mine.rs` (`rescue_keep`, `RescueTally`, BLAKE3 `keep_set_hash`) + arm-H per-state `b0`/`bd` tallies (`FLYBY_CONTINUATIONS_PER_ARM=4` documents the paper denominator) + five serde-default `TrajRowDoc` rescue fields (wire-safe) + the `--misses` …
Commits: `86fe38a`.

## 2026-10-08 — Issue 150 RESOLVED: the Homebrew cargo-refine shadow — option 3 (build-commit stamp) landed, option 2 (brew unlink, verdict-REVISEd to reversible) applied, the standing check fired twice in one day (file removed per the noise-reduction rule — this row is the durable record)

Found 2026-10-06 (dapps plan-045 verdict round 2, M3): the tap Cellar 0.2.1 binary shadowed `~/.cargo/bin` on PATH — bare `cargo refine` silently ran the RELEASED build.
Commits: `0a4c010c`, `3a9d1e96`, `6ee548ca`, `32d0ac34`.

## 2026-10-08 — Issue 133 CLOSED-NEGATIVE: Matryoshka-k supervision for `rule_embed` — the shipped fixed objective wins at every measurable depth (file removed per the noise-reduction rule — this row is the durable record)

Reopen fired on the count bar 2026-10-08 (1095 in-vocab span↔rule pairs = 4.1× the Bench 099 bar, tier 800 = 3.0×; doc_markdown 47% of pairs); Claude arbitration `9537e7f3` took B1 with three binding conditions. T1 randomized-k matrix (five arms × k ∈ {1,3,5,10,20} × both BLAKE3 splits): NEGATIVE (bench 109) …
Commits: `007435e5`, `3ca8ac4d`.

## 2026-10-08 — Issue 148 CLOSED: the traj-mining lane named + labeled + deployed — T-a/T-b/T-c all landed, no healer-side work remains (file removed per the noise-reduction rule — this row is the durable record)

T-a the training-source designation paragraph in `src/kat_sync_payload.rs` (landed 2026-10-05; katgpt-rs joined the lane same day via the T-c lever-1 verdict: 385 rows / 353 new).
Commits: `34d32063`.

## 2026-10-08 — Issue 153 CLOSED: CLI version alert + binary self-update + the stale/new corpus surface — T1–T5 all LANDED, zero open code items (file removed per the noise-reduction rule — this row is the durable record)

Owner ask 2026-10-08, verdict GO/GO/GO/GO + one deferred. T1 the alert (`src/update_check.rs`, `RIIR_REFINE_UPDATE_CHECK=0` opts out, `CI` skips); T2 `cargo refine upgrade` (verified download → SHA256SUMS → backup-swap, refuses dev builds + unknown triples; `RIIR_REFINE_AUTO_UPGRADE=1` opt-in) …
Commits: `bdc2bad0`, `fcd3f2db`.

## 2026-10-08 — Issue 155 FIXED: the 154 fix commit never backticked the allowlist cell — prose-green residue caught by the 172nd scoped boundary run (file removed per the noise-reduction rule — this row is the durable record)

`6135b0f3`'s message claimed "backtick the cell; C3 green" but its diff touched ONLY the drift-ledger prose — the `May depend on` row kept its bare column-1 cell, so `allowlist()` (family-regex or backticked tokens only) still read `reflexer-vessel` as UNDECLARED …

## 2026-10-08 — Issue 154 FIXED: the reflexer-vessel BOUNDARY.md allowlist cell backticked — the 170th boundary run's C3 shape finding (file removed per the noise-reduction rule — this row is the durable record)

The Plan-202-R4 row spelled column 1 bare (`| reflexer-vessel |`), and `ci_boundary_contract.sh`'s `allowlist()` recognises only family-regex names or BACKTICKED cells — the committed edge read as an undeclared C3 violation, workspace gate exit 1.
Commits: `7fdd7b08`.

## 2026-10-08 — Issue 152 FIXED: sync_export_cli green again — the seeded rows carry their src_path now, and the fail-closed law is pinned at the process boundary (file removed per the noise-reduction rule — this row is the durable record)

Root cause (as filed): `21f76593` (2026-10-03, riir-dapps 119 T2 — empty-path fail-closed) landed after the test file; the suite seeded legacy rows via `TrajStore::record` (no `src_path`), so the DEFAULT policy's non-empty `allow_paths` blocked them at `gate_row` → 2/4 red since then (pre-existing …)
Commits: `1759fd41`.

## 2026-10-08 — the RIIR_HEAL_* compat shim RETIRED + the live .heal path residue closed (owner call: "heal is retire for refine branding")

The owner adjudicated plan 192 P3.1's legacy-name honoring lazy: `install_env_rename_compat()` deleted from lib.rs + all three bin mains (`5aa91f82`) — legacy `RIIR_HEAL_*` no longer honored anywhere in this repo; AGENTS.md kill-switch rows → the `RIIR_REFINE_*` spellings the code reads …
Commits: `f71bf4a`, `21f76593`.

## 2026-10-08 — Issue 102 CLOSED: corpus lifecycle management (Library Drift adoption) — T1–T5 all LANDED, zero open items (file removed per the noise-reduction rule — this row is the durable record)

Landed: Research 167 `1cb35cce` (2026-09-16) · T1 the `--frontier-report` fade section `56f0b0fd` (report-first, live REFUSAL under the population floor …)
Commits: `d0457204`, `8e0d52f6`, `d9d69cfc`, `4233f7b9`.

## 2026-10-08 — Issue 148 T-b follow-up LANDED: the rustc_errors driver-lane strategy-menu capture (the second lane on the v4 label path)

Capture (`34d32063`): the `--fix-compile` driver computes the strategy menu at each apply decision — chosen-site candidate strategy ids MINUS the chosen one, sorted + deduped, capped at `REJECTED_CAP` …

## 2026-10-07 — Issue 148 T-b: payload v4 — the rejected-candidate label (the suite-mint training source rides the traj lane)

Capture: ONE `matcher.suggest_file` per edited file at `--fix --write`; each edit joins its OWN-PASS line (chunk-neighborhood, exact at pass 0) and carries span rules minus the applied rule (cap 16) into `record_applied` + the post-verify `record_traj`/`record_traj_fail` via `EditAtLine.rejected` …

## 2026-10-07 — Issue 151 CLOSED: the secret-blind surface shipped end to end — scan/warn/blind live, marketing card deployed, the --blind red-window closed (file removed per the noise-reduction rule — this row is the durable record)

Surface (this repo): `56d73162` (`src/secret_blind.rs` — `--secret-scan` report with blinded contexts + the quiet `[sec]` in-run warn + `--secret-scan --blind` shareable text; fingerprint never on stdout) · `675b2acf` (record) · `3d2c6208` (reef fixture) · `fd414fc9` (docs).
Commits: `eed88c6`, `e8ab39f`, `47057d3`, `4b4125a`.

## 2026-10-07 — the 9cfd0c33 corpus expansion's red window closed: the ncswap fixture E0422 + the gen4 re-freeze + the all-features lane unblocked

Three compounding pre-existing reds behind the recorded riir-infer-gpu E0308 (which blocked every all-features lane): (1) the sibling E0308 (riir-infer `d09d1ec`, the metal upload arm's `attn_wo` missed the `q4_refuse` wrapper); (2) ncswap ground-truth `Tkn` unqualified from the child `mod ground_truth` …
Commits: `fac3dc36`.

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

Root cause: the 3 applying arms ran `--fix` without `--no-verify` — red only under `--all-features` where `fix_verify` compiles in; fixed with `--no-verify`. Plus: `heal.rs` unreachable-`return` at the mid posture moved inside the `kat_transport` block …

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

110: all three scratch-crate oracles BLAKE3-key the directory but wrote a FIXED package name — under a shared `CARGO_TARGET_DIR` two crates are one cargo unit and the second gets the first's cached answer (the more carefully somebody isolates, the more reliably the oracles lied) …

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

Dev-sweep network discipline: `RIIR_HEAL_KAT_METER=0 RIIR_HEAL_KAT_GATE=0 RIIR_HEAL_KAT_SERVICE_URL=http://127.0.0.1:9` neutralizes the lease context. Screen: 4 CLEAN, seal-game-editor healed (`e2112bce`, 61→49), 4 blocked with named re-arm conditions, riir-shader recorded-not-touched.

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

Multi-domain modelless code healer behind the `HealDomain` trait (Plan 001) — developer tooling, NOT game/training/chain. L0 rule distillation → L1 ternary SIMD drafter → L2 syn validator (+ opt-in domain oracles) → L3 Pareto search (opt-in) → L4 LoRA seam (inert unless installed; NOT promoted …)
Commits: `e7b1f89`.

## CF ops surface + the `.env` contract — full record (was AGENTS.md lines 3044–3177; Issues 044/045/051/062/065 resolutions + dashboard-auth state + deploy hazard)

## CF ops surface + the `.env` contract (Issues 044/045 — read before touching ops)

LLM ops plane: `riir-kat-housekeeping` worker, `riir-ops-dashboard` worker, nightly generator (`scripts/ops_nightly.sh`); credentials in `.env` (gitignored; pipe into `wrangler secret put` via stdin, never echo). Issue 051 T8 defer preserved: `fixer_regate` arms stay CF-pinned BY DESIGN (model comparisons).

## The `riir-clippy` auto-continue skill — full description (was AGENTS.md lines 3178–3212)

## The `riir-clippy` skill (auto-continue idle loop)

`.agents/skills/riir-clippy/SKILL.md` (canonical home THIS repo, global is a symlink) encodes the idle-time workflow — 13 queue items (fix issues → finish plans → clippy sweeps → resolved-issue hygiene → batch mining + dry-run → boundary check → substrate-first → benchmark sweep → online unblock → defer-marking → …)

## Production adoption — full record (was AGENTS.md lines 3213–3525; verdict evidence + resolved issues 037/038/050/059/060/061/066/067/068/069/070/071)

## Production adoption — `cargo heal` as the dev-time default (2026-08-24)

Verdict GO (live trial): 12/12 planted fixes landed, 2/2 divergence classes correctly refused, compile gate 2 checks / 0 reverts, idempotent. Global binary + post-mining intake (mining improvement candidates from the healer's OWN usage artifacts; seed inventory 8 ranked classes …)

## Guard-landing protocol — full record incl. the measured story (was AGENTS.md lines 3526–3583)

## Guard-landing protocol — a guard is not landed until the residue is counted

Measured, not hypothetical: the `227637e` guards were correct yet 32 pre-guard residue sites landed the next day. The screen is the claim: `cargo clippy --workspace --all-features --all-targets --keep-going` (not `check` — the guarded classes are accepted by check's typeck and rejected by clippy's; not default features …)

## Plan 001 multi-domain architecture — full record (was AGENTS.md lines 3639–3665)

Plan 001 generalized the clippy-only healer into the multi-domain `HealDomain` framework; Proposal 012's GPU kernel optimizer absorbed as `kernel_opt` (user correction overrode the "new repo" recommendation); cross-domain `--domain auto` fans out with the router verdict as a per-span ANNOTATION, never a gate (the 8-D centroids mis-routed fixtures).

## `kernel_opt`-specific layers — full record (was AGENTS.md lines 3666–3685)

1. `KernelExpertRouter` wraps katgpt-core `pick_domain<15, 8>` (8-dim activity vector, 15 per-category centroids, 15/15 reachable; `build_activity()` the consumer constructor; modelless argmax, zero alloc).
2. `KernelThermalRouter` — Plasma→Hot→Warm→Cold→Freeze (`ThermalTier` `#[repr(u8)]`); the scaffold returns Plasma (modelless-first); Warm/Cold/Freeze require 4090 CUDA + LoRA (deferred to riir-train).

## Feature Flag Discipline — full record (was AGENTS.md lines 3760–3809; per-feature batch narratives + corpus-growth history)

## Feature Flag Discipline

`default` = `["clippy_lints"]`. Per-feature records: rust_perf promotion (Bench 013/015/016/017/020 GOAT; 37/40 benchable rules Pareto-dominant incl. `regex-hoist-lazylock` 5443–5675× …)
Commits: `4bfe3b30`.

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

`ComponentOverride` + `heal_fixpoint_cfg` (empty = byte-identical, PINNED) + `--necessity-bench`; smoke e2e's first real finding: `needless_return` solo_resolved=1 but Δdisabled=−17 — solo rate massively understates composed importance.

## 2026-09-15 — Intake class P22: `--platform-audit`, the platform-dead_code decl-vs-use cfg lattice

Items declared ungated whose every use sits inside a platform-gated context — invisible to any single-platform CI matrix. Validated against katgpt-pruners @ `ea4c2873^` (exactly NEON_U8; 1 TP / 0 FP over 5950 decls). First workspace sweep 14 rows → 3 TP / 11 FP …

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

Found during the kat_ua (riir-kat 003 T15) landing's guard run; pre-existing and reproducible since `492381b2`. Root cause: under `--all-features` both net pins are on, so the `command_service_url()` conflict refusal fired at STARTUP (via the Issue-153 update check's URL resolution) …

## 2026-10-11 — Issue 089 CLOSED: the frontier-band curriculum delivered report-first; the admission gate stays declined on the record

Landed 2026-09-11→09-12 (`11c1799d` the self-winding tail): T1 `--frontier-report` (per-rule `r/f/n/p̂/LCB₅` + band-mass; snapshots append to `.benchmarks/frontier_snapshots.jsonl` with the `store` discriminator), T2 saturation candidates (report-only), T3 `--misses` generalized to all domains, T5 provenance weighting …

## 2026-10-11 — Issue 139 CLOSED: noise-weighted evidence consumed report-only (T3); T1/T2/T4 terminal-deferred behind the qualified tripwire

T3 LANDED 2026-10-01 behind `noise_weighted_evidence` (report-only `--frontier-report` censoring decomposition; live store 28 rules / 0 unseen — armed but vacuous on this data). T1/T2/T4 DEFERRED 2026-10-01 as measured VACUOUS (10 of 657 n≥2 groups mixed = 1.5%; prior-epoch σ̂ is 0 for 99.75% of groups) …
Commits: `9537e7f3`, `9ad5e21ce`.

## 2026-10-11 — Issue 140 CLOSED: insight-conditioned retry landed with its measured verdict; T4/T5 have an empty input population

T1 (distiller `src/insight.rs`), T2 (admission predicate), T3 (`InsightConditioning` at the `with_ordering`+`with_observer` seams; eval lane `--only insight`) LANDED 2026-10-01 …
Commits: `977ff1e4651c`, `cd756252`, `9cfd0c33`, `b72bb62f`.

## 2026-10-11 — Issue 157 CLOSED: span-conditional contrastive negatives DECLINED-for-now on volume; the tripwire lives in bench 111

Owner triage RESOLVED 2026-10-10 (verdict session `2b8a1316`, rounds 1–3 AGREE): the retrieval-negative consumer is NOT wanted today — ~12 negative pairs over ONE span (the v4 `rejected` label landed 2026-10-08), kill criterion 1 (volume-vacuous); NOT a mechanism negative …
Commits: `b41e1403`.

## 2026-10-11 — Issue 158 CLOSED: the Matryoshka-k re-test tripwire absorbed into its record (bench 109 + AGENTS.md)

The file was a tripwire by its own text; none of T-A/T-B/T-C has fired (corpus still at the Bench-109 generation; no k-subvector consumer shipped). The 133 negative's boundary stands: the shipped FIXED objective wins shallow-k everywhere (rig 134 vs ≤ 130); deep-k corpus-limited at 1.16 labels/span …
Commits: `007435e5`.

## 2026-10-14 — Issue 163 CLOSED: the conflict posture keeps its loopback lane (the --all-features hermetic tier green again)

Verdict AGREE (reviewer session `6bb9b19a`, 2026-10-14): option 1 — reorder the seam in riir-kat — landed `riir-kat 9a36759`: `ensure_net_allowed` checks the loopback/`allow-local` branch BEFORE the empty-overlap refusal (EMPTY check stays ahead of the allowed-set match …)
Commits: `2682d7a0`, `492381b2`.

## 2026-10-24 — Issue 170 CLOSED: the retrieval gate was RED at every latent_retrieval posture (router 60.3% < 72.3%) and no standing lane could see it

Found by Issue 169 T0's paired-baseline cross-check (Bench 120), verified pre-existing at plain `latent_retrieval`. **T1 attributed, NOT a defect:** growth competition at the SHARED 128-slot GlobalFreq vocab …
Commits: `4c02b8f8`, `80fb8920`, `013d5c04`, `746421f5`.

## 2026-10-26 — Issue 171 CLOSED: T0b package-scoped re-checks promoted (the RA pre-filter deferred on measurement, verdict AGREE)

Landed `1f77c585`: `fix_verify` implies `scoped_recheck` (GOAT PASS) — every verify round re-checks only packages the round's writes/reverts (own or dependency) actually touched; clean untouched packages skip the subprocess spawn + JSON parse. Kill-switch `RIIR_REFINE_SCOPED_RECHECK=0` …

## 2026-10-26 — Issue 172 CLOSED: all 16 pre-existing all-features failures accounted (14 fixed, one filed as 173, one closed via riir-dapps 138)

T0 verdict: `ci_feature_guard.sh` Layer 5c runs every test target at `--all-features`, so the 16 reds were a live red CI lane, not a dormant configuration — all 16 re-verified pre-existing at `8d731fd5^` (comm of FAILED sets empty both ways).
Commits: `fd4be603`, `e067e1b`, `6f52f25f`, `4085aa24`.

## 2026-10-26 — Issue 173 CLOSED: `latent_suggest_goat` G2 latency regression fixed at the source (1587 → ~680 µs/chunk), bar made posture-aware

T1 profile (macOS `sample`, line-tables release): the per-query hot path was riir-neuron-db's `Bm25Index::search` score accumulator, not riir-refine — std SipHash on a `HashMap<&[u8], f32>` outweighed the search body.
Commits: `ef8599a`, `12b1af4`.

## 2026-10-26 — Issue 174 CLOSED: CLI tests no longer adopt the box's real `~/.ssh/id_ed25519`; Issue 172's out-of-lane rows green

riir-auth `account_key` adopts `$HOME/.ssh/id_ed25519` IN PLACE as the account key when one exists, so every CLI test that ran `login` with only the account dirs isolated signed with the developer's real SSH identity on a box that had one and a minted key elsewhere — two code paths chosen by the box.
Commits: `7f076057`, `b543c256`, `3b89cba1`.

## 2026-10-27 — Issue 179 CLOSED: kat_account adopts the provider-identity substrate — all six items landed the same day

Landed `95bc8c18` (amo-auth-followup): `kat account` + `--info` render the
Commits: `eae0f05`, `1258eef`, `c130df9c`, `f22a5ec5`.

## 2026-10-27 — INCIDENT: a bare `git commit` in the shared worktree swept a sibling's staged work under the wrong message (`7f965bc3`)

Folded from `.issues/180_incident_commit_sweep.md` (removed per the noise

## 2026-10-27 — INCIDENT: a Protocol-H worktree sweep removed a staging worktree holding an unlanded commit; the misfired follow-up command then wiped a sibling's WIP in the shared tree

Three-fault chain, all measured in the reflogs (session: zed-amo fmt-census
Commits: `a2389e6`, `3a5acfc`, `d58afb51`, `c9923d2f`.

## 2026-10-28 — Issue 187 CLOSED: the migration proposal issue is FILER-authored; no GitHub write credential exists anywhere in the worker

Folded from `.issues/187_migration_proposal_filer_authored_no_worker_write_token.md`
Commits: `c12d9cb`, `0b143d5`, `7dec777c`, `9d35593`.

## 2026-10-28 — Issues 183 + 184 CLOSED: the migrate release law landed (owner-👍 authorization + rule-C weighted voting + the autonomous cron closer), rehearsed live on devnet; the live half superseded by the filer-authored flow

Folded from `.issues/183_migration_release_requires_owner_endorsement.md` +
Commits: `a7a1c13`, `86a1622`, `e968b3e`, `921d2739`.

## Lessons

A fresh worktree + empty target dir is the only clippy reading that means "0" — a stale-cache close-out claimed 0 and 34 findings were live.

## 2026-10-24 — AGENTS.md compaction: per-batch corpus histories moved verbatim

The batch-by-batch narratives below (rust_perf + kernel_opt Role bullets, their Feature-Flag
Commits: `fa63d97`, `88d0e57`, `81ae003`, `4d17995`.
