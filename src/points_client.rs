//! The points client (Plan 325 T3–T5; the wire is
//! [`riir_ledger`] — Proposal 026 §2; the service plane is P2, so every
//! verb answers a not-yet-deployed service with a typed, honest refusal —
//! never a hang, never a fake).
//!
//! Posture laws baked in here:
//! - **Offline first**: every verb consults [`crate::offline::offline`]
//!   before any dial; reads degrade to the local line, sends skip.
//! - **Anti-Sybil rank** (Proposal 026 §5.1): `rank` renders the public
//!   board for anyone, but "own rank" requires the joined identity — an
//!   anonymous caller gets the named reason, never a rank.
//! - **Vocabulary** (GAP 10): contribute / contribution / contributor /
//!   contribution points in every rendered word.
//! - **Client UUID** (T16.1): the only identifier this client invents; the
//!   v4-shape law is the wire crate's validator.
//! - **Typed refusals**: a non-2xx body that parses as an
//!   `ErrorEnvelope` surfaces its code + message verbatim; anything else
//!   is a transport-class error with the status attached.
//!
//! The service URL: `RIIR_REFINE_POINTS_SERVICE_URL` overrides; the
//! default is the mainnet host family the points plane rides (the
//! kat-service worker). No `--net` machinery — the points plane is not
//! the KAT service surface (the dialing law's default-mainnet posture is
//! kept; the env is the operator override).

use riir_ledger::FORMAT_VERSION;
use riir_ledger::dto::{
    BalanceResponse, ErrorEnvelope, InfoResponse, JoinRequest, JoinResponse, LeaveRequest,
    LeaveResponse, RankResponse,
};

/// The operator override env for the points service base URL.
pub const POINTS_SERVICE_URL_ENV: &str = "RIIR_REFINE_POINTS_SERVICE_URL";

/// The GitHub device-flow client-id env (the leaf's own name; the id is
/// PUBLIC by design — never a secret).
pub const GITHUB_CLIENT_ID_ENV: &str = "RIIR_AUTH_GITHUB_CLIENT_ID";

/// The compiled-default OAuth app (issue 192 GAP 5, owner: "we already set
/// up github; that's why we can login Refine with github rn" — the
/// EXISTING app IS the product's app, minimal `read:user`). Public so
/// `join` works out of the box; the env override wins for operators.
pub const DEFAULT_GITHUB_CLIENT_ID: &str = "Ov23liAe58wk2byrYeOv";

/// The default dial — the points plane's OWN host family (the same
/// kat-service worker, route-based at the P2 deploy; a separate name keeps
/// the charitable binary free of the KAT ledger host literals — the
/// charitable gate's hard-zero host class stays intact). The points
/// routes are Proposal 026 P2 (not deployed yet): until they answer, verbs
/// refuse with [`PointsError::ServiceNotLive`], which the CLI renders as
/// the W0 roadmap sentence. The env override is the operator's dial.
pub const DEFAULT_SERVICE_URL: &str = "https://honor.riir.ai";

/// The join path (Proposal 026 §2).
pub const JOIN_PATH: &str = "/points/join";
/// The leave path.
pub const LEAVE_PATH: &str = "/points/leave";
/// The balance path.
pub const BALANCE_PATH: &str = "/points/balance";
/// The rank path.
pub const RANK_PATH: &str = "/points/rank";
/// The info path.
pub const INFO_PATH: &str = "/points/info";

/// Signature headers (the P2 server validates `format_version ‖ canonical
/// JSON` against these; the wire crate's `signed_message_bytes` is the
/// one canonicalization).
pub const SIG_PUBKEY_HEADER: &str = "X-Points-Pubkey";
pub const SIG_HEADER: &str = "X-Points-Signature";

/// The default connect/read timeout (the ~5 s budget law — never blocks
/// the run longer; the plan's earn-sync budget is the ceiling posture).
pub const POINTS_TIMEOUT_SECS: u64 = 5;

