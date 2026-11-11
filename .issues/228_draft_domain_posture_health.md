# Issue 228 — draft domain: the no-default/all-features posture failures are pre-existing and unfixed (clippy + 1 hard test failure + a parallel-env race class)

**Claimed:** zed-7423d19b@amo, 1794441423, expires 2026-11-13T23:57:03+00:00 — the D-b + D-c mechanical clippy rows land this session (D-a/D-d stay open, they need design work)
**Status:** OPEN 2026-11-12 — found during the issue-227 gate battery (the sec fix lane), every row baseline-proven at the then-HEAD `59f8cf50` via a detached origin worktree (`/Users/katopz/git/.i227baseline-wt`, since removed): these failures exist WITHOUT any of the sec changes. The batch-351 landing note (`.docs/11_domains/sec.md`'s Batch-351 section) honestly disclosed the clippy half as pre-existing but no issue carried the cluster — filing it here so the draft lane (or any posture lane) can drive it closed. None of the rows are sec-scoped; nothing here blocks a sec-only posture (sec + default are green).

## The rows (fix order: D-a first — a genuinely failing test, not a lint)

- [ ] **D-a: `cargo test --no-default-features --lib` FAILS at HEAD** — `draft::clippy_feed::tests::oracle_edits_decline_doc_markdown_on_command_lines` panics at `src/draft/clippy_feed.rs:1656` (`assertion left == right failed: left: 0, right: 1`). 1069 pass / 1 fail in that posture. Baseline-proven pre-existing (same panic, same counts, at the origin worktree with zero sec changes). Fix direction: read the test's expectation against the oracle's actual decline count in the no-default feature set — a `#[cfg(feature)]`-gated edit arm (doc-markdown decline) is likely compiled out under `--no-default-features` while the test still counts it (the feature-gated-count-assertion law: assert COUNTS only at postures where the arm compiles in, or gate the assertion).
- [ ] **D-b: `cargo clippy --no-default-features --lib -- -D warnings` FAILS** — `src/draft/latent_matcher.rs:386` `unused macro definition: resolve_corpus` (`unused-macros` via `-D warnings`). The macro is presumably consumed only by cfg-gated call sites (the default-features build compiles clean). Fix: `#[allow(unused_macros)]` with a reason naming the gating posture, or gate the macro definition itself under the same feature(s) as its call sites.
- [ ] **D-c: `cargo clippy --all-features --lib -- -D warnings` FAILS ×3, all `src/draft/index_cache.rs`** — `:605` `unused doc comment` on a field doc inside (presumably) a non-item position; `:471` `cast_possible_truncation` (`enc.u32(v.len() as u32)`); `:665` `explicit_iter_loop` (`for slot in a.iter_mut()` → `&mut a`). Fix: mechanical — move/reword the doc onto an item, `#[allow(clippy::cast_possible_truncation)]` with a reason (or a checked conversion), `for slot in &mut a`.
- [ ] **D-d (suite health): the all-features parallel test run trips an env/isolation race class** — 8 failures under `cargo test --all-features --lib` that ALL PASS individually: `draft::index_cache` ×3 (`g1_cached_restore_suggestions_are_bit_identical_to_rebuild`, `max_cache_mb_env_semantics`, `prune_enforces_the_byte_cap`) + `points_client` ×5 (`contribute_sends_the_folded_sorted_signed_body`, `contribute_surfaces_the_typed_not_joined_refusal`, `join_sends_the_signed_wire_and_parses_the_answer`, `service_url_env_overrides_the_default`, `typed_server_refusal_surfaces_code_and_message`). Verified live during the same battery: `service_url_env_overrides_the_default --exact` passes (5111 filtered). Mechanism: tests mutating process env (env-var overrides) run concurrently — one test's env write leaks into another's read. Fix direction: serialize the env-mutating tests (a shared `Mutex` guard or a serial test harness), or scope env reads through a helper the tests can stub — never `std::env::set_var` in a parallel test without a lock.

## Repro (all from the repo root)

```sh
cargo test --no-default-features --lib            # D-a: 1 failure
cargo clippy --no-default-features --lib -- -D warnings   # D-b
cargo clippy --all-features --lib -- -D warnings   # D-c: 3 errors
cargo test --all-features --lib                    # D-d: 8 failures; each passes with --exact
```

## Notes

- The postures where the batch-351 gates claimed green (sec + default) remain green post-227: sec 4303/0, default 3855/0, clippy -D both clean, retrieval floors 29/0 — this issue does not reopen 227.
- D-d's 8 failures were observed under a `nice -n 10`-throttled run on a loaded box; they reproduce as a class (not load-dependent — the env race is deterministic given unlucky scheduling). Re-run once single-threaded (`--test-threads=1`) to confirm the class before fixing: if all 8 pass at `--test-threads=1`, the race is proven and the fix is the serialization above.
- The draft lane owns these files; a fix lane here claims the issue per the lease law (claim row + push before implementing).
