//! Plan 202 R3a: `SeatRerankClient` — the RERANK half of the escalation
//! chain (the candidates-exist arm). refine's L0–L3 candidates become
//! the `options`; the hosted expert (the instinct `Ext` seat) picks one
//! or abstains; an abstain means THE MODELLESS ANSWER STANDS. Generation
//! stays local and modelless — this client never speaks generate
//! traffic, and generate transports (`HostedExpertL4`, `DaemonL4`)
//! never touch the seat: two arms, two transports, one escalation chain.
//!
//! ## The wire
//!
//! The instinct serve edge's `/decide` envelope: `{suite, state, options}`
//! in, `{options, pick, pick_index, abstained, …}` out. Std-only HTTP/1.1
//! (the `riir-agents::decision_client` shape — this is a decision-plane
//! edge, not an LLM leg, so `riir-llm` stays out of the tree; the body is
//! hand-serialized with ONE writer, `serde_json` parses on the reply side
//! only).
//!
//! ## The contract (every violation declines — never a forced bad pick)
//!
//! 1. **The options are EXACTLY the candidates** — the body carries the
//!    candidate list verbatim, in order; nothing else of the host.
//! 2. **The echo is the trust anchor**: the reply's `options` must equal
//!    the sent candidates element-wise BEFORE any pick is read. An index
//!    into a different presentation is a different answer.
//! 3. **L4 invariant 3** — a pick can never replace a parsing candidate
//!    with a non-parsing one: the answer's candidate is taken from the
//!    CALLER's list by index (`candidates[i]`), and a reply whose `pick`
//!    field names anything else declines `Contract` (the responder's own
//!    answer disagrees with itself — untrustworthy tier).
//! 4. **Abstention is first-class**: `abstained` (or a null pick) is
//!    `Ok(SeatAnswer::Abstained)` — the caller keeps the modelless fix.
//! 5. **Consent + deadline pre-socket**, the shared
//!    [`crate::code_egress`] scope: unset refuses before any socket, an
//!    expired deadline declines before any socket, and a late reply that
//!    arrives complete is DROPPED (compute-once — the modelless answer
//!    is already in hand; that is why candidates existed).
//!
//! ## Authentication
//!
//! The instinct serve edge is loopback-bound by deployment (closed CORS,
//! no auth header today); the client sends no Authorization header. An
//! `https://` endpoint is a pre-socket config refusal — this client
//! speaks plain HTTP; TLS belongs to a terminating proxy in front of
//! the seat, never here.

use crate::code_egress::{deadline_expired, EgressConsent, EgressRefusal};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// The connect-phase cap (the `decision_client` rule): a dead lane must
/// be RECORDED as one, fast — never eat the caller's whole budget in a
/// blackholed connect.
const CONNECT_BUDGET_CAP: Duration = Duration::from_millis(250);
const CONNECT_BUDGET_FLOOR: Duration = Duration::from_millis(10);

/// What the seat answered. The pick carries the CALLER's candidate (by
/// index) — never the wire's copy (L4 invariant 3). Both variants carry
/// the seat's receipt when the reply had one (additive on the wire — a
/// seat without receipts still answers; Bench 107 finding 4: the heal
/// side's audit trail was blind to the seat-side evidence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeatAnswer {
    /// The expert picked `candidates[index]`; `candidate` is that
    /// caller-owned string, copied out for the caller's convenience.
    Pick {
        index: usize,
        candidate: String,
        receipt: Option<SeatReceipt>,
    },
    /// The expert abstained — the modelless answer stands.
    Abstained { receipt: Option<SeatReceipt> },
}

/// The seat's decision receipt, parsed LENIENTLY: every field optional
/// (the receipt is additive on the wire; the CONTRACT fields — options
/// echo, pick, abstained — are validated in [`SeatRerankClient::parse_reply`],
/// never here). Surfaced for the heal-side audit trail (the R7 freeze).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatReceipt {
    /// The seat's build fingerprint (`receipt.build`).
    pub build: Option<String>,
    /// `receipt.input_blake3` — the BLAKE3 of the request state.
    pub input_blake3: Option<String>,
    /// `receipt.decision_blake3` — the BLAKE3 of the canonical decision.
    pub decision_blake3: Option<String>,
    /// The seat's own decision time in µs (`us`).
    pub us: Option<u64>,
    /// The seat's confidence (`confidence`) — specialist-scaled, not
    /// comparable to the modelless scorer's.
    pub confidence: Option<String>,
    /// The lane that answered (`arm`).
    pub arm: Option<String>,
}

