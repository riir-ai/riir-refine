//! Entries slice - Batch 316 (2026-10-30): the impeccable io/process
//! robustness pool (Plan 316 / Research 356). FIVE `rust_perf` rules, all
//! ANNOTATED `NotBenchable` `bench_spec` None (io x4 + concurrency x1):
//!
//! - `per-read-timeout-vs-request-deadline` (io, pbakaus/impeccable @
//!   `02523aec` Apache-2.0) - the A- lead: `SO_RCVTIMEO` bounds ONE blocked
//!   read and any byte resets it, so a slow-drip or silent peer pins a
//!   request read forever under per-read timeouts alone; arm the total
//!   deadline at BOTH layers (socket timeout = the deadline so a parked
//!   read returns on every platform -- Windows does not unblock a recv
//!   already parked in the kernel when another thread shuts the socket
//!   down -- plus a 50 ms-poll watchdog that calls shutdown(Both)), and
//!   restore the prior timeout afterward. SEC CROSS-ROUTE recorded
//!   in-rule (the B293 cross-domain precedent).
//! - `child-piped-stderr-drain-for-process-lifetime` (io) - a piped
//!   stderr/stdout nobody reads is a fixed-size kernel pipe; when it
//!   fills, the child blocks on every later write and wedges mid-run.
//!   Drain it for the life of the process when it is load-bearing;
//!   `Stdio::null()` explicitly when it is not. Piped-and-unread is the
//!   unlawful posture.
//! - `kill-without-wait-zombie` (io) - `kill()` posts a signal, it does not
//!   reap: pair every kill with a wait, on error paths and Drop included.
//!   Home firing datum: this repo's own `src/l4_daemon.rs` EOF-during-load
//!   error path.
//! - `connect-timeout-walk-over-resolved-addrs` (io) - `TcpStream::
//!   connect` tries every resolved address under the ambient (unbounded)
//!   timeout: one black-holed address stalls the whole dial. Walk the
//!   resolved addresses yourself, per-address `connect_timeout`, keep the
//!   last error.
//! - `arrival-order-turnstile-lane` (concurrency) - thread-per-connection
//!   servers lose Node's end-callback atomicity: mutations reach the
//!   state lock in socket-completion order. The FIFO ticket lane
//!   serializes them in ARRIVAL order (issue at accept, `wait_turn` before
//!   state, release on finish / before parking / on Drop), unbounded on
//!   purpose, bounded upstream by the request deadline, poison-recovered.
//!
//! Plus ONE count-neutral rider face: the BOX-FILTER AREA-AVERAGE face on
//! `prefix-sum-2d` (`entries_01`, the same 2D-window-sum family -- the
//! source's resize is area-averaging down, bilinear up).
//!
//! Every quote is verbatim at the pin (the .raw clone HEAD-equal to
//! `02523aec3e4c4580102d996d1b6a71fbe062e03f` this session). Fixtures are
//! ORIGINAL machinery (never the source pasted), each compiled and
//! executed standalone (rustc --edition 2021 --test) before this table
//! was assembled; the buggy/fixed arm pair shares the surrounding harness
//! and differs only on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        PerfRule {
            rule_id: "per-read-timeout-vs-request-deadline".to_string(),
            name: "a per-read timeout bounds ONE blocked read and any received byte resets it - a slow-drip or silent peer keeps each read returning Ok(1) (or blocking under the window) while the REQUEST never completes, pinning whatever the connection holds (a pre-handler lane ticket, a thread, a file descriptor); bound one request's whole read with a total wall-clock deadline armed at BOTH layers: set the socket read timeout to the full deadline so a silent peer's parked read returns on every platform (Windows does not unblock a recv already parked in the kernel when another thread calls shutdown on the same socket - there the watchdog alone cannot enforce the deadline), keep a watchdog thread that polls a done flag at a coarse step and shuts the socket down at the deadline (the backstop for a connection that keeps trickling bytes without ever completing a request), and restore the prior read timeout when the read finishes".to_string(),
            category: "io".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `per-read-timeout-vs-request-deadline`
// (riir-refine rust_perf corpus, Batch 316) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's request-read lane (a
// thread-per-connection localhost server reading one request -- head plus
// Content-Length body -- off an accepted socket; impeccable
// common/http.rs is cited in context_notes).
//
// The reader bounds each individual `read` (the peer's drip arrives well
// inside every per-read window, so the per-read bound NEVER fires) but
// carries no deadline across the WHOLE read: a slow-drip or silent peer
// pins the read loop forever, and with it the connection's ticket in any
// pre-handler lane.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// What the peer's socket delivers: one byte per call -- the drip -- and
/// then never EOF (the connection stays open, the peer stays silent).
/// A real socket's per-read timeout would bound this call; the point is
/// that the drip ANSWERS inside any such window, so the bound never fires.
struct DripPeer {
    staged: Vec<u8>,
    pos: usize,
}

impl Read for DripPeer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.pos < self.staged.len() {
            buf[0] = self.staged[self.pos];
            self.pos += 1;
            return Ok(1);
        }
        // Past the staged bytes the request is still incomplete: the peer
        // keeps the connection open and sends nothing more. A read parked
        // here returns only when the (never-firing) per-read window
        // elapses or the peer closes -- neither happens on its own.
        std::thread::park();
        Ok(0)
    }
}

const HEAD_END: &[u8] = b"\r\n\r\n";
const BODY_LEN: usize = 8;

/// The per-read-timeout-only reader: every `read` is individually
/// bounded, no clock spans the request. THE DEFECT: with a drip peer the
/// loop consumes bytes forever and never returns.
fn read_request_per_read_only(peer: &mut DripPeer, done: Arc<AtomicBool>) {
    let mut buf: Vec<u8> = Vec::new();
    let mut head_seen = false;
    let mut body_seen = 0usize;
    let mut byte = [0u8; 1];
    loop {
        // Per-read bound only: the peer answers inside every window, so
        // this call always returns Ok(1) and the reset clock restarts.
        match peer.read(&mut byte) {
            Ok(0) => break, // peer closed: give up (never happens here)
            Ok(_) => {
                buf.push(byte[0]);
                if !head_seen && buf.ends_with(HEAD_END) {
                    head_seen = true;
                } else if head_seen {
                    body_seen += 1;
                }
                if head_seen && body_seen >= BODY_LEN {
                    break; // the complete request
                }
            }
            Err(_) => break, // per-read timeout would land HERE only
        }
    }
    done.store(true, Ordering::SeqCst);
}

fn make_drip_peer() -> DripPeer {
    // A real request's opening bytes -- then the drip stops mid-body.
    let mut staged = b"POST /upload HTTP/1.1\r\nHost: x\r\n\r\n".to_vec();
    staged.extend_from_slice(b"abcd"); // 4 of the 8 body bytes, then silence
    DripPeer { staged, pos: 0 }
}

#[test]
fn the_drip_peer_pins_the_per_read_timeout_only_reader_past_any_deadline() {
    // The very bytes that starve the fixed arm (see that arm's fixture)
    // starve this one forever: every read answers, so no per-read bound
    // fires and there is no total clock to fire either.
    let done = Arc::new(AtomicBool::new(false));
    let mut peer = make_drip_peer();
    let t = std::thread::spawn({
        let done = done.clone();
        move || read_request_per_read_only(&mut peer, done)
    });
    std::thread::sleep(std::time::Duration::from_millis(150));
    // A 100 ms total deadline (the source's own const) expired 50 ms ago;
    // a deadline-bounded reader would have returned by now. This one is
    // still inside the loop: the ticket it holds is pinned.
    assert!(
        !done.load(Ordering::SeqCst),
        "the per-read-timeout-only reader must still be pinned after the \
         total deadline expired: the drip answers every read, so no \
         per-read bound ever fires"
    );
    // The thread never finishes on its own (the peer never closes); the
    // test process's exit reaps it. Nothing here joins it -- that is the
    // defect: only a closed socket or a shutdown ends the loop.
    let _ = t;
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `per-read-timeout-vs-request-deadline`
// (riir-refine rust_perf corpus, Batch 316) -- the REMEDY arm.
//
// Same drip peer, same request -- but the read is bounded at BOTH layers,
// exactly the source's shape:
// (1) the socket layer: a read that makes no progress for its window
//     returns a timeout ON ITS OWN (the source sets the socket read
//     timeout to the full deadline -- needed because on Windows a recv
//     already parked in the kernel is NOT unblocked by another thread's
//     shutdown);
// (2) the watchdog layer: a thread polling a done flag calls
//     shutdown(Both) at the deadline -- modeled by a shutdown flag +
//     Thread::unpark -- so a connection that keeps trickling bytes (each
//     byte inside every window) still gets its read unblocked, and a
//     fast, normal read frees the watchdog promptly (no pinned thread).
// One deadline spans the whole request; a peer that never finishes it is
// given up on no matter how slowly it fed the socket.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::Thread;
use std::time::{Duration, Instant};

/// What the peer's socket delivers: one byte per call -- the drip -- then
/// silence. A read parks (the blocked recv); it returns when the socket
/// window expires (the layer-(1) stand-in) or when a shutdown lands (the
/// layer-(2) stand-in) -- whichever first.
struct DripPeer {
    staged: Vec<u8>,
    pos: usize,
    window: Duration,
    shut_down: Arc<AtomicBool>,
}

impl Read for DripPeer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.pos < self.staged.len() {
            buf[0] = self.staged[self.pos];
            self.pos += 1;
            return Ok(1);
        }
        // The peer never sends another byte and never closes. The read
        // parks in the kernel (modeled: a bounded sleep -- the kernel
        // wakes the read at the window) and returns when the window
        // expires or a shutdown lands -- whichever first.
        let since = Instant::now();
        loop {
            if self.shut_down.load(Ordering::SeqCst) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "shutdown(Both) unblocked the parked read",
                ));
            }
            let remaining = self.window.saturating_sub(since.elapsed());
            if remaining.is_zero() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "read window elapsed (the socket layer's own clock)",
                ));
            }
            std::thread::sleep(remaining.min(Duration::from_millis(10)));
        }
    }
}

