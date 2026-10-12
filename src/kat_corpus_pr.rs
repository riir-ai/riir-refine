//! The devnet→mainnet corpus PR client half (riir-dapps Plan 050, S1.11)
//! — the refine-integrated PR builder + submitter.
//!
//! The git model, one line: devnet is the dev branch, mainnet is main, a
//! corpus PR is an owner-attested batch of mined pattern pairs carried
//! from one ledger to the other. This module is the CLIENT half that
//! lives in the healer: draft (auto-rebase + chunk + sign) + submit
//! (envelope, then chunks in order), with the per-pair bookkeeping
//! under `.refine/net/<dst>/corpuspr/<src>/`.
//!
//! ## The mirror law (the boundary allowlist)
//!
//! riir-refine may NOT depend on riir-dapps. The corpus-PR wire — the
//! signing domain, the leaf/chunk/top-root derivations, the record
//! identity, the caps, and the two route bodies — mirrors riir-dapps
//! `src/kat/corpuspr.rs` BYTE-FOR-BYTE, pinned by the SAME frozen golden
//! vectors that pin the dapps lib and the Python builder
//! (`build_corpus_pr.py --selftest`): a THREE-WAY pin. A drift on any
//! side reds that side's vectors; that IS the drift detector (the
//! `kat_lease_client` wire-mirror precedent).
//!
//! ## The S1.0 rebase (the owner round-2 amendment)
//!
//! Rebase is the builder's FIRST step: read the destination's serving
//! lease (an ordinary PUBLIC read — T10/T2b gate the PR WRITE, never
//! reads), key every source row by `record_id` + `blake3(fix)`, drop
//! what the destination already carries (identical) or supersedes (same
//! record, different fix — MAINNET WINS), print the disclosure line,
//! and refuse loudly when nothing is new. Rebase is an optimization +
//! hygiene layer, NEVER the correctness defense — the TOCTOU window
//! between rebase and ingest closes SERVER-side (exactly-once + the
//! three-arm + the recomputed-root check), exactly as a git rebase does
//! not replace merge-time conflict resolution.
//!
//! ## The S1.11 gates (every PR write)
//!
//! - **T10 attested-intent**: the destination must READ as mainnet (its
//!   URL names mainnet AND its `GET /` service-info attests `env:
//!   mainnet` + a ledger id — `kat_net_intent`'s own fail-closed judge);
//!   the source is judged by its own URL's intent and must serve its
//!   ledger id (the T2b binding).
//! - **T2b stamps**: the per-pair state file records both hosts + both
//!   ledger ids at the last PR; ANY drift (a devnet reset, a mainnet
//!   reset, a copied dir) refuses loudly with the remedy — never
//!   auto-rebirth (the adopt-state law). A reset is a fresh namespace:
//!   the server's per-pair cursor restarts with it.
//! - **Net pins**: both dials pass `ensure_net_allowed` — a pinned
//!   build never dials outside its pin, even with operator URLs.
//!
//! ## Layout (per-net, the T1 law)
//!
//! ```text
//! .refine/net/<dst-key>/corpuspr/<src-key>/
//!   state.json          the T2b pair stamp + the last registered pr_seq
//!   pr/<seq>/envelope.json, chunk_NNNN.json, manifest.json
//!   submissions.jsonl   the append-only submit log
//! ```
//!
//! The PR-attestation key is the DEDICATED class
//! (`KAT_CORPUSPR_PUBKEY_HEX` on the worker); the key FILE here is the
//! `kat_corpus_sign --gen-corpuspr` 64-hex-seed format. The lane ships
//! dark until the owner mints the key — a submit against an unconfigured
//! worker fails at its 503 gate, and the `--key-file`-less gauge mode
//! exists precisely for the dark era (the S1.0 divergence gauge).

// The wire derivations mirror dapps byte-for-byte: their `usize as u32`
// LE length embeddings are the CONTRACT (fields are validated against
// the caps BEFORE any root is computed, so the cast is bounded by
// construction — the `kat_lease_client` mirror allow precedent).
#![allow(unknown_lints)]
#![allow(clippy::cast_possible_truncation)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use blake3::Hasher as B3;
use ed25519_dalek::{Signer as _, SigningKey};
use riir_kat::kat_net_intent::{
    ServiceAttestation, ServiceNet, ensure_attested_intent, intended_net_for_url,
};

// ── the wire mirror (golden-pinned to riir-dapps src/kat/corpuspr.rs) ──────

/// The PR signing domain — born network-bound (the T12 splice law):
/// every signed message embeds BOTH ledger ids as 32 raw bytes.
pub const CORPUSPR_SIGNING_DOMAIN: &[u8] = b"riir-kat-corpuspr:v1:";
/// The per-row leaf prefix: `BLAKE3(prefix ‖ le32 len ‖ field …)` over
/// `(lint_key, shape, fix)` — length-prefixed (no forgeable separator).
const CORPUSPR_LEAF_PREFIX: &[u8] = b"kat-corpuspr-leaf-v1";
/// The chunk-root prefix — ordered leaves, n-embedded.
const CORPUSPR_CHUNK_PREFIX: &[u8] = b"kat-corpuspr-chunk-v1";
/// The top-root prefix — the envelope's commitment over the chunk roots.
const CORPUSPR_TOP_PREFIX: &[u8] = b"kat-corpuspr-top-v1";

/// Rows per chunk — the worker's hard cap (the lib's `MAX_ROWS_PER_CHUNK`).
pub const MAX_ROWS_PER_CHUNK: usize = 4_096;
/// Chunks per PR — the envelope's hard cap.
pub const MAX_CHUNKS_PER_PR: usize = 64;
/// Lint-key length cap (the lib's `MAX_LINT_KEY_BYTES`).
pub const MAX_LINT_KEY_BYTES: usize = 256;
/// Shape/fix length cap (the lib's `MAX_FIELD_BYTES`).
pub const MAX_FIELD_BYTES: usize = 16 * 1024;
/// The only licence grant accepted (the carried-corpus law).
pub const CORPUSPR_LICENCE: &str = "MIT";
/// The builder's chunk size — HTTP-comfort bodies under the lib's cap
/// (the `build_corpus_pr.py` `CHUNK_ROWS` precedent).
pub const CHUNK_ROWS: usize = 1_024;

/// Lower-hex of a byte slice (roots, ids, signatures — display + wire).
fn hex_bytes(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(b.len() * 2);
    for x in b {
        out.push(HEX[(x >> 4) as usize] as char);
        out.push(HEX[(x & 0xf) as usize] as char);
    }
    out
}

/// Parse exactly `N * 2` hex chars into `[u8; N]` (ids, seeds, sigs).
fn unhex_fixed<const N: usize>(s: &str) -> Option<[u8; N]> {
    let b = s.as_bytes();
    if b.len() != N * 2 || !b.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        let hi = (b[2 * i] as char).to_digit(16)?;
        let lo = (b[2 * i + 1] as char).to_digit(16)?;
        // Hex digits are ≤ 15, so the nibble pair is ≤ 255 by construction.
        *o = u8::try_from((hi << 4) | lo).expect("hex nibbles fit a byte");
    }
    Some(out)
}

/// One PR row — a mined pattern pair (the lease `PatternEntry` shape:
/// `lint_key` = the rule id, `shape` = the buggy code, `fix` = the fixed
/// code), plus the emitting-domain label (the PACK's domain — entries
/// carry no per-entry field on the lease wire).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrRow {
    pub lint_key: String,
    pub shape: String,
    pub fix: String,
    pub domain: Option<String>,
}

impl PrRow {
    /// The row's leaf hash — the chunk-root unit. Covers exactly
    /// `(lint_key, shape, fix)`, length-prefixed; the domain label is
    /// NOT hashed (the lib's law — the builder's Python mirror hashes
    /// the same three).
    #[must_use]
    pub fn leaf_hash(&self) -> [u8; 32] {
        let mut h = B3::new();
        h.update(CORPUSPR_LEAF_PREFIX);
        for f in [&self.lint_key, &self.shape, &self.fix] {
            h.update(&(f.len() as u32).to_le_bytes());
            h.update(f.as_bytes());
        }
        h.finalize().into()
    }

    /// `shape_id` = the first 12 hex chars of `blake3(shape)` — one
    /// derivation for PR rows and native rows (the shared keyspace).
    #[must_use]
    pub fn shape_id(&self) -> String {
        blake3::hash(self.shape.as_bytes()).to_hex().as_str()[..12].to_string()
    }

    /// `record_id` = `blake3(lint_key ‖ 0x00 ‖ shape_id)` — the store
    /// identity the rebase keys on (the duplicate barrier).
    #[must_use]
    pub fn record_id(&self) -> [u8; 32] {
        let mut h = B3::new();
        h.update(self.lint_key.as_bytes());
        h.update(&[0u8]);
        h.update(self.shape_id().as_bytes());
        h.finalize().into()
    }

    /// The row caps (the lib's `validate` — fail HERE, at the builder,
    /// not at ingest).
    ///
    /// # Errors
    ///
    /// A human-readable refusal naming the field and its bound.
    pub fn validate(&self) -> Result<(), String> {
        if self.lint_key.is_empty() || self.lint_key.len() > MAX_LINT_KEY_BYTES {
            return Err(format!(
                "corpus-pr row lint_key length {} outside 1..={MAX_LINT_KEY_BYTES}",
                self.lint_key.len()
            ));
        }
        if self.shape.is_empty() || self.shape.len() > MAX_FIELD_BYTES {
            return Err(format!(
                "corpus-pr row shape length {} outside 1..={MAX_FIELD_BYTES}",
                self.shape.len()
            ));
        }
        if self.fix.is_empty() || self.fix.len() > MAX_FIELD_BYTES {
            return Err(format!(
                "corpus-pr row fix length {} outside 1..={MAX_FIELD_BYTES}",
                self.fix.len()
            ));
        }
        if let Some(d) = self.domain.as_deref() {
            if d.is_empty() || d.len() > MAX_LINT_KEY_BYTES {
                return Err("corpus-pr row domain absent-or-over-cap".into());
            }
        }
        Ok(())
    }
}

