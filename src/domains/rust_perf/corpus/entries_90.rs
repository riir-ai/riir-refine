// Batch 373 — the cargo-atlas transcription pair (the rust_perf half of
// Plan 389; the sec pair R1/R2 is the sec corpus's own lane) — Research
// 423 (flowerornament/atlas @ 49d9ee41cca1cb8359fe28e2be13b430bba96569,
// MIT OR Apache-2.0; the verdict clone was removed at verdict close per
// Research 423 — the pin clone was re-created for this batch and read
// fresh: HEAD == pin verified at `.raw/atlas` before the first corpus
// edit, Plan 389 T0.1's standing rule).
//
// rust_perf 380 → 382: R3 `child-exchange-concurrent-bounded-drain` (io,
// the owned-child exchange law — B) + R4 `serialize-under-cap-refuses-
// before-spawn` (io, GATED AT AUTHORING per Plan 389 T1.4 and LANDED:
// the mid-serialize cap-checking Write impl adjudicated DISTINCT under
// one read of both targets — vs B274 `batch-load-admission-sized-from-
// files-refuses-past-the-hard-budget` (entries_47): admission where the
// count is METADATA-derivable before the work begins; atlas where the
// only true count is the work's own OUTPUT (escaped wire bytes — their
// own test name pins it: `request_limit_counts_escaped_json_and_refuses_
// before_spawning`), so the cap rides the serializer's Write sink and
// the refusal lands MID-serialize before the child spawns — the
// mechanisms compose, never collide; vs entries_58 `reserve-then-commit-
// capacity-accounting`: a single-shot exact per-write check against
// cap − written, no store, no eviction, no reservation, no race window).
//
// Fixtures `corpus/fixtures/b389_fixtures.rs` F1–F2 are ORIGINAL
// plain-Rust reproductions (zero repo source pasted; the F1 child is an
// in-process scripted model over a blocking-bounded pipe pair — the
// hazard shape demonstrated WITHOUT a real child and WITHOUT a hang,
// every arm well under 2 s), standalone-compiled AND executed BEFORE
// entry. NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R3: one owned child exchange — concurrent pipes, bounded reads ──
pub fn r3_child_exchange_concurrent_bounded_drain() -> PerfRule {
    PerfRule {
        rule_id: "child-exchange-concurrent-bounded-drain".to_string(),
        name: "a child that receives a request AND produces output needs its pipes drained CONCURRENTLY with the write — the request is written on its own thread while BOTH output streams drain on theirs, each stream take(limit + 1)-bounded with a typed refusal (\"stream exceeds {limit} bytes\"), the child killed on the FIRST pipe failure BEFORE the remaining pipes are joined (a completion channel, never polling), and a Drop guard kills + waits on every path (setup, thread-creation, read, write, join failures) — serial write-then-read deadlocks once the child's output exceeds the OS pipe buffer while the request still fills stdin (the child blocks writing stdout, stops reading stdin, the write never completes), and an unbounded read lets a runaway child stream into memory until the host OOMs; child failure and failure to deliver the request are carried as independent observations (input_error beside Output), never folded into one verdict".to_string(),
        category: "io".to_string(),
        buggy_code: r#"// NAIVE: write the whole request, THEN read the output — and read
// it unbounded. Once the child's output fills the OS pipe buffer
// (~64 KiB class) while our request still sits in stdin, the child
// blocks writing stdout, stops reading stdin, and our write_all
// blocks with it: both sides wait on each other forever. The
// read_to_end after it never even runs — and where a reader DOES
// run unbounded, a runaway child that never closes stdout grows
// the buffer until the host OOMs.
fn exchange_serial(child: &mut Child, request: &[u8]) -> io::Result<Vec<u8>> {
    child.stdin.as_mut().unwrap().write_all(request)?; // BLOCKS: the child is stalled on stdout
    let mut out = Vec::new();
    child.stdout.as_mut().unwrap().read_to_end(&mut out)?; // unbounded: the runaway arm
    Ok(out)
}

// the failure shape at the call site: one Result folds "the child
// failed" and "the request was never delivered" into one verdict —
// the diagnostic cannot tell them apart."#.to_string(),
        fixed_code: r#"// FIXED: one owned exchange — the request written on its own thread
// while BOTH output pipes drain on theirs, every stream
// take(limit + 1)-bounded, the child killed on the first failure
// BEFORE the remaining joins, a Drop guard kill+wait on every path,
// and delivery failure carried beside the exit.
struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    } // Drop also covers setup, thread creation, read, write and join failures
}

