# Research 252 — stellar/rs-soroban-sdk distill verdict (4090, user-directed)

**Status:** LANDED — Batch 222 (2026-10-10, this box: BOTH candidates converted as own rust_perf rules in `src/domains/rust_perf/corpus/entries_24.rs`; candidate 2 verified RICHER at the pin — two mechanisms, the macro-side MARKER read + the keep_reachable fn-pointer form — and its plain-Rust fixture STOOD ALONE, so the own-rule-or-drop adjudication resolved OWN-RULE; counts rust_perf 196 → 198, wasm family 1 → 3, annotated 142 → 144, without_spec 97 → 99; quotes re-verified at the pin via the .raw clone per T1 of Plan 216; the riir-chain lane intel below still rides for the chain-owning session).

## Source

- Repo: https://github.com/stellar/rs-soroban-sdk (stellar org, "Rust SDK for Soroban contracts")
- Pin: `0d72c4c9ed7dd5f948b4ef5ac2cbcdd9f21380f9` (main HEAD via GitHub API, 2026-10-09 authored / pushed_at 2026-10-10T04:47:10Z)
- License: **Apache-2.0** (API `license.spdx_id`)
- Inspection date: 2026-10-10 (shikuwa/4090 box)
- Method: zread structure fetch + `raw.githubusercontent` file reads at main tip + GitHub API (repo meta + HEAD commit full diff). **No `.raw` clone was created** — per the freshness law, every quote below re-verifies at the pin at mine time. Maintenance: active (flagship Stellar contract SDK, 211 stars, 119 forks, pushed this session's window).

## Verdict

**YES B− corpus (rust_perf, wasm category — family members #2 and #3 beside the lone `wasm-batch-ffi`) / LANE INTEL MODERATE (riir-chain RSM determinism lane) / product-coverage: seeds the wasm family.**

## Domain routing

`rust_perf` → category `wasm`. The tempting WRONG domains, named:

- **NOT `dist`** — the profile-level size axis (`opt-level`, `panic = "abort"`, `strip`, size profiles) is ALREADY OWNED by dist's ship-size family (`shipped-binary-missing-size-profile` —61% measured, `cli-panic-unwind-tables`, `shipped-binary-unstripped`). The surviving atoms here are SOURCE-LEVEL call-site shapes, not manifest knobs.
- **NOT `kernel_opt`** — no GPU content anywhere.
- **NOT `clippy_lints`** — neither candidate is an upstream-detectable lint shape; both are deliberate API/design patterns with crate-level context (wasm-targeting, custom-section artifacts).

## Repo character (the axis disclosure)

The SDK's perf axis is **wasm binary SIZE, not CPU cycles**: workspace `[profile.release]` is `opt-level = "z"`, `panic = "abort"`, `overflow-checks = true`, `strip = "symbols"`, `lto = true`, `codegen-units = 1`. Contracts have on-chain size budgets; the repo's CI and patterns are organized around keeping the shipped wasm small. The repo itself is a **host-call facade** — `Map`/`Vec`/`Bytes` delegate every operation to `env.map_*` / host objects (`soroban-env-host`); the actual VM, cost model, and data-structure machinery live in the sibling repo **`stellar/rs-soroban-env`** (named in the follow-up pool below). Bulk of the SDK = macro plumbing + conversion boilerplate = ambient structure (KILL class), correctly not mined.

## Ranked candidate table

| # | Pattern | Source file | Grade | Notes |
|---|---|---|---|---|
| 1 | `wasm-unwrap-panic-machinery-elimination` | `soroban-sdk/src/unwrap.rs` | **B− (rule)** | See below. Dedup CLEAN. |
| 2 | `linker-dce-custom-section-volatile-marker` | `soroban-sdk/src/events.rs` + HEAD commit #2114 diff | **C+/B− (own-rule-or-drop at authoring)** | See below. |
| 3 | wasm32 size-profile recipe (`opt-level = "z"` etc.) | root `Cargo.toml` | **DROP** | Dedup: dist `shipped-binary-missing-size-profile` + `cli-panic-unwind-tables` own the law; "z vs s for wasm targets" is a parameter detail → context-note candidate on the existing dist rule at most. |
| 4 | Consensus-seeded PRNG / test_snapshots / LedgerSnapshot | `src/prng.rs`, `test_snapshots/`, `soroban-ledger-snapshot/` | **NOT corpus rules** | Testing/determinism infrastructure → the LANE-INTEL output (§riir-chain). |
| — | Map/Vec host facades, macro plumbing, storage/TTL API, `arbitrary_extra`, spec generation | various | **DROP (ambient)** | API surface; the machinery is in rs-soroban-env. |

### Candidate 1 — `wasm-unwrap-panic-machinery-elimination` (B−)

The `UnwrapOptimized` trait: on wasm targets, `unwrap`/`expect` sites compile to a match + trap instruction, eliminating the panic machinery; on native they stay plain:

```rust
#[inline(always)]
fn unwrap_optimized(self) -> Self::Output {
    #[cfg(target_family = "wasm")]
    match self {
        Some(t) => t,
        None => core::arch::wasm32::unreachable(),
    }
    #[cfg(not(target_family = "wasm"))]
    self.unwrap()
}
```

(verbatim at main tip; the same `#[cfg]` split covers `expect_optimized` and the `Result` impl.)

Secondary clause (same file): `UnwrapInfallible` destructures the never type instead of panicking —

```rust
Err(never) => match never {},
```

with the in-source law verbatim: "destructuring the `never` variable into an empty set of cases is the most honest since it's statically checked to _be_ infallible, not just an assertion of our hopes."

- **Mechanism magnitude story (bytes axis):** each `unwrap()` on wasm carries a call into `core::panicking` with a formatted message (static string + fmt plumbing) even under `panic = "abort"`; `core::arch::wasm32::unreachable()` is a 1-byte trap. Source-qualified (production-shipped in every Soroban contract); **no local wasm measurement this session** — the rule carries the size-axis provenance, never a local bar.
- **Two-regime:** wasm-size-critical artifacts (the `#[cfg(target_family = "wasm")]` arm) vs native (plain unwrap retained — the trait is a no-op shape there). Bounded fix-space: trait definition + call-site `.unwrap()` → `.unwrap_optimized()`.
- **Anti-regime (MANDATORY in-rule):** `unwrap_optimized` is a DELIBERATE semantics change on the impossible arm (trap vs panic — the panic handler, hook, and message are gone). Never rewrite an unwrap whose `None`/`Err` arm is reachable.
- **Dedup evidence (both vocabularies, grep this session):** `unwrap_optimized|panic_formatting|wasm.*unreachable|unreachable\(\)|binary.?size` over `src/domains/rust_perf/corpus/*.rs` → **0 hits**. The two corpus unwrap hits own DIFFERENT axes — `unwrap-or-else-lazy` (eager-vs-lazy default evaluation) and `oneshot-response-send-unwrap-panics` (panic-as-liveness-hazard, unwrap REMOVAL); distinct-from both in-rule plus `cli-panic-unwind-tables` (dist, PROFILE level vs this SOURCE call-site level — complementary levers, the source itself runs both: panic="abort" in the profile AND UnwrapOptimized at call sites).
- **GOAT test:** criterion 1 (benchable) FAILS for a **domain reason** — the win metric is wasm binary size, which the native Pareto validator cannot host. Per the dial9 correction (Research 179) that is a per-rule **NotBenchable annotation**, not a demotion. Criteria 2 (exactness: n/a on the precondition-impossible arm, stated in the anti-regime clause), 3 (bytes story above), 4 (deterministic) hold.

### Candidate 2 — `linker-dce-custom-section-volatile-marker` (C+/B−)

The repo's HEAD commit at the pin (#2114 "Fix event spec shaking") is itself the pattern: contract spec metadata ships as data-only statics in custom sections (`#[link_section = "contractspecv0"]`) that no compiled code references — the wasm linker's GC shakes them out, and the fix routes every publish path through a retention marker:

```rust
static MARKER: ...::shaking::Marker =
    ...::shaking::generate_marker_for_xdr(&UsedPublishEventDataType::spec_xdr());
let _ = unsafe { ::core::ptr::read_volatile(MARKER.as_ptr()) };
```

(diff verbatim at the pin; the commit body: events published via `Events::publish_event` or the trait's `publish` "never hit the event's spec shaking marker ... so the event and its field types are stripped from the built contract's spec even though the event is still published on-chain.")

- Regime: wasm (or `--gc-sections` native) artifacts carrying **data-only custom sections consumed by external tooling** — the bytes are the product, the linker cannot see that. Bounded fix-space: marker static + `read_volatile` read rooted in a reachable code path.
- Dedup: zero corpus hits (`link_section|spec_shaking|marker` — grep this session, 0). Novel axis: nothing in the corpus owns "keep data alive against linker DCE".
- NotBenchable (same size-axis domain reason). Admission at authoring: if the fixture needs Soroban context to make sense, drop to annotation; the plain-Rust stand-in (`#[link_section]` static + tooling-consumer framing) must stand alone.

## GOAT candidate statement

Candidate 1 is the headliner. Its GOAT fails at criterion 1 for the domain reason documented above (size axis), which the two failure classes resolve as ANNOTATION-grade (Batch 136 / dial9 precedents) — the corpus's wasm family is exactly this shape already (`wasm-batch-ffi` ships NotBenchable with `bench_spec: None`).

## riir-chain lane intel (the user-directed axis)

Grounded facts (this box, read-only): `../riir-chain/benches/bench_341_incremental_divergence.rs` documents an RSM that "compares R replicas' program-state tags on EVERY CPI transaction" — `commitment_root()` (O(N log N) collect+sort+fold) vs `chain_incremental_root` (O(1) LtHash multiset checksum maintained incrementally), plus a `drift_check` audit gate. Deterministic-replay/divergence machinery is LIVE there.

Transferable designs from this source (reference material for the chain-owning session; NOT implemented from this box — riir-chain is read-only here):

1. **Consensus-seeded PRNG** (`soroban-sdk/src/prng.rs` module doc, verbatim at the pin): "The network runs in strict consensus, so every node in the network seeds its PRNG with a consensus value, **not a random entropy source**... the seed is derived from the overall transaction-set hash and the hash-sorted position number of each transaction." Architecture: one hidden base PRNG seeded by consensus → independent local PRNGs per contract invocation (an invocation can touch only its own; state dies with the invocation; re-entry = new invocation). Test law: "In local tests, the base PRNG of each host is seeded to zero when the host is constructed, so each contract invocation's local PRNG seed ... will be determined strictly by its order of invocation in the test." — This is the upstream DISCIPLINE for any stochastic on-chain behavior: derive every random value from consensus data or replica diverge. The RSM accept-divergent check is the enforcement point; the PRNG design is what keeps the check green by construction. Warning clause worth keeping: consensus seeds are public and validator-biasable — never for secrets.
2. **Committed golden test snapshots** (`test_snapshots/` across every test crate): each test's full env interaction sequence (events, storage ops, auth) is recorded to committed JSON and byte-compared on re-run — any behavior change is a visible diff. Test-time sibling of riir-chain's runtime `drift_check`.
3. **LedgerSnapshot capture/merge** (`soroban-ledger-snapshot/src/lib.rs`): capture state-at-N, restore as a `SnapshotSource`, and `update(&host)` merge — "Existing entries in the snapshot that are untouched by the host will remain" (verbatim doc) — a replay/resync fixture shape: seed a test world from a captured snapshot, run the transition, merge back.
4. (One line) Budget/cost-model metering tables live in `rs-soroban-env`, not this repo — if riir-chain grows program fee metering, that is the reference repo.

**Not transferable:** the XDR/guest-host wasm split (riir-chain is native), the `opt-level = "z"` size posture (chain binaries are not size-constrained), the SDK macro surface.

**Recommendation:** the chain-owning session (M3 lane) evaluates items 1–3 against its RSM program-execution design; the filing content = this section + the `prng.rs` module doc as reference text.

## Product-coverage axis

The corpus `wasm` category carries exactly ONE rule (`wasm-batch-ffi`, entries_01). Soroban is the flagship Rust-wasm smart-contract SDK — this verdict seeds family members #2 (and #3 if candidate 2 survives authoring). **Follow-up pool (accumulation clause: pool sources read before the batch window fold into the same batch):** `stellar/rs-soroban-env` (the host VM — cost model, host functions, the real sorted map/vec impls; the DEEPER wasm source and the natural next distill), wasmi (already mined — B99/B101 lineage from the wasmi v2.0 blog), wasm-bindgen/wasm-pack docs (size-opt guidance).

## Honest caveats

- **No `.raw` clone; quotes read via raw.githubusercontent at main tip, pin sha read via API the same session** — the reads and the pin are one commit-window apart; per the freshness law, re-verify every quote at the pin at mine time (Plan 216 T1).
- **Unread files (unverified — re-read at mine time):** `bytes.rs`, `env.rs`, `storage.rs`, `testutils/*`, `tests/fuzz`, all of `soroban-sdk-macros`, and the whole `rs-soroban-env` repo (the deeper mining target).
- The rules land with the **size-axis NotBenchable annotation**; no wasm toolchain measurement exists on this box and none was attempted.
- The batch's unit of value is small (1–2 rules) — the grade is carried honestly by the B− band and the family-seeding rationale, not inflated.

## Queue slot

Plan 216 (`.plans/216_soroban_wasm_size_rule_batch.md`) — the batch queues behind the standing staged pools per yield discipline (the TensorFold Plan-215 pack and the conversion set convert first); no `.plans/` number collision (dual_allocation_gate clean 0/0/0 at allocation, 2026-10-10).
