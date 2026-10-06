//! Issue 136 — the admission-gated structural self-mutation vocabulary
//! (reef distill, Research 213 P1+P3+P5).
//!
//! The healer already owns OUTCOME gates on numbers (`self_evolve`'s
//! Lemma-4.2 tier certification, Elo/BetaPosterior selection, the
//! lexicographic keep rule) and an ORACLE gate on content (`frontier_miner`
//! verifies against real cargo clippy), but STRUCTURAL changes to its own
//! surface are human-batch-only: corpus entries land via mining batches,
//! `--active-set-demote` is a human act by law, config values evolve only
//! through GEPA-D's numeric bandit. This module is the missing PRE-oracle
//! layer, taken from reef @ `ecec4e2`:
//!
//! 1. **Per-kind shape gates** (reef P1) — a proposal to mutate the
//!    healer's structure is validated by its [`ProposalKind`]'s gate at
//!    proposal time; a refused proposal is a FAILED proposal: typed,
//!    logged by the caller, never applied. Shape only — the oracle
//!    (clippy), the score-bench/freeze gates (corpora) and GEPA-D (numeric
//!    config values) stay exactly where they are. ADDITIVE, pre-oracle.
//! 2. **Person-review classes keyed on KIND** (reef P3) — review is keyed
//!    on the KIND's privilege, never on a confidence score:
//!    [`ReviewClass::AlwaysReviewed`] kinds (playbook, fixture, config
//!    knob, active-set marker) execute or change executed behavior, so no
//!    oracle score can wave them through; [`ReviewClass::AutoServable`] is
//!    the data-shaped class (corpus rules), whose quality layer is the
//!    oracle + bench gates above.
//! 3. **Two-head commit chain** (reef P5) — [`ChainState`] carries
//!    `current` + `checkpoint` heads over immutable content-addressed
//!    artifacts with CAS advance, plus a `live` child built from the
//!    checkpoint WITHOUT moving a head (serve continuity while a candidate
//!    accumulates). A bad auto-served batch is a CAS repoint, not a revert.
//!
//! # What this module is NOT
//!
//! - **Not an auto-mutation activation.** Nothing here mutates anything:
//!   the gates are pure functions, the chain is a pure state machine, and
//!   the only filesystem surface (the store behind the `structural_admission`
//!   feature in [`crate::structural_admission_store`]) is opt-in and has no
//!   production caller. Activation of any auto-serve kind is a separate
//!   owner call.
//! - **Not a flat-tree violator.** The reef invariant ("the compose loader
//!   would mount its children through the same plugins, out of sight of
//!   every check that walks the root") is enforced by the type:
//!   [`ProposalPayload`] has exactly one edit per variant and no
//!   list-of-edits variant exists to construct.
//!
//! # P2 fold (secret shapes)
//!
//! reef lints persisted text for credential shapes at the persistence
//! boundary ("a pasted key in a rule would outlive rotation in every commit
//! record"). The directive-shaped half targets prompt-bearing stores, which
//! the healer does not persist today (Research 213 folds it on arrival);
//! the SECRET-shaped half is live here — every payload body that could
//! carry pasted source or config text is scanned at admission time.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::active_set::KNOWN_DOMAIN_KEYS;
use crate::kat_redact::FindingKind;

/// Shape bound on every proposal payload body (and on the serialized
/// artifact as a whole via the same cap). The reflexer vessel cap, reused:
/// a structural proposal is a reviewable diff, not a data dump.
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

/// Hard cap on a proposal's human title — metadata, but persisted in the
/// content-addressed artifact, so it gets a shape bound too.
pub const MAX_TITLE_BYTES: usize = 200;

/// The version of [`CHAIN_STATE_FILE`]. A file carrying a different version
/// is refused LOUDLY by the store — the chain's CAS semantics could differ
/// between versions, and a silent accept would be a repoint past an
/// unread state machine.
pub const CHAIN_STATE_VERSION: u32 = 1;

/// The heads file, RELATIVE to the repo root the caller resolves (the same
/// root the `.refine/` trajectory store hangs off).
pub const CHAIN_STATE_FILE: &str = ".refine/structural/heads.json";

/// The immutable content-addressed artifact directory, RELATIVE to the same
/// root. Artifact address = BLAKE3 of the canonical proposal bytes.
pub const ARTIFACTS_DIR: &str = ".refine/structural/artifacts";

/// The healer's mutable structural surface, typed (reef P1's node-kind
/// vocabulary mapped onto OUR surface). Kind is DERIVED from the payload —
/// there is no independent kind field to drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ProposalKind {
    /// A corpus rule entry in one of the retrieval domains
    /// ([`KNOWN_DOMAIN_KEYS`]). Data-shaped: the oracle + bench gates stay
    /// its quality layer, so this is the one [`ReviewClass::AutoServable`]
    /// kind.
    CorpusRule,
    /// A seed-corpus fixture — text that SHIPS CODE (the freeze/e2e lanes
    /// compile it). Privileged: [`ReviewClass::AlwaysReviewed`].
    Fixture,
    /// A value for an EXISTING config knob (no new knobs through proposals).
    /// Changes executed behavior — the issue's `-D` list example lives here:
    /// [`ReviewClass::AlwaysReviewed`].
    ConfigKnob,
    /// A demote/readmit/cap marker edit. The Issue-102 law ("nothing demotes
    /// automatically" until the fade-predictive clock fires) is encoded as
    /// the class: [`ReviewClass::AlwaysReviewed`].
    ActiveSetMarker,
    /// A playbook/recipe/script change — code that runs in the loop's own
    /// privileges. reef's `native_loop` analogue:
    /// [`ReviewClass::AlwaysReviewed`] regardless of any score.
    Playbook,
}

/// Person-review class of a proposal kind (reef P3). Keyed on the KIND's
/// privilege — never on confidence, never on an oracle score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ReviewClass {
    /// A person reviews before the proposal can serve, whatever its oracle
    /// score says. 4 of our 5 kinds land here — the honest finding is that
    /// the healer's surface is mostly privileged.
    AlwaysReviewed,
    /// MAY auto-serve through the commit chain once an owner activates that
    /// kind (no kind is activated today). The quality layer above the shape
    /// gate is unchanged: oracle for content, bench gates for corpora.
    AutoServable,
}