const HEAD_END: &[u8] = b"\r\n\r\n";
const BODY_LEN: usize = 8;
/// The total wall-clock budget for ONE request read (the source's own
/// const is 10 s -- far more than any legitimate upload needs; 150 ms
/// here so the test is quick).
const READ_DEADLINE: Duration = Duration::from_millis(150);

#[derive(Debug, PartialEq, Eq)]
enum ReadOutcome {
    Complete,
    DeadlineExceeded,
    PeerClosed,
}

fn read_request_deadline(
    peer: &mut DripPeer,
    done: Arc<AtomicBool>,
    shut_down: Arc<AtomicBool>,
    reader_thread: Thread,
) -> ReadOutcome {
    let started = Instant::now();
    // Layer (2): the watchdog. Poll the done flag at a coarse step so a
    // fast, normal read frees it promptly instead of pinning a thread
    // for the whole deadline; at the deadline, shut the socket down --
    // which unblocks any read parked in it.
    let watch_done = done.clone();
    let watchdog = std::thread::spawn(move || {
        let mut waited = Duration::ZERO;
        let step = Duration::from_millis(25);
        while waited < READ_DEADLINE {
            if watch_done.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(step);
            waited += step;
        }
        if !watch_done.load(Ordering::SeqCst) {
            // Unblock any in-progress read so the request read returns;
            // the handler then drops its ticket and the connection
            // closes.
            shut_down.store(true, Ordering::SeqCst);
            reader_thread.unpark();
        }
    });
    let outcome = read_loop(peer, started);
    done.store(true, Ordering::SeqCst);
    watchdog.join().unwrap();
    outcome
}

fn read_loop(peer: &mut DripPeer, started: Instant) -> ReadOutcome {
    let mut buf: Vec<u8> = Vec::new();
    let mut head_seen = false;
    let mut body_seen = 0usize;
    let mut byte = [0u8; 1];
    loop {
        // THE LAW: one clock across the whole read, checked every
        // iteration. Any byte resets a per-read window; nothing resets
        // this.
        if started.elapsed() >= READ_DEADLINE {
            return ReadOutcome::DeadlineExceeded;
        }
        match peer.read(&mut byte) {
            Ok(0) => return ReadOutcome::PeerClosed,
            Ok(_) => {
                buf.push(byte[0]);
                if !head_seen && buf.ends_with(HEAD_END) {
                    head_seen = true;
                } else if head_seen {
                    body_seen += 1;
                }
                if head_seen && body_seen >= BODY_LEN {
                    return ReadOutcome::Complete;
                }
            }
            Err(_) => {
                // Either layer's clock fired (socket window or watchdog
                // shutdown): the read returned, the request is given up.
                return ReadOutcome::DeadlineExceeded;
            }
        }
    }
}

fn make_drip_peer(shut_down: Arc<AtomicBool>) -> DripPeer {
    let mut staged = b"POST /upload HTTP/1.1\r\nHost: x\r\n\r\n".to_vec();
    staged.extend_from_slice(b"abcd");
    DripPeer { staged, pos: 0, window: Duration::from_millis(300), shut_down }
}

#[test]
fn the_total_deadline_gives_up_on_the_same_drip_peer_the_per_read_bound_never_caught() {
    let done = Arc::new(AtomicBool::new(false));
    let shut_down = Arc::new(AtomicBool::new(false));
    let mut peer = make_drip_peer(shut_down.clone());
    let t0 = Instant::now();
    let outcome =
        read_request_deadline(&mut peer, done, shut_down, std::thread::current());
    let took = t0.elapsed();
    assert_eq!(
        outcome,
        ReadOutcome::DeadlineExceeded,
        "the identical drip that pinned the per-read-only arm is given up \
         on at the total deadline"
    );
    assert!(
        took >= READ_DEADLINE && took < READ_DEADLINE + Duration::from_millis(150),
        "the deadline fired on time (took {took:?}, deadline {READ_DEADLINE:?})"
    );
}

#[test]
fn a_fast_legitimate_request_still_completes_under_the_deadline() {
    // The deadline must not cost the honest peer: the same loop lets a
    // request that completes inside the window finish normally -- and the
    // watchdog exits early on the done flag instead of pinning a thread.
    let done = Arc::new(AtomicBool::new(false));
    let staged = {
        let mut s = b"POST /upload HTTP/1.1\r\nHost: x\r\n\r\n".to_vec();
        s.extend_from_slice(b"abcdefgh"); // the full 8-byte body
        s
    };
    let mut peer = DripPeer {
        staged,
        pos: 0,
        window: Duration::from_millis(300),
        shut_down: Arc::new(AtomicBool::new(false)),
    };
    let outcome = read_request_deadline(
        &mut peer,
        done,
        Arc::new(AtomicBool::new(false)),
        std::thread::current(),
    );
    assert_eq!(outcome, ReadOutcome::Complete);
}