impl SeatReceipt {
    /// Lenient parse off the reply envelope. `confidence` is kept as its
    /// raw spelling (the edge emits a float; string form keeps `Eq` + a
    /// stable audit copy — this is an audit record, not a score input).
    fn parse(v: &serde_json::Value) -> Option<Self> {
        let r = &v["receipt"];
        let get = |k: &str| r[k].as_str().map(str::to_string);
        let parsed = Self {
            build: get("build"),
            input_blake3: get("input_blake3"),
            decision_blake3: get("decision_blake3"),
            us: v["us"].as_u64(),
            // The edge emits a JSON number; the raw spelling keeps Eq
            // and a stable audit copy — this is an audit record, not a
            // score input.
            confidence: v["confidence"]
                .as_str()
                .map(str::to_string)
                .or_else(|| v["confidence"].as_f64().map(|c| c.to_string())),
            arm: v["arm"].as_str().map(str::to_string),
        };
        // An envelope with NO receipt and NO timing is a seat older than
        // the receipt tier — `None`, not an empty record.
        let empty = Self::default();
        if parsed == empty {
            None
        } else {
            Some(parsed)
        }
    }
}

/// Why [`SeatRerankClient::rerank`] declined. A decline is ALWAYS safe:
/// the caller keeps the modelless answer it already has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeatDecline {
    /// The code-egress scope did not consent — no socket was opened.
    Egress(EgressRefusal),
    /// The candidates were empty — a rerank with nothing to rank is a
    /// generate-shaped miss; it is refused HERE and never reaches the
    /// seat (the generate arm owns no-candidate traffic).
    NoCandidates,
    /// The deadline had already expired — nothing was sent.
    DeadlineExpired,
    /// The seat's answer violates the rerank contract: echo mismatch,
    /// out-of-range pick, a `pick` field naming a non-candidate, or a
    /// shape the parser cannot trust. The message names the violation.
    Contract(String),
    /// Transport/HTTP failure (connect/write/read/status). Carries the
    /// short diagnostic (the `decision_client` rule), never a whole body.
    Wire(String),
}

impl std::fmt::Display for SeatDecline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Egress(r) => write!(f, "code-egress refused: {r}"),
            Self::NoCandidates => write!(
                f,
                "no candidates to rank — a rerank needs the L0–L3 candidates; \
                 no-candidate misses belong to the generate arm (R2), never the seat"
            ),
            Self::DeadlineExpired => {
                write!(f, "deadline expired — the modelless answer stands")
            }
            Self::Contract(m) => write!(f, "seat contract violation: {m}"),
            Self::Wire(m) => write!(f, "seat wire: {m}"),
        }
    }
}

/// The seat-rerank client: one hosted expert seat (endpoint + suite from
/// the R1 manifest row's data at the R6 wiring — construction args, never
/// a default). `rerank` is `&self`: the client holds no mutable state
/// (the slot lock is the seat's side).
pub struct SeatRerankClient {
    /// The connect addr (`host:port`) — scheme-stripped from the
    /// endpoint the manifest names (see [`Self::with_endpoint`]).
    addr: String,
    /// The original endpoint spelling (the `Host:` header value).
    host: String,
    /// The hosted suite the manifest row names — the expert selection
    /// key on the seat's side (the manifest is the only selection
    /// surface; there is no suite fallback here either).
    suite: String,
}

impl std::fmt::Debug for SeatRerankClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No endpoint (embeds the host), no suite — the CfClient rule:
        // a debug log never carries where the code goes.
        f.debug_struct("SeatRerankClient").finish_non_exhaustive()
    }
}

impl Clone for SeatRerankClient {
    fn clone(&self) -> Self {
        Self {
            addr: self.addr.clone(),
            host: self.host.clone(),
            suite: self.suite.clone(),
        }
    }
}

