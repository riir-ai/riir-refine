//! Batch 378 sec fixture preflight — the coganchor fence-construction
//! law (Plan 393 / Research 427, `agent-child-unsandboxed-account-
//! authority`) as ONE standalone file (1 rule, 4 arms).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! sources are humanfia/humanize @ `565e1205` + humanfia/qda @
//! `dc39a02b`, both Apache-2.0: coganchor's Landlock+loopback-proxy
//! fence is the FIX posture, qda's "runs as you, outside any sandbox"
//! README warning the DEFECT; the Rust arms below are never pasted
//! source). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b393_fixtures.rs`
//!
//! Arms (the b342 discipline — every classifier deterministic, every
//! assert pair names the honest and the adversary arm):
//! - F3a the defect arm: an unfenced agent-child spawn — the child's
//!   effective authority covers every path and host the account can
//!   reach (the qda `isolation.level: none` shape).
//! - F3b the fixed arm: the fence-construction posture — the child's
//!   effective authority is the DRAWN GRANT (scoped read/write sets,
//!   the egress proxy, the scratch); an out-of-scope write is refused
//!   fail-closed, and a carve-out the wall cannot hold REFUSES the
//!   spawn (never widens).
//! - F3c the all-safe arms: trusted build tooling (a closed verb
//!   grammar, no autonomy grant) and the user-initiated interactive
//!   shell (per-command approval present) both run UNFENCED correctly
//!   — the predicate must not fire on them.
//! - F3d the tampered arm: a fence that WIDENS an unholdable grant
//!   instead of refusing (the anti-law) — and the stand-in face: a
//!   stub allowed to run reads exit-0-as-success, so the killed
//!   verdict is the only honest one.

// ═══════════════════════════════════════════════════════════════════════
// the spawn-shape predicate (product-name-free, the plan's demotion
// criterion answered)
// ═══════════════════════════════════════════════════════════════════════

/// How the spawner hands the child its instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputShape {
    /// a task specification the spawner did not enumerate (a goal
    /// prompt / task file — the child decides its own commands)
    OpenTask,
    /// a fixed verb the caller enumerated at authoring time
    ClosedVerb,
}

/// Whether per-command approval exists for the child's follow-on
/// spawns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Approval {
    Absent, // the autonomy grant
    PerCommand,
}

/// The spawn-shape verdict: BOTH structural properties present = the
/// fence-construction posture applies.
#[derive(Debug, PartialEq, Eq)]
enum SpawnClass {
    AgentDriven,
    TrustedTooling,
    InteractiveShell,
}

