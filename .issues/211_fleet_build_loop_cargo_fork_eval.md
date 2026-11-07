# Issue 211 — fleet build-loop lever: the huacnlee/cargo fork + blaze (owner-gated pilot)

**Status:** ACTIVE — owner directive 2026-11-06 ("it's about how we build and fix too... fast compile like blaze and lower target like huacnlee/cargo is a plus"); [Bench 138](../.benchmarks/138_build_loop_pilot_huacnlee_fork_sccache.md) + [Bench 139](../.benchmarks/139_build_loop_target_dir_hazard_and_blaze_pilot.md) COMPLETE — the CARGO_TARGET_DIR hazard test is DONE and the fork FAILS it as a drop-in (worktree-per-lane is its only isolation); hygiene residue fixed; **blaze A/B MEASURED (via the bmo bridge) — FAIL, 0/14 valid arms, NOT adopted** (details in Bench 139 Part B); fleet fork adoption remains the owner call
**Claimed:** zed-13afd776@m3, 1794011737, expires 2026-11-09T00:35:37Z

## Source

`huacnlee/cargo` @ `12d1048a` (fetched 2026-11-06; MIT OR Apache-2.0; a fork of
`rust-lang/cargo` @ ea4e34c / cargo 1.101.0). Drop-in replacement binary: per-project
shared build dir (`~/.cargo/targets/<name>-<hash>/` keyed by the repo's `.git`),
cross-project shared dependency builds (`~/.cargo/targets/shared-deps/`, keyed
version+features+profile — Go-build-cache style), content-keyed workspace units
(worktrees of one commit reuse each other's crates AND incremental caches),
self-cleaning GC (units unused ≥ 1 day removed; a project keeps ≤ 2× its largest
build of the last day; shared deps kept 7 days / 50 GiB cap; LRU to fit; one loud
line when it frees; never removes a locked unit), fine-grained locking (parallel
agents share one dep build), dev-profile debuginfo default (line tables for
workspace crates, none for deps), macOS loose-object cleanup, background
self-update (checksum-verified rename-into-place; `CARGO_FORK_AUTO_UPDATE=0` off;
off under `CI`). Partial adoption knobs exist: `CARGO_SHARED_DEPS=0`,
`CARGO_CHECK_SHARED_DEPS=0`, `CARGO_SHARE_WORKSPACE_UNITS=0`, `[build.gc]` rows.

Distill verdict (2026-11-06, queue snapshot): **corpus NO / lane intel HIGH** —
this issue IS the lane-intel conversion. Measured claims are THEIRS (8-core Apple
Silicon, no pre-registered bar): 5 worktrees of a 525-crate workspace 33.6 GB →
5.1 GB (6.6×) and 8.5 min → 1.6 min (5.3×); a second project's first build
513.8 s → 16.2 s on cross-project shared deps. Treat as prior, never as evidence.

## Why this lane (the fleet's own measured pain)

- The `build-hygiene` discipline already mandates per-repo what the fork makes
  fleet-default: dev `line-tables-only` + no dep debuginfo, one shared dep tree
  instead of N posture copies (the `gate` profile bounded a 133 GB target dir —
  build-hygiene §1's measured shape). The fork generalizes the shared-tree idea
  across projects AND worktrees, zero-config.
- Every repo pays its own copy of `katgpt-core` + the shared dep graph; 32 repos ×
  postures × worktrees is the `disk-clean` skill's biggest class, and the global
  `CARGO_TARGET_DIR=/tmp/<plan>` rule exists BECAUSE sharing was unsafe. The
  fork's answer is fine-grained locking + GC instead of isolation.
- **riir-refine itself is the heaviest beneficiary:** the fix lane's release
  compile gates and the auto-oracle's `cargo clippy --workspace --all-targets`
  (+ gap passes) are the 97%-of-the-run legs (our own ⏱ phase-blame line). Shared
  deps cut first-run and gap-pass compile cost fleet-wide. The oracle-results
  cache (`08c5b6ee7`) covers REPEAT sessions; the fork covers the compile leg
  under it — complementary, not redundant.
- Transitive: riir-deployer builds (the zigbuild cross ladder) and every sibling's
  dev loop ride the same cargo. seal-game-editor's vessel cache bounds issue
  (riir-game-sdk 054) cites the fork's GC as a reference design — independent of
  adoption.

## Conflicts — each is a law change, not a nit; read before any pilot

1. **`CARGO_TARGET_DIR` is IGNORED (warning only).** The global
   `CARGO_TARGET_DIR=/tmp/<plan>` rule, the gates' shared-target-dir hazard
   premise (a gate whose verdict a concurrent cargo in the same target can
   invalidate — the `.cargo-lock` detection), and the clippy verifier's pinned
   scratch-target posture (Issue 110) all lean on target-dir control. The pilot
   must either prove the fork's locking actually prevents the gate-invalidation
   class (reproduce the hazard on purpose first) or run with
   `CARGO_SHARED_DEPS=0`-class partial adoption.
2. **Gate premises move:** debuginfo defaults, shared incremental caches, and the
   remapped compile dir (`-Zremap-cwd-prefix=.`) alter measured builds; budget
   gates and size/timing pins re-baseline; `cargo -V` names the fork, so the
   profile-is-part-of-the-claim law gains a "which cargo" clause.
3. **Supply chain + self-update on build rigs:** a third-party self-updating
   binary replacing cargo on m3/amo/bmo/4090 is an owner trust call; auto-update
   PINNED OFF (`CARGO_FORK_AUTO_UPDATE=0`) so no rig's toolchain moves mid-lane;
   verify `rust-toolchain.toml` pins (1.99.0 etc.) still resolve through the
   fork's rustup-proxy shim.
4. **Determinism/debugging:** dev builds carry less debuginfo by default
   (behavior unchanged; goldens/benches unaffected) — debugger workflows change
   (start from the workspace root; `dev-full`-class escape hatches still win).

## Pilot shape (pre-registered A/B — build-hygiene §2/§5 laws apply)

- [x] ONE box (m3), one workspace repo with a real dev loop; verify the uninstall
      script first (restores the rustup proxy — the demote path is one command).
      VERIFIED 2026-11-07 (zed-13afd776@m3): no uninstall script exists and none is
      needed — the fork is side-by-side (`~/.cargo/bin/cargo-hc`, the kept Bench-138
      state) and the rustup proxy at `~/.cargo/bin/cargo` was never replaced; the
      demote path is `rm ~/.cargo/bin/cargo-hc` plus an optional
      `rm -rf ~/.cargo/targets` (fork-GC-bounded, regenerable) — "uninstall = delete
      the binary" per Bench 138 §Anomalies.
- [ ] Pre-register the bars BEFORE install: edit→test-green p50 (interleaved A→B
      pairs), fresh-worktree build time, `du` over all target trees after 7 days,
      quick-gate time delta. Decision rule: adopt only if p50 improves ≥ 20% OR
      7-day disk halves, AND no gate premise breaks; otherwise uninstall.
- [ ] Box-state provenance beside every number (`bench_preflight`).
- [x] Verify the binary-sha-changed law (a comment touch must not relink).
      MEASURED 2026-11-07 (zed-13afd776@m3, hello-bin crate, dev profile, BOTH
      postures): the law AS PINNED fails identically under both cargos — fork
      (`cargo-hc`): comment touch → recompile + relink in 0.11 s, binary sha
      4301d551… → 0d1b53d2…; plain-cargo control (scratch `CARGO_TARGET_DIR`): the
      same rebuild in 0.08 s, sha 8037cab6… → e7de751a…. Cargo fingerprints are
      mtime/size-based, so ANY source edit (even a comment) rebuilds; byte-stable
      output is a content-hash/sccache property, not a cargo property. VERDICT: no
      fork delta — the pilot-shape premise was mis-pinned; recorded
      verified-negative, not an adoption blocker.
- [x] Explicitly test the CARGO_TARGET_DIR posture: run a gate concurrent with a
      build and confirm the verdict stays valid (the shared-target hazard,
      reproduced on purpose, before trusting the fork's locking). DONE — Bench
      139 Part A (`c26f9690`): the fork does NOT keep the verdict valid.
- [ ] 4090 note: Windows install exists (`install.ps1`) but the CUDA box syncs via
      schtasks; any adoption there is a separate owner call after m3 evidence.

## Not this issue

- No corpus rules — the source class is build infrastructure (verdict recorded in
  the queue snapshot; candidate kills listed there).
- No riir-refine code changes — the lever is fleet posture, not the rulewright.
- Fork internals were NOT read line-by-line (README + commit surface + their
  bench pages only); re-clone at the pin before quoting implementation details.

## Results — Bench 138 (2026-11-06, m3, side-by-side, system cargo untouched)

Measured (full record: `.benchmarks/138_build_loop_pilot_huacnlee_fork_sccache.md`;
fork v0.102.0-9 sha256-verified, side-by-side at `~/.cargo/bin/cargo-hc`;
sccache 0.19.0 via brew; both postures pinned to the identical rustc 1.99.0):

- **Fork worktree amortization: REAL at dep-heavy scale.** seal-game-editor
  (bevy tree): second-worktree gate **12.77 s vs 88.81 s plain (6.95×, 0.14× of
  the pre-registered bar)**; second-worktree disk +425 MB vs +1.27 GB. Small
  workspaces (riir-refine, riir-kat): only 0.75× — build.rs carve-outs keep
  workspace units per-checkout; only deps share.
- **Fork first-build tax: heavy.** bevy-scale first build 2.0× wall / 3.3× CPU
  vs plain (anomaly, unexplained — reproduce before quoting). NET at N=2
  worktrees the fork LOSES on wall and disk; **crossover at the 3rd worktree
  per repo**, widening linearly after.
- **sccache: NO wall gain** for the fixer posture (0 cross-project hits; thin
  cacheable surface: bins/proc-macros/path deps; dep units key differently
  across lockfiles). Its `CARGO_INCREMENTAL=0` half is a free 3× CPU cut on
  gates — a build-hygiene posture note, not a tool adoption.
- **The CARGO_TARGET_DIR hazard test — DONE in Bench 139 Part A** (below).
- Fork state on m3 after the bench (kept, disclosed): `cargo-hc` binary +
  `~/.cargo/targets/` 3.59 GB (fork-GC-bounded, `rm -rf` resets).

## Remaining tasks (this issue)

- [ ] Owner call: adopt the fork on the ≥3-concurrent-worktree rigs (the
      git-workflow ticket lanes — amo/bmo heavy-ticket work) vs keep
      side-by-side. Decision input: Bench 138 + the fleet worktree-concurrency
      census. **RESOLVED 2026-11-07 — delegated-owner verdict round 1 (Claude,
      session 4785a7a0, REVISE→resolved): DECLINE adoption** (the pre-registered
      rule fails twice: the CARGO_TARGET_DIR premise broke — same-checkout
      sessions get no isolation, and today's refine lane runs 3 concurrent
      agents on ONE checkout; no win at fleet-typical N=2, crossover at the 3rd
      worktree; sha-law no-delta). **`cargo-hc` UNINSTALLED per the verdict**
      (rm the binary + `~/.cargo/targets` — reversible; Bench 138/139 keep the
      ≥3-worktree dep-heavy data). Any FUTURE adoption is owner-gated.
- [x] The CARGO_TARGET_DIR hazard reproduction test — [Bench 139](../.benchmarks/139_build_loop_target_dir_hazard_and_blaze_pilot.md)
      Part A (`c26f9690`, m3, 9 cells × 4 runs, deterministic 4/4). A concurrent
      re-uplift (`build --features variant`) or `cargo clean` flips/kills a
      running `CARGO_BIN_EXE` gate under plain cargo in a shared target (P1/P3),
      and `CARGO_TARGET_DIR` isolation fixes it (P2). **The fork's locking does
      NOT prevent the class** (F1/F3 — locks cover the build, the window is the
      test run after the lock is released) **and the fork ignores
      `CARGO_TARGET_DIR`, so the isolation escape hatch is gone on one checkout**
      (F2 FAIL where P2 passes). Only a separate git worktree isolates under the
      fork (F4/F5 PASS). Verdict: FAIL as a drop-in; adoption needs the law
      rewritten to "one git worktree per concurrent lane" — sessions sharing a
      checkout (m3: 6 in `katgpt-rs/`) get NO isolation under the fork. The class
      is live: riir-refine has 18 `CARGO_BIN_EXE` files.
- [x] blaze pilot (dev-loop rung, build-hygiene §2 ladder rung 3) — **MEASURED, FAIL** (Bench 139 Part B, 2026-11-06, ran on bmo via the thunderbolt bridge — tailscale down, machine up, preflight PASSED both cells): blaze `0.20261027.0` cannot build either fleet workspace tried — sge's `coreaudio-sys` build script trips its MIR frontend (daemon cannot LOAD the workspace; every arm rc=101) and riir-kat's workspace panics its embedded rust-analyzer loader (`Unable to get FileSourceRootInput … this is a bug`); the plain-checkout control fails on blaze's own `unsupported: writable static __RUST_STD_INTERNAL_VAL` in test code and exits via its cargo fallback. **0/14 valid timed arms — the pre-registered correctness precondition unreachable → recorded, NOT adopted**; uninstalled from bmo + amo, `/tmp/blz*` scratch removed, the moot queued amo job `62549c83` cancelled (disclosed in Bench 139). Re-open triggers in Bench 139 (upstream build-script compilation + std-internal support, or a workspace it demonstrably loads+passes). Side note: the plain cargo loop it would have to beat is already 1.3-1.5 s on riir-kat and ~17 s on sge's feat-item lib (the 15 s test-run floor dominates).
- [ ] Upstream question: the 2-3.3× first-build tax (Bench 138 anomaly) —
      reproduce isolated or file upstream. **Verdict round 1 REVISE (D4): do
      NOT wait for a quiet m3 — the isolated A/B is QUEUED on amo** (Exclusive,
      cargo-hc shipped to `amo:~/bin/cargo-hc` sha-verified, seal-game-editor
      at amo's develop head; plain arm on scratch `CARGO_TARGET_DIR`, fork unit
      reset per rep, 2 interleaved reps). File upstream ONLY if reproduced;
      the posting act itself stays owner-reserved (outward-facing).
- [x] Hygiene residue the fork surfaced — riir-refine `b43a3c32` (the
      `cargo::unused_dependencies` hit was the TRAINER's edge to
      `riir-refine-fixer`, never used in code — dropped, BOUNDARY.md keeps it
      allowlisted; plus `paired_read` gated on `escalation_manifest`, dead under
      `--features cf_workers_ai`) + riir-neuron-db `04fc3c8` (redundant
      `package.homepage` removed).