impl ProposalKind {
    /// Every kind, enumerated — a kind added without a review-class row
    /// fails this match (no wildcard arm) and the tests pin the table.
    pub const ALL: &'static [ProposalKind] = &[
        ProposalKind::CorpusRule,
        ProposalKind::Fixture,
        ProposalKind::ConfigKnob,
        ProposalKind::ActiveSetMarker,
        ProposalKind::Playbook,
    ];

    /// The stable spelling used in logs and refusal messages.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            ProposalKind::CorpusRule => "corpus_rule",
            ProposalKind::Fixture => "fixture",
            ProposalKind::ConfigKnob => "config_knob",
            ProposalKind::ActiveSetMarker => "active_set_marker",
            ProposalKind::Playbook => "playbook",
        }
    }
}

/// Review class of a kind (reef P3 — kind-keyed, not policy-keyed).
#[must_use]
pub fn review_class(kind: ProposalKind) -> ReviewClass {
    match kind {
        ProposalKind::CorpusRule => ReviewClass::AutoServable,
        ProposalKind::Fixture
        | ProposalKind::ConfigKnob
        | ProposalKind::ActiveSetMarker
        | ProposalKind::Playbook => ReviewClass::AlwaysReviewed,
    }
}

/// Whether a corpus rule carries a benchmark shape. The Issue-136 gate
/// mirror of the corpus rule itself: an entry must carry a bench spec OR be
/// explicitly NOT-benchable (the `NotBenchable` annotation vocabulary) — a
/// rule that is neither is shape-inadmissible because nobody downstream can
/// tell how it was ever verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchShape {
    /// The entry carries a bench spec (the payload names it).
    BenchSpec(String),
    /// The entry is explicitly NOT benchable (suggest-only /
    /// measurement-altering classes).
    NotBenchable,
    /// No bench shape at all — REFUSED at admission.
    Absent,
}

/// An active-set marker edit — the [`ProposalKind::ActiveSetMarker`]
/// payload. Mirrors the marker file's three operations
/// ([`crate::active_set`]); one edit per proposal (flat-tree invariant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveSetEdit {
    /// Drop a rule from a domain's similarity fan-out.
    Demote { domain: String, rule_id: String },
    /// Return a demoted rule to the fan-out.
    Readmit { domain: String, rule_id: String },
    /// Set a domain's live-rule ceiling (0 = OFF; the Library-Drift
    /// default).
    SetCap { domain: String, cap: u32 },
}

/// The ONE structural edit a proposal carries (flat-tree invariant: no
/// variant groups sub-edits — one proposal IS one visible diff).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalPayload {
    /// A corpus rule for one of the retrieval domains. `body` is the
    /// entry's canonical JSON as the proposer intends it to land — shape
    /// (bench shape, domain, ids, secret-free, bounded) is checked here;
    /// entry-schema and content truth stay with the corpus loader and the
    /// oracle.
    CorpusRule {
        domain: String,
        rule_id: String,
        bench: BenchShape,
        body: String,
    },
    /// A fixture file (ships code — [`ReviewClass::AlwaysReviewed`]).
    Fixture { rel_path: String, body: String },
    /// A value (canonical JSON) for an EXISTING knob. New knobs are not a
    /// structural proposal — they are a code change.
    ConfigKnob { knob: String, value_json: String },
    /// One active-set marker edit.
    ActiveSetMarker { edit: ActiveSetEdit },
    /// A playbook/recipe/script change (executes — always reviewed).
    Playbook { rel_path: String, body: String },
}

impl ProposalPayload {
    /// The payload's kind — the single derivation every consumer shares.
    #[must_use]
    pub fn kind(&self) -> ProposalKind {
        match self {
            ProposalPayload::CorpusRule { .. } => ProposalKind::CorpusRule,
            ProposalPayload::Fixture { .. } => ProposalKind::Fixture,
            ProposalPayload::ConfigKnob { .. } => ProposalKind::ConfigKnob,
            ProposalPayload::ActiveSetMarker { .. } => ProposalKind::ActiveSetMarker,
            ProposalPayload::Playbook { .. } => ProposalKind::Playbook,
        }
    }
}

/// A structural self-mutation proposal: what a proposer submits, what the
/// shape gate admits, and what gets serialized into the content-addressed
/// artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralProposal {
    /// Human one-liner — the review surface's first line.
    pub title: String,
    /// The single edit (flat-tree invariant).
    pub payload: ProposalPayload,
}

impl StructuralProposal {
    /// The proposal's kind — derived from the payload, never carried.
    #[must_use]
    pub fn kind(&self) -> ProposalKind {
        self.payload.kind()
    }
}

/// Admission context — the compiled surface a shape gate must check its
/// payload AGAINST. Gates that need no context take none.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdmissionCtx<'a> {
    /// The knob names the compiled config surface actually exposes. A
    /// config proposal naming anything else is refused: no new knobs
    /// through proposals.
    pub known_knobs: &'a [&'a str],
}

