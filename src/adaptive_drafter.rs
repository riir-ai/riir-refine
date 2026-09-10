//! Stage 2 adaptive drafter — verified-fix pool, keyed by `(rule_id, buggy_hash)`.
//!
//! Mirrors `AdaptiveQuestDrafter` from
//! `riir-games-quest/quest_grammar/plasma_draft.rs`: a hash-keyed lookup over
//! a pool of "verified" outputs. The pool is populated OFF the hot path by
//! the tuning loop (see `HealPipeline::tune_against_fixture`); the hot path
//! is a single hash lookup.
//!
//! ## Two-stage routing (Issue 010 fix direction, `quest_grammar` precedent)
//!
//! ```text
//! draft_fix(rule, buggy):
//!   1. adaptive.lookup(rule, buggy)  → Some(fix)   // Stage 2 hit
//!   2. ternary.draft_fix(...)        → fallback    // Stage 1 otherwise
//! ```
//!
//! Stage 1 (`TernaryDrafter`) is always-on + modelless + ~µs but cannot
//! generalize (Issue 010: returns verbatim corpus text). Stage 2 serves
//! verified fixes the tuning loop has admitted — the pool is empty until
//! `tune_against_fixture` populates it.
//!
//! ## Domain-agnostic (Plan 001 Phase 1)
//!
//! Renamed from `ClippyAdaptiveDrafter` → `AdaptiveDrafter`. The pool stores
//! `String` (verified fix code) keyed by `BLAKE3(rule_id, buggy_code)`. No
//! domain-specific types — works with any domain.
//!
//! ## Determinism
//!
//! The pool key is `BLAKE3(rule_id \0 buggy_code)` — deterministic across
//! runs. The pool itself is a `papaya::HashMap` (lock-free) so concurrent
//! tuning + serving is safe.
//!
//! ## Alloc-free hot path
//!
//! `lookup` returns `Option<String>` (one String clone on hit, zero on miss).
//! The hash + table probe allocate no scratch.

use blake3::Hasher;
use papaya::HashMap;

/// Key for the adaptive pool: BLAKE3(`rule_id \0 buggy_code`) → 8 bytes.
type PoolKey = [u8; 8];

/// Compute the pool key for a `(rule_id, buggy_code)` pair.
///
/// Public so the tuning loop + tests can compute the same key the drafter
/// will look up.
#[must_use]
#[inline]
pub fn pool_key(rule_id: &str, buggy_code: &str) -> PoolKey {
    let mut hasher = Hasher::new();
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\x00");
    hasher.update(buggy_code.as_bytes());
    let mut key = [0u8; 8];
    key.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    key
}

/// Stage 2 adaptive drafter — pool of verified fixes keyed by input.
///
/// Issue 010 fix direction (mirrors `AdaptiveQuestDrafter`):
/// - Empty pool → caller routes to Stage 1 (`TernaryDrafter`).
/// - Populated pool → `lookup` returns the verified fix in O(1).
///
/// The pool is **populated OFF the hot path** by `HealPipeline::tune_against_fixture`,
/// which re-runs the domain validator (~130 ms for clippy — acceptable for tuning,
/// NOT for the hot path) to admit only fixes that actually resolve the rule.
#[derive(Debug, Default)]
pub struct AdaptiveDrafter {
    /// `papaya::HashMap` for lock-free concurrent tuning + serving.
    /// Key: `BLAKE3(rule_id, buggy_code)`. Value: the verified fix code.
    pool: HashMap<PoolKey, String>,
}

impl AdaptiveDrafter {
    /// Construct an empty adaptive drafter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a verified fix for `(rule_id, buggy_code)`.
    ///
    /// Returns `Some(verified_fix)` on hit, `None` on miss (caller falls back
    /// to Stage 1). O(1) hash lookup — no allocation on miss, one String clone
    /// on hit.
    #[must_use]
    pub fn lookup(&self, rule_id: &str, buggy_code: &str) -> Option<String> {
        let key = pool_key(rule_id, buggy_code);
        self.pool.pin().get(&key).cloned()
    }

