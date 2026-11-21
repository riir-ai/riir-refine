//! Batch 373 rust_perf fixture preflight — the cargo-atlas transcription
//! pair (Plan 389 / Research 423): flowerornament/atlas @
//! `49d9ee41cca1cb8359fe28e2be13b430bba96569` (MIT OR Apache-2.0; the pin
//! clone was re-created for this batch — HEAD == pin verified — and both
//! mechanism files read in full: src/process.rs, src/render/pele/process.rs).
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! MECHANISMS are re-authored with our own constants and model harnesses;
//! the source quotes stay in the corpus context_notes). Compiled AND
//! executed BEFORE entry (the b340..b376 pre-entry discipline):
//!   cd src/domains/rust_perf/corpus/fixtures
//!   rustc --edition 2021 --test b389_fixtures.rs -o /tmp/b389_perf_fixtures
//!   /tmp/b389_perf_fixtures
//!
//! Arms per rule (buggy / fixed / anti-regime / tamper):
//! - F1 `child-exchange-concurrent-bounded-drain`: a SCRIPTED bounded
//!   child over an in-process pipe model (each pipe holds PIPE_CAP
//!   bytes; a write to a full pipe BLOCKS, a read of an empty one
//!   blocks — the OS pipe semantics the hazard needs, nothing
//!   spawned): the buggy serial write-then-read exchange STALLS
//!   (confirmed inside a bounded observation window, then the model
//!   is released — the buggy arm never hangs); the fixed
//!   concurrent-drain exchange completes the SAME exchange inside
//!   the window; the take(limit+1) refusal fires the typed
//!   "stream exceeds" error at limit+1 and passes at limit; the
//!   kill-on-first-failure arm stops a runaway child BEFORE the join
//!   and its write counter stays bounded; the Drop guard kill+waits
//!   on a setup-failure path that never joins; input_error rides
//!   beside a completed output.
//! - F2 `serialize-under-cap-refuses-before-spawn`: a cap-checking
//!   Write impl the (std-only, hand-rolled) JSON serializer streams
//!   into — a source UNDER the cap in characters but OVER it on the
//!   wire (each NUL escapes to six bytes) refuses MID-serialize with
//!   the remedy named, the spawn counter stays 0 and only a partial
//!   buffer exists (the child never starts); the serialize-then-
//!   measure arm pins the late refusal's spent buffer; the source-
//!   byte precheck arm pins the miscount ADMITTING a wire past the
//!   cap; the under-cap round-trip pins the escaped byte arithmetic
//!   and spawns exactly once.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ── F1 machinery: the pipe model ────────────────────────────────────────
// An in-process stand-in for an OS pipe: a bounded byte buffer whose
// writes BLOCK while full and whose reads BLOCK while empty — the two
// semantics the deadlock needs, woken every WAKE_MS so every parked
// side re-checks state (close/kill detection is prompt, nothing ever
// hangs). NOTHING spawns.

const PIPE_CAP: usize = 8; // a deliberately short pipe buffer
const WAKE_MS: u64 = 25;
const STALL_WINDOW: Duration = Duration::from_millis(300);
const REQUEST_BYTES: usize = 64; // 8x the pipe cap

struct KillFlag(AtomicBool);
impl KillFlag {
    fn new() -> Arc<Self> {
        Arc::new(Self(AtomicBool::new(false)))
    }
    fn trip(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn is_killed(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct PipeState {
    buf: VecDeque<u8>,
    write_closed: bool, // the write end is gone: reads drain then EOF
    read_closed: bool,  // the read end is gone: writes fail (EPIPE shape)
}

#[derive(Clone)]
struct Pipe {
    state: Arc<Mutex<PipeState>>,
    readable: Arc<Condvar>,
    writable: Arc<Condvar>,
    kill: Option<Arc<KillFlag>>, // a killed child: writes fail, reads drain then EOF
}

fn pipe(kill: Option<Arc<KillFlag>>) -> (Pipe, Pipe) {
    let core = Pipe {
        state: Arc::new(Mutex::new(PipeState::default())),
        readable: Arc::new(Condvar::new()),
        writable: Arc::new(Condvar::new()),
        kill: kill.clone(),
    };
    (core.clone(), core) // the two ends share one buffer, like an OS pipe
}

fn broken_pipe() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "pipe peer closed")
}

impl Write for Pipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut st = self.state.lock().unwrap();
        loop {
            if let Some(k) = &self.kill {
                if k.is_killed() {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "child killed"));
                }
            }
            if st.read_closed {
                return Err(broken_pipe());
            }
            if st.buf.len() < PIPE_CAP {
                let n = (PIPE_CAP - st.buf.len()).min(bytes.len());
                st.buf.extend(&bytes[..n]);
                drop(st);
                self.readable.notify_all();
                return Ok(n);
            }
            let (st2, _) = self
                .writable
                .wait_timeout(st, Duration::from_millis(WAKE_MS))
                .unwrap();
            st = st2;
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Pipe {
    // the writer finishing (or dying): the reader drains, then EOF
    fn close_write_end(&self) {
        let mut st = self.state.lock().unwrap();
        st.write_closed = true;
        drop(st);
        self.readable.notify_all();
    }
}

