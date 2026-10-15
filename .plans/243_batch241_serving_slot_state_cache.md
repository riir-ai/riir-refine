# Plan 243 — Batch 241: the serving slot-state cache pack (P71/P72 own rules + P73 rider)

**Status:** IN_FLIGHT — claimed 2026-10-15 (claim row in `.distill/001`, pushed plan-first)

Source: ggml-org/llama.cpp, envelope delta `47f5a64ae` → `620d2225cc436ab0b001e3200e590e3b286ef9ab`
(14 commits graded; 3 convertible, rest NULL — version bumps, model registrations, CI, UI,
jinja C++ copy-ctor with no law). Read at the pins via the fetched F:/llama.cpp clone (fetch
`a0e6dc73..620d2225`, this session).

## The leads

- **P71 RULE-GRADE B+** — #30580 @ `61ad80bf93de0ac03a29f83f5f3e9f596bf40c2d` "server: fix
  the off-by-two in the SWA reuse threshold". THE LAW: a state-reuse admission threshold
  must equal the EXACT bound the attention semantics guarantees — "The first evaluated
  token attends to the positions above pos - n_swa, so a SWA state starting at
  pos - n_swa + 1 holds the whole window, but the reuse threshold asked for two more
  cells." A defensive margin (even +2) makes "a state read back from a slot file or from
  the RAM prompt cache ... always rejected and the slot rolled back to a checkpoint" —
  a cache that NEVER hits by construction (silent perf cliff, not correctness).
  "The SWA threshold now matches the window, while a state without SWA keeps the exact
  bound." + the harness instrument (`override_kv` — give the SWA test model a window
  smaller than the test context). Route: rust_perf, category caching. Dedup: P65 owns
  the partial-serialization contract, B219 owns keying/eviction — the REUSE-ADMISSION
  predicate is unowned.
- **P72 RULE-GRADE B** — #30567 @ `b0d239164b26adcd317be1f0cffd2a1294f6ba60` "server:
  prefer empty slots and keep states overwritten by child tasks". TWO faces, one law
  pair: (1) victim/selection ordering — "Free slots are now ordered empty first, then
  least recently used, for both the LRU fallback and the child slots" (an empty slot
  costs nothing to take; every occupied-slot policy — decayed-frequency,
  expert-cache-regime, B219 rewind-protection — orders among OCCUPIED only);
  (2) save-before-overwrite — "a child task saves the state it overwrites" (the
  displaced parent state is persisted to the prompt cache before the clobber).
  Measured: "With 4 grouped decision requests in flight and 12 states in rotation,
  prompt cache reads go from 5 to 36." Route: rust_perf, caching.
- **P73 RIDER-GRADE B− (count-neutral)** — #30574 @ `ea736c1d5c1c2cd0489abe788bc3a1859af5dc0b`
  "server: save the draft state in the slot file". Rider on entries_27
  `partial-state-serialization-honors-rollback-contract-at-leaf-cache` (P65, B239):
  the state-stream COMPLETENESS face — "The slot file held the target context and the
  context checkpoints, but never the draft context, so a restored slot drafted with an
  empty or stale draft KV until the cache refilled. The file now ends with the live
  draft state, the full draft context and the speculative state, in the same appendix
  style as the checkpoints." + the degrade arm: "A draft state that does not fit the
  draft context is dropped and the draft starts empty, while older files and older
  servers keep working" (forward/back compat).

## Tasks

- [ ] Re-verify every quote at the pins (full .diff reads of the three commits in
      F:/llama.cpp — never trust the staging read's transcription)
- [ ] Re-run dedup greps at authoring time (reuse threshold / empty-first / save-on-
      overwrite / draft-state persistence)
- [ ] Author P71 + P72 as OWN rust_perf rules (caching; NotBenchable annotated —
      serving contracts, no Rust-side A/B axis, the P65/P66 precedent) + P73 as a
      count-neutral rider on entries_27
- [ ] G1: batch241 needle + pseudocode-parse gate (both halves parse ×3 rules)
- [ ] G2: lib tests at default + rust_perf postures
- [ ] G3: clippy `-D warnings` at default + all-features + no-default (heal doc_markdown
      via `cargo refine --fix --write --verify` — the B192 class)
- [ ] Retrieval floors re-measure (`--features rust_perf,latent_retrieval --lib
      retrieval` — B239 precedent; floors 21/21 must hold, no silent re-pin)
- [ ] Freeze/score-bench: NOT owed (zero clippy-corpus delta, P30) — verify + record
- [ ] Count pins: rust_perf 203 → 205 at every pin site (tests.rs, mod.rs, router.rs
      if it carries rust_perf totals, AGENTS ×2, README, .docs/11_domains/rust_perf.md)
- [ ] Research note 273 (born-complete RECORD)
- [ ] Docs: AGENTS rust_perf bullet + rust_perf.md batch row + snapshot landing row
- [ ] Commit `feat: batch 241: ...` + push; update `.distill/001` in the same commit
- [ ] Tick this plan complete + push

## Notes

- Build isolation: `CARGO_TARGET_DIR=/tmp/riir-refine-b241`, removed post-verification.
- The corpus-string JSON-escape pass (Batch 236's write-escape lesson): one python pass
  for any inner-quote text.
- .distill/001 edits via python LF/CRLF-preserving splice only.
