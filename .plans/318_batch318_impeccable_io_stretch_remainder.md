# Plan 318 — Batch 318: the impeccable io stretch-tier remainder → rust_perf

**Status:** STAGED — claim-first (Batch 318; the plan-316 `- [-]` remainder: 2 B− rules, `generation-counter-stale-timer-invalidation` + `exclusive-mkdir-temp-resource-reservation`; research `.research/356` is the working inventory; source pbakaus/impeccable @ `02523aec3e4c4580102d996d1b6a71fbe062e03f`, Apache-2.0)

Completes Plan 316's honest partial: the stretch tier landed 1 of 3 (the turnstile); this batch converts the remaining two validated candidates. Same pool as B316 (the impeccable io/process pool's own deferred half) — NOT the plan-306 (Leviathan rusqlite/sec) or plan-310 (infinity tokio) pools, whose claims stand pre-expiry; plumbing collisions resolve by the sequential-commit rebase flow (the B310-2 resume precedent).

## Phase 0 — provenance + slot

- [ ] Re-clone at the pin: `git clone --depth 1 https://github.com/pbakaus/impeccable.git .raw/impeccable` + `git rev-parse HEAD` must equal `02523aec3e4c4580102d996d1b6a71fbe062e03f`; byte-verify the two source sites: `live/src/server_state.rs:427–448` (the gen-counter timer block) + `crates/browser/src/cdp.rs:135–158` (the mkdir reservation loop)
- [x] Claim the batch in `.distill/001_mining_queue_snapshot.md` (claim row + 48 h expiry; push is the arbiter); scope note: the b316 pool's own remainder, the plan-306/plan-310 pools untouched
- [x] `python3 ../katgpt-rs/scripts/dual_allocation_gate.py` at the plan allocation (0/0/0 measured at claim time)

## Phase 1 — dedup re-verification at the slot

- [ ] Re-run both-vocabulary greps per candidate against the corpus AT THE LANDING HEAD (356's zero-hits are dated pre-B316-landing): `generation` + `stale.*timer|timer.*invalid|Weak.*upgrade`, `mkdir|AlreadyExists|temp.*reserv|serial.*pid`
- [ ] Adjudicate home firing sites (informs fixture realism, never a gate): grep this repo + the workspace for detached-timer reschedule shapes and timestamp-named temp dirs

## Phase 2 — authoring (target 2 own rules)

- [ ] `generation-counter-stale-timer-invalidation` (B−; concurrency): fixture = detached `thread::spawn` timer armed with a captured gen vs gen-bump-under-lock on reschedule + re-lock-and-check + Weak-upgrade shutdown guard; carry the callback-staleness decision content (the stale closure must not act); anti-regime: single-threaded reschedules where the timer handle is cancelled directly
- [ ] `exclusive-mkdir-temp-resource-reservation` (B−; io): fixture = timestamp-only path (two calls same tick collide) vs the create_dir loop on `AlreadyExists` with atomic serial + pid + stamp; carry "a timestamp is not a reservation" verbatim-shaped decision content; anti-regime: single-caller contexts where collisions cannot occur
- [ ] Every entry: category per the rust_perf taxonomy (concurrency / io), kinship cites recorded, NotBenchable annotations (`bench_spec None` — ordering/liveness wins have no µs axis, the dial9 class)
- [ ] Fixtures are ORIGINAL minimal Rust reproductions — never paste impeccable source verbatim (license hygiene + binary-only distribution rule)
- [ ] Fixtures standalone-compiled AND executed (`rustc --edition 2021 --test`) before entry — every arm bounded

## Phase 3 — gates (at the native toolchain pin, isolated CARGO_TARGET_DIR)

- [ ] Batch needles + parse gate test (new needles wired into the corpus tests)
- [ ] `cargo test --lib` default posture (count floor = current + landed)
- [ ] `cargo test --lib --features rust_perf_bench` posture
- [ ] `cargo clippy --all-targets -- -D warnings` × postures (default / rust_perf_bench / no-default / all-features); mechanical findings → the healer FIRST (`cargo refine --fix --write --verify`)
- [ ] Retrieval floors both postures — HELD expected (suggest-class delta)
- [ ] Freeze/score-bench NOT owed (rust_perf-only delta, the P30 rule — verified at landing)
- [ ] Maintained binary REBUILD + install at the landing HEAD (rust_perf IS release-set — the B255/B308 law); `cargo refine --version` verifies the stamp

## Phase 4 — count pins + docs

- [ ] Corpus len witness sites re-pinned 285→287 (the historical batch-pin sites, the B239/B207 pin-hiding class) + annotated 232→234 + NotBenchable 163→165 + mod.rs header count
- [ ] AGENTS.md rust_perf Role bullet + README + `.docs/11_domains/rust_perf.md` batch line (AGENTS/README are sibling-dirty (b317): commit MY hunks only — clean-blob construction)
- [ ] `.research/356` status line → stretch tier landed 3 of 3
- [ ] Queue landing row (the claim's discharge row)

## Phase 5 — close

- [ ] Commit `feat: batch 318` (+ docs) on develop; push; the push is the arbiter
- [ ] `rm -rf .raw/impeccable` + the isolated CARGO_TARGET_DIR (a landed batch with a live `.raw/` entry is an unfinished batch)
