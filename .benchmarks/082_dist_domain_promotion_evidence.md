# Bench 082: `dist` domain promotion evidence — score-bench dist-ON + real-consumer scans (Issue 085)

**Status:** RECORD — score-bench dist-ON run #92 **FLAT** (condition 2 PASS) + condition-(4) scan table landed with true positives confirmed and two scanner blind spots documented; Issue 085 promotion evidence COMPLETE, default-on stays OWNER-DECIDED (recommendation: stay opt-in until the two evidence gaps land).
**ADDENDUM 2026-09-10 — B1 + B2 LANDED, scan table re-run:** both blind spots closed in `src/domains/dist/` (see §6); the scan table re-run at the same feature set. gm-tool now cited 3 (B1 closed); chaind down 3 → 1 (rules 1+2 silent, panic honest); deployer member down 3 → 2 (root strip satisfies rule 1); one NEW legitimate citation surfaced (viewbridge xtask panic — B1 exposing a previously-invisible bin); all negative controls held. The corpus/routing is UNCHANGED — only detector triggers widened — so no score-bench re-run is owed (FIXABLE_DIST stays [], vocab + OOD cannot move, the same mechanism as run #92's FLAT). Details: §6 below.

## What this is

The two promotion-gate items Issue 085 left owed (its UPDATE 2026-09-10): (a) the score-bench run with `dist` compiled in — the corpus/routing twin of the dist-off canonical runs #87–#91 — and (b) condition (4)'s real-consumer-repo scans with a locally built binary. Both had been cargo-gated; this session ran them at zero contention, isolated `CARGO_TARGET_DIR=/tmp/plan085`.

Binary: built at working-tree HEAD `fcc478c` (the tree carries sibling-agent WIP — batch-142 kernel corpus entries + the Issue-080 kat_soft_gate/contribution removal — none of it dist-related; the score-bench numbers match the pre-WIP baseline exactly, so the WIP did not move the measurement).

```bash
CARGO_TARGET_DIR=/tmp/plan085 cargo build --release --features score_bench,dist,kat_service --bin cargo-heal
# (kat_service is load-bearing: the cargo-heal bin's required-features row is
#  [latent_retrieval, rust_perf, clippy_lints, kat_service]; score_bench implies
#  the first three + clippy_verify + katgpt-core/regime_probe but NOT kat_service.)
CARGO_TARGET_DIR=/tmp/plan085 cargo test --features dist --test dist_gates   # 7/7 green
/tmp/plan085/release/cargo-heal --score-bench                                # run #92
/tmp/plan085/release/cargo-heal --suggest --domain dist <manifests...>       # scans, zero writes
```

## 1. Score-bench dist-ON: #92 vs the dist-off baseline — FLAT

| metric | #87–#91 baseline (dist off) | #92 (dist ON) | delta |
|---|---|---|---|
| heal_rate | 96.2% | **96.2%** | 0 |
| created_rate | 1.9% (1 new lint) | **1.9% (1 new lint)** | 0 |
| parse_safety | 100% | **100%** | 0 |
| decline_correctness | 100% | **100%** | 0 |
| edits_total | 64 | **64** | 0 |
| vocab_size | 38 | **38** | 0 |
| OOD gap / d | −0.729 nats / −1.11 | **−0.729 nats / −1.11** | bit-identical |
| OOD artifact | `blake3:faa021c8…1914` | **`blake3:faa021c8…1914`** | identical |

Honest read: **FLAT, and not merely within noise — the row is byte-identical to the tail rows apart from `ts`.** This is the expected signature, and the mechanism is clean: `FIXABLE_DIST = []` (suggest-only), so the dist corpus adds no proposer vocabulary (`vocab_size` unchanged at 38); the dist entries join the latent pool but the OOD entropy statistic consumes proposer rule-score vectors, so it cannot move either. The one place dist measurably changes compiled-in state remains the router-annotation floor, already re-measured and re-pinned by Issue 085's Phase-1 session (74/101 dist-on vs 73/101 dist-off, floor 69) — condition (3) was satisfied there and is unchanged by this run. Condition (2) **PASS**.

## 2. Condition (4): real-consumer-repo scans (actual tool verdicts)

`--suggest --domain dist` over every `Cargo.toml` in each repo (manifest files passed directly; the syn suggest loop never runs, zero writes by construction). The PRE-STAGED cites were static manifest reads; these are the scanner's verdicts.

| repo | manifest | citations | verdict |
|---|---|---|---|
| riir-game-sdk | root | 0 | root has `[package]`+`[workspace]`+`[[bench]]`+`[[example]]` but NO `[[bin]]` → ship-size rules silent by design |
| riir-game-sdk | crates/riir-asset-converter | **3** (`shipped-binary-unstripped`, `shipped-binary-missing-size-profile`, `cli-panic-unwind-tables`, all at the `[[bin]]` line) | **TRUE POSITIVE** — 2 explicit bins incl. `genesis_art_commit`; the repo ships unoptimized/unstripped artifacts (root defines no `[profile.release]`) |
| riir-game-sdk | crates/riir-gm-tool | 0 | **MISSED TRUE POSITIVE** — bin is autodiscovered from `src/main.rs`, no explicit `[[bin]]` section; the section-based scanner cannot see it (blind spot B1) |
| riir-game-sdk | other 5 members | 0 | lib crates, correct silence |
| riir-deployer | root | 0 | virtual `[workspace]` root (`[profile.release]` strip only, no `[[bin]]`) — the staged "partial knob set" cite cannot fire at the root |
| riir-deployer | crates/riir-deployer | **3** (same trio, at its `[[bin]]` line) | **TRUE POSITIVE** — 6 explicit bins, member carries no profiles |
| katgpt-rs | root + 31 members | 0 | negative control **HOLDS** (root: lto fat + codegen-units 1; no member carries `[[bin]]` sections) |
| riir-chain | root | 0 | root knobs strip+thin+codegen-units present, no `[[bin]]` → silent |
| riir-chain | crates/riir-chaind | **3** (same trio) | **DISAGREES with the expected negative control** — see blind spot B2 |
| riir-viewbridge | root + 5 members | 0 | negative control **HOLDS** (lto+codegen-units+strip at root; members have no `[[bin]]` sections) |
| riir-clippy | root (this repo) | 0 | negative control **HOLDS** — the `[profile.dist]` block completes the own-manifest fixture to silence (matches `dist_gates`' `canonical_repo_manifest_fires_three_and_completes_to_silence`) |

On the chaind firing: 2 of the 3 citations are FALSE for workspace builds — `riir-chaind` built via `cargo build --release` from the workspace root **inherits** the root's `[profile.release]` (strip + thin lto + codegen-units 1); the "add strip / add a size profile" advice is wrong for this manifest. The third (`cli-panic-unwind-tables`) is half-true at workspace level — no `panic` key exists anywhere in the riir-chain workspace — but its own context note hedges it ("NOT for servers that isolate panics per task/connection"), and a chain daemon is exactly such a server.

## 3. The two scanner blind spots the scan exposed

These are Phase-1 design limits surfacing as evidence, not regressions — the issue's rule table says the triggers key on `[[bin]]` + same-manifest `[profile.*]` keys, and both findings follow from that:

- **B1 — autodiscovered bins are invisible.** `riir-gm-tool` ships a real binary via `src/main.rs` with no `[[bin]]` section; the scanner reads section membership only, so the sdk workspace's most-used dev binary gets zero advice. A `ScanContext` extension (the `DistContext` seam already exists for caller evidence) could carry "src/main.rs present" from the walker.
- **B2 — member manifests cannot see root profile knobs.** Rules 1–3 read only the scanned manifest's own `[profile.*]` keys, so any member with `[[bin]]` and no local profiles fires all three even when the workspace root carries the full knob set (chaind). The `under_workspace_root` evidence is already plumbed (`dist_scan_context`) but rules 1–3 don't consume it; the honest fix is hedging the advice when under a workspace root ("verify the ROOT manifest carries the knobs; member profiles are ignored"), the mirror image of `member-profile-ignored`.

No code was changed for this bench — both findings are recorded as the promotion gate's condition-(4) output.

## 4. Conditions checklist (the issue's promotion gate)

| # | condition | state |
|---|---|---|
| 1 | suggest precision — fixtures + own Cargo.toml intended cites, ZERO on lib-only/complete-profile negatives | **PASS** — `dist_gates` 7/7 green at `/tmp/plan085` (2026-09-10), incl. the own-manifest completes-to-silence fixture; the scan table adds real-repo negatives |
| 2 | score-bench FLAT at the canonical state | **PASS** — run #92 FLAT (byte-identical row) |
| 3 | router-annotation re-measured + re-pinned per state | **PASS (prior session)** — 74/101 dist-on vs 73/101 dist-off, floor 69 |
| 4 | real consumer repo scanned with a true-positive cite | **PASS with caveats** — 2 true-positive repos (sdk/asset-converter, deployer/riir-deployer), 1 missed true positive (B1), 1 false-positive class (B2) |

## 5. Recommendation

**Stay opt-in.** The suggest tier earns trust on fixtures, own manifest, and the two clean true-positive repos — but a default-on domain that fires 3 wrong citations on `riir-chaind` and misses `riir-gm-tool` would teach operators to ignore it within a week. B1 + B2 are small, `DistContext`-shaped scanner extensions; land them, re-run this scan table, and re-present for the owner's default-on decision (which also still needs the owner pull evidence the issue names).

> **2026-09-10: B1 + B2 landed same day — §6 is the re-run.** The recommendation above is now UNBLOCKED of its scanner conditions; the remaining gate is owner pull evidence only.

## 6. ADDENDUM 2026-09-10 — B1 + B2 landed, scan table re-run

### 6.1 What landed (detector triggers only — corpus bytes UNTOUCHED)

Both fixes live in the domain's new `context.rs` + the rewritten ship-size block in `rules.rs`:

- **B1 — autodiscovered bins visible.** `DistContext` carries `autodiscovered_bin: bool` (tree evidence: the manifest's directory contains `src/main.rs`, or a `*.rs` file directly inside `src/bin/`). One `ships_bin(scan, ctx)` helper gates all three ship-size rules: explicit `[[bin]]` OR (`[package]` present AND autodiscovery evidence). The `[package]` precondition keeps a virtual workspace root's stray `src/` from firing — autodiscovery is a package feature. Findings on autodiscovered bins carry an honest excerpt (`autodiscovered bin (src/main.rs or src/bin/)`) instead of a fabricated `[[bin]]`.
- **B2 — members consult the ROOT's profile knobs.** The same ancestor walk that derives `under_workspace_root` now also resolves `root_profile_knobs: Option<ProfileKnobs>` (strip / lto / panic presence across the root manifest's `[profile.*]` sections) from the root manifest text already in hand — one read, no extra I/O. One `effective_knobs(scan, ctx)` resolver feeds all three rules: a knob is satisfied when THIS manifest sets it OR — under a workspace root — the ROOT sets it (union, because a standalone build honors self knobs and a root build honors root knobs and the scanner cannot know which the reader will run). `None` (no workspace ancestor, or unreadable) never satisfies anything — standalone behavior is byte-identical to Phase 1.
- The `member-profile-ignored` rule is UNCHANGED (still keys on an explicit `[[bin]]` + own `[profile.*]`). The panic rule keeps its context-note caveat (panic-isolating server) — for chaind it fired and the caveat is the correct read.
- `DistContext::derive(manifest_path)` now lives in the LIBRARY (one bounded ancestor walk, ≤32 levels, resolved once per scanned manifest — the Copy context IS the per-run cache; no rule re-reads a file). `heal.rs`'s `dist_scan_context` was deleted; the lane calls `derive` directly. Incidental repair: the old bin function carried a stray stacked `#[cfg(feature = "docker")]` (it only compiled when BOTH features were on — a latent break for `--no-default-features --features dist`).

### 6.2 Re-run scan table (same repos, same feature set, suggest lane, zero writes)

Binary rebuilt at the B1+B2 tree, `--features dist,latent_retrieval,rust_perf,clippy_lints,kat_service --bin cargo-heal` (the bin's required-features row needs the four; `dist` added — the prior session reached the same effective set via `score_bench` implying three of them). `CARGO_TARGET_DIR=/tmp/plan085b`.

| repo | manifest | Bench 082 (before) | re-run (after) | verdict |
|---|---|---|---|---|
| riir-game-sdk | root | 0 | 0 | holds |
| riir-game-sdk | crates/riir-asset-converter | 3 | 3 | TRUE POSITIVE — unchanged (explicit `[[bin]]`, root has no profiles) |
| riir-game-sdk | crates/riir-gm-tool | 0 — MISSED TRUE POSITIVE | **3** (`autodiscovered bin` excerpt, line 1) | **B1 CLOSED** — `src/main.rs`, no `[[bin]]` section (verified: grep count 0) |
| riir-game-sdk | other 5 members | 0 | 0 | holds |
| riir-deployer | root | 0 | 0 | holds |
| riir-deployer | crates/riir-deployer | 3 | **2** (`missing-size-profile` + `panic`; `unstripped` SILENT) | **B2 working** — the root carries `strip = true`, which satisfies rule 1 for member builds; the remaining 2 are TRUE (no lto/panic anywhere in that workspace) |
| katgpt-rs | root + 31 members | 0 | 0 | negative control HOLDS — no `src/main.rs`/`src/bin` anywhere, so B1 exposes nothing |
| riir-chain | root | 0 | 0 | holds |
| riir-chain | crates/riir-chaind | 3 — FALSE for workspace builds | **1** (`cli-panic-unwind-tables` only) | **B2 CLOSED** — rules 1+2 silent (root: strip + thin + codegen-units 1); the panic cite is HONEST: no `panic =` key exists anywhere in that workspace, and the advice's own caveat (panic-isolating server) is the correct hedge for a daemon |
| riir-viewbridge | root + 5 members | 0 | **1 on crates/xtask** (`panic`, autodiscovered) | NEW, legitimate-by-semantics — B1 exposes a previously-invisible real bin target; the root has lto + codegen-units + strip but NO `panic`, so rules 1+2 stay silent and only the panic rule fires. MARGINAL value: xtask is a dev-side tool that never ships; the citation is technically true (the bin target exists, the knob is genuinely absent workspace-wide) but is the kind a reader may learn to ignore |
| riir-clippy | own root | 0 | 0 | holds — `[profile.dist]` completes every knob, so even B1's view of `src/bin/heal.rs` stays silent |

Per-repo before/after: game-sdk 3 → 6 total cites (gm-tool found), deployer 3 → 2 (false cite removed), chain 3 → 1 (two false cites removed), viewbridge 0 → 1 (one marginal cite gained), katgpt-rs 0 → 0, own 0 → 0. Net: 2 false citations eliminated, 1 missed true positive found, 1 marginal citation gained, every stable negative held.

### 6.3 Why no score-bench re-run (and no .highwater touch)

The corpus (`FIXABLE_DIST = []`, suggest-only entries byte-identical) and the routing tables are UNTOUCHED — only detector TRIGGERS widened. Run #92's own mechanism proves the score cannot move: proposer vocabulary is corpus-driven (`vocab_size` pinned at 38) and the OOD statistic consumes proposer rule-score vectors. The dist lane's suggest output is not a score-bench input. `dist_gates` re-verified instead: **13/13 green** (7 pre-existing + 6 new B1/B2 fixtures, `CARGO_TARGET_DIR=/tmp/plan085b`); lib suite at the then-HEAD: 1206 passed (the tree carries sibling-agent WIP — kernel corpus entries + kat_burn_stack — which accounts for the delta vs the 1326 recorded in Phase 1; none of it dist-related). Clippy `--features dist --all-targets`: ZERO findings on every touched file (pre-existing `rust_perf/corpus.rs` ×2 and the katgpt-core `qsg_gossip` cfg warnings are not this change's).

### 6.4 Honest caveats — where the scanner still disagrees with cargo semantics

- **`autobins = false` is not read.** A manifest opting out of autodiscovery beside a present `src/main.rs` would still fire. No such manifest exists in the scanned repos; the evidence helper stays deliberately simple.
- **Deeper bin shapes are not derived.** `src/bin/<dir>/main.rs` targets (cargo discovers those too) and `[[bin]]` sections whose `path` points OUTSIDE the manifest's directory are invisible to the tree check — a bin can therefore still be missed in principle (the B1 class is narrowed, not provably closed). Conversely the tree check can never invent a bin that cargo would not build (it requires `[package]`).
- **Name-agnostic knobs.** A root carrying `strip` under `[profile.dist]` satisfies the strip rule even though a plain `cargo build --release` of the workspace does not take the dist profile. Key presence in ANY profile is the deliberate Phase-1 semantic (the advice always names its own profile). Profile-NAME matching (self vs root, per name) is the honest next narrowing if the owner wants it.
- **A member's own knobs still satisfy rules 1–3** even though a root build ignores them — the union trades that gap for not double-citing, and `member-profile-ignored` fires simultaneously to move the block to the root. Residual: a root build of such a member ships unstripped until the block is moved; the advice covers it, the rule does not.
- **`exclude`d members** still get `under_workspace_root` (and now root knobs) from the nearest-ancestor walk — a hedged inaccuracy inherited from Phase 1, unchanged.
- **Per-manifest, not per-bin.** A crate with N bins gets 3 citations, not 3N (pre-existing design, matches Bench 082's asset-converter row).

### 6.5 Remaining gate for default-on

Owner pull evidence only (the issue's §Promotion path, condition on adoption). Scanner conditions 1–4 are all green at this tree; the two evidence gaps §5 named are closed by this addendum.

## Run + environment

macOS M3 Max, 2026-09-10 ~01:00–01:15 local, zero cargo/GPU contention (solo session; sibling WIP present in the tree but not executing). All cargo at isolated `CARGO_TARGET_DIR=/tmp/plan085`. Score-bench ran the frozen v1 set (35 fixtures, real-clippy oracle, gen-1). Scans: suggest lane only, zero writes to any repo; the meter lines are accounting-only local projections (no `kat_transport` in the build → no network surface compiled).
