//! PAV span sources — plan 356 A1's three span pools.
//!
//! 1. **Fixtures** — the frozen 34-fixture score-bench set (the plan names it
//!    directly). Includes the clean/decline fixtures: the driver
//!    short-circuits them to a trivial root pair (q=1, `n_mc`=0) without
//!    spending daemon calls on code that has nothing to fix.
//! 2. **Store** — the lived-experience spans from `.refine/trajectories.kv`
//!    (`TrajRowDoc.shape`, the buggy half the healer actually met). Deduped
//!    by BLAKE3; rows whose `shape` does not parse standalone are counted
//!    and skipped (the daemon's prompt contract needs parseable code).
//! 3. **Extra** — a JSONL `{ "id": ..., "code": ..., "provenance": ... }`
//!    seam for real-repo miss dumps. The clippy lane has no persistent miss
//!    dump today (`.refine/misses/` is the `rustc_errors` queue and carries
//!    rendered diagnostics, not code spans); when A5's reachable-miss set
//!    gets dumped it plugs in here without touching the driver.

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;

use super::rollout::state_hash;

/// Where a span came from (dataset provenance column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanSource {
    Fixture,
    Store,
    Extra,
}

impl SpanSource {
    /// Stable lowercase name for JSONL.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Fixture => "fixture",
            Self::Store => "store",
            Self::Extra => "extra",
        }
    }
}

/// One span to build a prefix tree over.
#[derive(Debug, Clone)]
pub struct PavSpan {
    pub id: String,
    pub provenance: String,
    pub code: String,
    pub source: SpanSource,
}

/// Load the frozen score-bench fixtures as spans.
#[must_use]
pub fn fixture_spans() -> Vec<PavSpan> {
    crate::score_bench::fixtures::fixtures()
        .into_iter()
        .map(|f| PavSpan {
            id: f.id.to_string(),
            provenance: f.provenance.to_string(),
            code: f.code.to_string(),
            source: SpanSource::Fixture,
        })
        .collect()
}

/// Load lived-experience spans from the `.refine/` store under `root`.
///
/// Rows whose `shape` does not parse standalone are skipped (counted in the
/// return); duplicate shapes (the same span met under several lint keys)
/// collapse to one span. Tier-`Withdrawn` rows are skipped — a strategy the
/// memory has retired is not a distribution worth amortizing.
///
/// # Errors
/// `TrajStore::open` failures (WAL replay, unwritable dir).
pub fn store_spans(root: &Path) -> Result<(Vec<PavSpan>, usize), String> {
    let (store, _bad_rows) = crate::traj_store::TrajStore::open(root)?;
    let pruner = crate::pruner::RustAnalyzerPruner::new();
    let mut spans = Vec::new();
    let mut skipped = 0usize;
    let mut seen: HashSet<[u8; 32]> = HashSet::new();
    for row in store.dump() {
        if row.tier == crate::memory::EvidenceTier::Withdrawn.as_u8() {
            skipped += 1;
            continue;
        }
        if !pruner.validate(&row.shape, "").parses {
            skipped += 1;
            continue;
        }
        let h = state_hash(&row.shape);
        if !seen.insert(h) {
            continue;
        }
        spans.push(PavSpan {
            id: format!("store-{:08x}", u32::from_le_bytes([h[0], h[1], h[2], h[3]])),
            provenance: row.lint_key.clone(),
            code: row.shape,
            source: SpanSource::Store,
        });
    }
    Ok((spans, skipped))
}

/// Extra-spans JSONL row.
#[derive(Debug, Deserialize)]
pub struct ExtraSpanRow {
    pub id: String,
    pub code: String,
    #[serde(default)]
    pub provenance: Option<String>,
}

/// Load extra spans from a JSONL file (the miss-dump seam).
///
/// # Errors
/// Read failure or a malformed JSONL line (line number in the message).
pub fn extra_spans(path: &Path) -> Result<Vec<PavSpan>, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut spans = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: ExtraSpanRow =
            serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))?;
        spans.push(PavSpan {
            id: row.id,
            provenance: row.provenance.unwrap_or_else(|| path.display().to_string()),
            code: row.code,
            source: SpanSource::Extra,
        });
    }
    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_spans_load_the_frozen_set() {
        let spans = fixture_spans();
        // v1 froze 34; the P9 v2 step added the ruliology span deliberately
        // (a bench-version step, not silent growth). The harness loads
        // whatever is frozen — the pin here is uniqueness + floor, the
        // exact count belongs to score_bench's own gates.
        assert!(
            spans.len() >= 34,
            "frozen fixture set shrank: {}",
            spans.len()
        );
        let mut ids: Vec<_> = spans.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), spans.len(), "fixture ids must be unique");
        assert!(spans.iter().all(|s| s.source == SpanSource::Fixture));
        assert!(!spans[0].code.is_empty());
    }

    #[test]
    fn extra_spans_round_trip_and_reject_malformed() {
        let dir = std::env::temp_dir().join(format!("pav_extra_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("spans.jsonl");
        std::fs::write(
            &p,
            "{\"id\":\"m1\",\"code\":\"fn a() {}\\n\",\"provenance\":\"bench037\"}\n\
             {\"id\":\"m2\",\"code\":\"fn b() {}\\n\"}\n",
        )
        .unwrap();
        let spans = extra_spans(&p).unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].id, "m1");
        assert_eq!(spans[0].provenance, "bench037");
        assert_eq!(spans[1].provenance, p.display().to_string());

        std::fs::write(&p, "{\"id\":\"x\",\"code\": 3}\n").unwrap();
        assert!(extra_spans(&p).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