/// Why a proposal was refused (reef's FAILED-fiber vocabulary). Typed so a
/// future admission log can bucket refusals without re-parsing prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    EmptyTitle,
    TitleTooLong { len: usize, cap: usize },
    SecretShapedText { shape: &'static str, offset: usize },
    PayloadTooLarge { bytes: usize, cap: usize },
    EmptyBody,
    BenchShapeAbsent,
    UnknownDomain { domain: String },
    UnknownConfigKnob { knob: String },
    MalformedRelPath { path: String, reason: &'static str },
    MalformedToken { field: &'static str, value: String },
    MalformedJson { message: String },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::EmptyTitle => write!(f, "refused: empty title"),
            Refusal::TitleTooLong { len, cap } => {
                write!(f, "refused: title {len} bytes over the {cap}-byte cap")
            }
            Refusal::SecretShapedText { shape, offset } => write!(
                f,
                "refused: secret-shaped text (shape {shape} at byte {offset}) — reef P2: \
                 a pasted key would outlive rotation in every commit record"
            ),
            Refusal::PayloadTooLarge { bytes, cap } => {
                write!(f, "refused: payload {bytes} bytes over the {cap}-byte cap")
            }
            Refusal::EmptyBody => write!(f, "refused: empty payload body"),
            Refusal::BenchShapeAbsent => write!(
                f,
                "refused: corpus rule carries neither a bench spec nor an explicit \
                 NotBenchable annotation — its verification shape is undeclarable"
            ),
            Refusal::UnknownDomain { domain } => {
                write!(
                    f,
                    "refused: unknown domain key {domain:?} (not in the retrieval vocabulary)"
                )
            }
            Refusal::UnknownConfigKnob { knob } => write!(
                f,
                "refused: unknown config knob {knob:?} — new knobs are a code change, \
                 not a structural proposal"
            ),
            Refusal::MalformedRelPath { path, reason } => {
                write!(f, "refused: malformed relative path {path:?} ({reason})")
            }
            Refusal::MalformedToken { field, value } => {
                write!(f, "refused: malformed {field} {value:?} (single [A-Za-z0-9_-] token, <= 128 bytes)")
            }
            Refusal::MalformedJson { message } => {
                write!(f, "refused: payload JSON does not parse ({message})")
            }
        }
    }
}

impl std::error::Error for Refusal {}

/// Admit a proposal on SHAPE (reef P1) — the pre-oracle gate. Returns the
/// first refusal, ordered per kind. Pure: nothing is read, nothing is
/// written, nothing is mutated.
///
/// # Errors
///
/// Returns the first [`Refusal`] the payload's kind-gate produces: title
/// shape first, then the kind-specific checks in declaration order (domain
/// membership, token shape, bench shape, path shape, JSON parse, body
/// bounds, secret scan). An `Ok` proposal has passed every SHAPE gate —
/// content truth stays with the oracle layers above.
pub fn admit(proposal: &StructuralProposal, ctx: &AdmissionCtx<'_>) -> Result<(), Refusal> {
    let title = proposal.title.trim();
    if title.is_empty() {
        return Err(Refusal::EmptyTitle);
    }
    if proposal.title.len() > MAX_TITLE_BYTES {
        return Err(Refusal::TitleTooLong {
            len: proposal.title.len(),
            cap: MAX_TITLE_BYTES,
        });
    }
    match &proposal.payload {
        ProposalPayload::CorpusRule {
            domain,
            rule_id,
            bench,
            body,
        } => {
            if !KNOWN_DOMAIN_KEYS.contains(&domain.as_str()) {
                return Err(Refusal::UnknownDomain {
                    domain: domain.clone(),
                });
            }
            check_token("rule_id", rule_id)?;
            if matches!(bench, BenchShape::Absent) {
                return Err(Refusal::BenchShapeAbsent);
            }
            check_body(body)
        }
        ProposalPayload::Fixture { rel_path, body }
        | ProposalPayload::Playbook { rel_path, body } => {
            // Identical by design: both are (rel-path, bounded body) edits —
            // the kinds differ only in review class (reef P3), never in shape.
            check_rel_path(rel_path)?;
            check_body(body)
        }
        ProposalPayload::ConfigKnob { knob, value_json } => {
            if !ctx.known_knobs.contains(&knob.as_str()) {
                return Err(Refusal::UnknownConfigKnob { knob: knob.clone() });
            }
            serde_json::from_str::<serde_json::Value>(value_json).map_err(|e| {
                Refusal::MalformedJson {
                    message: e.to_string(),
                }
            })?;
            check_body(value_json)
        }
        ProposalPayload::ActiveSetMarker { edit } => match edit {
            ActiveSetEdit::Demote { domain, rule_id }
            | ActiveSetEdit::Readmit { domain, rule_id } => {
                if !KNOWN_DOMAIN_KEYS.contains(&domain.as_str()) {
                    return Err(Refusal::UnknownDomain {
                        domain: domain.clone(),
                    });
                }
                check_token("rule_id", rule_id)
            }
            ActiveSetEdit::SetCap { domain, .. } => {
                if !KNOWN_DOMAIN_KEYS.contains(&domain.as_str()) {
                    return Err(Refusal::UnknownDomain {
                        domain: domain.clone(),
                    });
                }
                Ok(())
            }
        },
    }
}

/// Bounded body checks shared by every text-bearing payload: non-empty,
/// under the cap, secret-free (reef P2 at the persistence boundary).
fn check_body(body: &str) -> Result<(), Refusal> {
    if body.is_empty() {
        return Err(Refusal::EmptyBody);
    }
    if body.len() > MAX_PAYLOAD_BYTES {
        return Err(Refusal::PayloadTooLarge {
            bytes: body.len(),
            cap: MAX_PAYLOAD_BYTES,
        });
    }
    if let Some(hit) = scan_secret_shapes(body).first() {
        return Err(Refusal::SecretShapedText {
            shape: hit.shape,
            offset: hit.offset,
        });
    }
    Ok(())
}

/// A single identifier (rule ids; domain keys are checked by membership in
/// [`KNOWN_DOMAIN_KEYS`], which implies the shape).
fn check_token(field: &'static str, value: &str) -> Result<(), Refusal> {
    let ok = !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(Refusal::MalformedToken {
            field,
            value: value.to_owned(),
        })
    }
}

/// Relative-path shape: canonical forward-slash form, no absolute path, no
/// Windows drive, no parent escape, no empty/redundant components.
/// Backslashes are refused outright (not normalized away) — the canonical
/// form is forward slashes and a proposal carrying `\\` is rewritten
/// nowhere behind the reviewer's back.
fn check_rel_path(path: &str) -> Result<(), Refusal> {
    if path.is_empty() {
        return Err(Refusal::MalformedRelPath {
            path: path.to_owned(),
            reason: "empty",
        });
    }
    if path.contains('\\') {
        return Err(Refusal::MalformedRelPath {
            path: path.to_owned(),
            reason: "backslash (canonical form is forward slashes)",
        });
    }
    if path.starts_with('/') {
        return Err(Refusal::MalformedRelPath {
            path: path.to_owned(),
            reason: "absolute path",
        });
    }
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(Refusal::MalformedRelPath {
            path: path.to_owned(),
            reason: "windows drive letter",
        });
    }
    for seg in path.split('/') {
        if seg.is_empty() {
            return Err(Refusal::MalformedRelPath {
                path: path.to_owned(),
                reason: "empty component",
            });
        }
        if seg == ".." {
            return Err(Refusal::MalformedRelPath {
                path: path.to_owned(),
                reason: "parent escape",
            });
        }
        if seg == "." {
            return Err(Refusal::MalformedRelPath {
                path: path.to_owned(),
                reason: "redundant component",
            });
        }
    }
    Ok(())
}

