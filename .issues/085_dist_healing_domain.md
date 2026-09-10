# Issue 085 — `dist` healing domain (Cargo.toml build-config advice)

**Status:** Phase 1 LANDED 2026-09-09; promotion evidence COMPLETE 2026-09-10 (score-bench dist-ON #92 FLAT + condition-(4) consumer scans + `dist_gates` 7/7 green — Bench 082) — **B1 + B2 LANDED 2026-09-10 (Bench 082 §6: scan table re-run, all scanner conditions green, `dist_gates` 13/13)**; default-on promotion stays OWNER-DECIDED — remaining gate: owner pull evidence ONLY.
**UPDATE 2026-09-10 (idle session, zero-contention diagnosis):** both build blockers named in the T-list are REPAIRED at HEAD — (1) the E0631 `v5_is_removal_rule` signature mismatch was `ce74bac`'s healer `redundant_closure` point-free rewrite of the load-bearing `&String→&str` coercion, compiled to nothing at default features (`#[cfg(feature = "score_bench")]`), restored to the closure form by Batch 140 `d2d2388`; (2) the Batch 138/139 kernel_verify `.parses` all-features conflict was fixed by `91493a1` (cfg-adaptive `verdict_passes` accessor in `corpus/tests.rs`). Batch 140 also RAN score-bench #87 FLAT 96.2/1.9 at the canonical state. Remaining owed for promotion: the score-bench run with `dist` ON (the corpus/routing twin — #87 was the dist-off canonical state) + condition (4)'s real-consumer-repo scan with a true-positive cite. Both are cargo-gated (the plan391 v9 quiet gate, riir-ai `.issues/876`); routed to ride the Batch 122 (Plan 122 T4) cargo window, which already owes score-bench + corpus-stats.
**UPDATE 2026-09-10 (promotion evidence session — Bench 082):** both owed items RAN at zero contention, isolated `CARGO_TARGET_DIR=/tmp/plan085`, binary built `--features score_bench,dist,kat_service` (the bin's required-features row needs `kat_service`; `score_bench` implies the other three). (1) Score-bench **dist-ON run #92: FLAT — byte-identical to the dist-off tail** (heal 96.2 / created 1.9 / parse 100 / decline 100 / edits 64 / vocab 38; OOD axis bit-identical, artifact `faa021c8…`). `FIXABLE_DIST = []` keeps the proposer vocabulary and the OOD statistic unmoved by construction — the FLAT is mechanical, not lucky. Condition (2) PASS. (2) Condition-(4) scans (actual verdicts, `--suggest --domain dist`, zero writes): **true positives confirmed on `riir-game-sdk/crates/riir-asset-converter` (3 cites, 2 explicit bins) and `riir-deployer/crates/riir-deployer` (3 cites, 6 bins)**; negative controls HELD on `katgpt-rs`, `riir-viewbridge`, this repo (own `[profile.dist]` completes to silence); **TWO static-cite disagreements:** `riir-gm-tool` is a MISSED true positive (bin autodiscovered from `src/main.rs`, no `[[bin]]` section — invisible to the section-based scanner = **B1**) and `riir-chain/crates/riir-chaind` FIRED 3 cites where a negative was expected (member with `[[bin]]` cannot see the root's strip+thin+1 = **B2**; 2 of 3 false for workspace builds, the panic cite half-true and self-hedged — a daemon is the advice's own caveat). Condition (4) PASS with caveats. `dist_gates` 7/7 green (condition 1 re-confirmed). Conditions (1)–(4) all PASS → evidence complete; **default-on NOT recommended yet** — B1/B2 are small `DistContext`-shaped scanner extensions; land them, re-run the Bench 082 scan table, then the owner decides (owner pull evidence still required per the gate). Full table + blind-spot writeup: `.benchmarks/082_dist_domain_promotion_evidence.md`.
**UPDATE 2026-09-10 (B1 + B2 landed — Bench 082 §6 addendum):** both scanner blind spots CLOSED in `src/domains/dist/` — new `context.rs` carries `DistContext::derive(manifest_path)` (one bounded ancestor walk, ≤32 levels, resolved once per manifest = the per-run cache) exposing `autodiscovered_bin` (B1: `src/main.rs` or directly-nested `src/bin/*.rs`, gated on `[package]` in the shared `ships_bin` helper) and `root_profile_knobs` (B2: strip/lto/panic presence from the nearest `[workspace]` ancestor, consumed by the shared `effective_knobs` self-or-root resolver); `member-profile-ignored` UNCHANGED; panic rule keeps its daemon caveat. Corpus/routing bytes UNTOUCHED (detector triggers only) → no score-bench owed (run #92's mechanism: vocab + OOD cannot move). Scan table re-run (same repos, `dist,latent_retrieval,rust_perf,clippy_lints,kat_service`, suggest lane, zero writes, `/tmp/plan085b`): **gm-tool 0 → 3 (B1 closed; `autodiscovered bin` excerpt)**; **chaind 3 → 1 (rules 1+2 silent — root strip+thin+1; panic cite honest, no `panic =` anywhere in that workspace, daemon caveat applies)**; deployer member 3 → 2 (root `strip = true` satisfies rule 1; remaining 2 true); viewbridge xtask 0 → 1 (NEW marginal: B1 exposes the real bin target; root lacks only `panic`; dev-side tool, reader-may-ignore class — recorded honestly); katgpt-rs (32 manifests), own root, sdk root + other members, deployer root, chain root: all hold at 0. `dist_gates` 7 → **13/13 green**; clippy `--features dist --all-targets` clean on all touched files. Incidental repair: `heal.rs`'s `dist_scan_context` (deleted) carried a stray stacked `#[cfg(feature = "docker")]` — it only compiled when BOTH features were on. Honest caveats (Bench 082 §6.4): `autobins = false` unread, `src/bin/<dir>/main.rs` + out-of-dir `[[bin]] path` still invisible, knobs are name-agnostic (any profile), a member's own knobs still satisfy rules 1–3 while rule 4 names the placement. **Remaining gate for default-on: owner pull evidence ONLY.**
**PRE-STAGED CITES 2026-09-10 (idle session, zero-cargo static scan of workspace manifests — now CONFIRMED/CORRECTED by the scanner, see the UPDATE above):** the condition-(4) scan should target `riir-game-sdk` first — its root manifest defines NO `[profile.release]` at all (only the riir-ai-723 dev-debuginfo rows), yet the workspace ships two real bin crates: `crates/riir-gm-tool` (`src/main.rs`, the GM dashboard binary) and `crates/riir-asset-converter` (2 bins incl. `genesis_art_commit`, a build-time tool) — the workspace-member profile trap applies verbatim (any fix must land in the ROOT manifest, members cannot carry `[profile.*]`). Secondary/weaker cite: `riir-deployer` has `strip = true` but no `lto`/`codegen-units` (partial knob set). Expected negative controls (well-profiled, must NOT fire): `katgpt-rs` (`lto = "fat"` + `codegen-units = 1` release+bench), `riir-chain` (strip+thin+1), `riir-viewbridge` (lto+1+strip with rationale comments). Cites are PROVISIONAL until the real `--domain dist` scanner confirms at the window (a static manifest read is not the tool's verdict).

## Premise (measured, this repo, 2026-09-09)

The shipped cargo-heal binary rode plain `--release` through v0.1.4 — 11.35 MiB
stripped, 70% of `.text` own-code corpus tables — while the entire knob set
(lto/codegen-units/opt-level/panic/strip) would have taken it to 4.34 MiB
(-61%). Found by the OWNER, not by any agent session, despite a rebuild-for-
staleness pass in between. The `binary_size_gate.sh` ceiling (commit 2c48a0d)
watches THIS repo's release lane; it cannot watch any OTHER repo, and it fires
at package time — after the bloat shipped. A healing domain fires at dev time,
on every manifest the healer scans. That is the closure the user demanded:
"we tend to deliver best software by heal/fix the perf/sec/mem/storage."

Verdict history: initially parked NO-GO in the post-mining intake
(`.distill/001_mining_queue_snapshot.md`, 2026-09-09 entry — kept, now marked
superseded); the owner's pushback corrected the reasoning. The "no hermetic
oracle" blocker was wrong by the repo's own precedent: `rust_perf` fixes
verify by `cargo test` re-run (well-formedness), never by benchmarking the
gain per-fix — the gain is proven ONCE at class level. That class-level proof
is this session's measurement.

## Non-goals (Phase 1)

- NO `--fix` writes to Cargo.toml (suggest-only; the bounded insert is a
  follow-up once the suggest tier earns trust).
- NOT a general TOML linter; NOT dependency auditing; NOT `cargo-machete`'s
  unused-dep territory.
- NEVER advice that edits `[profile.release]` directly when a separate
  `[profile.dist]` serves — `[profile.bench]` inherits release, and
  opt-level/codegen-units edits there poison the criterion benches (the
  measured reason this repo kept release clean).

## Phase 1 corpus (4 rules, all suggest-only, measured provenance)

| Rule | Trigger | Suggestion | Measured basis |
|---|---|---|---|
| `shipped-binary-unstripped` | `[[bin]]` + no `strip` key in any `[profile.*]` | `strip = true` — the symtab ships the full internal architecture | Proposal 003 G2: 14,485 symbols (2026-09-06) |
| `shipped-binary-missing-size-profile` | `[[bin]]` + no `lto` key in any `[profile.*]` | add a `[profile.dist]` block (lto fat + codegen-units 1 + opt-level "s" + panic abort + strip), never edit release | 11,903,168 B → 4,546,448 B on this repo (2026-09-09) |
| `cli-panic-unwind-tables` | `[[bin]]` + no `panic` key in any `[profile.*]` | `panic = "abort"` for short-lived CLI tools | part of the same -61% |
| `member-profile-ignored` | manifest carries `[profile.*]` + a PARENT manifest declares `[workspace]` (caller evidence, `ScanContext` pattern — silent without) | the block belongs in the workspace root; member profiles are ignored under `cargo build` from the root | cargo workspace semantics |

Context notes ride the corpus (`context_notes`): rule 2 carries the
bench-inheritance caveat; rule 3 carries the panic-isolating-server caveat
(a panic in a spawned task aborts the whole process under abort).

## Tasks

- [x] Feature `dist` (opt-in, default-off) in Cargo.toml.
- [x] `src/domains/dist/` — scanner (hand-written, zero deps), rules, corpus, `DistDomain` `HealDomain` impl, `refuses()` table.
- [x] Router + matcher integration: `DomainId::Dist`, `latent_matcher` arms, retrieval-eval names.
- [x] `heal.rs`: `--domain dist`, name-matched `Cargo.toml` collection, the suggest lane, `--corpus-stats` row, lease decode arm (`PACK_DOMAIN_DIST`).
- [x] `tests/dist_gates.rs` — corpus/detector agreement, fire/silence fixtures, member rule context gate, refusal pin.
- [x] Re-measure the router-annotation floor with dist on and pin it: `default + dist` measured **74/101 (73.3%)** vs the dist-off twin's 73/101 (+1 span, the docker-precedent mechanism) — floor pinned 69/101. The all-features arm could NOT be re-measured: all-features lib tests are red at HEAD from the PRE-EXISTING kernel_verify `.parses` conflict (Batch 138/139 tests vs `KernelVerifyResult` fields, unrelated to dist); that arm inherits the docker-on twin until that lane is repaired.
- [x] score-bench at the canonical state: BLOCKED — the `score_bench` feature build is broken at HEAD (E0631 `v5_is_removal_rule` signature mismatch, from `bb02431`, pre-existing, unrelated to dist). Documented instead: at the canonical state (default + latent_retrieval) the dist corpus is feature-gated OFF — the compiled corpus/index/routing are bit-identical by construction; full lib suite green both states (1312 no-dist / 1326 dist). The score-bench run is owed by whichever lane repairs the feature. → **RESOLVED 2026-09-10 (Bench 082): blockers repaired (Batch 140 `d2d2388` + `91493a1`); dist-ON run #92 FLAT, byte-identical row.**
- [x] Docs: AGENTS.md feature list, `.docs/11_domains/dist.md` + index row.
- [x] Mark the intake entry superseded (`.distill/001_mining_queue_snapshot.md`).
- [x] 2026-09-10 (Bench 082): promotion evidence COMPLETE — (1) `dist_gates` 7/7 green at `/tmp/plan085`; (2) score-bench dist-ON #92 FLAT; (3) router floor re-pin carried from Phase 1 (74/101 dist-on, floor 69); (4) consumer scans: true positives on riir-game-sdk/riir-asset-converter + riir-deployer/riir-deployer, negatives held (katgpt-rs, riir-viewbridge, own), two scanner blind spots documented (B1 autodiscovered-bin invisibility — gm-tool missed; B2 member-cannot-see-root-knobs — chaind false-fired). Default-on stays OWNER-DECIDED; recommendation: stay opt-in until B1/B2 land + owner pull evidence.

## Promotion path (the GOAT gate, written down before the work)

Opt-in until ALL of: (1) suggest precision — the in-tree fixtures + this
repo's own Cargo.toml produce exactly the intended citations and ZERO on the
lib-only/complete-profile negatives; (2) score-bench FLAT at the canonical
state; (3) router-annotation re-measured + re-pinned per state; (4) a real
consumer repo scanned with a true-positive cite. Default-on promotion follows
the `rust_perf` precedent and needs owner pull evidence.