fn bounded(input: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let count = u64::try_from(limit).ok().and_then(|n| n.checked_add(1))
        .ok_or_else(|| io::Error::other("invalid stream limit"))?;
    let mut bytes = Vec::new();
    input.take(count).read_to_end(&mut bytes)?; // limit + 1: one byte past the cap detects overflow
    if bytes.len() > limit {
        return Err(io::Error::other(format!("stream exceeds {limit} bytes")));
    }
    Ok(bytes)
}

fn exchange(mut command: Command, request: &[u8], limits: Limits) -> Result<Exchange> {
    let running = Running(command
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn()?); // guarded from the first line: every error path kills + waits
    thread::scope(|scope| {
        let mut running = running;
        let mut stdin = running.0.stdin.take().ok_or("child stdin unavailable")?;
        let stdout = running.0.stdout.take().ok_or("child stdout unavailable")?;
        let stderr = running.0.stderr.take().ok_or("child stderr unavailable")?;
        let (sender, receiver) = mpsc::channel();
        // writer thread: write_all(request), drop(stdin), report Done/Failed
        // reader threads: bounded(stdout, limits.stdout) / bounded(stderr, …)
        // On the first failure, stop the child before joining its pipes.
        // Completion events avoid polling or waiting on a child with full pipes.
        for completion in receiver {
            if completion == Completion::Failed {
                if running.0.try_wait()?.is_none() { let _ = running.0.kill(); }
                break; // still collect bounded stderr so an early refusal is not lost
            }
        }
        let input_error = writer.join().map_err(|_| "child input writer failed")?.err();
        let stdout = out.join().map_err(|_| "child output reader failed")?
            .map_err(|e| format!("stdout: {e}"))?;
        let stderr = err.join().map_err(|_| "child diagnostic reader failed")?
            .map_err(|e| format!("diagnostics: {e}"))?;
        let status = running.0.wait()?;
        Ok(Exchange { output: Output { status, stdout, stderr }, input_error })
    })
}"#.to_string(),
        context_notes: Some("Grade B — suggestion-only, honestly NotBenchable-by-domain (the win metric is DEADLOCK-FREEDOM plus a runaway child's memory ceiling — liveness on a real child's pipes; no wall/alloc validator prices a hang or an OOM, `bench_spec: None`). SOURCE (flowerornament/atlas @ 49d9ee41cca1cb8359fe28e2be13b430bba96569, MIT OR Apache-2.0; the verdict clone was removed at verdict close per Research 423 — re-cloned at the pin for this batch and HEAD == pin verified before the first corpus edit, Plan 389 T0.1; src/process.rs read IN FULL — the whole 127-line module, one owned exchange). THE LAW (src/process.rs:1-3 at the pin verbatim): 'One owned child exchange: concurrent pipes, bounded reads, cleanup on failure. Limits belong to each protocol. This bounds I/O, not total execution time; explicitly invoked programs must close their pipes and not delegate them.' THE API SPLIT (:18 verbatim): 'Child failure and failure to deliver its request are independent observations.' — `Exchange { output: Output, input_error: Option<io::Error> }`: the exit and the delivery failure carried side by side, never folded into one verdict (the render lane renders both: 'Pele render failed (...): <stderr>; request write: <input_error>'). THE GUARD (:29 verbatim): 'Drop also covers setup, thread creation, read, write and join failures.' — `Running(Child)` whose Drop is kill + wait, so EVERY error path (stdin/stdout/stderr take failures, spawn_scoped failures, join panics) stops the child; the scoped-threads comment (:69): 'This guard drops before scope joins workers, unblocking them on error.' THE BOUND (:38-49): each stream read through `take(limit.checked_add(1))` — one byte past the cap DETECTS overflow — with the typed refusal at :46: format!(\"stream exceeds {limit} bytes\"); the +1 arithmetic itself is checked ('invalid stream limit' on overflow). THE KILL-FIRST (:93-94 verbatim): 'On the first failure, stop the child before joining its pipes. Completion events avoid polling or waiting on a child with full pipes.' — a completion channel (Done/Failed) from each of the three workers; the FIRST Failed stops the child (try_wait retains an already-observed exit, else kill) BEFORE the remaining joins; (:97-99 verbatim): 'Retain any already-observed exit; otherwise stop the child. Still collect bounded stderr so an early refusal is not lost.' THE HAZARD (mechanism restatement — the doc's 'concurrent pipes' premise made concrete): a serial write-then-read exchange deadlocks once the child's output exceeds the OS pipe buffer (~64 KiB class) while the request still holds stdin — the child blocks writing stdout, stops reading stdin, and write_all never returns; the read_to_end after it never runs, and where a reader DOES run unbounded, read_to_end is the runaway arm — a child that never closes stdout grows the buffer until the host OOMs. take(limit + 1) is the two-sided answer: limit itself passes, limit + 1 refuses. DEDUP (T0.2 at the landing HEAD, 3 vocabularies: deadlock | stdin | Stdio | pipe | take( | stream exceeds — ZERO owners): entries_08 `host-lock-across-reentrant-blocking-call` owns the HOST-LOCK re-entrancy axis (a blocking call under a lock the callback re-takes — deadlock by lock scope, no pipes); the sec corpus's Stdio::piped() rows own the TOKEN-ON-STDIN secrecy posture (which streams get piped at all — secret-exposure, never liveness). ADJUDICATION both directions: `bounded-batch-buffer-pool` (entries_06) owns the ALLOCATION axis (a recycled-buffer pool's HARD cap on RETAINED capacity — memory pricing on our own buffers); THIS owns PIPE-LIVENESS (the bound is a read ceiling on a FOREIGN process's stream so the exchange can finish and a runaway child cannot grow memory — no pool, no ownership); compose (a drained exchange feeding a pooled consumer). `slow-consumer-latest-value-single-slot` (entries_70) owns COALESCING under a slow consumer (latest-wins drops, producer never blocks — freshness over completeness, no request to deliver); THIS owns the EXCHANGE (the request MUST be delivered whole and the output MUST be read — dropping either IS the failure); compose (an exchange feeding a latest-value slot). `child-piped-stderr-drain-for-process-lifetime` (entries_55) is the KIN — THAT owns the drain-for-LIFE of a long-lived child's stream (a supervisor's reader; the child outlives the exchange); THIS owns the bounded REQUEST/RESPONSE exchange (scoped reader threads, every stream take-bounded, child killed on first failure, pipes close when the exchange ends); compose (long-lived chatty child = the drain-for-life lane; request/response helper = THIS lane). AT-HOME POSITIVE INSTANCE (flagged, never refused): src/kat_migrate_normal.rs `gh_logged_in_as` — `gh api user` with stdout(Stdio::piped()) + stderr(Stdio::null()) + `.output()`: std's output() drains both streams to completion and there is NO request to deliver, so the serial hazard cannot fire — the law's own anti-regime, recorded as the at-home shape. REGIME: any child exchange where the parent writes a request AND reads output over pipes (renderer/protocol/helper invocations — formatter children, tool RPC, diagram renderers); limits belong to each protocol (the caller's own caps — I/O bounded, never execution time). ANTI-REGIMES: one-sided children (no request to deliver — Command::output() drains both streams by construction; the at-home instance above); children delegating their pipes (the source's own caveat: 'explicitly invoked programs must close their pipes and not delegate them'); wall-clock execution limits (the source: 'This bounds I/O, not total execution time' — a timeout-kill is a different axis, explicitly out of scope). Fixture: b389_fixtures.rs F1 (the serial stall CONFIRMED inside a bounded observation window over a scripted in-process child with 8-byte pipes — the buggy arm released, never hung; the SAME exchange completed by the concurrent drain inside the window; the take(limit+1) typed refusal at limit+1 and the pass at limit; the kill-on-first-failure arm stops a runaway child before the join and its write counter stays bounded; the Drop guard kill+wait on a setup-failure path that never joins; input_error beside a completed output). Plan 389 / Research 423 (flowerornament/atlas @ 49d9ee41cca1cb8359fe28e2be13b430bba96569, MIT OR Apache-2.0; the fixture is an original synthesis, standalone-compiled AND executed before entry).".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R4: the cap enforced DURING serialization, before the spawn ──
pub fn r4_serialize_under_cap_refuses_before_spawn() -> PerfRule {
    PerfRule {
        rule_id: "serialize-under-cap-refuses-before-spawn".to_string(),
        name: "a request serialized for a consumer gated on a byte cap must enforce the cap DURING the serialization — a cap-checking Write impl the serializer already streams into (every write checked against the room left, the FIRST oversize write refusing atomically with a message naming the remedy), never a serialize-then-measure (the oversized buffer is already built when the cap fires) and never a source-length pre-check (escaped wire bytes, not source character counts, determine the size — a source under the cap in characters can exceed it on the wire); the refusal lands mid-serialize so the expensive consumer never starts — the child is not spawned for a request that can never be delivered".to_string(),
        category: "io".to_string(),
        buggy_code: r#"// NAIVE (1): serialize fully, measure after — the oversized buffer
// is already built when the cap fires, and the spawn below still
// runs unless someone remembers to check before it.
fn request_after(source: &str) -> io::Result<Vec<u8>> {
    let bytes = serialize_full(source); // the whole wire, escapes included
    if bytes.len() > MAX_REQUEST {
        return Err(io::Error::other("request exceeds 16 MiB; narrow the view"));
    }
    Ok(bytes)
}

// NAIVE (2): pre-check the SOURCE length instead — cheaper, wrong:
// the wire counts ESCAPED bytes, and escapes grow the source (one
// NUL serializes to \u0000, six wire bytes), so a source under the
// cap in characters is over it on the wire.
fn request_precheck(source: &str) -> io::Result<Vec<u8>> {
    if source.len() > MAX_REQUEST {
        return Err(io::Error::other("request exceeds 16 MiB; narrow the view"));
    }
    Ok(serialize_full(source)) // the child receives a wire past the cap
}"#.to_string(),
        fixed_code: r#"// FIXED: enforce the cap where the bytes are produced — the Write
// impl the serializer streams into. Every write is checked against
// the room left; the first oversize write refuses ATOMICALLY
// (nothing half-valid is handed on) with the remedy named; the
// child spawns only after serialization returned Ok.
struct RequestBytes(Vec<u8>);
impl Write for RequestBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_REQUEST - self.0.len() {
            return Err(io::Error::other("request exceeds 16 MiB; narrow the view"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

fn request(source: &str) -> Result<Vec<u8>> {
    let mut bytes = RequestBytes(Vec::new());
    write_request(&mut bytes, source)?; // serde_json::to_writer(&mut bytes, …): mid-serialize refusal
    Ok(bytes.0)
}

fn render(source: &str) -> Result<String> {
    // Serialize within the cap before starting a child. Borrow the
    // inputs; escaped JSON bytes, not source character counts,
    // determine this limit.
    let request = request(source)?;
    let exchange = exchange(child_command(), &request, limits())?;
    // … the child never saw an oversize request: there was none.
    Ok(String::from_utf8_lossy(&exchange).into_owned())
}"#.to_string(),
        context_notes: Some("Grade C+/B− — suggestion-only, honestly NotBenchable (the win metric is SPAWN-AVOIDANCE plus the bounded build on oversize requests — a refusal-path property no wall/alloc validator prices; `bench_spec: None`). SOURCE (flowerornament/atlas @ 49d9ee41cca1cb8359fe28e2be13b430bba96569, MIT OR Apache-2.0; HEAD == pin verified this batch; src/render/pele/process.rs:12-56 + its test :110 read in full). THE LAW (src/render/pele/process.rs:12-14 at the pin): MAX_REQUEST = 16 MiB, MAX_SVG = 32 MiB, MAX_DIAGNOSTICS = 64 KiB — limits belong to each protocol. THE SINK (:23-36): `struct RequestBytes(Vec<u8>)` impls `Write`; every write checked against the room left (:25-29): `if bytes.len() > MAX_REQUEST - self.0.len() { return Err(io::Error::other(\"Pele request exceeds 16 MiB; narrow the view\")); }` — the FIRST oversize write refuses ATOMICALLY with the remedy named, and nothing half-valid is ever handed on. THE PLACEMENT (:51-52 verbatim): 'Serialize within the cap before starting a child. Borrow the inputs; escaped JSON bytes, not source character counts, determine this limit.' — `render` serializes to the sink FIRST and only a completed request reaches `process::exchange`; an oversize request refuses mid-serialize and the child never spawns. THEIR OWN TEST PINS THE ESCAPING FACE (:110): `request_limit_counts_escaped_json_and_refuses_before_spawning` — the body drives `render(&\"\\0\".repeat(MAX_REQUEST / 6), \"atlas\")` to the refusal: a source of MAX_REQUEST/6 NULs sits AT the cap in characters and PAST it on the wire (each NUL serializes to the six bytes \\u0000, plus the envelope), while the under-cap half round-trips the produced wire back to the source. THE GATE (Plan 389 T1.4 — LANDED on a two-sided read of both named targets): vs B274 `batch-load-admission-sized-from-files-refuses-past-the-hard-budget` (entries_47): THAT owns PRE-WORK ADMISSION where the count is DERIVABLE FROM METADATA before any work begins (files sized from their listings, nothing read; the ceiling is the consumer planner's hard budget; the refusal names the option remedy). THIS owns the case where the only TRUE count is the work's own OUTPUT — escaped wire bytes exist only after serializing, so an admission gate would either double-serialize (build to measure, then build again to hand over) or mis-measure on a source-length proxy (wrong in BOTH directions: a long plain-ASCII source at the char cap can sit under the wire cap, and a short control-char source at a fraction of the char cap is over it on the wire). The enforcement point is the differentiator: the cap rides the SERIALIZER'S OWN Write sink and the refusal lands MID-serialize, before the NEXT stage starts. The mechanisms COMPOSE — metadata admission where the count is derivable (B274), sink-cap where it is not (THIS). vs entries_58 `reserve-then-commit-capacity-accounting`: THAT owns a byte-budgeted STORE's insert/evict accounting (reserve the advertised size before the write lands, re-measure the real size at commit, evict only the delta — a race window between racing writers); THIS is a single-shot request with an EXACT per-write check against cap − written — no store, no eviction, no reservation, no race. DISTINCT. NEIGHBORS carried: `lower-bound-precheck-understates-never-fabricates` (entries_88, the prior batch) is the GOVERNOR on how such a cap may refuse (understate, never fabricate; THIS sink is exact — cap − written is the count itself, no bound involved); `request-amplification-guard-before-expensive-path` (sec, B374) owns the refuse-before-expensive-path PLACEMENT — compose (THIS is the mechanism that makes the early refusal possible when the count is the work's output). REGIME: any payload serialized for a consumer gated on a byte cap (child/IPC requests, upload bodies, queue messages, RPC frames) where oversize must refuse BEFORE the consumer starts; streaming serializers (serde_json::to_writer and kin — the sink pattern needs a Write-streaming serializer, never to_vec). ANTI-REGIMES: consumers that accept a stream with no total cap (chunk it — there is nothing to refuse); caps the CONSUMER itself enforces (one owner per cap — double-gating refuses requests the consumer would take); to_vec-then-measure is the pinned negative (the same count at the wrong point: the oversized allocation already happened). Fixture: b389_fixtures.rs F2 (the oversize-via-escaping refusal fires MID-serialize with spawn_count 0 and a partial buffer — the child never started; the serialize-then-measure arm pins the late refusal's spent buffer; the source-byte precheck arm pins the miscount ADMITTING a wire past the cap; the under-cap round-trip with the escaped byte count pinned). Plan 389 / Research 423 (flowerornament/atlas @ 49d9ee41cca1cb8359fe28e2be13b430bba96569, MIT OR Apache-2.0; the fixture is an original synthesis, standalone-compiled AND executed before entry).".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b389_rules() -> Vec<PerfRule> {
    vec![
        r3_child_exchange_concurrent_bounded_drain(),
        r4_serialize_under_cap_refuses_before_spawn(),
    ]
}