    /// Insert a verified fix into the pool. Called by the tuning loop after
    /// the domain validator confirms the fix resolves the rule.
    pub fn insert(&self, rule_id: &str, buggy_code: &str, verified_fix: String) {
        let key = pool_key(rule_id, buggy_code);
        self.pool.pin().insert(key, verified_fix);
    }

    /// Whether the pool has any entries. Empty → Stage 1 only.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pool.pin().is_empty()
    }

    /// Number of verified fixes in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pool.pin().len()
    }

    /// Clear the pool (revert to Stage 1 only). Mirrors `clear_seal_corpus`.
    pub fn clear(&self) {
        self.pool.pin().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pool_lookups_miss() {
        let drafter = AdaptiveDrafter::new();
        assert!(drafter.is_empty());
        assert_eq!(
            drafter.lookup("needless_return", "fn foo() { return 1; }"),
            None
        );
    }

    #[test]
    fn insert_then_lookup_hits() {
        let drafter = AdaptiveDrafter::new();
        drafter.insert(
            "needless_return",
            "fn bar(x: i32) -> i32 { return x + 1; }",
            "fn bar(x: i32) -> i32 { x + 1 }".to_string(),
        );
        assert!(!drafter.is_empty());
        assert_eq!(
            drafter.lookup("needless_return", "fn bar(x: i32) -> i32 { return x + 1; }"),
            Some("fn bar(x: i32) -> i32 { x + 1 }".to_string())
        );
    }

    #[test]
    fn different_buggy_code_misses() {
        // Same rule, different buggy_code → different key → miss.
        let drafter = AdaptiveDrafter::new();
        drafter.insert(
            "needless_return",
            "fn bar(x: i32) -> i32 { return x + 1; }",
            "fn bar(x: i32) -> i32 { x + 1 }".to_string(),
        );
        assert_eq!(
            drafter.lookup("needless_return", "fn baz() -> u8 { return 7; }"),
            None
        );
    }

    #[test]
    fn pool_key_is_deterministic() {
        // G1: same input → same key (BLAKE3 determinism).
        let k1 = pool_key("needless_return", "fn foo() { return 1; }");
        let k2 = pool_key("needless_return", "fn foo() { return 1; }");
        assert_eq!(k1, k2);
    }

    #[test]
    fn pool_key_differs_for_different_input() {
        let k1 = pool_key("needless_return", "fn foo() { return 1; }");
        let k2 = pool_key("needless_return", "fn bar() { return 2; }");
        assert_ne!(k1, k2);
    }

    #[test]
    fn pool_key_differs_for_different_rule() {
        let k1 = pool_key("needless_return", "fn foo() { return 1; }");
        let k2 = pool_key("redundant_closure", "fn foo() { return 1; }");
        assert_ne!(k1, k2);
    }

    #[test]
    fn clear_empties_pool() {
        let drafter = AdaptiveDrafter::new();
        drafter.insert("lint", "code", "fix".to_string());
        assert!(!drafter.is_empty());
        drafter.clear();
        assert!(drafter.is_empty());
    }

    #[test]
    fn concurrent_insert_and_lookup() {
        // papaya is lock-free — concurrent access is safe. This is a smoke test
        // (not a stress test) verifying the API works under a simple race.
        let drafter = std::sync::Arc::new(AdaptiveDrafter::new());
        let mut handles = Vec::with_capacity(4);
        for i in 0..4 {
            let d = drafter.clone();
            handles.push(std::thread::spawn(move || {
                let lint = format!("lint_{i}");
                let code = format!("code_{i}");
                let fix = format!("fix_{i}");
                d.insert(&lint, &code, fix.clone());
                assert_eq!(d.lookup(&lint, &code), Some(fix));
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(drafter.len(), 4);
    }
}
