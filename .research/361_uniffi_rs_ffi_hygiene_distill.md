# 361 — uniffi-rs FFI-Hygiene Distill

**Status:** DISTILLED — pending owner decision (Plan 327 staged behind the live queue heads)
**Date:** 2026-11-02 (zed-m3-mozilla-quartet/M3, user-directed four-repo @distill — uniffi-rs + sccache + rust-code-analysis + supply-chain; this note covers uniffi-rs, its sibling note is `.research/362`)
**Queue slot:** nominal batch per queue order / Plan 327 — staged behind the live queue heads

## Source

- **Repo:** `mozilla/uniffi-rs` — cross-language bindings generator (Rust core + generated Kotlin/Swift/Python bindings + fixtures + bindgen)
- **Provenance pin:** `ef9625b68dd51207f55f024a27649e58edb52b09` (shallow clone HEAD verified; MPL-2.0; every quote below byte-verified at this sha)
- **Shape:** 75.8k total Rust LOC including fixtures/bindgen; the mechanism core (`uniffi_core` + `uniffi_macros`) = 11.3k
- **Profile:** NO `[profile]` section in the root Cargo.toml — defaults, no benchmarking culture. A correctness/protocol repo, not perf. In-source corroboration: `oneshot.rs` "less efficient, but the difference is probably negligible" (dep-minimization over perf).
- **Completeness:** `todo!`/`unimplemented!` = 5 total (2 doc-comment examples, 3 codegen match arms). The Rust core is COMPLETE; the WIP is process-level ("a long way from 1.0", README), not code-level. Mechanism modules (`rustbuffer.rs`, `rustfuture/{future,scheduler,mod}.rs`, `rustcalls.rs`, `handle.rs`, `foreignbytes.rs`) read clean and test-dense (should_panic arms included).

## Verdict

**YES B− corpus [3–4 own sec-domain rules — FFI-hygiene family extension, ALL NotBenchable-by-domain per the dial9 split] / league-lane N/A STATED (no GPU/serving surface; the riir-viewbridge FFI lane is PARKED — home relevance, never an open lane) / product-coverage MODERATE — extends the sec FFI-hygiene family (7 rules standing) onto the pyo3/cxx/uniffi-class surface every mixed-language Rust deployment shares.**

Route **`sec`** (unsafe/FFI-hygiene class). The tempting WRONG domain: `rust_perf` — every candidate is a correctness/liveness win at the boundary (double-free, deadlock, leak), never latency, and the repo itself carries no perf posture. `wasm-batch-ffi` (rust_perf) is batching throughput — unrelated fix-space.

## Ranked candidates (coordinator-verified quotes at the pin)

**1. A− `arc-refcount-across-raw-boundary`** — `uniffi_core/src/ffi/rustfuture/future.rs:249–289` + `ffi/handle.rs:86–100` (both verified this session: `:251` `Arc::<Self>::increment_strong_count(ptr.cast::<Self>())` in the waker-clone vtable arm; `:270` comment "For wake_by_ref, we can use the pointer directly, without consuming it to re-create"; `:288` `Arc::<Self>::from_raw(ptr)` in the consuming wake/drop arm; `handle.rs::into_arc_borrowed` = `clone_arc_handle` (increment) then `Arc::from_raw`).
The law: when a raw Arc pointer crosses an unsafe boundary (waker-vtable data pointer, FFI handle), a BORROWED use is `increment_strong_count` + deref — never a `from_raw` round-trip then forget; a CONSUMING use is `from_raw` exactly once. Two-regime: borrowed-use vs consume-at-wake. Two corroborating instances in one file family (waker vtable + `into_arc_borrowed`).
GOAT: (ii) FAILS — `increment_strong_count` ≡ `Arc::clone` (one `fetch_add`); the buggy arms are double-free/leak (UB), not slow paths. Sec-lane NotBenchable correctness rule with an integer-exact test oracle (a refcount-ledger fixture asserts `strong_count` at every step — G1-equivalent trivially).