/// The chunk root — `BLAKE3(prefix ‖ le32 n ‖ ordered leaves)`. Ordered:
/// the chunk's index in the PR is its position in the envelope's root
/// list, so the fold must never sort.
#[must_use]
pub fn chunk_root(rows: &[PrRow]) -> [u8; 32] {
    let mut h = B3::new();
    h.update(CORPUSPR_CHUNK_PREFIX);
    h.update(&(rows.len() as u32).to_le_bytes());
    for r in rows {
        h.update(&r.leaf_hash());
    }
    h.finalize().into()
}

/// The top root — `BLAKE3(prefix ‖ le32 n ‖ ordered chunk roots)`. The
/// envelope's attested commitment.
#[must_use]
pub fn top_root(chunk_roots: &[[u8; 32]]) -> [u8; 32] {
    let mut h = B3::new();
    h.update(CORPUSPR_TOP_PREFIX);
    h.update(&(chunk_roots.len() as u32).to_le_bytes());
    for r in chunk_roots {
        h.update(r);
    }
    h.finalize().into()
}

/// The ENVELOPE signing message (S1.1):
/// `domain ‖ src(32) ‖ dst(32) ‖ pr_seq le64 ‖ top_root(32)`.
#[must_use]
pub fn envelope_message(
    src_ledger_id: &[u8; 32],
    dst_ledger_id: &[u8; 32],
    pr_seq: u64,
    top_root: &[u8; 32],
) -> Vec<u8> {
    let mut m = Vec::with_capacity(CORPUSPR_SIGNING_DOMAIN.len() + 104);
    m.extend_from_slice(CORPUSPR_SIGNING_DOMAIN);
    m.extend_from_slice(src_ledger_id);
    m.extend_from_slice(dst_ledger_id);
    m.extend_from_slice(&pr_seq.to_le_bytes());
    m.extend_from_slice(top_root);
    m
}

/// The CHUNK signing message (S1.1):
/// `domain ‖ src(32) ‖ dst(32) ‖ pr_seq le64 ‖ chunk_index le32 ‖
/// chunk_root(32)`. The row-set bytes ride INSIDE the root (the
/// commitment law — the signature is constant-size and binds the rows
/// transitively).
#[must_use]
pub fn chunk_message(
    src_ledger_id: &[u8; 32],
    dst_ledger_id: &[u8; 32],
    pr_seq: u64,
    chunk_index: u32,
    chunk_root: &[u8; 32],
) -> Vec<u8> {
    let mut m = Vec::with_capacity(CORPUSPR_SIGNING_DOMAIN.len() + 108);
    m.extend_from_slice(CORPUSPR_SIGNING_DOMAIN);
    m.extend_from_slice(src_ledger_id);
    m.extend_from_slice(dst_ledger_id);
    m.extend_from_slice(&pr_seq.to_le_bytes());
    m.extend_from_slice(&chunk_index.to_le_bytes());
    m.extend_from_slice(chunk_root);
    m
}

// ── the S1.0 rebase ────────────────────────────────────────────────────────

/// The S1.0 rebase outcome.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RebaseOutcome {
    pub kept: Vec<PrRow>,
    pub duplicates: usize,
    pub superseded: usize,
}

impl RebaseOutcome {
    /// The S1.0 disclosure line, verbatim (the Issue-149 disclosure
    /// spirit — never a silent drop).
    #[must_use]
    pub fn disclosure(&self, src_total: usize) -> String {
        format!(
            "rebase: kept {} of {src_total} rows ({} duplicates, {} superseded)",
            self.kept.len(),
            self.duplicates,
            self.superseded
        )
    }
}

fn fix_hash(fix: &str) -> [u8; 32] {
    *blake3::hash(fix.as_bytes()).as_bytes()
}

/// The S1.0 rebase — keep only what the destination does not carry.
/// Duplicate = same `record_id` AND same `blake3(fix)`; superseded =
/// same `record_id`, different fix (MAINNET WINS — the PR never
/// overwrites). Order-preserving (the patch keeps the source's canonical
/// order); the destination's FIRST fix at a record id wins its key (the
/// `build_corpus_pr.py` `setdefault` law).
#[must_use]
pub fn rebase(src_rows: &[PrRow], dst_rows: &[PrRow]) -> RebaseOutcome {
    let mut dst_by_rid: HashMap<[u8; 32], [u8; 32]> = HashMap::with_capacity(dst_rows.len());
    for r in dst_rows {
        dst_by_rid
            .entry(r.record_id())
            .or_insert_with(|| fix_hash(&r.fix));
    }
    let mut out = RebaseOutcome::default();
    for r in src_rows {
        match dst_by_rid.get(&r.record_id()) {
            Some(h) if *h == fix_hash(&r.fix) => out.duplicates += 1,
            Some(_) => out.superseded += 1,
            None => out.kept.push(r.clone()),
        }
    }
    out
}

// ── the chunk plan + the draft ─────────────────────────────────────────────

/// Split the kept rows into wire chunks (`CHUNK_ROWS` per chunk, never
/// above the envelope's chunk cap).
///
/// # Errors
///
/// The empty-carry refusal, or the too-many-chunks refusal naming the
/// split remedy.
pub fn chunk_plan(rows: Vec<PrRow>) -> Result<Vec<Vec<PrRow>>, String> {
    if rows.is_empty() {
        return Err("the rebase kept no rows — nothing to carry".into());
    }
    // Consume the vec (needless_pass_by_value): take() over the by-ref
    // iterator groups without cloning.
    let mut it = rows.into_iter();
    let mut chunks = Vec::new();
    loop {
        let c: Vec<PrRow> = it.by_ref().take(CHUNK_ROWS).collect();
        if c.is_empty() {
            break;
        }
        chunks.push(c);
    }
    if chunks.len() > MAX_CHUNKS_PER_PR {
        return Err(format!(
            "the carry needs {} chunks — the envelope caps at {MAX_CHUNKS_PER_PR} \
             ({} rows at {CHUNK_ROWS}/chunk); split the carry across PRs",
            chunks.len(),
            MAX_CHUNKS_PER_PR * CHUNK_ROWS
        ));
    }
    Ok(chunks)
}

/// The unsigned PR draft — the chunk plan + its deterministic roots.
#[derive(Clone, Debug)]
pub struct PrDraft {
    pub pr_seq: u64,
    pub chunks: Vec<Vec<PrRow>>,
    pub chunk_roots: Vec<[u8; 32]>,
    pub top_root: [u8; 32],
    pub total_rows: u64,
}

impl PrDraft {
    /// Build the draft from a chunk plan (roots recomputed, never
    /// trusted from outside).
    ///
    /// # Errors
    ///
    /// The `pr_seq` and chunk bounds (empty / over-cap).
    pub fn build(pr_seq: u64, chunks: Vec<Vec<PrRow>>) -> Result<Self, String> {
        if pr_seq == 0 {
            return Err("pr_seq must be ≥ 1 (PRs number from one)".into());
        }
        if chunks.is_empty() || chunks.len() > MAX_CHUNKS_PER_PR {
            return Err(format!(
                "{} chunks outside 1..={MAX_CHUNKS_PER_PR}",
                chunks.len()
            ));
        }
        for c in &chunks {
            if c.is_empty() || c.len() > MAX_ROWS_PER_CHUNK {
                return Err(format!(
                    "a chunk carries {} rows outside 1..={MAX_ROWS_PER_CHUNK}",
                    c.len()
                ));
            }
        }
        let chunk_roots: Vec<[u8; 32]> = chunks.iter().map(|c| chunk_root(c)).collect();
        let total_rows: u64 = chunks.iter().map(|c| c.len() as u64).sum();
        let top_root = top_root(&chunk_roots);
        Ok(Self {
            pr_seq,
            chunks,
            chunk_roots,
            top_root,
            total_rows,
        })
    }
}

// ── the per-pair state (T2b) ───────────────────────────────────────────────

/// The per-pair state file version.
pub const PAIR_STATE_VERSION: u32 = 1;
/// The pair state file name.
pub const PAIR_STATE_FILE: &str = "state.json";

/// The T2b pair stamp — both hosts + both ledger ids at the last PR,
/// plus the last REGISTERED `pr_seq` (the client-local tick of S1.0).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PairState {
    pub version: u32,
    pub src_host: String,
    pub src_ledger_id: String,
    pub dst_host: String,
    pub dst_ledger_id: String,
    pub last_pr_seq: u64,
    pub updated_unix: u64,
}

/// The per-pair directory: `.refine/net/<dst-key>/corpuspr/<src-key>/`
/// (the T1 per-net law; one namespace per `(src, dst)` pair — exactly
/// the server's exactly-once keyspace).
#[must_use]
pub fn pair_dir(root: &Path, dst_url: &str, src_url: &str) -> PathBuf {
    crate::kat_state_stamp::net_state_dir_for_url(root, dst_url)
        .join("corpuspr")
        .join(crate::kat_state_stamp::net_dir_key(src_url))
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// tmp + rename (the `LeaseStore` atomicity precedent; `rename` replaces
/// an existing destination on both Windows and Unix).
fn write_atomic(path: &Path, bytes: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Load the pair state — `None` when the lane is unborn.
///
/// # Errors
///
/// An unreadable or malformed state file refuses loudly (the adopt-state
/// law: never auto-rebirth — the remedy is a manual adjudication).
pub fn load_pair_state(pair_dir: &Path) -> Result<Option<PairState>, String> {
    let path = pair_dir.join(PAIR_STATE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "the corpus-PR state at {} is unreadable ({e}) — adjudicate the file before \
             continuing; never auto-rebirth (the adopt-state law)",
            path.display()
        )
    })?;
    let st: PairState = serde_json::from_str(&text).map_err(|e| {
        format!(
            "the corpus-PR state at {} is malformed ({e}) — inspect it, then remove it if a \
             ledger reset is expected (the per-pair cursor restarts with the new namespace)",
            path.display()
        )
    })?;
    if st.version != PAIR_STATE_VERSION {
        return Err(format!(
            "the corpus-PR state at {} carries version {} (this client defines \
             {PAIR_STATE_VERSION}) — adjudicate the file",
            path.display(),
            st.version
        ));
    }
    Ok(Some(st))
}

