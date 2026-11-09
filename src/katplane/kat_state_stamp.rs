//! Vendored from `riir-kat` (`863c590798b5`, 2026-11-09, issue 221
//! private-dep minimization): the feature-free protocol tier refine's
//! UNGATED surfaces consume (secret scan, units, consent dir, config
//! store, hosts, net pin, state stamp, soft gate). std-only by the
//! leaf's protocol-tier purity rule. The canonical copy stays in the
//! private sibling (`riir-kat` `src/kat_state_stamp.rs`); drift is caught by the
//! golden pins in `tests/katplane_parity.rs` and repaired by
//! re-vendoring via `scripts/vendor_katplane.sh` — never hand-edit one
//! side.

//! The per-network state stamp (riir-kat Issue 003 T2+T2b) + the T4
//! quarantine half.
//!
//! Every machine-local money-state file (`kat_burn_ack.json`,
//! `kat_charge.json`, `corpus-lease/charge.json`) carries two additive
//! fields written at write time:
//!
//! - `service_host` — the CANONICAL host that answered (legacy aliases
//!   canonicalized first, so `ai.gist.rs` and `mainnet.gist.rs` are one
//!   host); the directory decides placement (T1), the stamp lets a read
//!   detect a copied or re-pointed directory.
//! - `ledger_id` — the D8c ledger-instance id the server attested on the
//!   service-info read at write time, lowercase hex. This half is what a
//!   host-only stamp provably cannot do: catch a same-host RESET (the D9
//!   cutover, a devnet soak reset), where the host matches by
//!   construction.
//!
//! ## The judge (fail-closed, both laws)
//!
//! A money decision that would READ a stamp-less or mismatched file
//! refuses loud ([`StampVerdict::prose`] names the T4 remedy) — never
//! silently serves it. The strict half binds the burn ACK: a file with
//! no `ledger_id` while the server attests one is
//! [`StampVerdict::UnprovenFileId`] (the transition-window arm). The
//! charge books run the [`StampLaw::TransitionalId`] law — a host-only
//! file against an attesting server passes and the id upgrades at the
//! next stamped write — because their reset risk is already covered by
//! the ack's strict gate on the same push, and a refuse-here loop would
//! deadlock files that can only gain the id through the refused path.
//!
//! Two honest allowances, both measured shapes:
//! - `(None, None)` — both sides unattested — passes on the host match:
//!   the loopback dev rig whose `/` read is the T10 dev-rig allowance.
//!   Hosted never reaches it: T10 refuses a hosted money write without a
//!   ledger id before any stamp is consulted.
//! - A file that EXISTS keeps whatever stamp it carries — the writers
//!   stamp only BORN files (no file), never stamp over an existing one:
//!   stamping over would erase the exact evidence the judge refuses on.
//!
//! The module is protocol tier (feature-free): the fields ride
//! feature-free state structs, and the judge is pure.

/// The state-file field carrying the canonical host that answered.
pub const SERVICE_HOST_FIELD: &str = "service_host";

/// The state-file field carrying the D8c ledger-instance id (hex).
pub const LEDGER_ID_FIELD: &str = "ledger_id";

/// The quarantine directory under the `.refine` root — where
/// [`quarantine_state_file`] moves files the owner (or the future
/// `cargo refine config adopt-state` command) decides belong to no
/// ledger. A move, never a delete: the bytes survive twice (the `.bak`
/// copy beside the moved file).
pub const QUARANTINE_DIR: &str = "quarantine";

/// How strictly the ledger-id half judges a file that carries no id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampLaw {
    /// `(None, Some)` refuses: the burn ACK. The ack is the burn push's
    /// watermark; a file written against an unattested instance must not
    /// serve once the host attests a (possibly different) ledger.
    StrictId,
    /// `(None, Some)` passes: the charge books. Their host is already
    /// proven, their reset risk is covered by the same push's strict ack
    /// gate, and refusing would deadlock files that can only gain the id
    /// through the refused path. The id upgrades at the next stamped
    /// write ([`crate::kat_state_stamp`]'s writers never clobber an
    /// existing id with absence).
    TransitionalId,
}

