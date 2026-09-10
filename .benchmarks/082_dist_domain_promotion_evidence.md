# Bench 082: `dist` domain promotion evidence — score-bench dist-ON + real-consumer scans (Issue 085)

**Status:** RECORD — score-bench dist-ON run #92 **FLAT** (condition 2 PASS) + condition-(4) scan table landed with true positives confirmed and two scanner blind spots documented; Issue 085 promotion evidence COMPLETE, default-on stays OWNER-DECIDED (recommendation: stay opt-in until the two evidence gaps land).

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

## Run + environment

macOS M3 Max, 2026-09-10 ~01:00–01:15 local, zero cargo/GPU contention (solo session; sibling WIP present in the tree but not executing). All cargo at isolated `CARGO_TARGET_DIR=/tmp/plan085`. Score-bench ran the frozen v1 set (35 fixtures, real-clippy oracle, gen-1). Scans: suggest lane only, zero writes to any repo; the meter lines are accounting-only local projections (no `kat_transport` in the build → no network surface compiled).