fn stamp_host_of(url: &str) -> Result<String, String> {
    crate::kat_state_stamp::stamp_host(url)
        .ok_or_else(|| format!("cannot derive the stamp host for {url}"))
}

/// The T2b judge — ANY drift refuses (a devnet reset, a mainnet reset, a
/// copied dir); the remedy is a manual adjudication of the state file,
/// never an auto-rebirth.
///
/// # Errors
///
/// The specific drift, named, with the remedy line.
pub fn judge_pair_state(
    st: &PairState,
    src_url: &str,
    dst_url: &str,
    src_id: &[u8; 32],
    dst_id: &[u8; 32],
) -> Result<(), String> {
    let want_src = stamp_host_of(src_url)?;
    let want_dst = stamp_host_of(dst_url)?;
    if st.src_host != want_src || st.dst_host != want_dst {
        return Err(format!(
            "the corpus-PR pair state names {} → {} but this run dials {want_src} → \
             {want_dst} — the state file belongs to another lane (a copied dir?)",
            st.src_host, st.dst_host
        ));
    }
    let now_src = hex_bytes(src_id);
    let now_dst = hex_bytes(dst_id);
    if st.src_ledger_id != now_src {
        return Err(format!(
            "the SOURCE ledger id moved since the last PR ({} → {}) — a ledger reset is a \
             fresh namespace: if the reset is expected, remove the pair state and re-run (the \
             new pair restarts at pr_seq 1)",
            &st.src_ledger_id[..8],
            &now_src[..8]
        ));
    }
    if st.dst_ledger_id != now_dst {
        return Err(format!(
            "the DESTINATION ledger id moved since the last PR ({} → {}) — a mainnet reset \
             re-bases every pair: if the reset is expected, remove the pair state and re-run",
            &st.dst_ledger_id[..8],
            &now_dst[..8]
        ));
    }
    Ok(())
}

/// Persist the pair state (atomic tmp+rename).
///
/// # Errors
///
/// IO failures, named with the path.
pub fn save_pair_state(pair_dir: &Path, st: &PairState) -> std::io::Result<()> {
    std::fs::create_dir_all(pair_dir)?;
    write_atomic(
        &pair_dir.join(PAIR_STATE_FILE),
        &serde_json::to_string_pretty(st).unwrap_or_default(),
    )
}

/// Append one event to the pair's submissions log (best-effort — a log
/// failure never blocks the lane, the consent-heartbeat law).
///
/// # Errors
///
/// IO failures opening or writing the append-only log.
pub fn append_event(pair_dir: &Path, event: &serde_json::Value) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(pair_dir.join("submissions.jsonl"))?;
    writeln!(f, "{event}")
}

// ── the key + the signing ──────────────────────────────────────────────────

/// Load the PR-attestation key file (the `kat_corpus_sign
/// --gen-corpuspr` format: 64 hex chars, the raw 32-byte seed).
///
/// # Errors
///
/// The file is unreadable, or not exactly 64 hex chars.
pub fn load_key_seed(path: &Path) -> Result<[u8; 32], String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read the key file {}: {e}", path.display()))?;
    unhex_fixed::<32>(text.trim()).ok_or_else(|| {
        format!(
            "the key file {} must hold 64 hex chars (the 32-byte seed, the \
             --gen-corpuspr format)",
            path.display()
        )
    })
}

/// The gated run context — the resolved + attested pair (`run_corpus_pr`
/// builds it; tests enter the engine with a synthetic one).
#[derive(Clone, Debug)]
pub struct PrCtx {
    pub src_url: String,
    pub dst_url: String,
    pub src_ledger_id: [u8; 32],
    pub dst_ledger_id: [u8; 32],
}

/// The signed, submission-ready PR (wire bodies as JSON values — the
/// POST body is `to_string()`, the draft file is `to_string_pretty()`).
#[derive(Clone, Debug)]
pub struct SignedPr {
    pub pr_seq: u64,
    pub total_rows: u64,
    pub chunk_count: usize,
    pub top_root_hex: String,
    pub envelope: serde_json::Value,
    pub chunks: Vec<serde_json::Value>,
    pub manifest: serde_json::Value,
}

/// Sign the draft — the envelope + every chunk, over the EXACT
/// derivations this module computes (roots recomputed into the messages;
/// the commitment law, constant-size signatures). The submitter is the
/// payee of record, echoed verbatim into the envelope.
///
/// # Errors
///
/// The submitter is not 32 hex chars (the 16-byte account id).
pub fn build_signed_pr(
    key: &SigningKey,
    draft: &PrDraft,
    ctx: &PrCtx,
    submitter_hex: &str,
) -> Result<SignedPr, String> {
    let submitter = submitter_hex.trim();
    if unhex_fixed::<16>(submitter).is_none() {
        return Err(
            "submitter must be 32 hex chars (the 16-byte account id — the payee of record)"
                .into(),
        );
    }
    let src_hex = hex_bytes(&ctx.src_ledger_id);
    let dst_hex = hex_bytes(&ctx.dst_ledger_id);
    let msg = envelope_message(
        &ctx.src_ledger_id,
        &ctx.dst_ledger_id,
        draft.pr_seq,
        &draft.top_root,
    );
    let envelope = serde_json::json!({
        "src_ledger_id": &src_hex,
        "dst_ledger_id": &dst_hex,
        "pr_seq": draft.pr_seq,
        "total_rows": draft.total_rows,
        "submitter": submitter,
        "licence": CORPUSPR_LICENCE,
        "chunk_roots": draft.chunk_roots.iter().map(|r| hex_bytes(r)).collect::<Vec<_>>(),
        "signature_hex": hex_bytes(&key.sign(&msg).to_bytes()),
    });
    let mut chunks_out = Vec::with_capacity(draft.chunks.len());
    for (i, c) in draft.chunks.iter().enumerate() {
        // Infallback by construction (the ≤ 64 cap held in build_draft).
        let idx = u32::try_from(i).map_err(|_| "chunk index exceeds u32")?;
        let root = draft.chunk_roots[i];
        let msg = chunk_message(
            &ctx.src_ledger_id,
            &ctx.dst_ledger_id,
            draft.pr_seq,
            idx,
            &root,
        );
        let rows: Vec<serde_json::Value> = c
            .iter()
            .map(|r| match &r.domain {
                Some(d) => serde_json::json!({
                    "lint_key": r.lint_key, "shape": r.shape, "fix": r.fix, "domain": d
                }),
                None => serde_json::json!({
                    "lint_key": r.lint_key, "shape": r.shape, "fix": r.fix
                }),
            })
            .collect();
        chunks_out.push(serde_json::json!({
            "src_ledger_id": &src_hex,
            "dst_ledger_id": &dst_hex,
            "pr_seq": draft.pr_seq,
            "chunk_index": idx,
            "rows": rows,
            "chunk_root": hex_bytes(&root),
            "signature_hex": hex_bytes(&key.sign(&msg).to_bytes()),
        }));
    }
    let mut submit_order = vec!["envelope.json".to_string()];
    for i in 0..draft.chunks.len() {
        submit_order.push(format!("chunk_{i:04}.json"));
    }
    let manifest = serde_json::json!({
        "src": ctx.src_url,
        "dst": ctx.dst_url,
        "pr_seq": draft.pr_seq,
        "rows": draft.total_rows,
        "chunks": draft.chunks.len(),
        "submitter": submitter,
        "submit_order": submit_order,
    });
    Ok(SignedPr {
        pr_seq: draft.pr_seq,
        total_rows: draft.total_rows,
        chunk_count: draft.chunks.len(),
        top_root_hex: hex_bytes(&draft.top_root),
        envelope,
        chunks: chunks_out,
        manifest,
    })
}

/// Write the draft dir (the `build_corpus_pr.py` layout): `envelope.json`,
/// `chunk_NNNN.json` per chunk, `manifest.json` — each tmp+rename atomic.
///
/// # Errors
///
/// IO failures, named with the path.
pub fn write_submission_files(dir: &Path, signed: &SignedPr) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    write_atomic_json(&dir.join("envelope.json"), &signed.envelope)?;
    for (i, c) in signed.chunks.iter().enumerate() {
        write_atomic_json(&dir.join(format!("chunk_{i:04}.json")), c)?;
    }
    write_atomic_json(&dir.join("manifest.json"), &signed.manifest)
}

fn write_atomic_json(path: &Path, v: &serde_json::Value) -> std::io::Result<()> {
    let body =
        serde_json::to_string_pretty(v).map_err(std::io::Error::other)?;
    write_atomic(path, &body)
}

// ── the S1.11 gates (pure — `kat_net_intent` is feature-free) ──────────────

/// The S1.11 destination gate: the dst URL names mainnet AND its
/// attestation passes the T10 intent check (label `mainnet` + a ledger
/// id — the fail-closed judge).
///
/// # Errors
///
/// The refusal prose (either the URL reads as another net, or the T10
/// judge refuses).
pub fn gate_dst_mainnet(dst_url: &str, att: &ServiceAttestation) -> Result<(), String> {
    match intended_net_for_url(dst_url) {
        ServiceNet::Mainnet => {}
        other => {
            return Err(format!(
                "the PR destination must be the mainnet ledger — {dst_url} reads as {} (the \
                 S1.11 law: the tool refuses when the destination is not mainnet)",
                other.as_label()
            ));
        }
    }
    ensure_attested_intent(ServiceNet::Mainnet, att, dst_url)
}

/// The S1.11 source gate: the src's own T10 intent check + it MUST serve
/// its ledger id (a PR from an unidentifiable ledger cannot be bound —
/// the T2b/T14 law).
///
/// # Errors
///
/// The T10 refusal prose, or the missing-src-ledger-id refusal.
pub fn gate_src(src_url: &str, att: &ServiceAttestation) -> Result<(), String> {
    let intent = intended_net_for_url(src_url);
    ensure_attested_intent(intent, att, src_url)?;
    if att.ledger_id.is_none() {
        return Err(format!(
            "the SOURCE at {src_url} serves no ledger id — a PR source must be \
             ledger-identifiable (the T2b binding; redeploy the worker with its minted id)"
        ));
    }
    Ok(())
}

// ── the wire (kat_transport) ───────────────────────────────────────────────