/// Resolve the base URL: env override wins; else the mainnet default.
#[must_use]
pub fn service_url() -> String {
    std::env::var(POINTS_SERVICE_URL_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SERVICE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

/// The typed failure vocabulary every verb returns.
#[derive(Debug)]
pub enum PointsError {
    /// The offline env is set — the lane was skipped, nothing dialed.
    Offline,
    /// No account key exists — the verb needs the joined identity.
    NotJoined,
    /// The service did not answer / the route does not exist yet (P2
    /// pending). Carries the underlying reason for the run log.
    ServiceNotLive(String),
    /// The service answered with a typed refusal — rendered verbatim.
    Server(ErrorEnvelope),
    /// Anything else (a malformed body, a protocol surprise).
    Protocol(String),
}

impl PointsError {
    /// The one-line render the CLI prints. Honest about W0: until the
    /// service ships, this is the expected answer, and local healing is
    /// unaffected.
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            PointsError::Offline => crate::offline::refusal_line("points"),
            PointsError::NotJoined => {
                "points: no account here — run `cargo refine join` first (contribution credit + rank are joined-only)".to_string()
            }
            PointsError::ServiceNotLive(reason) => format!(
                "points: the contribution service is not reachable yet ({reason}) — it ships with the points plane (W0); local healing is unaffected"
            ),
            PointsError::Server(env) => {
                format!("points: {} — {}", env.code, env.message)
            }
            PointsError::Protocol(reason) => format!("points: protocol error: {reason}"),
        }
    }
}

/// The transport seam (the kat clients' shape: one bounded agent behind a
/// trait so tests inject a fake — no network in tests, ever).
pub trait PointsTransport {
    /// One JSON request. Returns `(status, body)` for ANY answered status
    /// (the caller classifies), or an error string for transport failures.
    ///
    /// # Errors
    /// A transport-class failure (DNS, connect, timeout) as a string — the
    /// caller folds it into [`PointsError::ServiceNotLive`].
    fn request(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
        headers: &[(&str, &str)],
    ) -> Result<(u16, String), String>;
}

/// The ureq transport (the `HttpFixstatTransport` shape: one bounded
/// agent, JSON in/out).
pub struct UreqPointsTransport {
    agent: ureq::Agent,
}

impl UreqPointsTransport {
    #[must_use]
    pub fn new(timeout_secs: u64) -> Self {
        let t = std::time::Duration::from_secs(timeout_secs.max(1));
        let agent = ureq::AgentBuilder::new()
            .timeout(t)
            .timeout_connect(t)
            .timeout_read(t)
            .build();
        Self { agent }
    }
}

