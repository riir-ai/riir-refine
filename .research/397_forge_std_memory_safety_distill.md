# Research 397 — FORGE Lab "Memory Safety Bugs in Rust Standard Library" distill verdict

**Status:** DISTILLED — batch-353-landed (Plan 363; 4 of 4 graded candidates converted, 2 honest drops recorded in the landing note)

## Source

- URL: <https://forge-blog.github.io/posts/rust-std-memory-safety/> — Microsoft FORGE Lab blog, Yechan Bae, 2026-11-09, CC BY 4.0.
- Source class: **blog post** (no repo; no `.raw/` clone). The pin is the publication itself — every quote below was taken from the fetched page this session. Rust issue numbers cited in the post (#156501, #156491, #161018, #156951, #164243, #158265, #165076, #164904) are upstream state AT PUBLICATION; re-verify any issue quote at mine time (issues move, the blog text does not).
- Provenance chain: FORGE Lab is the Rudra team (SOSP'21, the canonical ecosystem-wide unsafe-trust analyzer). The post is their std soundness sweep found with Hyenas (codename MDASH research version). The upstream of the CLASS is Rudra + the Rustonomicon exception-safety chapter; the post is this year's measured instance set.

## What the source is

Five soundness bugs in `std`/`core` unsafe code, all reachable from safe Rust, organized under three patterns of misplaced trust:

1. **Pattern 1 — trusting code not to panic** (panic safety): `MapWindows::clone` (#156501) arms a destructor over uninitialized memory, then runs user `T::clone` before writing the slots; the clone panicking unwinds through the broken invariant → drop-in-place on uninit → UB. Fix: hold the half-built buffer in `ManuallyDrop` until fully written (leak, not UB). The post's general lesson, verbatim: *"treat every call to code you don't control, such as `clone`, `drop`, comparisons, closures, and iterator methods, as a possible early exit. Either finish the work that may panic before breaking an invariant, or make the broken state safe to unwind through: hold it in `ManuallyDrop`, or use a guard whose destructor restores the invariant, like `Vec`'s internal `SetLenOnDrop`."* Plus the `Allocator` stabilization decision (Rust 1.100, 2026-11-12): implementors must never unwind — *"unwinding out of many allocation-related methods was found to be a pervasive source of unsoundness."*
2. **Pattern 2 — trusting user-provided implementations to be correct**: (a) `str::replace` with a custom `Pattern` (#156491): the ASCII fast path assumed a one-byte `Utf8Pattern::StringPattern` must be ASCII — but `StringPattern` held `&[u8]` and `as_utf8_pattern` is a safe method any impl can call; a `0x80` byte pattern corrupted the string into non-UTF-8 and `String::from_utf8_unchecked` laundered it. Fix: the type now guarantees it (`StringPattern` holds `&str`). (b) `join` trusting `Borrow::borrow` consistency: fixed in 2020 (CVE-2020-36323 shrink path) and needed a 2026 re-fix for the GROW path (two borrows, the second longer → remaining-length underflow → heap overflow). (c) open Pattern-contract issues #158265/#165076/#164904 (searcher built for a different string). The post's lesson, verbatim: *"Call the implementation once and keep the result, validate what it returns, or encode the property in a type that safe code can't forge... If none of those work, make the trait `unsafe`."* (d) The framing contrast: C++ puts comparator-correctness responsibility on the caller (`std::sort` UB on `<=`); Rust puts it on the unsafe code (`sort_by` with a bad `Ord` panics or mis-orders, never UB).
3. **Pattern 3 — trusting the underlying system**: (a) `process::exit` from a child thread unmaps the MAIN thread's alternate signal stack (#161018 — the bug dates to 2015, ~11 years in stable; `cleanup()` assumes it runs on main; exploitation assessed unlikely, tight race). (b) `CommandExt::exec` writes the process-global `environ` under the env lock's READ side (#156951) — a 2021 RW-lock migration kept the read lock a 2018 fix had taken exclusive; two racing failed execs can leave `environ` dangling into a later `env::vars_os()`. Fix in review: call `execve`, never touch `environ`. (c) `process::exit` double-free via thread-local destructors (#164243, reported by someone else; proposed fix: POSIX `_exit`).

## Domain routing

**`sec`** — the unsafe-hygiene / panic-safety axis. The tempting wrong domain: none really — `clippy_lints` cannot own these (semantic, not parse-provable), `rust_perf`/`kernel_opt` are wrong (correctness class, not perf). The transcription law applies in reverse here: the SOURCE is Rust (std itself), the FIXTURES are consumer-shaped original reproductions — the rules must fire on the unsafe code a CONSUMER authors, never on std's own symbols (a rule matching `MaybeUninit` + armed `Drop` + `clone()` in window is a consumer shape; a rule matching `MapWindows` is nothing).

## Dedup gate (grep evidence, both vocabularies, this session @ the landing corpus)

- `ManuallyDrop|panic.safe|exception.safety|unwind` over `src/domains/**` → owners: `panic-safe-autotune-candidate` (kernel_opt — crash ISOLATION of a tuning candidate, not soundness of a broken invariant), `ffi-extern-fn-catch-unwind` + `ffi-export-panicking-body` (sec Ffi — panic ACROSS the extern boundary, containment at the boundary), `panic-handler-nested-catch-unwind` (sec PanicSafety — the handler path guarding re-entry), `drop-impl-no-panic` (sec DropLeak — double-panic IN drop glue; the inverse direction: this source's class is a FOREIGN panic exposing the unsafe code's invariant), `cli-panic-unwind-tables` (dist — binary size). **Zero owners of the invariant-across-user-call-unwind class.** Lead.
- `as_ref|Borrow::|environ|read.lock|altstack|process::exit|invariant` over `src/domains/**` → the `as_ref` hits are syn-proposer syntax (`Expr::...as_ref()`), not the class; `ffi-callback-invoke-outside-lock` (sec Ffi — cross-FFI ABBA deadlock) is lock discipline during callbacks, distinct from global-write-under-shared-lock. **Zero owners of: call-twice-consistency, derived-property-validation, process-global-under-read-lock.**
- `unsafe-impl-send-sync-unsound-field` (UnsafeHygiene) — trusting a MARKER impl's thread-safety; distinct axis from trusting a VALUE impl's consistency. Named in-rule, not a collision.
- `nan-unsafe-sort-comparator` (PanicSafety, the family's single member) — the comparator-contract instance of Pattern 2's neighborhood; the general trusted-user-impl class is unowned. Family placement for the new rules: append-only `UnwindSafety` variant, or widen `PanicSafety`'s doc — batch-plan decision (the corpus's family law: append-only, no renumber).

## Ranked candidate table

| # | Pattern | Source (post section) | Grade | Notes |
|---|---|---|---|---|
| 1 | `unsafe-invariant-panic-exposed` | MapWindows::clone (#156501) + the general lesson | **A− (GOAT lead, suggest-only)** | Named class: unsafe code breaks an owned invariant (uninit under an armed destructor; a length prefix unwritten) and invokes code it doesn't control (`clone`/`drop`/comparison/closure) before restoring; an unwind exposes it. Three bounded fixes: `ManuallyDrop` until written / drop guard restoring the invariant (`SetLenOnDrop` shape) / finish panicking work first. Regime: applies when the intermediate state is NOT drop-safe and a user-reachable panic window exists; anti-regime: drop-safe temporaries, panic=abort, no user code in the window. UPSTREAM: present (std's own fix landed — decision content only, never PR material). |
| 2 | `user-impl-call-once-keep-result` | `join` borrow-twice (CVE-2020-36323 + 2026 grow-path re-fix) + the post's opening `AsRef` example | **B+** | Unsafe/invariant-carrying code calls a caller-provided safe method TWICE and assumes agreement (same length, same bytes); the impl may differ between calls → underflow/OOB. Fix: call once, keep. Distinct-from in-rule: `read-output-trust-inherits-argument-taint` (taint direction, not consistency). UPSTREAM: present (both std fixes landed). |
| 3 | `user-impl-derived-property-validate` | `str::replace` Pattern (#156491) | **B** | Unsafe code derives a property from a user-provided return value (one-byte pattern ⇒ ASCII) and skips the check (`from_utf8_unchecked`). Fixes: validate the return, or encode in the type (`&str` not `&[u8]`), or make the trait unsafe. Detection keyed on unchecked-constructors beside trait-derived values. UPSTREAM: present (type-encoding fix landed). Merge candidate with #2 if fixture work shows overlap — but the fix-spaces differ (call-once vs validate/type-encode), so author both. |
| 4 | `process-global-write-under-shared-lock` | `CommandExt::exec` environ (#156951) | **B−** | A process-global written under the lock's READ side while peers read through the same lock; a failed restore leaves it dangling. Consumer shape: global config/env swaps, `static` under RwLock read guards. Narrower surface — likely falsification casualty. UPSTREAM: present (fix in review at publication). |
| 5 | teardown-assumes-owning-thread | `process::exit` altstack (#161018) | **DROP** | The consumer shape (teardown mutating another thread's registered state) is too thin to bound; std-specific mechanism. Recorded as lane-intel. |
| 6 | allocator-no-unwind contract | the `Allocator` stabilization note | **DROP** | A trait CONTRACT for implementors, not a consumer-code fix pattern. Recorded as annotation material for #1's context_notes. |

## GOAT test (two-class split)

Criterion 1 fails for a **DOMAIN reason**: soundness classes have no perf validator — the sec corpus is suggestion-only, NotBenchable-by-domain (the dial9/136 lane). Per-rule ANNOTATION, never a demotion. Criteria 2–4 are clean for the lead: exact semantics (UB/not-UB on the panic path, fixture-provable with `catch_unwind`-driven deterministic reproduction), magnitude story (uninit drop glue = UB, the strongest possible "magnitude"), deterministic. Grade on decision content × novelty × dedup × yield: **YES**.

## Product-coverage axis — fires, STRONGLY

Rust std is the ultimate famous lib: every unsafe-consuming repo on earth carries this surface. Census this session: `PanicSafety` family = 1 member (`nan-unsafe-sort-comparator`); the trusted-user-impl class = 0 members; FFI containment = 3 members + 2 boundary rules; `unsafe-impl-send-sync-unsound-field` is the only impl-trust rule (marker axis). This source seeds the panic-safety/invariant family AND the trusted-user-impl family. **The rules fire at home too** (the dial9 lesson — issue-ledger zero ≠ surface zero): our workspace runs heavy unsafe (riir-infer GPU kernels, katgpt-core ternary SIMD, riir-viewbridge FFI, seal-node), and `cargo refine` ships publicly.

**Follow-up source pool** (the family's canonical upstreams, accumulation clause applies — pool sources read before the batch window fold into the same batch):
- The Rustonomicon, exception-safety chapter (the class's canonical text).
- Rudra (SOSP'21) — the founding unsafe-trust taxonomy (PanicSafety/UnsafeDependency), the same team's analyzer.
- The post's cited issues: #156501, #156491, #161018, #156951, #164243, #158265, #165076, #164904, CVE-2020-36323.
- `miri`'s unsupported-behavior catalog (the executor side of the same classes).

## Lane-intel axis

NONE — security/soundness source, no perf-league mapping (checked: no mechanism keyword maps to the league scoreboard or open riir-ai perf issues). The product-coverage note above is the at-home firing claim, not a lane mapping.

## Yield estimate

4 candidates (A−/B+/B/B−) → after ~50% coordinator falsification: **2–3 rules** (lead near-certain; #2 or #3 likely; #4 the swing). Nominal Batch 353.

## Honest caveats

- Blog-only pin: no sha. Quotes are from the fetched page (2026-11-11 session); upstream issue state moves — re-verify any issue-level quote at mine time. The RULE content derives from the post's general lessons, which are stable.
- The bugs are IN std; the rules must target the consumer shape (unsafe code you author) — detector tokens must be consumer-observable (`MaybeUninit` + armed `Drop` impl + a user call in the broken-invariant window; a `Borrow`/`AsRef`-bound method called twice; `from_utf8_unchecked` beside a trait-derived value; a global write under a read guard), never std symbols.
- #4 is the weakest candidate (narrow consumer surface); #2/#3 risk fixture overlap — the plan carries the merge decision.
- Two pattern-2 issues remain OPEN upstream (#158265/#165076) — more instances of the same class may surface; the rules should be authored to survive a wider class.

## Recording

- Queue line: `.distill/001_mining_queue_snapshot.md` (same commit).
- Batch plan: `.plans/363_forge_std_memory_safety_batch.md` (nominal Batch 353) — plan-first, no corpus edit before it.

Session: amo, epoch 1794465024
