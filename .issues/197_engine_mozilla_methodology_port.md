# Issue 197 — engine distill: the Mozilla-quartet methodology port (the uniffi-checksum / sccache-mock / vet-aggregate ideas)

**Status:** Open — T0b staged (plan 329); Claude verdict round 1 REVISE addressed, round 2 AGREE (session 6736d164; recorded at the filing commit)

2026-11-02, session zed-m3-mozilla-quartet. The corpus half of the four-repo distill landed as `.research/361` (uniffi-rs, YES B−) + `.research/362` (sccache, YES B−); rust-code-analysis and supply-chain graded NO as corpus sources. This issue is the OTHER half the user asked for: distill the way these projects ENGINEER into the healer itself — the same treatment the blaze idea got (issue 171: RA pre-filter oracle, T0b package-scoped re-checks landed as scoped_recheck GOAT PASS, T0a deferred-if-2) and the fusion/Fourier wild-idea lanes (issues 157/020: PoC-first, verdict-gated).

Substrate-first ran BEFORE filing (2026-11-02) and was RE-VERIFIED at the round-1 verdict (the reviewer's greps reproduced + extended ours):

- `pub fn with_cargo_bin` has **SIX verifier definitions** (`src/fix_verify.rs:230`, `src/compile_oracle.rs:144`, `src/domains/rustc_errors/driver.rs:642`, `src/domains/rust_perf/{bench.rs:286, verify.rs:140}`, `src/domains/clippy_lints/verify.rs:136` — plus the inner `CheckRunner::with_cargo_bin`). **No new mock framework — the matrix rides this seam family.**
- The score-bench generation ladder "is byte-stable and re-derivable from the corpus alone" (`score_bench/generations.rs`) — the stale-freeze class is structurally prevented. **Candidate C (freeze-table↔corpus digest handshake) DROPPED at substrate-first: the uniffi checksum doctrine is already satisfied by re-derivation.** (Verdict round 1 concurs.)
- `src/traj_store.rs` greps **ZERO** hits for `toolchain|rustc_version|clippy_version` — the trajectory-store audit (P4-b) is already ANSWERED (the defer-if-2 exit cannot fire); it is a real gap with a migration question, filed separately below.
- `.benchmarks/score_history.jsonl`: 217 rows; **45 lack `generation`**; the union schema is `{created_rate, cross_ooo, decline_correctness, demonstrability, edits_total, generation, heal_rate, n_clean, n_healable, n_ran, ood, parse_safety, rule_outcomes, sha, shaped, strata, ts, vocab_size}` — no toolchain axis anywhere (verified across ALL rows this revision, not the first row as originally filed).
- Toolchain-drift warnings exist at RUN time (`score_bench/report.rs:426` "frozen labels stale vs this toolchain"). **P4 narrows to the PERSISTED-row axis.**
- `src/domains/` contains non-rule entries beside the rule domains (`dist/`, `docker/`, `probe_witness.rs`) — P3's population check needs a by-name exemption list or it reds on day one.
- riir-refine has NO docs-gate family; the gate home is `scripts/ci_feature_guard.sh`.

## P1 — the corpus pins manifest (uniffi per-API checksum handshake × supply-chain vet-aggregate)

**The class is PAID FOR here:** Batch 308 landed with "the B298/B305-era pins sat STALE at 222/225 — two different pin families had diverged, repaired to measured truth with the miss disclosed" (batch-308 landing row, `.distill/001`). Count pins are hand-typed `assert_eq!(corpus.entries.len(), 289)`-shaped asserts across `corpus/tests.rs` (entries + categories + annotation families), `src/domains/kernel_opt/router.rs` (the expert totals — NOT `src/draft/domain_router.rs`, which is the draft's router and never a pin site), `mod.rs` doc headers, AGENTS.md/README/`.docs` rows.

**The port (the round-1 DRY shape):** ONE derive script (`scripts/pins_manifest.py`) derives `{per-domain rule count, category count, NotBenchable/benchable/unannotated counts, kernel-side witness}` from the corpus source and gates a COMMITTED `.benchmarks/pins_manifest.json`. The Rust asserts in `corpus/tests.rs` + `router.rs` **READ the manifest via `include_str!` + serde_json** instead of re-typing numbers — the manifest is the single source of truth and the asserts stay deliberate speed bumps, never a second hand-maintained copy (the rubber-stamp defense: `--write` cannot "fix" a failing Rust assert, only a real corpus change can). The script additionally gates the PROSE sites (mod.rs headers, AGENTS/README/`.docs` rows) against the same file.

**Home:** `scripts/ci_feature_guard.sh` (no docs-gate family exists here).

**G1 (two directions):** (a) drift one pin row in a fixture copy → bare run reds naming it (the B308 canary); (b) change the corpus WITHOUT `--write` → the gate reds on the stale manifest (the reverse direction — the manifest must never be run-to-pass).

**Ratchet (the cargo-vet import):** any by-name exemption list this gate grows takes only deletions over time (the `fmt_pending_expected.txt` drain-list shape) — never additions.

## P2 — the fix_verify failure-injection matrix (sccache mock_command doctrine)

**State:** the module docstring (`fix_verify.rs:55-60`) DOCUMENTS the refusal classes — cargo fails to run (bad `--verify-args`, broken manifest), killed mid-run ("OOM, user kill — output lost"), cargo-level refusal that emitted artifact JSON, spawn failure. The seam family (six verifiers, above) is tested for exactly TWO classes in `fix_verify` (`kill_path`, `loop_exhaustion`); `rust_perf/verify.rs` + `clippy_lints/verify.rs` each carry one `with_cargo_bin_overrides` posture test — the FAILURE MODES are not.

**The port:** the fake-cargo failure scripts are built ONCE as a shared `#[cfg(test)]` helper; the matrix runs over **`fix_verify` + `compile_oracle` minimum**; the other four verifiers are listed in the plan as covered-or-deferred, by name with a reason. One arm per documented class — binary-missing (spawn failure), garbage stdout (non-JSON lines), signal-kill (exit None), empty-output, permission-denied — each asserting the THREE-part contract: (a) the refusal is LOUD, (b) the tree is restored byte-identical, (c) the stats outcome is UNVERIFIED, never certified.

**The leverage half (round-1 addition):** per-class refusal COUNTERS in the verify stats — not just `report.error` strings. The taxonomy of WHY a verification refused is the part of the sccache import with the most product value (sccache reports per-cause compile stats); the counters make the failure modes first-class in every run roll-up.

**G1:** each arm fails if its class silently KEEPS an edit (the kept-edit-without-proof class P104 audits post-hoc — the matrix proves the refusal paths pre-empt it).

## P3 — the domain-wiring gate (rust-code-analysis declarative-registry doctrine)

**State:** the `findings by domain` roll-up labels are hand-maintained in `src/bin/heal.rs` (~20k-line bin; roll-up sites at :10304/:20090/:20109); adding a domain touches `domains/*/mod.rs` + fan-out + score_bench + docs with NO gate that every site agrees. rca's doctrine: the registry IS the list — a language cannot be half-registered.

**The port (round-1 shape):** ONE central `pub(crate)` const table in `src/domains/mod.rs` (stated plainly: a Rust const cannot be contributed from each `mod.rs` without `inventory`/`linkme` — no new deps, zero public-API surface, T0b stays behavior-neutral) + three tests: (1) every `src/domains/*/` directory with a `mod.rs` appears in the registry **or in a by-name EXEMPTION list with a reason per entry** (`dist/`, `docker/` = scan-only domains; `probe_witness.rs` = a file, not a domain — the list starts with exactly these three); (2) the heal.rs roll-up label map keys ⊆ registry; (3) score_bench domain rows ⊆ registry.

**G1:** a fixture dir absent from both registry and exemptions reds naming it.

**Ratchet:** the exemption list takes only deletions over time (same vet import as P1).

## P4 — the persisted-measurement axes (sccache cache-key completeness) — SPLIT at round 1

**P4-a (T0b-safe): provenance fields on the score_history row.** Add `rustc_version` (+ `clippy_driver_version` when the oracle lane ran) to the append row. The readers tolerate unknown fields (no `deny_unknown_fields` on the row struct — verified at round 1), so old readers stay green. **The G1 folds in the generation-less class: 45 of 217 rows carry no `generation` — different corpus generations may already pool silently, the same completeness class as the missing toolchain axis. The G1 asserts the report's comparisons never pool rows differing on (toolchain ∨ generation), and the generation-less legacy rows read as `generation: unknown`, never pooled.**

**P4-b (its OWN follow-up issue — a behavior change, NOT this plan): the trajectory-store key.** `traj_store.rs` has zero toolchain/rustc pinning (verified). The doctrine: an input that can change the outcome is in the key. Adding it invalidates/partitions existing keys → the follow-up issue carries the migration rule: rows without the toolchain read as `unknown-toolchain` and are NEVER pooled with pinned rows. Filed as `197-followup` at claim time if P4-b proceeds.

**Drop of defer-if-2:** the audit the deferral gate would have run is ALREADY ANSWERED (zero hits, verified twice) — the honest path is the split above, not a deferred audit.

## P5 — the uniffi golden-fixture check (T0 audit, round-1 addition)

uniffi pins every binding against golden fixtures per language. The healer analog: snapshot tests of `--fix` OUTPUT per domain (the fix text is the "binding"). T0 task: check whether the score-bench frozen generations + fixture gates already cover fix-output snapshots per domain; if yes → record P5 as already-satisfied (the Candidate-C outcome, cited as precedent); if no → the gap is a follow-up task in this plan's ledger, priced before any claim.

## Tiering (the issue-171 shape)

- **T0b — lands regardless (test/script-only, zero engine behavior):** P1 manifest gate + include_str! asserts + two-direction canary · P2 matrix arms + refusal counters · P3 registry + exemptions · P4-a provenance fields (+ the generation-less G1). T0b carries TWO output deltas, both labeled: the score_bench pooling split (P4-a's G1) is the only VERDICT-AFFECTING delta; the refusal counters (P2) add stats fields + a roll-up line — output-only, never verdict-affecting.
- **T0a — audited/behavioral, sequenced separately:** P4-b (follow-up issue, migration rule) · P5 (audit → satisfied-or-ledgered).
- ⛔ `score_bench/report.rs` comparison behavior changes (the pooling split) are PART of P4-a's G1 and are the ONE report-behavior delta in T0b — the plan labels it.

## Precedents cited

- Issue 171 — the blaze idea, ported (RA pre-filter oracle; T0b landed as scoped_recheck GOAT PASS).
- Issues 157/020 — fusion / Fourier wild-idea lanes (PoC-first, verdict-gated).
- The B308 stale-pin incident (this repo) — P1's paid-for motivation.
- P104 (the unverified-kept audit) — P2's refusal classes are the paths that pre-empt it.
- Candidate C's substrate-first drop — the re-derivation doctrine already satisfies the uniffi checksum law.

Plan: `.plans/329_engine_mozilla_methodology_port.md`. Related distill notes: `.research/361`, `.research/362`.
