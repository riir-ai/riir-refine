# Plan 418 — Batch 424: on-chain ledger state-machine sec family (lean-svm + percolator transcription)

**Status:** QUEUED — unclaimed; routed to the next sec-slot lane via `.distill/001_mining_queue_snapshot.md` (verdict row 2026-11-26). Claim with a `**Claimed:**` lease line BEFORE implementing (own line, nothing after the expiry token; the push is the arbiter).

Verdict record: `.research/452_lean_svm_percolator_sec_family.md` — YES B+ corpus (sec, NEW family seed, zero owners) / lane-intel HIGH / product-coverage YES.

Sources (pins, both Apache-2.0, both educational-only per their own READMEs — decision-content citation only, never runtime adoption):
- `aeyakovenko/lean-svm` @ `dd409de0246dbf6ca132a1c615575f659d00e64c` — load-bearing artifact: bundled `.agents/skills/svm-security-testing/SKILL.md` (10-class SVM vulnerability taxonomy, per-class bug/fix/Lean-proof/LiteSVM-test).
- `aeyakovenko/percolator` @ `afdadecaf807ff6f39a7f752e35d0cc5b2042ae1` — production-shape confirmations (spec.md L77 rounding law; README no-atomic-mutation + honest-proof-ledger discipline).

## Phase 0 — claim + T0

- [ ] Claim lease + push BEFORE any corpus edit
- [ ] T0 re-clone BOTH sources at the pins into `.raw/` (never a page fetch — the Research-452 process note); re-verify every candidate's source quotes byte-exact (Protocol A); re-run the upstream-state greps (all rows UPSTREAM: present — decision content only, never an outbound-PR angle)
- [ ] Dedup re-verification at landing HEAD: family greps (tombstone / revival / self-transfer / distinct-key / share-pool / 4626 / rounding-direction / program-id) + explicit read-vs Batch 423's five (landed `54ce0ed65` same day — disjoint by family, confirm by READ not pattern)
- [ ] rust_perf + clippy corpora zero-adjacency confirm (rounding / muldiv greps) — sec is the only route

## Phase 1 — entries (target 5 ± 1: R1–R5 solid; R6 optional, first drop)

- [ ] **R1 `division-rounds-against-the-account`** (LEAD, A− content): floor user-favorable amounts / ceil obligations-fees-debt, direction-as-policy, dust-extraction-loop harm; percolator spec.md L77 + README L49 + lean-svm §10 + `Math.lean` `mulDiv64`/`mulDiv64Ceil` provenance. In-rule adjudication BOTH directions vs `unchecked-wire-arithmetic` (bare-op wrap vs checked-direction policy) + `packed-field-silent-overflow-wrong-key` (bit-loss vs rounding-side)
- [ ] **R2 `account-revival-tombstone-close-discipline`** (B+): permanent nonzero tombstone distinct from every live magic + zero body on init AND close + exact `len == size`; folds §2's dedicated-discriminator init-flag half OR splits — decide at authoring on fixture shape
- [ ] **R3 `share-pool-first-depositor-boundary-states`** (B+): the four ERC-4626 boundary states (`totSh==0 ∧ assets>0` / `assets==0 ∧ totSh>0` / donation inflation at `totSh==1` / `receipt > total`) + trapped-rent closure half; each boundary state is a falsifier arm
- [ ] **R4 `account-pair-aliasing-distinct-key`** (B+): explicit distinct-key checks on aliasable pairs; the no-op self-transfer conservation break (source==dest / from==to / partyA==partyB). Adjudicate vs `packed-field-silent-overflow-wrong-key` (the corpus's one "aliasing" hit — bit-packing axis, distinct)
- [ ] **R5 `cpi-callee-id-unpinned`** (B−): a callee/program identifier arriving from the untrusted request executes unpinned; BOTH-directions adjudication vs `external-helper-config-pinned-invocation` (B389 atlas — same genus "callee identifier must come from trusted config", spawn vs CPI sink); carry the home ANTI-REGIME arm (riir-chain's compiled-match dispatch is the structural counter-instance)
- [ ] **R6 (optional)** `stale-leg-refresh-before-collateral-reduction` (B−): refresh-stamp equality across all active legs before collateral-reducing ops; DROP at authoring if the regime stays unreachable in a bounded Rust fixture
- [ ] Drops stay dropped (recorded): PDA pre-fund brick + `is_signer` (SystemProgram-vocab-specific, no bounded Rust-general shape); abort-semantics law (§"Error paths need zero cleanup" — positive instance AT HOME via MintCeiling "refusals write nothing" + dapps envelope refusals)
- [ ] Every entry: suggestion-only `NotBenchable-by-domain`, `bench_spec: None` (the dial9/136 annotation — domain-unbenchable, never a demotion); ORIGINAL Rust fixtures only (never pasted source — license + binary-only law); 4-arm discipline (buggy / fixed / anti-regime / tamper), standalone-compiled AND executed pre-entry at debug AND -O
- [ ] Pre-registered falsifiers: R5 dies if the B389 adjudication reads as duplicate; R6 dies on regime reachability; R1 dies if no two-regime shape (floor-side vs ceil-side) survives its fixture; R2 dies if the init-flag fold produces an unbounded rule

## Phase 2 — standing gates + count web

- [ ] sec lib suite green; count pins 104 → landed total (`grep -c rule_id` at landing HEAD)
- [ ] Retrieval floors both lanes — measure, hold or re-derive per the Plan-030 convention (family add shifts the blended pool)
- [ ] `pins_manifest --write` ONLY on genuine corpus delta; manifest-and-prose agree same commit
- [ ] AGENTS.md §sec count row + health_views tree total + the corpus count web in the same commit
- [ ] Score-bench/freeze NOT owed (zero clippy_lints delta — assert); release-set binary rebuild NOT owed (suggestion-only sec delta — assert, the B368/B402/B409/B423 convention)
- [ ] `.highwater` bumps ride the landing commit; dual-allocation gate at allocation AND after the final rebase; note 452 + queue row updated with the landing hash

## Phase 3 — close

- [ ] `rm -rf .raw/lean-svm .raw/percolator` + `/tmp` fixture targets at close (teardown law)
- [ ] Commit `feat:` with Session line; push; post-mining misses logged for any entry that dies at gate
