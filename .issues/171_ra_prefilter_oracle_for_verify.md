# Issue 171 — The blaze idea, ported: rust-analyzer in-process pre-filter oracle for `fix_verify` + `--fix-compile`

**Status:** PROPOSED — awaiting Claude verdict; implementation only after the GOAT below passes on a recorded corpus
**Claimed:** zed-glm-m3-verdict171, 1792922094, expires 2026-10-27T09:54:54Z
(the verdict ping-pong session for the five adjudication questions below — claim posted before the session; the M3 path the release note invites. The preserved five questions ride the release note; no other work claimed.)
**Lease released:** 2026-10-25T17:5x +07 (shikuwa/4090) — the verdict session could NOT run from this box: three `request_verdict` attempts (2 sessions + 1 fresh) all returned "Authentication required" from the `claude_code` reviewer backend (no Claude Code auth on the 4090). The verdict claim (commit `56a602a3`) is RELEASED so an unblocked lane (e.g. the M3, which ruled rethink 037 today) can run the ping-pong. The five adjudication questions were composed and are preserved in the session record — a claiming lane can reuse them verbatim: (1) does the T0→T1→T2→T3 ladder merit AGREE as written — any flaw in the tiered-rescue mechanism, class-split trust model, or G1 SUPERSET-OR-EQUAL yield contract; (2) is T0b correctly severable — safe to skip re-checking packages a round's reverts (own or dependency) did not touch given the verify loop's revert semantics; (3) is the 2-check arithmetic sound, or is there a case class where the pre-filter pays even at 2 checks (rescue round avoided when ALL dirty classes measure trusted); (4) installed-binary-first vs co-spiking the in-process ra_ap_* lane at T1; (5) should T0b land ahead of/independent of the verdict, or wait for the ruling. Implementation does NOT follow from this release — T0 measures first per the issue's own gate.
**Date:** 2026-10-25
**Type:** POC / optimization (healer wall-clock; verification oracle substrate)
**Origin:** [seal-remake Proposal 009](../../seal-remake/.proposals/009_blaze_dev_loop_build_driver.md) (the dev-loop half of the same idea) · blaze (indexable-inc) as prior art, NOT as a dependency
**Consumes:** `src/fix_verify.rs` (the verify loop) · `src/compile_oracle.rs` (the shared subprocess substrate) · `rustc_errors` (`--fix-compile`, the second oracle consumer)

---

## The cost shape today

`fix_verify --verify` (DEFAULT-ON with `--fix` on release builds) runs this loop per heal run:

1. baseline `cargo check` (abort if the tree is already broken),
2. write ALL fixed files,
3. `cargo check --all-targets --message-format=json` **per owning package**,
4. attribute errors → span-level/whole-file revert of offending edits,
5. re-check, loop bounded by the edit count.

Each round is a **fresh rustc subprocess** over the package: parse + macro-expand + typeck from
scratch. The module doc says it plainly: "This is a landing-time tool, not a hot loop." The
motivating measurement is Bench 019 §G5 run 1: 30 edits written, 6 errors in 4 files → multiple
cargo rounds before the kept set was clean. `--fix-compile` (the `rustc_errors` driver) pays the
same shape per repair iteration — its oracle is the same `compile_oracle` subprocess.

**The honest arithmetic first (this gates everything below):** the loop already reverts EVERY
attributable suspect in ONE round (`attribute_and_revert`, `src/fix_verify.rs:404`) — extra
rounds come from rustc hiding later errors behind earlier ones, not from per-edit round trips.
In the common dirty case (round 1 dirty + round 2 clean = 2 checks), a pre-filter plus an
unconditional rescue round costs the SAME 2 checks plus RA load — zero saving. A pre-filter can
only pay when (a) rounds WITHOUT it exceed 2, AND (b) trusted pre-reverts skip the rescue round.
So this issue measures first (T0) and closes as DEFER-with-number if the corpus says 2-check
world.

