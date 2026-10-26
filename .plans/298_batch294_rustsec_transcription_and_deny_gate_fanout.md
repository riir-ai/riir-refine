# Plan 298 — Batch 294: RustSec transcription batch + the supply-chain deny-gate fan-out

**Status:** IN PROGRESS — pilot LANDED 2026-10-26 (`deny.toml` + `scripts/deny_gate.sh` green in riir-refine; the first workspace scan recorded in `.research/332`; corpus batch + fan-out open below)
**Claimed:** zed-amo-b294-fanout, 1793042891, expires 2026-10-28T19:28:11Z
Scope: T2.4 fan-out slice — every repo whose ledger is ≤ 6 rows (the 6 CLEAN + the 8 atomic-polyfill-only + neuron-db/seal-render/riir-infer/riir-chain/katgpt-rs). NOT in this claim: T1 (the corpus batch — jumps no sec-pool queue), riir-ai (T2.1 owner-gated adjudication + contended lane), riir-train (routing restriction, trainers live), the big-family sets (game-sdk/seal-remake/seal-game-editor/mmorpg-examples/deployer/seal-online-remaster).

Source: RUSTSEC-2026-0380 (CVE-2026-115889, Cranelift 24-bit packed value-number silent overflow) + RUSTSEC-2026-0379 (CVE-2026-115893, aarch64 br_table wrong-target), both CVSS 9.9, both affecting the wasmtime 28.0.1 pinned in riir-ai/Cargo.lock. Evidence + class analysis: `.research/332_wasmtime_rustsec_distill_and_deny_gate.md`.

## T1 — the corpus batch (sec domain, 2 suggest-only rules)

- [ ] T1.1 Dedup re-verification at authoring: 3+ vocabulary greps per candidate over `riir-refine/src` (packed / bitfield / shift-mask / silent-overflow / wrong-key; capacity / refuse / fallback / silent / degrade / dispatch), incl. the Research 161 renewal check and the B258 recorded-drop adjudication carried in-rule (the batch's main falsification risk)
- [ ] T1.2 `packed-field-silent-overflow-wrong-key` (the 0380 class): original fixtures, parse detector + zero-FP negative suite, real-clippy probe evidence (expect silent — record per the T0.4 boundary), NotBenchable annotation, suggest-only (no FIXABLE_SEC arm), distinct-from lines for `unchecked-wire-arithmetic` + clippy `cast_possible_truncation`
- [ ] T1.3 `capacity-exceeded-silent-fallback-not-refusal` (the 0379 class): same discipline; distinct-from `length-prefix-validate-before-alloc` (alloc-DoS sink vs wrong-dispatch sink)
- [ ] T1.4 Corpus header table rows + count pins via ONE shape-asserted script pass + sec-family counts in AGENTS.md/docs
- [ ] T1.5 Gates: sec module tests, default lib, clippy −D ×3 postures, retrieval floors re-measure (corpus delta — re-pin with attribution if moved), freeze gen2/gen3 regen (corpus-touching landing, plan 137 T7.5), score-bench appended, release-set binary rebuild + install BEFORE push (sec compiles into the default build — the §0 bin-age law)

## T2 — the deny-gate fan-out

- [ ] T2.1 riir-ai: `deny.toml` + gate adoption; THE ADJUDICATION (OWNER-GATED): bump wasmtime 28→patched line (major migration of `cwm_wasm_loader`/`cognition_wasmtime`) vs pin the 21 wasmtime advisories with reasons + a tracking issue. Pilot recommendation: pin now, bump at feature graduation
- [ ] T2.2 Remediation lanes named to owners (recorded in Research 332 §lanes; the bumps are the owning lanes' edits, not this lane's): quinn-proto ≥ 0.11.18 (riir-net lane) · zstd stack incl. zstd-sys ≥ 2.1.2 for wasm32 bundles · rustls ≥ 0.23.46 + webpki-roots ≥ 1.0.10 · tokio ≥ 1.54.1 in seal-render · the small-bump set (anyhow/crossbeam-epoch/quick-xml/memmap2/webbrowser/h2/ctutils/crypto-bigint)
- [ ] T2.3 Unmaintained hygiene: atomic-polyfill (heapless/postcard bump = the katgpt-rs lane) + the per-repo pin ledger rows for the no-upgrade crates
- [x] T2.4 Per-repo `deny.toml` + gate adoption following the pilot pattern — **DONE 2026-10-27 for every repo ON amo (zed-amo-b294-fanout): 19 repos landed** — the 16-repo small-ledger slice + the big sets seal-remake (14→pinned, `4c7a15e`), riir-game-sdk (20→pinned, `54c137a`), seal-game-editor (12→pinned, `d636452c5`); every bump row names its plan-298 T2.2 owning lane; all first scans PASS at the landed ledgers. Landing table + hashes: `.research/332` §T2.4. NOT adoptable from this box: riir-ai (T2.1 owner-gated adjudication + contended lane), katgpt-web/riir-refab/riir-viewbridge/riir-deployer (not checked out on amo), riir-mmorpg-examples (first scan ERROR — stale-lock class, needs its lane's re-lock), riir-train (routing restriction), seal-online-remaster (deprecated, lowest priority — scan-only 27 rows recorded in research 332)
- [ ] T2.5 cargo-auditable in release.yml (gist-rs dist binaries) — the build command gains `cargo auditable build`; CI installs the subcommand
- [ ] T2.6 Nested-lockfile scope (tools/neuron-publish, tools/seal-e2e, tools/legacy-import): per-dir gate runs or a `--scan --deep` mode
- [ ] T2.7 CI lane decision (OWNER-GATED, the spending-limit posture): a main-push-only advisories job with a cached advisory DB vs workstation-only; document whichever is chosen in AGENTS.md
- [ ] T2.8 AGENTS.md: the Build Commands block gains the deny_gate.sh line + the sec-domain note ("cargo-deny owns vulnerable CRATES at lockfile level; the corpus owns vulnerable USAGE")

## T3 — records

- [x] T3.1 Pilot + first scan + Research 332 + the queue snapshot row (this commit)
- [ ] T3.2 Batch landing record at T1; queue snapshot discharge row
- [ ] T3.3 Commit+push per unit; rebase non-interactive over any sibling lane movement