/// A POST reply — status + body together (the verdict of a PR write
/// rides the BODY: `{"error", "code"}`); `status == 0` marks a
/// transport-level failure (`transport_error` carries the text).
#[derive(Clone, Debug)]
pub struct WireReply {
    pub status: u16,
    pub body: Vec<u8>,
    pub transport_error: Option<String>,
}

/// The PR lane's transport seam — every call takes the target base (one
/// trait object drives BOTH ledgers). The production impl is
/// [`HttpCorpusPrWire`]; tests drive the same trait with a mock (the
/// `LeaseTransport` mock precedent).
#[cfg(feature = "kat_transport")]
pub trait CorpusPrWire {
    /// `GET /` — the service-info read (the T10 attestation source).
    fn service_info(&self, base: &str) -> crate::kat_lease_client::HttpResponse;
    /// `GET /lease` — the manifest read (a PUBLIC read; never gated).
    fn get_lease(&self, base: &str) -> crate::kat_lease_client::HttpResponse;
    /// `GET /segment/<hex>` — one content-addressed segment.
    fn get_segment(
        &self,
        base: &str,
        id: &[u8; 32],
    ) -> crate::kat_lease_client::HttpResponse;
    /// `POST /corpuspr` — register one envelope.
    fn post_envelope(&self, base: &str, body: &str) -> WireReply;
    /// `POST /corpuspr/chunk` — submit one chunk's rows.
    fn post_chunk(&self, base: &str, body: &str) -> WireReply;
}

/// The ureq HTTP impl — the `HttpLeaseTransport` shape: one agent,
/// per-request timeouts, the fleet `User-Agent`, host-level failover
/// (`request_base`/`failover_base` consumed, never re-invented).
#[cfg(feature = "kat_transport")]
pub struct HttpCorpusPrWire {
    agent: ureq::Agent,
}

#[cfg(feature = "kat_transport")]
impl HttpCorpusPrWire {
    /// Build with a per-request timeout (seconds; every request —
    /// connect, headers, AND body read — is covered; a stalled server
    /// cannot outlive the budget).
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

    fn get(&self, base: &str, path: &str) -> crate::kat_lease_client::HttpResponse {
        use crate::kat_lease_client::HttpResponse;
        let mut base = crate::kat_lease_client::request_base(base);
        loop {
            let url = format!("{base}{path}");
            match riir_kat::kat_ua::apply(self.agent.get(&url)).call() {
                Ok(r) => return Self::body(r),
                Err(ureq::Error::Status(304, _)) => return HttpResponse::NotModified,
                Err(ureq::Error::Status(c, _)) => return HttpResponse::Status(c),
                Err(e) => match crate::kat_lease_client::failover_base(&base, &e) {
                    Some(next) => base = next,
                    None => return HttpResponse::Error(e.to_string()),
                },
            }
        }
    }

    fn body(r: ureq::Response) -> crate::kat_lease_client::HttpResponse {
        use crate::kat_lease_client::HttpResponse;
        use std::io::Read as _;
        match r.status() {
            304 => HttpResponse::NotModified,
            200 => {
                let mut buf = Vec::new();
                match r.into_reader().read_to_end(&mut buf) {
                    Ok(_) => HttpResponse::Ok(buf),
                    Err(e) => HttpResponse::Error(e.to_string()),
                }
            }
            c => HttpResponse::Status(c),
        }
    }

    fn reply_from(status: u16, r: ureq::Response) -> WireReply {
        use std::io::Read as _;
        let mut buf = Vec::new();
        let _ = r.into_reader().read_to_end(&mut buf);
        WireReply {
            status,
            body: buf,
            transport_error: None,
        }
    }

    fn reply_body(r: ureq::Response) -> WireReply {
        let status = r.status();
        Self::reply_from(status, r)
    }

    fn post(&self, base: &str, path: &str, body: &str) -> WireReply {
        let mut base = crate::kat_lease_client::request_base(base);
        loop {
            let url = format!("{base}{path}");
            let req = riir_kat::kat_ua::apply(self.agent.post(&url))
                .set("Content-Type", "application/json");
            match req.send_string(body) {
                Ok(r) => return Self::reply_body(r),
                Err(ureq::Error::Status(c, r)) => return Self::reply_from(c, r),
                Err(e) => match crate::kat_lease_client::failover_base(&base, &e) {
                    Some(next) => base = next,
                    None => {
                        return WireReply {
                            status: 0,
                            body: Vec::new(),
                            transport_error: Some(e.to_string()),
                        };
                    }
                },
            }
        }
    }
}

#[cfg(feature = "kat_transport")]
impl CorpusPrWire for HttpCorpusPrWire {
    fn service_info(&self, base: &str) -> crate::kat_lease_client::HttpResponse {
        self.get(base, "/")
    }

    fn get_lease(&self, base: &str) -> crate::kat_lease_client::HttpResponse {
        self.get(base, "/lease")
    }

    fn get_segment(
        &self,
        base: &str,
        id: &[u8; 32],
    ) -> crate::kat_lease_client::HttpResponse {
        self.get(base, &format!("/segment/{}", hex_bytes(id)))
    }

    fn post_envelope(&self, base: &str, body: &str) -> WireReply {
        self.post(base, "/corpuspr", body)
    }

    fn post_chunk(&self, base: &str, body: &str) -> WireReply {
        self.post(base, "/corpuspr/chunk", body)
    }
}

/// What one env's lease read produced — the carriable rows + the count
/// of rows the wire caps REFUSE (empty/oversized fields — the server's
/// own `validate` law; measured LIVE on devnet: the `doc_lazy_continuation`
/// pack serves an empty `buggy_code`). The uncarriable are SKIPPED with
/// a loud disclosure — the ingest secret-scan precedent ("the row is
/// skipped, the chunk stays valid"), never a silent drop and never a
/// single-row block on the whole carry.
#[derive(Clone, Debug, Default)]
pub struct LeaseRead {
    pub rows: Vec<PrRow>,
    pub skipped_uncarriable: usize,
}

/// Read one env's serving corpus as PR rows — the manifest passes
/// `verify_manifest` (the tamper-evident decode — strictly stronger than
/// the Python builder's header parse) and every segment is
/// BLAKE3-verified against its manifest id (the carried-builder read
/// law). The row's domain label is the PACK's domain.
///
/// # Errors
///
/// Transport/verdict failures, a tampered manifest or segment, or an
/// unparsable pack (all named).
#[cfg(feature = "kat_transport")]
pub fn read_lease_pairs(wire: &dyn CorpusPrWire, base: &str) -> Result<LeaseRead, String> {
    use crate::kat_lease_client::HttpResponse;
    let body = match wire.get_lease(base) {
        HttpResponse::Ok(b) => b,
        HttpResponse::NotModified => {
            return Err(format!("{base}/lease answered 304 without a conditional request"));
        }
        HttpResponse::Status(s) => return Err(format!("GET {base}/lease → HTTP {s}")),
        HttpResponse::Error(e) => return Err(format!("GET {base}/lease failed: {e}")),
    };
    let manifest = crate::kat_protocol_lease::verify_manifest(&body)
        .map_err(|e| format!("the {base}/lease manifest is not verifiable: {e}"))?;
    let mut read = LeaseRead::default();
    for desc in &manifest.segments {
        let id_hex = hex_bytes(&desc.id);
        let seg = match wire.get_segment(base, &desc.id) {
            HttpResponse::Ok(b) => b,
            HttpResponse::NotModified => {
                return Err(format!("{base}/segment/{id_hex}: 304 without a conditional"));
            }
            HttpResponse::Status(s) => {
                return Err(format!("GET {base}/segment/{id_hex} → HTTP {s}"));
            }
            HttpResponse::Error(e) => {
                return Err(format!("GET {base}/segment/{id_hex} failed: {e}"));
            }
        };
        if seg.len() != desc.len as usize || blake3::hash(&seg).as_bytes() != &desc.id {
            return Err(format!(
                "{base}: segment {id_hex} fails its BLAKE3 id — refusing (tamper)"
            ));
        }
        let pack = crate::kat_protocol_lease::parse_pack(&seg)
            .map_err(|e| format!("{base}: segment {id_hex} pack: {e}"))?;
        for e in &pack.entries {
            let row = PrRow {
                lint_key: e.rule_id.clone(),
                shape: e.buggy_code.clone(),
                fix: e.fixed_code.clone(),
                domain: Some(pack.domain.clone()),
            };
            if row.validate().is_ok() {
                read.rows.push(row);
            } else {
                read.skipped_uncarriable += 1;
            }
        }
    }
    Ok(read)
}

/// `GET /` + the T10 parse — what the server DECLARES about itself.
#[cfg(feature = "kat_transport")]
fn read_attestation(
    wire: &dyn CorpusPrWire,
    base: &str,
) -> Result<ServiceAttestation, String> {
    use crate::kat_lease_client::HttpResponse;
    match wire.service_info(base) {
        HttpResponse::Ok(b) => {
            riir_kat::kat_net_intent::parse_attestation(&b).map_err(|e| format!("{base}: {e}"))
        }
        HttpResponse::NotModified | HttpResponse::Status(_) => Err(format!(
            "{base}: the service-info read did not answer 200"
        )),
        HttpResponse::Error(e) => {
            Err(format!("{base}: the service-info read failed: {e}"))
        }
    }
}

// ── the submit engine + the command surface ────────────────────────────────

/// The command's CLI surface (the bin maps its flags onto this).
#[derive(Clone, Debug)]
pub struct CorpusPrArgs {
    /// The SOURCE env (url or alias — `devnet`): the dev branch.
    pub src: String,
    /// The DESTINATION env (url or alias — `mainnet`): the main ledger.
    pub dst: String,
    /// The explicit `pr_seq`; absent = local last + 1 (the client-local
    /// tick of S1.0).
    pub seq: Option<u64>,
    /// The payee account id (32 hex chars — the submitter of record).
    pub submitter: String,
    /// The PR-attestation key file (absent = the read-only gauge).
    pub key_file: Option<PathBuf>,
    /// Submit after drafting (POST envelope, then chunks in order).
    pub submit: bool,
}

/// What one run did — the disclosure lines in order (the bin prints
/// them; tests assert them), plus the outcome essentials.
#[derive(Clone, Debug, Default)]
pub struct PrReport {
    pub lines: Vec<String>,
    pub pr_seq: u64,
    pub total_rows: u64,
    pub chunks: usize,
    pub submitted: bool,
    pub pr_complete: bool,
}

