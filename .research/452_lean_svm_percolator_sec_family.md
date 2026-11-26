# lean-svm + percolator — the on-chain ledger state-machine sec family

**Status:** LANDED — Batch 424 (2026-11-26, Plan 418): R1–R5 landed as 5 sec rules (104→109); R6 dropped at its pre-registered first-drop; the routing premise correction + the lane-intel rows carry unchanged.

**Verdict: YES B+ corpus (sec — NEW family seed, zero owners) / LANE INTEL HIGH (chaind kernel question answered, chain-sec checklist rows named) / PRODUCT-COVERAGE YES (seeds the on-chain-program family; fires at home on riir-chain + riir-dapps).**

## Sources (pins)

- **aeyakovenko/lean-svm** @ `dd409de0246dbf6ca132a1c615575f659d00e64c` (shallow clone 2026-11-26, Apache-2.0, removed at filing) — Anatoly Yakovenko's Lean 4 → Solana sBPF optimizing compiler. Educational-only per its own README ("Not Audited for Production Use"). Load-bearing artifact for THIS verdict: `.agents/skills/svm-security-testing/SKILL.md` — a 10-class SVM vulnerability taxonomy, each class with bug shape / prevention / Lean proof pattern / LiteSVM adversarial test recipe.
- **aeyakovenko/percolator** @ `afdadecaf807ff6f39a7f752e35d0cc5b2042ae1` (shallow clone 2026-11-26, Apache-2.0, removed at filing) — the zero-copy perpetual-futures risk-engine library (Rust + Kani bounded model checking + Aristotle/Harmonic Lean proofs, spec.md frozen v16.9.1; the 571★ / 2,007 commits figures are page-render metadata from a pre-clone fetch — unpinned, provenance flavor only). NOT Lean-compiled — it is the proof-disciplined Rust engine; the Lean-compiled program lane lives in a separate `percolator-prog` repo (referenced from spec/README, NOT inspected this session).

