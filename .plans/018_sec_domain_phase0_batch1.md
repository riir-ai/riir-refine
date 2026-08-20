# Plan 018 — `sec` Domain Phase 0 + First Mining Batch (FFI/Drop Family)

Executes [Proposal 004](../.proposals/004_sec_domain_fourth_heal_domain.md). Seeded from [Research 078](../.research/078_sec_domain_seed_verdict_rust_secure_code_index.md). The batch number is assigned at land time per the queue snapshot (next by AGENTS.md count: Batch 70). Plan-first discipline (the Batch 60 adopted rule): no corpus edit before this plan exists — it does now.

## Phase 0 — domain scaffold

- [ ] T0.1 `SecDomain` impl at `src/domains/sec/` (mod.rs + corpus.rs), mirroring the `clippy_lints` domain shape. `SecRule { rule_id, sec_concept, buggy_code, fixed_code, family: SecFamily }` (`SecFamily`: `Ffi | DropLeak | UnsafeHygiene | CryptoHygiene`). If the `HealDomain` trait signature does NOT fit cleanly → STOP per Proposal 004 caveat 5 — file an issue, do not stretch the trait.
- [ ] T0.2 Feature flag `sec` in `Cargo.toml` (opt-in, default-off; syn-only, zero new dependencies — the `rust_perf` gating pattern).
- [ ] T0.3 Wiring: `DomainRouter` 4th-domain fan-out annotation + `--domain sec` in the cargo-heal CLI + inclusion in `--domain auto`.
- [ ] T0.4 Do-not-reimplement boundary gate: encode the banned clippy-lint list (`mem_forget`, `indexing_slicing`, `arithmetic_side_effects`, `unwrap_used`/`expect_used`, `improper_ctypes*`, `partialeq_ne_impl`, `undocumented_unsafe_blocks`, `disallowed_methods`) as a test asserting no sec rule's fixture fires an equivalent clippy restriction lint with the same fix.
- [ ] T0.5 Unsafe-count syn visitor (geiger-provenance taxonomy: unsafe fns incl. attribute-unsafety `#[no_mangle]`/`#[export_name]`, unsafe methods, expressions in nested scopes, impls, traits; `#![forbid(unsafe_code)]` detection) — feeds `SecVerdict.unsafe_count_delta`; no geiger dependency.
- [ ] T0.6 Phase-0 gate: `cargo check --features sec` + `cargo clippy --features sec --all-targets` clean; default build unchanged (`cargo tree` diff empty).

## Phase 1 — first batch: FFI/Drop family (Research 078 ANSSI-derived candidates)

- [ ] T1.1 Mine-time dedup re-verification: for EVERY candidate rule, grep `riir-clippy/src/` under both vocabularies (source term + rule-id operator name) AND re-verify the clippy-gap against current `rust-clippy` (the 2026-08-20 snapshot may be stale — Proposal 004 caveat 4). Record one evidence line per rule in the batch's research note.
- [ ] T1.2 Author the candidate rules (target 8-12, expecting ~50% coordinator falsification): `ffi-extern-fn-catch-unwind` (GOAT) · `ptr-deref-null-check-first` · `drop-impl-no-panic` · `ffi-fnptr-type-unsafe-extern` · `ffi-by-value-no-drop-type` · `ffi-portable-c-alias` · `extern-fn-reference-param-to-pointer` · `rc-cycle-interior-mutability` · `no-box-leak` · `maybe-uninit-requires-justification`. Fixtures are ORIGINAL minimal reproductions — never paste ANSSI/Rudra source (license hygiene + binary-only distribution rule). Each rule names its regime conditions (when it does NOT fire).
- [ ] T1.3 Proposers for the parse-only provable subset (start: `ffi-extern-fn-catch-unwind`, `ptr-deref-null-check-first`, `drop-impl-no-panic`); conservative gates: refuse signature-changing rewrites with live callers (the `static-str` caller-gate precedent), refuse when comments would be displaced, refuse non-`extern` fns.
- [ ] T1.4 GOAT gates (the Proposal 004 gate, executed):
  - [ ] G1: every fixture fires its rule; `clean_spans_fire_no_lint` extended with real excerpts from riir-viewbridge + riir-chain; banned-list boundary gate green; **negative control: `ffi-extern-fn-catch-unwind` fires ZERO times on riir-viewbridge's `#[riir_ffi]`-wrapped surface** (nonzero falsifies the proposer).
  - [ ] G2: suggest/fix latency within the existing domain family (µs-scale; no new hot-path work).
  - [ ] G3: full lib tests + feature-combo checks green; retrieval floors of other domains unchanged; count pins updated everywhere (the 18th-count-pin-site lesson — grep for stale corpus-count comments).
  - [ ] G4: scan-path alloc-free (`SuggestScratch` precedent).
  - [ ] G5-real: scoped `cargo heal --fix --verify --features sec` on riir-viewbridge + riir-chain files: compile-clean, findings zero-or-explained, auto-reverts documented.
- [ ] T1.5 Retrieval: sec corpus joins the latent index; floors established for the new domain (self-variant probes; do not regress other domains' floors).
- [ ] T1.6 Docs: AGENTS.md domain-table row + feature-flag entry; `.docs/11_domains/sec.md` (folder README index line); README feature list; queue snapshot batch entry.
- [ ] T1.7 Research note for the batch (per research-note shape, `**Status:** RECORD` on land) + Research 078 status pointer update + this plan's checkboxes marked + commit/push.

## Phase 2 — unsafe hygiene + crypto hygiene (GATED — defer until Phase 1 verdict)

- [-] T2.1 Opens ONLY if Phase 1 lands ≥6 rules post-falsification (Proposal 004 caveat 1). Candidates: `unsafe-impl-send-sync-missing-bound` (Rudra) · attribute-unsafety awareness · `weak-hash-to-blake3` · `password-hash-argon2` · `constant-time-secret-compare` — the crypto trio dep-gated via the `fxhashmap` manifest-scan pattern (fire only when the target already depends on the replacement crate; the healer never edits Cargo.toml).
- [-] T2.2 Miri/dudect/Kani oracle wiring (cited in Research 078; heavyweight — compile gate suffices while fixes are by-construction safe). Reopen trigger: a rule family whose fix-safety is NOT by-construction.
- [-] T2.3 Second seed source (RustCrypto conventions) to clear the 20-rule domain bar.

## Close condition (honest)

If Phase 1 falsification leaves <6 rules, the domain closes at Phase 1: corpus stays as the negative-result artifact (the `entropic-selection` precedent), Phase 2 never opens, and the feature flag is removed or left dormant with a doc note. Record the number either way.
