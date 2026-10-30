# Plan 316 — Batch 316: Impeccable io/process robustness → rust_perf

**Status:** STAGED — nominal Batch 316, queued behind the live queue (claim per the mining queue before any corpus edit; re-verify slot freedom at T0)

Distill verdict: `.research/356_impeccable_design_detector_distill.md` (YES B− corpus, pbakaus/impeccable @ `02523aec3e4c4580102d996d1b6a71fbe062e03f`, Apache-2.0). This batch converts the io/process robustness candidates into rust_perf corpus rules. The design-taxonomy half is NOT this batch (owner-gated future domain / skill transcription — see the note).

## Phase 0 — provenance + slot

- [ ] Re-clone at the pin: `git clone --depth 1 https://github.com/pbakaus/impeccable.git .raw/impeccable` + `git rev-parse HEAD` must equal `02523aec3e4c4580102d996d1b6a71fbe062e03f` (a moved HEAD = re-pin + re-verify every quote at the new sha BEFORE authoring)
- [ ] Claim the batch in `.distill/001_mining_queue_snapshot.md` (claim row + 48 h expiry; push is the arbiter); re-verify no live foreign claim owns the rust_perf slot (dedup-at-mining binds; the B293 cross-domain precedent)
- [ ] `python3 ../katgpt-rs/scripts/dual_allocation_gate.py` at the research/plan allocation (0/0/0 expected; highwaters re-read at landing)

## Phase 1 — dedup re-verification at the slot

- [ ] Re-run both-vocabulary greps per candidate against the corpus AT THE LANDING HEAD (the corpus moves; 356's zero-hit evidence is dated): `read_timeout|SO_RCVTIMEO|connect_timeout|set_read_timeout`, `piped.*drain|Stdio::null`, `kill\(\)|child.wait`, `turnstile|arrival.order`, generation-timer vocab
- [ ] Adjudicate home firing sites (informs in-rule fixture realism, never a gate): `riir-instinct/src/serve_edge.rs:943` + `riir-reflex/src/serve.rs:1197` (per-read-timeout-only posture — VERIFIED 2026-10-30, no watchdog; issues filed in both repos at verdict time), `riir-refine/src/l4_daemon.rs` ×3, `domains/docker/buildcheck.rs:246`, sge `world.rs:506` (kill sites — check wait() pairing; deployer verify.rs VERIFIED already-correct)
- [ ] Record the SEC CROSS-ROUTE in-rule (the B293 cross-domain precedent): the slow-drip shape is slowloris-class DoS — "sec considered" kinship line so a later sec batch never re-mines it (verdict session 687ea308 note 1)
- [ ] Read the needs_validation bodies the verdict flagged before authoring any rule whose trigger rests on them: `http.rs` read_request/turnstile full body, `cdp.rs` launch/kill paths, `server_state.rs` timer block

## Phase 2 — authoring (target 3–5 own rules + 1 rider)

- [ ] `per-read-timeout-vs-request-deadline` (A− lead; io): fixture = hand-rolled std TcpListener handler with `set_read_timeout` only vs total-deadline watchdog + `shutdown(Both)` + prior-timeout restore; carry the Windows parked-recv quirk verbatim (the platform-split regime is the decision content); NotBenchable annotation
- [ ] `child-piped-stderr-drain-for-process-lifetime` (B; io/concurrency): fixture = `Stdio::piped()` child + no reader vs drain-thread/stdio-null; carry the two-posture contrast (drain when stderr is load-bearing, `Stdio::null()` when quiet); anti-regime: short-lived children whose parent wait()s promptly (pipe buffer absorbs)
- [ ] `kill-without-wait-zombie` (B−; io): fixture = `kill()` without `wait()` vs kill+wait pairing (incl. Drop belt-and-braces); suggestion-grade
- [ ] `connect-timeout-walk-over-resolved-addrs` (B−; io): fixture = bare `TcpStream::connect` vs to_socket_addrs + per-addr `connect_timeout` walk keeping last error; regime: any off-loopback dial
- [ ] Stretch tier (land only if falsification leaves room): `arrival-order-turnstile-lane` (B) · `generation-counter-stale-timer-invalidation` (B−) · `exclusive-mkdir-temp-resource-reservation` (B−)
- [ ] Rider face on `prefix-sum-2d` (entries_01, count-neutral): 2D box-filter area-average via integral image, shrinking-resize regime (grow path bilinear untouched), integer inputs bit-exact
- [ ] Every entry: category assigned per the rust_perf taxonomy, kinship cites recorded (papaya/bounded-channel for turnstile; whole-round-warm family for any deadline sibling; nan-unsafe-sort-comparator for the coverage-validation note — no entry for it), NotBenchable annotations per the dial9 split
- [ ] Fixtures are ORIGINAL minimal Rust reproductions — never paste impeccable source verbatim (license hygiene + binary-only distribution rule)

## Phase 3 — gates (at the native toolchain pin, isolated CARGO_TARGET_DIR)

- [ ] Batch needles + parse gate test (new file needles wired into the batch test)
- [ ] `cargo test --lib` default posture (count floor = current + landed)
- [ ] `cargo test --lib --features rust_perf_bench` posture
- [ ] `cargo clippy --all-targets -- -D warnings` × postures (default / rust_perf_bench / no-default / all-features)
- [ ] Retrieval floors both postures — HELD, no re-pin expected (suggest-class delta)
- [ ] Freeze/score-bench per the P30 rule at landing time (rust_perf-only delta precedent: NOT owed — B308; verify against the standing rule at the landing window)
- [ ] Maintained binary REBUILD + install at the landing HEAD (rust_perf IS release-set — the B255/B308 law); `cargo refine --version` verifies build stamp

## Phase 4 — count pins + docs

- [ ] Corpus len witness sites re-pinned (tests.rs asserts + era witnesses + router expert-total + mod.rs slice header comment if the io/concurrency slices move)
- [ ] AGENTS.md rust_perf Role bullet + README + `.docs/11_domains/rust_perf.md` batch line
- [ ] `.research/356` status line → DISTILLED — Batch 316 landed (or the honest partial)
- [ ] Queue landing row (the claim's discharge row) in `.distill/001_mining_queue_snapshot.md`

## Phase 5 — close

- [ ] Commit `feat: batch 316` (+ docs) on develop; push; the push is the arbiter
- [ ] `rm -rf .raw/impeccable` (a landed batch with a live `.raw/` entry is an unfinished batch)