**The user's premise, corrected honestly:** "svm-lean reduced the percolator ix CUs by 10x vs the vibe coded rust version" is UNVERIFIED this session. lean-svm's own README head-to-head (vs HAND-ROLLED `#![no_std]` Rust, `rustc 1.95` + LLVM 22 `-O3 -C lto=fat`) shows **CU parity-to-slightly-worse, not 10x**: Noop 2/2 CU, Counter 45/50/52 vs 27/32/37 (Lean LOSES), Vault 195 vs 177 (Lean loses), DutchAuction 3,296/4,672 vs 3,096/4,735 (Lean ~3% ahead). The real lean-svm win is `.so` SIZE: 1.35–3.23× smaller. A "10x vs vibe-coded" claim, where it exists (likely toly's X timeline or percolator-prog), is BASELINE-RELATIVE — it measures the vibe-coded/anchor-style baseline's bloat (heap allocs, runtime init, sloppy ix bodies) against a disciplined build, not Lean-compiler magic. Percolator itself is disciplined Rust (zero-copy views, account-local bounded progress), which is exactly why its ix bodies are cheap.

## The verdict-shaped content: the 10-class ledger state-machine taxonomy

From `svm-security-testing/SKILL.md` (read in full, 244 lines) + percolator's production-shape confirmations:

| # | Class (lean-svm naming) | One-line law | Rust-shaped fix |
|---|---|---|---|
| 1 | Same-tx account revival | close that zeroes the magic (not a nonzero tombstone) + same-period refund lets a fresh-init inherit residual bytes | permanent nonzero `MAGIC_CLOSED` tombstone ≠ every live magic + zero body on close AND init + exact `len == size` (SKILL.md §1) |
| 2 | Dirty-buffer init | init eligibility keyed on a business field (`balance == 0`) reads garbage from a pre-filled body | dedicated immutable discriminator, never a mutable business field; explicit zero of every field incl. padding (§2) |
| 3 | Missing owner pinning / exact size / discriminator | `len >= size` instead of `==`, unpinned owner, missing type magic → cross-type confusion | one centralized validator per account type: owner + exact len + magic (§3) |
| 4 | Spoofed CPI target (`noop.so`) | a callee program-id that arrives from the instruction executes unverified | pin every CPI target against a const / immutable config account (§4) |
| 5 | Account aliasing / self-transfer | `source == dest` makes the transfer a no-op while internal accounting still moves — conservation orphans | explicit distinct-key check on every aliasable pair (§5) |
| 6 | CPI proxy unvalidated opcodes | a wrapper forwards user-supplied mode bytes it never allowlisted | allowlist every forwarded enum; pin every target account (§6) |
| 7 | Share-pool footguns (ERC-4626 class) | first-depositor inflation, drained-pool revival, donation inflation, receipt>total desync, trapped rent | four boundary states + `curSh <= totSh` guard + closure path (§7) |
| 8 | Missing signer / writable / PDA pre-funding | ownership checked without the signature flag; pre-funded PDA bricks `CreateAccount` | check the flag; allocate+assign+transfer instead of create (§8) |
| 9 | Stale cross-leg state | withdrawals/liquidations against one refreshed leg of a multi-leg account | refresh-stamp equality across ALL active legs before collateral-reducing ops (§9) |
| 10 | Arithmetic rounding direction | floor on user payouts / ceil on debt rounds in the user's favor → dust-extraction loops | floor user-favorable, ceil obligations, PROVEN direction (§10) |

Percolator carries the same discipline in production shape — its frozen spec states the rounding law canonically (spec.md L77, verified at pin): **"All divisions round against the account unless explicitly stated"**, and its README carries the no-atomic-mutation law (L26): "**an `Err` must not commit partial mutation**" — the SVM-rollback assumption our own MintCeiling proof states as "refusals write nothing".

## Dedup gate (evidence, both vocabularies, `src/domains/sec/corpus.rs` @ 54ce0ed65)

`grep -ci` counts: tombstone 0 · revival 0 · self-transfer 0 · self_transfer 0 · distinct-key 0 · "share pool" 0 · 4626 0 · "rounding direction" 0 · "program id" 0 · "first depositor" 0. The one `aliasing` hit is `packed-field-silent-overflow-wrong-key` ("masked key ALIASING below a packing width") — a bit-packing axis, NOT account-identity aliasing; adjudicated distinct both directions. The `unchecked-wire-arithmetic` cousin owns BARE-operator wrap on wire-byte integers; the rounding-direction rule owns CHECKED arithmetic with a direction POLICY — adjacent genus, distinct sink, in-rule adjudication mandatory. The 10 `stale` hits are cache/staleness contexts, none a cross-leg refresh-stamp law. **The corpus has zero owners of the on-chain/ledger family: 104 rules today, all web/crypto/FFI/numeric/agent-shaped.**

## GOAT two-class adjudication

Criterion 1 fails for a DOMAIN reason: conservation/rounding/revival wins have no validator host (correctness, not latency) — the dial9/136 annotation class, `NotBenchable-by-domain`, suggestion-only, `bench_spec: None`. That is a per-rule ANNOTATION, never a demotion; the batch grades on decision content × novelty × dedup clearance × yield. No candidate passes criteria 2–4 as a bench claim — none needs to; this is the corpus's standard suggestion-only posture.

## Ranked candidate table (Batch 424 pool; every row UPSTREAM: present — both sources carry their own landed fixes, decision content only, never an outbound-PR angle)

| # | Pattern (candidate rule id) | Source | Grade | Notes |
|---|---|---|---|---|
| R1 | `division-rounds-against-the-account` | percolator spec.md L77 + README L49 ("a same-side reduction is rounded against the reducing leg") + lean-svm §10 + `Math.lean` `mulDiv64`/`mulDiv64Ceil` split | **A− content** | The batch's LEAD. Direction-as-policy with the conservation harm named (dust-extraction loop). Floor user-favorable amounts, ceil obligations/fees/debt; "against the account" is the portable form. Adjudicate in-rule vs `unchecked-wire-arithmetic` (bare-op wrap vs checked-direction) and `packed-field-silent-overflow-wrong-key` (bit-loss vs rounding-side). |
| R2 | `account-revival-tombstone-close-discipline` | lean-svm §1 + SKILL.md MAGIC_CLOSED pattern | **B+** | Close writes a permanent nonzero tombstone distinct from every live magic + zero body on close AND init + exact-length check. FOLDS class #2 as the init-flag half (dedicated discriminator, never a business field) if the landing read finds one bounded rule; keep two entries if the fixtures want separate init/close arms. |
| R3 | `share-pool-first-depositor-boundary-states` | lean-svm §7 | **B+** | The four ERC-4626 boundary states (totSh==0∧assets>0; assets==0∧totSh>0; donation inflation at totSh==1; receipt>total) + the trapped-rent closure half. Decision-content rich; each state is a falsifier arm. |
| R4 | `account-pair-aliasing-distinct-key` | lean-svm §5 | **B+** | Self-transfer no-op breaking conservation (SPL source==dest; lamport from==to; partyA==partyB). Zero corpus owners of the identity-aliasing axis. |
| R5 | `cpi-callee-id-unpinned` | lean-svm §4 (+ §6 wrapper-mode fold option) | **B−** | A callee/program identifier arriving from the untrusted request executes unpinned. Adjudicate BOTH directions vs `external-helper-config-pinned-invocation` (B389 atlas — same genus "callee identifier must come from trusted config", different sink: process spawn vs cross-program invoke) and vs our home structural counter-instance (riir-chain dispatch is a compiled `match` over variants — the anti-regime arm). |
| R6 | `stale-leg-refresh-before-collateral-reduction` | lean-svm §9 + percolator stale/B-stale gates (README "Account-local safety") | **B−** | Refresh-stamp equality across all active legs before any collateral-reducing op. Stateful; regime = multi-position protocols. THIN at home (no multi-leg surface today) — keep as the batch's optional 6th, first drop candidate. |

**Drops (recorded):** PDA pre-fund brick + `is_signer` (§8) — SystemProgram-vocabulary-specific, no bounded Rust-general shape without on-chain vocab gating; the abort-semantics law (§"Error paths need zero cleanup") — a positive instance AT HOME (our MintCeiling "refusals write nothing" proof + dapps envelope refusals), never a new rule; share-pool trapped rent could fold into R3.

**Yield estimate: 5 ± 1 rules** (R1–R5 solid; R6 coin-flip). Above the recent 1–3 band — justified: an explicitly enumerated taxonomy with per-class bug/fix/proof/test recipes, and a zero-owner family seed. Falsification pressure sits on R5 (the B389 adjudication) and R6 (regime reachability at home).

## Lane-intel axis — the user's "may hit kernel level (chaind and else)" question, answered

**Target lane: chain-sec hygiene gate (quarterly) + `riir-ai/.agents/skills/chain-sec/SKILL.md` checklist.** Our checklist ALREADY translates the 10-point Solana audit (read this session); the deltas this source adds:

1. **Checklist point 6 (Close/revive) has a named, not-yet-landed row:** "no remove producer exists (safe direction); **tombstone rule BEFORE the first one lands**". lean-svm §1 IS that rule's reference design (MAGIC_CLOSED ≠ live magics + `zeroWords` on init AND close + exact `len == size`). When `OP_REMOVE`/`CpiAccountStore::remove` grows a producer, land the tombstone rule with this design cited. Routing: riir-chain act, named here, not filed from this session.
2. **Checklist point 8 (accounting distrust) gains the rounding-direction law:** current row carries "integer-truth law + commit-boundary finiteness screen + conservation fuzz" — no direction policy named. Percolator spec L77's "all divisions round against the account" is the canonical form; our swap/lending/mint arms are the audit surface.
3. **NOT transferable (recorded negatives, each with its reason):**
   - **The 10x CU number:** chaind's program plane has NO CU meter (native Rust execution; the only "compute budget" is shard-assignment's integer cost units). The binding budgets are the tick-tier model (50 ms hot tick) and the Glacial write rate (chain write p50 1,344.5 µs, bench 020). Program-ix cost lands there as µs, not CUs — a 10x-leaner ix body would show as cheaper writes, but no lane measures "ix CUs" today.
   - **Percolator's overflow-checks tradeoff:** their deployed PROGRAM wrapper builds without overflow checks (compute limit), covering the wrap with per-function no-panic proof pieces; their engine lib pins `overflow-checks = true` (verified in their Cargo.toml). Our G4 pin (bench 350) already took the fail-closed side for BOTH postures (`overflow-checks = true` dev+release, panic → `/healthz` halt + height-stall alert). Nothing to change; recorded so nobody re-litigates it via the percolator precedent.
   - **Spoofed-CPI class:** structurally impossible at home — our CPI dispatch is a compiled `match` over program variants with pinned wire tags (checklist point 3; `program_rate_gate.sh` rule 5). The rule (R5) is for the world's Rust, never a home finding.