**2. B+ `ffi-callback-invoke-outside-lock`** — `rustfuture/scheduler.rs:70–100` (verified: `store()` extracts `to_invoke` under the `state.lock()` scope, then `cb.invoke(poll)` runs AFTER the guard drops) + `oneshot.rs:40–46` (same shape at the wake site).
The law: extract the callback/waker under the lock, invoke it after release. Regime: the callback crosses a language/runtime boundary that can be held by a thread which re-enters this lock (foreign ABBA: internal Mutex vs the foreign runtime's lock, e.g. a GVL) — must defer; pure-internal callback — inline OK. Two in-repo instances.
DEDUP ADJUDICATION (the flippable one): kin `host-lock-across-reentrant-blocking-call` (entries_08, cubecl #1678) owns the SAME-LOCK self-re-entry failure mode; uniffi is strictly cross-FFI ABBA with a language boundary. The corpus's own same-shape-different-failure-mode precedent (`tokio-guard-across-await`) admits it as distinct — record this adjudication IN-RULE at authoring time so a reviewer flipping it demotes cleanly instead of colliding. NotBenchable (a deadlock bench hangs — the entries_08 honesty class).

**3. B `out-param-manuallydrop-not-maybeuninit`** — `rustcalls.rs:42–53`.
The law: a caller-initialized `#[repr(C)]` out-param is `&mut ManuallyDrop<T>` (safe reads, explicit drop discipline), not `&mut MaybeUninit` (unsafe reads required). Two-regime: the ABI guarantees caller init → ManuallyDrop; the callee owns initialization → MaybeUninit. Carries the set-twice = leak single-writer contract. Dedup: the corpus's only ManuallyDrop/MaybeUninit hit is an inline-const lifetime-retarget rule — a different decision (init-ownership vs lifetime). Sec-lane (leak/UB at the boundary).

**4. B− `panic-handler-nested-catch-unwind`** — `rustcalls.rs:224–249`.
The law: the error-REPORTING handler itself does fallible work (format/alloc) and gets its own `catch_unwind` + graceful degradation (skip the out-param set; let the empty-buffer fallback speak). Distinct from `ffi-extern-fn-catch-unwind` (which owns the extern BODY wrapper) and `drop-impl-no-panic` (Drop bodies) — the handler PATH is unowned. Adjacent: sharpen the rule to handler-path ownership or it reads as a wrapper-face positive instance.

**5. B− `null-sentinel-companion-conjunction`** — `rustbuffer.rs::destroy_into_vec` (verified verbatim: "Rust will never give us a null `data` pointer for a `Vec`, but foreign-language code can use it to cheaply pass an empty buffer" — then `if self.data.is_null() { assert!(capacity == 0); assert!(len == 0); vec![] } else { try_into().expect(...) + assert!(len <= capacity) + Vec::from_raw_parts }`).
The law: a nullable-pointer-means-empty ABI protocol validates the CONJUNCTION (null ∧ len==0 ∧ cap==0) — null alone is a LEGITIMATE encoding, corrupt iff a companion is nonzero; non-null requires len ≤ cap + width fits. HIGHEST KILL-LIST ADJACENCY: `ptr-deref-null-check-first` + `slice-from-raw-parts-len-guard` are UNARY checks; `nonmax-sentinel-option-niche` is type-width packing. Author as the conjunction invariant or demote to a positive-instance face.

**6. C+ `vec-promote-manuallydrop-not-forget`** (stretch) — `rustbuffer.rs::from_vec` (verified: `ManuallyDrop::new(v)` + `as_mut_ptr`, never `mem::forget`).
Promote `Vec` → `(ptr, len, cap)` repr(C) via `ManuallyDrop` so the borrow checker stays in play (a forgotten value can still be moved/used). Novel (borrowck playbook prose mention only) but thin; perf-neutral; sec-lane only.

**Drops:** `abi-metadata-checksum-export` + contract-version extern (codegen plumbing); `cross-dylib-alloc-free-symmetry` (real invariant — the allocator must match across the dylib seam — but no single-crate fixture can demonstrate it); `UniffiCompatibleFuture` cfg-Send bounds (ambient config); `FfiBufferElement` union (internal API design); `alignment-bit-provenance-tag` (C — one-bit tag, marginal value); `const-fn-str-eq-byte-unroll` (C — version-fragile const-comparison idiom); `pin-state-option-not-enum` (C — narrow, perf-neutral); `u64-wire-width-normalize` (C — borderline ambient; ABI fields widened then `try_into().expect` narrowed on read).

## Dedup evidence (coordinator-run, both vocabularies, at `5326d3b9`-era corpus head)

- `waker|raw_waker|increment_strong|Arc::from_raw|Weak::from_raw` over `src/domains/` → ONE unrelated comment hit (entries_53:117, the batch-miss-interleave fixture comment) → candidate #1 NOVEL.
- `catch_unwind` → `ffi-extern-fn-catch-unwind` + `drop-impl-no-panic` own adjacent spaces (adjudicated distinct above).
- `ManuallyDrop|MaybeUninit` → the inline-const retarget rule (different decision).
- `sentinel` → `nonmax-sentinel-option-niche` (type-width packing) + the unary guard rules (adjudicated distinct above).
- `lock.*callback|deadlock` → `host-lock-across-reentrant-blocking-call` (adjudicated adjacent-distinct above).

## GOAT

No perf-GOAT candidate exists in this repo — consistent with the no-profile signal. Criterion (ii) fails for every candidate for the DOMAIN reason: the wins are UB/liveness classes (double-free, deadlock, leak), not cycles/allocs, and a deadlock "bench" hangs. Per the two-class split that is per-rule ANNOTATION (NotBenchable, `bench_spec: None`), never a demotion — the corpus's tokio/bevy/deadlock families are built exactly this way. Test-oracle strength is real: the refcount-ledger fixture for #1 is integer-exact and deterministic.

## Caveats

- **Unread at verdict time:** `ffibuffer.rs` tail, `ffi_converter_traits/{,_impls}.rs` (~2k lines, likely ambient lift/lower plumbing), the macro `export/*.rs` in depth. If a rule hides there it is integer-serialization-class — low expected yield; re-check at mine time.
- #2's dedup verdict is reviewer-flippable — the in-rule adjudication note is mandatory at authoring.
- #5 carries the highest positive-instance adjacency risk — the conjunction framing is load-bearing.
- MPL-2.0 is source-shipping-compatible for our attribution precedent; fixtures remain ORIGINAL minimal reproductions regardless.
- Shallow clone — no history archaeology.

## Bottom line

Mine as ONE sec-domain batch: 3–4 rules after ~50% falsification, lead with **`arc-refcount-across-raw-boundary`**. Plan 327 staged behind the live queue heads. The two NO siblings of this quartet (rust-code-analysis, supply-chain) are conversation-only — see the queue rows this date.

Session: zed-m3-mozilla-quartet, epoch 1793597021.
