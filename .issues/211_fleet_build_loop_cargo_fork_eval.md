# Issue 211 — fleet build-loop lever: the huacnlee/cargo fork (owner-gated pilot)

**Status:** OPEN — POC/eval; owner-gated (installs a third-party cargo fork on build rigs)

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

- [ ] ONE box (m3), one workspace repo with a real dev loop; verify the uninstall
      script first (restores the rustup proxy — the demote path is one command).
- [ ] Pre-register the bars BEFORE install: edit→test-green p50 (interleaved A→B
      pairs), fresh-worktree build time, `du` over all target trees after 7 days,
      quick-gate time delta. Decision rule: adopt only if p50 improves ≥ 20% OR
      7-day disk halves, AND no gate premise breaks; otherwise uninstall.
- [ ] Box-state provenance beside every number (`bench_preflight`).
- [ ] Verify the binary-sha-changed law (a comment touch must not relink).
- [ ] Explicitly test the CARGO_TARGET_DIR posture: run a gate concurrent with a
      build and confirm the verdict stays valid (the shared-target hazard,
      reproduced on purpose, before trusting the fork's locking).
- [ ] 4090 note: Windows install exists (`install.ps1`) but the CUDA box syncs via
      schtasks; any adoption there is a separate owner call after m3 evidence.

## Not this issue

- No corpus rules — the source class is build infrastructure (verdict recorded in
  the queue snapshot; candidate kills listed there).
- No riir-refine code changes — the lever is fleet posture, not the rulewright.
- Fork internals were NOT read line-by-line (README + commit surface + their
  bench pages only); re-clone at the pin before quoting implementation details.