#[test]
fn the_socket_layer_alone_catches_a_silent_peer_even_without_the_watchdog() {
    // The Windows case: no watchdog help, a peer parked in the read. The
    // socket window ITSELF returns the read (the platform-split reason
    // the source sets the socket timeout to the deadline in addition to
    // the watchdog).
    let mut peer = make_drip_peer(Arc::new(AtomicBool::new(false)));
    peer.window = Duration::from_millis(80);
    let mut byte = [0u8; 1];
    // exhaust the staged bytes (the window itself can fire mid-drain
    // only after the staged bytes run out -- 80 ms against ~40 instant
    // reads is safe, but read errors are part of the drill either way)
    loop {
        match peer.read(&mut byte) {
            Ok(1) => continue,
            Ok(_) => continue,
            Ok(0) | Err(_) => break, // drained or the window fired
        }
    }
    let t0 = Instant::now();
    let r = peer.read(&mut byte);
    let took = t0.elapsed();
    assert!(r.is_err(), "the window returned the parked read");
    assert!(
        took >= Duration::from_millis(80),
        "the window governed the return ({took:?})"
    );
}
"#.to_string(),
            context_notes: Some("Grade A- -- NotBenchable ON THIS BOX (the decision content is a request-lane LIVENESS law -- which timeout bounds which unit of work, and which platform needs which unblock mechanism -- priced in a server's behavior under an adversarial peer, not in any A/B microbench; a stand-in benchmark cannot measure a lane-hold at all; source-reported behavior is PROVENANCE, never bars). SOURCE (pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f, Apache-2.0; every quote verified verbatim in the .raw clone HEAD-equal to the pin this session; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT -- the source names it in the set_header_timeout doc, verbatim: 'Set a per-read timeout on the socket. This is SO_RCVTIMEO: it bounds a single blocked read, not the whole connection, and any byte received resets it. It is a coarse backstop only; the total lane-hold bound is read_request_deadline, not this.' and the read_request_deadline doc carries the failure shape verbatim: 'any byte inside the window resets it, so a slow-drip or silent peer keeps each read returning Ok(1) (or blocking under the window) and read_request never returns, pinning the connection's ticket and wedging every later mutation behind it.' THE LAW, three parts (crates/common/src/http.rs @ 02523aec, read_request_deadline): (1) THE SOCKET HALF -- set the read timeout to the FULL deadline, comment verbatim: 'Bound the read at the socket as well as at the watchdog. Windows does not unblock a recv already parked in the kernel when another thread calls shutdown on the same socket, so there the watchdog alone cannot enforce the deadline and a silent connection held its ticket for the whole 60s header timeout. A read that makes no progress for the deadline now returns on every platform and the handler drops its ticket; the watchdog stays as the backstop for a connection that keeps trickling bytes without ever completing a request.' -- the PLATFORM-SPLIT decision content: the socket timeout is the layer that works everywhere; the watchdog is the layer that also catches trickle; (2) THE WATCHDOG HALF -- a thread polling a done flag at a 50 ms step, comment verbatim: 'Poll the done flag so a fast, normal read frees the watchdog promptly instead of pinning a thread for the whole deadline.' -- at the deadline it calls shutdown(Both): 'Unblock any in-progress read so read_request returns; the handler then drops its ticket and the connection closes.'; (3) THE RESTORE -- the prior timeout is read before ('let prev_timeout = stream.read_timeout().ok().flatten();') and restored after ('let _ = stream.set_read_timeout(prev_timeout);') so the deadline never leaks into the connection's later life. The budget is a const, verbatim: 'pub const READ_REQUEST_DEADLINE: Duration = Duration::from_secs(10);' -- '10s is far more than any legitimate local upload needs'. The source's race test feeds ~4 KB over ~500 ms -- 'finishes in a twentieth of it'. PRODUCT COVERAGE AT HOME (verified firing this session, issues filed): riir-instinct src/serve_edge.rs (the /decide read loop sets a per-read timeout with no total deadline and no watchdog) + riir-reflex src/serve.rs (same shape) -- the exact pre-fix posture. REGIME: any hand-rolled request reader over accepted sockets (thread-per-connection or small-pool servers); any lane whose admission order is a resource (tickets, semaphores, per-connection buffers); any read loop where a peer controls the byte arrival. ANTI-REGIMES: libraries whose request reading already carries a total deadline; single-shot reads with no lane or resource behind them (a wasted thread is the whole cost); fully-async IO where the timeout is a select/future race rather than a socket attribute (the same LAW, a different mechanism); protocols with their own idle/keepalive framing that already bounds the exchange. SEC CROSS-ROUTE (recorded per the B293 cross-domain precedent): the slow-drip shape IS slowloris-class pre-auth connection exhaustion -- a future sec batch must route here, never re-mine it; the fix-space is hand-rolled std::net request reading, not a sec-domain (tower-http/jwt/cookie) shape. KINSHIP/DISTINCT-FROM (read + disclosed): the corpus's io category owns socket-option and read-shape rules (the TCP_NODELAY request-response rule, the nonblocking head-probe open, the vectored-write capability flag) -- none owns a TIMEOUT-SEMANTICS law (per-read vs per-request); watchdog-deadline-moves-on-command-progress (B270, entries_41) owns a COMMAND-progress deadline in a serving control plane -- the progress-moves-the-clock shape, not a socket-read shape; timed-zero-refuses-round-stop (B284, entries_50) owns the measurement-instrument zero (an instrument failure, not a lane bound); the source itself composes the two: this rule bounds the lane-hold that arrival-order-turnstile-lane (this batch) leaves unbounded on purpose. NOVEL: grep 'read_timeout|SO_RCVTIMEO|request.*deadline|watchdog' over src/ at the landing HEAD (03e6ca67 lineage): one consumer (src/seat_rerank.rs set_read_timeout(Some(remaining)) -- a deadline-TRACKED read, the correct shape already), the update_check/groq transport constants are CONNECT-timeout consumers -- zero corpus rule owns the per-read-vs-request-deadline law. Suggestion-only (the defect is invisible to a parse-only proposer: it needs the loop shape AND what the peer controls). FIXTURE PROVENANCE: original machinery (never impeccable code) -- a DripPeer Read stand-in (one byte per call, then silence, never EOF), the buggy arm's per-read-only loop (the drip answers every read, so no per-read bound ever fires; the test proves the reader is still pinned 50 ms past where a 100 ms total deadline would have fired), the fixed arm's deadline loop + 25 ms-poll watchdog (the shutdown(Both) half modeled by a shutdown flag + Thread::unpark; the socket half modeled by the peer's own window returning the read -- the Windows-case test proves the socket layer alone catches a silent peer) + the honest-peer arm (a complete request still completes); both arms share the harness and differ only on the law. Cite: pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f -- crates/common/src/http.rs (set_header_timeout doc, READ_REQUEST_DEADLINE const + doc, read_request_deadline body: the prev_timeout capture, the 50 ms watchdog step, the shutdown(Both) comment); Batch 316 / Plan 316 / Research 356.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "child-piped-stderr-drain-for-process-lifetime".to_string(),
            name: "a child's Stdio::piped() stream nobody reads is a fixed-size kernel pipe -- once it fills, every child write BLOCKS and a chatty child wedges mid-run (it stops writing, stops answering its stdin, and looks hung); if the stream is load-bearing, drain it for the life of the process on a dedicated reader thread (forwarding lines through a channel for whoever parses them); if it is NOT load-bearing, pass Stdio::null() explicitly so the kernel discards and no pipe exists to fill -- the unlawful posture is piped-and-unread".to_string(),
            category: "io".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `child-piped-stderr-drain-for-process-lifetime`
// (riir-refine rust_perf corpus, Batch 316) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's launch lane (impeccable
// browser/cdp.rs spawns Chromium with stderr(Stdio::piped()) and is cited
// in context_notes). An OS pipe is a FIXED-SIZE kernel buffer: once it is
// full, every child write BLOCKS until the parent reads. A parent that
// takes a piped stderr (or stdout) and never reads it wedges a chatty
// child mid-run: it stops writing, stops answering its stdin, and looks
// hung.
//
// Modeled here with a bounded sync channel (capacity 8 lines -- the
// stand-in for the kernel's pipe buffer): `send` blocks when full, exactly
// as the child's stderr write blocks when the pipe is full.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::time::Duration;

/// The chatty child: 200 log lines to stderr, then a final marker.
/// Progress (lines actually handed to the pipe) is observable.
fn chatty_child(tx: std::sync::mpsc::SyncSender<String>, progress: Arc<AtomicUsize>) {
    for i in 0..200 {
        let line = format!("log line {i}: loading page, metrics, warnings...");
        // A full pipe blocks the child HERE -- its write parks until the
        // parent reads. A wedged child stops making progress entirely.
        if tx.send(line).is_err() {
            return; // pipe closed (the null posture ends here)
        }
        progress.fetch_add(1, Ordering::SeqCst);
    }
    progress.store(200, Ordering::SeqCst); // only reached if never blocked
}

fn spawn_child() -> (
    std::thread::JoinHandle<()>,
    std::sync::mpsc::Receiver<String>,
    Arc<AtomicUsize>,
) {
    // Capacity 8: the kernel pipe buffer stand-in.
    let (tx, rx) = sync_channel::<String>(8);
    let progress = Arc::new(AtomicUsize::new(0));
    let t = std::thread::spawn({
        let tx = tx.clone();
        let progress = progress.clone();
        move || chatty_child(tx, progress)
    });
    (t, rx, progress)
}

#[test]
fn a_piped_but_unread_stderr_wedges_the_chatty_child_mid_run() {
    let (t, rx, progress) = spawn_child();
    // THE DEFECT: the parent holds the pipe and never reads it. The
    // buffer (8 lines) fills almost immediately; the child blocks on its
    // next write.
    std::thread::sleep(Duration::from_millis(200));
    let stalled = progress.load(Ordering::SeqCst);
    assert!(
        stalled < 200,
        "the child must be wedged well short of its 200 lines with the \
         parent not reading (stalled at {stalled}): the full pipe blocked \
         its write"
    );
    // Cleanup: drain so the child can finish and the thread joins.
    drop(rx);
    let _ = t.join();
}

#[test]
fn the_wedge_is_not_a_slow_child_it_is_the_full_pipe() {
    // The same child, same rate -- but read from, it finishes 200 lines
    // promptly. The stall in the other test is the pipe, not the child.
    let (t, rx, progress) = spawn_child();
    let drained = std::thread::spawn(move || {
        for line in rx {
            let _ = line; // the drain (see the fixed arm's fixture)
        }
    });
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        progress.load(Ordering::SeqCst),
        200,
        "read from, the same child finishes all 200 lines inside the same \
         window"
    );
    let _ = t.join();
    let _ = drained.join();
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `child-piped-stderr-drain-for-process-lifetime`
// (riir-refine rust_perf corpus, Batch 316) -- the REMEDY arm.
//
// Two lawful postures, both from the source (impeccable browser/cdp.rs is
// cited in context_notes):
// (1) stderr IS load-bearing (a browser's startup errors and the endpoint
//     line surface there): pipe it AND drain it for the life of the
//     process -- its own comment is the law: 'Drain stderr for the life of
//     the process so a chatty browser never blocks on a full pipe'.
// (2) stderr is NOT load-bearing (a quiet helper whose noise nobody
//     reads): Stdio::null() explicitly -- the kernel discards, no pipe
//     exists to fill, no reader thread is owed.
// The unlawful posture is piped-and-unread: the pipe fills and the child
// wedges mid-write.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn chatty_child(tx: std::sync::mpsc::SyncSender<String>, progress: Arc<AtomicUsize>) {
    for i in 0..200 {
        let line = format!("log line {i}: loading page, metrics, warnings...");
        if tx.send(line).is_err() {
            return;
        }
        progress.fetch_add(1, Ordering::SeqCst);
    }
    progress.store(200, Ordering::SeqCst);
}