/// Why a state file may not serve a money decision. Every variant
/// renders through [`StampVerdict::prose`] — `refused locally:`-prefixed
/// (the T13 lesson: every client-local refusal carries the prefix so the
/// prose is greppable and the parse layers take the safe direction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StampVerdict {
    /// The file exists and carries no `service_host` — the mixed-era
    /// pre-stamp state this issue was filed for.
    Unstamped,
    /// The file was written against another host — a copied or re-pointed
    /// state directory.
    HostMismatch {
        /// The host recorded in the file.
        file: String,
        /// The host the money decision targets.
        current: String,
    },
    /// Same host, different ledger instance — the D9-reset arm no
    /// host-only stamp can catch.
    LedgerIdMismatch {
        /// The id recorded in the file.
        file_id: String,
        /// The id the server now attests.
        current_id: String,
    },
    /// The file carries no id while the server attests one (the strict
    /// law only).
    UnprovenFileId,
    /// The file carries an id but the current server could not attest
    /// its own (a failed service-info read or a pre-D8c deploy) —
    /// failing closed.
    UnattestedCurrent,
    /// The file's bytes could not be read as JSON — its provenance
    /// cannot be established at all.
    Unreadable(String),
}

impl StampVerdict {
    /// The one-line refusal: `refused locally:`-prefixed, naming the
    /// file and the T4 remedy.
    #[must_use]
    pub fn prose(&self, what: &str, path: &std::path::Path) -> String {
        let short = |id: &str| id.chars().take(8).collect::<String>();
        let shown = path.display();
        let body = match self {
            Self::Unstamped => format!(
                "{what} at {shown} carries no network stamp — it predates the per-network stamp \
                 and may belong to a DIFFERENT ledger; serving it into a money decision could \
                 report one ledger's cumulative into another (the drain-and-forgive bug)"
            ),
            Self::HostMismatch { file, current } => format!(
                "{what} at {shown} was written against `{file}` but this client targets \
                 `{current}` — the state directory was copied or re-pointed"
            ),
            Self::LedgerIdMismatch {
                file_id,
                current_id,
            } => format!(
                "{what} at {shown} is stamped for ledger {}… but the server now runs ledger {}… — \
                 the ledger was reset (a D9 cutover or a soak reset) or the file belongs to a \
                 previous instance",
                short(file_id),
                short(current_id)
            ),
            Self::UnprovenFileId => format!(
                "{what} at {shown} carries no ledger id while the server attests one — this \
                 client cannot prove the file belongs to the running ledger"
            ),
            Self::UnattestedCurrent => format!(
                "{what} at {shown} carries a ledger id but the server could not attest its own \
                 (a failed service-info read or a pre-D8c deploy) — failing closed"
            ),
            Self::Unreadable(e) => format!(
                "{what} at {shown} is unreadable ({e}) — its network provenance cannot be \
                 established"
            ),
        };
        format!(
            "refused locally: {body}. Remedy: `cargo refine config adopt-state <devnet|mainnet> \
             [--discard]` (per file), or move the file to `<root>/.refine/{QUARANTINE_DIR}/` \
             yourself."
        )
    }
}

/// Judge the host half: a stamp-less file refuses; a host must match
/// (case-insensitive — hosts are stored lowercase, compared tolerantly).
///
/// # Errors
///
/// [`StampVerdict::Unstamped`] or [`StampVerdict::HostMismatch`].
pub fn judge_stamp_host(file_host: Option<&str>, current_host: &str) -> Result<(), StampVerdict> {
    match file_host {
        None => Err(StampVerdict::Unstamped),
        Some(h) if h.eq_ignore_ascii_case(current_host) => Ok(()),
        Some(h) => Err(StampVerdict::HostMismatch {
            file: h.to_string(),
            current: current_host.to_string(),
        }),
    }
}