fn classify(input: InputShape, approval: Approval) -> SpawnClass {
    match (input, approval) {
        (InputShape::OpenTask, Approval::Absent) => SpawnClass::AgentDriven,
        (InputShape::ClosedVerb, Approval::Absent) => SpawnClass::TrustedTooling,
        (InputShape::ClosedVerb, Approval::PerCommand) => SpawnClass::InteractiveShell,
        // an open task with per-command approval is still supervised —
        // the human gates every follow-on command; treat as interactive
        (InputShape::OpenTask, Approval::PerCommand) => SpawnClass::InteractiveShell,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// the authority model: what the child can actually touch
// ═══════════════════════════════════════════════════════════════════════

#[derive(Default)]
struct Account {
    /// every path the invoking account can write
    writable: Vec<String>,
    /// hosts reachable without a proxy (an account-shaped egress list;
    /// `*` = anything)
    #[allow(dead_code)] // the shape the unfenced child inherits
    direct_hosts: Vec<String>,
}

#[derive(Default)]
struct ChildAuthority {
    writable: Vec<String>,
    /// the paths the drawn wall holds OUT (the carve-outs) — checked
    /// FIRST: a deny wins over any grant, fail-closed
    denied: Vec<String>,
    egress: Vec<String>,
}

impl ChildAuthority {
    fn can_write(&self, path: &str) -> bool {
        // the deny check first: a carved-out path stays out whatever
        // the grants say (the fence failing closed)
        if self
            .denied
            .iter()
            .any(|d| path == d || path.starts_with(&format!("{d}/")))
        {
            return false;
        }
        self.writable
            .iter()
            .any(|w| path == w || path.starts_with(&format!("{w}/")))
    }
    fn can_reach(&self, host: &str) -> bool {
        self.egress.iter().any(|h| h == host || h == "*")
    }
}

/// F3a — the DEFECT: the unfenced spawn. The child inherits the
/// invoking account's whole authority (the qda `none` shape: "the
/// build and command jobs it submits run as you, outside any
/// sandbox, and can change or delete any file your account can
/// write").
fn spawn_unfenced(account: &Account) -> ChildAuthority {
    ChildAuthority {
        writable: account.writable.clone(),
        denied: Vec::new(),            // no wall exists: nothing is carved out
        egress: vec!["*".to_string()], // unshaped network
    }
}

/// The fence as drawn: scoped grants + the carve-out algebra + the
/// egress proxy. `holdable = false` models a grant this wall cannot
/// hold — the fence REFUSES the spawn, never widens.
struct Fence {
    /// the read scope (carried for the authority model's completeness;
    /// the fixture's write-side asserts exercise the wall)
    #[allow(dead_code)]
    read: Vec<String>,
    write: Vec<String>,
    /// the carve-outs the wall holds (the taken-apart algebra's
    /// effective result: what nothing may write)
    denies: Vec<String>,
    proxy_hosts: Vec<String>, // what the loopback proxy admits
    scratch: String,
}

enum FenceVerdict {
    Drawn(Fence),
    Refused(&'static str),
}

/// The grants as asked.
struct Grants {
    read: Vec<String>,
    write: Vec<String>,
    carve_outs: Vec<String>,
    holdable: bool,
}

/// The fence construction: what is granted is what is there as the
/// wall is drawn; a carve-out that cannot be held is a program that
/// does not run.
fn draw_fence(grants: &Grants) -> FenceVerdict {
    if !grants.holdable {
        return FenceVerdict::Refused("a grant that cannot be held is refused, never widened");
    }
    // the write scope is the asked grants MINUS the carve-outs (the
    // taken-apart algebra: an entry made later in a carved-out
    // directory is in nothing granted)
    let write: Vec<String> = grants
        .write
        .iter()
        .filter(|w| {
            !grants
                .carve_outs
                .iter()
                .any(|c| w.as_str() == c.as_str() || w.starts_with(&format!("{c}/")))
        })
        .cloned()
        .collect();
    FenceVerdict::Drawn(Fence {
        read: grants.read.clone(),
        write,
        denies: grants.carve_outs.clone(),
        proxy_hosts: vec!["model-api.example".to_string()],
        scratch: "/tmp/agent-fence-1a2b3c4d".to_string(),
    })
}

/// F3b — the FIXED spawn: the child's authority is the drawn fence.
fn spawn_fenced(fence: &Fence, _account: &Account) -> ChildAuthority {
    let mut egress: Vec<String> = fence
        .proxy_hosts
        .iter()
        .map(|h| format!("via-proxy:{h}"))
        .collect();
    egress.push("proxy-listener".to_string()); // the one way out
    ChildAuthority {
        writable: {
            let mut w = fence.write.clone();
            w.push(fence.scratch.clone());
            w
        },
        denied: fence.denies.clone(),
        egress,
    }
}

/// The stand-in face: what the CALLER sees when a parked stub is
/// allowed to run instead of killed — its exit 0 is the same byte the
/// genuine success would have delivered.
fn stub_exit_status() -> u8 {
    0 // the stub ran, the kernel saw exit 0
}

fn genuine_success_exit_status() -> u8 {
    0 // the real remote completion, rewritten faithfully
}

/// The genuine completion, as the caller reads it (the variant the
/// tampered path IMITATES).
fn genuine_completion() -> RemoteOutcome {
    RemoteOutcome::Completed(genuine_success_exit_status())
}

#[derive(Debug, PartialEq, Eq)]
enum RemoteOutcome {
    Completed(u8),     // the remote status, faithfully rewritten
    StubReadAsSuccess, // the stub's exit 0 — the lie
    Killed,            // the only honest verdict when completion is unknown
}

/// The tampered completion path: release a stub whose remote status
/// never arrived.
fn release_stub_allow_run() -> RemoteOutcome {
    RemoteOutcome::StubReadAsSuccess // the caller reads "success"
}

/// The honest completion path: anything that fails once the exec may
/// have been let through KILLS the process.
fn release_stub_honest() -> RemoteOutcome {
    RemoteOutcome::Killed
}

#[test]
fn f3a_the_unfenced_spawn_holds_the_whole_account() {
    let account = Account {
        writable: vec![
            "/home/dev".to_string(),
            "/srv/shared".to_string(),
            "/etc".to_string(), // the account can sudo-write here
        ],
        direct_hosts: vec!["*".to_string()],
    };

    let child = spawn_unfenced(&account);
    // the defect, demonstrated: the agent-driven child can write the
    // account's EVERY path — settings, keys, the shared disk — and
    // reach any host
    assert!(
        child.can_write("/home/dev/.ssh/id_ed25519"),
        "the unfenced child reaches the account's keys"
    );
    assert!(
        child.can_write("/etc/hosts"),
        "the unfenced child reaches system files"
    );
    assert!(
        child.can_reach("internal-secrets.corp"),
        "the unfenced child reaches any host"
    );
}

#[test]
fn f3b_the_fenced_spawn_scopes_and_refuses_fail_closed() {
    let account = Account {
        writable: vec!["/home/dev".to_string(), "/srv/shared".to_string()],
        direct_hosts: vec!["*".to_string()],
    };
    let grants = Grants {
        read: vec!["/home/dev/project".to_string()],
        write: vec!["/home/dev/project".to_string()],
        carve_outs: vec!["/home/dev/project/secrets".to_string()],
        holdable: true,
    };

    let fence = match draw_fence(&grants) {
        FenceVerdict::Drawn(f) => f,
        FenceVerdict::Refused(why) => panic!("the holdable grant must draw: {why}"),
    };
    let child = spawn_fenced(&fence, &account);

    // the granted workdir IS writable
    assert!(
        child.can_write("/home/dev/project/src/main.rs"),
        "the granted workdir is writable"
    );
    // the carve-out is NOT (the taken-apart algebra holds it)
    assert!(
        !child.can_write("/home/dev/project/secrets/api.key"),
        "the carve-out must stay out of reach — the fence failing closed"
    );
    // everything OUTSIDE the grant is NOT writable — even paths the
    // ACCOUNT could write (the scoping is the whole point)
    assert!(
        !child.can_write("/home/dev/.ssh/id_ed25519"),
        "outside the grant = out of reach"
    );
    assert!(
        !child.can_write("/srv/shared"),
        "the account's other writable paths stay out"
    );
    // the egress is the proxy's shape, not the account's
    assert!(
        !child.can_reach("internal-secrets.corp"),
        "direct egress is gone"
    );
    assert!(
        child.can_reach("via-proxy:model-api.example"),
        "the model API rides the proxy"
    );
}

#[test]
fn f3c_trusted_tooling_and_interactive_shells_run_unfenced() {
    // (a) cargo-class trusted tooling: closed verb grammar, no
    // autonomy grant — the predicate must NOT classify it agent-driven
    assert_eq!(
        classify(InputShape::ClosedVerb, Approval::Absent),
        SpawnClass::TrustedTooling,
        "a closed-verb tool is trusted tooling, never agent-driven"
    );
    // (b) the user-initiated interactive shell: per-command approval —
    // the human IS the gate
    assert_eq!(
        classify(InputShape::ClosedVerb, Approval::PerCommand),
        SpawnClass::InteractiveShell,
        "an interactive shell is the user's own authority"
    );
    assert_eq!(
        classify(InputShape::OpenTask, Approval::PerCommand),
        SpawnClass::InteractiveShell,
        "an open task behind per-command approval is still supervised"
    );
    // (c) and only the open-input × autonomy-grant shape is the class
    assert_eq!(
        classify(InputShape::OpenTask, Approval::Absent),
        SpawnClass::AgentDriven,
        "open task + bypassed approvals = the agent-driven class"
    );
}

#[test]
fn f3d_tampered_fence_widens_and_the_stub_lies() {
    let account = Account {
        writable: vec!["/home/dev".to_string()],
        direct_hosts: vec!["*".to_string()],
    };

    // TAMPER 1: the unholdable grant WIDENED instead of refused (the
    // anti-law — the child runs with the account's whole authority
    // while the caller believes it held)
    let grants = Grants {
        read: vec!["/home/dev/project".to_string()],
        write: vec!["/home/dev/project".to_string()],
        carve_outs: vec!["/home/dev/project/secrets".to_string()],
        holdable: false, // this wall cannot hold the carve-out
    };
    let widened = spawn_unfenced(&account); // what "widening" amounts to
    assert!(
        widened.can_write("/home/dev/project/secrets/api.key"),
        "the widened grant exposes exactly the carve-out it could not hold"
    );
    // and the honest fence REFUSES this spawn
    assert!(matches!(
        draw_fence(&grants),
        FenceVerdict::Refused("a grant that cannot be held is refused, never widened")
    ));

    // TAMPER 2: the stand-in stub allowed to run — its exit status is
    // byte-identical to a genuine success (the remote-status-standin
    // face: the caller cannot tell them apart)
    assert_eq!(
        stub_exit_status(),
        genuine_success_exit_status(),
        "the stub's exit 0 is INDISTINGUISHABLE from a genuine success — that is the harm"
    );
    assert_eq!(
        release_stub_allow_run(),
        RemoteOutcome::StubReadAsSuccess,
        "the stub's exit 0 is the lie the face names"
    );
    // the genuine completion, for contrast — the variant the stub
    // IMITATES (the caller reads the same thing from both)
    assert_eq!(genuine_completion(), RemoteOutcome::Completed(0));
    // the honest path kills
    assert_eq!(release_stub_honest(), RemoteOutcome::Killed);
    // the two verdicts disagree — the stub arm FAILS by construction
    assert_ne!(
        release_stub_allow_run(),
        release_stub_honest(),
        "the tampered and honest completion paths must disagree"
    );
}