/// One credential-shape hit (reef P2's `_SECRET_TEXT` class, prefix-scanned
/// so no regex dep enters the tree).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretHit {
    /// The shape name (stable vocabulary for refusal logs).
    pub shape: &'static str,
    /// Byte offset of the prefix in the scanned text.
    pub offset: usize,
}

/// Legacy reef shape names for the kinds the private table used to carry.
/// The names are pinned by the reef tests (refusal logs carry them) and are
/// kept byte-identical through the Issue-151 T9 retirement of
/// `SECRET_PREFIX_TABLE` — the vocabulary is now `kat_redact`'s `FindingKind`
/// and this map is the persistence-boundary projection only.
fn reef_shape_name(kind: FindingKind, text: &str, offset: usize) -> &'static str {
    match kind {
        FindingKind::OpenAiStyleKey => "openai_key",
        FindingKind::GithubToken => {
            // One kind covers every GitHub prefix; the legacy split resolves
            // by peeking the text at the finding offset.
            if text[offset..].starts_with("github_pat_") {
                "github_pat_finegrained"
            } else {
                "github_pat_classic"
            }
        }
        FindingKind::AwsAccessKey => "aws_access_key_id",
        FindingKind::SlackToken => "slack_token",
        FindingKind::GoogleApiKey => "google_api_key",
        FindingKind::PrivateKeyBlock => "pem_private_key",
        _ => {
            // New families keep the canonical tag (the kebab spelling the
            // egress gate logs) — no second vocabulary grows here.
            leak_tag(kind)
        }
    }
}

fn leak_tag(kind: FindingKind) -> &'static str {
    // Box the leak: `FindingKind::tag` returns a 'static str, so this keeps
    // the match arm above honest about the legacy names it pins.
    match kind {
        FindingKind::GitlabToken => "gitlab-token",
        _ => "secret-shape",
    }
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// Scan text for credential shapes (reef P2). Since Issue 151 T9 this
/// consumes the ONE canonical scanner (`kat_redact::scan_secrets` — the
/// consolidated v4 pattern set) instead of a private prefix table; the
/// `AIza` fragmentation that motivated the consolidation cannot recur.
/// The persistence-boundary conservatism is the POST-FILTER, not a second
/// vocabulary:
/// - **prefix kinds + private-key blocks only** — entropy/env/bearer/conn/
///   rust-literal/JWT/telegram/discord findings are NOT admitted here: a
///   structural proposal legitimately carries hashes and quoted code, and
///   the entropy rules would block every such payload;
/// - **word boundary** — the byte before a finding must not continue the
///   token (`task-` must not read as an `sk-` key), so prose mentioning
///   these products does not trip the gate.
///   Returns every hit sorted by offset, so the FIRST reported one is
///   deterministic.
#[must_use]
pub fn scan_secret_shapes(text: &str) -> Vec<SecretHit> {
    let bytes = text.as_bytes();
    let mut hits: Vec<SecretHit> = crate::kat_redact::scan_secrets(text)
        .into_iter()
        .filter(|f| {
            matches!(
                f.kind,
                FindingKind::AwsAccessKey
                    | FindingKind::GithubToken
                    | FindingKind::GitlabToken
                    | FindingKind::SlackToken
                    | FindingKind::OpenAiStyleKey
                    | FindingKind::GoogleApiKey
                    | FindingKind::StripeLiveKey
                    | FindingKind::SendGridToken
                    | FindingKind::NpmToken
                    | FindingKind::PypiToken
                    | FindingKind::HuggingFaceToken
                    | FindingKind::ShopifyToken
                    | FindingKind::SquareToken
                    | FindingKind::DatabricksToken
                    | FindingKind::DigitalOceanToken
                    | FindingKind::FigmaToken
                    | FindingKind::SupabaseToken
                    | FindingKind::NotionToken
                    | FindingKind::LinearToken
                    | FindingKind::PerplexityKey
                    | FindingKind::XaiKey
                    | FindingKind::GroqKey
                    | FindingKind::MailgunKey
                    | FindingKind::AgeKey
                    | FindingKind::AtlassianToken
                    | FindingKind::FacebookToken
                    | FindingKind::PrivateKeyBlock,
            )
        })
        .filter(|f| f.start == 0 || !is_word_byte(bytes[f.start - 1]))
        .map(|f| SecretHit {
            shape: reef_shape_name(f.kind, text, f.start),
            offset: f.start,
        })
        .collect();
    hits.sort_by_key(|h| h.offset);
    hits
}

/// The canonical bytes a proposal artifact is committed under: compact
/// serde JSON (struct field order = declaration order, so the encoding is
/// deterministic). The ADDRESS is [`artifact_hash`].
///
/// # Errors
///
/// [`Refusal::MalformedJson`] if serialization fails (cannot happen for a
/// self-contained proposal — carried as `Result` so the artifact bytes are
/// never silently empty).
pub fn canonical_artifact_bytes(proposal: &StructuralProposal) -> Result<Vec<u8>, Refusal> {
    serde_json::to_vec(proposal).map_err(|e| Refusal::MalformedJson {
        message: e.to_string(),
    })
}

/// Content address of a proposal artifact: full 64-hex lowercase BLAKE3 of
/// [`canonical_artifact_bytes`]. Immutable by construction — a different
/// proposal is a different address.
///
/// # Errors
///
/// As [`canonical_artifact_bytes`].
pub fn artifact_hash(proposal: &StructuralProposal) -> Result<String, Refusal> {
    Ok(blake3::hash(&canonical_artifact_bytes(proposal)?)
        .to_hex()
        .to_string())
}