/// Judge the ledger-id half by the given [`StampLaw`]. The two
/// allowances live here: `(None, None)` (both sides unattested — the
/// loopback dev rig) and, under [`StampLaw::TransitionalId`],
/// `(None, Some)` (the id upgrades at the next stamped write).
///
/// # Errors
///
/// The id-half variants of [`StampVerdict`].
pub fn judge_stamp_id(
    file_id: Option<&str>,
    current_id: Option<&str>,
    law: StampLaw,
) -> Result<(), StampVerdict> {
    match (file_id, current_id) {
        (Some(f), Some(c)) if f.eq_ignore_ascii_case(c) => Ok(()),
        (Some(f), Some(c)) => Err(StampVerdict::LedgerIdMismatch {
            file_id: f.to_string(),
            current_id: c.to_string(),
        }),
        (Some(_), None) => Err(StampVerdict::UnattestedCurrent),
        (None, Some(_)) if law == StampLaw::StrictId => Err(StampVerdict::UnprovenFileId),
        (None, _) => Ok(()),
    }
}

/// The full judge — host half first (no network needed to refuse a
/// cross-net file), then the id half.
///
/// # Errors
///
/// The first failing half's [`StampVerdict`].
pub fn judge_stamp(
    file_host: Option<&str>,
    file_id: Option<&str>,
    current_host: &str,
    current_id: Option<&str>,
    law: StampLaw,
) -> Result<(), StampVerdict> {
    judge_stamp_host(file_host, current_host)?;
    judge_stamp_id(file_id, current_id, law)
}

/// The canonical stamp host for a service base URL: the two hosted nets
/// under their bare pinned names (aliases canonicalized first — one
/// ledger, one host); LOOPBACK under its bare name, port-less — loopback
/// is ONE dev plane and a port change is not a network change (the
/// ledgers on it are distinguished by the ledger-id half the moment a
/// real worker attests; an unattesting mock is a dev toy); every other
/// host as `host:port` lowercased — two custom proxies are two
/// deployments.
#[must_use]
pub fn stamp_host(url: &str) -> Option<String> {
    let canonical = crate::kat_service_urls::canonical_service_url(url.trim());
    let bare = crate::kat_service_config::host_of(&canonical)?.to_ascii_lowercase();
    #[cfg(not(all(feature = "net-devnet", not(feature = "net-mainnet"))))]
    if bare == "tuna.riir.ai" {
        // the points plane's gen-2 custom domain — same worker/ledger as
        // MAINNET_HOST; compiled out under a devnet-only pin (symmetry).
        return Some(bare);
    }
    #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
    if bare == crate::kat_service_urls::DEVNET_HOST {
        return Some(bare);
    }
    if bare == crate::kat_service_urls::MAINNET_HOST
        || crate::kat_service_config::is_loopback_host(&bare)
    {
        return Some(bare);
    }
    let rest = canonical.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    if host_port.is_empty() {
        None
    } else {
        Some(host_port.to_ascii_lowercase())
    }
}

/// The host the CURRENT resolution names ([`stamp_host`] of the resolved
/// service URL) — the stamp a BORN state file carries when its writer
/// has no server answer of its own (the charge books). Reads env +
/// config exactly like every money command's resolution; never panics.
#[must_use]
pub fn current_service_host() -> String {
    let (url, _source) = crate::kat_service_config::resolved_service_url(
        crate::kat_service_urls::MAINNET_SERVICE_URL,
    );
    stamp_host(&url).unwrap_or_else(|| "unresolved".to_string())
}

/// Lowercase hex for a ledger id — the state-file encoding (the
/// service-info read's own form).
#[must_use]
pub fn ledger_id_hex(id: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in id {
        out.push(HEX[usize::from(b >> 4)] as char);
        out.push(HEX[usize::from(b & 0x0F)] as char);
    }
    out
}

