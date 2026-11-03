# 364 — sanstun Distill (firezone/sanstun)

**Status:** DISTILLED — pending owner decision (nominal Batch 327 staged, plan-first; claim-first at the next mining slot per the queue)
**Date:** 2026-11-03 (zed-m3/M3, user-directed @distill: github.com/firezone/sanstun + the introducing-sanstun blog)
**Queue slot:** Batch 327 / Plan 332 — the sec crypto-hygiene + resource-exhaustion families, plus one rust_perf concurrency lead

## Source

- **Repo:** https://github.com/firezone/sanstun — a parallel-friendly **sans-IO WireGuard® protocol implementation in Rust**. Apache-2.0. Fork lineage: Firezone's `boringtun` fork (itself from Cloudflare's boringtun), now the successor — the fork is deprecated in favor of sanstun. crates.io-published (`sanstun 0.2.0`), docs.rs present, 101 stars, 524 commits, edition 2024, rust-version 1.89.
- **Pin:** `35cb1932948bd0776ef7d590329c1fdc0b1b9718` (tip, 2026-11-03, "feat!: reserve counters and decrypt data messages in batches (#129)"). Cloned to `.raw/sanstun` this session; **removed at the verdict commit** (pin carries the provenance; quotes re-verified at mine time per the standing law).
- **Inspection:** README + the 2026-10-31 blog (both fetched in full); `Cargo.toml`, `lib.rs` (exports), `interface.rs` (receive path), `ratelimiter.rs`, `replay.rs`, `session.rs` (reservation + batch sections, lines 120–200 + 380–545), `tunnel.rs` (types + grep of the begin/commit surface), `message.rs` (ctor visibility grep). **Unread (verify at mine time):** `handshake.rs`, `crypto.rs`, `keys.rs` (grepped only), `timers.rs`, `index.rs`, `ring_buffer.rs`, `tai64n.rs`, `rng.rs`, `errors.rs` bodies.
- **Completeness signals (good):** `[profile.release] lto = true, codegen-units = 1` and the same for `[profile.bench]` — they benchmark. Five criterion benches (`data_path`, `blake2s`, `handshake`, `chacha20poly1305`, `x25519_*`). No `todo!()` density observed in the read files. Production-operated (Firezone runs WireGuard in production; >70 PRs of fork patches distilled into this redesign). This is NOT a WIP repo — grade at repo level stands.
- **Scale:** 4,580 lines across 16 src files. Compact, high-quality, heavily tested (unit tests inline; `tests/it` integration tree).

## Verdict

