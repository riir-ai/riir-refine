# Plan 327 — Batch (nominal): uniffi-rs FFI hygiene → sec

**Status:** STAGED — distill verdict 361 (2026-11-02); claim per the queue before any corpus edit
**Claimed:** zed-amo-hkg327, 1793638171, expires 2026-11-04T16:49:31+00:00 (the queue-snapshot claim row is the fuller scope record)

Distill verdict: `.research/361_uniffi_rs_ffi_hygiene_distill.md` (YES B− corpus, mozilla/uniffi-rs @ `ef9625b68dd51207f55f024a27649e58edb52b09`, MPL-2.0). This batch converts the FFI-hygiene candidates into sec-domain corpus rules. ALL candidates are NotBenchable-by-domain (correctness/liveness wins, per the dial9 split) — the batch's proof path is the fixture falsification arms + the integer-exact refcount ledger, not a Pareto bench.

## Phase 0 — provenance + slot

- [ ] Re-clone at the pin: `git clone --depth 1 https://github.com/mozilla/uniffi-rs.git .raw/uniffi-rs` + `git rev-parse HEAD` must equal `ef9625b68dd51207f55f024a27649e58edb52b09` (moved HEAD = re-pin + re-verify every quote at the new sha BEFORE authoring)
- [ ] Claim the batch in `.distill/001_mining_queue_snapshot.md` (claim row + 48 h expiry; push is the arbiter); re-verify no live foreign claim owns the sec slot (the B293 cross-domain precedent)
- [ ] `py ../katgpt-rs/scripts/dual_allocation_gate.py` at the landing (0/0/0 expected)

## Phase 1 — dedup re-verification at the slot

- [ ] Re-run both-vocabulary greps per candidate against the corpus AT THE LANDING HEAD (361's zero-hit evidence is dated): `waker|raw_waker|increment_strong|Arc::from_raw`, `catch_unwind`, `ManuallyDrop|MaybeUninit`, `sentinel|null`, `lock.*callback|invoke`
- [ ] The two flagged adjudications are LOAD-BEARING at authoring: #2 `ffi-callback-invoke-outside-lock` vs `host-lock-across-reentrant-blocking-call` (same-shape-different-failure-mode — the in-rule adjudication note is mandatory, the tokio-guard precedent cited); #5 `null-sentinel-companion-conjunction` vs `ptr-deref-null-check-first`/`slice-from-raw-parts-len-guard` (unary vs conjunction — author as the conjunction or demote)
- [ ] Read the unread-at-verdict surfaces if any rule's trigger rests on them: `ffi_converter_traits/{,_impls}.rs`, `ffibuffer.rs` tail, macro `export/*.rs`

## Phase 2 — authoring (target 3–4 sec rules + 1 stretch)

- [ ] `arc-refcount-across-raw-boundary` (A− lead): fixture = raw Arc pointer crossing an unsafe boundary (a hand-rolled vtable-data-pointer or FFI-handle stand-in); buggy arm = `from_raw` round-trip + forget on a borrowed use / double `from_raw` on consume; fixed arm = `increment_strong_count` for borrowed, `from_raw` exactly once for consume; the discriminating test is the refcount-ledger assertion (`Arc::strong_count` at every step) — integer-exact, deterministic; NotBenchable annotation
- [ ] `ffi-callback-invoke-outside-lock` (B+): fixture = mutex-guarded state + a callback that re-enters via a second (foreign-side) lock vs extract-under-lock/invoke-after-release; carry the cross-FFI ABBA regime + the pure-internal anti-regime; NotBenchable (hang class, entries_08 honesty)
- [ ] `out-param-manuallydrop-not-maybeuninit` (B): fixture = repr(C) out-param caller-initialized (ManuallyDrop, safe reads, drop discipline) vs MaybeUninit (unsafe reads); carry the set-twice=leak single-writer contract; suggestion-grade
- [ ] `panic-handler-nested-catch-unwind` (B−): fixture = an error-reporting handler doing fallible format/alloc work with no nested guard vs its own catch_unwind + graceful degradation; distinct-from notes for ffi-extern-fn-catch-unwind (body wrapper) + drop-impl-no-panic
- [ ] Stretch: `null-sentinel-companion-conjunction` (B−) — ONLY if authored as the conjunction invariant (null ∧ len==0 ∧ cap==0 legitimate; companion-nonzero corrupt); else record as a positive-instance face on the unary guards
- [ ] Stretch: `vec-promote-manuallydrop-not-forget` (C+) — thin; land only if room remains after falsification
- [ ] Every entry: NotBenchable annotation per the dial9 split; kinship cites recorded (ffi family + host-lock + nonmax-sentinel); fixtures ORIGINAL minimal reproductions — never paste uniffi source verbatim (license hygiene + binary-only distribution rule)

## Phase 3 — gates (isolated CARGO_TARGET_DIR)

- [ ] Batch needles + parse gate test (new file needles wired into the batch test)
- [ ] `cargo test --lib` default posture (count floor = current + landed)
- [ ] `cargo test --lib --features sec` posture (sec is opt-in — the gated-posture rule)
- [ ] `cargo clippy --all-targets -- -D warnings` × postures (default / sec / no-default / all-features)
- [ ] Retrieval floors both postures
- [ ] Freeze/score-bench per the P30 rule at landing time (sec-only delta — verify against the standing rule)
- [ ] Maintained binary rebuild + install owed only if sec joins the release set (it is NOT — verify the release-set rule at landing; likely not owed)

## Phase 4 — count pins + docs

- [ ] Corpus len witness sites re-pinned (tests.rs asserts + the sec count pins + mod.rs/doc headers if slices move)
- [ ] AGENTS.md sec Role bullet + README + `.docs/11_domains/sec.md` batch line
- [ ] `.research/361` status line → DISTILLED — batch landed (or the honest partial with the falsification record)
- [ ] Queue snapshot landing row + close the claim; remove `.raw/uniffi-rs`

## Phase 5 — commit + push

- [ ] One commit: corpus + tests + pins + docs + note status + queue row (`feat:` prefix; Session line in the body)
- [ ] Push develop; verify CI posture (sec is opt-in — no CI lane fires; local gates ARE the verdict)