/// Read just the two stamp fields off a state file (a raw JSON read —
/// the caller's own struct stays the authority for everything else).
///
/// # Errors
///
/// A human-readable string when the file cannot be read or parsed — the
/// caller refuses (provenance cannot be established).
pub fn read_stamp_fields(
    path: &std::path::Path,
) -> Result<(Option<String>, Option<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok((
        v.get(SERVICE_HOST_FIELD)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        v.get(LEDGER_ID_FIELD)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    ))
}

/// Move one state file into `<root>/.refine/quarantine/<rel>` — the T4
/// `--discard` half's move helper (the riir-refine `adopt-state` command
/// drives this; nothing in this crate calls it). The bytes survive
/// TWICE: a `.bak` copy beside the moved file, then the move itself. A
/// cross-device rename falls back to copy+remove; a failed remove after
/// a successful copy is an ERROR (the quarantined copy exists, the
/// original is still in place — the operator sees the failure, never a
/// silent partial adopt).
///
/// # Errors
///
/// Filesystem failures, or a `rel` carrying `..` components.
pub fn quarantine_state_file(
    path: &std::path::Path,
    root: &std::path::Path,
    rel: &str,
) -> std::io::Result<std::path::PathBuf> {
    let rel_path = std::path::Path::new(rel);
    if rel_path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "quarantine rel must not carry `..` components",
        ));
    }
    let file = rel_path.file_name().map_or_else(
        || std::ffi::OsString::from(rel),
        std::borrow::ToOwned::to_owned,
    );
    let dest_dir = match rel_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        Some(p) => root.join(".refine").join(QUARANTINE_DIR).join(p),
        None => root.join(".refine").join(QUARANTINE_DIR),
    };
    std::fs::create_dir_all(&dest_dir)?;
    let bak = dest_dir.join(format!("{}.bak", file.to_string_lossy()));
    std::fs::copy(path, &bak)?;
    let mut dest = dest_dir.join(&file);
    if dest.exists() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        dest = dest_dir.join(format!("{}-{nanos}", file.to_string_lossy()));
    }
    if std::fs::rename(path, &dest).is_ok() {
        return Ok(dest);
    }
    std::fs::copy(path, &dest)?;
    std::fs::remove_file(path)?;
    Ok(dest)
}

// ── T1 (riir-kat Issue 003): the per-network STATE DIRECTORY seam ──────────

/// The per-network state subtree — every net-scoped state file lives under
/// `<root>/.refine/net/<net-key>/…` (Issue 003 T1: subdirectories, not
/// host-suffixed flat files — one migration move, one purge target, and
/// the outbox/claims queues inherit per-net scoping for free).
pub const NET_DIR_NAME: &str = "net";

/// The fs-safe directory key for a service base URL: [`stamp_host`]
/// applied FIRST (aliases canonicalize — `ai.gist.rs` and
/// `mainnet.gist.rs` are one ledger and get one directory; loopback is
/// port-less, one dev plane), then `:` → `_` so a custom `host:port` key
/// can never form an illegal Windows path component (the 4090 matters).
#[must_use]
pub fn net_dir_key(url: &str) -> String {
    stamp_host(url)
        .unwrap_or_else(|| "unresolved".to_string())
        .replace(':', "_")
}

/// The per-network state dir for a net key: `<root>/.refine/net/<key>`.
#[must_use]
pub fn net_state_dir(root: &std::path::Path, key: &str) -> std::path::PathBuf {
    root.join(".refine").join(NET_DIR_NAME).join(key)
}