fn spawn_child() -> (
    std::thread::JoinHandle<()>,
    std::sync::mpsc::Receiver<String>,
    Arc<AtomicUsize>,
) {
    let (tx, rx) = sync_channel::<String>(8);
    let progress = Arc::new(AtomicUsize::new(0));
    let t = std::thread::spawn({
        let tx = tx.clone();
        let progress = progress.clone();
        move || chatty_child(tx, progress)
    });
    (t, rx, progress)
}

/// Posture (1): the load-bearing pipe, drained for the LIFE of the
/// process on a dedicated reader thread. The child never blocks; lines
/// the parent has not consumed yet queue in the reader's hands.
fn drain_for_life(rx: std::sync::mpsc::Receiver<String>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in rx {
            // Forward / parse / log -- the source forwards through a
            // channel for endpoint discovery while a BufReader drains.
            let _ = line;
        }
    })
}

#[test]
fn a_drained_pipe_lets_the_same_chatty_child_finish() {
    let (t, rx, progress) = spawn_child();
    let drainer = drain_for_life(rx);
    let t0 = Instant::now();
    // Poll to completion -- no fixed sleep: prove the child finishes.
    let deadline = Instant::now() + Duration::from_secs(5);
    while progress.load(Ordering::SeqCst) < 200 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        progress.load(Ordering::SeqCst),
        200,
        "the drained pipe never wedges the child: all 200 lines written"
    );
    assert!(
        t0.elapsed() < Duration::from_secs(5),
        "the child finished promptly under the drain"
    );
    let _ = t.join();
    drainer.join().unwrap();
}

#[test]
fn the_quiet_helper_posture_discards_explicitly_and_owes_no_reader() {
    // Posture (2): Stdio::null() modeled as the kernel discard -- the
    // parent drops the pipe immediately; the child's writes fail fast
    // (never block) and it exits its loop. No reader thread is owed;
    // nothing wedges.
    let (t, rx, progress) = spawn_child();
    drop(rx); // the null posture: the discard happens at the OS boundary
    let t0 = Instant::now();
    let _ = t.join();
    assert!(
        t0.elapsed() < Duration::from_millis(500),
        "a child on a null sink exits its write loop fast (send errors, \
         never blocks)"
    );
    assert!(
        progress.load(Ordering::SeqCst) < 200,
        "the null posture discards output: the loop ended at the first \
         failed send, which is the point -- quiet helpers owe neither a \
         pipe nor a reader"
    );
}
"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is a process-lifetime LAW -- which streams a parent owes a reader for -- priced in a long-lived parent's behavior under a chatty child, not in any A/B microbench; a wedge is binary, its cost is a hung tool). SOURCE (pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f, Apache-2.0; quotes verified verbatim in the .raw clone HEAD-equal to the pin this session; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT -- a child spawned with a piped stream whose parent never reads it: the OS pipe is a fixed-size kernel buffer and the child's writes block once it fills. THE LAW, two lawful postures IN THE SAME SOURCE (crates/browser/src/cdp.rs @ 02523aec): (1) THE LOAD-BEARING PIPE IS DRAINED FOR LIFE -- launch takes '.stderr(Stdio::piped())' and the comment is the rule verbatim: 'Drain stderr for the life of the process so a chatty browser never blocks on a full pipe' -- the drain thread forwards lines through a channel (the endpoint-discovery consumer reads them) while a BufReader keeps draining; (2) THE QUIET HELPER IS NULL EXPLICITLY -- crates/live/src/svelte_bridge.rs spawns its helper with '.stderr(Stdio::null())' (line 42): nobody reads that helper's stderr, so no pipe exists to fill and no reader thread is owed; the same file family uses stdin(null) for one-shot probes whose stdin nobody would write. The unlawful posture is piped-and-unread -- the exact shape the rule name forbids. HOME ADJUDICATION (this session): src/kat_migrate_normal.rs + kat_migrate_client.rs + domains/docker/verify.rs all carry Stdio::null() or wait_with_output() -- consumers of the CORRECT postures; no firing site at home for this rule (the defects it prevents live in tools that spawn long-lived chatty children). REGIME: parents spawning long-lived or chatty children (browsers, language servers, emulators, daemons) where stderr/stdout is either load-bearing (drain it) or noise (null it); any spawn whose output volume is unbounded while the parent's interest is bounded. ANTI-REGIMES: short-lived children whose output fits the pipe buffer and whose parent wait()s promptly (the buffer absorbs it -- the wedge needs the child to OUTRUN the parent's interest); children already writing to files or inherited terminals; parents that read BOTH piped streams to completion (wait_with_output); supervised children whose supervisor multiplexes the pipes (a different mechanism, same law). KINSHIP/DISTINCT-FROM (read + disclosed): kill-without-wait-zombie (this batch) owns the EXIT half of child hygiene (the reaping) -- THIS owns the I/O half (the pipes) -- a wedged child is the PRE-kill failure this rule prevents and a zombie is the POST-kill leak the other prevents; the corpus's io category owns socket/file read-shape rules, none a child-pipe lifetime law. NOVEL: grep 'Stdio::null|piped.*drain|drain.*stderr' over src/ at the landing HEAD: consumers only (null-posture spawns + wait_with_output), zero corpus rule owns the piped-unread wedge law. Suggestion-only (a parse-only proposer cannot see spawn posture vs reader lifetime). FIXTURE PROVENANCE: original machinery (never impeccable code) -- a bounded sync_channel (capacity 8) as the kernel-pipe stand-in, a 200-line chatty child, the buggy arm's unread receiver (the child stalls short of 200 lines at 8+, pinned by the progress counter; the control test proves the SAME child finishes when read from -- the pipe, not the child), the fixed arm's drain_for_life reader (all 200 lines land, promptly) + the null posture (the receiver dropped: sends fail fast, the loop exits, nothing hangs, no reader owed); both arms share the harness and differ only on the law. Cite: pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f -- crates/browser/src/cdp.rs (launch: stderr(Stdio::piped()) + the drain-for-life thread + the forwarding channel) + crates/live/src/svelte_bridge.rs:42 (the Stdio::null() contrast); Batch 316 / Plan 316 / Research 356.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "kill-without-wait-zombie".to_string(),
            name: "kill() posts a signal -- it does not reap: the exited child stays in the process table as a zombie (defunct) until someone waits for it or the parent exits, holding a pid and a table entry per miss; pair EVERY kill with a wait, and carry the pairing on a Drop guard so error paths that skip the explicit close still reap (try_wait first: a child that already exited needs the wait, not the signal)".to_string(),
            category: "io".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `kill-without-wait-zombie`
// (riir-refine rust_perf corpus, Batch 316) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's child lifecycle (its
// Browser error paths and Drop guard are cited in context_notes; the
// home firing datum is riir-refine src/l4_daemon.rs:111 -- a kill()
// without wait() on the EOF-during-load error path).
//
// kill() posts a signal; it does NOT reap. The exited child stays in the
// process table as a zombie (defunct) until someone waits for it or the
// parent exits. A long-lived parent that spawns and kills children
// repeatedly accumulates one unreapable entry per miss.
//
// Modeled: a process table (the kernel's) + a Child handle. kill marks
// the child signaled/exited; ONLY wait removes the entry.

use std::collections::BTreeSet;

/// The kernel process table stand-in: live children and zombies both
/// hold entries; only a wait removes one.
#[derive(Default)]
struct ProcTable {
    entries: BTreeSet<u64>,
}

impl ProcTable {
    fn spawn(&mut self, pid: u64) {
        self.entries.insert(pid);
    }
    fn zombie_count(&self) -> usize {
        self.entries.len()
    }
    fn reap(&mut self, pid: u64) {
        self.entries.remove(&pid);
    }
}

struct Child<'a> {
    pid: u64,
    signaled: bool,
    exited: bool,
    table: &'a mut ProcTable,
}

impl<'a> Child<'a> {
    fn spawn(table: &'a mut ProcTable, pid: u64) -> Self {
        table.spawn(pid);
        Self { pid, signaled: false, exited: false, table }
    }

    /// kill(): post the signal. On a real OS the child dies -- but the
    /// table entry STAYS until a wait reaps it. kill() does not reap.
    fn kill(&mut self) {
        self.signaled = true;
        self.exited = true;
    }

    /// wait(): reap. Removes the table entry.
    fn wait(&mut self) {
        self.table.reap(self.pid);
    }
}

/// The defective path (modeled on l4_daemon.rs:111): a spawn/EOF error
/// path kills the child and returns the error -- no wait anywhere. The
/// handle drops; nothing reaps. THE DEFECT: the entry stays.
fn abort_load(table: &mut ProcTable, pid: u64) -> Result<(), String> {
    let mut child = Child::spawn(table, pid);
    // ... the child dies mid-load (EOF on its stdout before READY):
    let failed = true;
    if failed {
        child.kill();
        // THE DEFECT: no wait(). `child` drops here; the table entry
        // survives as a zombie.
        return Err("daemon exited during load without signalling READY".into());
    }
    Ok(())
}

