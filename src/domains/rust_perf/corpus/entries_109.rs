// Batch 426 — the resin serving-engine conversion (entries slot 109: upstream
// Batch 427 took entries_108 [entries_gpu_173 was the kernel half] between my
// claim and this landing — renumbered past it per the first-landed-keeps-it law;
// Plan 294 / Research 328,
// ekzhang/resin @ the fresh pin 157eb81d [re-pinned at T0 2026-11-27 — the
// 10-26 pin c14ee9d0 was 31 days stale; 128 commits upstream, every rule's
// quote re-verified at the NEW pin; Apache-2.0]). SEVEN rules: the six
// NotBenchable decision-content rows + the #7 GOAT lead (benchable, the
// Pareto validator adjudicates at its own cell) + the B200 rider face folded
// count-neutral into #7's landing commit notes.
//
// Dedup re-verified at the landing HEAD (both vocabularies, the verdict's
// grep set re-run + every hit read): the select/commit snapshot contract
// (#5), the prepare-then-commit-no-alloc transaction shape (#3), the
// incremental-publish Weak-hint axis (#2), the host plan-stability +
// lease-reuse window (#1), the quarantine/poison failure taxonomy (#4),
// the atomic-span planning ladder (#6), and the merge-two-runs owner
// (#7) — all still zero-owner (the entries_103 sort-key + merge rules own
// key-materialization + k-way merge over PRE-SORTED runs; #7 owns ONE
// aged-ordered queue merged with ONE sorted returning batch — the
// queue-maintenance axis, adjudicated in-rule).

use crate::domains::rust_perf::PerfRule;