/// [`net_state_dir`] keyed straight off a service base URL — the
/// placement half of the stamp: a file is BORN in the directory of the
/// net it is being pushed to, while the in-file stamp (T2) stays the
/// read-side defense (the two derive from the same [`stamp_host`] call,
/// so they cannot disagree).
#[must_use]
pub fn net_state_dir_for_url(root: &std::path::Path, url: &str) -> std::path::PathBuf {
    net_state_dir(root, &net_dir_key(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir-kat-stamp-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    const ID_A: [u8; 32] = [0xAu8; 32];
    const ID_B: [u8; 32] = [0xBu8; 32];

    #[test]
    fn the_judge_matrix_refuses_every_unprovable_shape_and_passes_the_provable_ones() {
        // Unstamped: the mixed-era M3 file — refuse, always.
        assert_eq!(
            judge_stamp(
                None,
                None,
                "mainnet.gist.rs",
                Some("aa"),
                StampLaw::StrictId
            ),
            Err(StampVerdict::Unstamped)
        );
        // Host match + id match: pass (both laws).
        assert_eq!(
            judge_stamp(
                Some("mainnet.gist.rs"),
                Some("aa"),
                "MAINNET.gist.rs",
                Some("AA"),
                StampLaw::StrictId
            ),
            Ok(())
        );
        // Host mismatch: refuse (a copied directory), id irrelevant.
        assert_eq!(
            judge_stamp(
                Some("devnet.gist.rs"),
                Some("aa"),
                "mainnet.gist.rs",
                Some("aa"),
                StampLaw::StrictId
            ),
            Err(StampVerdict::HostMismatch {
                file: "devnet.gist.rs".to_string(),
                current: "mainnet.gist.rs".to_string()
            })
        );
        // Same host, different ledger: the D9-reset arm — refuse, both laws.
        for law in [StampLaw::StrictId, StampLaw::TransitionalId] {
            assert_eq!(
                judge_stamp_id(Some("aaaa"), Some("bbbb"), law),
                Err(StampVerdict::LedgerIdMismatch {
                    file_id: "aaaa".to_string(),
                    current_id: "bbbb".to_string()
                })
            );
        }
        // File has an id, current server cannot attest: fail closed.
        assert_eq!(
            judge_stamp_id(Some("aaaa"), None, StampLaw::StrictId),
            Err(StampVerdict::UnattestedCurrent)
        );
        assert_eq!(
            judge_stamp_id(Some("aaaa"), None, StampLaw::TransitionalId),
            Err(StampVerdict::UnattestedCurrent)
        );
        // File has no id, server attests: STRICT refuses (the transition
        // window), TRANSITIONAL passes (the id upgrades at the next
        // stamped write).
        assert_eq!(
            judge_stamp_id(None, Some("bbbb"), StampLaw::StrictId),
            Err(StampVerdict::UnprovenFileId)
        );
        assert_eq!(
            judge_stamp_id(None, Some("bbbb"), StampLaw::TransitionalId),
            Ok(())
        );
        // Both sides unattested: the loopback dev-rig allowance.
        assert_eq!(judge_stamp_id(None, None, StampLaw::StrictId), Ok(()));
        assert_eq!(judge_stamp_id(None, None, StampLaw::TransitionalId), Ok(()));
    }

    #[test]
    fn every_verdict_prose_is_prefixed_and_names_the_remedy() {
        let dir = scratch("prose");
        let path = dir.join("kat_burn_ack.json");
        let cases = [
            StampVerdict::Unstamped,
            StampVerdict::HostMismatch {
                file: "devnet.gist.rs".to_string(),
                current: "mainnet.gist.rs".to_string(),
            },
            StampVerdict::LedgerIdMismatch {
                file_id: ledger_id_hex(ID_A),
                current_id: ledger_id_hex(ID_B),
            },
            StampVerdict::UnprovenFileId,
            StampVerdict::UnattestedCurrent,
            StampVerdict::Unreadable("not json".to_string()),
        ];
        for v in &cases {
            let p = v.prose("burn-ack", &path);
            assert!(p.starts_with("refused locally: "), "{p}");
            assert!(p.contains("adopt-state"), "{p}");
            assert!(p.contains(QUARANTINE_DIR), "{p}");
            assert!(p.contains("kat_burn_ack.json"), "{p}");
        }
        // The reset prose shows the SHORT ids (a 64-hex wall is unreadable
        // in a status line).
        let p = cases[2].prose("burn-ack", &path);
        assert!(p.contains(&ledger_id_hex(ID_A)[..8]), "{p}");
        assert!(
            !p.contains(&ledger_id_hex(ID_A)),
            "the full id must not render: {p}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stamp_host_canonicalizes_aliases_and_keeps_local_ports() {
        assert_eq!(
            stamp_host("https://ai.gist.rs").as_deref(),
            Some("mainnet.riir.ai"),
            "the legacy alias is ONE ledger with the pinned host"
        );
        assert_eq!(
            stamp_host("https://Mainnet.gist.rs/").as_deref(),
            Some("mainnet.riir.ai")
        );
        assert_eq!(
            stamp_host("https://devnet.gist.rs").as_deref(),
            Some("devnet.gist.rs")
        );
        assert_eq!(
            stamp_host("http://127.0.0.1:8787").as_deref(),
            Some("127.0.0.1"),
            "loopback is ONE dev plane — a port change is not a network change (the id half \
             distinguishes attesting workers)"
        );
        assert_eq!(
            stamp_host("http://localhost:9999").as_deref(),
            Some("localhost")
        );
        assert_eq!(
            stamp_host("https://proxy.example.com:8443").as_deref(),
            Some("proxy.example.com:8443"),
            "a custom host keeps its port — two proxies are two deployments"
        );
        assert!(stamp_host("not a url").is_none());
    }

    #[test]
    fn read_stamp_fields_reads_exactly_the_two_fields() {
        let dir = scratch("read");
        let path = dir.join("state.json");
        std::fs::write(
            &path,
            r#"{"service_host":"devnet.gist.rs","ledger_id":"aabb","charged_kat":5}"#,
        )
        .unwrap();
        let (h, id) = read_stamp_fields(&path).unwrap();
        assert_eq!(h.as_deref(), Some("devnet.gist.rs"));
        assert_eq!(id.as_deref(), Some("aabb"));
        // Absent fields read None — the unstamped shape.
        std::fs::write(&path, r#"{"charged_kat":5}"#).unwrap();
        let (h, id) = read_stamp_fields(&path).unwrap();
        assert_eq!(h, None);
        assert_eq!(id, None);
        // Unreadable → the error the caller refuses on.
        std::fs::write(&path, "not json").unwrap();
        assert!(read_stamp_fields(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quarantine_moves_twice_and_never_deletes() {
        let dir = scratch("quar");
        let src = dir.join("kat_burn_ack.json");
        std::fs::write(&src, r#"{"burn_kat_total":7}"#).unwrap();
        let root = dir.join("repo");
        std::fs::create_dir_all(&root).unwrap();

        let dest = quarantine_state_file(&src, &root, "kat_burn_ack.json").unwrap();
        assert_eq!(
            dest,
            root.join(".refine")
                .join(QUARANTINE_DIR)
                .join("kat_burn_ack.json")
        );
        assert!(!src.exists(), "the original moved");
        let moved = std::fs::read_to_string(&dest).unwrap();
        assert!(moved.contains("7"), "bytes survived the move: {moved}");
        let bak = std::fs::read_to_string(dest.with_extension("json.bak")).unwrap();
        assert_eq!(bak, moved, "the .bak copy carries the same bytes");

        // A second quarantine of the same rel name never clobbers: the
        // collision gets a suffixed name.
        std::fs::write(&src, r#"{"burn_kat_total":9}"#).unwrap();
        let dest2 = quarantine_state_file(&src, &root, "kat_burn_ack.json").unwrap();
        assert_ne!(dest2, dest, "the second move must not overwrite the first");
        assert!(std::fs::read_to_string(&dest).unwrap().contains("7"));

        // Subdirectory rels keep their shape under quarantine.
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let nested = dir.join("sub").join("charge.json");
        std::fs::write(&nested, "{}").unwrap();
        let dest3 = quarantine_state_file(&nested, &root, "corpus-lease/charge.json").unwrap();
        assert_eq!(
            dest3,
            root.join(".refine")
                .join(QUARANTINE_DIR)
                .join("corpus-lease")
                .join("charge.json")
        );
        assert!(!nested.exists());

        // `..` in rel refuses before touching anything.
        assert!(quarantine_state_file(&src, &root, "../escape.json").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Issue 221 (vendored): the round-trip needs the protocol tier, which
    // lives behind `kat_service` in riir-refine's crate root (in riir-kat
    // the reference was same-crate). The hex encoding itself stays pinned
    // ungated below.
    #[cfg(feature = "kat_service")]
    #[test]
    fn ledger_id_hex_round_trips_through_the_protocol_parse() {
        let hex = ledger_id_hex(ID_B);
        assert_eq!(hex.len(), 64);
        assert_eq!(
            crate::kat_protocol_lease::parse_service_ledger_id(
                format!(r#"{{"service":"kat","ledger_id":"{hex}"}}"#).as_bytes()
            ),
            Some(ID_B),
            "the state-file encoding is the service-info read's own form"
        );
    }

    #[test]
    fn ledger_id_hex_is_the_pinned_64hex_statefile_form() {
        let hex = ledger_id_hex(ID_B);
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(ledger_id_hex(ID_A), ledger_id_hex(ID_A), "stable per id");
        assert_ne!(ledger_id_hex(ID_A), ledger_id_hex(ID_B), "ids differ");
    }

    // ── T1: the per-net directory seam ─────────────────────────────────

    #[test]
    fn net_dir_keys_canonicalize_aliases_and_stay_fs_safe() {
        // One ledger, one directory — the alias collapses BEFORE keying.
        assert_eq!(net_dir_key("https://ai.gist.rs"), "mainnet.riir.ai");
        assert_eq!(net_dir_key("https://mainnet.gist.rs/"), "mainnet.riir.ai");
        assert_eq!(net_dir_key("https://riir.ai"), "mainnet.riir.ai");
        #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
        {
            assert_eq!(net_dir_key("https://devnet.ai.gist.rs"), "devnet.gist.rs");
        }
        // Loopback is ONE dev plane — port-less (the port is not a net).
        assert_eq!(net_dir_key("http://127.0.0.1:8787"), "127.0.0.1");
        // Custom hosts keep host+port, `:` → `_` (Windows-safe).
        assert_eq!(
            net_dir_key("https://proxy.example.com:8443"),
            "proxy.example.com_8443"
        );
    }

    #[test]
    #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
    fn net_state_dir_shape_and_url_form_agree() {
        let root = std::path::Path::new("/tmp/whatever");
        assert_eq!(
            net_state_dir(root, "devnet.gist.rs"),
            root.join(".refine")
                .join(NET_DIR_NAME)
                .join("devnet.gist.rs")
        );
        // The url form is the key form canonicalized — one derivation.
        assert_eq!(
            net_state_dir_for_url(root, "https://devnet.ai.gist.rs"),
            net_state_dir(root, "devnet.gist.rs")
        );
    }

    #[test]
    fn net_placement_and_stamp_host_cannot_disagree() {
        // The dir key IS the stamp host, fs-safe — the placement half and
        // the in-file stamp derive from one call, and this pins the
        // equivalence the T2/T4 story rests on (a file must sit in the
        // directory of the host its stamp names).
        for url in [
            "https://ai.gist.rs",
            "https://devnet.ai.gist.rs",
            "http://127.0.0.1:8787",
            "https://proxy.example.com:8443",
        ] {
            let host = stamp_host(url).expect("test urls resolve");
            assert_eq!(net_dir_key(url), host.replace(':', "_"), "{url}");
        }
    }
}
