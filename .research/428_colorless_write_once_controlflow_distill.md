# Research 428 — veecore/colorless-rs: Colorless Write-Once Controlflow — Distill Verdict

**Status:** DISTILLED — batch-377-landed (e41a514f3, 2026-11-22, zed-m3@m3 — rust_perf 382→384, entries_91: R1 `sync-async-twin-duplication` + R2 `box-dyn-future-in-hot-path-colorless` KEPT at the drop-or-keep gate; R3 stays the recorded DROP per the verdict; fixtures b394 7/7; the pre-registered BOX-STATE gate routing self-resolved — bmo's Tailscale dropped at gate time and amo's queue held the job behind Exclusive lanes, the gates ran locally on m3 at load 2.15/16 after the sibling's test drained, disclosed in the landing receipt)

**Source:** github.com/veecore/colorless-rs @ `b5db6b95cf2f18efbb12fabe5709ac003fcde622` (shallow clone `.raw/colorless-rs`, REMOVED at verdict close) + the users.rust-lang.org show-and-tell (topic 143702, posted 2026-11-16). MIT OR Apache-2.0 (LICENSE-MIT/LICENSE-APACHE/LICENSE-ZLIB in-tree), edition 2024, rust-version 1.95, workspace version 0.1.0 "Unreleased", **NOT on crates.io** (pre-alpha; keywords `["async", "cps", "function-coloring", "zero-cost"]`, categories asynchronous/concurrency/no-std/rust-patterns — all read at the pin from `Cargo.toml`).

**Inspection date:** 2026-11-21 (M3 docs-only lane, zero cargo zero GPU; user-directed @research × @distill).

**Verdict gate:** Claude reviewer session `e4234f17-2ab9-4f08-8cb5-d13a2fe2be97` — round 1 AGREE with binding conditions (dedup-claim correction, R1 body-test trigger, R2 widened dedup + anti-regimes), round 2 REVISE wording-only (Batch 376/377 taken ⇒ **nominal Batch 378**; 094 scan = population evidence not firing evidence; "no new rule fired" struck) — all applied with live verification (snapshot L2829/L2831 Batch 376 claim+landing read; plan 393 header read). Round 3 confirmation not run: reviewer session expired at re-call (cap not hit; the round-2 reviewer instruction was "fix them and land"). Recorded per the expiry clause.

## Verdict

**YES B− corpus (rust_perf; nominal Batch 378, Plan 394 staged unclaimed, re-derives at landing HEAD) / LANE INTEL none / PRODUCT COVERAGE FIRED.**

- **Routing:** `rust_perf`, NEW family "colorless controlflow / write-once sync-async" (concurrency-adjacent). NOT kernel_opt (zero kernel code — the compiler half is proc-macro/ANF lowering on syn, ~3.2k LOC under `compiler/lower/src/{cps,ir}`; library traits are monomorphization plumbing). NOT clippy_lints (the fix-space is architectural refactor, not a parse-provable local rewrite).
- **Implementation mining yield: ~0.** Proc-macro/compiler code is not hot-path Rust; the value is the conceptual family + the problem patterns it names.

## The mechanism (what the source IS)

1. **Color-parameterized control-flow traits** (`crates/library/core`): a function generic over `C: Color` returns `impl Colored<C, Output = T>`. Sync color `Identity` inlines combinators to direct calls ("Step transitions compile away into ordinary sequential instructions, and loops become native CPU jumps. No generator state is allocated, and no poll functions are registered." — `core/src/lib.rs` module doc, at the pin); async color `Async` lowers to "compact, unboxed enum state machines" with "memory layout grows linearly rather than exponentially". Built-in colors: `Identity` / `Optional` / `Fallible<E>` / `Async` / `FallibleAsync<E>` (+ "rainbow" feature extras), all dyn-safe, `no_std`.
2. **Sealed unsafe drive protocol** (`colored.rs` `ColoredInternal<C>`): destructive polling — `Poll::Ready` transfers the drop obligation to `finalize_ready` ("Returning normally or unwinding from `finalize_ready` leaves `self` logically dead"); readiness/panic cleanup travels the call stack instead of a completed tag per composed state machine.
3. **`#[colorless]` proc-macro** (`crates/compiler/{lower,codegen,macros}`): lowers the ONE written body (ANF/CPS IR) into per-color carriers; `.resolve` is "the color-generic equivalent of `.await`" (desugars to `C::map(step().into_colored(), |v| v)`). **Inert verification**: "The macro emits an unreachable inert copy of the relevant source code. Rust's normal borrow checker validates its references, moves, and ordinary source lifetimes" (`SAFETY.md`) — with the explicit caveat "It is *not* the complete unsafe proof" (five additional codegen obligations follow).
4. **⚠ Relaxed implicit destruction model** (their own docs): "A local binding that crosses a `.resolve` point may be dropped earlier or later than the lexical end of its enclosing scope. Sibling locals may be dropped in an order different from standard Rust reverse-lexical drop order." (README); CHANGELOG "Known limitations": "Exact lexical destruction timing and sibling destruction order are not preserved by macro-generated continuations." Strict lexical RAII is 📋 roadmap. **Do-not-adopt-until-RAII-lands hazard recorded — the corpus rules below cite the PROBLEM patterns and their remedies, never `#[colorless]`-specific advice.**
5. **Maturity (README matrix at the fetch):** direct calls/`.resolve` ✅ · conditionals ✅ · **loops 🔄 guided** (need `#[loop_state(...)]`) · local pinning 🧪 experimental (`pin_local!`/`borrow_mut!`/`reborrow!` carriers, `colorless-rt`, audited in `SAFETY.md`) · strict lexical RAII 📋 roadmap.
6. **Source-reported figures (PROVENANCE, never bars):** Identity path vs boxed dyn-Future baseline 2.06–2.5×, vs naive poll 1.34–1.53× (facade doc-queries; foreign benchmark, foreign codegen, never our bar).

## Dedup evidence (both vocabularies, live greps this session)

- Corpus: `twin|dupl|_sync.*_async|sync.*and.*async` over `src/domains/**` → only codegen rewrite-pair hits (unsigned/mut twins, `_mut` twins) — **zero owner of the sync/async controlflow axis**. `block_on|Box<dyn Future|boxed.?future|box.*future` → fixture content + the future-SIZE family only (`async-future-size` entries_02, `async-wrapper-future-size-exponential-nesting` entries_79) — **no erase-boxing-on-a-generic-boundary rule**. Named-async-rule census (`async|await|block_on|future|spawn_blocking` over rust_perf rule_ids): `async-yield-point`, `async-future-size`, `tokio-guard-across-await`, `blocking-sleep-in-async`, `async-wrapper-future-size-exponential-nesting`, `inline-actor-futures-pool-not-spawn-per-actor` — every one owns the RUNTIME-COST axis (yield points, state-machine size, guards, blocked workers), none owns the write-once axis.
- Workspace: `colorless|function.?coloring|write.?once|maybe.?async|colou?red.?function|ColoredFn` over every `**/.research/**` and `**/.plans/**` → zero relevant hits ("write once" hits are δ-Mem writes and KAT ledger write-once rows — unrelated senses). `colorless|ColoredFn|maybe_async` over `**/*.rs` → zero hits.
- **Population evidence (not firing evidence):** `maybe-async-0.2.11` sits in the corpus's mined population — `.benchmarks/data/094_t03/manifest.json:3340` and `094_t04/manifest.json:3450` (verdict ok, 32 warnings from rules already in the corpus at the 094 scan; whether R1 fires on it is a T0 measurement — R1 did not exist at that scan).
- Published prior art (§4): the CLASS is published — keyword-generics initiative (lang-level write-once), Zig colourless-async internals thread (2021), "In Defense of Async: Function Colors Are Rusty" (2022-01-03), "The Case for Inverse Coloring" (2026-01-11), the `maybe_async` / `async-generic` crates. Dedup was INTERNAL and it is clean.

## Ranked candidate table (all suggestion-only, NotBenchable-by-domain — criterion-1 domain miss: the win metric is maintenance-drift, the dial9/B136 annotation class)

| # | Rule | Grade | Notes |
|---|---|---|---|
| R1 | `sync-async-twin-duplication` | **B− family seed** | Paired `_sync`/`_async` twins (or blocking/nonblocking trait-impl pairs) duplicating control flow. TRIGGER (reviewer-conditioned): the name-suffix pair **PLUS a body test** — ≥2 duplicated control-flow statements; the single-expression DELEGATION wrapper is the anti-regime (the good pattern — `blocking-sleep-in-async` entries_06 already names it "the async twin"). Fix = suggest color-parameterized implementation; cited remedies: `maybe_async`, `async-generic`, hand-rolled `Colored<C>` trait, keyword-generics watch. Distinct-from `blocking-sleep-in-async` in-rule (call-site runtime defect vs repo-shape). NotBenchable-by-domain (maintenance axis). |
| R2 | `box-dyn-future-in-hot-path-colorless` | **C+/B− DROP-OR-KEEP** | A hot path returning `Box<dyn Future>`/erased futures where the consumer is generic over the caller's context. T0 dedup widened (reviewer-conditioned): entries_02/79 **+ Batch 363's `async-wrapper-future-size-exponential-nesting`** (its preflight boxes `Pin<Box<dyn Future>>`). ANTI-REGIME names the legal-erase cases (reviewer-conditioned): recursive futures, dyn-trait object boundaries, deliberate monomorphization/compile-time-bloat control. If the bounded shape fails at authoring, R2 drops. |
| R3 | `#[colorless]`-specific rules | **Expected DROP** | Zero real-world consumers (pre-crates.io), firing rate ~0. The generic guard-across-suspension class is owned by `tokio-guard-across-await`. Rules quote problem patterns only. |

**Yield:** 1–3 rules + 1 expected-drop after ~50% falsification. Faces: 0 expected (maybe 1 rider).

## Lane-intel: NONE

No open league lane maps — the workspace serving edges are deliberately std-only-sync (`riir-reflex` serve.rs, `riir-instinct` serve_edge.rs, riir-refine's hosted-expert/seat transport lanes). WATCH note, no filing: the `kat_transport` native-ureq vs CF-worker-fetch split is a real sync/async boundary in the stack, but no open issue asks for colorless-style unification and adopting an alpha relaxed-drop macro library into the KAT network is refused on the hazard above.

## Product-coverage: FIRED

Ecosystem-wide problem (tokio-era Rust: every protocol codec, embedded driver, and dual-API library faces it), family zero owners in the corpus. **Follow-up pool (accumulation clause active — pool sources read before the batch window fold into the same batch):** the `maybe_async` crate + its README's rewrite approach, the `async-generic` crate, keyword-generics initiative docs (announcement + progress reports), "In Defense of Async: Function Colors Are Rusty", the Zig colourless-async internals thread, "The Case for Inverse Coloring", cps/genawaiter-class crates.

## Caveats

- **Alpha, pre-crates.io, relaxed-drop by design** — the note and any landed rule carry the do-not-adopt hazard; the source's speedup figures are provenance, never bars.
- Fast-moving + zero-publish-history: re-clone + re-pin at T0 (the pin above is the verdict's provenance).
- Implementation unread at grade (compiler/lower internals 3.2k LOC skimmed for shape only, never quoted): re-read rows belong in Plan 394 T0 only if R2 survives and needs mechanism depth.
- NOT ML: no research-skill training/modelless classification applies; the routing home is riir-refine (fusion priority #1 — the rulewright corpus is the consumer surface). No katgpt-rs primitive exists and none is claimed.

## Recording (this commit)

`riir-refine/.research/428_colorless_write_once_controlflow_distill.md` (this file; highwater 427→428) · `riir-refine/.plans/394_colorless_controlflow_family_batch.md` (nominal Batch 378, STAGED UNCLAIMED — claim-before-edit, 48 h lease, push is the arbiter; highwater 393→394) · the verdict row at the top of `.distill/001_mining_queue_snapshot.md` · dual_allocation gate at the landing commit · `.raw/colorless-rs` removed at close. Corpus counts cited: rust_perf 380 / sec 80 / ane 13 / kernel_opt 1093 (post-B376 landing 2026-11-21; re-derived at the batch's own landing).

NOTHING INVENTED — every quote read at the pin this session. Session: zed-m3-colorless@m3, epoch 1795227779.