## The port (tiered, substrate-first — T0 gates everything)

**T0 — measure before building (both halves are worth landing regardless):**

- [ ] T0a Round-count census: `VerifyReport.n_checks` (src/fix_verify.rs:148) distribution over
      the Bench-019 corpus + `--misses` queue replays. Mostly 2 checks → the pre-filter CANNOT
      pay (the arithmetic above) → this issue closes as DEFER with the number recorded.
- [ ] T0b Package-scoped re-checks: today every round re-checks EVERY owning package, including
      packages that were clean and were touched by no revert (own or dependency). Skipping those
      is a cheap, authority-preserving wall-clock fix in its own right — land it regardless of
      what T0a says about RA.

**The substrate-first rule:** `src/pruner.rs` ALREADY reserves the seam — `rust_analyzer_bin`
(pruner.rs:68, "for the Phase 3"), a `Command::new` invocation site at :229. The INSTALLED
rust-analyzer binary (a rustup component — toolchain-matched, zero new deps) in its batch
diagnostics mode is tried BEFORE any `ra_ap_*` dependency. In-process crates are escalated to
ONLY if the binary mode's output shape or classification proves insufficient for span-matching.

**The syn-first rule:** the heal path already runs syn parse checks — syntax-broken edits are
caught free. RA is only consulted for what remains: type-level errors.

**The mechanism** (only if T0a says the rounds exist to save):

- feature `ra_prefilter` (opt-in; implies `fix_verify`); kill-switch `RIIR_REFINE_RA_PREFILTER=0`.
  NEVER in the release set.
- Load the owning package(s) into the RA analysis ONCE per run; **the load time is part of every
  measured total** (G2), never amortized away in a note.
- After write-all, BEFORE the cargo round: RA diagnostics; edits whose spans carry RA hard
  errors form the pre-revert set, split BY DIAGNOSTIC CLASS.
- **Tiered rescue:** the T1 spike measures RA-vs-rustc agreement PER CLASS. Classes with
  measured high agreement: pre-revert is FINAL — no rescue round (this is where the −1..−N
  saving lives). Classes known to disagree with rustc (cfg-posture, proc-macro expansion —
  whatever T1 measures): pre-reverted edits go through the rescue round.
- **The rescue is attribution-loop-aware:** a dirty rescue round feeds its errors back into the
  NORMAL `attribute_and_revert` loop — it never drops the whole batch (one bad edit must not
  take good edits down with it).
- **The rescue carries its own revert fallback, scoped to the rescue batch only.** It must
  NEVER reach `RevertStep::RestoreAll("(cascade)")` (fix_verify.rs:462) — the cascade would undo
  edits the earlier rounds ALREADY verified clean. Worst case in the rescue path: +2 checks on
  a false-red class; the saving: −1..−N checks on trusted classes.
- The tool's promise — **every kept edit is cargo-verified** — is preserved EXACTLY: a
  pre-revert only removes an edit from the tree before cargo sees it; it never puts an
  unverified edit in.
- Disclosure: one `ra prefilter:` line in the run summary (pre-reverted N by class · rescued M ·
  rounds saved K · RA load ms) + RA-vs-cargo disagreements logged to the post-mining intake
  (Issue 089 queue).
- Secondary consumer (only after fix_verify proves): `--fix-compile` pre-screens candidate
  repairs between rustc iterations.

## Rejected alternative (recorded)

Shelling out to `cargo blaze check` as the fast oracle: an external daemon, 1.99.0-only, tens of
GB of RAM, and a third-party correctness story inside the AUTHORITATIVE path — the healer's
authority must stay cargo. In-process RA keeps the dep tree honest and the authority unchanged.

## Honest caveats