impl SeatRerankClient {
    /// Build from the R1 manifest row's pieces. The endpoint accepts
    /// `http://host:port` or bare `host:port`; an `https://` endpoint is
    /// a pre-socket [`SeatDecline::Contract`] refusal (see the module
    /// doc — TLS belongs to a proxy, not this client).
    ///
    /// Construction opens NO socket and reads NO env — a client is
    /// inert; only `rerank` egresses.
    ///
    /// # Errors
    ///
    /// [`SeatDecline::Contract`] for an `https://` endpoint (pre-socket,
    /// loud — never a runtime surprise mid-heal).
    pub fn new(endpoint: &str, suite: &str) -> Result<Self, SeatDecline> {
        if endpoint.starts_with("https://") {
            return Err(SeatDecline::Contract(format!(
                "endpoint {endpoint:?} is https — the seat client speaks plain HTTP; \
                 terminate TLS at a proxy in front of the seat"
            )));
        }
        let addr = endpoint
            .strip_prefix("http://")
            .unwrap_or(endpoint)
            .trim_end_matches('/')
            .to_string();
        if addr.is_empty() {
            return Err(SeatDecline::Contract("empty endpoint".to_string()));
        }
        Ok(Self {
            addr,
            host: endpoint.trim_end_matches('/').to_string(),
            suite: suite.to_string(),
        })
    }

