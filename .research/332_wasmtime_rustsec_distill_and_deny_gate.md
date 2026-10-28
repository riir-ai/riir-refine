# Research 332 — wasmtime RUSTSEC-2026-0379/0380 distill + the cargo-deny supply-chain gate

**Status:** RECORD — the pilot gate LANDED green in riir-refine (deny.toml + scripts/deny_gate.sh); the corpus batch (T1) LANDED 2026-10-27 (sec 26 → 28, Plan 298 / claim `3dc941e9`); the T2.4 fan-out landed 19 repos + the M3-box completion (4 more, §T2.4 M3-box completion); T2.5/T2.6 landed (auditable + --deep); the riir-ai wasmtime adjudication (T2.1) and T2.7 (CI lane) remain OWNER-gated; the remaining remediation lanes are the owning lanes' edits (recorded below).

## Source

- The Hacker News, 2026-10-26: "Two Critical Wasmtime Flaws Could Let Malicious WebAssembly Modules Escape Sandboxes" (read in full).
- RUSTSEC-2026-0380 / [CVE-2026-115889](https://rustsec.org/advisories/RUSTSEC-2026-0380.html) (CC0): Cranelift packed 24-bit value-number fields overflow silently in release builds → wrong value references → miscompilation → type confusion → sandbox escape. CVSS 9.9 (AV:N/AC:L/PR:L/UI:N/S:C/C:H/I:H/A:H). Patched `>=36.0.19,<37` or `>=43.0.0`. Upstream PR 12613.
- RUSTSEC-2026-0379 / [CVE-2026-115893](https://rustsec.org/advisories/RUSTSEC-2026-0379.html) (CC0): aarch64 `br_table` out-of-range indexes jump via a ~1 MiB-reach branch; >~256K-target jump tables silently point to the WRONG target (present since 2020). Fix routes out-of-range through the jump table AND refuses >65,520 targets. CVSS 9.9. Patched `>=36.0.19,<37` / `>=48.0.7,<49` / `>=49.0.4`. Upstream PR 14826.

## Inspection

2026-10-26, zed-m3/M3. Web reads of all three sources; full-workspace lockfile greps (`wasmtime|cranelift` over `**/Cargo.lock` + `**/Cargo.toml`; `name = "wasmi"` inventory); cargo-deny 0.20.2 installed (`cargo install cargo-deny --locked`, 2m22s) and run across every `~/git` workspace with a `Cargo.lock` via the landed `scripts/deny_gate.sh --scan`.

## The class analysis (corpus candidates — Batch 294)

Both bugs are JIT-compiler-internal, but each generalizes to a Rust-shaped userland class with a bounded fix-space. Fixtures are original syntheses per the standing constraint (the advisory is class provenance, Batch 140/Research 161 precedent).

| Candidate | Grade | The class | Fix-space | Dedup |
|---|---|---|---|---|
| `packed-field-silent-overflow-wrong-key` | B− | A count/ID packed into a fixed-width bitfield that its legitimate range can exceed (wasmtime's 24-bit value numbers); release builds lose the high bits silently (no overflow checks); the corrupt value then keys a lookup — wrong-item access, not a panic | Capacity assert at the packing site / a checked newtype that refuses construction past the field width / widen the field; regime: the packed field keys a lookup AND the count's source is unbounded or externally influenced | DISTINCT-FROM `unchecked-wire-arithmetic` (wire-taint × arithmetic-op sink vs the packing SITE — the mask/shift is the tell) and clippy `cast_possible_truncation` (blanket `as`-cast pedantic, no key-usage scoping, no fix). ⚠ MUST adjudicate the B258 recorded drop ("integer overflow/truncation owned by unchecked-wire-arithmetic + cast_possible_truncation") in-rule — that drop is the batch's main falsification risk; the sink-level-novelty defense is the Plan-156 T2 shape |
| `capacity-exceeded-silent-fallback-not-refusal` | B−/C+ | A fixed-capacity structure (jump table, fixed-size wire section, table with a target-count cap) built from variable-length input degrades SILENTLY to a wrong encoding/target when capacity is exceeded — the 0379 fix's refusal half (">65,520 targets refused") is the lesson: capacity-exceeded input must refuse loud, never silently mis-dispatch | Early capacity check + typed error at the encode/build boundary; regime: any encoder/dispatcher with a documented hard cap; in-fleet population: seal-std vessel 512-entry caps, ARTB fixed sections, NeuronShard `#[repr(C)]` fixed layouts | DISTINCT-FROM `length-prefix-validate-before-alloc` (alloc-size DoS sink vs wrong-dispatch sink — the sink-distinct adjudication pattern); no Research 161 overlap (its harvest = unbounded-read only; its dropped set = recursion/set_len/uninit/symlink) |

NotBenchable-by-domain (no perf validator in sec — the dial9/B258 annotation class, never a demotion). Probe-parity discipline applies at authoring: no default/restriction clippy lint fires on either buggy shape (expected — verify against the pinned toolchain, record in the plan). Yield estimate 1–2 after coordinator falsification.

## The measurement — first workspace scan (the headline)

`scripts/deny_gate.sh --scan` (report-only) over every `~/git` workspace with a `Cargo.lock`, cargo-deny 0.20.2, advisory DB fetched 2026-10-26. **22 FINDING-bearing contract lockfiles, 9 CLEAN.** Presence ≠ shipped exposure (dev-deps and optional features count) — the per-posture triage is the fan-out's work; the grouping below names the owning lane.

| Repo | Advisories | The causes (grouped) |
|---|---|---|
| riir-ai | 57 | **21 wasmtime** (0379+0380 the question's pair; 0095/0096 CVSS 9.0 sandbox escapes; 0269/0352-class FS escapes; WASI/fuel/panic classes — ALL affecting the pinned 28.0.1) · quinn-proto family (0185, 0338, 0339, 0340, 0342 — remote panic/OOM) · rustls (0285, 0346, 0348, 0349) + emSign (0381, 0382) · zstd (0344, 0391, 0398, 0403, 0410-wasm32) · memmap2 0186 · anyhow 0190 · crossbeam-epoch 0204 · ctutils 0385 · crypto-bigint 0387 · tokio AsyncFd 0386 · quick-xml 0194/0195 · h2 0258 · webbrowser 0257 · unmaintained (2023-0089, 0436, 0449, 0119, 0134, 0141, 0192, 0370–0372) |
| seal-online-remaster | 27 | zip privilege-escalation trio (0367/0368/0369) · rustls family (0285, 0346, 0347, 0348, 0349) + emSign (0381, 0382, 0383) · zstd family incl. 0410 · tokio 0386 · steamworks 0121 · unmaintained set · deprecated repo — lowest priority |
| riir-mmorpg-examples | 21 | quinn family (0185, 0338–0342) · rustls (0285, 0346, 0348, 0349) + emSign · quick-xml · ruzstd 0344 · webbrowser · unmaintained set |
| riir-game-sdk | 20 | same families as mmorpg-examples (minus 0192) |
| seal-remake | 14 | quinn family (0338, 0340, 0343 sender panic) · rustls + emSign · ruzstd · unmaintained set |
| riir-deployer | 7 | rustls (0285, 0346, 0348, 0349) + emSign (0381, 0382) · unsafe-libyaml |
| riir-chain | 6 | anyhow 0190 · crossbeam-epoch 0204 · ctutils 0385 · tokio AsyncFd 0386 · crypto-bigint 0387 · atomic-polyfill |
| seal-game-editor | 12 | zstd family incl. **0410 (wasm32)** · quick-xml · webbrowser · ttf-parser · zvariant family · unsafe-libyaml |
| riir-neuron-db | 3 | anyhow 0190 · crossbeam-epoch 0204 · atomic-polyfill |
| seal-render | 2 | **tokio 0384 — the YANKED 1.54.0 Interval race (only-version-affected ⇒ the lock pins exactly 1.54.0)** · unsafe-libyaml |
| katgpt-rs, riir-train | 4 each | atomic-polyfill · paste · ttf-parser · crossbeam-epoch 0204 |
| riir-infer | 2 | atomic-polyfill · paste (the inference substrate is effectively clean) |
| riir-auth, riir-dao, riir-dapps, riir-instinct, riir-kat, riir-reflex, riir-reflexer, riir-rethink | 1 each | atomic-polyfill (transitive via katgpt-core → postcard → heapless 0.7) |
| CLEAN | — | katgpt-web, riir-llm, riir-refab, **riir-refine (post-pin)**, riir-shader, riir-viewbridge, seal-std |

Out-of-contract, scanned by location: `zed` 67 findings (incl. the same wasmtime pair), `zed-mcp-server-zai-*` clean. Nested lockfiles (seal-game-editor/tools/neuron-publish, tools/seal-e2e) are outside the scan loop — a known fan-out gap.

## The riir-ai wasmtime posture (the user's actual question)

- `wasmtime 28.0.1` is the ONLY wasmtime in any contract lockfile. Reach: riir-engine `cwm_wasm_loader` (optional, "NOT default-on", GOAT 326) + riir-wasm `cognition_wasmtime` (optional; Plan 390 made **wasmi the production backend on all three tiers**; wasmtime retained for Authority-tier experimentation) + riir-wasm **dev-dependencies** (tests/benches — this is why the lock carries it).
- Both 9.9 CVEs affect 28.0.1 (no patch line reaches 28.x); the aarch64 one (0379) applies to this Mac. So do the April-2026 wave (0085–0096, two CVSS 9.0) and the rest.
- **Runtime exposure: effectively nil** — no shipped binary enables the features; nothing JITs attacker-supplied modules; wasmi 2.0.0 (interpreter, different engine) is the production path everywhere, including the CF edge (Cranelift cannot run inside wasm32 tiers — the Plan 390 record).
- **Supply-chain exposure: real and now instrumented.** The adjudication is an OWNER call: (a) bump wasmtime 28 → a patched line (36.0.19+/43+/48.0.7+) — a major-semver migration of two experimental features; or (b) pin the 21 wasmtime advisories in a riir-ai `deny.toml` with reasons + a tracking issue, and bump when the features graduate. The pilot's recommendation: (b) now, (a) at graduation — pin-with-reason is the workspace's established ledger shape.

## The remediation lanes (named to owners — NOT this lane's edits)

1. **quinn-proto ≥ 0.11.18** (remote panic + OOM classes 0185/0338–0343) — the riir-net/webtransport lane (riir-ai, game-sdk, mmorpg-examples, seal-remake). The game network is the workspace's most exposed real surface.
2. **zstd stack**: ruzstd ≥ 0.9.1, zstd ≥ 0.14.3, zstd-safe ≥ 8.2.0, zstd-sys ≥ 2.1.2 (incl. **0410, wasm32-only allocator corruption — the web-bundle posture** in seal-game-editor/seal-online-remaster/riir-ai) — the vessel/texture compression lane.
3. **rustls ≥ 0.23.46 + webpki-roots ≥ 1.0.10 + rustls-webpki ≥ 0.103.16** (TLS 1.3 level confusion, Debug-secret leak, brotli pre-auth alloc, PQ downgrade, the emSign distrusted intermediates CVE-2026-111943 with well-known private keys) — every TLS-bearing repo.
4. **tokio ≥ 1.54.1 in seal-render** (0384 — the yanked 1.54.0 pin; trivial bump).
5. **Small real bumps**: anyhow ≥ 1.0.103 (0190 unsound downcast_mut), crossbeam-epoch ≥ 0.9.20 (0204), quick-xml ≥ 0.41.0 (0194/0195 DoS), memmap2 ≥ 0.9.11 (0186 — **check the NeuronShard zero-copy mmap lane's pinned version first**), webbrowser ≥ 1.2.2 (0257), h2 ≥ 0.4.16 (0258), ctutils ≥ 0.4.4 + crypto-bigint ≥ 0.7.6 (0385/0387 constant-time — relevant to any wallet/signing consumer).
6. **Unmaintained hygiene (upstream-driven)**: atomic-polyfill (18 repos; fix = heapless 0.8/postcard bump in katgpt-rs), paste, unsafe-libyaml, bincode, rustls-pemfile, ttf-parser, number_prefix, rustybuzz, zvariant family — per-repo pins with reasons where no upgrade exists.

## The cargo-deny pilot — measured config notes

- 0.20.2 defaults `unmaintained` to DENY level: the gate's first run red on RUSTSEC-2023-0089 (transitive, "no safe upgrade available") — unmaintained ≠ vulnerable.
- v2's `[advisories] unmaintained`/`yanked` keys are SCOPE selectors (`all/workspace/transitive/none`), NOT severities; there is no `[lints.rustsec]` severity table in 0.20.2. Measured path: severity control lives in the **ignore ledger with a reason per row** — which is the workspace's membership-pin convention anyway. The pin discipline: a finding is FIXED (bump) or PINNED (reason) — never silenced silently.
- `scripts/deny_gate.sh`: gate mode (findings ⇒ exit 1) + `--scan` survey mode (report-only), completion sentinel (the macOS-bash-3.2 trap-laundering lesson), loud skip when cargo-deny is absent, `~/.cargo/bin` fallback resolution, per-repo RUSTSEC-ID extraction in scan output.

## The architecture verdict (the "like clippy" correction)

The clippy lane is a **span oracle** feeding span healing. cargo-deny is a **lockfile oracle**: its findings are crate/version-level, never span-healable. The two fit together as: (a) deny = the gate (crate-level coverage — the corpus can never own that surface); (b) RustSec = a continuous TRANSCRIPTION feed for the sec corpus (advisory → class → rule when the fix is code-shaped) — Batch 140 was intake #1, this is #2, and the gate makes the feed mechanical (every new advisory hit in a scan is a transcription candidate). cargo-audit is subsumed (same advisory DB; one tool, one ledger); cargo-auditable is a third, different thing — release-lane transparency for the shipped binaries (plan task).

## Caveats

- Scan is lockfile-level: dev-deps/optional features counted; presence ≠ shipped. Per-posture triage belongs to the remediation lanes.
- Exact resolved versions per crate per repo not enumerated here (advisory presence proves a below-patched-floor resolve; the plan's bumps re-resolve).
- zed + zed-mcp-* are out-of-contract neighbors, scanned by directory location — recorded for completeness, not contract findings.
- The workspace had ZERO supply-chain instrumentation until this pilot — 22 finding-bearing lockfiles incl. two CVSS-9.9 and two CVSS-9.0 classes is the measured proof the gate was owed.
- Unread: none of the contract-repo advisory IDs (all 60+ distinct IDs fetched and classified this session); the zed-only tail (a few IDs) classified by family.

## Queue slot

Batch 294 / `.plans/298_batch294_rustsec_transcription_and_deny_gate_fanout.md` (plan-first; the plan owns the corpus batch, the fan-out, and the remediation-lane list). `.raw/` not used (no clone needed — advisories are web documents; the scan needs no checkout).

## T2.4 fan-out slice — LANDED 2026-10-27 (zed-amo-b294-fanout, amo; cargo-deny 0.20.2 prebuilt binary)

16 small-ledger repos adopted the gate (deny.toml + scripts/deny_gate.sh, named-path commits, all first scans PASS at the landed ledger):

| Repo | Ledger | Commit |
|---|---|---|
| riir-llm · riir-shader · seal-std | EMPTY (scan CLEAN) | `324df3c` · `73542ea` · `8c8d8ee` |
| riir-auth · riir-dao · riir-dapps · riir-instinct · riir-kat · riir-reflex · riir-reflexer · riir-rethink | 2023-0089 only | `a7cfdd8` · `4ebc76e` · `915b39e` · `31508e5` · `80b929f` · `ca2eb39` · `b24d13c` · `d41d203` |
| riir-neuron-db | 2023-0089 only — **dropped from 3 findings to 1 since this scan** (a dep bumped; re-scan before pinning more) | `44e2ee0` |
| riir-infer | 2023-0089 + 2024-0436 (paste, via gemm/laya) | `1f99c7e` (worktree push, see note) |
| seal-render | 2026-0384 (yanked tokio 1.54.0 Interval race — exactly-version-affected, reach = wasm-bindgen-futures under bevy on the wasm32 edge) + 2024-0449 (unsafe-libyaml via deprecated serde_yaml, DEV-only) — both pinned with the seal-render bump lane named (plan 298 T2.2) | `d92752e` |
| riir-chain | 0089 + 0190 (anyhow 1.0.102) + 0204 (crossbeam-epoch 0.9.18) + 0385 (ctutils 0.4.2) + 0386 (tokio 1.52.3 AsyncFd) + 0387 (crypto-bigint 0.5.5+0.7.5 via k256/ecdsa) — every bump row names its plan 298 T2.2 owning lane | `cce46874` |
| katgpt-rs (PUBLIC) | 0089 + 0436 (paste via metal 0.33) + 0192 (ttf-parser via plotters) + 0204 (crossbeam-epoch) — the atomic-polyfill FIX lane is THIS repo (T2.3); the ledger says so explicitly | `3478518d9` |

Fan-out notes:
- **Box deltas vs this file's first scan:** katgpt-web / riir-refab / riir-viewbridge are NOT checked out on amo — their adoption waits for a box that carries them. riir-deployer likewise absent here (7 findings, stays for its lane). riir-mmorpg-examples scan ERRORed (stale-lock class — needs a re-lock read, its lane's call).
- **Portability fix rode first:** the pilot's `--scan` hardcoded `/Users/kat/git/*/`; `be23c8dd` derives the farm root from the script location.
- **Two push races recovered per the shared-worktree recipe:** dapps + infer had siblings push between commit and push (my success-grep false-POSITIVED on the rejection text — the grep matched "develop" in `! [rejected]`); both landed via the temp-worktree cherry-pick path (`915b39e`, `1f99c7e`), local checkouts realigned by update-ref + mixed reset, sibling WIP untouched throughout. Lesson: a push-success check must grep for the ref-update arrow, never the branch name.
- NOT in this slice: riir-ai (T2.1 owner-gated adjudication + contended lane), riir-train (routing restriction; trainers live), the big sets (game-sdk 20 / seal-remake 14 / seal-game-editor 12 / mmorpg-examples 21 / deployer 7 / seal-online-remaster 27-deprecated) — a follow-up slice under the same claim until expiry, else re-claim.

## T2.4 big-set slice — LANDED same cycle (the second unit, 2026-10-27 ~03:5x +07)

The three remaining on-box big sets adopted with FULL per-finding pins (titles + crate versions read from fresh scans; every bump row names its plan-298 T2.2 owning lane — quinn→riir-net, rustls/emSign→the rustls lane, zstd stack→the zstd/bundle lanes, quick-xml/webbrowser→the small-bump set):

| Repo | Ledger (all PINNED, scan PASS) | Commit |
|---|---|---|
| seal-remake | 14 rows: 0089 · 0141 Bincode · 0134 rustls-pemfile · 0449 unsafe-libyaml · quinn 0338/0340/0343 · rustls 0285/0346/0348/0349 · emSign 0381/0382 · ruzstd 0344 (the vessel reader's caps noted) | `4c7a15e` |
| riir-game-sdk | 20 rows: the seal-remake set + quinn 0185/0339/0341/0342 + quick-xml 0194/0195 + webbrowser 0257 | `54c137a` |
| seal-game-editor | 12 rows: ttf-parser 0192 · unsafe-libyaml 0449 · quick-xml 0194/0195 · webbrowser 0257 · zbus/zvariant 0370/0371/0372 (Linux-desktop transitive, editor runs on macOS) · zstd 0391/0398/0403/0410 (0410 = the wasm32 allocator shim, browser-bundle posture) | `d636452c5` |

New unmaintained IDs classified this pass: 2025-0134 rustls-pemfile (superseded by rustls-pki-types), 2025-0141 Bincode, 2026-0370/0371/0372 the zbus/zvariant family. seal-online-remaster (27 rows, deprecated) stays scan-only; riir-mmorpg-examples scan ERROR (stale-lock class) needs its lane's re-lock before adoption; the not-on-amo repos (riir-ai / katgpt-web / riir-refab / riir-viewbridge / riir-deployer) + riir-train stay with their lanes as recorded above.

## T2.4 M3-box completion — LANDED 2026-10-27 ~07:1x +07 (zed-m3-t24-residue, M3; cargo-deny 0.20.2)

The not-on-amo repos from the fan-out's box-delta note adopted on the M3 (each: the pilot deny_gate.sh verbatim + a per-repo deny.toml + the AGENTS.md Build-Commands line; `staged_set_audit.py` before every commit; all first scans PASS at the landed ledger):

| Repo | Ledger | Commit |
|---|---|---|
| riir-refab | EMPTY (scan CLEAN) | `5d4c9d7` |
| riir-viewbridge | EMPTY (scan CLEAN) | `abfe3df` |
| katgpt-web | EMPTY (scan CLEAN) | `ae371e6` |
| riir-deployer | 0449 only (unsafe-libyaml via serde_yaml 0.9, archived upstream — pin names the owner-scoped serializer-migration lane) + the six TLS advisories FIXED rather than pinned: rustls 0.23.43→0.23.46 (0285/0346/0348/0349), rustls-webpki 0.103.15→0.103.16 (0382), webpki-roots manifest `"0.26"`→`"1"` with lockfile 1.0.11 (0381 emSign roots — 0.26.11 was a wrapper re-exporting 1.0.9's data; `TLS_SERVER_ROOTS` drop-in, `cargo check` green) | `bd5b712` |

riir-deployer was the one FINDINGS carrier: its six rustls-family advisories were all bumpable within existing semver requirements (the only manifest move is webpki-roots 0.26→1), so the deployer ledger lands at ONE pinned row — the first T2.4 ledger to prefer the fix path where the bump is in-requirement.

riir-neuron-db: the M3 box's local lock was stale at anyhow 1.0.102 / crossbeam-epoch 0.9.18 (the T2.3-discharge commit `3fd7050` recorded 1.0.104/0.9.21 floors as already-met on its box); `cargo update -p anyhow -p crossbeam-epoch` aligned this box's (gitignored, untracked) lock to the recorded floors and the local gate re-reads GREEN. No commit owed — ndb does not track its lockfile; the findings were a property of the local lock, never the repo surface.

Full-farm re-scan from the pilot (35 roots): every landed ledger's root row CLEAN; remaining unpinned FINDINGS rows unchanged from the recorded state (riir-ai owner-gated T2.1, riir-train routing-restricted, seal-online-remaster deprecated scan-only, zed + jup-tool + mmorpg-examples outside the contract set / off-box / stale-lock).

## T2.5 + T2.6 — LANDED 2026-10-27 (zed-amo-b294-t25t26, amo)

**T2.5 — cargo-auditable in the dist release lane** (riir-refine `.github/workflows/release.yml`): the build job installs `cargo-auditable` via `taiki-e/install-action@v2` (prebuilt; listed in the action's supported-tools table — no compile cost) and the dist build runs `cargo auditable build` — a pass-through wrapper, same flags, embedding the resolved dependency tree as the AUDITABLE_VERSION_INFO data section so `cargo audit bin`-class scanners can attribute vulnerable deps in the SHIPPED binaries without the source tree. YAML parse-validated locally; first exercised at the next `v*` tag run (release lanes are tag-triggered — the dry-run dispatch also carries it).

**T2.6 — `deny_gate.sh --scan --deep`**: one cargo-deny pass PER nested Cargo.lock (sub-workspaces excluded from their repo's root workspace), reusing the repo-root deny.toml where present — advisory pins are per-advisory-ID and path-free, so the same repo's ledger is the right config; no root deny.toml = default config (may report pinned advisories as findings — a read-the-row row, never a silent pass). Hidden/ephemeral trees (dot-dirs, target/, vendor/, node_modules/) are PRUNED in the find (traversal skipped, giant target/ stays cheap); a lockfile without a manifest beside it SKIPs loud. Report-only like `--scan`; plain `--scan` is unchanged. The header's former "Known scope gap" paragraph is replaced by the mode doc.

First deep pass, 13 nested locks across the farm (root-scan rows unchanged from the T2.4 ledgers except: riir-chain / riir-infer / seal-render / katgpt-rs now scan CLEAN at root — their T2.4 pins hold; riir-ai + riir-train + zed remain unpinned FINDINGS rows as recorded):

| Nested lock | Verdict | Note / owning lane |
|---|---|---|
| reflex-site/wasm-head | CLEAN | default config (no root deny.toml) |
| riir-chain/crates/riir-chain-engine-bridge | CLEAN | |
| riir-rethink/cloudflare/access | CLEAN | |
| riir-mmorpg-examples/cloudflare/container-tick-test | CLEAN | (its ROOT lock is the known stale-lock ERROR — nested workers scan fine) |
| riir-train/poc/discoloop_lab | CLEAN | |
| riir-ai/repro/wgpu_metal_contention | FINDINGS 2024-0436 | the never-wired reproducer — pin or leave, riir-ai T2.1's lane |
| riir-dapps/cloudflare/kat-service | FINDINGS 0285 · 0346 · 0348 · 0349 · 0381 · 0382 | the rustls + emSign sets — the T2.2 rustls lane; worker dep re-lock |
| riir-mmorpg-examples/cloudflare/warm-tier-do | FINDINGS 2023-0089 | atomic-polyfill class — T2.3 lane |
| riir-mmorpg-examples/wasm | FINDINGS 0089 · 2024-0449 | atomic-polyfill + unsafe-libyaml (serde_yaml dev class) — T2.3 lane |
| riir-train/poc/mdl_collapse_probe | FINDINGS 2023-0089 | poc scratch, atomic-polyfill class — T2.3 lane (routing-restricted) |
| seal-game-editor/tools/neuron-publish | FINDINGS 14: 0089 · 0192 · 0194 · 0195 · 0257 · 0370 · 0371 · 0372 · 0381 · 0382 · 0391 · 0398 · 0403 · 0410 | the editor's ROOT ledger (12 rows) does NOT cover its tools/* trees — they pull their own dep set incl. zbus/zvariant + zstd; the editor lane re-locks/pins per-tool when it next touches them |
| seal-game-editor/tools/seal-e2e | FINDINGS 14: 0089 · 0192 · 0194 · 0195 · 0257 · 0346 · 0347 · 0348 · 0349 · 0370 · 0371 · 0372 · 0381 · 0382 | same class as neuron-publish (+ quinn 0346/0347) — editor lane |
| seal-game-editor/tools/legacy-import | ERROR (stale lock) | manifest moved, lock not re-generated — the editor lane's re-lock, never a silent pass |

New ID this pass: 2026-0347 (quinn). Scope note: T2.6 delivers the INSTRUMENT; adjudicating the nested FINDINGS rows belongs to the owning lanes per the T2.2 discipline — this lane records, never bumps.

## T2.3 — LANDED 2026-10-27 (zed-amo-idle, amo; the katgpt-rs lane)

Claim `9e0ddb38` (epoch 1793047403) → landed same cycle. **The mechanism was NOT a version bump** — postcard 1.1.3 is the latest 1.x the resolver offers (`cargo update -p postcard --dry-run`: "Locking 0 packages") and postcard has no 2.x; a direct heapless requirement cannot help (postcard pins `^0.7`, semver-incompatible with 0.8/0.9 — both versions would coexist). **The actual mechanism: postcard's `default = ["heapless-cas"]` feature was the ONLY heapless edge** (crates.io manifest verified: `heapless-cas = ["heapless", "dep:heapless", "heapless/cas"]`), and heapless 0.7 is the only version carrying atomic-polyfill (0.9.3 already in-tree via bevy_tasks, clean). Fix: all **7** workspace postcard declarations (root, katgpt-core, katgpt-attn-match, katgpt-deprecated, katgpt-pruners, katgpt-proof-cert, katgpt-speculative-optional) → `default-features = false` (alloc kept; no workspace source references heapless).

Measured gates (zero-cargo until the i1117 flashmemory timing measurement verified ended; then isolated `CARGO_TARGET_DIR=/tmp/t23_check`):

| Gate | Result |
|---|---|
| `cargo update --workspace` | lock: atomic-polyfill 0 matches; heapless unified on 0.9.3; postcard deps = cobs/embedded-io×2/serde (no heapless) |
| `cargo tree -i atomic-polyfill` | "did not match any packages" |
| `cargo deny check advisories` | ok — with the 0089 row REMOVED from katgpt-rs deny.toml (the discharged row leaves the consumer disclosure in its comment) |
| clippy ×5 non-optional consumers (+ speculative `--features rt_turbo`) | ALL clean |
| lib tests: postcard-named + closure module (the heaviest postcard user) | 66 passed / 0 failed |

Scope note: the nested-lock 0089 rows recorded under T2.6 (mmorpg-examples wasm + warm-tier-do, train poc, editor tools ×2) are OUT of this lane — their owning lanes re-lock and drop their pins when they next touch those trees; the katgpt-rs deny.toml comment carries the drop-your-pin disclosure. The no-upgrade-crate pin rows (paste / ttf-parser / crossbeam-epoch classes) were already landed with T2.4's ledgers (every row reason + tracking reference) — no second pass needed.

## T1 — LANDED 2026-10-27 (shikuwa-4090-b294-rustsec-t1, the 4090 box; the corpus batch)

Claim `3dc941e9` (epoch 1793053492) → landed same cycle. The class analysis table's two candidates shipped VERBATIM (grade B− both, no drops): `packed-field-silent-overflow-wrong-key` (the 0380 class — mask-site sink, the B258 recorded drop adjudicated in-rule) + `capacity-exceeded-silent-fallback-not-refusal` (the 0379 class — the lying-cap face, distinct-from the alloc rule on the missing-check vs wrong-dispatch axis). Sec corpus 26 → 28 (6 fixable + 22 suggest-only).

Landed surfaces: `src/domains/sec/proposers/packed_field_wrong_key.rs` (10 tests), `src/domains/sec/proposers/capacity_silent_truncate.rs` (9 tests), corpus entries + header table rows, `eval_fixtures_sec` +2 spans (both routed correctly — router annotation re-pinned 60/167 → 62/169, measured 67/169 = 39.6%), count pins 26 → 28 via one shape-asserted script pass (`.plans/298_count_pins_b294.py`), the doc tables (AGENTS ×2, README ×2, sec.md §Batch 294).

Gates: sec module 215/0; sec lib 2964/0; default lib 2749/0; clippy −D ×4 postures + all-targets+sec zero; T7 wide probe (pedantic+nursery+suspicious+correctness+perf+style+complexity over both buggy fixtures): ZERO default/suspicious/correctness safety lints — `cast_possible_truncation` fires on r2's unrelated `as u16` length-encode (the B258-owned axis, not the rule's site, no fix for the degrade shape) + two nursery fixture-noise candidates (`missing_const_for_fn`, `must_use_candidate`); boundary gate 2/2 — `indexing_slicing` fires on the packed-key fixture with no machine fix (the ffi-export additive precedent), silent-cap fully silent; score-bench run 204 (gen-1 appended); freeze NOT owed (`generations.rs::atoms()` builds from the clippy seed corpus only — verified live, the P30 confirm).

## Addendum 2026-10-28 — the T2.2/T2.4 residue closed from the M3 box (m3-idle-deny-sweep)

The four not-on-amo repos + the two named lanes landed; the workspace
`--scan` now reads CLEAN everywhere except the three deliberate skips
(riir-ai = T2.1 owner-gated, riir-train = routing-restricted,
seal-online-remaster = deprecated) + zed (dev-fork upstream, not a
contract repo):

| lane | landing | note |
|---|---|---|
| riir-game-sdk | `4c8e403` + `15e38ec` | the sdk lane's own T2.2 residue: quinn-proto 0.11.19 / rustls 0.23.46 / rustls-webpki 0.103.16 / webpki-roots 1.0.11 / clipboard-win 5.4.3 / chacha20 0.10.2 / webbrowser 1.2.5, all scoped; 13 rows discharged (quinn family, rustls family, emSign pair, webbrowser 0257); rustls-pemfile 0134 STAYS (its 2.2.0 ghost survives the scoped update) |
| seal-render | `a4111eb` | the tokio 1.54.1 lane executed; 0384 discharged |
| riir-mmorpg-examples | `abec128` + `66eecb3` + `f1b159f` | the stale-lock gap ADOPTED: deny.toml + gate + 10-row ledger, first scan PASS zero warnings; the same scoped bumps rode in (its lock was refreshed at 1739e67); webbrowser discharged after |
| jup-tool | `cb55582` + `1484202` | ADOPTED with an EMPTY ledger: rustls 0.23.46 / rustls-webpki 0.103.16 / webpki-roots 1.0.11 scoped (the emSign family bumped away); AGENTS.md seeded |
| riir-dapps (nested) | `d6e70ed` | the kat-service worker re-lock: the six 0285/0346/0348/0349/0381/0382 findings from the first --deep run discharged |
| seal-game-editor (tools) | `a55992aa9` | the three nested locks take the rustls/emSign set: neuron-publish 0381/0382 + seal-e2e 0346–0349 discharged; legacy-import 0089 discharged upstream |
| seal-online-remaster | `fae7c0d` | the ROOT CAUSE fix for the lingering 0089 class: riir-neuron-db's workspace-scoped [patch] does not travel through path deps, so this workspace resolved the pre-T2.3 katgpt-core@main — the patch table (8 rows, the riir-auth precedent) + the two postcard default-features=false declarations + the 1.99.0 toolchain pin landed; heapless 0.7 + atomic-polyfill leave every lock resolving this graph |

Lesson recorded (the [patch]-does-not-travel class): a consumer workspace
resolving a path-dep's GIT deps resolves the pinned GIT source unless IT
owns the patch table — the advisory surface then freezes at the pinned
branch's vintage (main@37bb9cbf predates the T2.3 fix by months). Every
workspace that path-deps riir-neuron-db and lacks its own [patch] table
carries the same ghost; the seal-online-remaster fix is the template.

Nothing in this addendum re-pins a ledger row — every discharge names the
bump that made the advisory stop matching, and the remaining FINDINGS
rows above stay owned by their named lanes.