1. **RA is a THIRD typeck.** The workspace already measures one divergence class (check vs
   clippy: the E0689/E0631 escape class). RA diverges more — cfg/feature posture, proc-macro
   expansion edges. Mitigations: cargo stays final word; the prefilter only PRE-reverts
   (conservative on the tree); the tiered rescue bounds yield loss per class; disagreements are
   logged, never silent.
2. **If the binary-mode seam suffices, there is no new dep at all.** The `ra_ap_*` escalation
   (no semver — pin + GOAT on bump) happens only if T1 proves the installed binary cannot do
   the span-matching.
3. **The 2-check world is the likely one.** Batch-revert-per-round is already near-optimal;
   T0a may honestly close this issue. That is a correct answer, recorded with the number.
4. **RA load time can exceed the checks it replaces** on a large workspace warm-cache. G2
   counts it IN the total — never an amortization footnote.
5. **The naive prefilter is REJECTED by design:** reverting on RA red with no rescue trades
   kept-edit yield for wall-clock and breaks the tool's primary metric. The tiered rescue is
   not optional.

## GOAT gate

- **G1 (correctness, restated as two claims):** (a) **soundness preserved EXACTLY** — every
  kept edit is cargo-verified, byte-for-byte the prefilter-off promise; (b) **yield never
  regresses** — the kept-edit set is a SUPERSET-OR-EQUAL of the prefilter-off posture on every
  corpus case (the pre-filter may only cost wall-clock, never kept edits). The tiered rescue is
  proven by a planted false-red case in a DISAGREEING class (rescued) and a planted red in a
  TRUSTED class (pre-revert final, cargo round confirms clean).
- **G2 (perf):** paired interleaved wall-clock A/B on the corpus, same box state, **RA load
  time counted in the total**. Promotion bar: ≥ 2× on multi-round (> 2 check) cases AND no
  regression on clean or 2-check cases.
- **G3 (authority untouched):** all existing gates pass unchanged; `--verify` default posture
  identical with the feature off; cargo invoked exactly as today in the on posture.
- **G4 (measurement artifact):** the per-class divergence log populated — RA-vs-cargo
  disagreement rows recorded per run, feeding the post-mining queue (and re-tiering the rescue
  classes on evidence).

## Tasks

- [ ] T0a `n_checks` distribution on the Bench-019 corpus + misses-queue replays (defer-if-2 gate).
- [ ] T0b Package-scoped re-checks: skip clean packages unaffected by a round's reverts (land regardless).
- [ ] T1 Spike: the INSTALLED rust-analyzer binary (the `rust_analyzer_bin` seam, pruner.rs:68) over the Bench-019 fixtures; the per-CLASS agreement table vs rustc; escalate to in-process `ra_ap_*` ONLY if the binary mode cannot span-match.
- [ ] T2 The tiered prefilter: class-split pre-reverts, trusted classes final, disagreeing classes through the rescue round; rescue feeds `attribute_and_revert`; rescue-scoped revert fallback (never RestoreAll cascade).
- [ ] T3 Corpus GOAT (G1–G4 above) on recorded batches; verdict lands in `.benchmarks/`.
- [ ] T4 Divergence log → post-mining intake; re-tier classes on evidence; adapt or demote on the record.
- [ ] T5 (only after T3 PASS) the `--fix-compile` consumer spike.

## Related

- [seal-remake Proposal 009](../../seal-remake/.proposals/009_blaze_dev_loop_build_driver.md) — the same substrate idea, dev-loop half
- `src/fix_verify.rs` module doc (the loop, the failure-mode honesty, the P43/cfg audits that already bound what "verified" means) · `attribute_and_revert` (:404) · `RevertStep::RestoreAll` (:462) · `VerifyReport.n_checks` (:148)
- `src/compile_oracle.rs` — the seam where the prefilter slots · `src/pruner.rs:68` — the `rust_analyzer_bin` seam (substrate-first: try it before any new dep)
- Bench 019 §G5 — the motivating multi-round run
- Issue 089 post-mining intake — where the disagreement log feeds