**YES B− corpus / lane-intel N/A (stated, not skipped — no mapping to the open Metal/CUDA/llama.cpp league lanes; WireGuard's packet path shares no mechanism with GDN/prefill/decode/quant) / product-coverage MODERATE — the sec crypto-hygiene family gains its zeroize member (RustCrypto-wide dep, 0 corpus rules today), the resource-exhaustion family gains a cardinality face, and rust_perf concurrency gains one strong coordination lead.**

Domain routing: the headline pattern routes to **`rust_perf` concurrency** (NOT kernel_opt — it is a CPU host-path coordination law, the CPU-SIMD-is-not-kernel_opt trap in reverse). The sec candidates route to **`sec`** (crypto-hygiene + input-validation families), all suggest-only transcription class.

## The transcription law applies (no language gate)

Source is Rust — direct fixtures possible. But note for completeness: even the blog's prose half (sans-IO doctrine) would be transcribable; the corpus batch mines the CODE, not the doctrine.

## Ranked candidate table

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | **`batch-reserve-monotonic-counters-offload-bulk`** | `session.rs:156` (`reserve_counters`) + `session.rs:394-535` (`EncapsulateBatch`→`PendingEncapsulate`) + tip commit #129 | **B — the GOAT-named lead** | Reserve N consecutive counters in ONE bound-checked op (`first.checked_add(len)` + `<= REJECT_AFTER_MESSAGES` filter), hand the immutable reservation (fixed nonces, `Send`) to any worker thread; encryption becomes order-free and parallel. Blog-measured (provenance, never bars): begin_encapsulate 20.8 ns vs encrypt 933 ns @1280B — bookkeeping 2% of the work, "hints at a potential speedup of more than 45x" (their own hedge). `PendingEncapsulate::split_off` distributes counter ranges to multiple workers. Two-regime: per-item guarded reservation (serial trip per item) vs batched range + offload. The receiver's replay window tolerates gaps/reorder — the enabling precondition, named in-rule as the regime condition. NOT the sec checked-arithmetic rule (that owns the bound check; this owns the BATCH+OFFLOAD shape). NotBenchable-by-domain: the win lives in multi-threaded throughput the Pareto validator cannot host (the tokio/rayon/B136/entries_43-46 annotation class); the serial-side component (one reserve vs N) is benchable but trivial-magnitude — stated honestly in the bench_spec: None annotation. |
| 2 | **`secret-key-without-zeroize`** | `keys.rs:3,45,71` ("An X25519 static private key, zeroized on drop"), `Zeroizing` pervasive (8 files) | **B−** | sec CryptoHygiene, dep-gated on a `zeroize` import (the weak-hash-to-blake3 / constant-time-secret-compare house shape — fires only when the file already imports zeroize, so the suggestion cannot break the build). Trigger: a struct in the secret vocabulary (*Key/*Secret/*Seed/*Token) holding raw byte-array/String key material with no Drop impl and no Zeroize bound → key bytes persist in freed memory (heap spray, core dumps, swap). Fix: `Zeroizing::new(..)` wrap or `#[derive(Zeroize)]`/`ZeroizeOnDrop`. Suggest-only, NotBenchable-by-domain (correctness/security axis). DISTINCT-FROM `constant-time-secret-compare` (owns the COMPARE primitive; this owns LIFETIME/ERASE). |
| 3 | **`untrusted-keyed-state-unbounded`** | `ratelimiter.rs` `MAX_SOURCES = 65536` + full-table refusal + `allow()` returns false for newcomers | **B−** | sec input-validation / resource-exhaustion family, the CARDINALITY face: a HashMap keyed by peer-derived distinct keys (addr, session id, tenant, device) growing without a cap lets one peer family (many spoofed/distinct keys) exhaust memory — the many-keys memory DoS, cousin of the single-packet length-DoS. Fix shape (the source's own): bounded table + **fail-closed refusal of NEW keys when full** (eviction would reward the attacker by admitting them over honest entries — the source's test name says it: `full_table_refuses_new_sources_until_idle_ones_are_dropped`). Suggest-only (choosing the cap + policy is a decision). DISTINCT-FROM `length-prefix-validate-before-alloc` (one length field → one alloc) and `unbounded-read-into-vec` (one stream → growing buffer) — both are VOLUME faces; this is the CARDINALITY face. Real-world anchors: connection trackers, session maps, per-IP state. |
| 4 | **`gateway-type-forces-security-check`** | `interface.rs` (`Interface::receive` the only raw-datagram parser; rate limit + MAC verify + cookie under load) + `message.rs` (crate-private ctors on `HandshakeInitiation` etc.) + README §Security | **C+/B−** | sec suggest-only, architectural: security-relevant handler types whose message types carry PUBLIC constructors let callers synthesize inputs and bypass the gateway's checks (rate limit, MAC verification, replay latch) — the checkpoint exists but is skippable. Fix: crate-private ctors + the gateway type as the only producer (`Interface::receive` → `ReceivedMessage`), so the dangerous `handle_*` is UNREACHABLE without passing the checkpoint. DISTINCT-FROM `client-ip-from-untrusted-forwarded-header` (that owns rate-limit KEY spoofing; this owns CHECKPOINT BYPASS via constructor reachability). Honest recall limits: "which handler is security-relevant" is vocabulary-gated (receive/verify/rate/handshake/auth), precision will be poor — the corpus's SSRF-rule shape (policy-class suggest-only). |
| 5 | **reject-before-expensive-sink** | `session.rs` `will_accept` ("Reject known replays before paying for decryption") | **C — expected drop** | Cheap membership/state check before the AEAD open. Adjacent-but-distinct from `batch-load-admission-sized-from-files-refuses-past-the-hard-budget` (entries_47: pre-LOAD admission over files; this is pre-CRYPTO over packets) — but the buggy direction requires INVENTING the check (nobody writes decrypt-then-replay-check), the B258 recorded-drop class. Hold for the batch's T0 adjudication; conversion only if a parse-shaped trigger emerges. |

**Drops (recorded, with reasons):**
- **Sliding replay-window bitmap** (`replay.rs`, `[u64; 128]` + word-clearing on window advance) — a data-structure CHOICE, not a mechanical fix-space; the kernel_opt "sliding" hits (ring-store contiguity, load-ahead) are GPU storage senses, no CPU owner, but also no rule-shaped fix here. Algorithm-flavor.
- **Token bucket in Duration space** (`ratelimiter.rs` — tokens as `Duration`, refill by elapsed, `min(TOKEN_MAX)` cap) — algorithm flavor, niche; the corpus's rate-limit surface is owned by rule 3's class.
- **Sans-IO doctrine** (no internal buffering, time/RNG injected, no locks in protocol state) — architectural posture → skill-checklist material, never a corpus rule (the B258 architectural-bulk precedent).
- **GC-behind-clock-gate** (`gc()` at most once per `IDLE_TIMEOUT`, `extract_if` drain) — invent-machinery; zero rust_perf owner (greps `amortiz|extract_if` → kernel_opt only) but the static trigger is absence-of-GC — weak. Context note inside rule 3's fix space if the batch wants it.
- **V6 /64 source aggregation + `to_canonical`** (`Source::V6(upper 64 bits)`) — protocol-specific keying decision.
- **TAI64N timestamps, ring_buffer, timers** — protocol encoding / infrastructure plumbing.

## Dedup-gate evidence (greps run 2026-11-03 at corpus state rust_perf 292 / sec 35 — both re-derived live this session, matching AGENTS.md)

1. **batch-reserve/offload:** `consecutiv|counter reserv|sequence number|monotonic counter|batch.*offload` over rust_perf corpus → hits are the certifying-token family (B283: a builder-minted uid counter gating a VALIDATION SKIP — different axis: skip-revalidation vs offload-bulk) + bounded-peer-handoff-wait (wait bounds, not counters). `sans.io|worker thread|off-thread` → one entries_06 device-polling note. **Zero owner.**
2. **zeroize:** `zeroize` over ALL domains → **zero hits** (confirmed twice).
3. **unbounded keyed state:** `rate.limit|token.bucket` → only `client-ip-from-untrusted-forwarded-header` (key SPOOFING — distinct class, cited as distinct-from). `amortiz|extract_if` → kernel_opt only. **Zero owner for the cardinality face.**
4. **gateway/ctor:** `gateway|crate-private|private constructor` over sec + rust_perf → **zero hits.**
5. **reject-before:** `before pay|before the expensive|precheck|pre-check` → entries_47 `batch-load-admission-...` (file-loading admission; different surface) + kernel_opt quantized-kernel notes. **No crypto-sink owner** — but the candidate's own weakness (invented check) is the likelier killer, not dedup.
6. **checked arithmetic:** sec owns `unchecked-wire-arithmetic`; sanstun's `reserve_counters` USES the checked idiom — a positive instance of the existing rule (recorded, never re-owned); the NEW rule is the batch+offload shape above it.

## GOAT candidate identification (the skill's four questions, on candidate 1)

1. **Benchable?** NO — the win is parallel throughput (owner-thread bookkeeping 2% @1280B, bulk crypto offloaded); the Pareto validator hosts single-process latency + alloc counts. → **NotBenchable-by-domain annotation**, the established coordination-law class (Batch 136 rayon; entries_43-46 splash serving pool; B283). The serial component is benchable but trivial — recorded in the annotation, never a bar.
2. **Exact semantics achievable?** YES — integer counters, fixed nonces; the receiver's replay window tolerates reorder/gaps (the regime condition, stated in-rule). Fixture reproduces the two regimes exactly.
3. **Magnitude story?** YES, mechanism-level and measured (source-reported, provenance never bars): per-item synchronized reservation vs one range reservation + thread offload; the blog's own 45x ceiling hedge carried as the honest upper bound.
4. **Deterministic?** YES — same inputs, same reservations, no timing-dependent verdict in the fixture.

Criteria 2–4 pass; criterion 1 fails for a PATTERN-DOMAIN reason (coordination law) → per-rule annotation, grade on decision content × novelty × dedup × yield. **The verdict stands YES.**

## Lane-intel axis — N/A, stated

The league durable record + open lanes are Metal/CUDA GDN, prefill/decode, quant, ANE offload — WireGuard's data path shares no mechanism with any of them. No `perf_rematch` keyword maps (`prefill|decode|quant|ANE|GDN` → nothing in the source's mechanism set). Recorded N/A, not skipped — this verdict's value is entirely corpus-side.

## Product-coverage axis — MODERATE, one family named

- **Which ecosystem:** `zeroize` (RustCrypto) — candidate 2's rule EXTENDS the crypto-hygiene family into the secret-lifetime axis; `zeroize` is RustCrypto-wide (every secret-bearing crate touches it), so the rule's consumer population is large. `ring`/`x25519-dalek`/`chacha20`/`subtle` are the dep set — `constant-time-secret-compare` already owns the subtle face.
- **Family size today:** sec crypto-hygiene = 3 rules (weak-hash-to-blake3, password-hash-argon2, constant-time-secret-compare). Candidate 2 is the 4th member and the family's first LIFETIME rule.
- **Follow-up source pool** (accumulation clause applies — pool sources read before the batch window fold into the same batch): RustCrypto `zeroize` README/FAQ (the Deadline/AutoZero docs), `secrecy` crate (`SecretBox` + the Debug-leak class — a natural 5th family member: a secret type with a derived `Debug` leaking contents), RustCryptoUtils docs on zeroization strategies, Tony Arcieri's secret-management writeups. The `client-ip-from-untrusted-forwarded-header` rule shows the web-trust rate-limit surface is partly owned; the gateway rule (candidate 4) extends the trust-boundary family toward API-design-enforced checks.

## Caveats (honest)

- **Provenance chain:** sanstun ← firezone/boringtun (deprecated fork) ← cloudflare/boringtun. The upstream WireGuard spec (whitepaper) owns the protocol math; sanstun owns the sans-IO API shape + the batch reservation (tip commit #129 is sanstun-original, 2026-11-03 — brand new, the freshest possible mechanism surface).
- **Crate-private constructors claim** rests on the `message.rs` grep (only `pub(crate) fn new` seen) + the blog's explicit statement ("the constructors for types like `HandshakeInitiation` are all crate-private"). Re-verify at mine time in the T0 clone.
- **Unread files** listed above; the handshake/crypto internals were not graded — if the batch's T0 finds a kernel-adjacent numeric pattern there (BLAKE2s usage, HMAC), it adjudicates fresh.
- **Tip moved 2026-11-03** (#129 batch reservation) — the pin is fresh but fast-moving; re-clone + re-pin at T0 is mandatory, quotes re-verified per the standing law.
- Consumer-base narrowness: 101 stars, but Firezone is a production vendor and the fork lineage carried >70 patches — quality is not in question.

## Yield estimate

**3-4 rules land after ~50% coordinator falsification** (candidates 1–4; candidate 5 expected drop at T0). Corpus targets: rust_perf 292→293 (concurrency +1), sec 35→38-39 (crypto-hygiene +1, input-validation +1, web-trust/trust-boundary +1, all suggest-only + NotBenchable-annotated).

## Recording

Per the skill's YES path, one commit: this note + `.plans/332_sanstun_distill_batch.md` (plan-first, before any corpus edit) + the queue line in `.distill/001_mining_queue_snapshot.md`. Highwaters: `.research` 363→364, `.plans` 331→332. `.raw/sanstun` removed at the commit. Dual-allocation gate re-run at the landing.