/// Parse a POST reply: 200 → the JSON value; anything else → the coded
/// refusal surfaced verbatim (429 gains the idempotent-resume remedy).
#[cfg(feature = "kat_transport")]
fn reply_json(reply: &WireReply, what: &str) -> Result<serde_json::Value, String> {
    if let Some(e) = &reply.transport_error {
        return Err(format!("{what}: transport failure — {e}"));
    }
    if reply.status == 200 {
        return serde_json::from_slice(&reply.body)
            .map_err(|e| format!("{what}: the 200 body is not JSON ({e})"));
    }
    let parsed = serde_json::from_slice::<serde_json::Value>(&reply.body).ok();
    let prose = parsed
        .as_ref()
        .and_then(|v| v.get("error").and_then(serde_json::Value::as_str))
        .unwrap_or("<no error field>")
        .to_string();
    let code = parsed
        .as_ref()
        .and_then(|v| v.get("code").and_then(serde_json::Value::as_str))
        .unwrap_or("<no code>")
        .to_string();
    if reply.status == 429 {
        return Err(format!(
            "{what} rate-limited (HTTP 429, {code}): {prose} — resume later with the SAME \
             --seq and --submit: replays are idempotent (a byte-identical envelope answers \
             already-registered, an applied chunk answers idempotently)"
        ));
    }
    Err(format!(
        "{what} refused: HTTP {} ({code}): {prose}",
        reply.status
    ))
}

/// Submit one PR: envelope first (the attestation precedes the rows),
/// then chunks in manifest order (contiguity is the watermark law).
/// The pair state's `last_pr_seq` bumps AT REGISTRATION — a crash
/// mid-chunk resumes at the same seq (replays are idempotent).
#[cfg(feature = "kat_transport")]
fn submit_pr(
    wire: &dyn CorpusPrWire,
    ctx: &PrCtx,
    pair_dir: &Path,
    signed: &SignedPr,
    mut state: PairState,
    lines: &mut Vec<String>,
) -> Result<bool, String> {
    let reply = wire.post_envelope(&ctx.dst_url, &signed.envelope.to_string());
    let v = reply_json(&reply, "envelope")?;
    let registered = v
        .get("registered")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let _ = append_event(
        pair_dir,
        &serde_json::json!({
            "unix": unix_now(),
            "pr_seq": signed.pr_seq,
            "event": if registered { "envelope_registered" } else { "envelope_replay" },
        }),
    );
    state.last_pr_seq = signed.pr_seq;
    state.updated_unix = unix_now();
    save_pair_state(pair_dir, &state)
        .map_err(|e| format!("cannot persist the pair state: {e}"))?;
    lines.push(format!(
        "envelope: {} (pr_seq {}, {} chunks)",
        if registered {
            "registered"
        } else {
            "already registered — idempotent replay"
        },
        signed.pr_seq,
        signed.chunk_count
    ));
    for (i, body) in signed.chunks.iter().enumerate() {
        let reply = wire.post_chunk(&ctx.dst_url, &body.to_string());
        let v = reply_json(&reply, &format!("chunk {i}"))?;
        let get_u64 = |k: &str| {
            v.get(k)
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        };
        let complete = v
            .get("pr_complete")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        lines.push(format!(
            "chunk {}/{}: {} new, {} duplicated, {} superseded, {} secret-refused",
            i + 1,
            signed.chunk_count,
            get_u64("rows_new"),
            get_u64("rows_duplicated"),
            get_u64("rows_superseded"),
            get_u64("rows_secret_refused")
        ));
        let _ = append_event(
            pair_dir,
            &serde_json::json!({
                "unix": unix_now(),
                "pr_seq": signed.pr_seq,
                "event": "chunk_ok",
                "chunk_index": i,
                "pr_complete": complete,
            }),
        );
        if complete {
            lines.push(format!(
                "PR complete: pr_seq {} landed — the dst cursor is past it",
                signed.pr_seq
            ));
            return Ok(true);
        }
    }
    Ok(false)
}

/// The draft + optional submit tail of [`run_gated`] — split out so the
/// engine fn reads as the law sequence (gates → rebase → chunk → mode).
///
/// # Errors
///
/// Key loading, signing, draft/state persistence, and every submit
/// refusal (surfaced verbatim).
#[cfg(feature = "kat_transport")]
fn draft_and_maybe_submit(
    wire: &dyn CorpusPrWire,
    ctx: &PrCtx,
    args: &CorpusPrArgs,
    pair_dir: &Path,
    draft: &PrDraft,
    existing_last_seq: u64,
    report: &mut PrReport,
) -> Result<(), String> {
    let Some(key_path) = &args.key_file else {
        // The gauge (no key): S1.0's divergence gauge — read-only, no
        // writes at all. Exists for the dark era (no minted key yet).
        report.lines.push(format!(
            "gauge: would carry {} rows in {} chunk(s) — top root {}… (gauge only; pass \
             --key-file to draft the PR)",
            draft.total_rows,
            draft.chunks.len(),
            &hex_bytes(&draft.top_root)[..16]
        ));
        return Ok(());
    };

    let seed = load_key_seed(key_path)?;
    let key = SigningKey::from_bytes(&seed);
    let signed = build_signed_pr(&key, draft, ctx, &args.submitter)?;
    let out_dir = pair_dir.join("pr").join(format!("{:04}", draft.pr_seq));
    write_submission_files(&out_dir, &signed)
        .map_err(|e| format!("cannot write the PR draft at {}: {e}", out_dir.display()))?;
    // The T2b stamps bind now; `last_pr_seq` stays unconsumed until the
    // envelope registers server-side.
    let state = PairState {
        version: PAIR_STATE_VERSION,
        src_host: stamp_host_of(&ctx.src_url)?,
        src_ledger_id: hex_bytes(&ctx.src_ledger_id),
        dst_host: stamp_host_of(&ctx.dst_url)?,
        dst_ledger_id: hex_bytes(&ctx.dst_ledger_id),
        last_pr_seq: existing_last_seq,
        updated_unix: unix_now(),
    };
    save_pair_state(pair_dir, &state)
        .map_err(|e| format!("cannot persist the pair state: {e}"))?;
    let _ = append_event(
        pair_dir,
        &serde_json::json!({
            "unix": unix_now(),
            "pr_seq": draft.pr_seq,
            "event": "drafted",
            "rows": signed.total_rows,
            "chunks": signed.chunk_count,
        }),
    );
    report.lines.push(format!(
        "PR drafted: {} — {} chunk(s), submit in manifest order",
        out_dir.display(),
        signed.chunk_count
    ));
    if !args.submit {
        report
            .lines
            .push(format!("  POST {}/corpuspr        < envelope.json", ctx.dst_url));
        report.lines.push(format!(
            "  POST {}/corpuspr/chunk  < chunk_NNNN.json (in order)",
            ctx.dst_url
        ));
        return Ok(());
    }
    report.submitted = true;
    report.pr_complete = submit_pr(wire, ctx, pair_dir, &signed, state, &mut report.lines)?;
    Ok(())
}

/// The gated engine — everything after the T10/T2b/pin gates (tests
/// enter here with a synthetic ctx; the bin enters through
/// [`run_corpus_pr`]). Mode ladder: gauge (no key, read-only) → draft
/// (+key, files written, seq unconsumed) → submit (+`--submit`, envelope
/// registered then chunks in order).
///
/// # Errors
///
/// Every loud refusal: the all-duplicate carry (the S1.0 remedy line),
/// the over-cap carry, state/bookkeeping failures, and submit refusals
/// surfaced verbatim with the code.
#[cfg(feature = "kat_transport")]
pub fn run_gated(
    root: &Path,
    wire: &dyn CorpusPrWire,
    ctx: &PrCtx,
    args: &CorpusPrArgs,
) -> Result<PrReport, String> {
    let pair_dir = pair_dir(root, &ctx.dst_url, &ctx.src_url);
    let mut report = PrReport::default();
    let existing = load_pair_state(&pair_dir)?;
    if let Some(st) = &existing {
        judge_pair_state(
            st,
            &ctx.src_url,
            &ctx.dst_url,
            &ctx.src_ledger_id,
            &ctx.dst_ledger_id,
        )
        .map_err(|e| {
            format!(
                "{e} — the pair state lives at {}",
                pair_dir.join(PAIR_STATE_FILE).display()
            )
        })?;
    }
    let pr_seq = match args.seq {
        Some(s) => {
            if s == 0 {
                return Err("pr_seq must be ≥ 1 (PRs number from one)".into());
            }
            if let Some(st) = existing.as_ref() {
                if s <= st.last_pr_seq {
                    report.lines.push(format!(
                        "note: pr_seq {s} is not above the local last ({}) — the server's \
                         monotone check decides (a byte-identical envelope replays \
                         idempotently)",
                        st.last_pr_seq
                    ));
                }
            }
            s
        }
        None => existing.as_ref().map_or(1, |st| st.last_pr_seq + 1),
    };

    // The S1.0 rebase — the builder's first step (PUBLIC reads).
    let src_read = read_lease_pairs(wire, &ctx.src_url)?;
    let dst_read = read_lease_pairs(wire, &ctx.dst_url)?;
    if src_read.skipped_uncarriable > 0 || dst_read.skipped_uncarriable > 0 {
        report.lines.push(format!(
            "lease read: skipped {} + {} uncarriable rows (src + dst — empty/oversized \
             fields the wire caps refuse at ingest; the chunk stays valid)",
            src_read.skipped_uncarriable,
            dst_read.skipped_uncarriable
        ));
    }
    let rb = rebase(&src_read.rows, &dst_read.rows);
    report.lines.push(rb.disclosure(src_read.rows.len()));
    if rb.kept.is_empty() {
        return Err("nothing new for mainnet — mine on mainnet instead".into());
    }
    let chunks = chunk_plan(rb.kept)?;
    let draft = PrDraft::build(pr_seq, chunks)?;
    report.pr_seq = pr_seq;
    report.total_rows = draft.total_rows;
    report.chunks = draft.chunks.len();

    let existing_last_seq = existing.as_ref().map_or(0, |st| st.last_pr_seq);
    draft_and_maybe_submit(
        wire,
        ctx,
        args,
        &pair_dir,
        &draft,
        existing_last_seq,
        &mut report,
    )?;
    Ok(report)
}

