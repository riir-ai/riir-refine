# 356 — Impeccable Design-Detector Distill

**Status:** DISTILLED — Batch 316 LANDED (5 rust_perf io/process rules + 1 rider on prefix-sum-2d; the landing commit carries the verdict's full disposition; the design-taxonomy half stays owner-gated below) — and the stretch tier COMPLETED at Batch 318 (the generation-counter timer + the exclusive-mkdir reservation landed as entries_56, both NotBenchable; 3 of 3, plan 316's honest partial discharged)
**Date:** 2026-10-30 (zed-m3-impeccable-distill/M3, user-directed dual @research+@distill)
**Queue slot:** nominal Batch 316 / Plan 316 — staged behind the live queue

## Source

- **Repo:** `pbakaus/impeccable` — "The design vocabulary for your coding agent" (95.7k stars, 5.6k forks)
- **Provenance pin:** `02523aec3e4c4580102d996d1b6a71fbe062e03f` (shallow clone, HEAD verified; Apache-2.0 license read at the pin; quotes below byte-verified at this sha)
- **Shape:** 259k LOC Rust workspace (15 crates) powering an agent skill: 24 design command playbooks (steer/review/finish/iterate) + a **62-rule deterministic AI-slop detector** (4 engines: regex, static-html, browser/CDP, visual/screenshot-pixels). CLI `npx impeccable detect`, agent hook, live browser mode.
- **Provenance chain:** started from Anthropic's frontend-design skill (per README); the design-skill space has prior art (Hallmark, MIT, 1.8k stars — found in the §4 sweep; impeccable is the scaled production entry).

## Verdict

**YES B− corpus [3–5 own rust_perf rules — io/process robustness, all NotBenchable-by-domain per the dial9 split] / league-lane N/A STATED (no GPU/serving surface — a frontend-design tool) / product-coverage MODERATE-HIGH (std::net + std::process robustness; the lead class VERIFIED FIRING AT HOME on two production serve edges) / DESIGN-TAXONOMY INTEL: the famous half files as owner-gated future-domain seed + agent-skill adoption intel, never as a corpus batch.**

Route **`rust_perf`** (io + concurrency categories). The tempting WRONG domain: `sec` — the lead rule carries a slow-drip/DoS framing, but sec's web-trust classes are tower-http/jwt/cookie shaped; the fix-space here is hand-rolled `std::net` server code, which is rust_perf's io surface. A sec cross-cite in-rule is fine; the home is rust_perf.

## The two halves

### Half 1 — the corpus value: io/process robustness plumbing (the unglamorous crates)

The design rules' fix-space is CSS/TSX/HTML — no Rust shape anywhere (the transcription law's inverse: a Rust source whose famous half does NOT transcribe). But the engine plumbing carries real decision content the corpus does not own. All quotes byte-verified at the pin.

**Lead — A− `per-read-timeout-vs-request-deadline`** (`crates/common/src/http.rs:461–531`):

> "This is `SO_RCVTIMEO`: it bounds a single blocked `read`, not the whole connection, and any byte received resets it. It is a coarse backstop only; the total lane-hold bound is [`read_request_deadline`], not this."

> "a slow-drip or silent peer keeps each `read` returning `Ok(1)` (or blocking under the window) and `read_request` never returns, pinning the connection's ticket and wedging every later mutation behind it."

The fix shape: total wall-clock deadline + 50 ms-polling watchdog thread + `shutdown(Both)` + restore prior per-read timeout. The **Windows quirk is the gold** (measured triage D3 BLOCKER):

> "Windows does not unblock a `recv` already parked in the kernel when another thread calls `shutdown` on the same socket, so there the watchdog alone cannot enforce the deadline"

— the deadline must be enforced on BOTH axes per-platform: the watchdog cannot alone unblock a parked recv on Windows, and the per-read timeout cannot alone bound a slow drip. Two-regime decision content: applies to any hand-rolled server reading a framed request under `SO_RCVTIMEO` only; anti-regime: servers behind a framework with request deadlines already (tokio/hyper own this internally).

