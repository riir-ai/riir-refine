# Proposal 004 — The `sec` Domain: Security-Pattern Healing as the Fourth HealDomain

Status: **draft**
Blocked by: [Issue 037](../.issues/037_e2e_self_evolve_adoption_proof.md) — the existing loop's E2E proof (verdict wiring T2 is the same real-clippy-oracle substrate the sec boundary gate depends on). No fourth domain until the first three's loop is closed (gates E2–E5).
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Plan 001 (HealDomain meta-architecture) × Research 078 (seed inventory) × the `clippy_verify`/`fix_verify` validator shapes
Related: [Research 078](../.research/078_sec_domain_seed_verdict_rust_secure_code_index.md), [Plan 018](../.plans/018_sec_domain_phase0_batch1.md), [Proposal 001](001_multi_domain_modelless_code_healer.md)

## TL;DR

Open a fourth healing domain — `sec` — for security-relevant **source-code patterns with bounded fix-spaces**: FFI-boundary safety, Drop/panic safety, unsafe-Rust hygiene, and crypto-API hygiene. Seeded from the Research 078 survey (ANSSI rust-guide A−, Rudra taxonomy B+), first batch estimated 8-12 rules. This is **not** a new invention: Proposal 001's caveat #1 explicitly named "security audit healing" as the anticipated fourth domain and prescribed exactly this revisit. It is also **not** dependency auditing — cargo-audit/cargo-deny own that (DRY, non-negotiable).

## The problem this solves

The healer today is blind to security-shaped bounded-fix patterns:

1. **The FFI/Drop/leak space clippy provably does not cover.** Verified against `rust-clippy@master` during Research 078: the restriction group owns panic/arith/indexing (`unwrap_used`, `arithmetic_side_effects`, `indexing_slicing`) but has **zero** crypto-hygiene, constant-time, or FFI-boundary lints. `improper_ctypes*` covers type layout, not panic containment. The space is clear.
2. **The #1 candidate is already hand-enforced in this workspace.** riir-viewbridge's `#[riir_ffi]` macro (Plan 532 §9, "panic across FFI is UB") wraps every `extern "C"` fn in `catch_unwind` — a human wrote that enforcement because no tool provided it. That is consumer-pull evidence: the pattern exists, the fix is known-bounded, and today it's enforced by macro discipline in exactly one crate.
3. **The owner's standing crypto rules are review-enforced only.** blake3-over-SHA1/SHA256 and argon2-for-passwords live in global AGENTS.md, applied by hand. Clippy's `disallowed_methods` can *ban* sha1 but offers **no fix suggestion** — that delta (ban + bounded fix) is precisely this domain's niche.

What goes wrong without it: the same FFI/Drop/crypto mistakes get re-made in every new crate (each consumer re-derives its own macro or review checklist), and the workspace's security posture depends on discipline rather than substrate.

## The proposed design

### `SecDomain: HealDomain` — the trait signature holds

Proposal 001 caveat #1 prescribed: "ship the trait with exactly three impls... revisit if a fourth domain forces a change." This proposal is that revisit, and the answer is: **no trait change needed**.

```rust
pub struct SecDomain;

impl HealDomain for SecDomain {
    type Entry = SecRule;   // { rule_id, sec_concept, buggy_code, fixed_code,
                            //   family: SecFamily }  // Ffi | DropLeak | UnsafeHygiene | CryptoHygiene
    type Verdict = SecVerdict; // { parses, compiles, unsafe_count_delta: Option<i64> }
    // validate = the fix_verify compile-gate shape (cargo check, ~130ms class —
    // the CHEAP validator tier per Proposal 001 caveat 2), plus an optional
    // unsafe-count delta from our OWN syn visitor (geiger-provenance taxonomy,
    // zero new deps — we already parse every file).
}
```

Feature `sec` (opt-in, default-off, syn-only — **zero new dependencies**). Corpus schema mirrors `clippy_lints` (buggy/fixed fixture pairs; `sec_concept` parallel to `gpu_concept`). DomainRouter joins sec to the fan-out as the 4th annotation domain.

### Rule families (phased)

**Phase 1 — FFI-boundary + Drop/leak (ANSSI-derived, all parse-only detectable):**
`ffi-extern-fn-catch-unwind` (the GOAT candidate — in-house precedent above) · `ptr-deref-null-check-first` (`p.as_ref()?`) · `drop-impl-no-panic` (double-panic = abort) · `ffi-fnptr-type-unsafe-extern` · `ffi-by-value-no-drop-type` · `ffi-portable-c-alias` (`c_long`/`size_t`) · `extern-fn-reference-param-to-pointer` · `rc-cycle-interior-mutability` (Weak backedge) · `no-box-leak` · `maybe-uninit-requires-justification`.

