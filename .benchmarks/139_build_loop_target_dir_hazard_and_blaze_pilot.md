# Bench 139 — build-loop pilot II: the shared-target gate-invalidation hazard (plain vs huacnlee fork) + the blaze dev-loop A/B

**Status:** COMPLETE — Part A (hazard) MEASURED + verdict (katgpt-rs-18@m3); Part B (blaze) MEASURED 2026-11-06 ~21:2x via the bmo thunderbolt bridge (tailscale down, machine up — the ON/OFF jump rule), verdict FAIL (riir-refine issue 211)

Session: katgpt-rs-18@m3, 1793966832 (Part A + pre-registration); zed-m3-cargofork@m3, epoch 1793973547 (Part B measurement). Follows [Bench 138](138_build_loop_pilot_huacnlee_fork_sccache.md).

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

### Part B results — MEASURED (2026-11-06 ~21:2x, bmo via the thunderbolt bridge), VERDICT FAIL

**Blaze failed to build BOTH fleet workspaces tried — the pre-registered
correctness precondition ("identical pass count") is unreachable; 0/14 valid
arms. Per the decision rule: FAIL → recorded, not adopted.** The speed bar was
never reached because the load/precondition leg failed first:

- **seal-game-editor (the registered target):** the daemon cannot LOAD the
  workspace — `coreaudio-sys v0.2.18`'s build script contains a fn blaze's MIR
  frontend cannot compile (`clang_sys::build::dynamic::search_libclang_directories:
  IncompleteExpr`), and "a build with nothing of rustc's has nothing else to
  compile it". Every B cell (warm ×2 + 7 pairs) = build error, rc=101, ~20 s of
  load-attempt + fallback. The A arm was real and is the reported baseline:
  edit→test-green **17.0-18.0 s** (the test run floor is ~15 s of it; the
  suite's 2-5 flaky content-fixture failures on a bare worktree are
  cargo-vs-cargo identical and do not affect the A/B validity).
- **riir-kat (compatible-workspace control, leaf crate, no C build scripts):**
  the daemon PANICS loading the workspace — `vendor/rust-analyzer/crates/base-db/src/lib.rs:299:
  Unable to get FileSourceRootInput with vfs::FileId … this is a bug` — 0/7
  valid arms. The A arm: **444/444 green, edit→test-green 1.3-1.5 s** steady.
- **Control (harness exoneration):** `blaze test --lib` directly in bmo's own
  plain `~/git/riir-kat` checkout (no worktree, no symlinks) fails on blaze's
  own backend limit — `unsupported: writable static __RUST_STD_INTERNAL_VAL
  without a rustc symbol` across dozens of test fns (fn-pointer default
  closures, collect/from_iter paths — test code touching std internals its
  MIR/Cranelift backend cannot compile) — and exits via its cargo fallback, not
  its fast path. The worktree+symlink harness is NOT the cause.
- **An invalid lane, disclosed:** a riir-refine B2 cell set on bmo died in 0.08 s
  — bmo's `~/git` predates `riir-ledger` joining the workspace (no clone → cargo
  resolution refuses; a bmo sync-gap finding, not a blaze result).

Provenance (bmo, m5 ultra, quiet): sge cells `power=AC load=2.24 swap=756M
powermode=0(auto)`; riir-kat cells `power=AC load=1.77 powermode=0(auto)` —
preflight PASSED both. Blaze `0.20261027.0 (49ca152)`, its own
`nightly-2026-10-27`; cargo arm 1.99.0 (the sge/riir-kat pins). Scripts kept as
artifacts: `blz3_run_ab.sh` (riir-kat, the valid control pair) beside this
record; the sge script lives verbatim in the pre-registration above and was
`/tmp/blz/run_ab.sh` on bmo.

Reading per the pre-registered decision rule: **FAIL → recorded, not adopted.**
Blaze's own README concession ("It's young. Keep cargo check or your CI as the
final word") is the operative fact on our surface. Re-open triggers: upstream
ships `coreaudio-sys`-class build-script compilation + the std-internal-static
test-code support, OR a fleet workspace blaze demonstrably loads+passes.
Uninstalled from bmo (installed for this bench: `~/.blaze` 651 MB + the five
`~/.cargo/bin/{blaze,blaze-proc-macro-srv,ra-jit,ra-jit-rustc,ra-jit-rustc-first}`
binaryies) and from amo (the pre-registration install), `/tmp/blz*` scratch
removed on both; the queued amo job `62549c83` (head-of-line blocked, now moot)
is GONE from the queue (absent at close — expired or sibling-cancelled; my own
cancel found nothing to cancel) — no orphaned run remains. No numbers above may be quoted as
blaze SPEED results — blaze never reached a timed valid arm anywhere in this
bench.

Why it ran on bmo and not amo (kept from the pre-registration, for the record):
bmo — the preferred quiet runner — was tailnet-offline all day; the rig queue
laws forbid bypassing amo's head-of-line Exclusive job; the ON/OFF jump rule
(`ssh -J katopz@100.111.186.76 katopz@192.168.10.2`) reached the box (up 4
days, load 1.4-3.2, preflight PASSED) so the measurement ran there under the
identical pre-registered protocol, unchanged.

### Pre-measurement feasibility (kept verbatim from the pre-registration; installer sha + install surface are load-bearing for the uninstall above)

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

The amo queue block (job `62549c83` head-of-line behind the Exclusive budget
gate behind the sr1098 campaign) is CLOSED — superseded by the bmo-bridge
measurement above; the job was cancelled at close.