impl Read for Pipe {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let mut st = self.state.lock().unwrap();
        loop {
            if let Some(k) = &self.kill {
                // a killed child's already-written output stays readable;
                // after the drain, its output ends (EOF)
                if k.is_killed() && st.buf.is_empty() {
                    return Ok(0);
                }
            }
            if st.read_closed {
                return Err(broken_pipe());
            }
            if !st.buf.is_empty() {
                let n = st.buf.len().min(out.len());
                for slot in out.iter_mut().take(n) {
                    *slot = st.buf.pop_front().unwrap();
                }
                drop(st);
                self.writable.notify_all();
                return Ok(n);
            }
            if st.write_closed {
                return Ok(0); // EOF: the writer is gone and the buffer is empty
            }
            let (st2, _) = self
                .readable
                .wait_timeout(st, Duration::from_millis(WAKE_MS))
                .unwrap();
            st = st2;
        }
    }
}

impl Pipe {
    // the reader going away (the kill path): writes now fail
    fn close_read_end(&self) {
        let mut st = self.state.lock().unwrap();
        st.read_closed = true;
        drop(st);
        self.writable.notify_all();
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        // the LAST dropped end of an abandoned model must not park its peer
        // forever: dying ends close their side first (guarded: only fire once)
        if Arc::strong_count(&self.state) <= 1 && !self.state.lock().unwrap().write_closed {
            self.close_write_end();
        }
    }
}

// ── F1: the scripted children ───────────────────────────────────────────

// Mode A — the echo child: reads stdin, echoes it to stdout byte for
// byte, until stdin EOF. With a short pipe buffer and a request longer
// than the buffer this is exactly the shape that deadlocks a serial
// parent: the parent's write fills stdin, the child echoes into a full
// stdout, blocks, and stops reading stdin.
fn run_echo_child(mut stdin_r: Pipe, mut stdout_w: Pipe) {
    let mut buf = [0u8; 4];
    loop {
        match stdin_r.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if stdout_w.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
        }
    }
    stdout_w.close_write_end(); // the child exited: its reader drains, then EOF
}

// Mode B — the runaway child: never reads stdin, writes stdout forever
// (kill-checked each iteration). The exchange's bound + kill-first is
// what stops it; without the kill it would spin for the life of the model.
fn run_runaway_child(mut stdout_w: Pipe, kill: Arc<KillFlag>, written: Arc<AtomicUsize>) {
    let chunk = [0u8; 4];
    while !kill.is_killed() {
        if stdout_w.write_all(&chunk).is_err() {
            break;
        }
        written.fetch_add(chunk.len(), Ordering::SeqCst);
    }
    stdout_w.close_write_end();
}

// Mode C — the die-mid-request child: reads 4 bytes, closes its stdin
// read end (the parent's write now fails), echoes those 4 bytes, exits.
fn run_die_after_four_child(mut stdin_r: Pipe, mut stdout_w: Pipe) {
    let mut buf = [0u8; 4];
    let n = stdin_r.read(&mut buf).unwrap_or(0);
    stdin_r.close_read_end(); // the child dies mid-request
    if n > 0 {
        let _ = stdout_w.write_all(&buf[..n]);
    }
    stdout_w.close_write_end();
}

// ── F1: the exchanges ───────────────────────────────────────────────────

