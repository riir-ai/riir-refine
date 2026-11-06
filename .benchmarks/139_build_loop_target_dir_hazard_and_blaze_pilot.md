# Bench 139 — build-loop pilot II: the shared-target gate-invalidation hazard (plain vs huacnlee fork) + the blaze dev-loop A/B

**Status:** PARTIAL — Part A (hazard) MEASURED + verdict; Part B (blaze) PRE-REGISTERED (`c26f9690`, before install), installed on amo, QUEUED but NOT YET MEASURED — queue head-of-line blocked (see Part B results) (riir-refine issue 211)

Session: katgpt-rs-18@m3, 1793966832. Follows [Bench 138](138_build_loop_pilot_huacnlee_fork_sccache.md).

## Part A — the CARGO_TARGET_DIR hazard, reproduced on purpose

### Provenance

`PROVENANCE: power=AC Power load=26.55 swap=604.44M canary=skipped powermode=2(high)`
(`riir-reflex/scripts/bench_preflight.sh`, m3, 2026-11-06 ~19:1x local; m3 BUSY —
20 contenders, 14 rustc from sibling sessions). This is a CORRECTNESS
reproduction, not a timing cell: the race is sequenced by marker files, so box
load cannot change the verdict (every cell repeated 4×, 4/4 identical).

Postures: plain = rustup `cargo` with `RUSTUP_TOOLCHAIN=1.99.0-aarch64-apple-darwin`;
fork = `~/.cargo/bin/cargo-hc` (`cargo 1.101.0 (e611c7a87 2026-11-04) (huacnlee/cargo 0.102.0-9)`,
sha256 prefix `287d943b5db3a198`), same `RUSTUP_TOOLCHAIN`, `CARGO_FORK_AUTO_UPDATE=0`.

### The fixture (scratch, `/tmp/hz211`, removed at close)

A one-package workspace (own git repo — the fork keys its build dir by `.git`):
bin `hzbin` prints `A`, or `B` under feature `variant`; integration test
`tests/gate.rs` (the "gate") spawns `env!("CARGO_BIN_EXE_hzbin")` and asserts
the output equals its own compile-time posture. Sequencing: the gate reads the
bin once (`gate_pre`), writes `started`, waits for the concurrent builder's
`b_done`, then reads the bin again and asserts. Process A = `cargo test --test
gate` (default features). Process B, launched when `started` appears (i.e.
AFTER A's cargo released its build lock and is running the test binary — the
`.cargo-lock` window AGENTS.md documents):

- **flip**: `cargo build --features variant --bin hzbin`
- **clean**: `cargo clean`

Both postures prewarmed so B is a re-uplift (~0.05 s), never a compile — the
race is the uplift path, not compile time.

Why `CARGO_BIN_EXE`: uplifted artifacts (`target/<profile>/<bin>`) carry no
feature hash, so any build of the package in the same dir overwrites them. The
class is LIVE in this fleet: `git grep -l CARGO_BIN_EXE -- '*.rs'` = riir-refine
**18 files**, seal-game-editor 4, seal-remake 1 (katgpt-rs, riir-ai, riir-kat,
riir-train 0).

### Cells (4 runs each: 1 + 3 repeats; all 4/4 identical)

| cell | posture | B's target | B's checkout | scenario | gate exit | gate read | verdict |
|---|---|---|---|---|---|---|---|
| P1 | plain | SAME `CARGO_TARGET_DIR` | same | flip | **101** | want A **got B** | HAZARD REPRODUCED (false red) |
| P2 | plain | DISTINCT `CARGO_TARGET_DIR` | same | flip | 0 | A = A | isolation law WORKS |
| P3 | plain | SAME | same | clean | **101** | spawn ENOENT (bin gone) | HAZARD REPRODUCED |
| P4 | plain | SAME | separate git worktree | flip | **101** | want A **got B** | shared dir defeats worktree split |
| F1 | fork | (ignored) | same | flip | **101** | want A **got B** | fork does NOT prevent |
| F2 | fork | DISTINCT `CARGO_TARGET_DIR` (**ignored** — "this cargo always uses `./target`") | same | flip | **101** | want A **got B** | fork REMOVES the isolation escape hatch |
| F3 | fork | (ignored) | same | clean | **101** | spawn ENOENT | fork does NOT prevent |
| F4 | fork | (ignored) | separate git worktree | flip | 0 | A = A | per-checkout `./target` isolates |
| F5 | fork | (ignored) | separate git worktree | clean | 0 | A = A | per-checkout `./target` isolates |

### Verdict

- **The fork's fine-grained locking does NOT prevent the gate-invalidation
  class.** Locks cover the BUILD; the hazard window is AFTER cargo releases
  the lock and runs the test binary, which reads the uplifted artifact a
  second process re-uplifts (F1) or deletes (F3). Same as plain cargo (P1/P3).
- **The fork is strictly WORSE on the same checkout:** plain cargo's
  documented escape hatch (`CARGO_TARGET_DIR=/tmp/<plan>`, P2 PASS) is
  silently ignored by the fork (F2 FAIL). No knob restores it (binary
  strings: `CARGO_SHARED_DEPS`, `CARGO_CHECK_SHARED_DEPS`,
  `CARGO_SHARE_WORKSPACE_UNITS`, `CARGO_FORK_AUTO_UPDATE`,
  `CARGO_FORK_SELF_UPDATE`, `CARGO_FORK_PINNING_TOOLCHAIN` — none redirects the uplift dir).