4. **FV-methodology insight (research-note follow-up, candidate riir-chain `.research/` entry):** three proof architectures on one spectrum — lean-svm COMPILES the kernel-checked Lean term into the runtime artifact (proof = code, zero divergence surface); percolator PROVES the Rust (Kani bounded model checking + Aristotle Lean-about-compiled-Rust, with a proof ledger that tracks per-function drift via `changed_fns.py` re-verification); we TEST the Rust against Lean specs (8 `*_spec_match.rs` files, finite samples). The divergence surface differs: ours is the widest (spec-match finite samples), theirs narrowest (compile-the-proof). Percolator's honest cost lessons transfer directly: "No Kani proof executes a trade" (the INTRACTABLE-TIER disclosure IN the ledger), two unverified harnesses listed BY NAME, Aristotle's byte-level 9,419-byte account model making method-level proofs intractable (kernel-scoped proofs are the reachable tier — matches our "prove only the static invariants the runtime depends on" rule). Concrete scoped follow-up: a Kani pilot on our escrow/mint/replay kernel functions (bounded harnesses, macOS aarch64 supported) — candidate riir-chain `.issues/` entry, owner-visible, not filed this session.

## Product-coverage axis

The family's ecosystem class: Solana/Anchor program repos are a LARGE Rust surface (anchor, pinocchio, spl, plus percolator-class DeFi engines) — `cargo refine` consumers. Zero corpus family today = the gap. The rules fire at home too (riir-chain programs, riir-dapps kat ledger, escrow_multi's zero-magic checks — spot-checked: `escrow_multi.rs` uses `recorded == [0u8; 16]` zero-markers, owner pinning + foreign-owner tests EXIST). Follow-up source pool (accumulation clause per the dial9 precedent): percolator itself (this verdict), `spl/token-2022` program source, anchor's `checked_account`/discriminator docs, Neodyme/OtterSec known-exploit writeups (the "common SVM vulnerabilities" literature the SKILL.md distills), `sealevel-attacks` repo. Pool sources read before the batch window fold into Batch 424.