/// The command entry — resolve + gate (T10/T2b/pins), then the engine.
///
/// # Errors
///
/// URL validation, net-pin refusals, attestation failures, the S1.11
/// gates, and every engine refusal (surfaced verbatim).
#[cfg(feature = "kat_transport")]
pub fn run_corpus_pr(
    root: &Path,
    wire: &dyn CorpusPrWire,
    args: &CorpusPrArgs,
) -> Result<PrReport, String> {
    let src_url = crate::kat_service_config::validate_service_url(&args.src)?;
    let dst_url = crate::kat_service_config::validate_service_url(&args.dst)?;
    // Net pins gate BOTH dials (a pinned build never dials outside its
    // pin, even with operator URLs — the fail-closed posture).
    crate::kat_net_pin::ensure_net_allowed(&src_url)?;
    crate::kat_net_pin::ensure_net_allowed(&dst_url)?;
    let src_att = read_attestation(wire, &src_url)?;
    let dst_att = read_attestation(wire, &dst_url)?;
    gate_dst_mainnet(&dst_url, &dst_att)?;
    gate_src(&src_url, &src_att)?;
    let Some(src_ledger_id) = src_att.ledger_id else {
        return Err("gate_src passed without a src ledger id".into());
    };
    let Some(dst_ledger_id) = dst_att.ledger_id else {
        return Err("the mainnet T10 gate passed without a dst ledger id".into());
    };
    if src_ledger_id == dst_ledger_id {
        return Err(format!(
            "{src_url} and {dst_url} serve the SAME ledger id — a PR to the ledger it came \
             from is a no-op by construction"
        ));
    }
    run_gated(
        root,
        wire,
        &PrCtx {
            src_url,
            dst_url,
            src_ledger_id,
            dst_ledger_id,
        },
        args,
    )
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier as _;

    fn row(lint: &str, shape: &str, fix: &str) -> PrRow {
        PrRow {
            lint_key: lint.into(),
            shape: shape.into(),
            fix: fix.into(),
            domain: Some("clippy".into()),
        }
    }

    fn golden_row_a() -> PrRow {
        row(
            "needless_return",
            "fn b() { return true; }",
            "fn b() { true }",
        )
    }

    fn golden_row_b() -> PrRow {
        row(
            "uninlined_format_args",
            "fn f() { println!(\"{}\", x); }",
            "fn f() { println!(\"{x}\"); }",
        )
    }

    /// The frozen THREE-WAY golden vectors (dapps lib + Python builder +
    /// this mirror — any drift reds its own side).
    #[test]
    fn golden_roots_are_frozen() {
        assert_eq!(
            hex_bytes(&golden_row_a().leaf_hash()),
            "13d115b70ec5b0e24cef72ba1f7d6d61f5dd1f325837fbb17a07c4dd350e5118",
            "leaf vector — prefix + le32 lens + fields (the domain is NOT hashed)"
        );
        assert_eq!(
            golden_row_a().shape_id(),
            "8b3d139d8a3e",
            "the native shape_id law (hex[..12])"
        );
        let c = chunk_root(&[golden_row_a(), golden_row_b()]);
        assert_eq!(
            hex_bytes(&c),
            "05299caec62d602f60da11ec17a71df848b7701b196473c7a206c5a3f39c7c83",
            "chunk vector — ordered leaves, n-embedded"
        );
        assert_eq!(
            hex_bytes(&top_root(&[c, c])),
            "c0117fcad1b5c2cc4023dacb760e34bfdd924e107c27f9a97fb4d54b40fa3e28",
            "top vector — ordered chunk roots"
        );
    }

    /// The message layouts, pinned by an INDEPENDENT hex construction
    /// (any reorder or endianness drift reds).
    #[test]
    fn messages_are_pinned_hex() {
        let d = "726969722d6b61742d636f7270757370723a76313a"; // the domain, hex
        let e = hex_bytes(&envelope_message(&[7; 32], &[9; 32], 5, &[1; 32]));
        let want_e = format!(
            "{d}{}{}0500000000000000{}",
            "07".repeat(32),
            "09".repeat(32),
            "01".repeat(32)
        );
        assert_eq!(e, want_e);
        let c = hex_bytes(&chunk_message(&[7; 32], &[9; 32], 5, 2, &[1; 32]));
        let want_c = format!(
            "{d}{}{}050000000000000002000000{}",
            "07".repeat(32),
            "09".repeat(32),
            "01".repeat(32)
        );
        assert_eq!(c, want_c);
        // Both ledger ids embedded: flipping either byte changes the msg.
        let mut src2 = [7u8; 32];
        src2[0] ^= 1;
        assert_ne!(
            envelope_message(&src2, &[9; 32], 5, &[1; 32]),
            envelope_message(&[7; 32], &[9; 32], 5, &[1; 32])
        );
        assert_ne!(
            chunk_message(&[7; 32], &[9; 32], 5, 2, &[1; 32]),
            chunk_message(&[7; 32], &[9; 32], 5, 3, &[1; 32]),
            "the chunk index binds the message"
        );
    }

    #[test]
    fn rebase_keeps_drops_and_supersesses() {
        let a = row("r1", "s1", "f1");
        let a_dup = row("r1", "s1", "f1");
        let b = row("r2", "s2", "f2");
        let b_super = row("r2", "s2", "f2-mainnet");
        let c = row("r3", "s3", "f3");
        let out = rebase(&[a, b, c.clone()], &[a_dup, b_super]);
        assert_eq!(out.kept, vec![c], "order-preserving: only the new row");
        assert_eq!(out.duplicates, 1);
        assert_eq!(out.superseded, 1);
        assert_eq!(
            out.disclosure(3),
            "rebase: kept 1 of 3 rows (1 duplicates, 1 superseded)"
        );
        // A different SHAPE at the same lint key is a NEW record (the
        // record identity is (lint_key, shape)).
        let same_lint_new_shape = row("r1", "s1-prime", "f1");
        let out2 = rebase(
            std::slice::from_ref(&same_lint_new_shape),
            &[row("r1", "s1", "f1")],
        );
        assert_eq!(out2.kept, vec![same_lint_new_shape]);
    }

    #[test]
    fn chunk_plan_splits_at_chunk_rows_and_refuses_past_the_cap() {
        let rows: Vec<PrRow> = (0..=CHUNK_ROWS).map(|i| row(&format!("r{i}"), "s", "f")).collect();
        let chunks = chunk_plan(rows).expect("1025 rows → 2 chunks");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), CHUNK_ROWS);
        assert_eq!(chunks[1].len(), 1);
        assert!(chunk_plan(Vec::new()).is_err(), "the empty carry refuses");
        let too_many: Vec<PrRow> = (0..=MAX_CHUNKS_PER_PR * CHUNK_ROWS)
            .map(|i| row(&format!("r{i}"), "s", "f"))
            .collect();
        let err = chunk_plan(too_many).expect_err("65 chunks refuse");
        assert!(err.contains("split the carry across PRs"), "{err}");
    }

    #[test]
    fn row_caps_refuse_early() {
        assert!(row(&"x".repeat(MAX_LINT_KEY_BYTES + 1), "s", "f").validate().is_err());
        assert!(row("r", &"x".repeat(MAX_FIELD_BYTES + 1), "f").validate().is_err());
        assert!(row("r", "s", &"x".repeat(MAX_FIELD_BYTES + 1)).validate().is_err());
        assert!(row("r", "s", "f").validate().is_ok());
        let mut no_domain = row("r", "s", "f");
        no_domain.domain = None;
        assert!(no_domain.validate().is_ok());
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_refine_corpuspr_{}_{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn pair_state_round_trips_and_judges_drift() {
        let root = temp_root("state");
        let pd = pair_dir(&root, "https://dst.example", "https://src.example");
        assert!(load_pair_state(&pd).expect("unborn lane").is_none());
        let st = PairState {
            version: PAIR_STATE_VERSION,
            src_host: "src.example".into(),
            src_ledger_id: hex_bytes(&[1; 32]),
            dst_host: "dst.example".into(),
            dst_ledger_id: hex_bytes(&[2; 32]),
            last_pr_seq: 3,
            updated_unix: 42,
        };
        save_pair_state(&pd, &st).expect("save");
        assert_eq!(load_pair_state(&pd).expect("reload").unwrap(), st);
        // Matching pass.
        assert!(judge_pair_state(&st, "https://src.example", "https://dst.example", &[1; 32], &[2; 32]).is_ok());
        // A source reset refuses, naming the direction + the remedy.
        let err = judge_pair_state(&st, "https://src.example", "https://dst.example", &[9; 32], &[2; 32])
            .expect_err("src reset");
        assert!(err.contains("SOURCE ledger id moved"), "{err}");
        // A dst reset refuses.
        let err = judge_pair_state(&st, "https://src.example", "https://dst.example", &[1; 32], &[9; 32])
            .expect_err("dst reset");
        assert!(err.contains("DESTINATION ledger id moved"), "{err}");
        // A copied dir (host drift) refuses.
        let err = judge_pair_state(&st, "https://other.example", "https://dst.example", &[1; 32], &[2; 32])
            .expect_err("host drift");
        assert!(err.contains("another lane"), "{err}");
        // A malformed file refuses loudly (never auto-rebirth).
        std::fs::write(pd.join(PAIR_STATE_FILE), "{ nope").unwrap();
        assert!(load_pair_state(&pd).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn key_file_loads_the_gen_corpuspr_format() {
        let root = temp_root("key");
        let p = root.join("k.txt");
        std::fs::write(&p, hex_bytes(&[7u8; 32])).unwrap();
        assert_eq!(load_key_seed(&p).unwrap(), [7u8; 32]);
        std::fs::write(&p, "not-hex").unwrap();
        assert!(load_key_seed(&p).is_err());
        assert!(load_key_seed(&root.join("absent.txt")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn signed_pr_binds_the_roots_and_verifies() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let ctx = PrCtx {
            src_url: "https://src.example".into(),
            dst_url: "https://dst.example".into(),
            src_ledger_id: [1; 32],
            dst_ledger_id: [2; 32],
        };
        let draft =
            PrDraft::build(3, vec![vec![golden_row_a(), golden_row_b()]]).expect("draft");
        let signed = build_signed_pr(&key, &draft, &ctx, "00112233445566778899aabbccddeeff")
            .expect("sign");
        assert_eq!(signed.envelope["licence"], "MIT");
        assert_eq!(signed.envelope["submitter"], "00112233445566778899aabbccddeeff");
        assert_eq!(signed.envelope["total_rows"], 2);
        // The envelope sig verifies over the RECOMPUTED top root (the
        // server's own law, checked client-side).
        let sig = unhex_fixed::<64>(
            signed.envelope["signature_hex"].as_str().expect("sig hex"),
        )
        .expect("64 bytes");
        let roots: Vec<[u8; 32]> = signed.envelope["chunk_roots"]
            .as_array()
            .expect("roots")
            .iter()
            .map(|r| unhex_fixed::<32>(r.as_str().expect("hex")).expect("32 bytes"))
            .collect();
        let msg = envelope_message(&[1; 32], &[2; 32], 3, &top_root(&roots));
        assert!(key
            .verifying_key()
            .verify(&msg, &ed25519_dalek::Signature::from_bytes(&sig))
            .is_ok());
        // The chunk sig verifies over the recomputed chunk root at its
        // index.
        let chunk = &signed.chunks[0];
        let csig = unhex_fixed::<64>(chunk["signature_hex"].as_str().expect("sig hex"))
            .expect("64 bytes");
        let croot = unhex_fixed::<32>(chunk["chunk_root"].as_str().expect("root hex"))
            .expect("32 bytes");
        let cmsg = chunk_message(&[1; 32], &[2; 32], 3, 0, &croot);
        assert!(key
            .verifying_key()
            .verify(&cmsg, &ed25519_dalek::Signature::from_bytes(&csig))
            .is_ok());
        // The rows ride the chunk body; the submitter must be a 16-byte
        // account id.
        assert_eq!(chunk["rows"].as_array().expect("rows").len(), 2);
        assert!(build_signed_pr(&key, &draft, &ctx, "xyz").is_err());
    }

    #[test]
    fn gates_refuse_wrong_dst_and_unbound_src() {
        let mainnet_att = ServiceAttestation {
            env_label: Some("mainnet".into()),
            ledger_id: Some([1; 32]),
        };
        assert!(gate_dst_mainnet("https://mainnet.gist.rs", &mainnet_att).is_ok());
        assert!(
            gate_dst_mainnet("https://devnet.gist.rs", &mainnet_att).is_err(),
            "the URL names devnet"
        );
        let unlabeled = ServiceAttestation {
            env_label: None,
            ledger_id: Some([1; 32]),
        };
        assert!(
            gate_dst_mainnet("https://mainnet.gist.rs", &unlabeled).is_err(),
            "no label → refuse (T10 fail-closed)"
        );
        let dev_att = ServiceAttestation {
            env_label: Some("devnet".into()),
            ledger_id: Some([2; 32]),
        };
        assert!(gate_src("https://devnet.gist.rs", &dev_att).is_ok());
        let no_id = ServiceAttestation {
            env_label: Some("devnet".into()),
            ledger_id: None,
        };
        assert!(
            gate_src("https://devnet.gist.rs", &no_id).is_err(),
            "a src without a ledger id cannot be bound"
        );
    }
}

/// The wire-level engine tests (mock transport — no sockets; the
/// `LeaseTransport` mock precedent). `run_gated`'s full ladder is
/// exercised: gauge → draft → submit → rate-limit resume.
#[cfg(all(test, feature = "kat_transport"))]
mod wire_tests {
    use super::*;
    use crate::kat_lease_client::HttpResponse;
    use crate::kat_protocol_lease::{
        CORPUS_PACK_VERSION, CorpusPack, LeaseManifest, LeaseSegmentDesc, PatternEntry,
    };
    use ed25519_dalek::Verifier as _;
    use std::cell::RefCell;
    use std::collections::{HashMap, VecDeque};

    type Maps = (
        HashMap<String, Vec<u8>>,
        HashMap<String, Vec<u8>>,
        HashMap<[u8; 32], Vec<u8>>,
    );

    struct MockWire {
        infos: HashMap<String, Vec<u8>>,
        manifests: HashMap<String, Vec<u8>>,
        segments: HashMap<[u8; 32], Vec<u8>>,
        posts: RefCell<Vec<(String, String)>>,
        replies: RefCell<VecDeque<WireReply>>,
    }

    impl CorpusPrWire for MockWire {
        fn service_info(&self, base: &str) -> HttpResponse {
            HttpResponse::Ok(self.infos.get(base).cloned().unwrap_or_default())
        }

        fn get_lease(&self, base: &str) -> HttpResponse {
            HttpResponse::Ok(self.manifests.get(base).cloned().unwrap_or_default())
        }

        fn get_segment(&self, _base: &str, id: &[u8; 32]) -> HttpResponse {
            HttpResponse::Ok(self.segments.get(id).cloned().unwrap_or_default())
        }

        fn post_envelope(&self, _base: &str, body: &str) -> WireReply {
            self.posts.borrow_mut().push(("/corpuspr".into(), body.into()));
            self.replies.borrow_mut().pop_front().unwrap_or(WireReply {
                status: 200,
                body: br#"{"registered":true,"pr_seq":1,"chunks":1}"#.to_vec(),
                transport_error: None,
            })
        }

        fn post_chunk(&self, _base: &str, body: &str) -> WireReply {
            self.posts
                .borrow_mut()
                .push(("/corpuspr/chunk".into(), body.into()));
            self.replies.borrow_mut().pop_front().unwrap_or(WireReply {
                status: 200,
                body: br#"{"chunk_index":0,"rows_new":1,"rows_duplicated":0,"rows_superseded":0,"rows_secret_refused":0,"pr_complete":true}"#
                    .to_vec(),
                transport_error: None,
            })
        }
    }

    fn fixture_env(maps: &mut Maps, base: &str, env_label: &str, ledger: [u8; 32], entries: &[(&str, &str, &str)]) {
        let pack = CorpusPack {
            pack_version: CORPUS_PACK_VERSION,
            domain: "clippy".into(),
            entries: entries
                .iter()
                .map(|(l, s, f)| PatternEntry {
                    rule_id: (*l).into(),
                    buggy_code: (*s).into(),
                    fixed_code: (*f).into(),
                })
                .collect(),
        };
        let bytes = crate::kat_protocol_lease::render_pack(&pack);
        let desc = LeaseSegmentDesc::of(&bytes);
        let manifest = LeaseManifest::build(7, 1, vec![desc]);
        maps.0.insert(
            base.into(),
            serde_json::json!({"service": "kat", "env": env_label, "ledger_id": hex_bytes(&ledger)})
                .to_string()
                .into_bytes(),
        );
        maps.1.insert(base.into(), manifest.to_bytes());
        maps.2.insert(desc.id, bytes);
    }

    fn two_env_wire() -> MockWire {
        let mut maps: Maps = (HashMap::new(), HashMap::new(), HashMap::new());
        fixture_env(
            &mut maps,
            "https://src.example",
            "devnet",
            [1; 32],
            &[("r1", "s1", "f1"), ("r2", "s2", "f2"), ("r3", "s3", "f3")],
        );
        // dst carries r1 identically (dup) and r2 with its own fix
        // (superseded — MAINNET WINS); r3 is new.
        fixture_env(
            &mut maps,
            "https://dst.example",
            "mainnet",
            [2; 32],
            &[("r1", "s1", "f1"), ("r2", "s2", "f2-mainnet")],
        );
        MockWire {
            infos: maps.0,
            manifests: maps.1,
            segments: maps.2,
            posts: RefCell::new(Vec::new()),
            replies: RefCell::new(VecDeque::new()),
        }
    }

    fn ctx() -> PrCtx {
        PrCtx {
            src_url: "https://src.example".into(),
            dst_url: "https://dst.example".into(),
            src_ledger_id: [1; 32],
            dst_ledger_id: [2; 32],
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_refine_corpuspr_w_{}_{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// The shared e2e fixture: a temp root + key file + the two-env wire
    /// (src: r1,r2,r3; dst: r1-dup, r2-superseded — keeps only r3).
    struct E2e {
        root: PathBuf,
        wire: MockWire,
        ctx: PrCtx,
        key: SigningKey,
        key_path: PathBuf,
    }

    fn e2e_fixture(name: &str) -> E2e {
        let root = temp_root(name);
        let wire = two_env_wire();
        let key = SigningKey::from_bytes(&[9u8; 32]);
        let key_path = root.join("key.txt");
        std::fs::write(&key_path, hex_bytes(&[9u8; 32])).unwrap();
        E2e {
            root,
            wire,
            ctx: ctx(),
            key,
            key_path,
        }
    }

    fn pr_args(e: &E2e, seq: Option<u64>, key: bool, submit: bool) -> CorpusPrArgs {
        CorpusPrArgs {
            src: e.ctx.src_url.clone(),
            dst: e.ctx.dst_url.clone(),
            seq,
            submitter: "00112233445566778899aabbccddeeff".into(),
            key_file: key.then(|| e.key_path.clone()),
            submit,
        }
    }

    #[test]
    fn the_gauge_is_read_only() {
        let e = e2e_fixture("gauge");
        let gauge = run_gated(&e.root, &e.wire, &e.ctx, &pr_args(&e, None, false, false))
            .expect("gauge");
        assert!(
            gauge
                .lines
                .iter()
                .any(|l| l == "rebase: kept 1 of 3 rows (1 duplicates, 1 superseded)"),
            "{:?}",
            gauge.lines
        );
        assert!(gauge.lines.iter().any(|l| l.starts_with("gauge:")));
        let pd = pair_dir(&e.root, &e.ctx.dst_url, &e.ctx.src_url);
        assert!(!pd.join(PAIR_STATE_FILE).exists(), "the gauge writes nothing");
        let _ = std::fs::remove_dir_all(&e.root);
    }

    #[test]
    fn the_draft_binds_stamps_without_consuming_the_seq() {
        let e = e2e_fixture("draft");
        run_gated(&e.root, &e.wire, &e.ctx, &pr_args(&e, None, true, false)).expect("draft");
        let pd = pair_dir(&e.root, &e.ctx.dst_url, &e.ctx.src_url);
        let pr_dir = pd.join("pr").join("0001");
        assert!(pr_dir.join("envelope.json").exists());
        assert!(pr_dir.join("chunk_0000.json").exists());
        assert!(pr_dir.join("manifest.json").exists());
        let st: PairState = serde_json::from_str(
            &std::fs::read_to_string(pd.join(PAIR_STATE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(st.last_pr_seq, 0, "a draft does not consume the seq");
        assert_eq!(st.src_ledger_id, hex_bytes(&[1; 32]));
        let _ = std::fs::remove_dir_all(&e.root);
    }

    #[test]
    fn the_submit_registers_envelope_then_chunks_in_order() {
        let e = e2e_fixture("submit");
        run_gated(&e.root, &e.wire, &e.ctx, &pr_args(&e, None, true, false)).expect("draft");
        e.wire.replies.borrow_mut().push_back(WireReply {
            status: 200,
            body: br#"{"registered":true,"pr_seq":1,"chunks":1}"#.to_vec(),
            transport_error: None,
        });
        e.wire.replies.borrow_mut().push_back(WireReply {
            status: 200,
            body: br#"{"chunk_index":0,"rows_new":1,"rows_duplicated":0,"rows_superseded":0,"rows_secret_refused":0,"pr_complete":true}"#.to_vec(),
            transport_error: None,
        });
        let sub =
            run_gated(&e.root, &e.wire, &e.ctx, &pr_args(&e, Some(1), true, true)).expect("submit");
        assert!(sub.submitted && sub.pr_complete);
        let pd = pair_dir(&e.root, &e.ctx.dst_url, &e.ctx.src_url);
        let st: PairState = serde_json::from_str(
            &std::fs::read_to_string(pd.join(PAIR_STATE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(st.last_pr_seq, 1, "registration consumes the seq");
        // Posts in manifest order + the posted envelope signature
        // verifies over its recomputed top root (recompute-not-trust).
        let posts = e.wire.posts.borrow();
        assert_eq!(posts[0].0, "/corpuspr");
        assert_eq!(posts[1].0, "/corpuspr/chunk");
        let env_body: serde_json::Value = serde_json::from_str(&posts[0].1).unwrap();
        let sig = unhex_fixed::<64>(env_body["signature_hex"].as_str().unwrap()).unwrap();
        let roots: Vec<[u8; 32]> = env_body["chunk_roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| unhex_fixed::<32>(r.as_str().unwrap()).unwrap())
            .collect();
        assert!(e
            .key
            .verifying_key()
            .verify(
                &envelope_message(&[1; 32], &[2; 32], 1, &top_root(&roots)),
                &ed25519_dalek::Signature::from_bytes(&sig)
            )
            .is_ok());
        let log = std::fs::read_to_string(pd.join("submissions.jsonl")).unwrap();
        assert!(log.contains("envelope_registered") && log.contains("chunk_ok"));
        let _ = std::fs::remove_dir_all(&e.root);
    }

    #[test]
    fn all_duplicate_refuses_with_the_remedy_line() {
        let mut maps: Maps = (HashMap::new(), HashMap::new(), HashMap::new());
        fixture_env(&mut maps, "https://src.example", "devnet", [1; 32], &[("r1", "s1", "f1")]);
        fixture_env(
            &mut maps,
            "https://dst.example",
            "mainnet",
            [2; 32],
            &[("r1", "s1", "f1"), ("r9", "s9", "f9")],
        );
        let wire = MockWire {
            infos: maps.0,
            manifests: maps.1,
            segments: maps.2,
            posts: RefCell::new(Vec::new()),
            replies: RefCell::new(VecDeque::new()),
        };
        let root = temp_root("dup");
        let err = run_gated(&root, &wire, &ctx(), &CorpusPrArgs {
            src: "https://src.example".into(),
            dst: "https://dst.example".into(),
            seq: None,
            submitter: "00112233445566778899aabbccddeeff".into(),
            key_file: None,
            submit: false,
        })
        .expect_err("kept == 0 refuses");
        assert_eq!(err, "nothing new for mainnet — mine on mainnet instead");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn submit_stops_at_429_with_the_resume_remedy() {
        let root = temp_root("rate");
        let wire = two_env_wire();
        let ctx = ctx();
        let key_path = root.join("key.txt");
        std::fs::write(&key_path, hex_bytes(&[9u8; 32])).unwrap();
        // Draft first (writes the state + files).
        run_gated(
            &root,
            &wire,
            &ctx,
            &CorpusPrArgs {
                src: ctx.src_url.clone(),
                dst: ctx.dst_url.clone(),
                seq: Some(1),
                submitter: "00112233445566778899aabbccddeeff".into(),
                key_file: Some(key_path),
                submit: false,
            },
        )
        .expect("draft");
        // Envelope OK; the chunk is rate-limited.
        wire.replies.borrow_mut().push_back(WireReply {
            status: 200,
            body: br#"{"registered":true,"pr_seq":1,"chunks":1}"#.to_vec(),
            transport_error: None,
        });
        wire.replies.borrow_mut().push_back(WireReply {
            status: 429,
            body: br#"{"error":"corpus-pr: per-account hourly limit reached","code":"rate_limited"}"#
                .to_vec(),
            transport_error: None,
        });
        let err = run_gated(
            &root,
            &wire,
            &ctx,
            &CorpusPrArgs {
                src: ctx.src_url.clone(),
                dst: ctx.dst_url.clone(),
                seq: Some(1),
                submitter: "00112233445566778899aabbccddeeff".into(),
                key_file: Some(root.join("key.txt")),
                submit: true,
            },
        )
        .expect_err("429 stops the run");
        assert!(
            err.contains("resume later with the SAME --seq"),
            "{err}"
        );
        // Registration still consumed the seq (the envelope landed).
        let pd = pair_dir(&root, &ctx.dst_url, &ctx.src_url);
        let st: PairState = serde_json::from_str(
            &std::fs::read_to_string(pd.join(PAIR_STATE_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(st.last_pr_seq, 1);
        assert_eq!(wire.posts.borrow().len(), 2, "envelope + the refused chunk");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_source_reset_refuses_until_adjudicated() {
        let root = temp_root("reset");
        let wire = two_env_wire();
        let ctx = ctx();
        let key_path = root.join("key.txt");
        std::fs::write(&key_path, hex_bytes(&[9u8; 32])).unwrap();
        run_gated(
            &root,
            &wire,
            &ctx,
            &CorpusPrArgs {
                src: ctx.src_url.clone(),
                dst: ctx.dst_url.clone(),
                seq: None,
                submitter: "00112233445566778899aabbccddeeff".into(),
                key_file: Some(key_path),
                submit: false,
            },
        )
        .expect("the first PR binds the stamps");
        // The source resets: its ledger id moves.
        let drifted = PrCtx {
            src_ledger_id: [9; 32],
            ..ctx
        };
        let err = run_gated(
            &root,
            &wire,
            &drifted,
            &CorpusPrArgs {
                src: drifted.src_url.clone(),
                dst: drifted.dst_url.clone(),
                seq: None,
                submitter: "00112233445566778899aabbccddeeff".into(),
                key_file: None,
                submit: false,
            },
        )
        .expect_err("the drift refuses");
        assert!(err.contains("SOURCE ledger id moved"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn seg_reads_are_blake3_gated_and_manifest_tamper_refuses() {
        let mut maps: Maps = (HashMap::new(), HashMap::new(), HashMap::new());
        fixture_env(&mut maps, "https://src.example", "devnet", [1; 32], &[("r1", "s1", "f1")]);
        // A tampered segment: right id, wrong bytes (the hash check).
        let mut wire = MockWire {
            infos: maps.0,
            manifests: maps.1,
            segments: maps.2,
            posts: RefCell::new(Vec::new()),
            replies: RefCell::new(VecDeque::new()),
        };
        let bad_key = *wire.segments.keys().next().unwrap();
        wire.segments.insert(bad_key, b"tampered".to_vec());
        let err = read_lease_pairs(&wire, "https://src.example").expect_err("tamper");
        assert!(err.contains("BLAKE3 id"), "{err}");

        // A tampered manifest: the commitment must not verify.
        let mut maps2: Maps = (HashMap::new(), HashMap::new(), HashMap::new());
        fixture_env(&mut maps2, "https://src.example", "devnet", [1; 32], &[("r1", "s1", "f1")]);
        let mut manifest = maps2.1.remove("https://src.example").unwrap();
        manifest[4] ^= 0xff; // corrupt the epoch inside the body
        maps2.1.insert("https://src.example".into(), manifest);
        let wire2 = MockWire {
            infos: maps2.0,
            manifests: maps2.1,
            segments: maps2.2,
            posts: RefCell::new(Vec::new()),
            replies: RefCell::new(VecDeque::new()),
        };
        let err2 = read_lease_pairs(&wire2, "https://src.example").expect_err("tamper");
        assert!(err2.contains("not verifiable"), "{err2}");
    }

    #[test]
    fn uncarriable_rows_skip_with_a_count_never_a_block() {
        // The LIVE finding, pinned: an empty-shape row (devnet's
        // `doc_lazy_continuation` pack) is uncarriable — skip + count,
        // the chunk stays valid (the ingest secret-scan precedent).
        let mut maps: Maps = (HashMap::new(), HashMap::new(), HashMap::new());
        fixture_env(
            &mut maps,
            "https://src.example",
            "devnet",
            [1; 32],
            &[("r1", "", "f1"), ("r2", "s2", "f2")],
        );
        let wire = MockWire {
            infos: maps.0,
            manifests: maps.1,
            segments: maps.2,
            posts: RefCell::new(Vec::new()),
            replies: RefCell::new(VecDeque::new()),
        };
        let read = read_lease_pairs(&wire, "https://src.example").expect("read");
        assert_eq!(read.skipped_uncarriable, 1);
        assert_eq!(read.rows.len(), 1);
        assert_eq!(read.rows[0].lint_key, "r2");
    }
}