/// Validate an artifact-hash spelling: 64 lowercase hex chars.
///
/// # Errors
///
/// A static reason string: wrong length, or non-lowercase-hex bytes.
pub fn validate_artifact_hash(hash: &str) -> Result<(), &'static str> {
    if hash.len() != 64 {
        return Err("expected 64 hex chars");
    }
    if !hash
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("expected lowercase hex");
    }
    Ok(())
}

/// One committed position in the chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainHead {
    /// The artifact this head points at (64-hex BLAKE3).
    pub artifact: String,
    /// Monotone allocation number — every head ever minted by this chain
    /// has a distinct seq, across all three slots.
    pub seq: u64,
    /// Unix seconds, supplied by the caller (pure state machine: no clock
    /// reads inside).
    pub at_unix: u64,
    /// The commit note (who/why — a person reads this on review).
    pub note: String,
    /// Lineage: the artifact this head was built on (`current`/`checkpoint`:
    /// what was `current` at commit time; `live`: the checkpoint it is a
    /// child of).
    pub parent: Option<String>,
}

/// A chain slot name — error-message vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadSlot {
    Current,
    Checkpoint,
    Live,
}

impl fmt::Display for HeadSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeadSlot::Current => write!(f, "current"),
            HeadSlot::Checkpoint => write!(f, "checkpoint"),
            HeadSlot::Live => write!(f, "live"),
        }
    }
}

/// Chain machine errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// CAS refused: the slot's observed seq is not what the caller saw.
    CasMismatch {
        slot: HeadSlot,
        expected: Option<u64>,
        actual: Option<u64>,
    },
    MalformedArtifactHash {
        hash: String,
        why: &'static str,
    },
    /// `prepare_live` / `rollback_to_checkpoint` need a durable checkpoint.
    NoCheckpoint {
        action: &'static str,
    },
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainError::CasMismatch {
                slot,
                expected,
                actual,
            } => write!(
                f,
                "CAS refused on {slot}: expected {expected:?}, found {actual:?} — \
                 the chain moved under the caller; re-observe and retry"
            ),
            ChainError::MalformedArtifactHash { hash, why } => {
                write!(f, "malformed artifact hash {hash:?} ({why})")
            }
            ChainError::NoCheckpoint { action } => {
                write!(
                    f,
                    "{action} requires a durable checkpoint head; none exists"
                )
            }
        }
    }
}

impl std::error::Error for ChainError {}

/// The two-head commit chain state (reef P5): immutable content-addressed
/// artifacts + `current`/`checkpoint` heads with CAS advance + a `live`
/// child built from the checkpoint WITHOUT moving a head. Rollback is a CAS
/// repoint of `current` — a commit, not a revert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChainState {
    /// Always [`CHAIN_STATE_VERSION`] on write; the store refuses any other
    /// value on load.
    pub version: u32,
    /// The serving head — what an auto-served batch advances.
    pub current: Option<ChainHead>,
    /// The durable head — the known-good position rollbacks repoint to.
    pub checkpoint: Option<ChainHead>,
    /// The experimental child of the checkpoint (serve continuity while a
    /// candidate accumulates). Never a head-move: current/checkpoint stay
    /// put across `prepare_live`.
    pub live: Option<ChainHead>,
    /// The allocation counter every head's seq is drawn from.
    pub seq: u64,
}

impl ChainState {
    /// The empty chain — what an absent heads file means.
    #[must_use]
    pub fn genesis() -> Self {
        ChainState {
            version: CHAIN_STATE_VERSION,
            current: None,
            checkpoint: None,
            live: None,
            seq: 0,
        }
    }

    /// Advance `current` to `artifact` (CAS on the caller's observed seq;
    /// `None` = "I observed an empty chain"). The common call for BOTH
    /// promotion and rollback — a bad batch is a repoint, not a revert.
    ///
    /// # Errors
    ///
    /// [`ChainError::MalformedArtifactHash`] if the address is not 64
    /// lowercase hex; [`ChainError::CasMismatch`] if `current`'s seq is not
    /// `expected_current_seq` (re-observe and retry).
    pub fn advance(
        mut self,
        expected_current_seq: Option<u64>,
        artifact: &str,
        at_unix: u64,
        note: impl Into<String>,
    ) -> Result<ChainState, ChainError> {
        if let Err(why) = validate_artifact_hash(artifact) {
            return Err(ChainError::MalformedArtifactHash {
                hash: artifact.to_owned(),
                why,
            });
        }
        let actual = self.current.as_ref().map(|h| h.seq);
        if actual != expected_current_seq {
            return Err(ChainError::CasMismatch {
                slot: HeadSlot::Current,
                expected: expected_current_seq,
                actual,
            });
        }
        let seq = self.seq + 1;
        let parent = self.current.as_ref().map(|h| h.artifact.clone());
        self.current = Some(ChainHead {
            artifact: artifact.to_owned(),
            seq,
            at_unix,
            note: note.into(),
            parent,
        });
        self.seq = seq;
        Ok(self)
    }

    /// Install `artifact` as the durable checkpoint (CAS on the checkpoint
    /// slot; `None` = first checkpoint). The common call passes
    /// `current.artifact` — promoting the serving head to durable — but an
    /// operator may checkpoint any committed artifact.
    ///
    /// # Errors
    ///
    /// [`ChainError::MalformedArtifactHash`] / [`ChainError::CasMismatch`]
    /// as [`advance`], on the checkpoint slot.
    pub fn install_checkpoint(
        mut self,
        expected_checkpoint_seq: Option<u64>,
        artifact: &str,
        at_unix: u64,
        note: impl Into<String>,
    ) -> Result<ChainState, ChainError> {
        if let Err(why) = validate_artifact_hash(artifact) {
            return Err(ChainError::MalformedArtifactHash {
                hash: artifact.to_owned(),
                why,
            });
        }
        let actual = self.checkpoint.as_ref().map(|h| h.seq);
        if actual != expected_checkpoint_seq {
            return Err(ChainError::CasMismatch {
                slot: HeadSlot::Checkpoint,
                expected: expected_checkpoint_seq,
                actual,
            });
        }
        let seq = self.seq + 1;
        let parent = self.current.as_ref().map(|h| h.artifact.clone());
        self.checkpoint = Some(ChainHead {
            artifact: artifact.to_owned(),
            seq,
            at_unix,
            note: note.into(),
            parent,
        });
        self.seq = seq;
        Ok(self)
    }