impl PointsTransport for UreqPointsTransport {
    fn request(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
        headers: &[(&str, &str)],
    ) -> Result<(u16, String), String> {
        use std::io::Read as _;
        let mut call = match method {
            "POST" => self.agent.post(url),
            "GET" => self.agent.get(url),
            other => return Err(format!("unsupported method {other}")),
        };
        for (k, v) in headers {
            call = call.set(k, v);
        }
        let call = if body.is_some() {
            call.set("Content-Type", "application/json")
        } else {
            call
        };
        let sent = if let Some(json) = body {
            call.send_string(json)
        } else {
            call.call()
        };
        match sent {
            Ok(resp) => {
                let status = resp.status();
                let mut buf = String::new();
                resp.into_reader()
                    .read_to_string(&mut buf)
                    .map_err(|e| format!("read: {e}"))?;
                Ok((status, buf))
            }
            Err(ureq::Error::Status(c, resp)) => {
                let mut buf = String::new();
                let _ = resp.into_reader().read_to_string(&mut buf);
                Ok((c, buf))
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// The client: one base URL + one transport.
pub struct PointsClient<'a> {
    transport: &'a dyn PointsTransport,
    base: String,
}

impl<'a> PointsClient<'a> {
    #[must_use]
    pub fn new(transport: &'a dyn PointsTransport) -> Self {
        Self {
            transport,
            base: service_url(),
        }
    }

    /// Split the answered pair into the typed outcomes (the single
    /// arbiter: a 2xx must parse as its DTO; a non-2xx tries the
    /// `ErrorEnvelope`; a transport error is `ServiceNotLive` — from this
    /// client's posture an unreachable route IS a not-yet-live service).
    fn classify<T: serde::de::DeserializeOwned>(
        answered: Result<(u16, String), String>,
    ) -> Result<T, PointsError> {
        let (status, body) = answered.map_err(PointsError::ServiceNotLive)?;
        if (200..300).contains(&status) {
            return serde_json::from_str(&body)
                .map_err(|e| PointsError::Protocol(format!("2xx body did not parse: {e}")));
        }
        if let Ok(env) = serde_json::from_str::<ErrorEnvelope>(&body) {
            if !env.code.is_empty() {
                return Err(PointsError::Server(env));
            }
        }
        Err(PointsError::ServiceNotLive(format!("HTTP {status}")))
    }

    fn signature_headers(
        signing: &ed25519_dalek::SigningKey,
        message: &[u8],
    ) -> Vec<(&'static str, String)> {
        use ed25519_dalek::Signer as _;
        let sig = signing.sign(message);
        vec![
            (SIG_PUBKEY_HEADER, hex(&signing.verifying_key().to_bytes())),
            (SIG_HEADER, hex(&sig.to_bytes())),
        ]
    }

    /// POST /points/join — the binding row (client UUID ↔ GitHub identity
    /// ↔ account key). `github_login`/`github_id` come from the device
    /// flow's identity (in-memory only, never persisted — the
    /// provider-login law); this client stays decoupled from the identity
    /// substrate's types.
    ///
    /// # Errors
    /// [`PointsError`] — typed; the CLI renders the line.
    pub fn join(
        &self,
        signing: &ed25519_dalek::SigningKey,
        client_uuid: &str,
        github_login: &str,
        github_id: u64,
    ) -> Result<JoinResponse, PointsError> {
        let request = JoinRequest {
            format_version: FORMAT_VERSION,
            client_uuid: client_uuid.to_string(),
            account_pubkey_hex: hex(&signing.verifying_key().to_bytes()),
            github_login: github_login.to_string(),
            github_id,
        };
        let body =
            serde_json::to_string(&request).map_err(|e| PointsError::Protocol(e.to_string()))?;
        let message = request
            .signed_message_bytes()
            .map_err(|e| PointsError::Protocol(e.to_string()))?;
        let headers = Self::signature_headers(signing, &message);
        let header_refs: Vec<(&str, &str)> =
            headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url = format!("{}{JOIN_PATH}", self.base);
        let answered = self
            .transport
            .request("POST", &url, Some(&body), &header_refs);
        Self::classify(answered)
    }

    /// POST /points/leave — unbind. One call, no retry stacking: a failure
    /// is reported, the local state clears regardless (the caller's job).
    ///
    /// # Errors
    /// [`PointsError`].
    pub fn leave(
        &self,
        signing: &ed25519_dalek::SigningKey,
        client_uuid: &str,
    ) -> Result<LeaveResponse, PointsError> {
        let request = LeaveRequest {
            format_version: FORMAT_VERSION,
            client_uuid: client_uuid.to_string(),
            account_pubkey_hex: hex(&signing.verifying_key().to_bytes()),
        };
        let body =
            serde_json::to_string(&request).map_err(|e| PointsError::Protocol(e.to_string()))?;
        let message = request
            .signed_message_bytes()
            .map_err(|e| PointsError::Protocol(e.to_string()))?;
        let headers = Self::signature_headers(signing, &message);
        let header_refs: Vec<(&str, &str)> =
            headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url = format!("{}{LEAVE_PATH}", self.base);
        let answered = self
            .transport
            .request("POST", &url, Some(&body), &header_refs);
        Self::classify(answered)
    }

    /// GET /points/balance — the caller's own balance (joined identity
    /// required: the signature IS the auth).
    ///
    /// # Errors
    /// [`PointsError`].
    pub fn balance(
        &self,
        signing: &ed25519_dalek::SigningKey,
        client_uuid: &str,
    ) -> Result<BalanceResponse, PointsError> {
        use ed25519_dalek::Signer as _;
        let pubkey = hex(&signing.verifying_key().to_bytes());
        let signature = hex(&signing
            .sign(format!("{client_uuid}:{pubkey}").as_bytes())
            .to_bytes());
        let headers = [(SIG_PUBKEY_HEADER, pubkey), (SIG_HEADER, signature)];
        let header_refs: Vec<(&str, &str)> =
            headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url = format!("{}{BALANCE_PATH}?client_uuid={client_uuid}", self.base);
        let answered = self.transport.request("GET", &url, None, &header_refs);
        Self::classify(answered)
    }

    /// GET /points/rank — the public top-ten; the signature headers ride
    /// along only when a joined identity exists (else the server answers
    /// the board with `own_rank: null` — and the CLI adds the anti-Sybil
    /// sentence itself, never a server round-trip dependency).
    ///
    /// # Errors
    /// [`PointsError`].
    pub fn rank(
        &self,
        signing: Option<&ed25519_dalek::SigningKey>,
    ) -> Result<RankResponse, PointsError> {
        let headers = match signing {
            None => Vec::new(),
            Some(key) => Self::signature_headers(key, b"rank"),
        };
        let header_refs: Vec<(&str, &str)> =
            headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url = format!("{}{RANK_PATH}", self.base);
        let answered = self.transport.request("GET", &url, None, &header_refs);
        Self::classify(answered)
    }

    /// GET /points/info — the public aggregate (counts + domains only).
    ///
    /// # Errors
    /// [`PointsError`].
    pub fn info(&self) -> Result<InfoResponse, PointsError> {
        let url = format!("{}{INFO_PATH}", self.base);
        let answered = self.transport.request("GET", &url, None, &[]);
        Self::classify(answered)
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

// ── Rendering (pure; the CLI runners in the bin call these) ─────────────

/// Render the rank board (the public top-ten). `own` carries the caller's
/// joined identity state: `Ok(Some(response))` rows render the own-rank
/// line; an `Err(reason)` renders the anti-Sybil / not-joined sentence —
/// the named reason, never a rank (Proposal 026 §5.1).
#[must_use]
pub fn render_rank(response: &RankResponse, own_note: Option<&str>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "contribution leaderboard — epoch {}", response.epoch);
    if response.board.is_empty() {
        out.push_str("  (no contributions ranked yet — be the first to contribute)\n");
    }
    for (i, entry) in response.board.iter().enumerate() {
        let _ = writeln!(out, "  {:>2}. {:<24} {}", i + 1, entry.handle, entry.points);
    }
    match (response.own_rank, response.own_points, own_note) {
        (Some(rank), Some(points), _) => {
            let _ = writeln!(out, "  you: rank {rank} · {points} points");
        }
        _ => {
            if let Some(note) = own_note {
                out.push_str(note);
                out.push('\n');
            }
        }
    }
    out
}

/// Render the info aggregate (counts + domains only — the redaction
/// policy's scope law; no repo names, no user ids by wire construction).
#[must_use]
pub fn render_info(response: &InfoResponse) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "the contribution network — epoch {} · {} contributors · {} contributions",
        response.epoch, response.total_contributors, response.total_contributions
    );
    if response.top_domains.is_empty() {
        out.push_str("  (no contribution domains recorded yet)\n");
    }
    for domain in &response.top_domains {
        let _ = writeln!(out, "  {:<24} {}", domain.domain, domain.contributions);
    }
    out.push_str("  (aggregated counts only — never repo content, never per-user rows)\n");
    out
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use riir_ledger::dto::{DomainCount, RankEntry};

    #[test]
    fn rank_board_renders_top_ten_and_the_own_rank_line() {
        let response = RankResponse {
            format_version: 1,
            epoch: 42,
            board: vec![
                RankEntry {
                    handle: "alice".into(),
                    points: 900,
                },
                RankEntry {
                    handle: "bob".into(),
                    points: 700,
                },
            ],
            own_rank: Some(17),
            own_points: Some(120),
        };
        let text = render_rank(&response, None);
        assert!(text.contains("epoch 42"));
        assert!(text.contains(" 1. alice"));
        assert!(text.contains(" 2. bob"));
        assert!(text.contains("you: rank 17 · 120 points"));
    }

    #[test]
    fn anonymous_rank_carries_the_named_reason_never_a_rank() {
        let response = RankResponse {
            format_version: 1,
            epoch: 42,
            board: vec![RankEntry {
                handle: "alice".into(),
                points: 900,
            }],
            own_rank: None,
            own_points: None,
        };
        let note = "  your rank: joined identities only — a rotating ID could farm the board. Run `cargo refine join`.\n";
        let text = render_rank(&response, Some(note));
        assert!(text.contains("joined identities only"), "{text}");
        assert!(!text.contains("you: rank"), "{text}");
    }

    #[test]
    fn info_renders_counts_and_domains_only() {
        let response = InfoResponse {
            format_version: 1,
            epoch: 42,
            top_domains: vec![DomainCount {
                domain: "clippy".into(),
                contributions: 1_200,
            }],
            total_contributors: 33,
            total_contributions: 4_566,
        };
        let text = render_info(&response);
        assert!(text.contains("33 contributors"));
        assert!(text.contains("clippy"));
        assert!(text.contains("never repo content"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A fake transport: the scripted answers, the observed request kept.
    struct FakeTransport {
        answers: RefCell<Vec<(u16, String)>>,
        seen: RefCell<Vec<String>>,
    }

    impl FakeTransport {
        fn new(answers: Vec<(u16, String)>) -> Self {
            Self {
                answers: RefCell::new(answers),
                seen: RefCell::new(Vec::new()),
            }
        }
    }

    impl PointsTransport for FakeTransport {
        fn request(
            &self,
            method: &str,
            url: &str,
            body: Option<&str>,
            _headers: &[(&str, &str)],
        ) -> Result<(u16, String), String> {
            self.seen
                .borrow_mut()
                .push(format!("{method} {url} {body:?}"));
            self.answers
                .borrow_mut()
                .pop()
                .ok_or_else(|| "no scripted answer".to_string())
        }
    }

    fn key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[9u8; 32])
    }

    #[test]
    fn typed_server_refusal_surfaces_code_and_message() {
        let fake = FakeTransport::new(vec![(
            403,
            r#"{"format_version":1,"code":"not_joined","message":"join first"}"#.to_string(),
        )]);
        let client = PointsClient::new(&fake);
        let err = client.rank(None).unwrap_err();
        let line = err.line();
        assert!(line.contains("not_joined"), "{line}");
        assert!(line.contains("join first"), "{line}");
    }

    #[test]
    fn transport_failure_is_service_not_live_with_the_w0_sentence() {
        let fake = FakeTransport::new(vec![]);
        let client = PointsClient::new(&fake);
        let err = client.info().unwrap_err();
        assert!(matches!(err, PointsError::ServiceNotLive(_)));
        let line = err.line();
        assert!(line.contains("W0"), "{line}");
        assert!(line.contains("local healing is unaffected"), "{line}");
    }

    #[test]
    fn join_sends_the_signed_wire_and_parses_the_answer() {
        let answer = r#"{"format_version":1,"joined":true,"epoch":7}"#;
        let fake = FakeTransport::new(vec![(200, answer.to_string())]);
        let client = PointsClient::new(&fake);
        let response = client.join(
            &key(),
            "0f4e2d3c-4b5a-6c7d-8e9f-0a1b2c3d4e5f",
            "octocat",
            583_231,
        );
        let response = response.unwrap_or_else(|e| panic!("{e:?}"));
        assert!(response.joined);
        assert_eq!(response.epoch, 7);
        let seen = fake.seen.borrow();
        assert!(seen[0].starts_with("POST https://honor.riir.ai/points/join"));
        assert!(seen[0].contains("octocat"));
        assert!(seen[0].contains("583231"));
    }

    #[test]
    fn service_url_env_overrides_the_default() {
        std::env::set_var(POINTS_SERVICE_URL_ENV, "http://127.0.0.1:9/");
        assert_eq!(service_url(), "http://127.0.0.1:9");
        std::env::remove_var(POINTS_SERVICE_URL_ENV);
        assert_eq!(service_url(), DEFAULT_SERVICE_URL);
        // The default is the points plane's own host — never the KAT ledger
        // host (the charitable gate's hard-zero host class stays intact).
        assert_eq!(DEFAULT_SERVICE_URL, "https://honor.riir.ai");
    }
}
