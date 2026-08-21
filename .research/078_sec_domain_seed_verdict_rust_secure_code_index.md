# Sec-Domain Seed Verdict — the rust-secure-code/projects index (all 39 links inspected)

**Status:** RECORD — Phase 0+1 LANDED 2026-08-22 (Plan 018 complete: 6 rules shipped / 5 killed at mine-time falsification; see Research 079 + Bench 046).

- **Source:** https://github.com/rust-secure-code/projects (the WG's index — single README link list, 268★, GPL-3.0)
- **Inspected:** 2026-08-20, 4 parallel lanes (fuzzers / analyzers / supply-chain / input+side-channel+guides), every active link entered; 2 N/A skips (compiler sanitizers, org repos), 1 not-fetched (rust-vmm community — no code content by category)
- **Verdict on the index repo itself:** F as a direct mine (zero code). **Verdict on the link graph: B+ domain-opening evidence package** — the strongest single survey the distill program has produced for a new domain.

## The headline findings

1. **ANSSI Secure Rust Guidelines (ANSSI-FR/rust-guide) — SEC_SEED A−.** ~44 formal rule items; ~14-18 lint-shaped (~35%), of which **~10-12 are novel vs rustc/clippy** (verified against `rust-clippy@master`: the restriction group owns panic/arith/indexing but has ZERO crypto/constant-time/FFI-boundary lints). Top candidates: `ffi-extern-fn-catch-unwind` (in-house precedent: riir-viewbridge's `#[riir_ffi]` enforces exactly this), `ptr-deref-null-check-first`, `drop-impl-no-panic`, `ffi-fnptr-type-unsafe-extern`, `ffi-by-value-no-drop-type`, `ffi-portable-c-alias` (c_long/size_t), `rc-cycle-interior-mutability` (Weak backedge), `extern-fn-reference-param-to-pointer`, `no-box-leak`, `maybe-uninit-requires-justification`.
2. **Rudra (sslab-gatech/Rudra, archived 2026-04) — SEC_SEED B+.** Mine the TAXONOMY, never the engine (nightly-2021 rustc internals). Best: `unsafe-impl-send-sync-missing-bound` (parse-detectable, bounded fix = add `T: Send`/`T: Sync` bound or documented invariant). Panic-safety + destructor-safety analyzers are semi-bounded (guard/reorder shapes); higher-order invariant = kill (fuzzy detection, unbounded fix).
3. **Oracle ladder SOLVED** (the blocker my first pass flagged): cargo-geiger B+ (deterministic unsafe-count delta — the clippy_verify shape; granularity verified in `geiger_syn_visitor.rs`: 5 counters, incl. the attribute-unsafety nugget `#[no_mangle]`/`#[export_name]` = unsafe despite safe signature), Miri A− (documented UB list IS a rule taxonomy + buggy-aborts/fixed-passes = the G1 gate shape), dudect-bencher B+ (constant-time, statistical → release-gate class), Kani B+ (bit-precise integer rules, slow), loom B− (atomic-ordering rules only), cargo-fuzz B / fuzzcheck B− (crash + minify oracles).
4. **advisory-db — SEC_SEED C.** ~2-4% `unsound` class describes source patterns (e.g. RUSTSEC-2021-0145 atty unaligned read); ~90% version-bumps we must NOT duplicate (cargo-audit/cargo-deny own that — DRY). Category taxonomy (`crypto-failure`, `memory-exposure`) = sec vocabulary donor.

## Consolidated per-link verdicts

| Link | Bucket | Grade | One-line |
|---|---|---|---|
| ANSSI rust-guide | SEC_SEED | **A−** | ~10-12 novel lint-shaped rules; best seed found |
| Rudra | SEC_SEED | **B+** | Send/Sync variance taxonomy; mine taxonomy only |
| l3x | SEC_SEED | C | ~6 bounded rules; several regexes broken; LLM half violates modelless mandate |
| dangerous | SEC_SEED | C+ | full-consumption/consume-exact idioms as pattern texts |
| untrusted | SEC_SEED | C | regime vocabulary donor (zero-panic checked reads) |
| advisory-db | SEC_SEED | C | unsound class ≈2-4% source patterns; rest = cargo-audit's job |
| Miri | ORACLE | **A−** | UB list = rule taxonomy + oracle (nightly, slow) |
| cargo-geiger | ORACLE | **B+** | unsafe-count delta validation + attr-unsafety nugget |
| Kani (was rmc) | ORACLE | B+ | bit-precise overflow/panic/UB; needs harnesses |
| dudect-bencher | ORACLE | B+ | practical constant-time gate; statistical |
| cargo-fuzz | ORACLE | B | crash/UB + minify regression oracle |
| fuzzcheck | ORACLE | B− | typed property + auto-minify; zero-alloc mutation = existing make-unmake rule (dedup) |
| sidefuzz | ORACLE | B− | deterministic instruction counts but wasm-only, stale |
| loom | ORACLE | B− | atomic-ordering rules; shim-type limits scope |
| cargo-audit | ORACLE | C+ | exit codes deterministic only with pinned db revision |
| siderophile | ORACLE | C | callgraph ranker, "not guaranteed complete" |
| ctgrind | ORACLE | C | removed from RustCrypto/utils; unmaintained |
| guppy | RUST_PERF | C+ | bitset reachability precompute / cached-len / shared SCC (archived 2022; the soa-hot-field class — suggestion-grade) |
| scudo, mimalloc_rust | DROP | D | allocator bindings; GlobalAlloc wrapper config |
| cargo-deny | DROP | D | config-driven dep linting — the DRY kill example |
| crev, cargo-vet | DROP | D | trust/review data formats |
| rustsec-crate | DEAD | D | 404 → merged into RustSec/rustsec workspace |
| crates-audit | DROP | D | stale GitLab Pages placeholder |
| cargo-libafl | DROP | D | self-declared superseded/dead |
| honggfuzz-rs, afl.rs | DROP | D | engine bindings; process guidance |
| quickcheck, proptest | DROP | D | test-engine internals; proptest README itself refutes alloc-free shrinking (~10× slower) |
| rusty-radamsa | DROP | D | mutator port, generator-only API |
| rutenspitz | DROP | D | test DSL |
| MIRAI | DROP | D | engine; archived 2024 |
| Prusti | DROP | D | engine + spec authorship = process |
| crucible | DROP | D | **Haskell** symbolic-exec library |
| semval | DROP | D | pure API adoption |
| sanitizers / wg repos / rfmig / rust-vmm | N/A | — | compiler built-in / org repos, no code |

## Do-not-reimplement list (clippy restriction group already owns)

`mem_forget`, `indexing_slicing`/`missing_asserts_for_indexing`, `arithmetic_side_effects`, `unwrap_used`/`expect_used`/`panic`, `improper_ctypes(_definitions)`, `partialeq_ne_impl`, `undocumented_unsafe_blocks`, `disallowed_methods` (can BAN sha1/md5 but offers NO fix suggestion — that delta is exactly the sec domain's niche, incl. the owner's standing blake3-over-SHA1/SHA256 + argon2-for-passwords rules).

## Yield estimate + the bar

First-batch realistic: **8-12 rules** (ANSSI 10-12 novel − ~50% coordinator falsification + Rudra 2-3 bounded). Domain total: **15-20** with a second source (Lane D's suggestion: RustCrypto conventions). The ~20-rule scaffolding bar is REACHABLE but not cleared by this survey alone — the Proposal must pin the count by authoring the actual candidate list.

## Honest caveats

- Rudra/l3x taxonomies were enumerated from README + 1-2 source files; re-verify at mine time (archives drift).
- Miri/Kani/dudect are heavyweight oracles — the Proposal must decide which rules genuinely need them vs compile+semantics equivalence (the clippy_verify shape); dudect is statistical → release-gate class, never CI-blocking.
- Consumer pull evidence: in-house precedent (`#[riir_ffi]` catch_unwind enforcement in riir-viewbridge; the owner's standing crypto rules in AGENTS.md; riir-chain/riir-neuron-db security surfaces) — but no workspace repo currently runs a sec heal. Pull is plausible, not measured.
- Cost: Proposal + `HealDomain` extension + corpus + proposers + validator + docs = a Plan-001-scale effort.

## Queue state

Sec mining is **BLOCKED on the domain-existing precondition** (Proposal + `HealDomain` trait extension per Plan 001). This note is the seed inventory for that Proposal. Do not open a sec batch before it.