#[test]
fn kill_without_wait_leaves_a_zombie_in_the_table() {
    let mut table = ProcTable::default();
    let err = abort_load(&mut table, 4242);
    assert!(err.is_err());
    assert_eq!(
        table.zombie_count(),
        1,
        "the killed child was never waited: its process-table entry \
         survives as a zombie"
    );
}

#[test]
fn every_unwaited_kill_accretes_one_more_entry() {
    let mut table = ProcTable::default();
    for i in 0..8 {
        let _ = abort_load(&mut table, 100 + i);
    }
    assert_eq!(
        table.zombie_count(),
        8,
        "a long-lived parent that repeats the miss accretes one zombie \
         per kill-without-wait"
    );
}

#[test]
fn a_wait_after_the_fact_still_reaps() {
    // The remedy exists even post-hoc: any later wait on the same pid
    // reaps the zombie. (Rust's Child::try_wait() from the same handle
    // does this; the miss is paths where the handle is gone.)
    let mut table = ProcTable::default();
    let mut child = Child::spawn(&mut table, 7);
    child.kill();
    assert_eq!(child.zombies(), 1);
    child.wait();
    assert_eq!(child.zombies(), 0, "the wait reaps");
}

impl<'a> Child<'a> {
    fn zombies(&self) -> usize {
        self.table.zombie_count()
    }
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `kill-without-wait-zombie`
// (riir-refine rust_perf corpus, Batch 316) -- the REMEDY arm.
//
// The same process table; every kill is paired with a wait, and the
// guard's Drop carries the belt-and-braces pair so error paths that skip
// the explicit close still reap (the source's own Drop comment names the
// shape: 'Belt and braces for the error paths that skip close()').

use std::collections::BTreeSet;

#[derive(Default)]
struct ProcTable {
    entries: BTreeSet<u64>,
}

impl ProcTable {
    fn spawn(&mut self, pid: u64) {
        self.entries.insert(pid);
    }
    fn zombie_count(&self) -> usize {
        self.entries.len()
    }
    fn reap(&mut self, pid: u64) {
        self.entries.remove(&pid);
    }
}

struct Child<'a> {
    pid: u64,
    signaled: bool,
    exited: bool,
    table: &'a mut ProcTable,
}

impl<'a> Child<'a> {
    fn spawn(table: &'a mut ProcTable, pid: u64) -> Self {
        table.spawn(pid);
        Self { pid, signaled: false, exited: false, table }
    }

    fn kill(&mut self) {
        self.signaled = true;
        self.exited = true;
    }

    /// kill + wait: the PAIRED stop. The signal ends the child; the wait
    /// reaps it. A wedged child must not outlive us -- and a reaped one
    /// must not haunt the table.
    fn kill_and_wait(&mut self) {
        if !self.exited {
            self.kill();
        }
        self.wait();
    }

    fn wait(&mut self) {
        self.table.reap(self.pid);
    }
}

/// The guard: a running child held for a scope. Drop is the belt and
/// braces -- error paths that skip close() still kill AND wait.
struct ChildGuard<'a> {
    child: Option<Child<'a>>,
}

impl<'a> ChildGuard<'a> {
    fn spawn(table: &'a mut ProcTable, pid: u64) -> Self {
        Self { child: Some(Child::spawn(table, pid)) }
    }

    fn fail_mid_load(&mut self) -> Result<(), String> {
        // The error path: return WITHOUT touching the child. The Drop
        // below does the paired stop.
        Err("daemon exited during load without signalling READY".into())
    }
}

impl<'a> Drop for ChildGuard<'a> {
    fn drop(&mut self) {
        if let Some(c) = self.child.as_mut() {
            c.kill_and_wait();
        }
    }
}

#[test]
fn the_error_path_still_reaps_through_the_drop_guard() {
    let mut table = ProcTable::default();
    {
        let mut g = ChildGuard::spawn(&mut table, 4242);
        let err = g.fail_mid_load();
        assert!(err.is_err());
        // No explicit stop here -- the guard's Drop is the stop.
    }
    assert_eq!(
        table.zombie_count(),
        0,
        "the belt-and-braces Drop reaps: no zombie survives the error path"
    );
}

#[test]
fn an_explicit_kill_and_wait_reaps_immediately() {
    let mut table = ProcTable::default();
    let mut c = Child::spawn(&mut table, 9);
    c.kill_and_wait();
    assert_eq!(table.zombie_count(), 0);
}

#[test]
fn double_stop_is_idempotent() {
    // kill_and_wait on an already-exited child skips the signal and
    // still waits: the pairing is safe to repeat.
    let mut table = ProcTable::default();
    let mut c = Child::spawn(&mut table, 11);
    c.kill_and_wait();
    c.kill_and_wait();
    assert_eq!(table.zombie_count(), 0);
}
"#.to_string(),
            context_notes: Some("Grade B- -- NotBenchable ON THIS BOX (the decision content is a reaping bookkeeping law -- which path reaps which child -- priced in a long-lived parent's process-table hygiene; a zombie costs a table entry and a pid, invisible to any microbench). SOURCE (pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f, Apache-2.0; quotes verified verbatim in the .raw clone HEAD-equal to the pin this session; inner double quotes rendered as single quotes per house string-literal safety). THE LAW -- kill and wait are PAIRED on every path, five sites in crates/browser/src/cdp.rs @ 02523aec: the launch-timeout path (kill + wait + the temp-profile removal), the channel-disconnected path (WAIT ONLY -- the process already exited; killing a reaped pid is an error, the shape knows which half it needs), the close path (kill + wait), the error-path guard, and the Drop, comment verbatim: 'Belt and braces for the error paths that skip close(). A browser this process did not launch is never touched.' -- whose body is the try_wait-first shape: 'if let Ok(None) = child.try_wait() { let _ = child.kill(); let _ = child.wait(); }' (try_wait FIRST: an already-exited child needs only the reap; a running one gets signal + reap). HOME FIRING DATUM (adjudicated this session): src/l4_daemon.rs:111 -- the EOF-during-load error path kills and returns without a wait ('let _ = child.kill(); return Err(...UnexpectedEof...)') -- one zombie until process exit on every failed daemon boot; the same file's other three sites (97-98 the destructure arm, 227-228 the Drop 'Belt and braces: a wedged child must not outlive us holding 7 GB') ARE paired -- the miss is one error path, exactly the class the source's guard exists for; domains/docker/buildcheck.rs:246-247 and seal-game-editor world.rs halt() are paired (adjudicated, no defect); riir-deployer verify.rs verified already-correct at verdict time. REGIME: long-lived parents that spawn, kill, and respawn children repeatedly (tool daemons, device managers, test harnesses); any kill() on an ERROR path (the happy path gets the wait via wait_with_output; the error path is where the pairing falls off). ANTI-REGIMES: short-lived parents (the exit reaps everything); parents that only ever wait_with_output (the pairing is built in); supervisors with a SIGCHLD reaper thread (a different mechanism, same law); code that deliberately defers reaping to a pool of children reaped in batch (the batch IS the wait). KINSHIP/DISTINCT-FROM (read + disclosed): child-piped-stderr-drain-for-process-lifetime (this batch) owns the I/O half of child hygiene -- THIS owns the EXIT half; watchdog-deadline-moves-on-command-progress (B270, entries_41) owns a progress deadline on commands, not the reaping; timed-zero-refuses-round-stop (B284) owns instrument zeros, unrelated. NOVEL: grep 'kill()|child.wait|try_wait' over src/ at the landing HEAD: paired sites + corpus entries about GPU barriers (vocabulary collision only) -- zero corpus rule owns the reap law. Suggestion-only (the defect is a MISSING call on one path; a parse-only proposer cannot see path coverage). FIXTURE PROVENANCE: original machinery (never impeccable code) -- a ProcTable stand-in (the kernel's table; only wait removes entries) + a Child whose kill marks exited, the buggy arm's abort_load error path (kill, return, drop -- the table keeps the entry; the accretion test shows 8 misses = 8 zombies; the post-hoc test proves a later wait still reaps), the fixed arm's kill_and_wait pairing + the ChildGuard Drop belt-and-braces (the error path that never touches the child still reaps through the guard; double-stop idempotent); both arms share the harness and differ only on the law. Cite: pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f -- crates/browser/src/cdp.rs (kill+wait at the launch timeout, the Disconnected arm's wait-only, close, and the Drop guard 'Belt and braces for the error paths that skip close()' with try_wait-first); home: src/l4_daemon.rs:111 (the unpaired kill, the batch's own firing datum); Batch 316 / Plan 316 / Research 356.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "connect-timeout-walk-over-resolved-addrs".to_string(),
            name: "TcpStream::connect((host, port)) resolves a host to EVERY address it names and then tries them in order under the ambient TCP timeout -- one dead or deaf address (a dropped SYN: no refusal, no answer) stalls the whole dial while every later, possibly healthy, address waits behind it; walk the resolved addresses yourself (to_socket_addrs), each with an explicit connect_timeout bounded to the remaining budget, and keep the LAST error so the caller still learns why the dial failed".to_string(),
            category: "io".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `connect-timeout-walk-over-resolved-addrs`
// (riir-refine rust_perf corpus, Batch 316) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's dial lane (its
// connect_tcp walk and the file_system probe are cited in
// context_notes). TcpStream::connect((host, port)) resolves a host to
// EVERY address it names and then tries them in order under the ambient
// TCP timeout -- often minutes or effectively unbounded. One dead or
// deaf address (a dropped SYN: no RST, no refusal) stalls the whole dial
// while every later -- possibly healthy -- address waits behind it.
//
// Modeled: an address list with per-address outcomes; a black-holed
// first address consumes the entire budget when the dial carries no
// per-address bound.

use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Probe {
    /// SYN dropped: no refusal, no answer -- the OS default timeout
    /// governs, which is to say forever at dial timescales.
    BlackHole,
    /// RST back instantly: cheap to skip, never the bottleneck.
    Refused,
    /// Connects.
    Answers,
}

/// fake_connect: consume the dial budget the way each outcome does.
fn fake_connect(outcome: Probe, budget: &mut Duration) -> Result<(), &'static str> {
    match outcome {
        Probe::Refused => {
            // instant RST: ~0 ms
        }
        Probe::BlackHole => {
            // No per-address bound: the attempt parks until the OS gives
            // up -- the whole remaining budget.
            *budget = Duration::ZERO;
        }
        Probe::Answers => {
            if budget.is_zero() {
                return Err("budget exhausted");
            }
            // a fast connect
            *budget = budget.saturating_sub(Duration::from_millis(5));
            return Ok(());
        }
    }
    Err(match outcome {
        Probe::Refused => "connection refused",
        _ => "timed out",
    })
}

