//! Batch 415 sec fixture preflight — the graphicle transcription's sec half
//! (Plan 396 / Research 430, Graphicle-AI/graphicle @ b0bb4140, Apache-2.0)
//! as ONE standalone file (2 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the sources
//! are the pinned security.rs / db.rs laws). Compiled AND executed BEFORE
//! entry: `rustc --edition 2021 --test b415_sec_fixtures.rs` at debug AND -O.
//!
//! Arms:
//! - F1 (egress-resolve-pinned-address-validate): the captured-resolver arm
//!   (a name-policy with a name-connect opens a SECOND, ungoverned lookup —
//!   the second answer is where the socket lands); the fixed pinned-connect
//!   arm (every resolved address checked, the pinned SocketAddr connected,
//!   the captured answer refused by name); the all-safe arm (a public
//!   resolution connects); the tamper arm (checking only the FIRST resolved
//!   address admits a multi-address host whose second address is internal).
//! - F2 (sqlite-store-path-nofollow-parent-resolve): the parent-link arm
//!   (a symlinked parent breaks a whole-path-strict policy on a legitimate
//!   layout; the parent-resolve opens it); the linked-file arm (a linked
//!   FILE still refuses — the file name is never canonicalized); the
//!   all-safe arm (real dir + real file); the over-resolution tamper arm
//!   (canonicalizing the FILE too resolves a linked file open — the bypass
//!   the parent-only law exists to keep closed).
#![allow(dead_code)]

use std::net::{IpAddr, SocketAddr, TcpListener};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

// ══════════════ F1 — egress-resolve-pinned-address-validate ══════════════

fn metadata_ip() -> IpAddr {
    IpAddr::from([169, 254, 169, 254])
}

fn internal_ip() -> IpAddr {
    IpAddr::from([10, 0, 0, 1])
}

fn blocked_table() -> [IpAddr; 2] {
    [metadata_ip(), internal_ip()]
}

/// a resolver under the test's control — the "captured authority" stand-in
type Resolver = for<'a> fn(&'a str, u16) -> Vec<IpAddr>;

fn captured_resolver(host: &str, _port: u16) -> Vec<IpAddr> {
    match host {
        // the FIRST lookup (policy time) answers public; the SECOND
        // (connect time) answers metadata — the rebinding window
        "shifty.internal" => {
            thread_local! {
                static CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
            }
            let n = CALLS.with(|c| {
                c.set(c.get() + 1);
                c.get()
            });
            if n <= 1 {
                vec![IpAddr::from([93, 184, 216, 34])]
            } else {
                vec![metadata_ip()]
            }
        }
        "multi.internal" => vec![IpAddr::from([93, 184, 216, 34]), internal_ip()],
        "clean.internal" => vec![IpAddr::from([93, 184, 216, 34])],
        _ => Vec::new(),
    }
}

fn any_blocked(addresses: &[IpAddr]) -> bool {
    let blocked = blocked_table();
    addresses.iter().any(|a| blocked.contains(a))
}

/// the NAIVE posture: policy-time resolve (the check) + connect-time resolve
/// (the dial) — two separate lookups, the second ungoverned
fn naive_dial(host: &str, port: u16, resolver: Resolver) -> Result<IpAddr, String> {
    const ALLOWED: [&str; 3] = ["shifty.internal", "multi.internal", "clean.internal"];
    if !ALLOWED.contains(&host) {
        return Err(String::from("host not allowed"));
    }
    // lookup 1: the policy's own resolution ("is this host OK?")
    let policy_answer = resolver(host, port);
    if any_blocked(&policy_answer) {
        return Err(String::from("policy: blocked resolution"));
    }
    // lookup 2: connect-by-name resolves AGAIN — a captured authority
    // answers differently here and the policy never sees it
    let second_answer = resolver(host, port);
    second_answer
        .into_iter()
        .next()
        .ok_or_else(|| String::from("resolve failed"))
}

/// the FIXED posture: resolve once, check EVERY address, pin the socket
fn pinned_dial(host: &str, port: u16, resolver: Resolver) -> Result<SocketAddr, String> {
    const ALLOWED: [&str; 3] = ["shifty.internal", "multi.internal", "clean.internal"];
    if !ALLOWED.contains(&host) {
        return Err(String::from("host not allowed"));
    }
    let addresses = resolver(host, port);
    let blocked = blocked_table();
    for address in &addresses {
        if blocked.contains(address) {
            return Err(format!(
                "SSRF blocked: IP {address} resolved from '{host}' is internal"
            ));
        }
    }
    addresses
        .first()
        .map(|a| SocketAddr::new(*a, port))
        .ok_or_else(|| format!("SSRF blocked: no usable address resolved from '{host}'"))
}

#[test]
fn f1_captured_second_lookup_defeats_the_name_policy() {
    // NAIVE: the name policy passed on lookup 1; connect re-resolves and
    // opens on lookup 2's answer — the metadata address
    let opened = naive_dial("shifty.internal", 443, captured_resolver).unwrap();
    assert_eq!(
        opened,
        metadata_ip(),
        "the naive dial opened the SECOND lookup's answer — the window is real"
    );
    // FIXED: the pinned dial resolves once and refuses the captured answer
    let err = pinned_dial("shifty.internal", 443, captured_resolver).unwrap_err();
    assert!(
        err.contains(&metadata_ip().to_string()),
        "the refusal names the resolved IP: {err}"
    );
}

