# Research 349 — infinity agent-runtime distill (hydro-project/infinity @ `1abc6fcff1f186124f924b56aa92c3a3684c90c5`)

**Status:** DISTILLED — pending owner decision (verdict 2026-10-29 ~13:5xZ; plan 310 = nominal Batch 306 claims plan-first at the next mining slot)

## The source

hydro-project/infinity — "An agent runtime and tool protocol for principled agent
concurrency" (<https://github.com/hydro-project/infinity>, site infinity.hydro.run).
The framing that matters: *"Infinity does for agents what async did for threads"* —
agents run slow tools concurrently, yield while they wait, and **hibernate as pure
data between turns** ("no task, no stack, no open connection... wakes on the next
message"); the repo's own density claim is 75k agents on a Raspberry Pi. Reactive
Agent Protocol (RAP) = serve tools over the network without holding a connection
open (results delivered whenever ready, even to agents with no live process), MCP
compatibility layer, serverless scale-to-zero (AWS Lambda CDK constructs).

- Provenance chain: hydro-project is the Hydro/DFIR dataflow group; this runtime is
  their agentic-concurrency product line. Pattern provenance is the tokio/futures
  ecosystem itself — the corpus already carries the ryhl actor recipe (Batch 152/154)
  and the dial9 tokio family; the MINE here is the **deliberate deviations from that
  recipe** plus the runtime-specific laws around them. No upstream port lineage.
- License Apache-2.0 (LICENSE.txt verified at the pin). Pin **`1abc6fcff1f186124f924b56aa92c3a3684c90c5`**
  (HEAD 2026-10-27 13:54:48 -0700, "fix: enable aws-config sso feature"; 513 commits).
  Shallow clone `.raw/infinity`, read + removed at the verdict commit.
- Shape: 21-crate Cargo workspace, ~37k LOC Rust. Serious hygiene posture: workspace
  `[lints]` (clippy allow_attributes_without_reason etc.), `clippy.toml`, conventional
  commits, snapshot-tested event processor (`.snap` fixtures). Well-documented module
  docs — most headline quotes below are verbatim module docs.
- Files read this verdict: `infinity-agent-core/src/{lib,event_processor,stores}.rs`,
  `system/{mod,defer,thread}.rs` (head), `system/local/{router,driver}.rs` (full main
  loop), `tools/{sleep,mod}.rs` (heads), `rap-protocol/src/lib.rs` (head),
  `infinity-daemon/src/{rap_callback,memory_store}.rs` (heads), `sandbox-core/src/server.rs`
  (structure + streaming handler), `rap-client/src/callback_server.rs` (full),
  workspace manifests + root AGENTS.md. **Unread (re-read at mine time per the law):
  `infinity-agent-cli/src/{terminal,acp_server,inline_viewport}.rs`, `slack_bot/slack_io.rs`,
  `slack-dataflow/flow.rs`, `sandbox-local/backend.rs`, the provider convert layers,
  `infinity-mcp-bridge/lib.rs`, the Lambda runtime internals.**

## Ranked candidates

| # | Pattern | Source site | Grade | Dedup / notes |
|---|---|---|---|---|
| 1 | `closed-channel-select-arm-gate` — a `tokio::select!` arm polling a channel that CLOSES mid-loop is ALWAYS READY from then on; with `biased` the loop spins hot on the `None` arm. Gate the arm with a state bool set on the first `None` (or swap to `Option<Receiver>`), keeping the closed-as-signal semantics explicit | `system/local/driver.rs` ~L143: *"Set when subscribe_rx is closed, so selects stop polling it (a closed channel is always ready and would otherwise spin)"* — the gate appears in BOTH select loops (in-flight + park) | **B+** (own rule, suggestion-only, NotBenchable — spin/liveness CPU class, the adaptive-yield/blocking-sleep annotation family) | `always ready` → 0 corpus hits; `tokio::select` hits are the blocking-sleep fixture + the oneshot-cancel comment + the crossbeam select park rule (entries_02:499, different mechanism). The `tokio::select!` ARM-LIFECYCLE surface is uncovered. NOVEL |
| 2 | `streamed-chunk-tail-coalesce-kind-gated` — streamed chunks of the SAME kind extend the buffer TAIL (`last_mut` + `push_str`) instead of pushing a new entry; any other kind pushes. Two-regime by kind; per-chunk upstream events stay per-chunk (the coalescing is the history side only) | `event_processor.rs` `try_merge_buffer_text` ~L608: *"If the last buffered turn entry is an assistant text message, append `text` to it and return `true`... This coalesces consecutive text chunks within a turn into one message"* + the pinned negative `text_chunks_not_coalesced_across_non_text_item` | **B — GOAT lead** (BENCHABLE: alloc-count axis exact; String concat = bit-identical final text) | Kinship recorded BOTH directions: `coalesce-nearby-reads-bounded-gap-cacheability-gated` (entries_05) and entries_31's span normalize share the `last_mut`-extend SHAPE but live in span/range algebra over syscalls — this is message-kind discrimination + String growth, no span algebra, different anti-regimes. THE BATCH'S MAIN FALSIFICATION RISK: thin mechanism (push_str into tail) — if T1 grades the kind-gate ambient, demote to a rider on the coalesce family, never force it |
| 3 | `inline-actor-futures-pool-not-spawn-per-actor` — for an idle-heavy fleet, own the per-actor futures in ONE dispatcher-pooled `FuturesUnordered` (LocalSet/`!Send` posture) instead of `tokio::spawn` per actor: the actor's memory is released the moment it goes idle, and respawn-on-input replaces keep-alive tasks | `system/local/router.rs` module doc: *"The router owns the driver futures directly (one FuturesUnordered pool rather than one spawned task per thread), so a driver's memory is fully released the moment it goes idle"* + the driver lifecycle (Idle → exit; router respawns; `check_before_idle`) + the hibernation economics (`sleep_until_event_or_input` tool, pure-data turn state) | **B−** (own rule, suggestion-only, NotBenchable — per-idle-actor MEMORY-FOOTPRINT axis absent from the Pareto validator) | `FuturesUnordered` → **0** corpus hits; `spawn_local` → 0. The corpus's actor family IS the ryhl spawn-per-actor recipe (entries_06 cycle-deadlock, sender-per-work-item, oneshot hygiene) — this is the documented DEVIATION axis for fleet-density serving, the same genus as batch-blocking-pool-segment's "global resource" law but on the task/lifetime axis. Carries riders 6b below |
| 4 | `recv-one-then-try-recv-drain` — after a blocking `recv()` returns one item, drain the rest eagerly with `try_recv()` into the same batch: the unit of work per wake is everything that queued while you were busy, not one message | `system/local/driver.rs` — the drain loop `while let Ok(item) = rx.try_recv() { batch.push(item); }` runs in BOTH arms (in-flight interrupt arm + idle arm) and again after the park loop | **B−** (own rule or face on #3; suggestion-only, NotBenchable — channel-wakeup timing class) | `try_recv` → 0 corpus hits. Family-triangle completion: `steal-batch-not-per-item` (entries_06) owns the STEALER side of a crossbeam deque; `channel-send-batching-unpark` (entries_02) owns the SENDER side; the RECEIVER-side drain-per-wake has no owner. Distinct-from lines required against both |
| 5 | `defer-deferrable-inputs-while-work-in-flight` — classify inputs: PRIORITY input (user text) interrupts the in-flight step deliberately; SYNTHETIC events (subscription events, child-thread reports, parent messages) DEFER behind a platform-portable queue trait (`InMemory` / `NoDeferral` / transport-level re-enqueue), draining when the pending call resolves — a synthetic interrupt would inject a fake result and the real one drops stale | `system/defer.rs` module doc: *"processing an unrelated synthetic event... would *interrupt* the pending call: the runtime injects a synthetic 'interrupted' result so the history stays well-formed, and the real result is later dropped as stale"* + `filter_deferrable` + the driver's defer-ready check | **B−** (own rule, suggestion-only, NotBenchable — work-interruption/latency class) | Distinct from `deferred-command-flush-hoist-out-of-batch-loop` (entries_08 — flush PLACEMENT inside a batch loop, a sync-point-amortization law; this is INTERRUPTION AVOIDANCE keyed on input class, no flush involved). `defer` otherwise hits trait plumbing only. NOVEL |
| 6a | `cancel-sender-registered-before-spawn` — register the cancellation oneshot's SENDER before spawning the cancellable task, so no cancel can arrive in the spawn window and be lost | `sandbox-core/src/server.rs` ~L825: *"Because the oneshot sender is registered in `invoke_handler` *before* the task is spawned, there is no window where a cancel can be lost"* | **C+ rider** on the oneshot family (entries_06 `oneshot-response-send-unwrap-panics` is send-error hygiene — this is registration ORDER; adjacent, not owned) | `lost.cancel` → 0 hits. Rider-grade |
| 6b | `no-yield-idle-recheck-respawn-covers-race` — decide idle WITHOUT yielding to the dispatcher (a synchronous final drain/recheck), so the exit race has exactly two outcomes: input appears in rx (process it) or arrives after the receivers drop (the router respawns a fresh driver). The respawn path is the race's designated owner, never lost-wakeup | `system/local/driver.rs` `check_before_idle`: *"an input either appears in `rx` here or is sent after the receivers are dropped, causing the router to spawn a new driver"* | **C+ rider** on rule #3 (the lifecycle half of the pool law) | Adjacent to the idle/respawn family; no owner. Rider-grade |

## GOAT candidate test (rule #2, the only benchable lead)

1. **Benchable** — YES: rust_perf_bench Pareto validator + the exact alloc-count axis (N chunk `String`s → amortized `push_str` growth); time arm feasible.
2. **Exact semantics** — YES: `String` concatenation, bit-identical final text; the kind-gate keeps interleaved kinds as separate entries (their own test pins the negative).
3. **Magnitude story** — mechanism-level: a 500-chunk LLM turn pays ~500 `String` header+capacity allocations (+ per-entry Vec slot + serde round-trip at persist) vs ~O(log) growth reallocations. Allocation COUNT collapse is the exact axis; per-op timing is modest — honest, no inflated claim.
4. **Deterministic** — YES.

All four pass ⇒ GOAT lead. Falsification risk: T1 may grade the kind-gate AMBIENT (the extend-tail shape is span-merge kin) ⇒ demote to rider, never force.

## Drops + kills (dedup evidence)

- **Router admission-before-spawn** (`thread_exists` + `is_thread_stopped` checks refuse events for unknown/user-stopped threads before any driver spawns, `system/local/router.rs` ~L317-352, with the fail-open-on-store-error disclosure "processing event anyway") = **POSITIVE INSTANCE of `reject-plan-runs-before-setup-with-outliving-side-effects`** (entries_44, B271). Kill recorded as evidence the rule fires at home; possible rider face at mine time (the STOPPED-thread arm + the fail-open disclosure is a fine second face).
- `dedup-id-only-naturally-non-idempotent-inputs` (sync(): *"Only inputs that are not naturally idempotent need durable dedup IDs: user text and subscription events... Tool results are deduplicated against the history tail itself"*) — nice discrimination law, C+ thin, storage-shaped; **drop-or-thin** at T1.
- `strkind!` newtype-ID macro — macro plumbing, ambient structure, KILL.
- Hibernation / `sleep_until_event_or_input` / Tokio-vs-durable-timer posture / MCP bridge — architecture and product shape, not corpus rules; feeds rule #3's regime story.
- Lazy per-thread JSON persistence with a load-once set (`memory_store.rs`) — memoization-family adjacency, thin; drop unless a T1 read finds a distinct mechanism.
- **WATCH row (not a rule): the RAP callback endpoint is an UNAUTHENTICATED localhost HTTP listener** (`rap-client/callback_server.rs` binds `127.0.0.1:0`, accepts any POST carrying a parseable `RapCallback`; dedup by minted ID only). The trust argument is local-process-equals-same-user, which is defensible for a dev-loop tool — but any local process can forge tool results into a running session. Architectural posture, not a code-shaped fix-space; recorded so the sec family's webhook/egress classes are not expected to cover it. Revisit only if RAP grows a cross-host deployment story.
- Sandbox "stacked sandboxes" (git-worktree-per-thread isolation) — product architecture, no rule.

## Lane intel

**League mapping: NONE — this is a zero-GPU source; no league cell moves, nothing transferable to the scored bout lanes. Stated, not skipped.**

Workspace agent-lane mapping (design intel, not config deltas):

1. **seal-remake bot-host / presence lanes** — the bot host runs 1000-bot batches as AVTD streams in ONE process with impostor LOD past 48 full bodies; Infinity's memory-per-idle-actor economics (rule #3: pool-owned futures, hibernate-as-data, respawn-on-event) is directly citable design intel if/when bot residency economics get re-priced (the `SEAL_PRESENCE_FULL_BODIES`/impostor posture). Follow-up: cite Research 349 in the bot-host design note at its next touch — no issue filed from a verdict.
2. **riir-agents** (Plan 523 shared-context / DeLM, decision_gates) — the event-priority taxonomy (rule #5: user input interrupts, synthetic events defer) + hibernation-between-turns maps onto shared-context session design. Follow-up: available as design reference at the next riir-agents session pass.

What is NOT transferable: every density/memory number (75k agents, Pi — vendor claim, no harness); the Lambda/SQS/EventBridge serverless posture (their platform); RAP wire details (their protocol; the open spec is theirs to evolve).

## Product coverage

- **tokio family EXISTS in the corpus** (~16 rules across entries_02/06: async-yield-point, tokio-guard-across-await, bounded-channel-backpressure, channel-send-batching-unpark, batch-blocking-pool-segment, adaptive-yield-consecutive-ready, bounded-fanout-semaphore, blocking-sleep-in-async, rayon-bridge-oneshot, tcp-nodelay, write-vectored, oneshot-response-send-unwrap-panics, actor-bounded-channel-cycle-deadlock, steal-batch-not-per-item, sender-per-work-item-terminates-channel, nested-par-inside-channel-bridge-deadlocks). This source EXTENDS it (#1, #4, #5) and contributes the deviation axis (#3).
- **futures-util / FuturesUnordered = ZERO corpus coverage** — a genuine top-tier ecosystem gap (hyper/tonic/tower/axum internals run on FuturesUnordered/stream combinators). Rule #3 seeds the family.
- **Follow-up source pool** (accumulation clause: pool sources read before the batch window fold into the same batch): ryhl.io blogs (already mined, B152/154 — re-read for FuturesUnordered-adjacent content only), tokio docs `mpsc`/`select!` semantics pages, futures-rs `futures-util` docs (FuturesUnordered, StreamExt), tokio-stream crate docs, tokio-rs/mini-redis (the canonical teaching implementation).

## Caveats

- Young, fast-moving repo (513 commits, HEAD two days old at inspection) — the pin is load-bearing; re-clone at mine time, quotes re-verified at the pin.
- Suggestion-heavy mix — this is the dial9 shape: one benchable lead + suggestion-grade family extensions. Expected batch class: 2-4 own rust_perf rules, the rest riders/annotations. NOT a kernel source; no kernel_opt route (routing table honored — nothing here is GPU).
- Unread files listed above; anything citing them re-reads first at T0.
- The commit state this verdict landed under: the M3 checkout was HELD by sibling WIP (`src/bin/heal.rs` overlap blocked pull, behind 26); artifacts staged alone via a detached worktree on origin/develop (the 10-28e temp-index precedent); highwaters read AT ORIGIN (research 348→349, plans 309→310 — origin's 348/309 were consumed by Batch 304's note + claim).

## Recording

Queue slot: nominal **Batch 306** (B304/B305 landed 10-29 on origin), plan **310**, this note **349**, one commit, `docs:` prefix, staged via detached worktree. `.raw/infinity` removed at the verdict commit.