/// The naive dial: `TcpStream::connect(("host", port))` -- resolved
/// addresses tried in order, each under the ambient (unbounded) timeout.
fn naive_dial(addrs: &[Probe], total: Duration) -> Result<(), &'static str> {
    let mut budget = total;
    let mut last = "no address";
    for &probe in addrs {
        match fake_connect(probe, &mut budget) {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
        if budget.is_zero() {
            return Err(last);
        }
    }
    Err(last)
}

#[test]
fn one_black_holed_first_address_burns_the_whole_dial() {
    let addrs = [Probe::BlackHole, Probe::Answers];
    let r = naive_dial(&addrs, Duration::from_millis(500));
    // The error identity tells the story: "timed out" is the BLACK
    // HOLE's error (address 0) -- the walk never reached the healthy
    // address at index 1, whose would-be error ("budget exhausted" on a
    // connect attempt) never surfaced.
    assert_eq!(
        r.unwrap_err(),
        "timed out",
        "the dial died on the black-holed FIRST address: the unbounded \
         attempt consumed the entire budget before index 1 was tried"
    );
}

#[test]
fn the_naive_shape_is_order_fragile_even_with_a_total_bound() {
    // Same addresses, healthy one FIRST: the dial succeeds cheaply. The
    // failure is the unbounded attempt, not the address set.
    let addrs = [Probe::Answers, Probe::BlackHole];
    assert!(naive_dial(&addrs, Duration::from_millis(500)).is_ok());
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `connect-timeout-walk-over-resolved-addrs`
// (riir-refine rust_perf corpus, Batch 316) -- the REMEDY arm.
//
// The same address set -- but the dial WALKS the resolved addresses
// itself: per-address connect_timeout with a bounded slice, keep the
// last error, and a black hole costs its slice, not the whole dial.
// (The source's walk: `for addr in host_port.to_socket_addrs()? { match
// TcpStream::connect_timeout(&addr, timeout) { Ok(s) => return Ok(s),
// Err(e) => last = e } }` -- the doc line is the rule: 'TcpStream::
// connect over every address host:port resolves to, each with a
// deadline.')

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Probe {
    BlackHole,
    Refused,
    Answers,
}

/// Per-address attempt with a BOUNDED slice: a black hole can eat its
/// slice and nothing more.
fn bounded_connect(
    outcome: Probe,
    slice: Duration,
    budget: &mut Duration,
) -> Result<(), &'static str> {
    let spent = slice.min(*budget);
    *budget = budget.saturating_sub(spent);
    match outcome {
        Probe::Refused => Err("connection refused"),
        Probe::BlackHole => Err("timed out"), // the slice, then move on
        Probe::Answers => {
            if spent < Duration::from_millis(5) {
                Err("budget exhausted")
            } else {
                Ok(())
            }
        }
    }
}

/// The walked dial: per-address timeout, last error kept.
fn walked_dial(addrs: &[Probe], total: Duration, per_addr: Duration) -> Result<(), &'static str> {
    let mut budget = total;
    let mut last = "no address";
    for &probe in addrs {
        match bounded_connect(probe, per_addr, &mut budget) {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
        if budget.is_zero() {
            break;
        }
    }
    Err(last)
}

#[test]
fn the_walk_reaches_the_healthy_address_behind_the_black_hole() {
    let addrs = [Probe::BlackHole, Probe::Answers];
    let t0 = Instant::now();
    let r = walked_dial(
        &addrs,
        Duration::from_millis(500),
        Duration::from_millis(150),
    );
    assert!(
        r.is_ok(),
        "the black hole cost its 150 ms slice; the healthy address at \
         index 1 connects inside the remaining budget"
    );
    assert!(
        t0.elapsed() < Duration::from_millis(500),
        "the dial did not burn its whole budget on the first address"
    );
}

#[test]
fn the_last_error_survives_the_walk() {
    // All addresses fail: the caller still gets an informative error --
    // the walk keeps the LAST one (the source's `last = e`).
    let addrs = [Probe::Refused, Probe::BlackHole];
    let r = walked_dial(
        &addrs,
        Duration::from_millis(500),
        Duration::from_millis(100),
    );
    assert_eq!(r.unwrap_err(), "timed out", "the last attempt's error");
}