// THE BUGGY FORM: write the whole request, THEN read. Stalls on a short
// pipe with an echo child and a request longer than the buffer.
fn exchange_serial(mut stdin_w: Pipe, mut stdout_r: Pipe, request: &[u8]) -> io::Result<Vec<u8>> {
    stdin_w.write_all(request)?; // BLOCKS here once the pipes fill
    stdin_w.close_write_end();
    let mut out = Vec::new();
    stdout_r.read_to_end(&mut out)?;
    Ok(out)
}

#[derive(Clone, Copy, PartialEq)]
enum Completion {
    Done,
    Failed,
}

#[derive(Debug)]
struct Exchange {
    output: Vec<u8>,
    input_error: Option<io::Error>,
}

// THE FIXED FORM: the request written CONCURRENTLY with the output
// drained, the drain take(limit + 1)-bounded, the child stopped on the
// first failure BEFORE the joins, delivery failure carried beside output.
fn exchange_concurrent(
    stdin_w: Pipe,
    stdout_r: Pipe,
    request: Arc<Vec<u8>>,
    limit: usize,
    kill: Arc<KillFlag>,
) -> io::Result<Exchange> {
    thread::scope(|scope| {
        let (sender, receiver) = mpsc::channel::<Completion>();

        // the writer thread: deliver the request, drop stdin (unblocks a
        // child waiting on more input), report the outcome
        let sent = sender.clone();
        let mut w = stdin_w;
        let req = request.clone();
        let writer = scope.spawn(move || {
            let result = w.write_all(&req);
            w.close_write_end();
            let _ = sent.send(if result.is_ok() {
                Completion::Done
            } else {
                Completion::Failed
            });
            result
        });

        // the reader thread: bounded drain — take(limit + 1)
        let mut r = stdout_r;
        let reader = scope.spawn(move || {
            let result = bounded(&mut r, limit);
            let _ = sender.send(if result.is_ok() {
                Completion::Done
            } else {
                Completion::Failed
            });
            result
        });

        // On the first failure, stop the child before joining its pipes.
        // (All-Done exits via channel disconnect: both senders die with
        // their threads, mirroring the source's moved-original pattern.)
        for completion in receiver {
            if completion == Completion::Failed {
                kill.trip(); // the model's kill = the child stops
                break;
            }
        }

        let input_error = writer.join().unwrap().err();
        let output = reader.join().unwrap()?;
        Ok(Exchange {
            output,
            input_error,
        })
    })
}

// the bounded drain, mirroring the source's take(limit + 1) shape
fn bounded(input: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let count = u64::try_from(limit)
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| io::Error::other("invalid stream limit"))?;
    let mut bytes = Vec::new();
    input.take(count).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::other(format!("stream exceeds {limit} bytes")));
    }
    Ok(bytes)
}

// ── F1: the Drop guard (kill + wait on every path) ──────────────────────

struct ChildMock {
    killed: Arc<AtomicBool>,
    waited: Arc<AtomicBool>,
}
impl ChildMock {
    fn kill(&self) {
        self.killed.store(true, Ordering::SeqCst);
    }
    fn wait(&self) {
        self.waited.store(true, Ordering::SeqCst);
    }
}
struct Running(ChildMock);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.kill();
        self.0.wait();
    }
}

// a setup failure (the pipe was never opened) returns Err — and `running`
// drops on that path, killing + waiting the child though nothing joined
fn setup_exchange(child: ChildMock) -> Result<Pipe, &'static str> {
    let _running = Running(child); // the guard's whole job is its Drop on the error path
    let stdin: Option<Pipe> = None; // the simulated setup failure
    let stdin = stdin.ok_or("child stdin unavailable")?;
    Ok(stdin) // never reached in the failure arm
}

// ── F1 arms ─────────────────────────────────────────────────────────────

#[test]
fn f1_serial_write_then_read_stalls_on_a_short_pipe() {
    let kill = KillFlag::new();
    let (stdin_w, stdin_r) = pipe(Some(kill.clone()));
    let (stdout_w, stdout_r) = pipe(Some(kill.clone()));
    let child = thread::spawn(move || run_echo_child(stdin_r, stdout_w));
    let request: Vec<u8> = vec![0x41; REQUEST_BYTES];

    let (done_tx, done_rx) = mpsc::channel();
    let started = Instant::now();
    thread::spawn(move || {
        let r = exchange_serial(stdin_w, stdout_r, &request);
        let _ = done_tx.send(r);
    });

    // the bounded observation: the serial exchange must NOT complete
    // inside the window — the stall confirmed; nothing hangs (the model
    // is released right after)
    let stalled = done_rx.recv_timeout(STALL_WINDOW).is_err();
    assert!(
        stalled,
        "the serial exchange completed inside the window — the stall did not reproduce"
    );

    kill.trip(); // release: the model's kill unblocks both parked sides
    let outcome = done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("the released serial exchange must return");
    assert!(
        outcome.is_err(),
        "the released serial exchange reports the broken pipe"
    );
    let _ = child.join();
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the whole arm stays bounded"
    );
}