**Phase 2 — unsafe hygiene + crypto hygiene (gated on Phase 1 survival):**
`unsafe-impl-send-sync-missing-bound` (Rudra's strongest; bounded fix = add `T: Send`/`T: Sync` bound or documented invariant) · attribute-unsafety awareness (`#[no_mangle]`/`#[export_name]` count as unsafe despite safe signatures — geiger's counting nugget) · `weak-hash-to-blake3` / `password-hash-argon2` (**dep-gated**: fire only when the target crate already depends on the replacement — the `fxhashmap` manifest-scan pattern; the healer edits source, never Cargo.toml) · `constant-time-secret-compare` (subtle substitution, also dep-gated).

### The do-not-reimplement boundary (non-negotiable)

clippy already owns: `mem_forget`, `indexing_slicing`, `arithmetic_side_effects`, `unwrap_used`/`expect_used`/`panic`, `improper_ctypes*`, `partialeq_ne_impl`, `undocumented_unsafe_blocks`, `disallowed_methods`. The boundary is encoded as a banned-list + gate test: every sec rule must be authorable without firing where a clippy restriction lint already fires with an equivalent fix.

### What sec explicitly is NOT

- **Not dependency auditing.** Version-bump advisories are cargo-audit/cargo-deny's job (~90% of advisory-db — Research 078 Lane C's DRY finding).
- **Not a lint engine.** dylint exists for authoring custom lints; our value is the corpus + heal pipeline + modelless retrieval + bounded fix-space proposers.
- **Not judgment-bearing detection.** Rudra's higher-order-invariant class (fuzzy detection, unbounded fix) and attacker-reachability questions are out of scope BY DESIGN — a wrong security "fix" is worse than no fix.

## Honest caveats — READ BEFORE IMPLEMENTING  (MANDATORY)

1. **Density is marginal.** Estimated 15-19 total rules vs the ~20-rule scaffolding bar (Research 078). Phase 1 (~8-12 candidates) must land **≥6 rules post-falsification** or the domain closes at Phase 1 and Phase 2 never opens. Say the number at gate time.
2. **Consumer pull is plausible, not measured.** In-house precedent (`#[riir_ffi]`, owner crypto rules, riir-chain/riir-neuron-db security surfaces) but no workspace repo runs a sec heal today. Phase 1 therefore includes a real-repo validation run: riir-viewbridge as the **clean-span negative control** (its macro should make `ffi-extern-fn-catch-unwind` fire zero times — a wrong nonzero there falsifies the proposer) + riir-chain scoped files.
3. **Security auto-fix risk is real.** Mitigations: bounded fix-spaces only; suggest-only by default (`--fix` opt-in, same as other domains); conservative proposers (refuse when the fix changes a public trait signature — the `static-str`/`bool-to-bitpack` caller-gate precedent); compile gate on every applied fix.
4. **The ANSSI novel-count is a snapshot.** ~10-12 novel was verified against `rust-clippy@master` at survey time (2026-08-20); clippy evolves. Re-pin the dedup at authoring time (the standing mine-time rule).
5. **The trait-fit claim is pre-implementation.** If landing `SecDomain` forces a `HealDomain` signature change, stop and revisit per Proposal 001 caveat #1's own mitigation — do not stretch the trait silently.

## Fusion lineage

- **Plan 001** (`HealDomain` + the domain-as-feature architecture) — the substrate this extends; its caveat #1 anticipated this exact domain.
- **Research 078** — the seed inventory: ANSSI taxonomy (A−), Rudra detection classes (B+), geiger counting granularity, the clippy-gap verification, the do-not-reimplement list.
- **`clippy_verify` + `fix_verify`** — the validator shapes sec reuses (compile gate ~130ms class + auto-revert on error).
- **`fxhashmap` dep-gate pattern** (manifest text-scan before proposing a dep-swap fix) — reused verbatim for the crypto-hygiene family.

## GOAT gate