    /// Build/replace the `live` child of the durable checkpoint WITHOUT
    /// moving a head (reef P5's serve continuity). Requires a checkpoint:
    /// an experiment with no durable parent has nothing to fall back to.
    /// `live` itself is replaced unconditionally — it is the experimental
    /// lane, and its seq still comes from the monotone allocation counter.
    ///
    /// # Errors
    ///
    /// [`ChainError::MalformedArtifactHash`] for a bad address;
    /// [`ChainError::NoCheckpoint`] when no durable checkpoint exists.
    pub fn prepare_live(
        mut self,
        candidate_artifact: &str,
        at_unix: u64,
        note: impl Into<String>,
    ) -> Result<ChainState, ChainError> {
        if let Err(why) = validate_artifact_hash(candidate_artifact) {
            return Err(ChainError::MalformedArtifactHash {
                hash: candidate_artifact.to_owned(),
                why,
            });
        }
        let checkpoint = self.checkpoint.as_ref().ok_or(ChainError::NoCheckpoint {
            action: "prepare_live",
        })?;
        let parent = checkpoint.artifact.clone();
        let seq = self.seq + 1;
        self.live = Some(ChainHead {
            artifact: candidate_artifact.to_owned(),
            seq,
            at_unix,
            note: note.into(),
            parent: Some(parent),
        });
        self.seq = seq;
        Ok(self)
    }

    /// Repoint `current` back to the durable checkpoint's artifact — the
    /// rollback shape (a CAS commit, not a revert of history).
    ///
    /// # Errors
    ///
    /// [`ChainError::NoCheckpoint`] when no durable checkpoint exists;
    /// [`advance`]'s errors otherwise (the rollback IS an advance).
    pub fn rollback_to_checkpoint(
        self,
        expected_current_seq: Option<u64>,
        at_unix: u64,
        note: impl Into<String>,
    ) -> Result<ChainState, ChainError> {
        let artifact = self
            .checkpoint
            .as_ref()
            .ok_or(ChainError::NoCheckpoint {
                action: "rollback_to_checkpoint",
            })?
            .artifact
            .clone();
        self.advance(expected_current_seq, &artifact, at_unix, note)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus_proposal(bench: BenchShape) -> StructuralProposal {
        StructuralProposal {
            title: "add manual_range_contains fixture".to_owned(),
            payload: ProposalPayload::CorpusRule {
                domain: "clippy".to_owned(),
                rule_id: "manual_range_contains".to_owned(),
                bench,
                body: "{\"rule_id\":\"manual_range_contains\"}".to_owned(),
            },
        }
    }

    fn ctx() -> AdmissionCtx<'static> {
        AdmissionCtx {
            known_knobs: &["self_evolve_enabled", "rayon_threads"],
        }
    }

    // ── review classes (reef P3) ────────────────────────────────────────

    #[test]
    fn review_class_is_keyed_on_kind_privilege() {
        let expected = [
            (ProposalKind::CorpusRule, ReviewClass::AutoServable),
            (ProposalKind::Fixture, ReviewClass::AlwaysReviewed),
            (ProposalKind::ConfigKnob, ReviewClass::AlwaysReviewed),
            (ProposalKind::ActiveSetMarker, ReviewClass::AlwaysReviewed),
            (ProposalKind::Playbook, ReviewClass::AlwaysReviewed),
        ];
        assert_eq!(
            ProposalKind::ALL.len(),
            expected.len(),
            "a kind without a class row"
        );
        for (kind, class) in expected {
            assert_eq!(review_class(kind), class, "kind {}", kind.label());
        }
    }

    #[test]
    fn kind_is_derived_from_payload_not_carried() {
        assert_eq!(
            corpus_proposal(BenchShape::NotBenchable).kind(),
            ProposalKind::CorpusRule
        );
        let fixture = StructuralProposal {
            title: "t".to_owned(),
            payload: ProposalPayload::Fixture {
                rel_path: "tests/fixtures/x.rs".to_owned(),
                body: "fn x() {}".to_owned(),
            },
        };
        assert_eq!(fixture.kind(), ProposalKind::Fixture);
    }

    // ── corpus rule gate ────────────────────────────────────────────────

    #[test]
    fn corpus_rule_admitted_with_bench_spec_or_not_benchable() {
        assert!(admit(
            &corpus_proposal(BenchShape::BenchSpec("matvec_g1".to_owned())),
            &ctx()
        )
        .is_ok());
        assert!(admit(&corpus_proposal(BenchShape::NotBenchable), &ctx()).is_ok());
    }

    #[test]
    fn corpus_rule_without_bench_shape_refused() {
        assert_eq!(
            admit(&corpus_proposal(BenchShape::Absent), &ctx()),
            Err(Refusal::BenchShapeAbsent)
        );
    }

    #[test]
    fn corpus_rule_unknown_domain_refused() {
        let mut p = corpus_proposal(BenchShape::NotBenchable);
        if let ProposalPayload::CorpusRule { domain, .. } = &mut p.payload {
            *domain = "rustc_errors".to_owned();
        }
        assert_eq!(
            admit(&p, &ctx()),
            Err(Refusal::UnknownDomain {
                domain: "rustc_errors".to_owned()
            })
        );
    }

    #[test]
    fn corpus_rule_malformed_rule_id_refused() {
        for bad in ["has space", "a/b", ""] {
            let mut p = corpus_proposal(BenchShape::NotBenchable);
            if let ProposalPayload::CorpusRule { rule_id, .. } = &mut p.payload {
                *rule_id = bad.to_owned();
            }
            assert!(admit(&p, &ctx()).is_err(), "rule id {bad:?} must refuse");
        }
    }