#[test]
fn f1_concurrent_drain_completes_the_same_exchange() {
    let kill = KillFlag::new();
    let (stdin_w, stdin_r) = pipe(Some(kill.clone()));
    let (stdout_w, stdout_r) = pipe(Some(kill.clone()));
    let child = thread::spawn(move || run_echo_child(stdin_r, stdout_w));
    let request = Arc::new(vec![0x42u8; REQUEST_BYTES]);

    let started = Instant::now();
    let exchange = exchange_concurrent(stdin_w, stdout_r, request.clone(), REQUEST_BYTES * 4, kill)
        .expect("the concurrent drain completes the exchange the serial form stalled on");
    let elapsed = started.elapsed();

    assert_eq!(
        exchange.output, *request,
        "the echo child's output came back whole — the drain kept pace with the write"
    );
    assert!(
        exchange.input_error.is_none(),
        "the request was delivered: no input_error"
    );
    assert!(
        elapsed < STALL_WINDOW,
        "the fixed form finishes inside the window the buggy form stalled past ({elapsed:?})"
    );
    let _ = child.join();
}

#[test]
fn f1_stream_limit_refusal_is_typed_at_limit_plus_one() {
    // exactly limit passes; limit + 1 refuses, naming the limit
    let data: Vec<u8> = (0..64u8).collect();
    let ok = bounded(&data[..], 64).expect("limit itself passes");
    assert_eq!(ok.len(), 64);
    let err = bounded(&data[..], 63).expect_err("limit + 1 refuses");
    assert!(
        err.to_string().contains("stream exceeds 63 bytes"),
        "the refusal names the limit: {err}"
    );
    // the empty stream is an answer, not a refusal
    assert_eq!(bounded(&[][..], 16).expect("empty passes").len(), 0);
}

#[test]
fn f1_first_failure_kills_the_child_before_the_join() {
    let kill = KillFlag::new();
    let (stdin_w, stdin_r) = pipe(Some(kill.clone()));
    let (stdout_w, stdout_r) = pipe(Some(kill.clone()));
    let written = Arc::new(AtomicUsize::new(0));
    let child = {
        let written = written.clone();
        let kill = kill.clone();
        thread::spawn(move || run_runaway_child(stdout_w, kill, written))
    };
    // the reader's bound is the first failure: 65 bytes refuse
    let request = Arc::new(Vec::new());
    let started = Instant::now();
    let err = exchange_concurrent(stdin_w, stdout_r, request, 64, kill.clone())
        .expect_err("the bound refusal surfaces");
    assert!(
        err.to_string().contains("stream exceeds 64 bytes"),
        "the typed refusal fires: {err}"
    );
    assert!(
        kill.is_killed(),
        "the exchange stopped the child on the first failure"
    );
    let _ = stdin_r; // the parent's stdin end: held for the model's lifetime
    let _ = child.join().expect("the runaway child stopped promptly");
    let total = written.load(Ordering::SeqCst);
    assert!(
        total < 256,
        "the kill bounded the runaway: {total} bytes written, not an unbounded stream"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "kill-first keeps the whole arm fast"
    );
}

#[test]
fn f1_drop_guard_kills_and_waits_on_setup_failure() {
    let killed = Arc::new(AtomicBool::new(false));
    let waited = Arc::new(AtomicBool::new(false));
    let child = ChildMock {
        killed: killed.clone(),
        waited: waited.clone(),
    };
    let result = setup_exchange(child);
    assert!(result.is_err(), "the setup failure refuses");
    assert!(
        killed.load(Ordering::SeqCst),
        "the Drop guard killed the child"
    );
    assert!(
        waited.load(Ordering::SeqCst),
        "the Drop guard waited the child"
    );
}

