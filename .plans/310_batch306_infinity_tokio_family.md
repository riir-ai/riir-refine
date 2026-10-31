# Plan 310 — Batch 306: infinity tokio-family batch (rust_perf)

**Status:** STAGED — claim plan-first at the next mining slot (nominal Batch 306; Research 349 is the working inventory; source hydro-project/infinity @ `1abc6fcff1f186124f924b56aa92c3a3684c90c5`, Apache-2.0)
**Lease closed:** zed-glm-b306, epoch 1793283695, was expires 2026-10-31T14:21:35+00:00 (staleness fold — the plan stays STAGED; Research 349 is the working inventory; re-claim plan-first at the next mining slot)

- [ ] T0 — re-clone at the pin into `.raw/b306/` (shallow; removed at close); re-verify every Research-349 quote at the pin; read the note's unread files BEFORE grading anything citing them (`infinity-agent-cli/src/{terminal,acp_server,inline_viewport}.rs`, `slack_bot/slack_io.rs`, `slack-dataflow/flow.rs`, `sandbox-local/backend.rs`, provider converts, `infinity-mcp-bridge/lib.rs`)
- [ ] T1 — dedup re-verification (3+ vocabularies per candidate) against rust_perf 278 + the B304/B305 landings; adjudicate each grade: closed-channel-arm-gate vs the select-family hits; the coalesce GOAT lead vs `coalesce-nearby-reads` span-merge kinship (own-rule-vs-rider — the batch's main falsification risk, demote honestly if the kind-gate grades ambient); inline-actor-pool vs the ryhl actor family; recv-drain vs steal-batch/channel-send distinct-froms; defer-queue vs deferred-command-flush-hoist; honest drops allowed
- [ ] T2 — author 2-4 own rust_perf rules + riders via ≤3-rule slices (the subagent stream-idle law); fixtures ORIGINAL Rust reproductions, never source verbatim; NotBenchable annotations per class (spin/liveness, memory-footprint, wakeup-timing, interruption-latency); the coalesce lead carries the bench spec (alloc-count axis) unless demoted at T1; riders: cancel-sender-before-spawn (on the oneshot host), no-yield-idle-recheck + the stopped-thread admission face (on the pool rule)
- [ ] T3 — coordinator wiring: entries file (next `entries_NN.rs`), category registration, count pins via ONE shape-asserted script pass (tests.rs + router.rs + the Plan-003 category table — the B299/B259 pin-set-wider lessons), batch306 needle gates keyed to LANDED text
- [ ] T4 — gates: batch306 needle+parse at the right feature posture (the green-zero class); default lib; rust_perf_bench lib; clippy -D ×3 postures; retrieval floors RE-MEASURE (corpus delta); freeze gen2+gen3 regen + score-bench + release-set binary rebuild + install — ALL OWED (rust_perf corpus delta, P30/B293)
- [ ] T5 — Research 349 Status → RECORD at landing; AGENTS.md rust_perf count bump (278 → landing); `.docs/11_domains` delta; queue snapshot landing row in the same commit
- [ ] T6 — commit `feat: batch 306: infinity tokio-family ...` + push; `.raw/b306/` removed at close

## Candidate inventory (Research 349 — re-graded at T0/T1, honest drops allowed)

| # | Candidate | Category | Grade |
|---|---|---|---|
| 1 | closed-channel-select-arm-gate | concurrency | **B+** (suggestion-only) |
| 2 | streamed-chunk-tail-coalesce-kind-gated | allocation/io | **B — GOAT lead** (benchable alloc axis) |
| 3 | inline-actor-futures-pool-not-spawn-per-actor | concurrency | B− (suggestion-only; seeds futures-util family) |
| 4 | recv-one-then-try-recv-drain | concurrency | B− (family-triangle receiver side) |
| 5 | defer-deferrable-inputs-while-work-in-flight | concurrency | B− (suggestion-only) |
| 6a | cancel-sender-registered-before-spawn | rider (oneshot host) | C+ |
| 6b | no-yield-idle-recheck-respawn-covers-race | rider (pool rule) | C+ |
| — | stopped-thread admission face | rider (entries_44 host) | possible face, adjudicate at T1 |
| — | dedup-id-only-naturally-non-idempotent-inputs | — | drop-or-thin at T1 |