#[test]
fn every_address_bounded_still_fails_fast_when_all_are_dead() {
    let addrs = [Probe::BlackHole, Probe::BlackHole, Probe::BlackHole];
    let t0 = Instant::now();
    let r = walked_dial(
        &addrs,
        Duration::from_millis(10_000),
        Duration::from_millis(100),
    );
    assert!(r.is_err());
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "three dead addresses cost three slices (~300 ms), never the OS \
         default timeout"
    );
}
"#.to_string(),
            context_notes: Some("Grade B- -- NotBenchable ON THIS BOX (the decision content is a dial-lane BUDGET law -- which address consumes which share of a dial's budget -- priced against network failures a fixture cannot summon on demand; source-reported behavior is PROVENANCE, never bars). SOURCE (pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f, Apache-2.0; quotes verified verbatim in the .raw clone HEAD-equal to the pin this session; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT -- bare TcpStream::connect: one resolved address that black-holes (SYN dropped, no RST) parks the dial under the ambient timeout (minutes at OS default) while every later address waits. THE LAW -- the source's own helper IS the rule, doc verbatim: 'TcpStream::connect over every address host:port resolves to, each with a deadline.' -- body verbatim: 'for addr in host_port.to_socket_addrs()? { match TcpStream::connect_timeout(&addr, timeout) { Ok(s) => return Ok(s), Err(e) => last = e } } Err(last)' -- the walk, the per-address bound, the LAST error kept. The same shape twice more in the repo: detect/file_system.rs tcp_connect (to_socket_addrs + connect_timeout per addr, probing whether a dev server port answers) and its http_get_once (per-addr connect_timeout bounded to the REQUEST's remaining deadline -- the walk and the budget compose). The error kept is the LAST one, so a refused fast-fail before a timeout still surfaces the timeout: the caller learns what actually blocked. HOME PRECEDENT (consumer, verified this session): src/seat_rerank.rs:331 walks to_socket_addrs with per-addr connect_timeout against a deadline budget -- the correct shape already lives at home; update_check/groq carry fixed CONNECT_TIMEOUTs through their HTTP clients (bounded at the client layer). REGIME: any non-loopback dial by host name -- multi-A-record hosts, IPv4/IPv6 dual stacks, VPN/flap-prone networks where one resolved path is dead; local tooling dialing a daemon by 'localhost:port' where stale /etc/hosts or IPv6-first resolution adds a dead first address. ANTI-REGIMES: loopback-only dials where the resolution is a single address (nothing to walk); HTTP clients whose connector already walks with a connect timeout (hyper/reqwest do); dials already bounded by a supervisor deadline at a coarser layer (compose: the walk needs its own per-address bound anyway -- the supervisor bound alone still stalls on the first address until IT fires). KINSHIP/DISTINCT-FROM (read + disclosed): the TCP_NODELAY request-response rule (B153) owns a socket OPTION on an established connection -- THIS owns the dial phase before any connection exists; per-read-timeout-vs-request-deadline (this batch) owns the post-connect read phase -- the two compose into a fully bounded exchange (the source's file_system probe carries both: connect_timeout per addr + read/write timeouts per request); tier-write-defers-to-urgent-reads (B270) owns priority among established IO, unrelated. NOVEL: grep 'connect_timeout|to_socket_addrs|TcpStream::connect' over src/ at the landing HEAD: consumers only (seat_rerank's correct walk, transport constants), zero corpus rule owns the walk law. Suggestion-only (the defect needs name-resolution multiplicity + a failure mode the parse cannot see). FIXTURE PROVENANCE: original machinery (never impeccable code) -- a Probe enum stand-in (BlackHole consumes the whole budget unbounded / its slice bounded; Refused is instant; Answers connects), the buggy arm's naive dial (the error IDENTITY proves the healthy address was never tried: 'timed out' is address 0's error, and the control test shows the same set succeeds in the healthy-first order -- the unbounded attempt is the defect, not the addresses), the fixed arm's walked dial (the black hole costs its slice and the healthy address at index 1 connects; the last error survives an all-fail walk; all-dead fails fast in three slices); both arms share the harness and differ only on the law. Cite: pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f -- crates/browser/src/cdp.rs connect_tcp (the doc line + the walk body) + crates/detect/src/file_system.rs (tcp_connect + http_get_once's remaining-deadline walk); home: src/seat_rerank.rs:331 (the consumer precedent); Batch 316 / Plan 316 / Research 356.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        PerfRule {
            rule_id: "arrival-order-turnstile-lane".to_string(),
            name: "a thread-per-connection server loses the single-threaded server's end-callback atomicity: two concurrent state-mutating requests reach the state lock in SOCKET-COMPLETION order, so a small-body request that finished fast overtakes an earlier, larger request that logically preceded it and reads a store the earlier one never wrote (the 404 unknown_session class); serialize state mutations in ARRIVAL order through one FIFO ticket lane -- the accept loop issues monotonically increasing tickets, each handler reads and parses its FULL body first (still parallel) and wait_turn()s only before touching state, release on handler finish / before parking (SSE, long polls) / on Drop -- and leave the wait unbounded ON PURPOSE (ordering must not give up): the lane-hold is bounded upstream by the request deadline, never by the lane".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `arrival-order-turnstile-lane`
// (riir-refine rust_perf corpus, Batch 316) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's thread-per-connection
// server (its Turnstile and the Node end-callback motivation are cited in
// context_notes). Without a lane, two concurrent state-mutating handlers
// reach the state lock in SOCKET-COMPLETION order: a small-body request
// that finished fast overtakes an earlier, larger request that logically
// preceded it -- the checkpoint then sees a store the earlier generate
// never wrote (the source's own 404 unknown_session failure).

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The shared store both mutations touch.
#[derive(Default)]
struct Store {
    session: Option<u64>,
}

/// The state lock, taken bare at mutation time: whoever gets there first
/// wins -- completion order, not arrival order.
#[derive(Default)]
struct Server {
    store: Mutex<Store>,
    /// handler event log, in completion order
    events: Mutex<Vec<String>>,
}

fn log(server: &Server, what: String) {
    server.events.lock().unwrap().push(what);
}

/// Handler A: arrived FIRST, big body -- finishes late.
fn generate(server: &Server, _session: u64, body_ms: u64) {
    std::thread::sleep(Duration::from_millis(body_ms)); // the big upload
    log(server, "generate body done (arrived first)".to_string());
    // THE DEFECT: the mutation runs whenever the lock frees -- the
    // later, small request may have already taken it.
    let mut st = server.store.lock().unwrap();
    st.session = Some(42);
    drop(st);
    log(server, "generate mutation done".to_string());
}

/// Handler B: arrived SECOND, tiny body -- finishes first, then mutates.
fn checkpoint(server: &Server, body_ms: u64) -> Option<u64> {
    std::thread::sleep(Duration::from_millis(body_ms));
    log(server, "checkpoint body done (arrived second)".to_string());
    let st = server.store.lock().unwrap();
    let observed = st.session; // reads whatever the store holds NOW
    drop(st);
    log(server, "checkpoint mutation done".to_string());
    observed
}

#[test]
fn the_fast_small_request_overtakes_the_earlier_big_one() {
    let server = Arc::new(Server::default());
    let s2 = server.clone();
    // A arrives first (100 ms body), B second (5 ms body). B starts
    // after A so arrival order is fixed.
    let a = std::thread::spawn({
        let s = server.clone();
        move || generate(&s, 42, 100)
    });
    std::thread::sleep(Duration::from_millis(10));
    let b = std::thread::spawn(move || checkpoint(&s2, 5));
    let observed = b.join().unwrap();
    a.join().unwrap();
    // THE DEFECT, pinned: B's mutation ran while the store was still
    // empty -- it observed None although its request arrived AFTER the
    // generate that creates the session it checkpoints.
    let events = server.events.lock().unwrap();
    let b_mut = events
        .iter()
        .position(|e| e == "checkpoint mutation done")
        .expect("checkpoint mutated");
    let a_mut = events
        .iter()
        .position(|e| e == "generate mutation done")
        .expect("generate mutated");
    assert!(
        b_mut < a_mut,
        "checkpoint's mutation ran BEFORE generate's: completion order \
         beat arrival order (observed session: {observed:?})"
    );
    assert_eq!(
        observed, None,
        "the checkpoint saw the session-less store the racing generate \
         never wrote: the 404 unknown_session class"
    );
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `arrival-order-turnstile-lane`
// (riir-refine rust_perf corpus, Batch 316) -- the REMEDY arm.
//
// The FIFO ticket lane: the accept loop issues tickets in arrival order;
// each handler reads/parses its body FIRST (still parallel), then
// wait_turn()s before touching server state, and releases when done --
// or on Drop, so a connection that dies without producing a request
// cannot wedge the lane. Ordering is deterministic no matter how the
// bodies interleave; the wait is unbounded ON PURPOSE (ordering must
// not give up) -- a stalled ticket is bounded upstream by the request
// deadline, not by the lane.

use std::collections::BTreeSet;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

pub struct Turnstile {
    inner: Mutex<TurnstileState>,
    cv: Condvar,
}

struct TurnstileState {
    next: u64,
    outstanding: BTreeSet<u64>,
}

impl Default for Turnstile {
    fn default() -> Self {
        Self {
            inner: Mutex::new(TurnstileState { next: 1, outstanding: BTreeSet::new() }),
            cv: Condvar::new(),
        }
    }
}

impl Turnstile {
    /// Test census: tickets issued but not yet released (survives a
    /// poisoned mutex through the same recovery path).
    pub fn outstanding_len(&self) -> usize {
        self.state().outstanding.len()
    }

    /// Poison recovery: the state is a plain counter + id set --
    /// recoverable, so a panic in one handler must not wedge the lane.
    fn state(&self) -> MutexGuard<'_, TurnstileState> {
        match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    /// The accept loop, in accept order.
    pub fn issue(self: &Arc<Self>) -> Ticket {
        let mut st = self.state();
        let n = st.next;
        st.next += 1;
        st.outstanding.insert(n);
        Ticket { turnstile: self.clone(), n, released: false }
    }
}

pub struct Ticket {
    turnstile: Arc<Turnstile>,
    n: u64,
    released: bool,
}

impl Ticket {
    /// Block until every EARLIER outstanding ticket is released. No
    /// timeout on purpose: ordering never gives up.
    pub fn wait_turn(&self) {
        let mut st = self.turnstile.state();
        loop {
            match st.outstanding.iter().next() {
                Some(&first) if first < self.n => {}
                _ => return,
            }
            st = match self.turnstile.cv.wait(st) {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
        }
    }

    pub fn release(mut self) {
        self.release_impl();
    }

    fn release_impl(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        let mut st = self.turnstile.state();
        if st.outstanding.remove(&self.n) {
            self.turnstile.cv.notify_all();
        }
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        self.release_impl();
    }
}

// ── the server shape ─────────────────────────────────────────────────

#[derive(Default)]
struct Store {
    session: Option<u64>,
}

struct Server {
    turnstile: Arc<Turnstile>,
    store: Mutex<Store>,
    events: Mutex<Vec<String>>,
}

fn generate(server: &Arc<Server>, ticket: Ticket, session: u64, body_ms: u64) {
    std::thread::sleep(Duration::from_millis(body_ms)); // the big upload
    server.events.lock().unwrap().push(format!("generate body done"));
    ticket.wait_turn(); // my turn: every earlier ticket released
    let mut st = server.store.lock().unwrap();
    st.session = Some(session);
    drop(st);
    server.events.lock().unwrap().push(format!("generate mutation done"));
}

fn checkpoint(server: &Arc<Server>, ticket: Ticket, body_ms: u64) -> Option<u64> {
    std::thread::sleep(Duration::from_millis(body_ms));
    server.events.lock().unwrap().push(format!("checkpoint body done"));
    ticket.wait_turn();
    let st = server.store.lock().unwrap();
    let observed = st.session;
    drop(st);
    server.events.lock().unwrap().push(format!("checkpoint mutation done"));
    observed
}

#[test]
fn the_lane_orders_mutations_by_arrival_not_by_body_completion() {
    let turnstile = Arc::new(Turnstile::default());
    let server = Arc::new(Server {
        turnstile: turnstile.clone(),
        store: Mutex::new(Store::default()),
        events: Mutex::new(Vec::new()),
    });
    // Accept order: A's ticket FIRST, B's second.
    let t_a = turnstile.issue();
    let t_b = turnstile.issue();
    let s = server.clone();
    let a = std::thread::spawn(move || generate(&s, t_a, 42, 100));
    std::thread::sleep(Duration::from_millis(10));
    let s2 = server.clone();
    let observed = std::thread::spawn(move || checkpoint(&s2, t_b, 5))
        .join()
        .unwrap();
    a.join().unwrap();
    // The same fast-small/slow-big bodies that raced the naive arm now
    // order correctly: B saw the session A wrote.
    assert_eq!(
        observed,
        Some(42),
        "the checkpoint observed the session its earlier generate wrote: \
         body-completion speed no longer reorders state"
    );
    let events = server.events.lock().unwrap();
    let a_mut = events
        .iter()
        .position(|e| e == "generate mutation done")
        .expect("generate mutated");
    let b_mut = events
        .iter()
        .position(|e| e == "checkpoint mutation done")
        .expect("checkpoint mutated");
    assert!(
        a_mut < b_mut,
        "the earlier arrival mutated first, regardless of body size"
    );
}

#[test]
fn a_dropped_ticket_releases_and_never_wedges_the_lane() {
    let turnstile = Arc::new(Turnstile::default());
    let dead = turnstile.issue(); // accepted, then dies before any body
    drop(dead); // the Drop release
    let later = turnstile.issue();
    let t0 = std::time::Instant::now();
    later.wait_turn(); // must NOT block on the dead ticket
    assert!(
        t0.elapsed() < Duration::from_millis(50),
        "a dropped ticket releases: later arrivals never wait behind it"
    );
    later.release();
}

#[test]
fn a_poisoned_lane_state_recovers_instead_of_wedging() {
    let turnstile = Arc::new(Turnstile::default());
    let t1 = turnstile.issue();
    // One handler panics while holding the lane state.
    let clone = turnstile.clone();
    let _ = std::thread::spawn(move || {
        let _g = clone.inner.lock().unwrap();
        panic!("handler exploded mid-request");
    })
    .join();
    // The poisoned mutex must not wedge the lane: the next issue
    // recovers the plain counter state through into_inner.
    let t2 = turnstile.issue();
    assert_eq!(
        turnstile.outstanding_len(),
        2,
        "the outstanding set survived the poison: the recovered state is \
         the true lane state, not a fresh one"
    );
    drop(t1); // the earlier ticket releases through the same recovery
    let t0 = std::time::Instant::now();
    t2.wait_turn(); // t1 gone: returns immediately
    assert!(
        t0.elapsed() < std::time::Duration::from_millis(50),
        "ordering keeps working across the poisoned handler"
    );
    t2.release();
}
"#.to_string(),
            context_notes: Some("Grade A- -- NotBenchable ON THIS BOX (the decision content is a request-lane LIVENESS law -- which timeout bounds which unit of work, and which platform needs which unblock mechanism -- priced in a server's behavior under an adversarial peer, not in any A/B microbench; a stand-in benchmark cannot measure a lane-hold at all; source-reported behavior is PROVENANCE, never bars). SOURCE (pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f, Apache-2.0; every quote verified verbatim in the .raw clone HEAD-equal to the pin this session; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT -- the source names it in the set_header_timeout doc, verbatim: 'Set a per-read timeout on the socket. This is SO_RCVTIMEO: it bounds a single blocked read, not the whole connection, and any byte received resets it. It is a coarse backstop only; the total lane-hold bound is read_request_deadline, not this.' and the read_request_deadline doc carries the failure shape verbatim: 'any byte inside the window resets it, so a slow-drip or silent peer keeps each read returning Ok(1) (or blocking under the window) and read_request never returns, pinning the connection's ticket and wedging every later mutation behind it.' THE LAW, three parts (crates/common/src/http.rs @ 02523aec, read_request_deadline): (1) THE SOCKET HALF -- set the read timeout to the FULL deadline, comment verbatim: 'Bound the read at the socket as well as at the watchdog. Windows does not unblock a recv already parked in the kernel when another thread calls shutdown on the same socket, so there the watchdog alone cannot enforce the deadline and a silent connection held its ticket for the whole 60s header timeout. A read that makes no progress for the deadline now returns on every platform and the handler drops its ticket; the watchdog stays as the backstop for a connection that keeps trickling bytes without ever completing a request.' -- the PLATFORM-SPLIT decision content: the socket timeout is the layer that works everywhere; the watchdog is the layer that also catches trickle; (2) THE WATCHDOG HALF -- a thread polling a done flag at a 50 ms step, comment verbatim: 'Poll the done flag so a fast, normal read frees the watchdog promptly instead of pinning a thread for the whole deadline.' -- at the deadline it calls shutdown(Both): 'Unblock any in-progress read so read_request returns; the handler then drops its ticket and the connection closes.'; (3) THE RESTORE -- the prior timeout is read before ('let prev_timeout = stream.read_timeout().ok().flatten();') and restored after ('let _ = stream.set_read_timeout(prev_timeout);') so the deadline never leaks into the connection's later life. The budget is a const, verbatim: 'pub const READ_REQUEST_DEADLINE: Duration = Duration::from_secs(10);' -- '10s is far more than any legitimate local upload needs'. The source's race test feeds ~4 KB over ~500 ms -- 'finishes in a twentieth of it'. PRODUCT COVERAGE AT HOME (verified firing this session, issues filed): riir-instinct src/serve_edge.rs (the /decide read loop sets a per-read timeout with no total deadline and no watchdog) + riir-reflex src/serve.rs (same shape) -- the exact pre-fix posture. REGIME: any hand-rolled request reader over accepted sockets (thread-per-connection or small-pool servers); any lane whose admission order is a resource (tickets, semaphores, per-connection buffers); any read loop where a peer controls the byte arrival. ANTI-REGIMES: libraries whose request reading already carries a total deadline; single-shot reads with no lane or resource behind them (a wasted thread is the whole cost); fully-async IO where the timeout is a select/future race rather than a socket attribute (the same LAW, a different mechanism); protocols with their own idle/keepalive framing that already bounds the exchange. SEC CROSS-ROUTE (recorded per the B293 cross-domain precedent): the slow-drip shape IS slowloris-class pre-auth connection exhaustion -- a future sec batch must route here, never re-mine it; the fix-space is hand-rolled std::net request reading, not a sec-domain (tower-http/jwt/cookie) shape. KINSHIP/DISTINCT-FROM (read + disclosed): the corpus's io category owns socket-option and read-shape rules (the TCP_NODELAY request-response rule, the nonblocking head-probe open, the vectored-write capability flag) -- none owns a TIMEOUT-SEMANTICS law (per-read vs per-request); watchdog-deadline-moves-on-command-progress (B270, entries_41) owns a COMMAND-progress deadline in a serving control plane -- the progress-moves-the-clock shape, not a socket-read shape; timed-zero-refuses-round-stop (B284, entries_50) owns the measurement-instrument zero (an instrument failure, not a lane bound); the source itself composes the two: this rule bounds the lane-hold that arrival-order-turnstile-lane (this batch) leaves unbounded on purpose. NOVEL: grep 'read_timeout|SO_RCVTIMEO|request.*deadline|watchdog' over src/ at the landing HEAD (03e6ca67 lineage): one consumer (src/seat_rerank.rs set_read_timeout(Some(remaining)) -- a deadline-TRACKED read, the correct shape already), the update_check/groq transport constants are CONNECT-timeout consumers -- zero corpus rule owns the per-read-vs-request-deadline law. Suggestion-only (the defect is invisible to a parse-only proposer: it needs the loop shape AND what the peer controls). FIXTURE PROVENANCE: original machinery (never impeccable code) -- a DripPeer Read stand-in (one byte per call, then silence, never EOF), the buggy arm's per-read-only loop (the drip answers every read, so no per-read bound ever fires; the test proves the reader is still pinned 50 ms past where a 100 ms total deadline would have fired), the fixed arm's deadline loop + 25 ms-poll watchdog (the shutdown(Both) half modeled by a shutdown flag + Thread::unpark; the socket half modeled by the peer's own window returning the read -- the Windows-case test proves the socket layer alone catches a silent peer) + the honest-peer arm (a complete request still completes); both arms share the harness and differ only on the law. Cite: pbakaus/impeccable @ 02523aec3e4c4580102d996d1b6a71fbe062e03f -- crates/common/src/http.rs (set_header_timeout doc, READ_REQUEST_DEADLINE const + doc, read_request_deadline body: the prev_timeout capture, the 50 ms watchdog step, the shutdown(Both) comment); Batch 316 / Plan 316 / Research 356.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