#[test]
fn f1_input_error_rides_beside_output() {
    let kill = KillFlag::new();
    let (stdin_w, stdin_r) = pipe(Some(kill.clone()));
    let (stdout_w, stdout_r) = pipe(Some(kill.clone()));
    let child = thread::spawn(move || run_die_after_four_child(stdin_r, stdout_w));
    let request = Arc::new(vec![0x43u8; REQUEST_BYTES]);

    let exchange = exchange_concurrent(stdin_w, stdout_r, request.clone(), REQUEST_BYTES, kill)
        .expect("the exchange completes despite the delivery failure");

    assert!(
        exchange.input_error.is_some(),
        "the request was NOT delivered: input_error is an independent observation"
    );
    assert_eq!(
        exchange.output,
        request[..4],
        "the output the child produced BEFORE dying still came back whole"
    );
    let _ = child.join();
}

// ── F2 machinery: the capped sink + the std-only serializer ─────────────

const MAX_REQUEST: usize = 64; // the fixture's own tiny cap

struct RequestBytes {
    buf: Vec<u8>,
}
impl Write for RequestBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_REQUEST - self.buf.len() {
            return Err(io::Error::other(
                "request exceeds 64 bytes; narrow the view",
            ));
        }
        self.buf.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// a std-only JSON string escaper streamed through the sink — every
// write() is a cap-check point, exactly like serde_json::to_writer
fn json_escape_into(w: &mut impl Write, s: &str) -> io::Result<()> {
    w.write_all(b"\"")?;
    for c in s.chars() {
        match c {
            '"' => w.write_all(b"\\\"")?,
            '\\' => w.write_all(b"\\\\")?,
            '\n' => w.write_all(b"\\n")?,
            '\u{0}' => w.write_all(b"\\u0000")?, // six wire bytes per NUL
            c if (c as u32) < 0x20 => write!(w, "\\u{:04x}", c as u32)?,
            c => {
                let mut buf = [0u8; 4];
                w.write_all(c.encode_utf8(&mut buf).as_bytes())?;
            }
        }
    }
    w.write_all(b"\"")
}

fn write_request(w: &mut impl Write, source: &str) -> io::Result<()> {
    w.write_all(b"{\"source\":")?;
    json_escape_into(w, source)?;
    w.write_all(b",\"prefix\":\"atlas\"}")
}

#[derive(Default)]
struct SpawnGuard {
    spawns: AtomicUsize,
}
impl SpawnGuard {
    fn spawn_child(&self) -> usize {
        self.spawns.fetch_add(1, Ordering::SeqCst) + 1
    }
    fn count(&self) -> usize {
        self.spawns.load(Ordering::SeqCst)
    }
}

// THE FIXED FORM: serialize within the cap, THEN start the child
fn render_mock(source: &str, guard: &SpawnGuard) -> io::Result<Vec<u8>> {
    let mut bytes = RequestBytes { buf: Vec::new() };
    write_request(&mut bytes, source)?; // the cap fires HERE, mid-serialize
    let request = bytes.buf;
    guard.spawn_child(); // the child starts only after serialization returned Ok
    Ok(request)
}

// NAIVE (1): serialize fully (uncapped), measure after
fn request_after(source: &str, guard: &SpawnGuard) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    write_request(&mut buf, source).unwrap(); // the whole wire, escapes included
    if buf.len() > MAX_REQUEST {
        return Err(io::Error::other(
            "request exceeds 64 bytes; narrow the view",
        ));
    }
    guard.spawn_child();
    Ok(buf)
}

// NAIVE (2): pre-check the SOURCE length — admits a wire past the cap
fn request_precheck(source: &str, guard: &SpawnGuard) -> io::Result<Vec<u8>> {
    if source.len() > MAX_REQUEST {
        return Err(io::Error::other(
            "request exceeds 64 bytes; narrow the view",
        ));
    }
    let mut buf = Vec::new();
    write_request(&mut buf, source).unwrap();
    guard.spawn_child(); // the child RECEIVES the oversize request
    Ok(buf)
}

// ── F2 arms ─────────────────────────────────────────────────────────────

