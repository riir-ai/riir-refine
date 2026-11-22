//! Batch 398 rust_perf fixture preflight — the prefix-cache commit law
//! (Plan 398 / Research 432, ddalcu/mlx-serve @ 96311b26 MIT) as ONE
//! standalone file (1 rule, 4 arms).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is Zig: `src/engines/mlx/cache/prefix_cache.zig`). Compiled
//! AND executed BEFORE entry:
//! `rustc --edition 2021 --test b398_perf_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F13a the defect arm: every cancelled prefill commits (prologues
//!   pollute the LRU) and a budget decline exits via plain return — the
//!   caller logs "committed N/M" for a commit that never happened.
//! - F13b the fixed arm: the COMMIT-time worthiness floor skips the
//!   expensive capture below 256 tokens; the outcome is TYPED (Ok /
//!   DiskOnly / KeptResident / Declined) and Ok carries the post-trim
//!   EFFECTIVE length, never the candidate's forwarded length.
//! - F13c the all-safe arm: a caller that logs the typed truth never
//!   misreports a decline.
//! - F13d the tampered arm: an Ok status carrying the candidate's
//!   FORWARDED length (not the post-trim effective length) reds — the
//!   scheduler would trust a longer cache than exists.

// ────────────────────────── the model ──────────────────────────

const MIN_CANCELLED_COMMIT_TOKENS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitStatus {
    /// committed at this many tokens — the post-trim EFFECTIVE length
    Ok(usize),
    DiskOnly(usize),
    KeptResident(usize),
    Declined,
}

struct Entry {
    key_len: usize,
    bytes: usize,
}

struct Cache {
    entries: Vec<Entry>,
    budget: usize,
    /// counts capture_snapshot calls — the expensive side
    captures: usize,
}

impl Cache {
    fn new(budget: usize) -> Self {
        Cache { entries: Vec::new(), budget, captures: 0 }
    }

    fn capture_snapshot(&mut self) {
        self.captures += 1;
    }

    fn resident_bytes(&self) -> usize {
        self.entries.iter().map(|e| e.bytes).sum()
    }
}

// ────────────────────────── the arms ──────────────────────────

#[test]
fn f13a_the_defect_commits_everything_and_lies_on_decline() {
    // the BUGGY arm: no commit-time floor (prologues commit), and the
    // budget decline is a plain bool — the caller cannot tell a decline
    // from a success-shaped answer.
    let mut cache = Cache::new(8_192);
    let mut commits_logged = 0;
    let commit = |c: &mut Cache, tokens: usize| -> bool {
        c.capture_snapshot();
        c.entries.push(Entry { key_len: tokens, bytes: 4_096 });
        true
    };
    // chat-template prologues (8-12 tokens) commit as "reusable"
    for tokens in [8usize, 12, 4] {
        assert!(commit(&mut cache, tokens));
        commits_logged += 1;
    }
    assert_eq!(cache.entries.len(), 3, "prologue pollution committed");
    assert_eq!(cache.captures, 3, "the expensive capture paid per prologue");
    // the budget decline: plain return — the caller's log claims a commit
    let commit_with_budget = |c: &mut Cache, _tokens: usize| -> bool {
        if c.resident_bytes() + 4_096 > c.budget {
            return false; // silent: the caller logs "committed" anyway
        }
        c.capture_snapshot();
        c.entries.push(Entry { key_len: _tokens, bytes: 4_096 });
        true
    };
    let declined = commit_with_budget(&mut cache, 300);
    assert!(!declined);
    commits_logged += 1; // the lying scheduler logs it regardless
    assert_eq!(commits_logged, 4, "the log claims 4 commits; 3 happened");
    assert_eq!(cache.captures, 3);
}

#[test]
fn f13b_the_fixed_arm_prices_worthiness_and_types_the_truth() {
    // the FIXED arm: the floor skips the capture; the decline is a value.
    let mut cache = Cache::new(4_096);
    let commit = |c: &mut Cache, tokens: usize, effective: usize| -> CommitStatus {
        if tokens < MIN_CANCELLED_COMMIT_TOKENS {
            return CommitStatus::Declined; // no capture: LRU pollution skipped
        }
        if c.resident_bytes() + 4_096 > c.budget {
            // KeptResident only when a resident entry already covers it
            if c.entries.iter().any(|e| e.key_len >= tokens) {
                return CommitStatus::KeptResident(tokens);
            }
            return CommitStatus::Declined;
        }
        c.capture_snapshot();
        c.entries.push(Entry { key_len: effective, bytes: 4_096 });
        CommitStatus::Ok(effective) // the post-trim EFFECTIVE length
    };
    // the prologue: declined, no capture paid
    assert_eq!(commit(&mut cache, 12, 12), CommitStatus::Declined);
    assert_eq!(cache.captures, 0);
    // the honest candidate commits at the EFFECTIVE length
    assert_eq!(commit(&mut cache, 400, 380), CommitStatus::Ok(380));
    assert_eq!(cache.captures, 1);
    // the budget decline with a covering resident entry (380 covers
    // 360): KeptResident — the longer candidate discarded
    assert_eq!(commit(&mut cache, 360, 360), CommitStatus::KeptResident(360));
    assert_eq!(cache.captures, 1, "no second capture for a kept-resident hit");
    // the budget decline with no cover (900 > 380): Declined — a VALUE
    // the caller logs
    assert_eq!(commit(&mut cache, 900, 900), CommitStatus::Declined);
}

#[test]
fn f13c_the_caller_logs_the_typed_truth() {
    // the all-safe arm: a caller that renders the status never
    // misreports a decline.
    fn render(status: CommitStatus, log: &mut Vec<String>) {
        match status {
            CommitStatus::Ok(n) => log.push(format!("committed {n}")),
            CommitStatus::DiskOnly(n) => log.push(format!("disk-only {n}")),
            CommitStatus::KeptResident(n) => log.push(format!("kept-resident {n}")),
            CommitStatus::Declined => log.push("declined".to_string()),
        }
    }
    let mut log = Vec::new();
    render(CommitStatus::Ok(380), &mut log);
    render(CommitStatus::Declined, &mut log);
    assert_eq!(log[0], "committed 380");
    assert_eq!(log[1], "declined", "the decline names itself");
}

#[test]
fn f13d_the_forwarded_length_lie_reds() {
    // the TAMPERED arm: Ok carrying the candidate's FORWARDED length
    // (not the post-trim effective length) — the scheduler would trust
    // a longer cache than exists and skip prefill work it must pay.
    let forwarded = 400;
    let effective_after_trim = 380;
    let lying = CommitStatus::Ok(forwarded);
    let honest = CommitStatus::Ok(effective_after_trim);
    assert_ne!(lying, honest, "the lie is a different value");
    let trusted_len = |s: &CommitStatus| match *s {
        CommitStatus::Ok(n) | CommitStatus::DiskOnly(n) | CommitStatus::KeptResident(n) => n,
        CommitStatus::Declined => 0,
    };
    // the scheduler skipping prefill of `forwarded` would re-forward the
    // trimmed tail blind: the pin is the EFFECTIVE length, always.
    assert_eq!(trusted_len(&honest), effective_after_trim);
    assert!(trusted_len(&lying) > trusted_len(&honest), "the lie overstates");
}