#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b426_rules() -> Vec<PerfRule> {
    vec![
        // ── #1: the host plan-stability + lease-reuse window (B+) ──
        PerfRule {
            rule_id: "overlap-continuation-lease-reuse-under-lookahead-window".to_string(),
            name: "a decode step N+1 may reuse the SAME resource lease as step N (overlap continuation) only under a worst-case lookahead window: the host conservatively reserves the longest possible next-step write span up front, the runtime substitutes exact feedback before launch, and reuse holds ONLY while the worst case provably cannot allocate, rotate, replace, or publish any leased page — any failure after enqueue quarantines the shared lease".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// The decode loop re-acquires a fresh lease for step N+1 while
// step N is still in flight — serialize on the host, or overlap
// without a worst-case write window and risk a mid-flight
// allocation invalidating the shared pages.
fn next_step(engine: &mut Engine) -> Result<Step> {
    let lease = engine.kv.acquire_lease_for_running()?; // re-acquire EVERY step
    let plan = engine.plan_step(&lease)?;
    engine.launch(&plan)
}
"#.to_string(),
            fixed_code: r#"// The overlap-continuation posture: one lease held across the
// two-step window, a conservative lookahead reservation covering
// the longest possible next-step write, exact feedback substituted
// at launch, and the reuse precondition checked per step.
fn next_step_overlapped(engine: &mut Engine) -> Result<Step> {
    if engine.can_reuse_lease_for_continuation() {
        // conjuncts: scheduler empty ∧ phase Decode ∧ rows == plan rows ∧
        // (max_output - generated) > 2*max ∧ the whole 2*max span inside
        // ONE block — any miss falls through to re-acquire
        engine.reserve_continuation_lookahead()?; // worst-case span up front
        let plan = engine.plan_step_from_feedback()?;
        return engine.launch(&plan);
    }
    let lease = engine.kv.acquire_lease_for_running()?;
    let plan = engine.plan_step(&lease)?;
    engine.launch(&plan)
}
"#.to_string(),
            context_notes: Some("NotBenchable — device-serving domain reason (the lease lives under a GPU KV pool; the validator has no such host). THE LAW, verbatim at the fresh pin (ekzhang/resin @ 157eb81d, crates/resin-core/src/engine.rs:1417-1480, Apache-2.0): the continuation reservation is armed only under `reserve_continuation` — phase Decode ∧ rows == the plan's rows ∧ per-row `(max_output − generated) > 2·maximum` — and the span arithmetic carries the design comment verbatim: 'Reserve one history page of progress plus the two-step physical write envelope. The plan's actual CSR, rather than a second runner setting, authenticates the resulting lease' — the reserve shrinks to min(page+max+writes, per-group max_write limits, per-row context remaining), and when the worst case does not fit (`can_ensure_contexts` fails) the lookahead FALLS BACK to the single-step decode_write_lookahead (:1465-1478) — never a hard failure, a graceful de-escalation. The failure taxonomy is the second half: 'A prepared cohort resolves in one place. Before runner acceptance, failure releases access and discards uncommitted cold allocations; resident/replay state stays owned by its request for the next attempt' (:1478-1482). ⛔ ADJUDICATIONS (both directions): distinct-from `split-submission-in-flight-cap-overlap` (B181 gpu_50 — budgets an in-flight CAP to CREATE overlap; THIS owns LEASE-REUSE ACROSS overlap under a worst-case-write window); distinct-from `fused-read-write-dst-input-overlap-refusal` (the kernel-shape refusal) + `rollback-snapshot-covers-all-mutable-state` (the snapshot completeness axis); the quarantine half composes with #4 (this batch's failure-taxonomy rule — the shared lease quarantines on any post-enqueue failure). REGIME: host-side serving loops where a hardware step is in flight while the host plans the next one and both touch shared pooled resources. ANTI-REGIME: steps with no shared resource lease; hosts that never overlap (serialize — the buggy arm is their correct posture). Batch 426 / Plan 294 / Research 328 + the T0 addendum.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #2: the incremental-prefix publication Weak-hint (B) ──
        PerfRule {
            rule_id: "incremental-prefix-publication-extends-from-weak-hint".to_string(),
            name: "publishing a structure's incremental growth extends from the CACHED HINT of the last published state (a Weak handle upgrade) instead of recomputing the whole ancestor chain — append-only growth preserves the hint, replacement/reset INVALIDATES the hint before mutating positions, and a hint that fails to upgrade (evicted) forces a recompute, never a resurrection".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// Every publish recomputes the full chain from the root —
// even when the structure only appended, and even when a
// cached snapshot of the last published prefix is still alive.
fn publish(&mut self, blocks: usize) -> Result<Published> {
    let mut chain = Vec::new();
    let mut cur = self.root.clone();
    for _ in 0..blocks {
        chain.push(self.materialize(&cur)?); // the WHOLE ancestor walk
        cur = self.child(&cur);
    }
    self.commit(chain)
}
"#.to_string(),
            fixed_code: r#"// The hint-extended publish: append-only growth extends from the
// upgraded hint (only the delta is materialized); replacement or
// reset clears the hint BEFORE mutating positions; an evicted
// hint (upgrade fails) recomputes — it never resurrects.
fn publish(&mut self, blocks: usize) -> Result<Published> {
    let previous = self.snapshot.upgrade().filter(|tail| tail.len() <= blocks);
    let shared = previous.as_ref().map_or(0, |tail| tail.len());
    let mut chain = previous.as_ref().map(|p| p.as_ref().clone()).unwrap_or_default();
    for page in &self.pages[shared..blocks] {
        chain.push(self.materialize(page)?); // only the delta
    }
    self.commit(chain)
}

fn reset_positions(&mut self) {
    self.snapshot = Weak::new(); // invalidate BEFORE the mutation
    self.rewind_positions();
}
"#.to_string(),
            context_notes: Some("NotBenchable — the hint lives under a paged KV pool with lifecycle handles (no validator host). THE LAW, verbatim at the fresh pin (ekzhang/resin @ 157eb81d, crates/resin-core/src/resources/paged.rs:921-927, Apache-2.0): 'Append-only growth preserves this prefix. Replacement and reset invalidate the private hint before changing existing positions, so publishing never has to compare the entire ancestor chain.' The mechanism (:927-931): `let previous = source.snapshot.upgrade().filter(|tail| tail.len() <= demand.blocks); let shared = previous.as_ref().map_or(0, |tail| tail.len());` — then only `source.pages[shared..demand.blocks]` is validated, and a full-length hint is reused as the tail (`if shared == demand.blocks { previous...}`). The invalidation discipline (:1066-1068): `live.snapshot = Weak::new(); fresh.snapshot = Weak::new();` on the state-restore path — the hint dies BEFORE the swap. ⛔ ADJUDICATIONS: genus neighbor `slot-keeping-scan-runs-in-place-publish-only-what-moved` (B281-era gpu_751 — the in-place scan diffing only what moved; THIS owns the HINT-DELIMITED CHAIN extension axis — the delta is delimited by the hint's length, not by a scan diff; compose on a slot structure that publishes); `prefix-cache-entry-one-token-early` (B200) owns the entry-BOUNDARY axis; THIS owns the publish-COST axis. REGIME: append-only structures published incrementally (prefix caches, blockchains of checkpoints, growing indexes). ANTI-REGIME: structures replaced wholesale between publishes (the hint can never survive — the reset arm IS the rule); single-shot publication. Batch 426 / Plan 294 / Research 328.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #3: prepare-fallible-metadata-then-commit-without-allocation (B−) ──
        PerfRule {
            rule_id: "prepare-fallible-metadata-then-commit-without-allocation".to_string(),
            name: "a reservation that can fail halfway performs EVERY fallible step (capacity checks, demand overflow, metadata allocation, exhaustion bounds) into prepared metadata BEFORE any pool mutation — the publish phase touches only planned entries and performs no allocation, so failures leave the pool byte-identical".to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"// The reservation mutates as it goes: grow, then allocate,
// then mutate — a failure at step 3 leaves the pool half-grown.
fn reserve(&mut self, demands: &[Demand]) -> Result<()> {
    for d in demands {
        self.grow_group(d)?;      // mutates capacity
        self.allocate_pages(d)?;  // allocates more
        self.commit_group(d)?;    // can still fail here
    }
    Ok(())
}
"#.to_string(),
            fixed_code: r#"// The prepare-then-commit split: every fallible step (try_reserve,
// overflow checks, exhaustion bounds) lands in prepared metadata;
// the publish touches only planned entries — no allocation, and a
// failure anywhere leaves the pool untouched.
fn reserve(&mut self, demands: &[Demand]) -> Result<()> {
    let mut growth = Vec::new();
    for d in demands {
        let additional = d.blocks.checked_sub(self.group_len(d.group)?)
            .ok_or(CachePoolError::StaleHandle)?;
        self.try_reserve_exact(d.group, additional)?; // fallible, no mutation
        growth.push((d.group, additional));
    }
    if growth.iter().all(|(_, n)| *n == 0) {
        return Ok(()); // the no-op early exit, pre-publication
    }
    let additions = self.allocate_pages(&growth)?; // the only allocation
    for (group, mut pages) in additions {
        self.append(group, &mut pages); // publish: no fallible step
    }
    Ok(())
}
"#.to_string(),
            context_notes: Some("NotBenchable — the transaction shape lives under a slab KV pool (no validator host). THE LAW, verbatim at the fresh pin (ekzhang/resin @ 157eb81d, crates/resin-core/src/resources/paged.rs:869-884, Apache-2.0): the growth loop `try_reserve_exact(additional)`s EVERY group's demand first (fallible, uncommitted), the all-zero early exit comes next ('if growth.iter().all(|demand| demand.blocks == 0) { return Ok(()); }'), and ONLY THEN `allocate_pages` + append — the publish performs no allocation. The exhaustion pre-checks are pinned by the source's own test names: 'generation_exhaustion_before_commit_leaves_the_pool_unchanged', 'writer_id_exhaustion_leaves_the_pool_unchanged' — writer-id/epoch/generation bounds checked pre-mutation. ⛔ ADJUDICATION: `compaction-reserve-before-prompt-build` owns ONE reserve-before clause of this shape; THIS owns the whole PLAN/COMMIT SPLIT as a transaction shape (prepare everything fallible, publish nothing fallible) — compose on a compactor. REGIME: multi-entry reservations on shared pools (allocators, caches, schedulers). ANTI-REGIME: single-entry reservations (one try + commit inline is already atomic); infallible appends. Batch 426 / Plan 294 / Research 328.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #4: pre-accept-rejection-restores-post-accept-failure-quarantines (B−) ──
        PerfRule {
            rule_id: "pre-accept-rejection-restores-post-accept-failure-quarantines".to_string(),
            name: "a batch's failure taxonomy splits at acceptance: a pre-accept rejection (nothing hardware-visible happened) RESTORES the cohort exactly — leases released, turn ordinals intact, retry clean; any post-accept failure (side effects may have landed) QUARANTINES — the lease stays unreusable, resources unreturned, and the engine poisons once so every later step refuses rather than guessing".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// One failure path for everything: release the lease and
// retry — even when the hardware may have executed the step
// (its effects are unknown, the retry double-fires).
fn on_step_failure(engine: &mut Engine, batch: Batch, err: Error) {
    engine.release_lease(&batch.lease);
    engine.requeue(batch); // the poisoned step re-fires
    if err.is_fatal() { /* nothing special */ }
}
"#.to_string(),
            fixed_code: r#"// The split taxonomy: pre-accept rejection restores; post-accept
// failure quarantines + poisons ONCE (first error wins, later
// steps refuse without touching the quarantined resources).
fn on_step_failure(engine: &mut Engine, batch: Batch, err: Error) {
    match err.classify() {
        Rejected => {
            engine.release_lease(&batch.lease); // no side effects: restore EXACTLY
            engine.requeue_preserving_order(batch);
        }
        _ => {
            engine.quarantine_lease(&batch.lease); // unreusable, unreturned
            engine.poison_once(err); // first error wins; every later step refuses
        }
    }
}
"#.to_string(),
            context_notes: Some("NotBenchable — the taxonomy lives under an execution engine (no validator host). THE LAW, verbatim at the fresh pin (ekzhang/resin @ 157eb81d, crates/resin-core/src/engine.rs, Apache-2.0): the error enum's own doc (:70) 'Execution changed state or its effects are unknown; resources are quarantined'; the poison gate (:2253-2259) `if let Some(error) = &self.poisoned_error { return Err(Error::internal(format!(\"Engine is poisoned and its resources are quarantined: {error}\"))); }` — first error wins, every later call refuses; the pre-accept restore: the prepare closure's failure 'releases access and discards uncommitted cold allocations; resident/replay state stays owned by its request for the next attempt' (:1479-1482); the quarantine half (:346) 'may have changed model state, either failure must take the quarantine path'. ⛔ ADJUDICATION: `reject-plan-runs-before-setup-with-outliving-side-effects` (B271) owns the PRE-accept ORDERING half (reject before setup so no side effects outlive); THIS owns the POST-accept QUARANTINE + poison-once half — compose into one complete taxonomy on any engine with both seams; the sonar B272 fail-stop leads remain unlanded (re-verified at landing HEAD) — if they land, THIS folds to a face on that family. REGIME: engines executing hardware/remote steps with pooled resources. ANTI-REGIME: pure-computation pipelines (a failed step has no unknown-effect class). Batch 426 / Plan 294 / Research 328.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #5: epoch-guarded-selection-identity (B−) ──
        PerfRule {
            rule_id: "epoch-guarded-selection-identity".to_string(),
            name: "a select-then-act pair whose queue can mutate between the two steps returns an identity token with the selection (epoch + the exact ordered cohort); the act asserts the token matches the CURRENT queue state — same epoch AND the cohort still a prefix of the queue order — and fails LOUD on any intervening mutation instead of acting on wrong rows".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// Select returns bare rows; act trusts them blind — an
// intervening requeue/reorder silently shifts the rows'
// meaning and the act hits the wrong cohort.
let selected = queue.select(budget);
// ... planning, possible requeues/reorders in between ...
queue.take_for_plan(&selected); // trusts the stale rows
"#.to_string(),
            fixed_code: r#"// The guarded pair: select returns (rows, Selection{epoch, identities});
// take asserts the epoch unchanged AND the selected identities still
// a PREFIX of the current order — any mutation in between fails loud.
let (rows, selection) = queue.select(budget);
// ... planning, possible requeues/reorders in between ...
queue.take_for_plan(&selection)?; // Err on any intervening mutation
"#.to_string(),
            context_notes: Some("NotBenchable — correctness guard, zero-cost happy path (the check is an epoch compare + prefix walk). THE LAW, verbatim at the fresh pin (ekzhang/resin @ 157eb81d, crates/resin-core/src/scheduler.rs:32-35, Apache-2.0): 'Planning inspects selected requests before the engine pins and removes them. The epoch and identities make the later removal fail loudly if an accidental queue mutation or reordering is inserted between those steps.' The prefix discipline (the Selection impl doc): 'Retain the oldest prefix of a previously selected homogeneous cohort... it must never skip an older matching row to backfill a later one' — the truncate path preserves prefix semantics. ⛔ ADJUDICATIONS: distinct-from `graph-pool-handle-fresh-per-generation` (per-HANDLE generation invalidation vs a queue-ORDER snapshot contract); distinct-from `revalidation-skipped-by-certifying-identity-token` (B283 — skip-when-same vs fail-when-changed, opposite directions); kin `optimistic-concurrency-version-check` shape at the queue seam (the epoch IS the optimistic version). REGIME: plan-then-act over mutable ordered collections (schedulers, job queues, batch planners). ANTI-REGIME: single-threaded flows with no intervening mutation point; unordered sets (no prefix semantics to preserve). Batch 426 / Plan 294 / Research 328.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #6: atomic-span-aware-chunk-placement (B−) ──
        PerfRule {
            rule_id: "atomic-span-aware-chunk-placement".to_string(),
            name: "a token/work-budget chunker must not split caller-declared ATOMIC spans across chunks — the bounded three-way resolution: extend the chunk end to the span end when it fits the hard budget, truncate to BEFORE the span when it doesn't, and refuse outright only when the span alone exceeds the budget on an empty plan".to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"// The chunker slices at the budget boundary blindly — an
// atomic span (a prompt+completion pair, a request header
// chain) gets split mid-span and both halves are wrong.
fn chunk(items: &[Item], budget: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    for group in items.chunks(budget) {
        chunks.push(Chunk::new(group)); // splits spans mid-way
    }
    chunks
}
"#.to_string(),
            fixed_code: r#"// The span-aware chunker: three-way resolution per atomic span —
// extend to the span end (fits), truncate before the span
// (doesn't fit but the chunk is non-empty), refuse (the span
// alone exceeds the budget on an empty plan).
fn chunk(items: &[Item], budget: usize) -> Result<Vec<Chunk>, &'static str> {
    let mut chunks = Vec::new();
    let mut end = 0usize;
    while end < items.len() {
        let mut stop = (end + budget).min(items.len());
        for &(span_start, span_end) in &ATOMIC_SPANS {
            if span_start < stop && stop < span_end {
                if span_end <= end + budget { stop = span_end; }        // extend
                else if span_start > end { stop = span_start; }         // truncate
                else if chunks.is_empty() { return Err("span exceeds budget"); } // refuse
            }
        }
        chunks.push(Chunk::new(&items[end..stop]));
        end = stop;
    }
    Ok(chunks)
}
"#.to_string(),
            context_notes: Some("NotBenchable — thin but bounded: the three-way ladder is fully testable. THE LAW, verbatim at the fresh pin (ekzhang/resin @ 157eb81d, crates/resin-core/src/planning.rs:109-119, Apache-2.0): `for &(span_start, span_end) in &request.atomic_prefill_spans { if span_start < end && end < span_end { if span_end <= hard_end { end = span_end; } else if span_start > request.logical_context { end = span_start; } else if steps.is_empty() { ... } } }` + the invariant pin (:125) '.expect(\"atomic spans preserve positive request work\")' — the refuse arm fires only on an EMPTY plan so every produced chunk carries positive work. ⛔ ADJUDICATION: no corpus owner of the span-aware planning ladder (re-verified at landing HEAD); the checkpoint-alignment half of the same chunker (round chunk ends DOWN to checkpoints strictly before the input boundary — 'A cached request must replay at least one token to recover its logits') is the B200 rider face folded into this batch's landing notes, count-neutral. REGIME: budget chunkers over work with caller-declared atomic units (token planners, batch builders, rate-limited senders). ANTI-REGIME: chunkers over uniform ungrouped items (no spans declared). Batch 426 / Plan 294 / Research 328.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #7: the GOAT lead — merge-returning-batch-sorted-into-aged-queue ──
        PerfRule {
            rule_id: "merge-returning-batch-sorted-into-aged-queue".to_string(),
            name: "a queue maintained in age/key order absorbs a returned batch by SORTING THE BATCH alone and merging the two sorted runs in one preallocated pass — O(k log k + n) instead of re-sorting the union O((n+k) log(n+k)); the law: only the returning batch may arrive out of order (the queue never does), so sort the small side and merge".to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"// The returning batch is appended and the WHOLE queue re-sorted —
// O((n+k) log(n+k)) every cycle, allocating fresh storage for a
// queue whose base order never broke.
fn requeue(queue: &mut Vec<Job>, returning: Vec<Job>) {
    queue.extend(returning);
    queue.sort_by_key(|j| j.turn); // re-sorts the n already-ordered rows
}
"#.to_string(),
            fixed_code: r#"// The merge posture: sort the k returning rows (they alone may be
// out of order), then merge two sorted runs in one preallocated
// pass — O(k log k + n), zero per-element allocation.
fn requeue(queue: &mut Vec<Job>, mut returning: Vec<Job>) {
    returning.sort_by_key(|j| j.turn); // only k sorts
    let mut merged = Vec::with_capacity(queue.len() + returning.len());
    let mut ready = std::mem::take(queue).into_iter().peekable();
    let mut returning = returning.into_iter().peekable();
    while let (Some(queued), Some(returned)) = (ready.peek(), returning.peek()) {
        if queued.turn <= returned.turn {
            merged.push(ready.next().expect("queued item was peeked"));
        } else {
            merged.push(returning.next().expect("returning item was peeked"));
        }
    }
    merged.extend(ready);
    merged.extend(returning);
    *queue = merged;
}
"#.to_string(),
            context_notes: Some("NotBenchable-by-domain — the aged queue is a serving scheduler shape (the validator's fixture harness cannot model the k<<n serving regime where the law pays; the decision content is the ORDER-MAINTENANCE contract). THE LAW, verbatim at the fresh pin (ekzhang/resin @ 157eb81d, crates/resin-core/src/scheduler.rs:328-348, Apache-2.0): 'Queue mutations preserve age order. Only the returning batch may arrive out of order, so sort it and merge instead of sorting every waiting row.' The mechanism: `returning.sort_by_key(Schedulable::turn_ordinal)` + `Vec::with_capacity(self.ready.len() + returning.len())` + the two-peek merge with `queued.turn_ordinal() <= returned.turn_ordinal()` — STABLE left-preference on ties (the queued row wins the tie, preserving age precedence). Magnitude: k<<n serving regime — k=32, n=10,000 → ~10.2k comparisons vs ~133k for the full re-sort (~13×). ⛔ ADJUDICATIONS: `sort-key-lazy-when-comparisons-per-item-low` (B416 entries_103) owns the KEY-MATERIALIZATION axis in a k-way merge (never build a collation key per item when comparisons are few) — compose: #7's sort keys on the already-stored ordinal, no key built; the B416 `merge_two_streams` fixture merges PRE-SORTED runs k-way — #7 owns ONE aged-ordered queue + ONE returning batch (the queue-MAINTENANCE contract: the base order is an invariant, not a convenience); `bsearch-sorted-slice`/`sorted-input-previous-row-fast-path`/`sortedness-check-branchless-accumulate` own sorted-READ mechanisms, not queue absorption. REGIME: queues maintained in order where re-insertions arrive as batches (serving survivors/rejects, retry queues, scheduler returns). ANTI-REGIME: unsorted base queues (nothing preserved to merge against); per-item requeue (sort-per-item is worse than either). Batch 426 / Plan 294 / Research 328 — the GOAT lead, the Pareto validator adjudicates at its own cell; an honest drop if it Pareto-dominates (the B243 precedent).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