    // ── config knob gate ────────────────────────────────────────────────

    #[test]
    fn config_knob_unknown_refused_known_admitted() {
        let mk = |knob: &str| StructuralProposal {
            title: "tune knob".to_owned(),
            payload: ProposalPayload::ConfigKnob {
                knob: knob.to_owned(),
                value_json: "true".to_owned(),
            },
        };
        assert_eq!(
            admit(&mk("nonexistent_knob"), &ctx()),
            Err(Refusal::UnknownConfigKnob {
                knob: "nonexistent_knob".to_owned()
            })
        );
        assert!(admit(&mk("self_evolve_enabled"), &ctx()).is_ok());
    }

    #[test]
    fn config_knob_malformed_json_refused() {
        let p = StructuralProposal {
            title: "t".to_owned(),
            payload: ProposalPayload::ConfigKnob {
                knob: "rayon_threads".to_owned(),
                value_json: "{not json".to_owned(),
            },
        };
        assert!(matches!(
            admit(&p, &ctx()),
            Err(Refusal::MalformedJson { .. })
        ));
    }

    // ── active-set marker gate ──────────────────────────────────────────

    #[test]
    fn active_set_marker_edits_validate() {
        let ok = StructuralProposal {
            title: "demote zombie".to_owned(),
            payload: ProposalPayload::ActiveSetMarker {
                edit: ActiveSetEdit::Demote {
                    domain: "perf".to_owned(),
                    rule_id: "zombie_rule".to_owned(),
                },
            },
        };
        assert!(admit(&ok, &ctx()).is_ok());

        let bad_domain = StructuralProposal {
            title: "t".to_owned(),
            payload: ProposalPayload::ActiveSetMarker {
                edit: ActiveSetEdit::SetCap {
                    domain: "nope".to_owned(),
                    cap: 4,
                },
            },
        };
        assert_eq!(
            admit(&bad_domain, &ctx()),
            Err(Refusal::UnknownDomain {
                domain: "nope".to_owned()
            })
        );
    }

    // ── rel-path + body gates ───────────────────────────────────────────

    fn playbook(path: &str) -> StructuralProposal {
        StructuralProposal {
            title: "t".to_owned(),
            payload: ProposalPayload::Playbook {
                rel_path: path.to_owned(),
                body: "echo ok".to_owned(),
            },
        }
    }

    #[test]
    fn rel_path_shapes_refuse() {
        for (path, reason) in [
            ("/etc/passwd", "absolute path"),
            ("a/../b", "parent escape"),
            ("a\\b", "backslash (canonical form is forward slashes)"),
            ("a//b", "empty component"),
            ("C:evil", "windows drive letter"),
            ("a/./b", "redundant component"),
            ("", "empty"),
        ] {
            let got = admit(&playbook(path), &ctx());
            assert!(got.is_err(), "path {path:?} must refuse");
            if let Err(Refusal::MalformedRelPath {
                reason: got_reason, ..
            }) = got
            {
                assert_eq!(got_reason, reason, "path {path:?}");
            }
        }
    }