## Caveats

- lean-svm is EDUCATIONAL-ONLY per its own README — decision-content citation only; never a runtime-adoption candidate.
- Lean compiler internals (Runtime.lean/CPI.lean/Math.lean/Compiler/*) NOT read this session (2,788 lines across 6 runtime files + the compiler tree) — every quote above comes from README/SKILL.md/spec.md, verified at the pins; verify module-level claims at mine time (T0 re-clone per Protocol A).
- The 10x percolator claim: UNVERIFIED (see the correction at top); percolator-prog not inspected.
- Process note: the first percolator pass went through a GitHub page fetch BEFORE the clone — the rendered page's line-wrapped quotes failed raw-file greps and nothing on it was pin-verifiable. Every verdict-bearing quote above was re-verified against the pinned clone; the page contributed nothing load-bearing. Standing fix: papers/posts fetch over the wire, code sources clone into `.raw/` first, always.
- Same-day sibling landing: Batch 423 (plan 417 tranche B) landed on THIS sec corpus hours ago (`54ce0ed65`, 99→104 rules) — Batch 424's entries must dedup against 423's five (wholesale-env-harvest / predictable-temp-dir / ambiguous-outcome-staging / auth-header-sensitive / package-source-redirection — all disjoint from this family, clearance confirmed by the zero-owner greps).
- Both sources Apache-2.0 — transcription-clean per the fixture law (original Rust reproductions only, never pasted source).

## Recording

- Queue slot: nominal **Batch 424**, plan **418** (`.plans/418_onchain_ledger_sec_family_batch.md`), routed to the next sec-slot lane (fleet idle lanes read the queue snapshot; this m3 session files the verdict, does not claim the batch).
- This note is the verdict record; the plan carries the proof path (per-entry 4-arm fixtures, no bench — NotBenchable-by-domain annotation, count pins 104→~109-110, retrieval floors, pins_manifest, AGENTS count web, health_views total).