    /// The request body for one rerank — the SNIPPET-AND-CANDIDATES-ONLY
    /// builder. The signature takes the snippet and the candidate list
    /// and nothing else: there is no path from a file handle to this
    /// function. Hand-serialized (one writer — the `decision_client`
    /// rule; serde lives on the parse side only).
    ///
    /// # Errors
    ///
    /// [`SeatDecline::NoCandidates`] — an empty candidate list is a
    /// generate-shaped miss and never becomes seat traffic.
    pub fn build_body(&self, snippet: &str, candidates: &[String]) -> Result<String, SeatDecline> {
        if candidates.is_empty() {
            return Err(SeatDecline::NoCandidates);
        }
        let cap = snippet.len() + 64 + candidates.iter().map(|c| c.len() + 8).sum::<usize>();
        let mut out = String::with_capacity(cap);
        out.push_str("{\"suite\":\"");
        json_escape_into(&self.suite, &mut out);
        out.push_str("\",\"state\":\"");
        json_escape_into(snippet, &mut out);
        out.push_str("\",\"options\":[");
        for (i, c) in candidates.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            json_escape_into(c, &mut out);
            out.push('"');
        }
        out.push_str("]}");
        Ok(out)
    }

    /// The full call: consent → candidates → deadline → wire → contract.
    /// The deadline is REQUIRED — the rerank arm always runs under a
    /// budget (the modelless answer is already in hand; an unbounded
    /// hosted call has no posture in the escalation chain).
    ///
    /// # Errors
    ///
    /// Every [`SeatDecline`] class; never panics on malformed remote
    /// output (the conservative contract: a decline keeps the modelless
    /// answer; a panic would take down a heal that had one).
    pub fn rerank(
        &self,
        snippet: &str,
        candidates: &[String],
        deadline: Instant,
    ) -> Result<SeatAnswer, SeatDecline> {
        self.rerank_checked_with(snippet, candidates, deadline, EgressConsent::from_env())
    }

    /// The consent-injectable core of [`Self::rerank`] — the form tests
    /// drive directly, so no test ever mutates the process env (the
    /// `is_armed_with` seam pattern; parallel tests never race a global).
    ///
    /// # Errors
    ///
    /// Same taxonomy as [`Self::rerank`].
    pub fn rerank_checked_with(
        &self,
        snippet: &str,
        candidates: &[String],
        deadline: Instant,
        consent: EgressConsent,
    ) -> Result<SeatAnswer, SeatDecline> {
        // (1) consent — BEFORE anything else; refusal opens no socket.
        if let EgressConsent::Refused(r) = consent {
            return Err(SeatDecline::Egress(r));
        }
        // (2) the candidates — an empty list never reaches the seat (the
        // generate arm owns no-candidate traffic; structurally, this
        // client cannot be talked into speaking it).
        if candidates.is_empty() {
            return Err(SeatDecline::NoCandidates);
        }
        let body = self.build_body(snippet, candidates)?;
        self.rerank_body_checked_with(&body, candidates, &[], deadline, consent)
    }

    /// The pre-built-body core (plan 326 T5's header seam): the
    /// escalation dispatch builds the body ONCE so the drain's
    /// attestation signs the EXACT bytes on the wire, then routes
    /// through here with the composed headers. `headers` carries each
    /// name EXACTLY once — the `kat_ua` law (a duplicate header name is
    /// a silent comma-join and a 403 on conforming edges); this client
    /// writes each entry verbatim, never a second setter on a name
    /// already present.
    ///
    /// # Errors
    ///
    /// Same taxonomy as [`Self::rerank`].
    pub fn rerank_body_checked_with(
        &self,
        body: &str,
        candidates: &[String],
        headers: &[(&str, &str)],
        deadline: Instant,
        consent: EgressConsent,
    ) -> Result<SeatAnswer, SeatDecline> {
        // The same pre-socket guards as [`Self::rerank_checked_with`] —
        // both public entries enforce the full contract.
        if let EgressConsent::Refused(r) = consent {
            return Err(SeatDecline::Egress(r));
        }
        if candidates.is_empty() {
            return Err(SeatDecline::NoCandidates);
        }
        // (3) deadline — an expired budget declines pre-socket.
        if deadline_expired(deadline) {
            return Err(SeatDecline::DeadlineExpired);
        }
        let raw = self.post(body, headers, deadline)?;
        Self::parse_reply(&raw, candidates, deadline)
    }

    /// The HTTP/1.1 exchange, deadline-phased (the `decision_client`
    /// shape): a capped connect, write under the remaining budget, read
    /// to EOF under the remaining budget. `Connection: close` — the edge
    /// answers one request per connection. `headers` are written each
    /// EXACTLY once between Content-Length and Connection (the
    /// `kat_ua` law — see [`Self::rerank_body_checked_with`]).
    fn post(
        &self,
        body: &str,
        headers: &[(&str, &str)],
        deadline: Instant,
    ) -> Result<String, SeatDecline> {
        let expired = || SeatDecline::DeadlineExpired;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let connect_budget = remaining.min(CONNECT_BUDGET_CAP).max(CONNECT_BUDGET_FLOOR);
        let stream = match self.addr.to_socket_addrs() {
            Ok(mut it) => match it.next() {
                Some(a) => TcpStream::connect_timeout(&a, connect_budget),
                None => {
                    return Err(SeatDecline::Wire(format!(
                        "endpoint {} did not resolve",
                        self.addr
                    )))
                }
            },
            Err(e) => return Err(SeatDecline::Wire(format!("resolve: {e}"))),
        };
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => return Err(SeatDecline::Wire(format!("connect: {e}"))),
        };
        let mut request = format!(
            "POST /decide HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\n",
            self.host,
            body.len()
        );
        // Extra headers (the drain attestation when composed) — each
        // name written EXACTLY once, verbatim; the values are
        // client-composed hex so no CRLF can ride in one.
        for (name, value) in headers {
            request.push_str(name);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }
        request.push_str("Connection: close\r\n\r\n");
        request.push_str(body);
        if deadline_expired(deadline) {
            return Err(expired());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let _ = stream.set_write_timeout(Some(remaining));
        if let Err(e) = stream.write_all(request.as_bytes()) {
            return Err(SeatDecline::Wire(format!("write: {e}")));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let _ = stream.set_read_timeout(Some(remaining));
        let mut raw = String::new();
        if let Err(e) = stream.read_to_string(&mut raw) {
            return Err(SeatDecline::Wire(format!("read: {e}")));
        }
        // The total-deadline gate: a SLOW-but-complete reply is dropped,
        // never applied (compute-once — the modelless answer stands).
        if deadline_expired(deadline) {
            return Err(expired());
        }
        Ok(raw)
    }

    /// Parse the seat's reply against the SENT candidates — the contract
    /// validation is the answer's trust anchor, spelled once:
    /// status → body → ECHO → abstain/pick → range → pick-field.
    fn parse_reply(
        raw: &str,
        candidates: &[String],
        deadline: Instant,
    ) -> Result<SeatAnswer, SeatDecline> {
        // The status line: "HTTP/1.1 200 OK".
        let status_ok = raw
            .split_whitespace()
            .nth(1)
            .is_some_and(|code| code == "200");
        if !status_ok {
            // The edge's error body carries a machine-readable `code` —
            // surface IT, never the whole body.
            let code = raw
                .split_once("\r\n\r\n")
                .and_then(|(_, body)| serde_json::from_str::<serde_json::Value>(body).ok())
                .and_then(|v| v["code"].as_str().map(str::to_string))
                .unwrap_or_else(|| "http_error".into());
            return Err(SeatDecline::Wire(code));
        }
        let Some((_, body)) = raw.split_once("\r\n\r\n") else {
            return Err(SeatDecline::Wire("no body".into()));
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
            return Err(SeatDecline::Wire("bad body".into()));
        };
        // The echo is the trust anchor: the reply's options must equal
        // the sent candidates element-wise BEFORE any pick is read. An
        // index into a different presentation is a different answer.
        let echo: Option<Vec<&str>> = v["options"]
            .as_array()
            .map(|a| a.iter().filter_map(|o| o.as_str()).collect());
        let echo_ok = echo.as_deref().is_some_and(|e| {
            e.len() == candidates.len()
                && e.iter()
                    .zip(candidates.iter())
                    .all(|(w, c)| *w == c.as_str())
        });
        if !echo_ok {
            return Err(SeatDecline::Contract(format!(
                "options echo mismatch — sent {} candidate(s) in a fixed order; the seat \
                 answered a different presentation, so its index space is not ours",
                candidates.len()
            )));
        }
        if Instant::now() >= deadline {
            return Err(SeatDecline::DeadlineExpired);
        }
        // Abstention first-class: the modelless answer stands.
        if v["abstained"].as_bool() == Some(true) {
            return Ok(SeatAnswer::Abstained {
                receipt: SeatReceipt::parse(&v),
            });
        }
        let Some(idx_u64) = v["pick_index"].as_u64() else {
            return Err(SeatDecline::Contract(
                "neither abstained nor pick_index — untrustworthy answer shape".into(),
            ));
        };
        let idx = usize::try_from(idx_u64).map_err(|_| {
            SeatDecline::Contract(format!("pick_index {idx_u64} does not fit usize"))
        })?;
        if idx >= candidates.len() {
            return Err(SeatDecline::Contract(format!(
                "pick_index {idx} out of range for {} candidate(s)",
                candidates.len()
            )));
        }
        // L4 invariant 3: the pick names a candidate WE sent, or the
        // answer is refused — a responder whose own pick field disagrees
        // with its index is not a tier to trust with code.
        if let Some(p) = v["pick"].as_str() {
            if p != candidates[idx] {
                return Err(SeatDecline::Contract(
                    "pick field names a non-candidate — the seat's answer disagrees with \
                     its own index (a parsing candidate can never be replaced this way)"
                        .to_string(),
                ));
            }
        }
        Ok(SeatAnswer::Pick {
            index: idx,
            candidate: candidates[idx].clone(),
            receipt: SeatReceipt::parse(&v),
        })
    }
}