- **The fork's replacement protection is the GIT WORKTREE** (F4/F5 PASS):
  each checkout keeps its own `./target` uplift dir while deps/units share
  under `~/.cargo/targets/`. So adopting the fork requires rewriting the
  isolation law from "`CARGO_TARGET_DIR=/tmp/<plan>` per plan" to "one git
  worktree per concurrent lane" — two sessions in ONE checkout (this fleet's
  common shape on m3: 6 sessions in `katgpt-rs/`) have NO isolation under the
  fork at all.
- Gate (issue 211 §Conflicts 1): **FAIL as a drop-in** — the fork's locking is
  NOT a replacement for the isolation law. Conditional adoption (worktree-per-lane
  rigs only, e.g. the git-workflow ticket lanes) stays an owner call.

## Part B — blaze dev-loop A/B (PRE-REGISTERED before any measurement)

Tool: blaze (`indexable-inc/blaze`, formerly ra-jit until 2026-10-24) — a
daemon that keeps a workspace compiled in memory; `blaze build|check|run|test|bench`
take cargo's flags; builds with its OWN pinned nightly regardless of
`rust-toolchain.toml`; macOS Apple silicon + Linux; installs to `~/.cargo/bin`
+ `~/.blaze` (no root). Their claims (not evidence): edit rebuilds ~10× on
macOS arm64.

Pre-registered protocol (fixed before install):

- **Rig:** amo (M5 Ultra; bmo — the preferred quiet runner — was OFFLINE at
  registration: ssh timeout to 100.100.117.44). Run under
  `agent-mail rig run amo --exclusive` so the pairs see a quiet box;
  provenance line beside the numbers.
- **Loop:** seal-game-editor (bevy/egui tree) in a `/tmp` worktree of
  amo's clone (siblings symlinked read-only); target `-p feat-item --lib`
  (egui + editor-core/widgets/data/release + seal-anim, 83 tests).
- **Edit:** rewrite one appended probe fn body in `crates/feat-item/src/lib.rs`
  (`pub fn blaze_probe() -> u32 { <i> }`, i changes per iteration — a real
  codegen change, not a comment touch).
- **A** = `cargo test -p feat-item --lib` (rust-toolchain 1.99.0, the plain
  dev loop); **B** = `blaze test -p feat-item --lib` (daemon warm).
  Both warmed with one untimed edit→test first.
- **Metric:** edit→test-green wall (edit applied → command exits 0 with the
  test count reported), **interleaved A→B pairs, N = 7**, p50 per arm.
- **Bar:** blaze p50 ≤ **0.5×** cargo p50 (≥ 2×) AND identical pass count
  (correctness: blaze must report the same tests passing; a divergence FAILS
  regardless of speed). Also reported (not gated): first-build wall, daemon
  RSS (`blaze status`), disk.
- **Decision rule:** PASS → blaze becomes the recommended dev-loop rung
  (build-hygiene §2 rung 3) for bevy-scale loops, with cargo/CI kept as the
  final word (blaze's own README says so); FAIL → recorded, not adopted.

### Part B results

**NOT MEASURED YET — blocked in the rig queue, not by the tool.** No number
below is a blaze result.

Feasibility (verified, not assumed):

- Install needs NO root: the installer (`https://ix.dev/blaze`, 353 lines,
  sha256 `2cca089b…98eb6`, read before running) writes only
  `~/.cargo/bin/{blaze,blaze-proc-macro-srv,ra-jit,ra-jit-rustc,ra-jit-rustc-first}`
  + `~/.blaze/` and checks the release tarball against its `SHA256SUMS`.
  Installed on amo 2026-11-06: `blaze 0.20261027.0 (49ca152)`, stable channel,
  toolchain `nightly-2026-10-27` (123 MB download, 24.8 s), `~/.blaze` = 661 MB.
  Side-by-side — no cargo wrapper enabled, system cargo untouched. Uninstall:
  `blaze stop; rm -rf ~/.blaze ~/.cargo/bin/{blaze,blaze-proc-macro-srv,ra-jit,ra-jit-rustc,ra-jit-rustc-first}`.
- Platform: macOS Apple silicon + Linux only (Windows v2 untested by upstream)
  → amo/bmo/m3 qualify, the 4090 does not.

Run state: the pre-registered script `/tmp/blz/run_ab.sh` on amo is queued as
`agent-mail rig run amo --shared --cores 8 --mem 12 --timeout 3h` (job
`01a1b0cb-3d02-76e3-bbe5-3b3a62549c83`). It creates its own `/tmp/blz`
worktree of amo's seal-game-editor (the probe edit lives only in that
worktree), symlinks siblings, runs the warm-up + 7 interleaved pairs, then
stops the daemon, removes the worktree and symlinks; results land in
`/tmp/blz/out/{meta,cells}.txt` (+ `blaze_status.txt`, `disk.txt`).

Why it has not run (2026-11-06, ~1.5 h waited): bmo — the preferred quiet
runner — was OFFLINE (ssh timeout to 100.100.117.44) throughout; on amo an
Exclusive job (`2993a478`, riir-game-sdk budget gate) sits at queue head
waiting for an empty rig, while a Shared ~36 h campaign (`44cdb52a`,
sr1098, running 8 h at the time) holds amo — so every Shared job behind the
Exclusive is head-of-line blocked even with 81 GB free / load 6 of 30.
Bypassing the queue was rejected (fleet law: heavy work through the queue).
m3 refused (owner laptop, BUSY: 20 contenders). **Next agent:** read
`/tmp/blz/out/cells.txt` on amo once the job ends (or re-queue the same
script on bmo when it returns), then apply the bar above unchanged.