    #[test]
    fn payload_too_large_refused() {
        let big = "x".repeat(MAX_PAYLOAD_BYTES + 1);
        let p = StructuralProposal {
            title: "t".to_owned(),
            payload: ProposalPayload::Playbook {
                rel_path: "a.sh".to_owned(),
                body: big,
            },
        };
        assert!(matches!(
            admit(&p, &ctx()),
            Err(Refusal::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn empty_and_oversized_titles_refused() {
        let mut p = playbook("a.sh");
        p.title = "   ".to_owned();
        assert_eq!(admit(&p, &ctx()), Err(Refusal::EmptyTitle));
        p.title = "x".repeat(MAX_TITLE_BYTES + 1);
        assert!(matches!(
            admit(&p, &ctx()),
            Err(Refusal::TitleTooLong { .. })
        ));
    }

    // ── secret shapes (reef P2) ─────────────────────────────────────────

    #[test]
    fn secret_scan_fires_on_each_shape() {
        let samples: &[(&str, &str)] = &[
            ("openai_key", "carry sk-abc123XYZdef456GHI789 in config"),
            (
                "github_pat_classic",
                "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890",
            ),
            (
                "github_pat_finegrained",
                // Real-shaped: the scanner floor is parity with gitleaks'
                // `github_pat_\w{82}` (verdict R2 raised it from 22), so
                // the fixture must be a real-length token.
                "token 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 ok",
            ),
            ("aws_access_key_id", "key AKIAIOSFODNN7EXAMPLE rotated"),
            (
                "slack_token",
                "xoxb-123456789012-1234567890123-abcdefghijklmnop",
            ),
            ("slack_token", "xoxp-1234567890-1234567890123-abcdefabcdef"),
            (
                "google_api_key",
                "AIzaSyA1234567890abcdefghijklmnopqrstuvwx",
            ),
            ("pem_private_key", "-----BEGIN RSA PRIVATE KEY-----\nMIIEpA"),
        ];
        for (shape, text) in samples {
            let hits = scan_secret_shapes(text);
            assert_eq!(hits.len(), 1, "shape {shape}: {text:?}");
            assert_eq!(hits[0].shape, *shape, "text {text:?}");
        }
    }

    #[test]
    fn secret_scan_near_misses_stay_silent() {
        for text in [
            "task-runner and risk-assessment do not fire", // "sk-" inside words
            "sk-short",                                    // boundary ok, run too short
            "ghp_only_nine", // word class, but ghp_ wants alnum AND run is short
            "akiaiosfodnn7example lowercase", // AWS is uppercase-only
            "-----BEGIN CERTIFICATE-----", // cert, not a private key
            "see https://example.com/docs", // plain prose
        ] {
            assert!(
                scan_secret_shapes(text).is_empty(),
                "near-miss fired: {text:?}"
            );
        }
    }

    #[test]
    fn secret_in_payload_body_refuses_admission() {
        let p = StructuralProposal {
            title: "t".to_owned(),
            payload: ProposalPayload::Fixture {
                rel_path: "tests/fixtures/leak.rs".to_owned(),
                body: "const KEY: &str = \"AKIAIOSFODNN7EXAMPLE\";".to_owned(),
            },
        };
        assert!(matches!(
            admit(&p, &ctx()),
            Err(Refusal::SecretShapedText { .. })
        ));
    }

    // ── content addressing ──────────────────────────────────────────────

    #[test]
    fn canonical_bytes_deterministic_and_hashes_distinguish() {
        let a = corpus_proposal(BenchShape::NotBenchable);
        let mut b = corpus_proposal(BenchShape::NotBenchable);
        if let ProposalPayload::CorpusRule { rule_id, .. } = &mut b.payload {
            *rule_id = "other_rule".to_owned();
        }
        let ha = artifact_hash(&a).unwrap();
        let ha2 = artifact_hash(&a).unwrap();
        let hb = artifact_hash(&b).unwrap();
        assert_eq!(ha, ha2, "same proposal -> same address");
        assert_ne!(ha, hb, "different proposals -> different addresses");
        assert_eq!(validate_artifact_hash(&ha), Ok(()));
    }

    #[test]
    fn malformed_artifact_hashes_refused() {
        let upper = "A".repeat(64);
        let not_hex = "g".repeat(64);
        for bad in ["", "abc", upper.as_str(), not_hex.as_str()] {
            assert!(
                validate_artifact_hash(bad).is_err(),
                "hash {bad:?} must refuse"
            );
        }
    }

    // ── chain state machine (reef P5) ───────────────────────────────────

    const A: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const B: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    #[test]
    fn chain_advance_cas_and_seq_monotone() {
        let s = ChainState::genesis();
        assert_eq!(s.seq, 0);
        let s = s.advance(None, A, 100, "first").unwrap();
        assert_eq!(s.current.as_ref().unwrap().seq, 1);
        assert_eq!(s.current.as_ref().unwrap().parent, None);
        let s = s.advance(Some(1), B, 101, "second").unwrap();
        assert_eq!(s.current.as_ref().unwrap().seq, 2);
        assert_eq!(s.current.as_ref().unwrap().parent.as_deref(), Some(A));
        assert_eq!(s.seq, 2);
    }

    #[test]
    fn chain_cas_mismatch_names_slot_and_seqs() {
        let s = ChainState::genesis()
            .advance(None, A, 100, "first")
            .unwrap();
        let err = s
            .clone()
            .advance(None, B, 101, "stale observer")
            .unwrap_err();
        assert_eq!(
            err,
            ChainError::CasMismatch {
                slot: HeadSlot::Current,
                expected: None,
                actual: Some(1)
            }
        );
        let err2 = s.advance(Some(99), B, 101, "wrong seq").unwrap_err();
        assert!(matches!(
            err2,
            ChainError::CasMismatch {
                expected: Some(99),
                ..
            }
        ));
    }

    #[test]
    fn chain_checkpoint_cas_and_promote() {
        let s = ChainState::genesis()
            .advance(None, A, 100, "first")
            .unwrap();
        let s = s.install_checkpoint(None, A, 110, "durable").unwrap();
        let cp = s.checkpoint.as_ref().unwrap();
        assert_eq!(cp.artifact, A);
        assert_eq!(cp.seq, 2);
        assert_eq!(cp.parent.as_deref(), Some(A));
        // checkpoint CAS refuses a stale observer
        let err = s
            .clone()
            .install_checkpoint(None, B, 120, "stale")
            .unwrap_err();
        assert!(matches!(
            err,
            ChainError::CasMismatch {
                slot: HeadSlot::Checkpoint,
                ..
            }
        ));
    }

    #[test]
    fn prepare_live_builds_child_without_moving_heads() {
        let s = ChainState::genesis()
            .advance(None, A, 100, "first")
            .unwrap();
        // no checkpoint yet -> refused
        assert_eq!(
            s.clone().prepare_live(B, 120, "candidate"),
            Err(ChainError::NoCheckpoint {
                action: "prepare_live"
            })
        );
        let s = s.install_checkpoint(None, A, 110, "durable").unwrap();
        let (cur_seq, cp_seq) = (
            s.current.as_ref().unwrap().seq,
            s.checkpoint.as_ref().unwrap().seq,
        );
        let s = s.prepare_live(B, 120, "candidate").unwrap();
        let live = s.live.as_ref().unwrap();
        assert_eq!(live.artifact, B);
        assert_eq!(
            live.parent.as_deref(),
            Some(A),
            "live is a child of the checkpoint"
        );
        assert_eq!(s.current.as_ref().unwrap().seq, cur_seq, "current unmoved");
        assert_eq!(
            s.checkpoint.as_ref().unwrap().seq,
            cp_seq,
            "checkpoint unmoved"
        );
        assert_eq!(s.seq, 3, "allocation counter still monotone");
    }

    #[test]
    fn rollback_repoints_current_to_checkpoint() {
        let s = ChainState::genesis()
            .advance(None, A, 100, "first")
            .unwrap();
        assert_eq!(
            s.clone().rollback_to_checkpoint(None, 130, "rollback"),
            Err(ChainError::NoCheckpoint {
                action: "rollback_to_checkpoint"
            })
        );
        let s = s.install_checkpoint(None, A, 110, "durable").unwrap();
        let s = s.advance(Some(1), B, 120, "bad batch").unwrap();
        let s = s
            .rollback_to_checkpoint(Some(3), 130, "repoint to durable")
            .unwrap();
        assert_eq!(
            s.current.as_ref().unwrap().artifact,
            A,
            "current repointed to the checkpoint"
        );
        assert_eq!(s.current.as_ref().unwrap().seq, 4, "rollback is a commit with a fresh allocation (1 advance, 2 checkpoint, 3 bad batch, 4 repoint)");
        // history intact: checkpoint still A
        assert_eq!(s.checkpoint.as_ref().unwrap().artifact, A);
    }

    #[test]
    fn heads_json_round_trip() {
        let s = ChainState::genesis()
            .advance(None, A, 100, "first")
            .unwrap()
            .install_checkpoint(None, A, 110, "durable")
            .unwrap()
            .prepare_live(B, 120, "candidate")
            .unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let back: ChainState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