Opt-in `sec` stays default-off until: **G1** every corpus fixture fires its rule + `clean_spans_fire_no_lint` extended with real production excerpts (riir-viewbridge, riir-chain) + zero clippy-duplication on the banned-list gate. **G2** suggest/fix latency within the existing domain family (µs-scale span processing; no new hot-path work). **G3** all lib tests + feature-combo checks green; other domains' retrieval floors unchanged. **G4** scan path alloc-free (the `SuggestScratch` precedent). **G5-real** scoped `--fix --verify` run on riir-viewbridge + riir-chain: compile-clean, zero-or-explained findings on the negative control. Promotion to default-on additionally requires measured consumer pull (≥1 workspace repo adopting sec in CI), mirroring the `rust_perf` promotion precedent.

## What ships now (riir-clippy) vs deferred

### Ships now — Phase 0 + Phase 1
`src/domains/sec/` (mod + corpus + proposers for the parse-only provable subset), feature `sec`, DomainRouter/CLI wiring, the banned-list boundary gate, the unsafe-count syn visitor, first batch (FFI/Drop family, ~8-12 rules), docs.

### Deferred — Phase 2 and beyond
Unsafe-hygiene + crypto-hygiene families (gated on Phase 1 survival ≥6 rules); Miri/dudect/Kani oracle wiring (cited in Research 078, heavyweight — compile gate suffices for Phase 1 families whose fixes are by-construction safe, e.g. `catch_unwind` wrap, `as_ref()?`); the untrusted-input family (dangerous/untrusted vocabulary, C-grade today); second seed source (RustCrypto conventions) to clear the 20-rule bar.

### Explicitly NOT shipped by this proposal
Dependency auditing (cargo-audit/cargo-deny), secret detection (gitleaks-class — different tooling), reachability/taint analysis, any Miri/dudect CI integration, any change to the `HealDomain` trait signature.

## Phased rollout (sketch — Plan 018 expands this)

### Phase 0 — scaffold
- [ ] `SecDomain` impl + corpus schema + feature flag + router/CLI wiring + boundary gate

### Phase 1 — first batch (FFI/Drop family)
- [ ] dedup re-verification → author rules + original fixtures → proposers → GOAT gates → real-repo validation → docs

### Phase 2 — unsafe + crypto hygiene (gated)
- [ ] opens only if Phase 1 lands ≥6 rules post-falsification

## Risks

1. **Corpus thinness** (correctness-of-scope risk): if falsification kills >50% of Phase 1, the domain is a 4-rule costume — close it honestly (the `entropic-selection` negative-result precedent).
2. **False-positive reputational risk** (adoption risk): a sec tool that cries wolf gets disabled. Conservative proposers + real-repo negative controls are the mitigation; measure the FP rate at gate time and write it down.
3. **Scope creep into dependency auditing** (architectural risk): the moment a rule wants Cargo.toml or lockfile reads, stop — that's cargo-audit's surface.
4. **Effort split** (opportunity risk): rust_perf external-source intake is actively productive; sec Phase 0+1 is sized as ONE batch-cycle unit, not a campaign.

## Out of scope

WASM domain (the other named future domain — unrelated), L4 fallback changes, the binary-distribution surface, any change to existing domains' corpora.

## References

1. **Research 078** (in-house, distilled) — the 39-link survey, seed grades, oracle ladder, do-not-reimplement list.
2. **Proposal 001** (in-house) — the HealDomain architecture + caveat #1's explicit anticipation of "security audit healing".
3. **ANSSI-FR/rust-guide** — https://github.com/ANSSI-FR/rust-guide (cited-only; the guideline taxonomy).
4. **Rudra** — "Rudra: Finding Memory Safety Bugs in Rust at the Ecosystem Level" (SOSP'21), https://github.com/sslab-gatech/Rudra (cited-only; the detection taxonomy — mine the taxonomy, never the archived engine).
5. **dylint** (Trail of Bits) — https://trailofbits.github.io/dylint/ (cited-only; prior art for custom Rust lint *authoring* — our delta is the corpus + heal loop, not the engine).
6. **SafeFFI** — arxiv 2510.20688 (cited-only; sanitizer-based FFI-boundary misuse detection — dynamic-analysis complement to our static parse-only family).
7. **cargo-geiger** — unsafe-counting taxonomy provenance (5 counters + attribute-unsafety); consumed as design, not as a dependency.

## TL;DR

Ship it: open `sec` as the fourth `HealDomain` (Proposal 001's own anticipated revisit — signature holds), seeded from Research 078, first batch = the FFI/Drop family with `ffi-extern-fn-catch-unwind` as the GOAT candidate and riir-viewbridge as the live negative control. Next action: execute [Plan 018](../.plans/018_sec_domain_phase0_batch1.md); Phase 2 opens only if Phase 1 lands ≥6 rules post-falsification.