/// JSON string escape, into the caller's buffer (no intermediate
/// allocations; control bytes take the `\u00XX` form). The
/// `decision_client`'s escape, byte-identical — one spelling of the
/// wire's writer across the stack.
fn json_escape_into(s: &str, out: &mut String) {
    use std::fmt::Write as _;
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc::{channel, Receiver};

    /// A loopback one-shot seat server: accepts ONE connection, records
    /// the request bytes, answers `status`/`body` over HTTP. The
    /// receiver lets a test assert on exactly what left the box (the
    /// R2 `fake_expert` pattern).
    fn fake_seat(status: &str, body: &str) -> (String, Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback bind");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = channel();
        let owned_status = status.to_string();
        let owned_body = body.to_string();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                // Read the WHOLE request: loop until the headers end and
                // the announced Content-Length has arrived (the R2 rule —
                // one read can return headers-only).
                let mut buf = Vec::with_capacity(65536);
                let mut chunk = [0u8; 8192];
                loop {
                    let n = sock.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(hend) = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
                    {
                        let head = String::from_utf8_lossy(&buf[..hend]).to_ascii_lowercase();
                        let clen: usize = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse().ok())
                            .unwrap_or(0);
                        if buf.len() >= hend + clen {
                            break;
                        }
                    }
                }
                let _ = tx.send(buf);
                let http = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: \
                     {}\r\nConnection: close\r\n\r\n{}",
                    owned_status,
                    owned_body.len(),
                    owned_body
                );
                let _ = sock.write_all(http.as_bytes());
            }
        });
        (format!("http://{addr}"), rx)
    }

    /// A listener that must accept NOTHING — bound, then switched to
    /// nonblocking so a test can assert zero connections arrived.
    struct SilentListener(TcpListener);

    impl SilentListener {
        fn bind() -> Self {
            let l = TcpListener::bind("127.0.0.1:0").expect("silent bind");
            l.set_nonblocking(true).expect("nonblocking");
            Self(l)
        }
        fn assert_no_connections(&self) {
            std::thread::sleep(Duration::from_millis(120));
            assert!(
                self.0.accept().is_err(),
                "a socket was opened despite the pre-socket refusal"
            );
        }
    }

    fn ok_doc(options: &[&str], pick_index: u64, pick: &str) -> String {
        let opts: Vec<String> = options.iter().map(|o| format!("\"{o}\"")).collect();
        format!(
            "{{\"suite\":\"clippy_fix\",\"arm\":\"ENC\",\"lane\":\"hybrid\",\
             \"options\":[{}],\"pick\":\"{}\",\"pick_index\":{},\"probabilities\":null,\
             \"specialist_scores\":[0.9],\"confidence\":0.9,\"escalated\":true,\
             \"abstained\":false,\"us\":42,\"lane_load\":\"ready\",\
             \"receipt\":{{\"build\":\"t\",\"input\":\"i\",\"decision\":\"d\",\"lane\":\"hybrid\"}}}}",
            opts.join(","),
            pick,
            pick_index
        )
    }

    const CANDIDATES: [&str; 3] = ["fn f() -> i32 { 1 }", "fn f() -> i32 { 2 }", "let x = 3;"];
    const SNIPPET: &str = "fn f() -> i32 { 0 }";

    fn candidates() -> Vec<String> {
        CANDIDATES.iter().map(|s| (*s).to_string()).collect()
    }

    fn budget() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }

    // ── (§6 carry-forward) consent: Unset must never open a socket ────

    #[test]
    fn consent_unset_opens_no_socket() {
        let silent = SilentListener::bind();
        let client = SeatRerankClient::new("127.0.0.1:1", "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::parse(None))
            .unwrap_err();
        assert!(
            matches!(err, SeatDecline::Egress(EgressRefusal::Unset)),
            "unset must refuse as Unset: {err:?}"
        );
        silent.assert_no_connections();
    }

    #[test]
    fn consent_junk_refuses_loud_and_opens_no_socket() {
        let silent = SilentListener::bind();
        let client = SeatRerankClient::new("127.0.0.1:1", "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(
                SNIPPET,
                &candidates(),
                budget(),
                EgressConsent::parse(Some("true")),
            )
            .unwrap_err();
        let SeatDecline::Egress(EgressRefusal::NotConsent(got)) = &err else {
            panic!("junk must refuse as NotConsent: {err:?}")
        };
        assert_eq!(got, "true");
        assert!(
            err.to_string().contains("snippet"),
            "the refusal names the accepted spelling: {err}"
        );
        silent.assert_no_connections();
    }

    // ── (§6 carry-forward) deadline: expiry declines pre-socket ───────

    #[test]
    fn expired_deadline_declines_before_the_socket() {
        let silent = SilentListener::bind();
        let client = SeatRerankClient::new("127.0.0.1:1", "clippy_fix").unwrap();
        let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), past, EgressConsent::Allowed)
            .unwrap_err();
        assert_eq!(err, SeatDecline::DeadlineExpired);
        silent.assert_no_connections();
    }

    // ── generate traffic never reaches the seat ───────────────────────

    #[test]
    fn empty_candidates_refuse_before_the_socket() {
        let silent = SilentListener::bind();
        let client = SeatRerankClient::new("127.0.0.1:1", "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &[], budget(), EgressConsent::Allowed)
            .unwrap_err();
        assert_eq!(err, SeatDecline::NoCandidates);
        assert!(
            err.to_string().contains("generate arm"),
            "the refusal routes the caller to the generate arm: {err}"
        );
        silent.assert_no_connections();
    }

    #[test]
    fn https_endpoint_is_a_pre_socket_config_refusal() {
        let silent = SilentListener::bind();
        let err = SeatRerankClient::new("https://seat.example", "clippy_fix").unwrap_err();
        assert!(matches!(err, SeatDecline::Contract(_)), "{err:?}");
        silent.assert_no_connections();
    }

    // ── snippet-only payload ──────────────────────────────────────────

    #[test]
    fn body_carries_snippet_and_candidates_and_nothing_else() {
        let client = SeatRerankClient::new("127.0.0.1:1", "clippy_fix").unwrap();
        let decoy = "FILE CONTENT THAT MUST NEVER LEAVE THE BOX";
        let body = client.build_body(SNIPPET, &candidates()).unwrap();
        assert!(body.contains(SNIPPET), "the snippet rides state: {body}");
        for c in CANDIDATES {
            assert!(body.contains(c), "each candidate rides options: {body}");
        }
        assert!(
            body.contains("\"options\":[\""),
            "the candidates ARE the options array: {body}"
        );
        assert!(
            !body.contains(decoy),
            "the builder takes the snippet + candidates only — a file's content is \
             unrepresentable in the body"
        );
        // One writer: the body is exactly the three-field envelope.
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["options", "state", "suite"],
            "three fields, no extras"
        );
    }

    // ── the happy paths ───────────────────────────────────────────────

    #[test]
    fn pick_round_trips_to_the_callers_candidate() {
        let doc = ok_doc(&CANDIDATES, 1, CANDIDATES[1]);
        let (addr, rx) = fake_seat("200 OK", &doc);
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let answer = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap();
        assert_eq!(
            answer,
            SeatAnswer::Pick {
                index: 1,
                candidate: CANDIDATES[1].to_string(),
                // The ok_doc's receipt block parses leniently: build
                // lands, the blake3 spellings it doesn't use stay None.
                receipt: Some(SeatReceipt {
                    build: Some("t".to_string()),
                    us: Some(42),
                    confidence: Some("0.9".to_string()),
                    arm: Some("ENC".to_string()),
                    ..SeatReceipt::default()
                }),
            }
        );
        // The wire carried the candidates EXACTLY as presented (the
        // server half of "the options are EXACTLY the L0-L3 candidates").
        let sent = String::from_utf8_lossy(&rx.recv().unwrap()).into_owned();
        for c in CANDIDATES {
            assert!(sent.contains(c), "candidate must ride verbatim: {sent}");
        }
        assert!(
            sent.contains(&format!("Host: {addr}")),
            "host header: {sent}"
        );
    }

    #[test]
    fn abstain_stands_the_modelless_answer() {
        let opts: Vec<String> = CANDIDATES.iter().map(|o| format!("\"{o}\"")).collect();
        let doc = format!(
            "{{\"suite\":\"clippy_fix\",\"arm\":\"ENC\",\"options\":[{}],\"pick\":null,\
             \"pick_index\":null,\"abstained\":true,\"confidence\":0.0,\"us\":7}}",
            opts.join(",")
        );
        let (addr, _rx) = fake_seat("200 OK", &doc);
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let answer = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap();
        // The doc carries `us:7` — the receipt tier reads it even on an
        // abstain (the audit trail is about the EXCHANGE, not the pick).
        assert_eq!(
            answer,
            SeatAnswer::Abstained {
                receipt: Some(SeatReceipt {
                    us: Some(7),
                    confidence: Some("0".to_string()),
                    arm: Some("ENC".to_string()),
                    ..SeatReceipt::default()
                })
            }
        );
    }

    #[test]
    fn the_seat_receipt_surfaces_on_a_pick() {
        // The full instinct edge receipt shape (Bench 107 finding 4: the
        // client DROPPED all of it — the audit trail was blind). Every
        // field lands; a field the seat omits stays None.
        let opts: Vec<String> = CANDIDATES.iter().map(|o| format!("\"{o}\"")).collect();
        let doc = format!(
            "{{\"suite\":\"clippy_fix\",\"arm\":\"ENC\",\"options\":[{}],\"pick\":\"{}\",\
             \"pick_index\":1,\"abstained\":false,\"confidence\":0.9,\"us\":42,\
             \"lane_load\":\"ready\",\"receipt\":{{\"build\":\"deadbeef\",\"features\":[\"default\"],\
             \"input_blake3\":\"aa\",\"decision_blake3\":\"bb\"}}}}",
            opts.join(","),
            CANDIDATES[1]
        );
        let (addr, _rx) = fake_seat("200 OK", &doc);
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let answer = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap();
        let SeatAnswer::Pick { receipt, .. } = answer else {
            panic!("expected a pick: {answer:?}");
        };
        let r = receipt.expect("the reply carried a receipt");
        assert_eq!(r.build.as_deref(), Some("deadbeef"));
        assert_eq!(r.input_blake3.as_deref(), Some("aa"));
        assert_eq!(r.decision_blake3.as_deref(), Some("bb"));
        assert_eq!(r.us, Some(42));
        assert_eq!(r.confidence.as_deref(), Some("0.9"));
        assert_eq!(r.arm.as_deref(), Some("ENC"));
    }

    // ── the contract pins (every violation declines) ──────────────────

    #[test]
    fn a_pick_can_never_name_a_non_candidate() {
        // The seat answers a valid index + correct echo but a pick field
        // naming something we never sent — L4 invariant 3: refused.
        let doc = ok_doc(&CANDIDATES, 0, "NON-CANDIDATE INVENTED BY THE TIER");
        let (addr, _rx) = fake_seat("200 OK", &doc);
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap_err();
        let SeatDecline::Contract(m) = &err else {
            panic!("a non-candidate pick must decline Contract: {err:?}")
        };
        assert!(m.contains("non-candidate"), "{m}");
    }

    #[test]
    fn echo_mismatch_declines_even_with_a_valid_index() {
        // The seat re-ordered the presentation: its index space is not
        // ours — the pick would silently answer a different candidate.
        let reordered = [CANDIDATES[1], CANDIDATES[0], CANDIDATES[2]];
        let doc = ok_doc(&reordered, 0, reordered[0]);
        let (addr, _rx) = fake_seat("200 OK", &doc);
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap_err();
        let SeatDecline::Contract(m) = &err else {
            panic!("an echo mismatch must decline Contract: {err:?}")
        };
        assert!(m.contains("echo mismatch"), "{m}");
    }

    #[test]
    fn out_of_range_pick_declines() {
        let doc = ok_doc(&CANDIDATES, 9, "whatever");
        let (addr, _rx) = fake_seat("200 OK", &doc);
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap_err();
        let SeatDecline::Contract(m) = &err else {
            panic!("an out-of-range pick must decline Contract: {err:?}")
        };
        assert!(m.contains("out of range"), "{m}");
    }

    #[test]
    fn neither_pick_nor_abstain_declines() {
        let opts: Vec<String> = CANDIDATES.iter().map(|o| format!("\"{o}\"")).collect();
        let doc = format!(
            "{{\"options\":[{}],\"pick\":null,\"pick_index\":null,\"abstained\":false}}",
            opts.join(",")
        );
        let (addr, _rx) = fake_seat("200 OK", &doc);
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap_err();
        assert!(matches!(err, SeatDecline::Contract(_)), "{err:?}");
    }

    // ── wire failures are declines, never panics ──────────────────────

    #[test]
    fn non_200_declines_wire_naming_the_edge_code() {
        let (addr, _rx) = fake_seat(
            "503 Service Unavailable",
            "{\"error\":\"suite is still loading\",\"code\":\"loading\"}",
        );
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap_err();
        assert_eq!(err, SeatDecline::Wire("loading".into()), "{err:?}");
    }

    #[test]
    fn malformed_body_declines_wire_not_panic() {
        let (addr, _rx) = fake_seat("200 OK", "<html>not json</html>");
        let client = SeatRerankClient::new(&addr, "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap_err();
        assert_eq!(err, SeatDecline::Wire("bad body".into()));
    }

    #[test]
    fn dead_endpoint_declines_wire_fast() {
        // Port 1 on loopback: nothing listens (the R2 dead-endpoint rule).
        let client = SeatRerankClient::new("127.0.0.1:1", "clippy_fix").unwrap();
        let err = client
            .rerank_checked_with(SNIPPET, &candidates(), budget(), EgressConsent::Allowed)
            .unwrap_err();
        assert!(matches!(err, SeatDecline::Wire(_)), "{err:?}");
    }

    #[test]
    fn a_late_complete_reply_is_dropped_not_applied() {
        // The compute-once rule: the reply arrives INSIDE the budget's
        // bytes but the total deadline passed while it was in flight —
        // the parse gate drops it. Driven at the parse seam (the wire
        // half is timing-bound; the RULE is the parse-side gate).
        let doc = ok_doc(&CANDIDATES, 0, CANDIDATES[0]);
        let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        let err = SeatRerankClient::parse_reply(
            &format!("HTTP/1.1 200 OK\r\n\r\n{doc}"),
            &candidates(),
            past,
        )
        .unwrap_err();
        assert_eq!(err, SeatDecline::DeadlineExpired);
    }

    #[test]
    fn debug_never_carries_the_endpoint() {
        let client = SeatRerankClient::new("secret-host:9999", "clippy_fix").unwrap();
        let dbg = format!("{client:?}");
        assert!(
            !dbg.contains("secret-host"),
            "Debug never renders the endpoint (the CfClient rule): {dbg}"
        );
    }
}