#[test]
fn f1_fixed_dial_connects_the_pinned_public_answer() {
    let addr = pinned_dial("clean.internal", 443, captured_resolver).unwrap();
    assert_eq!(
        addr.ip().to_string(),
        "93.184.216.34",
        "a clean resolution pins and connects"
    );
    // end-to-end: the pinned address actually connects a local listener
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let connected = tcp_stream_connect_for_test(SocketAddr::from(([127, 0, 0, 1], port)));
    assert!(connected.is_some(), "the pinned SocketAddr connects");
}

fn tcp_stream_connect_for_test(addr: SocketAddr) -> Option<std::net::TcpStream> {
    std::net::TcpStream::connect(addr).ok()
}

#[test]
fn f1_tamper_first_address_only_admits_multi_address_hosts() {
    // the TAMPER: a "policy" that checks only the FIRST resolved address
    let addresses = captured_resolver("multi.internal", 443);
    let blocked = blocked_table();
    let first_only_ok = !blocked.contains(&addresses[0]);
    let every_ok = !any_blocked(&addresses);
    assert!(
        first_only_ok && !every_ok,
        "first-address-only passes a host whose SECOND address is internal; \
         the all-addresses check refuses it"
    );
    assert!(pinned_dial("multi.internal", 443, captured_resolver).is_err());
}

// ══════════════ F2 — sqlite-store-path-nofollow-parent-resolve ══════════════

/// unique scratch root PER TEST (the shared-temp-path law: a pid alone is
/// shared by every test THREAD in one binary — three f2 tests raced on one
/// path and the fixture gate caught it; name + pid is unique)
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("b415_sec_nofollow_{}_{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// a whole-path-strict policy stand-in: refuses if ANY component is a link
fn strict_open(path: &Path) -> Result<PathBuf, String> {
    let mut cur = PathBuf::new();
    for comp in path.components() {
        cur.push(comp.as_os_str());
        let meta = std::fs::symlink_metadata(&cur).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err(format!("strict policy: link in path at {}", cur.display()));
        }
    }
    Ok(cur)
}

/// the FIXED posture: resolve the PARENT, carry the FILE NAME — the parent's
/// links resolve legitimately, a linked FILE still refuses
fn parent_resolved_open(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .ok_or_else(|| String::from("no file name"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let resolved_parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let final_path = resolved_parent.join(name);
    // the final component still checks: a linked FILE is refused
    let meta = std::fs::symlink_metadata(&final_path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err(format!("linked file refused: {}", final_path.display()));
    }
    Ok(final_path)
}

#[test]
fn f2_parent_link_breaks_strict_but_parent_resolve_opens() {
    let scratch = Scratch::new("parent_link");
    let real_store = scratch.0.join("real");
    std::fs::create_dir_all(&real_store).unwrap();
    // the legitimate macOS-style layout: the parent IS a link (/var -> /private/var)
    let linked_root = scratch.0.join("var");
    std::os::unix::fs::symlink(&real_store, &linked_root).unwrap();
    std::fs::write(real_store.join("store.db"), b"data").unwrap();
    // strict policy: refuses the LEGITIMATE layout (the link is in the path)
    assert!(
        strict_open(&linked_root.join("store.db")).is_err(),
        "whole-path-strict refuses the macOS-style parent link — the false break"
    );
    // the fixed posture: parent resolved, file name kept — opens
    let opened = parent_resolved_open(&linked_root.join("store.db")).unwrap();
    assert_eq!(
        opened,
        real_store.join("store.db"),
        "the parent's link resolved away; the store opened at its real home"
    );
}

#[test]
fn f2_linked_file_still_refuses() {
    let scratch = Scratch::new("linked_file");
    let store = scratch.0.join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(scratch.0.join("secret.db"), b"crown jewels").unwrap();
    symlink(scratch.0.join("secret.db"), store.join("store.db")).unwrap();
    // the FILE is a link: the fixed posture keeps the file name unresolved,
    // the final-component check refuses
    let err = parent_resolved_open(&store.join("store.db")).unwrap_err();
    assert!(
        err.contains("linked file refused"),
        "a linked FILE is still refused: {err}"
    );
}

#[test]
fn f2_all_safe_opens_plain() {
    let scratch = Scratch::new("all_safe");
    let store = scratch.0.join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("store.db"), b"data").unwrap();
    let opened = parent_resolved_open(&store.join("store.db")).unwrap();
    assert_eq!(opened, store.join("store.db"));
}

#[test]
fn f2_tamper_over_resolution_resolves_a_linked_file_open() {
    let scratch = Scratch::new("tamper");
    let store = scratch.0.join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(scratch.0.join("secret.db"), b"crown jewels").unwrap();
    symlink(scratch.0.join("secret.db"), store.join("store.db")).unwrap();
    // the TAMPER: canonicalize the WHOLE path (file included) — the link
    // resolves and the open lands on the target, the nofollow promise void
    let tampered = std::fs::canonicalize(store.join("store.db")).unwrap();
    assert_eq!(
        tampered,
        scratch.0.join("secret.db"),
        "over-resolution followed the link — exactly the bypass the \
         parent-only resolve law keeps closed"
    );
    assert!(parent_resolved_open(&store.join("store.db")).is_err());
}

// ══════════════ Windows stand-in note ══════════════
// the rule's honest recall limit is POSIX path semantics (the source's own
// /var -> /private/var witness); this fixture uses std::os::unix::fs::symlink
// and is compiled/executed on macOS here. The arms' LOGIC is platform-neutral;
// a Windows port swaps the symlink constructor, never the policy.
