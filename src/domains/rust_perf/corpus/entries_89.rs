// Batch 376 — the anemll-forge branch-delta rust_perf half (the storage/
// serving caching pair, Plan 387 / Research 421 — the branch-delta re-grade
// of the 09-30 verdict; Anemll/anemll-forge @ fe8de185d7ff45e5663a21f2ecfd3d9e1f2ffe40 branch
// v8-gpu-ane-prefill, MIT — RE-PINNED at the batch T0 past the verdict pin
// 912925f7 (the tip moved 2026-11-19; every quote below byte-verified at the
// NEW pin; the two-pin diff 117ded62..fe8de185 proves the v8_prefill docs
// NEW since the 09-30 record). Original batch numbering: nominal Batch 376.
//
// rust_perf 378 → 380: R1 `snapshot-state-kv-prefix-consistency-drop-invalid`
// (caching, the batch's consistency law — a stored snapshot whose tokens are
// no longer a prefix of the live KV state is INVALID and must be dropped,
// never restored) + R2 `hybrid-state-prefix-snapshot-reuse` (caching, the
// staged splash candidate landed at its second instance — the anemll 'sysk'
// copy-aside: keep the prefix snapshot's rows before a non-prefix writer
// rewrites them, restore in one grow). The A5 + F3 count-neutral FACES land
// on entries_74's ane/host pair (the per-runtime residency split + the
// DRAM-measured interference re-adjudication) in the same commit.
//
// Fixtures `corpus/fixtures/b376_fixtures.rs` F1–F3 are ORIGINAL plain-Rust
// reproductions (zero repo source pasted), standalone-compiled AND executed
// BEFORE entry. NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: a snapshot that is not a prefix of the live state is dropped ──
#[allow(clippy::useless_format)] // the bare-string .to_string() form hits an unresolved parse interaction (the B376 T2 en-route repair)
pub fn r1_snapshot_state_kv_prefix_consistency_drop_invalid() -> PerfRule {
    PerfRule {
        rule_id: "snapshot-state-kv-prefix-consistency-drop-invalid".to_string(),
        name: "a snapshot of joint (state, KV-rows) memory held beside a live cache must be VALIDATED at restore time against what the cache now holds — a snapshot whose tokens are NOT a prefix of the live rows is invalid (a second writer — another conversation, a summary prompt, a continuation — rewrote the rows under it) and must be DROPPED so the request runs cold, never restored: restoring it silently pairs one component's state (the recurrent-state rows the snapshot copied) with the OTHER writer's rows (the cache memory they pointed into), and the composite is wrong with no error — the typed-shape answer is a `restorable(snap)` predicate at restore time plus a typed refusal naming the snapshot, never a silent composite".to_string(),
        category: "caching".to_string(),
        buggy_code: r#"// NAIVE: snapshots are (state, row-range) pairs; restore trusts the
// range. A second prompt rewrote rows 0..34K while this snapshot sat
// in the cache — restore hands back the OLD state over the NEW rows:
// the DeltaNet state of one system prompt over another prompt's KV
// rows. Silent: the decode runs, the answers drift.
struct Snapshot {
    state: [f32; 64],     // the recurrent layer's own state copy
    rows: Range<usize>,   // the KV rows it was valid over
}

struct Cache {
    kv: Vec<Row>,
    snaps: Vec<Snapshot>,
}

fn restore(cache: &mut Cache, snap: &Snapshot) {
    // no validity check: the row range may name rewritten memory
    cache.state = snap.state;
    cache.context_len = snap.rows.end;
}"#.to_string(),
        fixed_code: r#"// FIXED: restore VALIDATES first — the snapshot's token identity
// must still be a prefix of what the cache holds; a mismatch drops
// the snapshot (the request re-runs cold) and says so. The reuse
// half (keep the prefix across the rewrite) is the companion rule.
struct Snapshot {
    state: [f32; 64],
    rows: Range<usize>,
    token_digest: u64, // BLAKE3-8 of the snapshot's prefix tokens
}

struct Cache {
    kv: Vec<Row>,
    snaps: Vec<Snapshot>,
    state: [f32; 64],
    context_len: usize,
}

enum Restore {
    Restored,
    Dropped { why: &'static str },
}

fn restore(cache: &mut Cache, snap: &Snapshot, live_prefix: u64) -> Restore {
    if snap.token_digest != live_prefix {
        // not a prefix of what the cache now holds: another writer
        // rewrote the rows — the composite would be silently wrong
        return Restore::Dropped { why: "snapshot not a prefix of the live cache; runs cold" };
    }
    cache.state = snap.state;
    cache.context_len = snap.rows.end;
    Restore::Restored
}"#.to_string(),
        context_notes: Some(format!("{}", "Grade B− — decision-content, honestly NotBenchable (the harm is a silently wrong composite state — a correctness axis a wall/alloc validator cannot host; the metric-mismatch class, `bench_spec: None`). SOURCE (Anemll/anemll-forge @ fe8de185d7ff45e5663a21f2ecfd3d9e1f2ffe40 branch v8-gpu-ane-prefill, MIT — re-pinned at the batch T0 past the verdict pin 912925f; T0 quote-verified at the new pin). THE LAW, V8_SERVER.md:183-187 verbatim: 'A correctness bug found on the way: a summary request (another system prompt, cold) rewrites KV rows 0 to 34K, but the main conversation's 'system' snapshot stayed in the cache (a V8 cold run does not take an in-run snapshot for a cut below 128, so the summary's own 'system' did not replace it). The next main request restored it: the DeltaNet state of the main system prompt over the summary prompt's KV rows (13:14 and 13:36 in that session). Snapshots whose tokens are not a prefix of the KV cache's are now dropped, so that request runs cold.' — the fix is DROP-AND-RUN-COLD, never restore-anyway; the runtime half names the API shape (qwen38_coreai_model.py): 'restore(snap) shrinks to the smallest context holding the snapshot and raises if its KV rows were dropped (restorable(snap) tells)' — the restorable() PREDICATE + the typed raise, exactly the fixture's shape. THE DROP POLICY (V8_SERVER.md:68-70): the sweep runs 'After every prefill and every decode' — validation is continuous, not restore-point-only, and the dropped reason names the writer class ('their KV rows were rewritten by another conversation, a summary request or another continuation'). ⛔ ADJUDICATION vs the family, both directions: `wal-watermark-pinned-before-snapshot` (entries_81) owns CAPTURE ORDER (pin the durable id BEFORE pinning the snapshot — a base at 0 forces whole-history tail walks); THIS owns RESTORE-TIME VALIDITY (the captured snapshot going stale under a second writer — the capture order was fine and the composite is still wrong); compose (a correct capture order does not save a snapshot a later writer invalidates — the restore check is the second half of the same discipline). `snapshot-then-release-shared-drafter-context` (entries_14) owns the RELEASE half at the hand-off seam (the owner's own lists die at the snapshot); THIS owns the VALIDITY half (what the snapshot may be restored OVER); compose. sec's `stale-proposal-merges-against-shown-base` owns the merge-BASE freshness of a review proposal (which base a diff applies to) — THIS owns a cached composite over a live store (the rows are shared memory, not a diff base); `resume-revalidates-provenance-before-reuse` (entries_05) owns build-artifact resume identity — the provenance axis vs THIS's cache-composite axis. REGIME: any snapshot/restart/resume mechanism over a live store a second writer can rewrite (KV caches with multi-conversation servers, session state beside a shared cache, speculative-drafter contexts, editor undo state beside a live document). ANTI-REGIMES: single-writer stores (nothing can invalidate — the check is dead weight); snapshots that COPY the rows (value snapshots cannot go stale-by-reference — the law governs range/digest snapshots over shared memory); stores where a stale composite fails loudly downstream (still drop — the typed drop beats the downstream explosion). Fixture: b376_fixtures.rs F1 (the second-writer rewrite: restore refuses + runs cold; the honest-twin control passes; the typed reason names the writer class). Plan 387 / Research 421 (the branch-delta re-grade)")),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R2: prefix snapshots of hybrid state are kept across the rewrite ──
#[allow(clippy::useless_format)] // the bare-string .to_string() form hits an unresolved parse interaction (the B376 T2 en-route repair)
pub fn r2_hybrid_state_prefix_snapshot_reuse() -> PerfRule {
    PerfRule {
        rule_id: "hybrid-state-prefix-snapshot-reuse".to_string(),
        name: "a hybrid serving stack's recurrent-state snapshots taken at prefix boundaries are REUSABLE across requests that share the prefix — when a non-prefix writer is about to rewrite the rows a snapshot stands on, copy the snapshot's rows aside (a bounded, priced copy) so the shared-prefix request restores in one grow instead of replaying the whole prefix cold; the copy is freed on restore. The reuse is admitted only under the consistency rule (a non-prefix snapshot is dropped, never restored) — reuse and validity are one discipline in two halves".to_string(),
        category: "caching".to_string(),
        buggy_code: r#"// NAIVE: a second prompt (a summary) must rewrite rows 0..34K, so the
// 'system' snapshot standing on those rows is simply lost — the next
// request sharing that 8.4K-token system prompt replays the whole
// prefix cold (about 13 s of prefill for what was a 0.2 s restore).
fn rewrite_prefix(cache: &mut Cache, new_prompt: &Prompt) {
    cache.kv[..34_000].clear(); // the snapshot's rows die here
    fill(cache, new_prompt);    // the snapshot is gone with them
}

fn next_main_request(cache: &mut Cache) -> Answer {
    // cold replay of the 8.4K-token system prompt: about 13 s
    prefill(cache, &SYSTEM_PROMPT)
}

fn prefill(_c: &mut Cache, _p: &Prompt) -> Answer {
    Answer
}

fn fill(_c: &mut Cache, _p: &Prompt) {}

struct Cache {
    kv: Vec<Row>,
}

struct Row;
struct Prompt;
struct Answer;

const SYSTEM_PROMPT: Prompt = Prompt;"#.to_string(),
        fixed_code: r#"// FIXED: before the non-prefix writer rewrites, the snapshot is kept
// WITH a copy of its rows (33 KB/token — 266 MB for the 8.4K system
// prompt, the copy 13-28 ms); the next shared-prefix request grows the
// context entry to hold the rows, writes them back (5-17 ms), and the
// copy is freed. The reuse is admitted under the consistency rule.
struct KeptSnapshot {
    snap: Snapshot,
    rows_copy: Vec<Row>, // the priced aside-copy
}

fn rewrite_prefix(cache: &mut Cache, new_prompt: &Prompt, kept: &mut Option<KeptSnapshot>) {
    // a non-prefix writer is about to rewrite the rows the 'system'
    // snapshot stands on: keep it with a copy of its rows FIRST
    if let Some(sys) = cache.take_system_snapshot() {
        *kept = Some(KeptSnapshot {
            snap: sys.clone(),
            rows_copy: cache.kv[sys.rows.clone()].to_vec(), // 13-28 ms
        });
    }
    cache.kv[..34_000].clear();
    fill(cache, new_prompt);
}

fn next_main_request(cache: &mut Cache, kept: &mut Option<KeptSnapshot>) -> Answer {
    if let Some(k) = kept.take() {
        // the restore grows the context entry first, writes back, frees
        let restore = restore_checked(cache, &k.snap, &k.rows_copy);
        if restore.is_ok() {
            return answer_after_restore(cache); // 0.2 s, not ~13 s
        }
    }
    prefill(cache, &SYSTEM_PROMPT) // the consistency rule's cold path
}

fn restore_checked(_c: &mut Cache, _s: &Snapshot, _rows: &[Row]) -> Result<(), ()> {
    Ok(())
}

fn answer_after_restore(_c: &mut Cache) -> Answer {
    Answer
}

fn take_system_snapshot(_c: &mut Cache) -> Option<Snapshot> {
    None
}

fn prefill(_c: &mut Cache, _p: &Prompt) -> Answer {
    Answer
}

fn fill(_c: &mut Cache, _p: &Prompt) {}

struct Cache {
    kv: Vec<Row>,
    state: [f32; 64],
}

#[derive(Clone)]
struct Snapshot {
    rows: std::ops::Range<usize>,
}

struct Row;
struct Prompt;
struct Answer;

const SYSTEM_PROMPT: Prompt = Prompt;"#.to_string(),
        context_notes: Some(format!("{}", "Grade B− — decision-content, honestly NotBenchable (the win is a cold-replay elimination measured on the source's serving stack — a serving-latency axis no CPU validator hosts; the metric-mismatch class, `bench_spec: None`). TWO-SOURCE CONVERGENCE, both read at their pins: (1) the STAGED splash candidate (incoai/splash, the 11-08 blog revisit staging — 'For hybrid models, Splash also snapshots the GDN state at prefix boundaries, so cache reuse covers the recurrent layers too instead of replaying them', paged-KV prefix-indexed so a shared-prefix request reuses pages AND state; the face: on a long prefill only the draft state generation + the prefix snapshots need is computed) — the staging's 'plan-first serving batch' scoping is discharged here: the second instance arrives. (2) the anemll 'sysk' mechanism (fe8de185, V8_SERVER.md:70-72 verbatim): ''sysk': before a prompt that does not start with the 'system' snapshot's tokens rewrites the rows under it (Pi's summary request), that snapshot is kept with a copy of its KV rows (about 33 KB per token: 266 MB for 8.4K tokens, 13 to 28 ms). The conversation's next prompt restores it: the context entry grows to hold the rows, they are written back (5 to 17 ms), the GPU's V copy is refilled from row 0, the copy is freed.' — the live cell (V8_SERVER.md:211-212): 'restored at 8433 with 26 new tokens in 0.2 s (cold: about 13 s)'. THE ADMISSION LAW: reuse rides the consistency rule (`snapshot-state-kv-prefix-consistency-drop-invalid`, this batch) — the kept snapshot is restored ONLY through the same prefix-validity check; the copy-aside exists precisely so a VALID snapshot survives an invalidating writer. THE PRICING DISCIPLINE: the aside-copy has a byte cost (33 KB/token) and a time cost (13-28 ms), both named — the keep decision is a budget line, not a default; the copy is freed at restore (a lease, not a leak). Distinct-from, both directions: B337 `gdn-state-snapshot-per-position-verify-early-exit` (kernel_opt) owns WITHIN-GENERATION verify-early-exit state snapshots — THIS owns CROSS-REQUEST reuse of prefix-boundary snapshots; `wal-watermark-pinned-before-snapshot` (entries_81) owns capture order — compose; `snapshot-then-release-shared-drafter-context` (entries_14) owns releasing the owner's own lists at the hand-off — the rows_copy here is the KEPT side of that same seam discipline, compose; `frozen-artifact-plus-delta-tail-overlay` (entries_81) owns immutable-base + bounded-tail freshness — adjacent composition, different substrate (a file artifact vs live cache memory); the STAGED TensorFold fork-point row (10-28b, never landed) was the retention pole — no conflict. REGIME: hybrid/transformer serving stacks with recurrent-state layers (GDN/Mamba-class states whose snapshots must travel WITH the KV rows they were valid over) + any multi-request server with shared long prefixes (system prompts, few-shot heads). ANTI-REGIMES: value snapshots (rows copied already — reuse is trivial and this law is free); single-request processes; prefixes too short to amortize the copy (the pricing line decides). Fixture: b376_fixtures.rs F2 (the kept snapshot survives the rewrite and restores in the cheap path; the invalid kept snapshot falls to the consistency rule's cold path; the copy is freed at restore). Plan 387 / Research 421 (the fold of the 11-08 staged candidate at its second instance)")),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b376_rules() -> Vec<PerfRule> {
    vec![
        r1_snapshot_state_kv_prefix_consistency_drop_invalid(),
        r2_hybrid_state_prefix_snapshot_reuse(),
    ]
}