#[test]
fn f2_oversize_via_escaping_refuses_mid_serialize_before_spawn() {
    let guard = SpawnGuard::default();
    // 10 NULs: 10 characters (far under the cap) escaping to 60 wire
    // bytes — the wrapper pushes the wire past 64. Mirrors the source's
    // own test source shape: "\0".repeat(MAX_REQUEST / 6).
    let source = "\u{0}".repeat(10);
    assert!(
        source.len() <= MAX_REQUEST,
        "the source is UNDER the cap in characters: {}",
        source.len()
    );

    let mut bytes = RequestBytes { buf: Vec::new() };
    let err = write_request(&mut bytes, &source).expect_err("oversize must refuse");
    assert!(
        err.to_string().contains("exceeds 64"),
        "the refusal names the cap and the remedy: {err}"
    );
    assert_eq!(guard.count(), 0, "the child never spawned");
    assert!(
        !bytes.buf.is_empty() && bytes.buf.len() < MAX_REQUEST,
        "the refusal fired MID-serialize with a partial buffer: {} bytes",
        bytes.buf.len()
    );

    // and through the render lane: the spawn line never runs on Err
    assert!(render_mock(&source, &guard).is_err());
    assert_eq!(
        guard.count(),
        0,
        "still zero spawns after the render refusal"
    );
}

#[test]
fn f2_serialize_then_measure_spends_the_buffer_before_the_refusal() {
    let guard = SpawnGuard::default();
    let source = "\u{0}".repeat(10);
    let err = request_after(&source, &guard).expect_err("the late check still refuses");
    assert!(err.to_string().contains("exceeds 64"));
    assert_eq!(guard.count(), 0, "it refused before spawning — but LATE");
    // the pin: the full oversized buffer was already built when the cap
    // fired. Reproduce what serialize_full held:
    let mut full = Vec::new();
    write_request(&mut full, &source).unwrap();
    assert!(
        full.len() > MAX_REQUEST,
        "the naive form built {} bytes past the {}-byte cap before refusing",
        full.len(),
        MAX_REQUEST
    );
}

#[test]
fn f2_source_byte_precheck_admits_a_wire_past_the_cap() {
    let guard = SpawnGuard::default();
    let source = "\u{0}".repeat(10);
    // the precheck ADMITS (10 source bytes <= 64) and hands the child a
    // 90-byte wire — the miscount in its wrong direction
    let wire = request_precheck(&source, &guard).expect("the precheck admits");
    assert_eq!(
        guard.count(),
        1,
        "the child spawned for an undeliverable request"
    );
    assert!(
        wire.len() > MAX_REQUEST,
        "the delivered wire is past the cap: {} bytes",
        wire.len()
    );
    // the pin both ways: source chars under, wire bytes over
    assert!(source.chars().count() <= MAX_REQUEST);
    assert!(wire.len() > MAX_REQUEST);
}

#[test]
fn f2_under_cap_round_trips_and_pins_the_escaped_bytes() {
    let guard = SpawnGuard::default();
    let source = "caf\u{e9} \"q\"";
    let wire = render_mock(source, &guard).expect("under-cap serializes fine");
    assert_eq!(guard.count(), 1, "exactly one child for the one request");
    // pinned wire arithmetic: {"source": (10) + quotes (2) + escaped string + ,"prefix":"atlas"} (18)
    // c a f = 3 ascii; é = 2 wire bytes; space = 1; \"q\" = 2+1+2 → source escapes to 11
    assert_eq!(
        wire.len(),
        10 + 2 + 11 + 18,
        "the wire counts ESCAPED bytes: {wire:?}"
    );
    let expected: Vec<u8> = b"{\"source\":\"caf\xC3\xA9 \\\"q\\\"\",\"prefix\":\"atlas\"}".to_vec();
    assert_eq!(wire, expected, "the wire round-trips the source escaped");
    assert!(wire.len() <= MAX_REQUEST);
}

fn main() {
    // the preflight harness: run every arm, exit nonzero on panic (the
    // `--test` binary does this itself; this main is the non-test runner)
    f1_serial_write_then_read_stalls_on_a_short_pipe();
    f1_concurrent_drain_completes_the_same_exchange();
    f1_stream_limit_refusal_is_typed_at_limit_plus_one();
    f1_first_failure_kills_the_child_before_the_join();
    f1_drop_guard_kills_and_waits_on_setup_failure();
    f1_input_error_rides_beside_output();
    f2_oversize_via_escaping_refuses_mid_serialize_before_spawn();
    f2_serialize_then_measure_spends_the_buffer_before_the_refusal();
    f2_source_byte_precheck_admits_a_wire_past_the_cap();
    f2_under_cap_round_trips_and_pins_the_escaped_bytes();
    println!("b389 fixtures: 10/10 green");
}