**VERIFIED FIRING AT HOME (the coordinator's own grep, 2026-10-30):**
- `riir-instinct/src/serve_edge.rs:82,943` — `READ_TIMEOUT = 10s` per-read only. NO watchdog, NO total deadline, NO `shutdown(Both)`.
- `riir-reflex/src/serve.rs:45,1197` — identical posture.

A slow-drip peer (1 byte per 9 s) holds a connection slot indefinitely on both production serve edges. Same pre-fix shape impeccable measured and fixed. (Also in-family: `riir-reflex/src/lanes/http_mini.rs:96`.)

**B `child-piped-stderr-drain-for-process-lifetime`** (`crates/browser/src/cdp.rs:193–209`):

> "Drain stderr for the life of the process so a chatty browser never blocks on a full pipe"

A `Stdio::piped()` child whose stderr nobody reads fills its 64 KB pipe → child blocks on write → parent's launch loop hangs. Contrast pair IN THE SAME REPO (the teaching content): `live/src/svelte_bridge.rs` deliberately uses `Stdio::null()` for a quiet helper — two postures, chosen by whether stderr lines carry load-bearing data. Dedup: zero corpus hits (`piped.*drain`); the two grep hits are our own buildcheck code (not a rule) and an unrelated quote.

**B− `kill-without-wait-zombie`** (`cdp.rs:215-217,329-331,448-461`; `svelte_bridge.rs` `Bridge::drop`; `common/src/proc.rs::terminate`): impeccable pairs `kill()`+`wait()` EVERYWHERE, including the Drop impl ("Belt and braces for the error paths that skip close()"). Dedup: zero corpus-rule hits. Fires at home candidate sites: `riir-refine/src/l4_daemon.rs` (3 sites), `domains/docker/buildcheck.rs:246`, `seal-game-editor world.rs:506` — per-site adjudication is the batch's T1 (the deployer verify.rs site is verified ALREADY CORRECT: pairs kill+wait).

**B− `connect-timeout-walk-over-resolved-addrs`** (`cdp.rs:689–699`): resolve all addrs, `connect_timeout` each, keep last error — plain `connect` hangs on a black-holed host. Sibling face same file: "The handshake reads under a deadline too: a port that accepts and never answers fails instead of hanging" (cdp.rs:489–491). Dedup: zero corpus hits (`connect_timeout`).

**B `arrival-order-turnstile-lane`** (`http.rs:533–677`): FIFO ticket lane serializing thread-per-connection state mutations in body-completion order (porting a single-threaded event-loop's ordering guarantee to threads). Drop-release, mutex-poison recovery (`Err(p) => p.into_inner()`), anti-regime in-source (read-only/long routes must release the ticket up front). Distinct from `papaya-lockfree`/`bounded-channel-backpressure` (ordering, not throughput). NotBenchable.

**B− `generation-counter-stale-timer-invalidation`** (`live/src/server_state.rs:427–448`): bump gen under lock on reschedule; detached timer re-locks and acts only if gen matches; Weak-upgrade guard for the shutdown race. Classic callback-staleness defense, clean.

**B− `exclusive-mkdir-temp-resource-reservation`** (`cdp.rs:135–158`): loop on `AlreadyExists` + atomic serial + pid + stamp —

> "A timestamp is not a reservation: concurrent calls can observe the same tick"

### Riders + coverage validation

- **Rider face on `prefix-sum-2d`** (entries_01): 2D box-filter area-average via integral image (`comp/src/raster.rs:71–107`) — O(out·area)→O(out), shrinking-resize regime only (grow path is bilinear, untouched). Dedup-collided with the owned law → rider face, not a rule. Integer inputs → bit-exact G1.
- **COVERAGE VALIDATION, no corpus change**: `nan-unsafe-sort-comparator` (P8) doc owns BOTH faces ("`partial_cmp(..).unwrap_or(Equal)` (or `.unwrap()`)") — impeccable carries 6 `.unwrap()` panic sites + ~36 `.unwrap_or(Equal)` sites across `comp/`+`comp-verbs/`+`core/checks/` (coordinator recount; the engine-lane agent's "~17" conflated faces). The rule fires on real code in a 95.7k-star repo — positive-instance evidence for the rule's record, nothing to land.

### Dedup evidence (coordinator-run, both vocabularies, `src/domains/`)

| Pattern | Greps | Hits |
|---|---|---|
| slow-drip/deadline | `read_timeout\|SO_RCVTIMEO\|connect_timeout\|set_read_timeout` | 0 |
| stderr drain | `stderr` ∩ `drain\|piped` | 0 rules (2 non-rule hits) |
| kill+wait | `child.wait\|kill()` in rust_perf | 0 |
| turnstile | `turnstile\|arrival.order` | 0 |
| generation timer | "generation" | unrelated determinism naming only |
| box-filter SAT | `prefix-sum-2d` | OWNED → rider |

## Half 2 — the design-taxonomy intel (research half; NOT a corpus batch)

The 62 rules (31 slop + 31 quality; `crates/foundation/src/registry.rs`) with 4 engines (regex / static-html via html5ever+hand-ported csstree / browser CDP / visual screenshot-pixels). Rule data (registry rows) split from rule logic (`core/src/checks/*`, `core/src/browser/*`). Full 62-rule inventory with trigger/fix-space/decision-content classification lives in the inspection record (two parallel inspection lanes this session; the detect-lane report carried the complete table).

The transferable ARCHITECTURE (not rules):

1. **Paint-gate family** (`core/src/browser/painted.rs`): one measured regime layer — "does a visitor see it at rest" — above 25+ rules; four gate classes (Text/Box/Raster/Toggle) + visible-share floors + covered-for-good hit-testing, memoized once per element. Kills hidden-slide/carousel false positives BY CONSTRUCTION.
2. **Project-token context**: "deliberate brand choice beats the pattern" encoded three ways — DESIGN.md allowlists (design-system-* rules fire ONLY when the dimension is declared), hostname-suffix brand-font exemptions (`BRAND_FONT_DOMAINS`), declaration-based suppression with a named corpus decision (`drop_declared_purple_findings`, r3-23). The decision-content model: a rule + its documented escape hatch, with the anti-laundering clause where needed (`undersized-ui-text`: "being ON the DESIGN.md ramp does NOT exempt").
3. **Measured-vs-declared geometry**: rendered measurement overrides authored CSS (`cramped-padding` REN-403 flex-centred-label exemption, `line-length` REN-402 "charges only when ≥2 lines run long"), each threshold carrying a recorded failure story.
4. **Saturation/repetition regimes**: `em-dash-overuse` (≥8 AND ≥1/500 chars), `aphoristic-cadence` (≥3: "once is fine; the pattern is the tell"), `monotonous-spacing` (>60% one value, ≤3 unique) — fire on the census, not the instance.
5. **Context-classification tiers**: `FinePrint::{No,Advisory,Exempt}` severity modulation instead of more rules.

**Adoption paths (owner-gated, cheapest first):**
- (a) **Agent-skill transcription** — the workspace's web-family fronts (reflex-site, riir-dapps pages, rethink.riir.ai) are agent-built and carry real slop exposure; the deterministic-floor + semantic-review methodology and the strongest rule families transcribe into the `web-family` skill as a conformance checklist (or a small scan tool). No corpus, no domain, immediate.
- (b) **Future riir-refine domain** — a `web_design`/`frontend` domain (fourth no-`syn` scanner, after dist/shader_perf) seeded by this registry; needs a Proposal + `HealDomain` trait extension per Plan 001 + an owner call. The verdict recommends but does NOT assume the domain (the unlanded-domain law).
- (c) **Hook architecture watch** — the deterministic post-edit scan feeding findings back to the agent before "work done" validates our own docs-gate/guard pattern; a note for the zed fork's agent lanes. Watch-only.

## Method record (honest)

- Two parallel inspection lanes (detect-crate rules; engine crates mining) + coordinator verification of every load-bearing quote and dedup grep. TWO SUBAGENT-REPORT CORRECTIONS landed by verification: (1) the `.unwrap()` panic-face count is 6, not ~17 (the conflation was all-`partial_cmp`-sites); (2) `nan-unsafe-sort-comparator` already owns BOTH faces, so the impeccable sites are coverage validation, NOT a rider-face candidate — the engine lane's rider suggestion was withdrawn.
- Unread bodies (marked needs_validation for any per-rule row at mine time): `core/browser/element_checks.rs` (13.6k), `css_scan.rs`, `measures.rs`, `rules.rs`, `line_breaks.rs`, `text_collectors.rs`, `crates/html` internals, `skills/` crate (8.7k prompt assembly), `live/manual_edits` bodies.
- The engine is a byte-parity JS→Rust port (UTF-16 indexing, `fdlibm_trig` JS-Math parity, golden parity tests vs the JS engine) — provenance interest only.
- Star count from the task premise, not independently verified.

## Yield estimate

3–5 rust_perf rules after ~50% coordinator falsification (lead: `per-read-timeout-vs-request-deadline`; then stderr-drain, kill-without-wait, connect-timeout-walk; turnstile/gen-timer/mkdir as the stretch tier) + 1 rider face + 1 coverage-validation note. All own rules NotBenchable-by-domain (hang-prevention/ordering wins have no µs axis on the Pareto validator — the dial9 annotation class, first-class decision-content entries).

## Plan

Batch plan: `.plans/316_batch316_impeccable_io_robustness_rust_perf.md` (T0 re-clone at pin → T1 dedup re-verify + home-site adjudication → T2 author → T3 gates → T4 count pins → T5 binary rebuild (rust_perf IS release-set) → T6 docs → T7 landing row). The `.raw/impeccable` clone is REMOVED at this verdict's commit; re-clone at the pin at mine time.
