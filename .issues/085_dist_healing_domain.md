# Issue 085 — `dist` healing domain (Cargo.toml build-config advice)

**Status:** OPEN — Phase 1 (`79dcf9b`, 2026-09-09) + the B1/B2 scanner extensions (`759e209d`, 2026-09-10) LANDED; all four promotion conditions PASS (Bench 082 + its §6 addendum). Feature `dist` stays OPT-IN. **Remaining gate: owner pull evidence ONLY** — default-on is owner-decided; no agent work is owed. Changelog: HISTORY.md "Issue 085".
**Static re-verification 2026-09-12 (documentation pass, zero cargo):** every cited artifact exists and says what this header claimed — `.benchmarks/082_dist_domain_promotion_evidence.md` §6 + §6.4, `src/domains/dist/context.rs` (`DistContext::derive`, `autodiscovered_bin`, `root_profile_knobs`), `tests/dist_gates.rs` **13 `#[test]` fns present in source** (a source count is NOT an execution — "13/13 green" is Bench 082's 2026-09-10 run, not re-verified here; the target is armed: `#![cfg(feature = "dist")]` + `required-features = ["dist"]`), `.docs/11_domains/dist.md` + both `.docs/11_domains/README.md` rows, the AGENTS.md feature row, and the SUPERSEDED intake entry in `.distill/001_mining_queue_snapshot.md`. Two drifts found and repaired in the same pass: the shipped corpus is **5 rules**, not the 4 the table below listed (Batch 143's `large-api-dependency-mostly-unused-hint` — `DIST_RULES`/`corpus.rs` both carry 5), and the §6.4 honest caveats existed only in the bench + this header — they are now on the domain doc page too.

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

## Corpus (5 rules, all suggest-only, measured provenance)

> Phase 1 shipped rules 1–4; Batch 143 added rule 5 (BuildTime family). The
> shipped set is `rules::DIST_RULES` — 5 ids, `FIXABLE_DIST = []`.

| Rule | Trigger | Suggestion | Measured basis |
|---|---|---|---|
| `shipped-binary-unstripped` | `[[bin]]` + no `strip` key in any `[profile.*]` | `strip = true` — the symtab ships the full internal architecture | Proposal 003 G2: 14,485 symbols (2026-09-06) |
| `shipped-binary-missing-size-profile` | `[[bin]]` + no `lto` key in any `[profile.*]` | add a `[profile.dist]` block (lto fat + codegen-units 1 + opt-level "s" + panic abort + strip), never edit release | 11,903,168 B → 4,546,448 B on this repo (2026-09-09) |
| `cli-panic-unwind-tables` | `[[bin]]` + no `panic` key in any `[profile.*]` | `panic = "abort"` for short-lived CLI tools | part of the same -61% |
| `member-profile-ignored` | manifest carries `[profile.*]` + a PARENT manifest declares `[workspace]` (caller evidence, `ScanContext` pattern — silent without) | the block belongs in the workspace root; member profiles are ignored under `cargo build` from the root | cargo workspace semantics |
| `large-api-dependency-mostly-unused-hint` | any `*dependencies` section naming a known-large API crate (`windows`, `windows-sys`, `rustix`, `aws-sdk-*`) | measure `-Zhint-mostly-unused` (nightly-only, `.cargo/config.toml` rustflags) — carries its own measure-first caveat, the scanner cannot see the flag adopted | Batch 143 (nnethercote Dec-2025: large compile-time wins for small-fraction consumers) |

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
