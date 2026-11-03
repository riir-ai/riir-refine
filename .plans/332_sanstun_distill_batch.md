# Plan 332 — Batch (nominal 327): sanstun → rust_perf concurrency + sec families

**Status:** STAGED — claim-first at the next mining slot (distill verdict YES B−; no corpus edit before the claim + T0)
**Claimed:** zed-m3-batch327, 1793708074, expires 2026-11-05T12:14Z

Distill verdict: `.research/364_sanstun_distill.md` (YES B− corpus / lane-intel N-A stated / product-coverage MODERATE). Source: firezone/sanstun @ `35cb1932948bd0776ef7d590329c1fdc0b1b9718` (Apache-2.0, tip 2026-11-03). Five candidates: 1 rust_perf concurrency lead (B, NotBenchable-by-domain annotation class) + 4 sec transcription candidates (suggest-only, 2 expected to survive cleanly, 1 adjudicated at T0, 1 expected drop).

## Phase 0 — provenance + slot + claim

- [ ] Fetch + claim: `git fetch origin` in riir-refine; check no live foreign claim owns the slot (claim_lease_check + the queue's live claims); append the Claimed line here + the claim row in `.distill/001_mining_queue_snapshot.md`; push (the arbiter)
- [ ] Re-clone at the pin: `git clone --depth 1 https://github.com/firezone/sanstun.git .raw/sanstun` + `git rev-parse HEAD` must equal `35cb1932948bd0776ef7d590329c1fdc0b1b9718` (moved HEAD = re-pin + re-verify every quote at the new sha BEFORE authoring — the tip is fast-moving, #129 landed the day of the verdict)
- [ ] License re-verify at the clone root (Apache-2.0 + NOTICE.txt boringtun lineage)
- [ ] T0 re-reads the verdict's UNREAD list: `handshake.rs`, `crypto.rs`, `keys.rs`, `timers.rs`, `index.rs`, `ring_buffer.rs`, `tai64n.rs`, `rng.rs` — grade fresh anything kernel/numeric-adjacent
- [ ] Re-verify the crate-private-constructor claim in `message.rs` at the pin (the blog says it; the grep showed it — confirm per type)

## Phase 1 — dedup re-verification (mandatory, 3+ vocabularies per candidate)

- [ ] `batch-reserve-monotonic-counters-offload-bulk`: grep rust_perf + kernel_opt corpora under `reserve|consecutive counters|nonce range|Send.*encrypt|offload bulk|batch reservation`; the B283 certifying-token family is the named kinship (validation-skip vs offload-bulk — read + rule distinct in the entry)
- [ ] `secret-key-without-zeroize`: grep all domains under `zeroize|Zeroizing|secret.*drop|key material`; confirm zero owners at the landing state
- [ ] `untrusted-keyed-state-unbounded`: grep sec + rust_perf under `MAX_SOURCES|bounded table|refuse.*full|cardinality|per-key state`; confirm distinct-from length-prefix-validate-before-alloc + unbounded-read-into-vec in-rule
- [ ] `gateway-type-forces-security-check`: grep sec under `gateway|checkpoint|crate-private|constructor`; confirm distinct-from client-ip-from-untrusted-forwarded-header in-rule
- [ ] Candidate 5 (reject-before-expensive-sink) T0 adjudication: convert ONLY if a parse-shaped trigger emerges; expected honest drop (the B258 invented-check class)

## Phase 2 — entry authoring (original fixtures, never source paste)

- [ ] rust_perf: candidate 1 → one entry file (next `entries_NN.rs`), category `concurrency`; fixture = the two-regime shape (per-item guarded reservation vs batched range + Send reservation handed off); NotBenchable annotation (`bench_spec: None`) with the serial-component honesty note carried in-rule; the replay-window-tolerance regime condition stated in-rule
- [ ] sec: candidate 2 → crypto-hygiene family, dep-gated on a `zeroize` import (the constant-time-secret-compare shape)
- [ ] sec: candidate 3 → input-validation family, the cardinality face; fix = bounded table + fail-closed refusal of NEW keys when full (the source's own eviction-refusal rationale carried in-rule)
- [ ] sec: candidate 4 → trust-boundary family, suggest-only with honest recall limits (vocabulary-gated security-handler detection)
- [ ] Every fixture standalone-compiles + executes before landing (the Batch-259 discipline); every source quote machine-verified at the pin

## Phase 3 — wiring + count pins

- [ ] mod.rs registration (rust_perf entry file list + sec count), category comments
- [ ] Count pins via ONE shape-asserted python pass (tests.rs sites + router.rs + the domain docs headers — the B265 script-job law); targets rust_perf 292→293, sec 35→38 (or 35→37 if candidate 4 drops at authoring)
- [ ] batch327 needle gate + parse gate (add to the standing gates; needles keyed AFTER reading the landed text — the Batch-259 lesson)

## Phase 4 — gates

- [ ] `cargo test --lib` default features (green + count floor held)
- [ ] `cargo test --lib --features rust_perf_bench` (the annotated-not-benchable pin moves if candidate 1 lands)
- [ ] sec-domain gate posture (the B258/B297 probe-parity convention for suggest-only sec entries)
- [ ] `cargo clippy --all-targets -- -D warnings` at default + affected feature postures
- [ ] Retrieval floors re-measure (corpus delta) — re-pin only with attribution
- [ ] Freeze-table regeneration IF clippy_lints corpus moved (expected NOT owed — P30, sec/rust_perf delta only); score-bench appended per convention
- [ ] Release-set binary rebuild + install (rust_perf is release-set surface — the §0 bin-age law)

## Phase 5 — records + close

- [ ] Research 364 addendum: the landing state (candidates converted/dropped with reasons, quote re-verification results, T0 findings)
- [ ] Queue snapshot landing row (this file's discharge)
- [ ] AGENTS.md sec + rust_perf rule counts if moved; `.docs/11_domains/*.md` if the house updates them per batch
- [ ] `py ../katgpt-rs/scripts/dual_allocation_gate.py` clean at the landing
- [ ] Commit + push (all artifacts one unit); `.raw/sanstun` REMOVED at close

## Honest drops allowed

Candidate 5 (expected); candidate 4 if the trigger cannot go even suggest-shaped; any candidate whose T0 re-read at a moved pin invalidates its quotes. The uniffi precedent (4 of 6) is the expected band.
